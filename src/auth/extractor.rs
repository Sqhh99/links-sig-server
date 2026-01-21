//! Axum extractor for Bearer token authentication
//!
//! Provides `OptionalAuth` extractor that parses Authorization header
//! and validates the JWT token, injecting user info into handlers.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::state::AppState;
use crate::types::ErrorResponse;

use super::jwt::{decode_token, AccessTokenClaims};

/// Authenticated user information extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User identity (from JWT sub claim)
    pub identity: String,
    /// Full claims from the JWT
    pub claims: AccessTokenClaims,
}

/// Optional authentication extractor
///
/// This extractor attempts to parse and validate a Bearer token from
/// the Authorization header. If no token is present or validation fails,
/// `None` is returned instead of rejecting the request.
///
/// Use this for endpoints that support both authenticated and guest access.
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

/// Error type for auth extraction
pub struct AuthError {
    message: String,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl FromRequestParts<AppState> for OptionalAuth {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Try to extract Authorization header
        let auth_header: Option<TypedHeader<Authorization<Bearer>>> =
            TypedHeader::from_request_parts(parts, state).await.ok();

        match auth_header {
            Some(TypedHeader(auth)) => {
                let token = auth.token();
                
                // Decode and validate token
                match decode_token(token, &state.config.api_secret) {
                    Ok(claims) => {
                        let user = AuthUser {
                            identity: claims.sub.clone(),
                            claims,
                        };
                        Ok(OptionalAuth(Some(user)))
                    }
                    Err(_) => {
                        // Token invalid, treat as no auth (guest)
                        Ok(OptionalAuth(None))
                    }
                }
            }
            None => {
                // No Authorization header, guest access
                Ok(OptionalAuth(None))
            }
        }
    }
}

/// Required authentication extractor
///
/// Similar to `OptionalAuth`, but rejects requests without valid authentication.
/// Use this for endpoints that require authentication.
#[derive(Debug, Clone)]
pub struct RequiredAuth(pub AuthUser);

impl FromRequestParts<AppState> for RequiredAuth {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Try to extract Authorization header
        let auth_header: Option<TypedHeader<Authorization<Bearer>>> =
            TypedHeader::from_request_parts(parts, state).await.ok();

        match auth_header {
            Some(TypedHeader(auth)) => {
                let token = auth.token();
                
                // Decode and validate token
                match decode_token(token, &state.config.api_secret) {
                    Ok(claims) => {
                        let user = AuthUser {
                            identity: claims.sub.clone(),
                            claims,
                        };
                        Ok(RequiredAuth(user))
                    }
                    Err(e) => Err(AuthError {
                        message: format!("Invalid token: {}", e),
                    }),
                }
            }
            None => Err(AuthError {
                message: "Authorization header required".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_debug() {
        // Simple test to ensure AuthUser can be debugged
        let claims = AccessTokenClaims {
            exp: 0,
            iat: None,
            nbf: 0,
            iss: "test".to_string(),
            sub: "user".to_string(),
            video: crate::auth::jwt::VideoGrant::default(),
            metadata: None,
            name: None,
        };

        let user = AuthUser {
            identity: "user".to_string(),
            claims,
        };

        // Should not panic
        let _ = format!("{:?}", user);
    }
}
