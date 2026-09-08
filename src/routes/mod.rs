mod customers;
mod invoices;
mod webhooks;

use actix_web::web;

use crate::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(handlers::health::health));
    customers::configure(cfg);
    invoices::configure(cfg);
    webhooks::configure(cfg);
}
