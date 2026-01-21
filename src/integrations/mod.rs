//! Integrations module - External service adapters
//!
//! This module contains adapters for external services:
//! - `livekit`: LiveKit API client with trait abstraction for testing

pub mod livekit;

pub use livekit::{LiveKitClient, LiveKitService};
