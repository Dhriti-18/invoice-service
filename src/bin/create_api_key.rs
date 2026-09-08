//create api keys for businesses
use invoice_service::{db, models::business::generate_api_key};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            eprintln!("usage: create-api-key <business-name>");
            std::process::exit(1);
        });

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    let business = db::businesses::insert(&pool, &name).await?;
    let generated = generate_api_key();
    db::api_keys::insert(&pool, business.id, &generated.prefix, &generated.hash).await?;

    println!("business_id: {}", business.id);
    println!("api_key:     {}", generated.plaintext);
    println!("(store this now -- it cannot be recovered later, only its hash is kept)");

    Ok(())
}
