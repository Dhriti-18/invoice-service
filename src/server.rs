use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;

use crate::config::Config;
use crate::routes;
use crate::services::webhook_delivery;
use crate::state::AppState;

pub async fn run() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env();

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    let state = AppState {
        db,
        http_client: reqwest::Client::new(),
        mock_psp_url: config.mock_psp_url.clone(),
    };

    // Runs independently of the HTTP server
    webhook_delivery::spawn(state.db.clone(), state.http_client.clone());

    println!("invoice-service listening on 0.0.0.0:{}", config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(routes::configure)
    })
    .bind(("0.0.0.0", config.port))?
    .run()
    .await
}
