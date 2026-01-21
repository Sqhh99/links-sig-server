//! Services module - Business logic layer
//!
//! This module contains business logic services:
//! - `auth_service`: Token generation, user state assembly
//! - `meeting_service`: Meeting/room operations, participant management

pub mod auth_service;
pub mod meeting_service;

pub use auth_service::AuthService;
pub use meeting_service::MeetingService;
