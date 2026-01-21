//! Authentication handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::services::{AuthService, UserAuthService};
use crate::state::AppState;
use crate::types::{
    AppError, LoginRequest, RegisterRequest, RequestRegisterCodeRequest, TokenRequest,
};

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

/// Request verification code for registration
///
/// POST /api/auth/register/request-code
///
/// Request body:
/// ```json
/// {
///   "email": "user@example.com"
/// }
/// ```
///
/// Response (200 OK):
/// ```json
/// {
///   "message": "Verification code sent to your email",
///   "retryAfterSecs": 60
/// }
/// ```
///
/// Errors:
/// - 400: Invalid email format
/// - 409: Email already registered
/// - 429: Too many requests (rate limited)
pub async fn handle_request_register_code(
    State(state): State<AppState>,
    Json(req): Json<RequestRegisterCodeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response =
        UserAuthService::request_register_code(&state.db, &*state.email, &state.config, req)
            .await?;
    Ok((StatusCode::OK, Json(response)))
}

/// Complete registration with verification code
///
/// POST /api/auth/register
///
/// Request body:
/// ```json
/// {
///   "email": "user@example.com",
///   "code": "123456",
///   "password": "securePassword123"
/// }
/// ```
///
/// Response (201 Created):
/// ```json
/// {
///   "userId": "550e8400-e29b-41d4-a716-446655440000",
///   "email": "user@example.com",
///   "token": "eyJ..."
/// }
/// ```
///
/// Errors:
/// - 400: Invalid email, weak password, or invalid/expired code
/// - 409: Email already registered
pub async fn handle_register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = UserAuthService::register(&state.db, &state.config, req).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Login with email and password
///
/// POST /api/auth/login
///
/// Request body:
/// ```json
/// {
///   "email": "user@example.com",
///   "password": "securePassword123"
/// }
/// ```
///
/// Response (200 OK):
/// ```json
/// {
///   "userId": "550e8400-e29b-41d4-a716-446655440000",
///   "email": "user@example.com",
///   "token": "eyJ..."
/// }
/// ```
///
/// Errors:
/// - 401: Invalid email or password
pub async fn handle_login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = UserAuthService::login(&state.db, &state.config, req).await?;
    Ok((StatusCode::OK, Json(response)))
}
