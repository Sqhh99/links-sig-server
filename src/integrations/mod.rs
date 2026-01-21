//! Integrations module - External service adapters
//!
//! This module contains adapters for external services:
//! - `livekit`: LiveKit API client with trait abstraction for testing
//! - `db`: PostgreSQL database connection pool
//! - `email`: Email sending with SMTP and mock implementations

pub mod db;
pub mod email;
pub mod livekit;

pub use db::{create_pool, run_migrations};
#[allow(unused_imports)]
pub use email::{EmailError, EmailSender, FakeEmailSender, SmtpEmailSender};
pub use livekit::{LiveKitClient, LiveKitService};

