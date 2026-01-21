//! Application state - Shared dependencies across handlers
//!
//! Contains the AppState struct that holds all shared resources
//! like configuration, LiveKit client, etc.

use std::sync::Arc;

use crate::config::Config;
use crate::integrations::{LiveKitClient, LiveKitService};

/// Application state shared across all handlers
///
/// This struct holds all shared dependencies that handlers need access to.
/// It is cloned for each request (Arc ensures cheap cloning).
#[derive(Clone)]
pub struct AppState {
    /// Server configuration
    pub config: Arc<Config>,
    /// LiveKit client for room operations
    pub livekit: Arc<dyn LiveKitService>,
}

impl AppState {
    /// Create a new AppState with the given configuration
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        let livekit_client = LiveKitClient::new(config.clone());

        Self {
            config,
            livekit: Arc::new(livekit_client),
        }
    }

    /// Create AppState with a custom LiveKit service (for testing)
    pub fn with_livekit<L: LiveKitService + 'static>(config: Config, livekit: L) -> Self {
        Self {
            config: Arc::new(config),
            livekit: Arc::new(livekit),
        }
    }
}

/// Test configuration builder
impl Config {
    /// Create a test configuration with default values
    ///
    /// This is intended for testing purposes only.
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
        }
    }
}
