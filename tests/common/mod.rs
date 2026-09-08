// Shared fixtures for the integration tests in this directory.
//
// Each test spins up the *real* invoice-service HTTP server (same
// AppState/router as main.rs) on an OS-assigned loopback port, backed by a
// real Postgres and a real mock-psp reachable over HTTP -- these are
// black-box tests against the actual binary's behavior, not against mocks of
// our own code.
//
// Requires `docker compose up -d postgres mock-psp` (or local equivalents)
// running before `cargo test`. DATABASE_URL / MOCK_PSP_URL are read the same
// way the app itself reads them, with .env.example-matching localhost
// defaults so `cargo test` works out of the box against a local compose
// setup. See README "Testing".

use std::net::TcpListener;

use invoice_service::{db, models::business::generate_api_key, routes, state::AppState};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

pub struct TestApp {
    pub base_url: String,
    pub client: Client,
    pub db: PgPool,
}

pub async fn spawn_app() -> TestApp {
    let db = db_pool().await;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral test port");
    let port = listener.local_addr().unwrap().port();

    let state = AppState {
        db: db.clone(),
        http_client: Client::new(),
        mock_psp_url: mock_psp_url(),
    };

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(actix_web::web::Data::new(state.clone()))
            .configure(routes::configure)
    })
    .listen(listener)
    .expect("attach listener to actix server")
    .run();

    // Fire-and-forget: lives for the rest of the test process. There is no
    // shutdown handshake because there is nothing to clean up beyond the
    // process exiting when the test binary does.
    tokio::spawn(server);

    TestApp {
        base_url: format!("http://127.0.0.1:{port}"),
        client: Client::new(),
        db,
    }
}

async fn db_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://invoice:invoice@localhost:5432/invoice_service".to_string());
    let pool = PgPool::connect(&database_url)
        .await
        .expect("connect to test database -- is `docker compose up -d postgres` running?");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations against test database");
    pool
}

fn mock_psp_url() -> String {
    std::env::var("MOCK_PSP_URL").unwrap_or_else(|_| "http://localhost:9090".to_string())
}

impl TestApp {
    /// Fresh business + bearer token, named uniquely per call so concurrent
    /// tests never collide with each other's rows in the shared test database.
    pub async fn new_business(&self) -> (Uuid, String) {
        let business = db::businesses::insert(&self.db, &format!("test-business-{}", Uuid::new_v4()))
            .await
            .expect("insert business");
        let key = generate_api_key();
        db::api_keys::insert(&self.db, business.id, &key.prefix, &key.hash)
            .await
            .expect("insert api key");
        (business.id, key.plaintext)
    }

    /// Creates a customer and a one-line-item invoice, then finalizes it to
    /// `open` -- all through the real HTTP endpoints. Returns (invoice_id,
    /// total_amount_cents).
    pub async fn create_open_invoice(&self, token: &str, unit_amount_cents: i64) -> (Uuid, i64) {
        let customer: Value = self
            .client
            .post(format!("{}/customers", self.base_url))
            .bearer_auth(token)
            .json(&json!({
                "name": "Test Customer",
                "email": format!("{}@example.com", Uuid::new_v4()),
            }))
            .send()
            .await
            .expect("create customer request failed")
            .error_for_status()
            .expect("create customer returned an error status")
            .json()
            .await
            .expect("parse create customer response");
        let customer_id = customer["id"].as_str().expect("customer id");

        let created: Value = self
            .client
            .post(format!("{}/invoices", self.base_url))
            .bearer_auth(token)
            .json(&json!({
                "customer_id": customer_id,
                "line_items": [
                    { "description": "Widget", "quantity": 1, "unit_amount_cents": unit_amount_cents }
                ],
            }))
            .send()
            .await
            .expect("create invoice request failed")
            .error_for_status()
            .expect("create invoice returned an error status")
            .json()
            .await
            .expect("parse create invoice response");
        let invoice_id: Uuid = created["invoice"]["id"]
            .as_str()
            .expect("invoice id")
            .parse()
            .expect("invoice id is a uuid");
        let total = created["invoice"]["total_amount_cents"]
            .as_i64()
            .expect("total_amount_cents");

        self.client
            .post(format!("{}/invoices/{invoice_id}/finalize", self.base_url))
            .bearer_auth(token)
            .send()
            .await
            .expect("finalize invoice request failed")
            .error_for_status()
            .expect("finalize invoice returned an error status");

        (invoice_id, total)
    }

    pub async fn pay(&self, token: &str, invoice_id: Uuid, idempotency_key: &str, card_token: &str) -> Response {
        self.client
            .post(format!("{}/invoices/{invoice_id}/pay", self.base_url))
            .bearer_auth(token)
            .header("Idempotency-Key", idempotency_key)
            .json(&json!({ "card_token": card_token }))
            .send()
            .await
            .expect("pay request failed")
    }

    pub async fn get_invoice(&self, token: &str, invoice_id: Uuid) -> Value {
        self.client
            .get(format!("{}/invoices/{invoice_id}", self.base_url))
            .bearer_auth(token)
            .send()
            .await
            .expect("get invoice request failed")
            .json()
            .await
            .expect("parse get invoice response")
    }

    pub async fn count_payment_attempts(&self, invoice_id: Uuid, status: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM payment_attempts WHERE invoice_id = $1 AND status = $2",
        )
        .bind(invoice_id)
        .bind(status)
        .fetch_one(&self.db)
        .await
        .expect("count payment attempts")
    }

    pub async fn count_payment_attempts_for_key(&self, business_id: Uuid, idempotency_key: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM payment_attempts WHERE business_id = $1 AND idempotency_key = $2",
        )
        .bind(business_id)
        .bind(idempotency_key)
        .fetch_one(&self.db)
        .await
        .expect("count payment attempts for key")
    }
}
