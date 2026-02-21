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
use chrono::{Duration, SecondsFormat, Utc};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use links_sig_rust_server::auth::encode_user_token;
use links_sig_rust_server::auth::jwt::decode_token;
use links_sig_rust_server::integrations::FakeEmailSender;
use links_sig_rust_server::services::MeetingLifecycleService;
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

async fn create_test_user_with_display_name(
    db: &sqlx::PgPool,
    email: &str,
    display_name: &str,
) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(email)
    .bind("test-password-hash")
    .bind(display_name)
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

async fn get_meeting_allow_guest_join(db: &sqlx::PgPool, meeting_no: &str) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT allow_guest_join FROM meetings WHERE meeting_no = $1")
        .bind(meeting_no)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn get_meeting_empty_since(
    db: &sqlx::PgPool,
    meeting_no: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT empty_since FROM meetings WHERE meeting_no = $1",
    )
    .bind(meeting_no)
    .fetch_one(db)
    .await
    .unwrap()
}

async fn get_meeting_auto_end_reason(db: &sqlx::PgPool, meeting_no: &str) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT auto_end_reason FROM meetings WHERE meeting_no = $1",
    )
    .bind(meeting_no)
    .fetch_one(db)
    .await
    .unwrap()
}

async fn get_meeting_ended_at(
    db: &sqlx::PgPool,
    meeting_no: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT ended_at FROM meetings WHERE meeting_no = $1",
    )
    .bind(meeting_no)
    .fetch_one(db)
    .await
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
async fn insert_meeting_with_state(
    db: &sqlx::PgPool,
    meeting_no: &str,
    creator_user_id: Uuid,
    status: &str,
    scheduled_start_at: chrono::DateTime<chrono::Utc>,
    opened_at: Option<chrono::DateTime<chrono::Utc>>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    cancelled_at: Option<chrono::DateTime<chrono::Utc>>,
    first_participant_joined_at: Option<chrono::DateTime<chrono::Utc>>,
    empty_since: Option<chrono::DateTime<chrono::Utc>>,
    no_join_auto_end_minutes: i32,
    empty_auto_end_minutes: i32,
) {
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
            opened_at,
            ended_at,
            cancelled_at,
            allow_guest_join,
            requires_password,
            no_join_auto_end_minutes,
            empty_auto_end_minutes,
            first_participant_joined_at,
            empty_since
        )
        VALUES (
            $1, $2, $3, $4, 'seed', $5, $6, $7, $8, false, false, $9, $10, $11, $12
        )
        "#,
    )
    .bind(meeting_no)
    .bind(room_name)
    .bind(creator_user_id)
    .bind(status)
    .bind(scheduled_start_at)
    .bind(opened_at)
    .bind(ended_at)
    .bind(cancelled_at)
    .bind(no_join_auto_end_minutes)
    .bind(empty_auto_end_minutes)
    .bind(first_participant_joined_at)
    .bind(empty_since)
    .execute(db)
    .await
    .unwrap();
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

fn build_create_meeting_payload(allow_guest_join: bool) -> String {
    let scheduled = (Utc::now() - Duration::minutes(1)).to_rfc3339();
    format!(
        r#"{{"allowGuestJoin":{},"topic":"test","scheduledStartAt":"{}"}}"#,
        allow_guest_join, scheduled
    )
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
    let (app, db, token, _user_id, _config) = build_authed_app("meeting_creator@example.com").await;

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
    assert_eq!(json["status"], "scheduled");
    assert_eq!(json["requiresPassword"], false);
    assert_eq!(json["allowGuestJoin"], false);
    assert!(!get_meeting_allow_guest_join(&db, meeting_no).await);
}

#[tokio::test]
async fn test_create_meeting_allow_guest_join_true_persisted() {
    let (app, db, token, _user_id, _config) =
        build_authed_app("allow_guest_true@example.com").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(true)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let json = body_to_json(response.into_body()).await;
    let meeting_no = json["meetingNo"].as_str().unwrap();

    assert_eq!(json["allowGuestJoin"], true);
    assert!(get_meeting_allow_guest_join(&db, meeting_no).await);
}

