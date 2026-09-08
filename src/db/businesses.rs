use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::business::Business;

pub async fn insert(pool: &PgPool, name: &str) -> Result<Business, AppError> {
    sqlx::query_as::<_, Business>(
        "INSERT INTO businesses (name) VALUES ($1) RETURNING id, name, created_at",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

#[allow(dead_code)]
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Business>, AppError> {
    sqlx::query_as::<_, Business>("SELECT id, name, created_at FROM businesses WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)
}
