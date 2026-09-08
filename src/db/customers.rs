use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::customer::Customer;

pub async fn insert(
    pool: &PgPool,
    business_id: Uuid,
    name: &str,
    email: &str,
) -> Result<Customer, AppError> {
    sqlx::query_as::<_, Customer>(
        "INSERT INTO customers (business_id, name, email)
         VALUES ($1, $2, $3)
         RETURNING id, business_id, name, email, created_at",
    )
    .bind(business_id)
    .bind(name)
    .bind(email)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.constraint() == Some("customers_business_id_email_key") => {
            AppError::Conflict("a customer with this email already exists for this business".into())
        }
        _ => AppError::from(e),
    })
}

pub async fn find_by_id(
    pool: &PgPool,
    business_id: Uuid,
    id: Uuid,
) -> Result<Option<Customer>, AppError> {
    sqlx::query_as::<_, Customer>(
        "SELECT id, business_id, name, email, created_at
         FROM customers
         WHERE id = $1 AND business_id = $2",
    )
    .bind(id)
    .bind(business_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list(pool: &PgPool, business_id: Uuid) -> Result<Vec<Customer>, AppError> {
    sqlx::query_as::<_, Customer>(
        "SELECT id, business_id, name, email, created_at
         FROM customers
         WHERE business_id = $1
         ORDER BY created_at DESC",
    )
    .bind(business_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}
