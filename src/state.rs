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
    #[allow(dead_code)]
    pub fn with_livekit<L: LiveKitService + 'static>(config: Config, livekit: L) -> Self {
        Self {
            config: Arc::new(config),
            livekit: Arc::new(livekit),
        }
    }
}
