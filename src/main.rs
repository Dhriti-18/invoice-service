#[actix_web::main]
async fn main() -> std::io::Result<()> {
    invoice_service::server::run().await
}
