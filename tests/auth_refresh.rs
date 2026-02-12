//! Auth refresh integration tests
//!
//! Tests for POST /auth/refresh

mod support;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use links_sig_rust_server::auth::{decode_user_token, encode_user_token};
use links_sig_rust_server::integrations::FakeEmailSender;
use support::{
    body_to_json, build_test_app_with_state, build_test_config, cleanup_test_user,
    run_test_migrations, setup_test_db, FakeLiveKitService,
};

async fn create_test_user(db: &sqlx::PgPool, email: &str, password: &str) -> Uuid {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    let user_id: Uuid =
        sqlx::query_scalar("INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id")
            .bind(email)
            .bind(&password_hash)
            .fetch_one(db)
            .await
            .unwrap();

    user_id
}

#[tokio::test]
async fn test_refresh_returns_200_and_new_token() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let test_email = format!("refresh_ok+{}@example.com", Uuid::new_v4());
    let test_password = "securePassword123";
    cleanup_test_user(&db, &test_email).await;
    let user_id = create_test_user(&db, &test_email, test_password).await;

    let config = build_test_config();
    let old_token =
        encode_user_token(user_id, &test_email, &config.jwt_secret, config.jwt_expiration_secs)
            .unwrap();

    let state = links_sig_rust_server::AppState::with_livekit(
        config.clone(),
        db.clone(),
        FakeLiveKitService::new(),
        Arc::new(FakeEmailSender::new()),
    );
    let app = build_test_app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header(header::AUTHORIZATION, format!("Bearer {}", old_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_to_json(response.into_body()).await;

    assert_eq!(json["userId"], user_id.to_string());
    assert_eq!(json["email"], test_email);
    assert_eq!(json["expiresInSecs"], config.jwt_expiration_secs);
    assert!(json["token"].as_str().is_some());

    let refreshed_token = json["token"].as_str().unwrap();
    let claims = decode_user_token(refreshed_token, &config.jwt_secret).unwrap();
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.email, test_email);

    cleanup_test_user(&db, &test_email).await;
}

#[tokio::test]
async fn test_refresh_requires_auth() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let state = links_sig_rust_server::AppState::with_livekit(
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
                .uri("/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_invalid_token_returns_401() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let state = links_sig_rust_server::AppState::with_livekit(
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
                .uri("/auth/refresh")
                .header(header::AUTHORIZATION, "Bearer not-a-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let json = body_to_json(response.into_body()).await;
    assert!(json["error"].as_str().is_some());
}
