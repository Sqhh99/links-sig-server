//! Routes module - Centralized router configuration
//!
//! All API routes are defined here and organized by domain.

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::handlers::{
    handle_cancel_meeting, handle_create_meeting, handle_create_room, handle_delete_room,
    handle_end_room, handle_get_token, handle_guest_join_meeting, handle_health,
    handle_join_meeting, handle_kick_participant, handle_leave_meeting,
    handle_list_my_host_meetings, handle_list_my_meeting_records, handle_list_participants,
    handle_list_rooms, handle_login, handle_refresh_token,
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
/// - POST /api/auth/login - Login (creates the account on first use)
/// - POST /api/auth/refresh - Refresh user token
/// - POST /api/meetings - Create meeting with meeting number
/// - POST /api/meetings/{meeting_no}/join - Join meeting by meeting number
/// - POST /api/meetings/{meeting_no}/guest-join - Guest join by meeting number
/// - POST /api/meetings/{meeting_no}/leave - Leave meeting by meeting number
/// - POST /api/meetings/{meeting_no}/cancel - Cancel scheduled meeting
/// - GET /api/me/meeting-records - List current user's records
/// - GET /api/me/host-meetings - List current host's created meetings
pub fn build_api_router() -> Router<AppState> {
    // LiveKit token route
    let token_routes = Router::new().route("/token", post(handle_get_token));

    // User auth routes
    let user_auth_routes = Router::new()
        .route("/login", post(handle_login))
        .route("/refresh", post(handle_refresh_token));

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
        .route("/{meeting_no}/join", post(handle_join_meeting))
        .route("/{meeting_no}/guest-join", post(handle_guest_join_meeting))
        .route("/{meeting_no}/leave", post(handle_leave_meeting))
        .route("/{meeting_no}/cancel", post(handle_cancel_meeting));

    // Current user routes
    let me_routes = Router::new()
        .route("/meeting-records", get(handle_list_my_meeting_records))
        .route("/host-meetings", get(handle_list_my_host_meetings));

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
