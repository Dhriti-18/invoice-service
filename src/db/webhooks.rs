use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::webhook::{WebhookDelivery, WebhookEndpoint};

const ENDPOINT_COLUMNS: &str = "id, business_id, url, secret, active, created_at";
const DELIVERY_COLUMNS: &str = "id, business_id, endpoint_id, event_id, event_type, payload, status, \
                                 attempt_count, next_attempt_at, last_error, last_response_status, \
                                 created_at, updated_at";

pub async fn register_endpoint(
    pool: &PgPool,
    business_id: Uuid,
    url: &str,
    secret: &[u8],
) -> Result<WebhookEndpoint, AppError> {
    sqlx::query_as::<_, WebhookEndpoint>(&format!(
        "INSERT INTO webhook_endpoints (business_id, url, secret)
         VALUES ($1, $2, $3)
         RETURNING {ENDPOINT_COLUMNS}"
    ))
    .bind(business_id)
    .bind(url)
    .bind(secret)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_endpoints(pool: &PgPool, business_id: Uuid) -> Result<Vec<WebhookEndpoint>, AppError> {
    sqlx::query_as::<_, WebhookEndpoint>(&format!(
        "SELECT {ENDPOINT_COLUMNS} FROM webhook_endpoints WHERE business_id = $1 ORDER BY created_at DESC"
    ))
    .bind(business_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn find_endpoint(pool: &PgPool, id: Uuid) -> Result<Option<WebhookEndpoint>, AppError> {
    sqlx::query_as::<_, WebhookEndpoint>(&format!(
        "SELECT {ENDPOINT_COLUMNS} FROM webhook_endpoints WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}

async fn active_endpoints_for(pool: &PgPool, business_id: Uuid) -> Result<Vec<WebhookEndpoint>, AppError> {
    sqlx::query_as::<_, WebhookEndpoint>(&format!(
        "SELECT {ENDPOINT_COLUMNS} FROM webhook_endpoints WHERE business_id = $1 AND active = true"
    ))
    .bind(business_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn enqueue(pool: &PgPool, business_id: Uuid, event_type: &str, payload: &Value) -> Result<(), AppError> {
    let endpoints = active_endpoints_for(pool, business_id).await?;
    for endpoint in endpoints {
        sqlx::query(
            "INSERT INTO webhook_deliveries (business_id, endpoint_id, event_type, payload)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(business_id)
        .bind(endpoint.id)
        .bind(event_type)
        .bind(payload)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
    }
    Ok(())
}

pub async fn list_deliveries(pool: &PgPool, business_id: Uuid) -> Result<Vec<WebhookDelivery>, AppError> {
    sqlx::query_as::<_, WebhookDelivery>(&format!(
        "SELECT {DELIVERY_COLUMNS} FROM webhook_deliveries
         WHERE business_id = $1 ORDER BY created_at DESC LIMIT 200"
    ))
    .bind(business_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn fetch_due(pool: &PgPool, limit: i64) -> Result<Vec<WebhookDelivery>, AppError> {
    sqlx::query_as::<_, WebhookDelivery>(&format!(
        "SELECT {DELIVERY_COLUMNS} FROM webhook_deliveries
         WHERE status = 'pending' AND next_attempt_at <= now()
         ORDER BY next_attempt_at
         LIMIT $1"
    ))
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn mark_delivered(pool: &PgPool, id: Uuid, response_status: i32) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE webhook_deliveries
         SET status = 'succeeded', attempt_count = attempt_count + 1,
             last_response_status = $1, last_error = NULL, updated_at = now()
         WHERE id = $2",
    )
    .bind(response_status)
    .bind(id)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(AppError::from)
}

pub async fn mark_failed(
    pool: &PgPool,
    id: Uuid,
    attempt_count: i32,
    next_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    error: &str,
    response_status: Option<i32>,
) -> Result<(), AppError> {
    let status = if next_attempt_at.is_some() { "pending" } else { "exhausted" };
    sqlx::query(
        "UPDATE webhook_deliveries
         SET status = $1, attempt_count = $2, next_attempt_at = COALESCE($3, next_attempt_at),
             last_error = $4, last_response_status = $5, updated_at = now()
         WHERE id = $6",
    )
    .bind(status)
    .bind(attempt_count)
    .bind(next_attempt_at)
    .bind(error)
    .bind(response_status)
    .bind(id)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(AppError::from)
}
