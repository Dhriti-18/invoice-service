use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct PaymentAttempt {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub business_id: Uuid,
    pub status: String,
    pub amount_cents: i64,
    pub card_token: String,
    pub psp_ref: Option<String>,
    pub failure_code: Option<String>,
    pub idempotency_key: String,
    #[serde(skip)]
    pub request_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PayInvoiceRequest {
    pub card_token: String,
}
