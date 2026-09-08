use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::payment_attempt::PaymentAttempt;

const COLUMNS: &str = "id, invoice_id, business_id, status, amount_cents, card_token, psp_ref, \
                        failure_code, idempotency_key, request_hash, created_at, updated_at";

pub async fn insert_pending(
    pool: &PgPool,
    invoice_id: Uuid,
    business_id: Uuid,
    amount_cents: i64,
    card_token: &str,
    idempotency_key: &str,
    request_hash: &[u8],
) -> Result<PaymentAttempt, AppError> {
    sqlx::query_as::<_, PaymentAttempt>(&format!(
        "INSERT INTO payment_attempts
            (invoice_id, business_id, status, amount_cents, card_token, idempotency_key, request_hash)
         VALUES ($1, $2, 'pending', $3, $4, $5, $6)
         RETURNING {COLUMNS}"
    ))
    .bind(invoice_id)
    .bind(business_id)
    .bind(amount_cents)
    .bind(card_token)
    .bind(idempotency_key)
    .bind(request_hash)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("idx_payment_attempts_one_pending_per_invoice") =>
        {
            AppError::Conflict("a payment is already in progress for this invoice".into())
        }
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("idx_payment_attempts_one_success_per_invoice") =>
        {
            AppError::Conflict("this invoice has already been paid".into())
        }
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("payment_attempts_business_id_idempotency_key_key") =>
        {
            AppError::Conflict(
                "a payment attempt with this idempotency key is already being processed".into(),
            )
        }
        _ => AppError::from(e),
    })
}

pub async fn mark_succeeded(pool: &PgPool, id: Uuid, psp_ref: &str) -> Result<PaymentAttempt, AppError> {
    sqlx::query_as::<_, PaymentAttempt>(&format!(
        "UPDATE payment_attempts SET status = 'succeeded', psp_ref = $1, updated_at = now()
         WHERE id = $2 AND status = 'pending'
         RETURNING {COLUMNS}"
    ))
    .bind(psp_ref)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn mark_failed(pool: &PgPool, id: Uuid, failure_code: &str) -> Result<PaymentAttempt, AppError> {
    sqlx::query_as::<_, PaymentAttempt>(&format!(
        "UPDATE payment_attempts SET status = 'failed', failure_code = $1, updated_at = now()
         WHERE id = $2 AND status = 'pending'
         RETURNING {COLUMNS}"
    ))
    .bind(failure_code)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

#[allow(dead_code)]
pub async fn find_by_id(pool: &PgPool, business_id: Uuid, id: Uuid) -> Result<Option<PaymentAttempt>, AppError> {
    sqlx::query_as::<_, PaymentAttempt>(&format!(
        "SELECT {COLUMNS} FROM payment_attempts WHERE id = $1 AND business_id = $2"
    ))
    .bind(id)
    .bind(business_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}
