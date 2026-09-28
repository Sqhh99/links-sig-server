//! Configuration module - Environment-based configuration loading
//!
//! All configuration is loaded from environment variables with sensible defaults
//! for development.

use std::env;

/// Server configuration
#[derive(Clone, Debug)]
pub struct Config {
    // ========================================================================
    // LiveKit Settings
    // ========================================================================
    /// LiveKit API URL (HTTP/HTTPS)
    pub livekit_url: String,
    /// LiveKit WebSocket URL for clients
    pub livekit_ws_url: String,
    /// LiveKit API Key
    pub api_key: String,
    /// LiveKit API Secret
    pub api_secret: String,

    // ========================================================================
    // Server Settings
    // ========================================================================
    /// Server port
    pub server_port: u16,
    /// Server host
    pub server_host: String,
    /// Enable HTTPS
    pub enable_https: bool,
    /// Base URL for application share links
    pub app_base_url: String,
    /// SSL certificate file path
    pub ssl_cert_file: String,
    /// SSL key file path
    pub ssl_key_file: String,

    // ========================================================================
    // Database Settings
    // ========================================================================
    /// PostgreSQL connection URL
    pub database_url: String,

    // ========================================================================
    // Meeting Lifecycle Settings
    // ========================================================================
    /// Background lifecycle check interval in seconds
    pub meeting_lifecycle_interval_secs: u64,
    /// PostgreSQL advisory lock key for lifecycle worker
    pub meeting_lifecycle_lock_key: i64,

    // ========================================================================
    // JWT Settings (User Auth)
    // ========================================================================
    /// JWT secret for user access tokens (separate from LiveKit API secret)
    pub jwt_secret: String,
    /// JWT token expiration in seconds (default: 7 days)
    pub jwt_expiration_secs: u64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            // LiveKit settings
            livekit_url: env::var("LIVEKIT_URL")
                .unwrap_or_else(|_| "http://localhost:7880".to_string()),
            livekit_ws_url: env::var("LIVEKIT_WS_URL")
                .unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string()),
            api_key: env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "devkey".to_string()),
            api_secret: env::var("LIVEKIT_API_SECRET").unwrap_or_else(|_| "secret".to_string()),

            // Server settings
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .unwrap_or(8081),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "localhost".to_string()),
            enable_https: env::var("ENABLE_HTTPS")
                .map(|v| v == "true")
                .unwrap_or(false),
            app_base_url: env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            ssl_cert_file: env::var("SSL_CERT_FILE")
                .unwrap_or_else(|_| "./certs/server.crt".to_string()),
            ssl_key_file: env::var("SSL_KEY_FILE")
                .unwrap_or_else(|_| "./certs/server.key".to_string()),

            // Database settings
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://links_sig:links_sig_password@localhost:5432/links_sig".to_string()
            }),
            meeting_lifecycle_interval_secs: env::var("MEETING_LIFECYCLE_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            meeting_lifecycle_lock_key: env::var("MEETING_LIFECYCLE_LOCK_KEY")
                .unwrap_or_else(|_| "424242".to_string())
                .parse()
                .unwrap_or(424242),

            // JWT settings
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-super-secret-jwt-key-change-in-production".to_string()),
            jwt_expiration_secs: env::var("JWT_EXPIRATION_SECS")
                .unwrap_or_else(|_| "604800".to_string()) // 7 days
                .parse()
                .unwrap_or(604800),
        }
    }

    /// Create a test configuration with default values
    ///
    /// This is intended for testing purposes only.
    #[allow(dead_code)]
    pub fn for_tests() -> Self {
        Self {
            livekit_url: "http://localhost:7880".to_string(),
            livekit_ws_url: "ws://127.0.0.1:7880".to_string(),
            api_key: "test-api-key".to_string(),
            api_secret: "test-api-secret".to_string(),
            server_port: 8081,
            server_host: "localhost".to_string(),
            enable_https: false,
            app_base_url: "http://localhost:3000".to_string(),
            ssl_cert_file: "./certs/server.crt".to_string(),
            ssl_key_file: "./certs/server.key".to_string(),
            database_url:
                "postgres://links_sig_test:links_sig_test_password@localhost:5433/links_sig_test"
                    .to_string(),
            meeting_lifecycle_interval_secs: 60,
            meeting_lifecycle_lock_key: 424242,
            jwt_secret: "test-jwt-secret-key".to_string(),
            jwt_expiration_secs: 604800,
        }
    }
}
