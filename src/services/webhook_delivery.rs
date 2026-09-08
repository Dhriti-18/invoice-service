use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use std::time::Duration;

use crate::db;
use crate::errors::AppError;
use crate::models::webhook::WebhookDelivery;

type HmacSha256 = Hmac<Sha256>;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(10);
const BACKOFF_SECONDS: [i64; 6] = [30, 120, 600, 1800, 7200, 21600];

pub fn sign(secret: &[u8], timestamp: i64, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(format!("{timestamp}.{body}").as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    format!("t={timestamp},v1={signature}")
}

pub fn spawn(pool: PgPool, http_client: reqwest::Client) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = deliver_due(&pool, &http_client).await {
                tracing::error!(error = %e, "webhook delivery pass failed");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn deliver_due(pool: &PgPool, http_client: &reqwest::Client) -> Result<(), AppError> {
    let due = db::webhooks::fetch_due(pool, 50).await?;

    for delivery in due {
        let endpoint = match db::webhooks::find_endpoint(pool, delivery.endpoint_id).await? {
            Some(e) if e.active => e,
            _ => continue,
        };

        let envelope = serde_json::json!({
            "id": delivery.event_id,
            "type": delivery.event_type,
            "created_at": delivery.created_at,
            "data": delivery.payload,
        });
        let body = envelope.to_string();
        let timestamp = Utc::now().timestamp();
        let signature = sign(&endpoint.secret, timestamp, &body);

        let result = http_client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .header("X-Webhook-Event-Id", delivery.event_id.to_string())
            .timeout(DELIVERY_TIMEOUT)
            .body(body)
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                db::webhooks::mark_delivered(pool, delivery.id, resp.status().as_u16() as i32).await?;
            }
            Ok(resp) => {
                let status = resp.status().as_u16() as i32;
                record_failure(pool, &delivery, Some(status), &format!("non-2xx response: {status}")).await?;
            }
            Err(e) => {
                record_failure(pool, &delivery, None, &e.to_string()).await?;
            }
        }
    }

    Ok(())
}

async fn record_failure(
    pool: &PgPool,
    delivery: &WebhookDelivery,
    response_status: Option<i32>,
    error: &str,
) -> Result<(), AppError> {
    let next_attempt_count = delivery.attempt_count + 1;
    let next_attempt_at: Option<DateTime<Utc>> = BACKOFF_SECONDS
        .get(delivery.attempt_count as usize)
        .map(|secs| Utc::now() + chrono::Duration::seconds(*secs));

    db::webhooks::mark_failed(pool, delivery.id, next_attempt_count, next_attempt_at, error, response_status).await
}
