//! Authentication handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::auth::decode_user_token;
use crate::services::{AuthService, UserAuthService};
use crate::state::AppState;
use crate::types::{AppError, LoginRequest, TokenRequest};

// ============================================================================
// LiveKit Token Handler
// ============================================================================

/// Generate LiveKit access token
///
/// POST /api/token
///
/// Request body:
/// ```json
/// {
///   "roomName": "my-room",
///   "participantName": "John",
///   "isHost": false
/// }
/// ```
///
/// Response:
/// ```json
/// {
///   "token": "eyJ...",
///   "url": "wss://livekit.example.com",
///   "roomName": "my-room",
///   "isHost": true
/// }
/// ```
pub async fn handle_get_token(
    State(state): State<AppState>,
    Json(req): Json<TokenRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = AuthService::generate_token(&state.config, &*state.livekit, req).await?;
    Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// User Authentication Handlers
// ============================================================================

/// Log in with username and password
///
/// POST /api/auth/login
///
/// The first login with an unused username creates the account, so there is
/// no separate registration endpoint.
///
/// Request body:
/// ```json
/// {
///   "username": "alice",
///   "password": "securePassword123"
/// }
/// ```
///
/// Response (200 OK for an existing account, 201 Created for a new one):
/// ```json
/// {
///   "userId": "550e8400-e29b-41d4-a716-446655440000",
///   "username": "alice",
///   "token": "eyJ...",
///   "accountCreated": false
/// }
/// ```
///
/// Errors:
/// - 400: Missing fields, or a new account whose username or password breaks the rules
/// - 401: The username exists and the password does not match
pub async fn handle_login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = UserAuthService::login(&state.db, &state.config, req).await?;
    let status = if response.account_created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(response)))
}

/// Refresh user JWT token
///
/// POST /api/auth/refresh
///
/// Headers:
/// Authorization: Bearer <user-jwt>
///
/// Response (200 OK):
/// ```json
/// {
///   "userId": "550e8400-e29b-41d4-a716-446655440000",
///   "username": "alice",
///   "token": "eyJ...",
///   "expiresInSecs": 604800
/// }
/// ```
pub async fn handle_refresh_token(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<impl IntoResponse, AppError> {
    let TypedHeader(auth) =
        auth_header.ok_or_else(|| AppError::unauthorized("Authorization header required"))?;

    let claims = decode_user_token(auth.token(), &state.config.jwt_secret)?;

    let response = UserAuthService::refresh_token(&state.db, &state.config, &claims.sub).await?;
    Ok((StatusCode::OK, Json(response)))
}
