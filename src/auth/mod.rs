//! Auth module - JWT handling and request authentication
//!
//! This module contains:
//! - `jwt`: JWT token encoding/decoding for LiveKit access tokens
//! - `user_jwt`: JWT token encoding/decoding for user authentication
//! - `extractor`: Axum extractor for Bearer token authentication

pub mod extractor;
pub mod jwt;
pub mod user_jwt;

// Re-exports for external use (some may be used only in tests or future features)
#[allow(unused_imports)]
pub use extractor::OptionalAuth;
#[allow(unused_imports)]
pub use jwt::{AccessToken, AccessTokenClaims, VideoGrant};
#[allow(unused_imports)]
pub use user_jwt::{decode_user_token, encode_user_token, extract_user_id, UserAccessTokenClaims};
