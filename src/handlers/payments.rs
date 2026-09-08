use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::db;
use crate::errors::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedBusiness;
use crate::models::invoice::InvoiceState;
use crate::models::payment_attempt::{PayInvoiceRequest, PaymentAttempt};
use crate::services::psp_client::{self, PspError, PspOutcome};
use crate::state::AppState;

const KNOWN_CARD_TOKENS: [&str; 5] = [
    "tok_success",
    "tok_insufficient_funds",
    "tok_card_declined",
    "tok_timeout",
    "tok_network_error",
];

pub async fn pay_invoice(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    http_req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<PayInvoiceRequest>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();

    let idempotency_key = http_req
        .headers()
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Validation("Idempotency-Key header is required".into()))?
        .to_string();

    let card_token = body.card_token.trim().to_string();
    if !KNOWN_CARD_TOKENS.contains(&card_token.as_str()) {
        return Err(AppError::Validation(format!(
            "unknown card_token '{card_token}'"
        )));
    }

    let request_hash = db::idempotency::compute_request_hash(invoice_id, &card_token);

    if let Some(existing) =
        db::idempotency::find_by_key(&state.db, auth.business_id, &idempotency_key).await?
    {
        if existing.request_hash != request_hash {
            return Err(AppError::IdempotencyKeyReuse);
        }
        return Ok(respond_for_attempt(&existing));
    }

    let invoice = db::invoices::find_by_id(&state.db, auth.business_id, invoice_id)
        .await?
        .ok_or_else(|| AppError::NotFound("invoice not found".into()))?;

    let current: InvoiceState = invoice.state.parse().map_err(AppError::Internal)?;
    if current != InvoiceState::Open {
        return Err(AppError::Conflict(format!(
            "invoice is not payable in state '{}'",
            invoice.state
        )));
    }

    let attempt = db::payment_attempts::insert_pending(
        &state.db,
        invoice_id,
        auth.business_id,
        invoice.total_amount_cents,
        &card_token,
        &idempotency_key,
        &request_hash,
    )
    .await?;

    match psp_client::charge(&state.http_client, &state.mock_psp_url, &card_token).await {
        Ok(PspOutcome::Succeeded { psp_ref }) => {
            let attempt = db::payment_attempts::mark_succeeded(&state.db, attempt.id, &psp_ref).await?;
            let transitioned = db::invoices::transition_state(
                &state.db,
                auth.business_id,
                invoice_id,
                InvoiceState::Open,
                InvoiceState::Paid,
            )
            .await?;
            if !transitioned {
                tracing::warn!(invoice_id = %invoice_id, "payment succeeded but invoice was not in 'open' state to transition");
            }
            db::webhooks::enqueue(
                &state.db,
                auth.business_id,
                "invoice.paid",
                &serde_json::json!({ "invoice_id": invoice_id, "payment_attempt": &attempt }),
            )
            .await?;
            Ok(respond_for_attempt(&attempt))
        }
        Ok(PspOutcome::Failed { code }) => {
            let attempt = db::payment_attempts::mark_failed(&state.db, attempt.id, &code).await?;
            db::webhooks::enqueue(
                &state.db,
                auth.business_id,
                "invoice.payment_failed",
                &serde_json::json!({ "invoice_id": invoice_id, "payment_attempt": &attempt }),
            )
            .await?;
            Ok(respond_for_attempt(&attempt))
        }
        Err(PspError::Timeout) => {
            tracing::warn!(attempt_id = %attempt.id, "psp call timed out; attempt left pending");
            Ok(HttpResponse::Accepted().json(&attempt))
        }
        Err(PspError::NetworkError(detail)) => {
            tracing::warn!(attempt_id = %attempt.id, error = %detail, "psp network error; marking attempt failed");
            let attempt =
                db::payment_attempts::mark_failed(&state.db, attempt.id, "network_error").await?;
            Ok(respond_for_attempt(&attempt))
        }
    }
}

fn respond_for_attempt(attempt: &PaymentAttempt) -> HttpResponse {
    if attempt.status == "pending" {
        HttpResponse::Accepted().json(attempt)
    } else {
        HttpResponse::Ok().json(attempt)
    }
}
