use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::business::ApiKeyRecord;

pub async fn insert(
    pool: &PgPool,
    business_id: Uuid,
    key_prefix: &str,
    key_hash: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO api_keys (business_id, key_prefix, key_hash) VALUES ($1, $2, $3)")
        .bind(business_id)
        .bind(key_prefix)
        .bind(key_hash)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
    Ok(())
}

pub async fn find_active_by_prefix(
    pool: &PgPool,
    key_prefix: &str,
) -> Result<Option<ApiKeyRecord>, AppError> {
    sqlx::query_as::<_, ApiKeyRecord>(
        "SELECT id, business_id, key_prefix, key_hash, revoked_at
         FROM api_keys
         WHERE key_prefix = $1 AND revoked_at IS NULL",
    )
    .bind(key_prefix)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}
