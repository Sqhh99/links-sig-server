use std::env;

/// Server configuration
#[derive(Clone, Debug)]
pub struct Config {
    /// LiveKit API URL (HTTP/HTTPS)
    pub livekit_url: String,
    /// LiveKit WebSocket URL for clients
    pub livekit_ws_url: String,
    /// LiveKit API Key
    pub api_key: String,
    /// LiveKit API Secret
    pub api_secret: String,
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
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            livekit_url: env::var("LIVEKIT_URL")
                .unwrap_or_else(|_| "http://localhost:7880".to_string()),
            livekit_ws_url: env::var("LIVEKIT_WS_URL")
                .unwrap_or_else(|_| "ws://localhost:7880".to_string()),
            api_key: env::var("LIVEKIT_API_KEY")
                .unwrap_or_else(|_| "devkey".to_string()),
            api_secret: env::var("LIVEKIT_API_SECRET")
                .unwrap_or_else(|_| "secret".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .unwrap_or(8081),
            server_host: env::var("SERVER_HOST")
                .unwrap_or_else(|_| "localhost".to_string()),
            enable_https: env::var("ENABLE_HTTPS")
                .map(|v| v == "true")
                .unwrap_or(false),
            ssl_cert_file: env::var("SSL_CERT_FILE")
                .unwrap_or_else(|_| "./certs/server.crt".to_string()),
            ssl_key_file: env::var("SSL_KEY_FILE")
                .unwrap_or_else(|_| "./certs/server.key".to_string()),
        }
    }
}
