//! Services module - Business logic layer
//!
//! This module contains business logic services:
//! - `auth_service`: LiveKit token generation
//! - `user_auth_service`: User registration, login, verification
//! - `meeting_service`: Meeting/room operations, participant management
//! - `meeting_registry_service`: Business meeting numbers and meeting records
//! - `meeting_lifecycle_service`: Automatic scheduled/open meeting transitions

pub mod auth_service;
pub mod meeting_lifecycle_service;
pub mod meeting_registry_service;
pub mod meeting_service;
pub mod user_auth_service;

pub use auth_service::AuthService;
pub use meeting_lifecycle_service::MeetingLifecycleService;
pub use meeting_registry_service::MeetingRegistryService;
pub use meeting_service::MeetingService;
pub use user_auth_service::UserAuthService;
