//! Meetings API integration tests
//!
//! Tests for:
//! - POST /meetings
//! - POST /meetings/{meeting_no}/join
//! - POST /meetings/{meeting_no}/leave
//! - GET /me/meeting-records

mod support;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use links_sig_rust_server::auth::encode_user_token;
use links_sig_rust_server::auth::jwt::decode_token;
use links_sig_rust_server::integrations::FakeEmailSender;
use links_sig_rust_server::{AppState, Config};
use support::{
    body_to_json, build_test_app_with_state, build_test_config, run_test_migrations, setup_test_db,
    FakeLiveKitService,
};

async fn create_test_user(db: &sqlx::PgPool, email: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(email)
        .bind("test-password-hash")
        .execute(db)
        .await
        .unwrap();
    user_id
}

async fn build_authed_app_with_livekit(
    email: &str,
    fake_livekit: FakeLiveKitService,
) -> (axum::Router, sqlx::PgPool, String, Uuid, Config) {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let config = build_test_config();
    let unique_email = format!("{}+{}@example.com", email, Uuid::new_v4());
    let user_id = create_test_user(&db, &unique_email).await;
    let token = encode_user_token(user_id, &unique_email, &config.jwt_secret, 3600).unwrap();

    let state = AppState::with_livekit(
        config.clone(),
        db.clone(),
        fake_livekit,
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);
    (app, db, token, user_id, config)
}

async fn build_authed_app(email: &str) -> (axum::Router, sqlx::PgPool, String, Uuid, Config) {
    build_authed_app_with_livekit(email, FakeLiveKitService::new()).await
}

async fn insert_meeting(db: &sqlx::PgPool, meeting_no: &str, creator_user_id: Uuid) {
    let room_name = format!("m-{}", meeting_no);
    sqlx::query(
        "INSERT INTO meetings (meeting_no, room_name, creator_user_id) VALUES ($1, $2, $3)",
    )
    .bind(meeting_no)
    .bind(room_name)
    .bind(creator_user_id)
    .execute(db)
    .await
    .unwrap();
}

async fn find_nonexistent_meeting_no(db: &sqlx::PgPool) -> String {
    for _ in 0..64 {
        let candidate = format!("{:09}", (Uuid::new_v4().as_u128() % 1_000_000_000) as u32);
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM meetings WHERE meeting_no = $1 LIMIT 1",
        )
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

#[tokio::test]
async fn test_create_meeting_requires_auth() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let state = AppState::with_livekit(
        build_test_config(),
        db,
        FakeLiveKitService::new(),
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_meeting_returns_meeting_number_and_share_url() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("meeting_creator@example.com").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let json = body_to_json(response.into_body()).await;

    let meeting_no = json["meetingNo"].as_str().unwrap();
    assert_eq!(meeting_no.len(), 9);
    assert!(meeting_no.chars().all(|c| c.is_ascii_digit()));

    assert_eq!(json["roomName"], format!("m-{}", meeting_no));
    assert!(json["shareUrl"]
        .as_str()
        .unwrap()
        .contains(&format!("meetingNo={}", meeting_no)));
}

#[tokio::test]
async fn test_join_same_meeting_twice_does_not_duplicate_record() {
    let (app, _db, token, _user_id, _config) = build_authed_app("repeat_join@example.com").await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/meetings/{}/join", meeting_no))
                    .header(header::AUTHORIZATION, format!("Bearer {}", token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"participantName":"Tester"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let records_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me/meeting-records")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(records_response.status(), StatusCode::OK);
    let records_json = body_to_json(records_response.into_body()).await;

    let records = records_json["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["meetingNo"], meeting_no);
    assert_eq!(records[0]["joinCount"], 2);
}

#[tokio::test]
async fn test_join_nonexistent_meeting_returns_404() {
    let (app, db, token, _user_id, _config) = build_authed_app("join_404@example.com").await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/join", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_join_token_identity_is_jwt_user_id() {
    let (app, _db, token, user_id, config) = build_authed_app("identity_join@example.com").await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    let join_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/join", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"participantName":"Tester Name"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(join_response.status(), StatusCode::OK);

    let join_json = body_to_json(join_response.into_body()).await;
    let lk_token = join_json["token"].as_str().unwrap();
    let claims = decode_token(lk_token, &config.api_secret).unwrap();

    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.name, Some("Tester Name".to_string()));
}

#[tokio::test]
async fn test_leave_meeting_requires_auth() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let creator_email = format!("leave_auth_creator+{}@example.com", Uuid::new_v4());
    let creator_user_id = create_test_user(&db, &creator_email).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    insert_meeting(&db, &meeting_no, creator_user_id).await;

    let state = AppState::with_livekit(
        build_test_config(),
        db,
        FakeLiveKitService::new(),
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/leave", meeting_no))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_leave_meeting_is_idempotent() {
    let temp_db = setup_test_db().await;
    run_test_migrations(&temp_db).await;

    let config = build_test_config();
    let unique_email = format!("leave_idempotent+{}@example.com", Uuid::new_v4());
    let user_id = create_test_user(&temp_db, &unique_email).await;
    let meeting_no = find_nonexistent_meeting_no(&temp_db).await;
    insert_meeting(&temp_db, &meeting_no, user_id).await;

    let token = encode_user_token(user_id, &unique_email, &config.jwt_secret, 3600).unwrap();
    let room_name = format!("m-{}", meeting_no);
    let fake_livekit = FakeLiveKitService::new().with_participant(
        &room_name,
        links_sig_rust_server::types::LiveKitParticipant {
            sid: Some("PA_leave_test".to_string()),
            identity: user_id.to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("Leave Tester".to_string()),
            version: Some(1),
            permission: None,
            region: None,
            is_publisher: Some(false),
        },
    );

    let state = AppState::with_livekit(
        config,
        temp_db.clone(),
        fake_livekit,
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/leave", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_json = body_to_json(first_response.into_body()).await;
    assert_eq!(first_json["left"], true);
    assert_eq!(first_json["identity"], user_id.to_string());

    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/leave", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_response.status(), StatusCode::OK);
    let second_json = body_to_json(second_response.into_body()).await;
    assert_eq!(second_json["left"], false);
}
