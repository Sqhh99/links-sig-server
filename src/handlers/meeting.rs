//! Meeting (room and participant) handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::services::MeetingService;
use crate::state::AppState;
use crate::types::{AppError, CreateRoomRequest};

/// List all active rooms
///
/// GET /api/rooms
///
/// Response: Array of Room objects
pub async fn handle_list_rooms(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let rooms = MeetingService::list_rooms(&*state.livekit).await?;
    Ok((StatusCode::OK, Json(rooms)))
}

/// Create a new room
///
/// POST /api/rooms
///
/// Request body:
/// ```json
/// {
///   "name": "my-room"
/// }
/// ```
///
/// Response: Room object (201 Created)
pub async fn handle_create_room(
    State(state): State<AppState>,
    Json(req): Json<CreateRoomRequest>,
) -> Result<impl IntoResponse, AppError> {
    let room = MeetingService::create_room(&*state.livekit, req).await?;
    Ok((StatusCode::CREATED, Json(room)))
}

/// Delete a room
///
/// DELETE /api/rooms/{room_name}
///
/// Response: MessageResponse
pub async fn handle_delete_room(
    State(state): State<AppState>,
    Path(room_name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let response = MeetingService::delete_room(&*state.livekit, &room_name).await?;
    Ok((StatusCode::OK, Json(response)))
}

/// List participants in a room
///
/// GET /api/rooms/{room_name}/participants
///
/// Response: ListParticipantsResponse
pub async fn handle_list_participants(
    State(state): State<AppState>,
    Path(room_name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let participants = MeetingService::list_participants(&*state.livekit, &room_name).await?;
    Ok((StatusCode::OK, Json(participants)))
}

/// Kick a participant from a room
///
/// DELETE /api/rooms/{room_name}/participants/{identity}
///
/// Response: MessageResponse
pub async fn handle_kick_participant(
    State(state): State<AppState>,
    Path((room_name, identity)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let response =
        MeetingService::kick_participant(&*state.livekit, &room_name, &identity).await?;
    Ok((StatusCode::OK, Json(response)))
}

/// End a meeting (kick all participants and delete room)
///
/// POST /api/rooms/{room_name}/end
///
/// Response: MessageResponse
pub async fn handle_end_room(
    State(state): State<AppState>,
    Path(room_name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let response = MeetingService::end_meeting(&*state.livekit, &room_name).await?;
    Ok((StatusCode::OK, Json(response)))
}
