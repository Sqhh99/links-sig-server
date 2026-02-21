//! Token endpoint integration tests
//!
//! Tests for POST /token
//!
//! These tests require a running test database. Run with:
//! ```
//! docker compose up -d postgres-test
//! cargo test --test token
//! ```

mod support;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use links_sig_rust_server::types::LiveKitRoom;
use tower::ServiceExt;

use support::{
    body_to_json, build_test_app, build_test_app_with_state, build_test_state_with_livekit,
    run_test_migrations, setup_test_db, FakeLiveKitService,
};

async fn setup() -> sqlx::PgPool {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    db
}

fn build_fake_room(name: &str) -> LiveKitRoom {
    LiveKitRoom {
        sid: Some(format!("RM_{}", name)),
        name: name.to_string(),
        empty_timeout: Some(300),
        max_participants: Some(10),
        creation_time: Some(1234567890),
        turn_password: None,
        enabled_codecs: None,
        metadata: Some(String::new()),
        num_participants: Some(0),
        num_publishers: Some(0),
        active_recording: Some(false),
    }
}

// ============================================================================
// POST /token
// ============================================================================

#[tokio::test]
async fn test_get_token_returns_200() {
    let db = setup().await;
    let fake_livekit = FakeLiveKitService::new().with_room(build_fake_room("test-room"));
    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"roomName": "test-room", "participantName": "test-user", "isHost": false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_token_returns_valid_json() {
    let db = setup().await;
    let fake_livekit = FakeLiveKitService::new().with_room(build_fake_room("test-room"));
    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"roomName": "test-room", "participantName": "test-user", "isHost": false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;

    // Verify response structure
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(
        json.get("token").is_some(),
        "Response should have 'token' field"
    );
    assert!(
        json.get("url").is_some(),
        "Response should have 'url' field"
    );
    assert!(
        json.get("roomName").is_some(),
        "Response should have 'roomName' field"
    );
    assert!(
        json.get("isHost").is_some(),
        "Response should have 'isHost' field"
    );

    // Verify token is a non-empty string (JWT format)
    let token = json["token"].as_str().unwrap();
    assert!(!token.is_empty(), "Token should not be empty");
    assert!(
        token.split('.').count() == 3,
        "Token should be in JWT format (3 parts separated by dots)"
    );
}

#[tokio::test]
async fn test_get_token_always_returns_non_host_for_guest() {
    let db = setup().await;
    let fake_livekit = FakeLiveKitService::new().with_room(build_fake_room("test-room"));
    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"roomName": "test-room", "participantName": "host-user", "isHost": true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;

    // Guest token endpoint should never grant host
    assert_eq!(json["isHost"], false, "isHost should always be false");
}

#[tokio::test]
async fn test_get_token_without_body_returns_error() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 4xx error for missing body
    assert!(
        response.status().is_client_error(),
        "Should return client error for missing request body"
    );
}

#[tokio::test]
async fn test_get_token_with_invalid_json_returns_error() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{ invalid json }"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 4xx error for invalid JSON
    assert!(
        response.status().is_client_error(),
        "Should return client error for invalid JSON"
    );
}

#[tokio::test]
async fn test_get_token_without_room_name_returns_400() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["error"], "roomName is required");
}

#[tokio::test]
async fn test_get_token_for_nonexistent_room_returns_404() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"roomName": "nonexistent-room", "participantName": "guest"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["error"], "Room not found");
}

#[tokio::test]
async fn test_get_token_for_business_meeting_room_returns_403() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"roomName": "m-123456789", "participantName": "guest"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let json = body_to_json(response.into_body()).await;
    assert_eq!(
        json["error"],
        "Business meetings must be joined via /api/meetings/{meeting_no}/join"
    );
}
