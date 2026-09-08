use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::invoice::{Invoice, InvoiceLineItem, InvoiceState};

pub struct NewLineItem<'a> {
    pub description: &'a str,
    pub quantity: i32,
    pub unit_amount_cents: i64,
}

const INVOICE_COLUMNS: &str =
    "id, business_id, customer_id, total_amount_cents, currency, state, due_date, created_at, updated_at";

pub async fn insert(
    pool: &PgPool,
    business_id: Uuid,
    customer_id: Uuid,
    due_date: Option<NaiveDate>,
    total_amount_cents: i64,
    line_items: &[NewLineItem<'_>],
) -> Result<(Invoice, Vec<InvoiceLineItem>), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let invoice = sqlx::query_as::<_, Invoice>(&format!(
        "INSERT INTO invoices (business_id, customer_id, total_amount_cents, state, due_date)
         VALUES ($1, $2, $3, 'draft', $4)
         RETURNING {INVOICE_COLUMNS}"
    ))
    .bind(business_id)
    .bind(customer_id)
    .bind(total_amount_cents)
    .bind(due_date)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let mut items = Vec::with_capacity(line_items.len());
    for (idx, item) in line_items.iter().enumerate() {
        let row = sqlx::query_as::<_, InvoiceLineItem>(
            "INSERT INTO invoice_line_items (invoice_id, position, description, quantity, unit_amount_cents)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, invoice_id, position, description, quantity, unit_amount_cents, amount_cents, created_at",
        )
        .bind(invoice.id)
        .bind(idx as i32)
        .bind(item.description)
        .bind(item.quantity)
        .bind(item.unit_amount_cents)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::from)?;
        items.push(row);
    }

    tx.commit().await.map_err(AppError::from)?;

    Ok((invoice, items))
}

pub async fn find_by_id(pool: &PgPool, business_id: Uuid, id: Uuid) -> Result<Option<Invoice>, AppError> {
    sqlx::query_as::<_, Invoice>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoices WHERE id = $1 AND business_id = $2"
    ))
    .bind(id)
    .bind(business_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}

pub async fn find_by_id_for_update(
    tx: &mut sqlx::PgConnection,
    business_id: Uuid,
    id: Uuid,
) -> Result<Option<Invoice>, AppError> {
    sqlx::query_as::<_, Invoice>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoices WHERE id = $1 AND business_id = $2 FOR UPDATE"
    ))
    .bind(id)
    .bind(business_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)
}

pub async fn find_line_items(pool: &PgPool, invoice_id: Uuid) -> Result<Vec<InvoiceLineItem>, AppError> {
    sqlx::query_as::<_, InvoiceLineItem>(
        "SELECT id, invoice_id, position, description, quantity, unit_amount_cents, amount_cents, created_at
         FROM invoice_line_items WHERE invoice_id = $1 ORDER BY position",
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list(pool: &PgPool, business_id: Uuid, state: Option<InvoiceState>) -> Result<Vec<Invoice>, AppError> {
    match state {
        Some(s) => {
            sqlx::query_as::<_, Invoice>(&format!(
                "SELECT {INVOICE_COLUMNS} FROM invoices WHERE business_id = $1 AND state = $2 ORDER BY created_at DESC"
            ))
            .bind(business_id)
            .bind(s.as_str())
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, Invoice>(&format!(
                "SELECT {INVOICE_COLUMNS} FROM invoices WHERE business_id = $1 ORDER BY created_at DESC"
            ))
            .bind(business_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(AppError::from)
}

pub async fn transition_state(
    pool: &PgPool,
    business_id: Uuid,
    id: Uuid,
    from: InvoiceState,
    to: InvoiceState,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "UPDATE invoices SET state = $1, updated_at = now()
         WHERE id = $2 AND business_id = $3 AND state = $4",
    )
    .bind(to.as_str())
    .bind(id)
    .bind(business_id)
    .bind(from.as_str())
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(result.rows_affected() == 1)
}
