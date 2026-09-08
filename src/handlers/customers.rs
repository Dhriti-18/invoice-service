// create_customer, get_customer, list_customers

use actix_web::{web, HttpResponse};
use uuid::Uuid;

use crate::db;
use crate::errors::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedBusiness;
use crate::models::customer::CreateCustomerRequest;
use crate::state::AppState;

pub async fn create_customer(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    body: web::Json<CreateCustomerRequest>,
) -> AppResult<HttpResponse> {
    let name = body.name.trim();
    let email = body.email.trim();

    if name.is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::Validation("email must be a valid address".into()));
    }

    let customer = db::customers::insert(&state.db, auth.business_id, name, email).await?;
    Ok(HttpResponse::Created().json(customer))
}

pub async fn get_customer(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let customer_id = path.into_inner();

    let customer = db::customers::find_by_id(&state.db, auth.business_id, customer_id)
        .await?
        .ok_or_else(|| AppError::NotFound("customer not found".into()))?;

    Ok(HttpResponse::Ok().json(customer))
}

pub async fn list_customers(
    auth: AuthenticatedBusiness,
    state: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let customers = db::customers::list(&state.db, auth.business_id).await?;
    Ok(HttpResponse::Ok().json(customers))
}
