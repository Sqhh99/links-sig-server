//! Application state - Shared dependencies across handlers
//!
//! Contains the AppState struct that holds all shared resources
//! like configuration, LiveKit client, database pool, email sender, etc.

use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::integrations::{EmailSender, LiveKitClient, LiveKitService};

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
    /// Database connection pool
    pub db: PgPool,
    /// Email sender for verification codes
    pub email: Arc<dyn EmailSender>,
}

impl AppState {
    /// Create a new AppState with all dependencies
    pub fn new(
        config: Config,
        db: PgPool,
        email: Arc<dyn EmailSender>,
    ) -> Self {
        let config = Arc::new(config);
        let livekit_client = LiveKitClient::new(config.clone());

        Self {
            config,
            livekit: Arc::new(livekit_client),
            db,
            email,
        }
    }

    /// Create AppState with custom services (for testing)
    #[allow(dead_code)]
    pub fn with_services<L, E>(
        config: Config,
        db: PgPool,
        livekit: L,
        email: E,
    ) -> Self
    where
        L: LiveKitService + 'static,
        E: EmailSender + 'static,
    {
        Self {
            config: Arc::new(config),
            livekit: Arc::new(livekit),
            db,
            email: Arc::new(email),
        }
    }

    /// Create AppState with a custom LiveKit service (for testing)
    #[allow(dead_code)]
    pub fn with_livekit<L: LiveKitService + 'static>(
        config: Config,
        db: PgPool,
        livekit: L,
        email: Arc<dyn EmailSender>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            livekit: Arc::new(livekit),
            db,
            email,
        }
    }
}
