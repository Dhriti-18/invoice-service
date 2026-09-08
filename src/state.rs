use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub http_client: reqwest::Client,
    pub mock_psp_url: String,
}
