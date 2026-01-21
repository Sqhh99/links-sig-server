//! Authentication handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::services::AuthService;
use crate::state::AppState;
use crate::types::{AppError, TokenRequest};

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
