// create_invoice, get_invoice, list_invoices, finalize_invoice, void_invoice

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::db;
use crate::errors::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedBusiness;
use crate::models::invoice::{CreateInvoiceRequest, InvoiceState};
use crate::money::Cents;
use crate::state::AppState;

pub async fn create_invoice(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    body: web::Json<CreateInvoiceRequest>,
) -> AppResult<HttpResponse> {
    if body.line_items.is_empty() {
        return Err(AppError::Validation(
            "an invoice needs at least one line item".into(),
        ));
    }

    if let Some(due_date) = body.due_date {
        if due_date < chrono::Utc::now().date_naive() {
            return Err(AppError::Validation(
                "due_date must not be in the past".into(),
            ));
        }
    }

    db::customers::find_by_id(&state.db, auth.business_id, body.customer_id)
        .await?
        .ok_or_else(|| AppError::Validation("customer_id does not refer to a known customer".into()))?;

    let mut total = Cents::ZERO;
    let mut items = Vec::with_capacity(body.line_items.len());
    for item in &body.line_items {
        if item.quantity <= 0 || item.quantity > i32::MAX as i64 {
            return Err(AppError::Validation(
                "quantity must be a positive integer".into(),
            ));
        }
        if item.unit_amount_cents < 0 {
            return Err(AppError::Validation(
                "unit_amount_cents must not be negative".into(),
            ));
        }

        let line_total = Cents(item.unit_amount_cents)
            .checked_mul_qty(item.quantity)
            .ok_or_else(|| AppError::Validation("line item amount overflows".into()))?;
        total = total
            .checked_add(line_total)
            .ok_or_else(|| AppError::Validation("invoice total overflows".into()))?;

        items.push(db::invoices::NewLineItem {
            description: item.description.trim(),
            quantity: item.quantity as i32,
            unit_amount_cents: item.unit_amount_cents,
        });
    }

    let (invoice, line_items) = db::invoices::insert(
        &state.db,
        auth.business_id,
        body.customer_id,
        body.due_date,
        total.0,
        &items,
    )
    .await?;

    db::webhooks::enqueue(
        &state.db,
        auth.business_id,
        "invoice.created",
        &serde_json::json!({ "invoice": &invoice, "line_items": &line_items }),
    )
    .await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "invoice": invoice,
        "line_items": line_items,
    })))
}

pub async fn get_invoice(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;
    let line_items = db::invoices::find_line_items(&state.db, invoice.id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "invoice": invoice,
        "line_items": line_items,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListInvoicesQuery {
    pub state: Option<String>,
}

pub async fn list_invoices(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    query: web::Query<ListInvoicesQuery>,
) -> AppResult<HttpResponse> {
    let filter = match &query.state {
        Some(s) => Some(s.parse::<InvoiceState>().map_err(AppError::Validation)?),
        None => None,
    };

    let invoices = db::invoices::list(&state.db, auth.business_id, filter).await?;
    Ok(HttpResponse::Ok().json(invoices))
}

pub async fn finalize_invoice(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;

    let current: InvoiceState = invoice.state.parse().map_err(AppError::Internal)?;
    if current != InvoiceState::Draft {
        return Err(AppError::Conflict(format!(
            "cannot finalize an invoice in state '{}', only 'draft' invoices can be finalized",
            invoice.state
        )));
    }

    let transitioned = db::invoices::transition_state(
        &state.db,
        auth.business_id,
        id,
        InvoiceState::Draft,
        InvoiceState::Open,
    )
    .await?;
    if !transitioned {
        return Err(AppError::Conflict(
            "invoice state changed concurrently, retry".into(),
        ));
    }

    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;
    Ok(HttpResponse::Ok().json(invoice))
}

pub async fn void_invoice(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;

    let current: InvoiceState = invoice.state.parse().map_err(AppError::Internal)?;
    if current != InvoiceState::Draft && current != InvoiceState::Open {
        return Err(AppError::Conflict(format!(
            "cannot void an invoice in state '{}'",
            invoice.state
        )));
    }

    let transitioned =
        db::invoices::transition_state(&state.db, auth.business_id, id, current, InvoiceState::Void).await?;
    if !transitioned {
        return Err(AppError::Conflict(
            "invoice state changed concurrently, retry".into(),
        ));
    }

    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;
    Ok(HttpResponse::Ok().json(invoice))
}

pub async fn mark_uncollectible_invoice(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;

    let current: InvoiceState = invoice.state.parse().map_err(AppError::Internal)?;
    if current != InvoiceState::Open {
        return Err(AppError::Conflict(format!(
            "cannot mark an invoice uncollectible from state '{}', only 'open' invoices qualify",
            invoice.state
        )));
    }

    let transitioned = db::invoices::transition_state(
        &state.db,
        auth.business_id,
        id,
        InvoiceState::Open,
        InvoiceState::Uncollectible,
    )
    .await?;
    if !transitioned {
        return Err(AppError::Conflict(
            "invoice state changed concurrently, retry".into(),
        ));
    }

    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;
    Ok(HttpResponse::Ok().json(invoice))
}
