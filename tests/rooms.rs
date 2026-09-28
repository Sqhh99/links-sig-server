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
use uuid::Uuid;

use links_sig_rust_server::auth::encode_user_token;
use links_sig_rust_server::types::{LiveKitParticipant, LiveKitRoom};
use links_sig_rust_server::AppState;
use support::{
    body_to_json, build_test_app, build_test_app_with_state, build_test_config,
    build_test_state_with_livekit, run_test_migrations, setup_test_db, FakeLiveKitService,
};

async fn setup() -> sqlx::PgPool {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    db
}

async fn create_test_user(db: &sqlx::PgPool, username: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(username)
        .bind("test-password-hash")
        .execute(db)
        .await
        .unwrap();
    user_id
}

async fn insert_meeting(db: &sqlx::PgPool, meeting_no: &str, creator_user_id: Uuid) {
    let room_name = format!("m-{}", meeting_no);
    sqlx::query(
        r#"
        INSERT INTO meetings (
            meeting_no,
            room_name,
            creator_user_id,
            status,
            topic,
            scheduled_start_at,
            opened_at
        )
        VALUES ($1, $2, $3, 'open', '', NOW() - INTERVAL '1 minute', NOW())
        "#,
    )
    .bind(meeting_no)
    .bind(room_name)
    .bind(creator_user_id)
    .execute(db)
    .await
    .unwrap();
}

async fn get_meeting_status(db: &sqlx::PgPool, meeting_no: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT status FROM meetings WHERE meeting_no = $1")
        .bind(meeting_no)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn find_nonexistent_meeting_no(db: &sqlx::PgPool) -> String {
    for _ in 0..64 {
        let candidate = format!("{:09}", (Uuid::new_v4().as_u128() % 1_000_000_000) as u32);
        let exists =
            sqlx::query_scalar::<_, i64>("SELECT 1 FROM meetings WHERE meeting_no = $1 LIMIT 1")
                .bind(&candidate)
                .fetch_optional(db)
                .await
                .unwrap();

        if exists.is_none() {
            return candidate;
        }
    }

    panic!("Failed to find a non-existent meeting number for test");
}

fn build_user_token(user_id: Uuid, username: &str) -> String {
    let config = build_test_config();
    encode_user_token(user_id, username, &config.jwt_secret, 3600).unwrap()
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
    assert!(
        json.get("name").is_some() || json.get("sid").is_some(),
        "Room response should have identifying fields"
    );
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
    assert!(
        json.get("message").is_some(),
        "Response should have 'message' field"
    );
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
        .with_participant(
            "room-with-participants",
            LiveKitParticipant {
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
            },
        );

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
    assert!(
        json.get("participants").is_some(),
        "Response should have 'participants' field"
    );
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
    assert!(
        json.get("message").is_some(),
        "Response should have 'message' field"
    );
}

#[tokio::test]
async fn test_kick_participant_business_room_requires_host() {
    let db = setup().await;

    let host_username = format!("host_{}", Uuid::new_v4());
    let guest_username = format!("guest_{}", Uuid::new_v4());
    let host_user_id = create_test_user(&db, &host_username).await;
    let guest_user_id = create_test_user(&db, &guest_username).await;
    let guest_token = build_user_token(guest_user_id, &guest_username);
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    let room_name = format!("m-{}", meeting_no);
    insert_meeting(&db, &meeting_no, host_user_id).await;

    let fake_livekit = FakeLiveKitService::new().with_participant(
        &room_name,
        LiveKitParticipant {
            sid: Some("PA_guest".to_string()),
            identity: guest_user_id.to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("Guest".to_string()),
            version: Some(1),
            permission: None,
            region: None,
            is_publisher: Some(false),
        },
    );

    let state = AppState::with_livekit(build_test_config(), db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/rooms/{}/participants/{}",
                    room_name, guest_user_id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", guest_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_kick_participant_business_room_host_allowed() {
    let db = setup().await;

    let host_username = format!("host2_{}", Uuid::new_v4());
    let guest_username = format!("guest2_{}", Uuid::new_v4());
    let host_user_id = create_test_user(&db, &host_username).await;
    let guest_user_id = create_test_user(&db, &guest_username).await;
    let host_token = build_user_token(host_user_id, &host_username);
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    let room_name = format!("m-{}", meeting_no);
    insert_meeting(&db, &meeting_no, host_user_id).await;

    let fake_livekit = FakeLiveKitService::new().with_participant(
        &room_name,
        LiveKitParticipant {
            sid: Some("PA_guest2".to_string()),
            identity: guest_user_id.to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("Guest2".to_string()),
            version: Some(1),
            permission: None,
            region: None,
            is_publisher: Some(false),
        },
    );

    let state = AppState::with_livekit(build_test_config(), db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/rooms/{}/participants/{}",
                    room_name, guest_user_id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_end_business_room_requires_host() {
    let db = setup().await;

    let host_username = format!("host3_{}", Uuid::new_v4());
    let guest_username = format!("guest3_{}", Uuid::new_v4());
    let host_user_id = create_test_user(&db, &host_username).await;
    let guest_user_id = create_test_user(&db, &guest_username).await;
    let guest_token = build_user_token(guest_user_id, &guest_username);
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    let room_name = format!("m-{}", meeting_no);
    insert_meeting(&db, &meeting_no, host_user_id).await;

    let fake_livekit = FakeLiveKitService::new().with_room(LiveKitRoom {
        sid: Some("RM_business_end".to_string()),
        name: room_name.clone(),
        empty_timeout: Some(300),
        max_participants: Some(10),
        creation_time: Some(1234567890),
        turn_password: None,
        enabled_codecs: None,
        metadata: Some(String::new()),
        num_participants: Some(1),
        num_publishers: Some(1),
        active_recording: Some(false),
    });

    let state = AppState::with_livekit(build_test_config(), db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/rooms/{}/end", room_name))
                .header(header::AUTHORIZATION, format!("Bearer {}", guest_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_end_business_room_marks_meeting_ended() {
    let db = setup().await;

    let host_username = format!("host4_{}", Uuid::new_v4());
    let host_user_id = create_test_user(&db, &host_username).await;
    let host_token = build_user_token(host_user_id, &host_username);
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    let room_name = format!("m-{}", meeting_no);
    insert_meeting(&db, &meeting_no, host_user_id).await;

    let fake_livekit = FakeLiveKitService::new().with_room(LiveKitRoom {
        sid: Some("RM_business_end_host".to_string()),
        name: room_name.clone(),
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

    let state = AppState::with_livekit(build_test_config(), db.clone(), fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/rooms/{}/end", room_name))
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(get_meeting_status(&db, &meeting_no).await, "ended");
}

#[tokio::test]
async fn test_kick_participant_non_business_room_still_allows_without_auth() {
    let db = setup().await;

    let fake_livekit = FakeLiveKitService::new().with_participant(
        "general-room",
        LiveKitParticipant {
            sid: Some("PA_general_guest".to_string()),
            identity: "guest-identity".to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("General Guest".to_string()),
            version: Some(1),
            permission: None,
            region: None,
            is_publisher: Some(false),
        },
    );

    let state = build_test_state_with_livekit(db, fake_livekit);
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/rooms/general-room/participants/guest-identity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
