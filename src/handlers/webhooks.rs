// register_endpoint, list_endpoints, list_deliveries

use actix_web::{web, HttpResponse};

use crate::db;
use crate::errors::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedBusiness;
use crate::models::webhook::{generate_secret, RegisterWebhookEndpointRequest};
use crate::state::AppState;

pub async fn register_endpoint(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    body: web::Json<RegisterWebhookEndpointRequest>,
) -> AppResult<HttpResponse> {
    let url = body.url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::Validation(
            "url must be an absolute http:// or https:// URL".into(),
        ));
    }

    let secret = generate_secret();
    let endpoint = db::webhooks::register_endpoint(&state.db, auth.business_id, url, &secret).await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "id": endpoint.id,
        "business_id": endpoint.business_id,
        "url": endpoint.url,
        "active": endpoint.active,
        "created_at": endpoint.created_at,
        "secret": hex::encode(&secret),
    })))
}

pub async fn list_endpoints(auth: AuthenticatedBusiness, state: web::Data<AppState>) -> AppResult<HttpResponse> {
    let endpoints = db::webhooks::list_endpoints(&state.db, auth.business_id).await?;
    Ok(HttpResponse::Ok().json(endpoints))
}

pub async fn list_deliveries(auth: AuthenticatedBusiness, state: web::Data<AppState>) -> AppResult<HttpResponse> {
    let deliveries = db::webhooks::list_deliveries(&state.db, auth.business_id).await?;
    Ok(HttpResponse::Ok().json(deliveries))
}
