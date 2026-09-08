use actix_web::web;

use crate::handlers::customers::{create_customer, get_customer, list_customers};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/customers")
            .route("", web::post().to(create_customer))
            .route("", web::get().to(list_customers))
            .route("/{id}", web::get().to(get_customer)),
    );
}