#[tokio::test]
async fn test_create_meeting_then_host_can_join_immediately() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("immediate_join_creator@example.com").await;

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
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(join_response.status(), StatusCode::OK);
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
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(false)))
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
async fn test_join_before_scheduled_start_returns_409_with_code() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("join_not_started@example.com").await;
    let future = (Utc::now() + Duration::minutes(5)).to_rfc3339();
    let payload = format!(r#"{{"topic":"future","scheduledStartAt":"{}"}}"#, future);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))
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
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(join_response.status(), StatusCode::CONFLICT);
    let join_json = body_to_json(join_response.into_body()).await;
    assert_eq!(join_json["code"], "MEETING_NOT_STARTED");
}

#[tokio::test]
async fn test_guest_join_requires_host_open_returns_409_with_code() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("guest_host_first@example.com").await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(true)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    let guest_join_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(guest_join_response.status(), StatusCode::CONFLICT);
    let guest_json = body_to_json(guest_join_response.into_body()).await;
    assert_eq!(guest_json["code"], "HOST_NOT_JOINED");
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
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(false)))
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
async fn test_join_uses_user_display_name_when_participant_name_is_empty() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let config = build_test_config();
    let unique_email = format!("display_name_join+{}@example.com", Uuid::new_v4());
    let display_name = "Display Join Name";
    let user_id = create_test_user_with_display_name(&db, &unique_email, display_name).await;
    let token = encode_user_token(user_id, &unique_email, &config.jwt_secret, 3600).unwrap();

    let state = AppState::with_livekit(
        config.clone(),
        db.clone(),
        FakeLiveKitService::new(),
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(false)))
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
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(join_response.status(), StatusCode::OK);

    let join_json = body_to_json(join_response.into_body()).await;
    let lk_token = join_json["token"].as_str().unwrap();
    let claims = decode_token(lk_token, &config.api_secret).unwrap();

    assert_eq!(claims.name, Some(display_name.to_string()));
}

#[tokio::test]
async fn test_guest_join_denied_when_allow_guest_join_false() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let creator_email = format!("guest_denied_creator+{}@example.com", Uuid::new_v4());
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
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"participantName":"Guest"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["error"], "Guest join is not allowed for this meeting");
}

