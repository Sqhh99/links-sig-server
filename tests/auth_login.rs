//! Auth login integration tests
//!
//! Tests for POST /auth/login
//!
//! These tests require a running test database. Run with:
//! ```
//! docker compose up -d postgres-test
//! cargo test --test auth_login
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

/// Create a test user with known credentials
async fn create_test_user(db: &sqlx::PgPool, email: &str, password: &str) {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind(email)
        .bind(&password_hash)
        .execute(db)
        .await
        .unwrap();
}

// ============================================================================
// POST /auth/login
// ============================================================================

#[tokio::test]
async fn test_login_returns_200() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_login@example.com";
    let test_password = "securePassword123";
    cleanup_test_user(&db, test_email).await;
    create_test_user(&db, test_email, test_password).await;

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
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "password": "{}"}}"#,
                    test_email, test_password
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    assert!(json.get("userId").is_some());
    assert!(json.get("email").is_some());
    assert!(json.get("token").is_some());

    // Verify token is valid JWT
    let token = json["token"].as_str().unwrap();
    assert_eq!(token.split('.').count(), 3, "Token should be JWT format");

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_wrong_password@example.com";
    cleanup_test_user(&db, test_email).await;
    create_test_user(&db, test_email, "correctPassword123").await;

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
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "password": "wrongPassword"}}"#,
                    test_email
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

#[tokio::test]
async fn test_login_nonexistent_user_returns_401() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "nonexistent@example.com";
    cleanup_test_user(&db, test_email).await;

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
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "password": "anyPassword123"}}"#,
                    test_email
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_empty_password_returns_401() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_empty_password@example.com";
    cleanup_test_user(&db, test_email).await;
    create_test_user(&db, test_email, "correctPassword123").await;

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
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "{}", "password": ""}}"#,
                    test_email
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}

#[tokio::test]
async fn test_login_case_insensitive_email() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = "test_case@example.com";
    let test_password = "securePassword123";
    cleanup_test_user(&db, test_email).await;
    create_test_user(&db, test_email, test_password).await;

    let config = build_test_config();
    let fake_email = Arc::new(FakeEmailSender::new());
    let state = links_sig_rust_server::AppState::with_livekit(
        config,
        db.clone(),
        FakeLiveKitService::new(),
        fake_email,
    );
    let app = build_test_app_with_state(state);

    // Login with uppercase email
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email": "TEST_CASE@EXAMPLE.COM", "password": "{}"}}"#,
                    test_password
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Cleanup
    cleanup_test_user(&db, test_email).await;
}
