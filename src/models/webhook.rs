use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

const SECRET_BYTES: usize = 32;

pub fn generate_secret() -> Vec<u8> {
    let mut bytes = vec![0u8; SECRET_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub business_id: Uuid,
    pub url: String,
    #[serde(skip)]
    pub secret: Vec<u8>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterWebhookEndpointRequest {
    pub url: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub business_id: Uuid,
    pub endpoint_id: Uuid,
    pub event_id: Uuid,
    pub event_type: String,
    pub payload: Value,
    pub status: String,
    pub attempt_count: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub last_response_status: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
