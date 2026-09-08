use actix_web::web;

use crate::handlers::invoices::{
    create_invoice, finalize_invoice, get_invoice, list_invoices, mark_uncollectible_invoice, void_invoice,
};
use crate::handlers::payments::pay_invoice;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/invoices")
            .route("", web::post().to(create_invoice))
            .route("", web::get().to(list_invoices))
            .route("/{id}", web::get().to(get_invoice))
            .route("/{id}/finalize", web::post().to(finalize_invoice))
            .route("/{id}/void", web::post().to(void_invoice))
            .route("/{id}/mark_uncollectible", web::post().to(mark_uncollectible_invoice))
            .route("/{id}/pay", web::post().to(pay_invoice)),
    );
}
