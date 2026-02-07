//! Meetings API integration tests
//!
//! Tests for:
//! - POST /meetings
//! - POST /meetings/{meeting_no}/join
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
use links_sig_rust_server::integrations::FakeEmailSender;
use links_sig_rust_server::AppState;
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

async fn build_authed_app(email: &str) -> (axum::Router, sqlx::PgPool, String) {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let config = build_test_config();
    let unique_email = format!("{}+{}@example.com", email, Uuid::new_v4());
    let user_id = create_test_user(&db, &unique_email).await;
    let token = encode_user_token(user_id, &unique_email, &config.jwt_secret, 3600).unwrap();

    let state = AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);
    (app, db, token)
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
    let (app, _db, token) = build_authed_app("meeting_creator@example.com").await;

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
    let (app, _db, token) = build_authed_app("repeat_join@example.com").await;

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
    let (app, _db, token) = build_authed_app("join_404@example.com").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings/123456789/join")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
