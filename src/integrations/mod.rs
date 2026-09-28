//! Integrations module - External service adapters
//!
//! This module contains adapters for external services:
//! - `livekit`: LiveKit API client with trait abstraction for testing
//! - `db`: PostgreSQL database connection pool

pub mod db;
pub mod livekit;

pub use db::{create_pool, run_migrations};
pub use livekit::{LiveKitClient, LiveKitService};
