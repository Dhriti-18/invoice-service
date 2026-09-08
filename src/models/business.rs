use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Business {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKeyRecord {
    pub id: Uuid,
    pub business_id: Uuid,
    pub key_prefix: String,
    pub key_hash: String,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub const API_KEY_PREFIX: &str = "sk_live_";
const SECRET_BYTES: usize = 24; 
const PREFIX_HEX_CHARS: usize = 8;

pub struct GeneratedApiKey {
    pub plaintext: String,
    pub prefix: String,
    pub hash: String,
}

pub fn generate_api_key() -> GeneratedApiKey {
    let mut secret_bytes = [0u8; SECRET_BYTES];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let secret_hex = hex::encode(secret_bytes);

    let plaintext = format!("{API_KEY_PREFIX}{secret_hex}");
    let prefix = format!("{API_KEY_PREFIX}{}", &secret_hex[..PREFIX_HEX_CHARS]);
    let hash = hash_api_key(&plaintext);

    GeneratedApiKey { plaintext, prefix, hash }
}

pub fn hash_api_key(plaintext: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn extract_prefix(plaintext: &str) -> Option<String> {
    let rest = plaintext.strip_prefix(API_KEY_PREFIX)?;
    if rest.len() < PREFIX_HEX_CHARS {
        return None;
    }
    Some(format!("{API_KEY_PREFIX}{}", &rest[..PREFIX_HEX_CHARS]))
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
