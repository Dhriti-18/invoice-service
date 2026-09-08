use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::payment_attempt::PaymentAttempt;

pub fn compute_request_hash(invoice_id: Uuid, card_token: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(invoice_id.as_bytes());
    hasher.update(b"|");
    hasher.update(card_token.as_bytes());
    hasher.finalize().to_vec()
}

pub async fn find_by_key(
    pool: &PgPool,
    business_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<PaymentAttempt>, AppError> {
    sqlx::query_as::<_, PaymentAttempt>(
        "SELECT id, invoice_id, business_id, status, amount_cents, card_token, psp_ref, failure_code,
                idempotency_key, request_hash, created_at, updated_at
         FROM payment_attempts WHERE business_id = $1 AND idempotency_key = $2",
    )
    .bind(business_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}
