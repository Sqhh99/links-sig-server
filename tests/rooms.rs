//! Rooms endpoint integration tests
//!
//! Tests for:
//! - GET /rooms/
//! - POST /rooms/
//! - DELETE /rooms/{room_name}
//! - GET /rooms/{room_name}/participants
//! - DELETE /rooms/{room_name}/participants/{identity}
//! - POST /rooms/{room_name}/end
//!
//! These tests require a running test database. Run with:
//! ```
//! docker compose up -d postgres-test
//! cargo test --test rooms
//! ```

mod support;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;

use support::{body_to_json, build_test_app, build_test_state_with_livekit, build_test_app_with_state, FakeLiveKitService, setup_test_db, run_test_migrations};
use links_sig_rust_server::types::{LiveKitRoom, LiveKitParticipant};

async fn setup() -> sqlx::PgPool {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    db
}

// ============================================================================
// GET /rooms/
// ============================================================================

#[tokio::test]
async fn test_list_rooms_returns_200() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/rooms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_rooms_returns_empty_array() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/rooms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    
    // Response should be a JSON array
    assert!(json.is_array(), "Response should be a JSON array");
}

#[tokio::test]
async fn test_list_rooms_with_existing_rooms() {
    let db = setup().await;
    let fake_livekit = FakeLiveKitService::new().with_room(LiveKitRoom {
        sid: Some("RM_test123".to_string()),
        name: "test-room".to_string(),
        empty_timeout: Some(300),
        max_participants: Some(10),
        creation_time: Some(1234567890),
        turn_password: None,
        enabled_codecs: None,
        metadata: Some(String::new()),
        num_participants: Some(2),
        num_publishers: Some(1),
        active_recording: Some(false),
    });

    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/rooms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    
    assert!(json.is_array(), "Response should be a JSON array");
    assert_eq!(json.as_array().unwrap().len(), 1, "Should have one room");
    
    // Verify room structure (example assertion - adjust based on actual response)
    let room = &json[0];
    assert!(room.get("name").is_some(), "Room should have 'name' field");
}

// ============================================================================
// POST /rooms/
// ============================================================================

#[tokio::test]
async fn test_create_room_returns_201() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rooms")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name": "new-test-room"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_room_returns_valid_json() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rooms")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name": "new-test-room"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let json = body_to_json(response.into_body()).await;
    
    // Verify response is valid JSON object
    assert!(json.is_object(), "Response should be a JSON object");
    
    // Room response should have standard LiveKit room fields
    // Adjust these assertions based on your actual response structure
    assert!(json.get("name").is_some() || json.get("sid").is_some(), 
        "Room response should have identifying fields");
}

#[tokio::test]
async fn test_create_room_without_body_returns_error() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rooms")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 4xx error for missing/invalid body
    assert!(
        response.status().is_client_error(),
        "Should return client error for invalid request"
    );
}

// ============================================================================
// DELETE /rooms/{room_name}
// ============================================================================

#[tokio::test]
async fn test_delete_room_returns_200() {
    let db = setup().await;
    // First create a room, then delete it
    let fake_livekit = FakeLiveKitService::new().with_room(LiveKitRoom {
        sid: Some("RM_delete".to_string()),
        name: "room-to-delete".to_string(),
        empty_timeout: Some(300),
        max_participants: Some(10),
        creation_time: Some(1234567890),
        turn_password: None,
        enabled_codecs: None,
        metadata: Some(String::new()),
        num_participants: Some(0),
        num_publishers: Some(0),
        active_recording: Some(false),
    });

    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/rooms/room-to-delete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    
    // Should return a message response
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(json.get("message").is_some(), "Response should have 'message' field");
}

// ============================================================================
// GET /rooms/{room_name}/participants
// ============================================================================

#[tokio::test]
async fn test_list_participants_returns_200() {
    let db = setup().await;
    let fake_livekit = FakeLiveKitService::new()
        .with_room(LiveKitRoom {
            sid: Some("RM_participants".to_string()),
            name: "room-with-participants".to_string(),
            empty_timeout: Some(300),
            max_participants: Some(10),
            creation_time: Some(1234567890),
            turn_password: None,
            enabled_codecs: None,
            metadata: Some(String::new()),
            num_participants: Some(1),
            num_publishers: Some(0),
            active_recording: Some(false),
        })
        .with_participant("room-with-participants", LiveKitParticipant {
            sid: Some("PA_test".to_string()),
            identity: "test-user".to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("Test User".to_string()),
            version: Some(1),
            permission: None,
            region: None,
            is_publisher: Some(false),
        });

    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/rooms/room-with-participants/participants")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    
    // Should have participants array
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(json.get("participants").is_some(), "Response should have 'participants' field");
}

// ============================================================================
// POST /rooms/{room_name}/end
// ============================================================================

#[tokio::test]
async fn test_end_room_returns_200() {
    let db = setup().await;
    let fake_livekit = FakeLiveKitService::new().with_room(LiveKitRoom {
        sid: Some("RM_end".to_string()),
        name: "room-to-end".to_string(),
        empty_timeout: Some(300),
        max_participants: Some(10),
        creation_time: Some(1234567890),
        turn_password: None,
        enabled_codecs: None,
        metadata: Some(String::new()),
        num_participants: Some(0),
        num_publishers: Some(0),
        active_recording: Some(false),
    });

    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rooms/room-to-end/end")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    
    // Should return a message response
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(json.get("message").is_some(), "Response should have 'message' field");
}
