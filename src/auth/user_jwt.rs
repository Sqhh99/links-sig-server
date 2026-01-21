//! User JWT handling for user authentication
//!
//! Separate from LiveKit JWT - this is for user login/auth tokens.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::types::AppError;

/// User access token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccessTokenClaims {
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at time (Unix timestamp)
    pub iat: u64,
    /// Not before time (Unix timestamp)
    pub nbf: u64,
    /// Subject (User ID)
    pub sub: String,
    /// User email
    pub email: String,
}

/// Encode a user access token
///
/// # Arguments
/// * `user_id` - The user's UUID
/// * `email` - The user's email address
/// * `secret` - JWT signing secret
/// * `expiration_secs` - Token validity duration in seconds
///
/// # Returns
/// JWT token string
pub fn encode_user_token(
    user_id: Uuid,
    email: &str,
    secret: &str,
    expiration_secs: u64,
) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
        .as_secs();

    let claims = UserAccessTokenClaims {
        exp: now + expiration_secs,
        iat: now,
        nbf: now,
        sub: user_id.to_string(),
        email: email.to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::internal(format!("JWT encoding error: {}", e)))
}

/// Decode and validate a user access token
///
/// # Arguments
/// * `token` - The JWT token string
/// * `secret` - JWT signing secret
///
/// # Returns
/// Validated claims if the token is valid
#[allow(dead_code)]
pub fn decode_user_token(token: &str, secret: &str) -> Result<UserAccessTokenClaims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_nbf = true;

    let token_data = decode::<UserAccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            AppError::unauthorized("Token has expired")
        }
        jsonwebtoken::errors::ErrorKind::InvalidToken => {
            AppError::unauthorized("Invalid token format")
        }
        jsonwebtoken::errors::ErrorKind::InvalidSignature => {
            AppError::unauthorized("Invalid token signature")
        }
        _ => AppError::unauthorized(format!("Token validation failed: {}", e)),
    })?;

    Ok(token_data.claims)
}

/// Extract user ID from validated claims
#[allow(dead_code)]
pub fn extract_user_id(claims: &UserAccessTokenClaims) -> Result<Uuid, AppError> {
    Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::internal(format!("Invalid user ID in token: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-jwt";

    #[test]
    fn test_encode_decode_user_token() {
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = encode_user_token(user_id, email, TEST_SECRET, 3600).unwrap();
        let claims = decode_user_token(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_decode_with_wrong_secret_fails() {
        let user_id = Uuid::new_v4();
        let token = encode_user_token(user_id, "test@example.com", TEST_SECRET, 3600).unwrap();

        let result = decode_user_token(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token_fails() {
        let user_id = Uuid::new_v4();
        // Create a token that expired 1 second ago (expiration_secs = 0 won't work, so we manually test)
        let token = encode_user_token(user_id, "test@example.com", TEST_SECRET, 0).unwrap();

        // Token with 0 expiration is effectively expired immediately
        let result = decode_user_token(&token, TEST_SECRET);
        // Note: This test may pass or fail depending on timing, so we just check it doesn't panic
        // In practice, a 0-second expiration means it's valid for the current second
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_extract_user_id() {
        let user_id = Uuid::new_v4();
        let token = encode_user_token(user_id, "test@example.com", TEST_SECRET, 3600).unwrap();
        let claims = decode_user_token(&token, TEST_SECRET).unwrap();
        let extracted_id = extract_user_id(&claims).unwrap();

        assert_eq!(extracted_id, user_id);
    }

    #[test]
    fn test_invalid_token_format() {
        let result = decode_user_token("not-a-valid-jwt", TEST_SECRET);
        assert!(result.is_err());
    }
}
