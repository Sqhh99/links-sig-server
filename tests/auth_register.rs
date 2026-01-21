//! Auth registration integration tests
//!
//! Tests for:
//! - POST /auth/register/request-code
//! - POST /auth/register
//!
//! These tests require a running test database. Run with:
//! ```
//! docker compose up -d postgres-test
//! cargo test --test auth_register
//! ```

mod support;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;

use links_sig_rust_server::integrations::FakeEmailSender;
use support::{
    body_to_json, build_test_app_with_state, build_test_config, cleanup_test_user,
    run_test_migrations, setup_test_db, FakeLiveKitService,
};

/// Helper to extract verification code from fake email
fn extract_code_from_email(email_body: &str) -> Option<String> {
    // Email format: "Your verification code is: 123456\n..."
    email_body
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("Your verification code is: "))
        .map(|s| s.to_string())
}

// ============================================================================
// POST /auth/register/request-code
// ============================================================================

#[tokio::test]
async fn test_request_register_code_returns_200() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    
    let test_email = "test_request_code@example.com";
    cleanup_test_user(&db, test_email).await;

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        fake_email.clone(),
    );
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"email": "{}"}}"#, test_email)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    assert!(json.get("message").is_some());
    assert!(json.get("retryAfterSecs").is_some());

    // Verify email was sent
    let emails = fake_email.get_emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, test_email);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

#[tokio::test]
async fn test_request_register_code_invalid_email_returns_400() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db,
        FakeLiveKitService::new(),
        fake_email,
    );
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email": "invalid-email"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_request_register_code_existing_email_returns_409() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "existing_user@example.com";
    cleanup_test_user(&db, test_email).await;

    // First create a user
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind(test_email)
        .bind("$argon2id$v=19$m=19456,t=2,p=1$dummy_hash")
        .execute(&db)
        .await
        .unwrap();

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        fake_email,
    );
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"email": "{}"}}"#, test_email)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

// ============================================================================
// POST /auth/register
// ============================================================================

#[tokio::test]
async fn test_register_returns_201() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_register@example.com";
    cleanup_test_user(&db, test_email).await;

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        fake_email.clone(),
    );

    // Step 1: Request verification code
    let app = build_test_app_with_state(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"email": "{}"}}"#, test_email)))
                .unwrap(),
        )
        .await
        .unwrap();

    // Extract code from fake email
    let emails = fake_email.get_emails();
    let code = extract_code_from_email(&emails[0].body).expect("Should have code in email");

    // Step 2: Complete registration
    let app = build_test_app_with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "code": "{}", "password": "securePassword123"}}"#,
                    test_email, code
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let json = body_to_json(response.into_body()).await;
    assert!(json.get("userId").is_some());
    assert!(json.get("email").is_some());
    assert!(json.get("token").is_some());

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

#[tokio::test]
async fn test_register_invalid_code_returns_400() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_invalid_code@example.com";
    cleanup_test_user(&db, test_email).await;

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        fake_email.clone(),
    );

    // Request code first
    let app = build_test_app_with_state(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"email": "{}"}}"#, test_email)))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try with wrong code
    let app = build_test_app_with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "code": "000000", "password": "securePassword123"}}"#,
                    test_email
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

#[tokio::test]
async fn test_register_weak_password_returns_400() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_weak_password@example.com";
    cleanup_test_user(&db, test_email).await;

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        fake_email.clone(),
    );

    // Request code first
    let app = build_test_app_with_state(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"email": "{}"}}"#, test_email)))
                .unwrap(),
        )
        .await
        .unwrap();

    let emails = fake_email.get_emails();
    let code = extract_code_from_email(&emails[0].body).expect("Should have code");

    // Try with weak password (less than 8 characters)
    let app = build_test_app_with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "code": "{}", "password": "short"}}"#,
                    test_email, code
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}
