// Route wiring for /webhooks -- register/list endpoints, list deliveries

use actix_web::web;

use crate::handlers::webhooks::{list_deliveries, list_endpoints, register_endpoint};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/webhooks")
            .route("/endpoints", web::post().to(register_endpoint))
            .route("/endpoints", web::get().to(list_endpoints))
            .route("/deliveries", web::get().to(list_deliveries)),
    );
}
