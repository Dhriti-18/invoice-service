// Request handlers, one file per resource — parse request, call db/services, return response

pub mod health;
pub mod customers;
pub mod invoices;
pub mod payments;
pub mod webhooks;
