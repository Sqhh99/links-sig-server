//! Meeting (room and participant) handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use uuid::Uuid;

use crate::auth::decode_user_token;
use crate::services::{MeetingRegistryService, MeetingService};
use crate::state::AppState;
use crate::types::{AppError, CreateRoomRequest, JoinMeetingRequest, MeetingRecordsQuery};

/// List all active rooms
///
/// GET /api/rooms
///
/// Response: Array of Room objects
pub async fn handle_list_rooms(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
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
    let response = MeetingService::kick_participant(&*state.livekit, &room_name, &identity).await?;
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

/// Create a business meeting with a 9-digit meeting number
///
/// POST /api/meetings
pub async fn handle_create_meeting(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<impl IntoResponse, AppError> {
    let (user_id, _email) = parse_user_from_auth_header(auth_header, &state)?;
    let response =
        MeetingRegistryService::create_meeting(&state.db, &state.config, user_id, None).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Join a meeting by 9-digit meeting number
///
/// POST /api/meetings/{meeting_no}/join
pub async fn handle_join_meeting(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    Path(meeting_no): Path<String>,
    Json(req): Json<JoinMeetingRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (user_id, user_email) = parse_user_from_auth_header(auth_header, &state)?;
    let response = MeetingRegistryService::join_meeting(
        &state.db,
        &state.config,
        &meeting_no,
        user_id,
        &user_email,
        req,
    )
    .await?;
    Ok((StatusCode::OK, Json(response)))
}

/// List current user's meeting records
///
/// GET /api/me/meeting-records
pub async fn handle_list_my_meeting_records(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    Query(query): Query<MeetingRecordsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let (user_id, _email) = parse_user_from_auth_header(auth_header, &state)?;
    let response =
        MeetingRegistryService::list_user_records(&state.db, user_id, query.page, query.page_size)
            .await?;
    Ok((StatusCode::OK, Json(response)))
}

fn parse_user_from_auth_header(
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    state: &AppState,
) -> Result<(Uuid, String), AppError> {
    let TypedHeader(auth) =
        auth_header.ok_or_else(|| AppError::unauthorized("Authorization header required"))?;

    let token = auth.token();

    let claims = decode_user_token(token, &state.config.jwt_secret)?;
    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("Invalid user token"))?;

    Ok((user_id, claims.email))
}
