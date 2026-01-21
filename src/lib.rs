//! LiveKit Signaling Server Library
//!
//! This module exposes the core components for integration testing.
//! For production, use the binary entry point (main.rs).

pub mod auth;
pub mod config;
pub mod handlers;
pub mod integrations;
pub mod routes;
pub mod services;
pub mod state;
pub mod types;

// Re-export commonly used items for convenience
pub use config::Config;
pub use routes::build_api_router;
pub use state::AppState;

// Re-export integrations for testing
pub use integrations::{EmailSender, FakeEmailSender};
