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
    // JWT Settings (User Auth)
    // ========================================================================
    /// JWT secret for user access tokens (separate from LiveKit API secret)
    pub jwt_secret: String,
    /// JWT token expiration in seconds (default: 7 days)
    pub jwt_expiration_secs: u64,

    // ========================================================================
    // Verification Code Settings
    // ========================================================================
    /// HMAC secret for verification code hashing
    pub code_hmac_secret: String,
    /// Verification code length (default: 6)
    pub code_length: usize,
    /// Rate limit: minimum seconds between sending codes (default: 60)
    pub code_rate_limit_secs: u64,
    /// Code expiration in seconds (default: 600 = 10 minutes)
    pub code_expiration_secs: u64,

    // ========================================================================
    // SMTP Settings
    // ========================================================================
    /// SMTP host
    pub smtp_host: String,
    /// SMTP port
    pub smtp_port: u16,
    /// SMTP sender email address
    pub smtp_sender: String,
    /// SMTP password or API key
    pub smtp_password: String,
    /// Use SSL/TLS for SMTP
    pub smtp_use_ssl: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            // LiveKit settings
            livekit_url: env::var("LIVEKIT_URL")
                .unwrap_or_else(|_| "http://localhost:7880".to_string()),
            livekit_ws_url: env::var("LIVEKIT_WS_URL")
                .unwrap_or_else(|_| "ws://localhost:7880".to_string()),
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
            ssl_cert_file: env::var("SSL_CERT_FILE")
                .unwrap_or_else(|_| "./certs/server.crt".to_string()),
            ssl_key_file: env::var("SSL_KEY_FILE")
                .unwrap_or_else(|_| "./certs/server.key".to_string()),

            // Database settings
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://links_sig:links_sig_password@localhost:5432/links_sig".to_string()),

            // JWT settings
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-super-secret-jwt-key-change-in-production".to_string()),
            jwt_expiration_secs: env::var("JWT_EXPIRATION_SECS")
                .unwrap_or_else(|_| "604800".to_string()) // 7 days
                .parse()
                .unwrap_or(604800),

            // Verification code settings
            code_hmac_secret: env::var("CODE_HMAC_SECRET")
                .unwrap_or_else(|_| "your-super-secret-hmac-key-change-in-production".to_string()),
            code_length: env::var("CODE_LENGTH")
                .unwrap_or_else(|_| "6".to_string())
                .parse()
                .unwrap_or(6),
            code_rate_limit_secs: env::var("CODE_RATE_LIMIT_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            code_expiration_secs: env::var("CODE_EXPIRATION_SECS")
                .unwrap_or_else(|_| "600".to_string()) // 10 minutes
                .parse()
                .unwrap_or(600),

            // SMTP settings
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.example.com".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            smtp_sender: env::var("SMTP_SENDER")
                .unwrap_or_else(|_| "noreply@example.com".to_string()),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_else(|_| "".to_string()),
            smtp_use_ssl: env::var("SMTP_USE_SSL")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }

    /// Create a test configuration with default values
    ///
    /// This is intended for testing purposes only.
    #[allow(dead_code)]
    pub fn for_tests() -> Self {
        Self {
            livekit_url: "http://localhost:7880".to_string(),
            livekit_ws_url: "ws://localhost:7880".to_string(),
            api_key: "test-api-key".to_string(),
            api_secret: "test-api-secret".to_string(),
            server_port: 8081,
            server_host: "localhost".to_string(),
            enable_https: false,
            ssl_cert_file: "./certs/server.crt".to_string(),
            ssl_key_file: "./certs/server.key".to_string(),
            database_url: "postgres://links_sig_test:links_sig_test_password@localhost:5433/links_sig_test".to_string(),
            jwt_secret: "test-jwt-secret-key".to_string(),
            jwt_expiration_secs: 604800,
            code_hmac_secret: "test-hmac-secret-key".to_string(),
            code_length: 6,
            code_rate_limit_secs: 60,
            code_expiration_secs: 600,
            smtp_host: "smtp.test.local".to_string(),
            smtp_port: 587,
            smtp_sender: "noreply@test.local".to_string(),
            smtp_password: "test-password".to_string(),
            smtp_use_ssl: false,
        }
    }
}
