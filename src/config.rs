// App configuration loaded from environment variables
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub mock_psp_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let mock_psp_url = std::env::var("MOCK_PSP_URL")
            .unwrap_or_else(|_| "http://localhost:9090".to_string());

        Self {
            port,
            database_url,
            mock_psp_url,
        }
    }
}
