//! Auth module - JWT handling and request authentication
//!
//! This module contains:
//! - `jwt`: JWT token encoding/decoding, claims definition, VideoGrant
//! - `extractor`: Axum extractor for Bearer token authentication

pub mod extractor;
pub mod jwt;

pub use extractor::OptionalAuth;
pub use jwt::{AccessToken, AccessTokenClaims, VideoGrant};
