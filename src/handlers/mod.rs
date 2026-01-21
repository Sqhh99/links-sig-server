//! Handlers module - HTTP request handlers
//!
//! This module contains all HTTP handlers organized by domain:
//! - `health`: Health check and status endpoints
//! - `auth`: Authentication and token endpoints
//! - `meeting`: Room and participant management endpoints

pub mod auth;
pub mod health;
pub mod meeting;

pub use auth::*;
pub use health::*;
pub use meeting::*;