#[tokio::test]
async fn test_guest_join_allowed_when_flag_true_and_guest_permissions_are_limited() {
    let (app, db, token, _user_id, config) = build_authed_app("guest_allowed@example.com").await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(true)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_json = body_to_json(create_response.into_body()).await;
    let meeting_no = create_json["meetingNo"].as_str().unwrap().to_string();

    assert!(get_meeting_allow_guest_join(&db, &meeting_no).await);

    let host_open_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/join", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"participantName":"Host"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(host_open_response.status(), StatusCode::OK);

    let guest_join_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"participantName":"Guest Viewer"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(guest_join_response.status(), StatusCode::OK);

    let guest_join_json = body_to_json(guest_join_response.into_body()).await;
    assert_eq!(guest_join_json["isHost"], false);

    let lk_token = guest_join_json["token"].as_str().unwrap();
    let claims = decode_token(lk_token, &config.api_secret).unwrap();

    assert!(claims.sub.starts_with("GUEST-"));
    assert_eq!(claims.name, Some("Guest Viewer".to_string()));
    assert_eq!(claims.video.can_publish, Some(false));
    assert_eq!(claims.video.can_subscribe, Some(true));
    assert_eq!(claims.video.can_publish_data, Some(false));

    let metadata: serde_json::Value =
        serde_json::from_str(claims.metadata.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(metadata["isGuest"], true);
    assert_eq!(metadata["isHost"], false);
}

#[tokio::test]
async fn test_password_protection_requires_password_for_non_host_and_guest() {
    let (app, db, host_token, _host_user_id, config) =
        build_authed_app("meeting_pwd_host@example.com").await;
    let member_email = format!("meeting_pwd_member+{}@example.com", Uuid::new_v4());
    let member_user_id = create_test_user(&db, &member_email).await;
    let member_token =
        encode_user_token(member_user_id, &member_email, &config.jwt_secret, 3600).unwrap();

    let scheduled = (Utc::now() - Duration::minutes(1)).to_rfc3339();
    let payload = format!(
        r#"{{"allowGuestJoin":true,"topic":"pwd","scheduledStartAt":"{}","password":"secret12"}}"#,
        scheduled
    );

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    let host_open_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/join", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(host_open_response.status(), StatusCode::OK);

    let member_no_password = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/join", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", member_token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"participantName":"Member"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(member_no_password.status(), StatusCode::FORBIDDEN);
    let member_no_password_json = body_to_json(member_no_password.into_body()).await;
    assert_eq!(member_no_password_json["code"], "PASSWORD_REQUIRED");

    let member_wrong_password = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/join", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", member_token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"participantName":"Member","meetingPassword":"wrong"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(member_wrong_password.status(), StatusCode::FORBIDDEN);
    let member_wrong_password_json = body_to_json(member_wrong_password.into_body()).await;
    assert_eq!(member_wrong_password_json["code"], "PASSWORD_INVALID");

    let guest_no_password = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"participantName":"Guest"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(guest_no_password.status(), StatusCode::FORBIDDEN);
    let guest_no_password_json = body_to_json(guest_no_password.into_body()).await;
    assert_eq!(guest_no_password_json["code"], "PASSWORD_REQUIRED");

    let guest_ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"participantName":"Guest","meetingPassword":"secret12"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(guest_ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_guest_join_nonexistent_meeting_returns_404() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;

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
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_guest_join_ended_meeting_returns_409() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let creator_email = format!("guest_ended_creator+{}@example.com", Uuid::new_v4());
    let creator_user_id = create_test_user(&db, &creator_email).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    insert_meeting(&db, &meeting_no, creator_user_id).await;
    sqlx::query("UPDATE meetings SET status = 'ended', ended_at = NOW() WHERE meeting_no = $1")
        .bind(&meeting_no)
        .execute(&db)
        .await
        .unwrap();

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
                .uri(format!("/meetings/{}/guest-join", meeting_no))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["error"], "Meeting has ended");
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

#[tokio::test]
async fn test_leave_meeting_last_participant_ends_meeting_immediately() {
    let temp_db = setup_test_db().await;
    run_test_migrations(&temp_db).await;

    let config = build_test_config();
    let unique_email = format!("leave_ended+{}@example.com", Uuid::new_v4());
    let user_id = create_test_user(&temp_db, &unique_email).await;
    let meeting_no = find_nonexistent_meeting_no(&temp_db).await;
    insert_meeting(&temp_db, &meeting_no, user_id).await;

    let token = encode_user_token(user_id, &unique_email, &config.jwt_secret, 3600).unwrap();
    let room_name = format!("m-{}", meeting_no);
    let fake_livekit = FakeLiveKitService::new().with_participant(
        &room_name,
        links_sig_rust_server::types::LiveKitParticipant {
            sid: Some("PA_leave_ended".to_string()),
            identity: user_id.to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("Leave Ended".to_string()),
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

    let leave_response = app
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

    assert_eq!(leave_response.status(), StatusCode::OK);
    assert_eq!(get_meeting_status(&temp_db, &meeting_no).await, "ended");
    assert!(get_meeting_empty_since(&temp_db, &meeting_no)
        .await
        .is_none());
}

#[tokio::test]
async fn test_leave_meeting_keeps_open_when_other_participants_remain() {
    let temp_db = setup_test_db().await;
    run_test_migrations(&temp_db).await;

    let config = build_test_config();
    let host_email = format!("leave_active_host+{}@example.com", Uuid::new_v4());
    let host_user_id = create_test_user(&temp_db, &host_email).await;
    let leaver_email = format!("leave_active_member+{}@example.com", Uuid::new_v4());
    let leaver_user_id = create_test_user(&temp_db, &leaver_email).await;
    let other_user_id = Uuid::new_v4();
    let meeting_no = find_nonexistent_meeting_no(&temp_db).await;
    insert_meeting(&temp_db, &meeting_no, host_user_id).await;

    let token = encode_user_token(leaver_user_id, &leaver_email, &config.jwt_secret, 3600).unwrap();
    let room_name = format!("m-{}", meeting_no);
    let fake_livekit = FakeLiveKitService::new()
        .with_participant(
            &room_name,
            links_sig_rust_server::types::LiveKitParticipant {
                sid: Some("PA_leave_active_self".to_string()),
                identity: leaver_user_id.to_string(),
                state: Some(1),
                tracks: None,
                metadata: Some(String::new()),
                joined_at: Some(1234567890),
                name: Some("Leaver".to_string()),
                version: Some(1),
                permission: None,
                region: None,
                is_publisher: Some(false),
            },
        )
        .with_participant(
            &room_name,
            links_sig_rust_server::types::LiveKitParticipant {
                sid: Some("PA_leave_active_other".to_string()),
                identity: other_user_id.to_string(),
                state: Some(1),
                tracks: None,
                metadata: Some(String::new()),
                joined_at: Some(1234567890),
                name: Some("Other".to_string()),
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

    let leave_response = app
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

    assert_eq!(leave_response.status(), StatusCode::OK);
    assert_eq!(get_meeting_status(&temp_db, &meeting_no).await, "open");
    assert!(get_meeting_empty_since(&temp_db, &meeting_no)
        .await
        .is_none());
}

#[tokio::test]
async fn test_leave_meeting_host_ends_meeting_even_when_others_remain() {
    let temp_db = setup_test_db().await;
    run_test_migrations(&temp_db).await;

    let config = build_test_config();
    let host_email = format!("leave_host_end+{}@example.com", Uuid::new_v4());
    let host_user_id = create_test_user(&temp_db, &host_email).await;
    let other_user_id = Uuid::new_v4();
    let meeting_no = find_nonexistent_meeting_no(&temp_db).await;
    insert_meeting(&temp_db, &meeting_no, host_user_id).await;

    let token = encode_user_token(host_user_id, &host_email, &config.jwt_secret, 3600).unwrap();
    let room_name = format!("m-{}", meeting_no);
    let fake_livekit = FakeLiveKitService::new()
        .with_participant(
            &room_name,
            links_sig_rust_server::types::LiveKitParticipant {
                sid: Some("PA_leave_host_self".to_string()),
                identity: host_user_id.to_string(),
                state: Some(1),
                tracks: None,
                metadata: Some(String::new()),
                joined_at: Some(1234567890),
                name: Some("Host".to_string()),
                version: Some(1),
                permission: None,
                region: None,
                is_publisher: Some(false),
            },
        )
        .with_participant(
            &room_name,
            links_sig_rust_server::types::LiveKitParticipant {
                sid: Some("PA_leave_host_other".to_string()),
                identity: other_user_id.to_string(),
                state: Some(1),
                tracks: None,
                metadata: Some(String::new()),
                joined_at: Some(1234567890),
                name: Some("Other".to_string()),
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

    let leave_response = app
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

    assert_eq!(leave_response.status(), StatusCode::OK);
    assert_eq!(get_meeting_status(&temp_db, &meeting_no).await, "ended");
}

#[tokio::test]
async fn test_cancel_meeting_by_host_sets_cancelled() {
    let (app, db, token, _user_id, _config) = build_authed_app("cancel_host@example.com").await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(false)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    let cancel_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/cancel", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel_response.status(), StatusCode::OK);
    assert_eq!(get_meeting_status(&db, &meeting_no).await, "cancelled");
}

#[tokio::test]
async fn test_cancel_meeting_non_host_forbidden() {
    let (app, _db, host_token, _host_user_id, config) =
        build_authed_app("cancel_non_host_owner@example.com").await;
    let member_email = format!("cancel_non_host_member+{}@example.com", Uuid::new_v4());
    let member_user_id = create_test_user(&_db, &member_email).await;
    let member_token =
        encode_user_token(member_user_id, &member_email, &config.jwt_secret, 3600).unwrap();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(false)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    let cancel_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/cancel", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", member_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_host_meetings_returns_only_creator_meetings() {
    let (app, _db, host_token, _host_user_id, config) =
        build_authed_app("host_list@example.com").await;
    let other_email = format!("host_list_other+{}@example.com", Uuid::new_v4());
    let other_user_id = create_test_user(&_db, &other_email).await;
    let other_token =
        encode_user_token(other_user_id, &other_email, &config.jwt_secret, 3600).unwrap();

    let payload = build_create_meeting_payload(false);
    for _ in 0..2 {
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/meetings")
                    .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::CREATED);
    }

    let other_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", other_token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(other_create.status(), StatusCode::CREATED);

    let host_list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me/host-meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(host_list_response.status(), StatusCode::OK);
    let host_list_json = body_to_json(host_list_response.into_body()).await;
    let meetings = host_list_json["meetings"].as_array().unwrap();
    assert_eq!(meetings.len(), 2);
}

#[tokio::test]
async fn test_lifecycle_no_join_timeout_marks_scheduled_meeting_ended() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    let creator_email = format!("lifecycle_no_join+{}@example.com", Uuid::new_v4());
    let creator_user_id = create_test_user(&db, &creator_email).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;

    insert_meeting_with_state(
        &db,
        &meeting_no,
        creator_user_id,
        "scheduled",
        Utc::now() - Duration::minutes(20),
        None,
        None,
        None,
        None,
        None,
        5,
        10,
    )
    .await;

    MeetingLifecycleService::run_once(&db, &FakeLiveKitService::new(), 424242)
        .await
        .unwrap();

    assert_eq!(get_meeting_status(&db, &meeting_no).await, "ended");
    assert_eq!(
        get_meeting_auto_end_reason(&db, &meeting_no)
            .await
            .as_deref(),
        Some("no_join_timeout")
    );
    assert!(get_meeting_ended_at(&db, &meeting_no).await.is_some());
}

#[tokio::test]
async fn test_lifecycle_empty_timeout_marks_open_meeting_ended() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    let creator_email = format!("lifecycle_empty_timeout+{}@example.com", Uuid::new_v4());
    let creator_user_id = create_test_user(&db, &creator_email).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    let opened_at = Utc::now() - Duration::minutes(30);

    insert_meeting_with_state(
        &db,
        &meeting_no,
        creator_user_id,
        "open",
        opened_at,
        Some(opened_at),
        None,
        None,
        Some(opened_at),
        Some(Utc::now() - Duration::minutes(15)),
        15,
        5,
    )
    .await;

    MeetingLifecycleService::run_once(&db, &FakeLiveKitService::new(), 424243)
        .await
        .unwrap();

    assert_eq!(get_meeting_status(&db, &meeting_no).await, "ended");
    assert_eq!(
        get_meeting_auto_end_reason(&db, &meeting_no)
            .await
            .as_deref(),
        Some("empty_timeout")
    );
}

#[tokio::test]
async fn test_lifecycle_non_empty_room_clears_empty_since() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    let creator_email = format!("lifecycle_non_empty+{}@example.com", Uuid::new_v4());
    let creator_user_id = create_test_user(&db, &creator_email).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;
    let room_name = format!("m-{}", meeting_no);
    let opened_at = Utc::now() - Duration::minutes(30);

    insert_meeting_with_state(
        &db,
        &meeting_no,
        creator_user_id,
        "open",
        opened_at,
        Some(opened_at),
        None,
        None,
        Some(opened_at),
        Some(Utc::now() - Duration::minutes(1)),
        15,
        60,
    )
    .await;

    let fake_livekit = FakeLiveKitService::new().with_participant(
        &room_name,
        links_sig_rust_server::types::LiveKitParticipant {
            sid: Some("PA_lifecycle_non_empty".to_string()),
            identity: creator_user_id.to_string(),
            state: Some(1),
            tracks: None,
            metadata: Some(String::new()),
            joined_at: Some(1234567890),
            name: Some("Host".to_string()),
            version: Some(1),
            permission: None,
            region: None,
            is_publisher: Some(false),
        },
    );

    MeetingLifecycleService::run_once(&db, &fake_livekit, 424244)
        .await
        .unwrap();

    assert_eq!(get_meeting_status(&db, &meeting_no).await, "open");
    assert!(get_meeting_empty_since(&db, &meeting_no).await.is_none());
    assert!(get_meeting_auto_end_reason(&db, &meeting_no)
        .await
        .is_none());
}

#[tokio::test]
async fn test_lifecycle_skips_when_advisory_lock_is_held_by_another_session() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    let creator_email = format!("lifecycle_lock_skip+{}@example.com", Uuid::new_v4());
    let creator_user_id = create_test_user(&db, &creator_email).await;
    let meeting_no = find_nonexistent_meeting_no(&db).await;

    insert_meeting_with_state(
        &db,
        &meeting_no,
        creator_user_id,
        "scheduled",
        Utc::now() + Duration::hours(6),
        None,
        None,
        None,
        None,
        None,
        120,
        120,
    )
    .await;

    let lock_key = 424245i64;
    let mut conn = db.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(lock_key)
        .execute(&mut *conn)
        .await
        .unwrap();

    MeetingLifecycleService::run_once(&db, &FakeLiveKitService::new(), lock_key)
        .await
        .unwrap();

    assert_eq!(get_meeting_status(&db, &meeting_no).await, "scheduled");
    assert!(get_meeting_ended_at(&db, &meeting_no).await.is_none());

    sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock($1)")
        .bind(lock_key)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_host_meetings_default_excludes_ended_and_cancelled() {
    let (app, db, host_token, host_user_id, _config) =
        build_authed_app("host_default_filter@example.com").await;
    let now = Utc::now();

    let no1 = find_nonexistent_meeting_no(&db).await;
    let no2 = find_nonexistent_meeting_no(&db).await;
    let no3 = find_nonexistent_meeting_no(&db).await;
    let no4 = find_nonexistent_meeting_no(&db).await;

    insert_meeting_with_state(
        &db,
        &no1,
        host_user_id,
        "scheduled",
        now + Duration::hours(1),
        None,
        None,
        None,
        None,
        None,
        15,
        10,
    )
    .await;
    insert_meeting_with_state(
        &db,
        &no2,
        host_user_id,
        "open",
        now - Duration::hours(1),
        Some(now - Duration::hours(1)),
        None,
        None,
        Some(now - Duration::hours(1)),
        None,
        15,
        10,
    )
    .await;
    insert_meeting_with_state(
        &db,
        &no3,
        host_user_id,
        "ended",
        now - Duration::hours(2),
        Some(now - Duration::hours(2)),
        Some(now - Duration::hours(1)),
        None,
        Some(now - Duration::hours(2)),
        None,
        15,
        10,
    )
    .await;
    insert_meeting_with_state(
        &db,
        &no4,
        host_user_id,
        "cancelled",
        now + Duration::hours(2),
        None,
        None,
        Some(now),
        None,
        None,
        15,
        10,
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me/host-meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_to_json(response.into_body()).await;
    let meetings = json["meetings"].as_array().unwrap();
    let statuses: Vec<&str> = meetings
        .iter()
        .filter_map(|item| item["status"].as_str())
        .collect();
    assert_eq!(meetings.len(), 2);
    assert!(statuses
        .iter()
        .all(|value| *value == "scheduled" || *value == "open"));
}

#[tokio::test]
async fn test_host_meetings_include_ended_returns_all_statuses() {
    let (app, db, host_token, host_user_id, _config) =
        build_authed_app("host_include_ended@example.com").await;
    let now = Utc::now();

    for status in ["scheduled", "open", "ended", "cancelled"] {
        let meeting_no = find_nonexistent_meeting_no(&db).await;
        let (opened_at, ended_at, cancelled_at) = match status {
            "open" => (Some(now - Duration::hours(2)), None, None),
            "ended" => (
                Some(now - Duration::hours(3)),
                Some(now - Duration::hours(1)),
                None,
            ),
            "cancelled" => (None, None, Some(now - Duration::minutes(30))),
            _ => (None, None, None),
        };
        insert_meeting_with_state(
            &db,
            &meeting_no,
            host_user_id,
            status,
            now,
            opened_at,
            ended_at,
            cancelled_at,
            None,
            None,
            15,
            10,
        )
        .await;
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me/host-meetings?includeEnded=true")
                .header(header::AUTHORIZATION, format!("Bearer {}", host_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_to_json(response.into_body()).await;
    let meetings = json["meetings"].as_array().unwrap();
    assert_eq!(meetings.len(), 4);
}

#[tokio::test]
async fn test_host_meetings_invalid_status_returns_400_with_code() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("host_invalid_status@example.com").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me/host-meetings?status=unknown")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["code"], "INVALID_MEETING_STATUS");
}

#[tokio::test]
async fn test_host_meetings_time_range_filters_by_scheduled_start_at() {
    let (app, db, token, host_user_id, _config) =
        build_authed_app("host_time_range@example.com").await;
    let now = Utc::now();
    let inside_no = find_nonexistent_meeting_no(&db).await;
    let outside_no = find_nonexistent_meeting_no(&db).await;

    insert_meeting_with_state(
        &db,
        &inside_no,
        host_user_id,
        "scheduled",
        now - Duration::hours(2),
        None,
        None,
        None,
        None,
        None,
        15,
        10,
    )
    .await;
    insert_meeting_with_state(
        &db,
        &outside_no,
        host_user_id,
        "scheduled",
        now - Duration::days(3),
        None,
        None,
        None,
        None,
        None,
        15,
        10,
    )
    .await;

    let time_from = (now - Duration::days(1)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let uri = format!("/me/host-meetings?timeFrom={}", time_from);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_to_json(response.into_body()).await;
    let meetings = json["meetings"].as_array().unwrap();
    assert_eq!(meetings.len(), 1);
    assert_eq!(meetings[0]["meetingNo"], inside_no);
}

#[tokio::test]
async fn test_cancel_open_meeting_returns_409_with_not_cancellable_code() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("cancel_open_meeting@example.com").await;
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(build_create_meeting_payload(false)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let meeting_no = body_to_json(create_response.into_body()).await["meetingNo"]
        .as_str()
        .unwrap()
        .to_string();

    let open_response = app
        .clone()
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
    assert_eq!(open_response.status(), StatusCode::OK);

    let cancel_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/meetings/{}/cancel", meeting_no))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel_response.status(), StatusCode::CONFLICT);
    let json = body_to_json(cancel_response.into_body()).await;
    assert_eq!(json["code"], "MEETING_NOT_CANCELLABLE");
}

#[tokio::test]
async fn test_create_meeting_password_length_boundaries() {
    let (app, _db, token, _user_id, _config) =
        build_authed_app("password_boundary@example.com").await;
    let scheduled = (Utc::now() - Duration::minutes(1)).to_rfc3339();

    let too_short = format!(
        r#"{{"scheduledStartAt":"{}","password":"12345"}}"#,
        scheduled
    );
    let ok_min = format!(
        r#"{{"scheduledStartAt":"{}","password":"123456"}}"#,
        scheduled
    );
    let too_long = format!(
        r#"{{"scheduledStartAt":"{}","password":"{}"}}"#,
        scheduled,
        "a".repeat(33)
    );

    let res_short = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(too_short))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_short.status(), StatusCode::BAD_REQUEST);

    let res_ok_min = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(ok_min))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ok_min.status(), StatusCode::CREATED);

    let res_too_long = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/meetings")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(too_long))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_too_long.status(), StatusCode::BAD_REQUEST);
}
