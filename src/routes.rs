//! Routes module - Centralized router configuration
//!
//! All API routes are defined here and organized by domain.

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::handlers::{
    handle_create_meeting, handle_create_room, handle_delete_room, handle_end_room,
    handle_get_token, handle_health, handle_join_meeting, handle_kick_participant,
    handle_list_my_meeting_records, handle_list_participants, handle_list_rooms, handle_login,
    handle_register, handle_request_register_code,
};
use crate::state::AppState;

/// Build the API router with all routes
///
/// Route structure:
/// - POST /api/token - Generate LiveKit access token
/// - GET /api/rooms - List all rooms
/// - POST /api/rooms - Create a new room
/// - DELETE /api/rooms/{room_name} - Delete a room
/// - GET /api/rooms/{room_name}/participants - List participants
/// - DELETE /api/rooms/{room_name}/participants/{identity} - Kick participant
/// - POST /api/rooms/{room_name}/end - End meeting
/// - GET /api/health - Health check
/// - POST /api/auth/register/request-code - Request verification code
/// - POST /api/auth/register - Complete registration
/// - POST /api/auth/login - Login
/// - POST /api/meetings - Create meeting with meeting number
/// - POST /api/meetings/{meeting_no}/join - Join meeting by meeting number
/// - GET /api/me/meeting-records - List current user's records
pub fn build_api_router() -> Router<AppState> {
    // LiveKit token route
    let token_routes = Router::new().route("/token", post(handle_get_token));

    // User auth routes
    let user_auth_routes = Router::new()
        .route("/register/request-code", post(handle_request_register_code))
        .route("/register", post(handle_register))
        .route("/login", post(handle_login));

    // Room/meeting routes
    let room_routes = Router::new()
        .route("/", get(handle_list_rooms))
        .route("/", post(handle_create_room))
        .route("/{room_name}", delete(handle_delete_room))
        .route("/{room_name}/participants", get(handle_list_participants))
        .route(
            "/{room_name}/participants/{identity}",
            delete(handle_kick_participant),
        )
        .route("/{room_name}/end", post(handle_end_room));

    // Business meeting routes
    let meeting_routes = Router::new()
        .route("/", post(handle_create_meeting))
        .route("/{meeting_no}/join", post(handle_join_meeting));

    // Current user routes
    let me_routes = Router::new().route("/meeting-records", get(handle_list_my_meeting_records));

    // Health routes
    let health_routes = Router::new().route("/", get(handle_health));

    // Combine all routes under /api
    Router::new()
        .merge(token_routes)
        .nest("/auth", user_auth_routes)
        .nest("/rooms", room_routes)
        .nest("/meetings", meeting_routes)
        .nest("/me", me_routes)
        .nest("/health", health_routes)
}
