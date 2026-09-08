// Reads the API key header, looks up the owning business, rejects if invalid
//
// Implemented as a request extractor rather than an actix Transform middleware:
// adding `auth: AuthenticatedBusiness` to a handler's signature is enough to
// require authentication for that route, and every db call downstream is
// naturally scoped to `auth.business_id` -- there's no separate "did auth
// pass" flag to forget to check.

use std::future::Future;
use std::pin::Pin;

use actix_web::{dev::Payload, http::header::AUTHORIZATION, web, FromRequest, HttpRequest};
use uuid::Uuid;

use crate::db;
use crate::errors::AppError;
use crate::models::business::{constant_time_eq, extract_prefix, hash_api_key};
use crate::state::AppState;

/// Proof that this request carried a valid, unrevoked API key. Every
/// business-scoped query should be filtered by `business_id` from here.
#[derive(Debug, Clone, Copy)]
pub struct AuthenticatedBusiness {
    pub business_id: Uuid,
}

impl FromRequest for AuthenticatedBusiness {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let presented_key = extract_bearer_token(&req).ok_or(AppError::Unauthorized)?;
            let prefix = extract_prefix(&presented_key).ok_or(AppError::Unauthorized)?;

            let state = req
                .app_data::<web::Data<AppState>>()
                .expect("AppState not registered as app_data");

            let record = db::api_keys::find_active_by_prefix(&state.db, &prefix)
                .await?
                .ok_or(AppError::Unauthorized)?;

            let presented_hash = hash_api_key(&presented_key);
            if !constant_time_eq(&presented_hash, &record.key_hash) {
                return Err(AppError::Unauthorized);
            }

            Ok(AuthenticatedBusiness {
                business_id: record.business_id,
            })
        })
    }
}

/// Expects `Authorization: Bearer sk_live_...`. Chosen over a bespoke
/// `X-API-Key` header because it's the standard slot for bearer credentials,
/// which means it's the one most HTTP libraries, proxies, and log-scrubbing
/// tools already know to redact.
fn extract_bearer_token(req: &HttpRequest) -> Option<String> {
    let header_value = req.headers().get(AUTHORIZATION)?;
    let value = header_value.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}
