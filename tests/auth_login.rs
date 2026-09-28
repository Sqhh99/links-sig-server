//! Auth login integration tests
//!
//! Tests for POST /auth/login, which signs in existing accounts and creates
//! the account on first use of a username.
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
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use support::{build_test_app, cleanup_test_user, run_test_migrations, setup_test_db};

/// Create a test user with known credentials
async fn create_test_user(db: &sqlx::PgPool, username: &str, password: &str) {
    create_test_user_with_display_name(db, username, password, None).await;
}

async fn create_test_user_with_display_name(
    db: &sqlx::PgPool,
    username: &str,
    password: &str,
    display_name: Option<&str>,
) {
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

    sqlx::query("INSERT INTO users (username, password_hash, display_name) VALUES ($1, $2, $3)")
        .bind(username)
        .bind(&password_hash)
        .bind(display_name)
        .execute(db)
        .await
        .unwrap();
}

/// A username that satisfies the creation rules and is unique per test run
fn unique_username(prefix: &str) -> String {
    format!("{}_{}", prefix, &Uuid::new_v4().simple().to_string()[..8])
}

async fn count_users(db: &sqlx::PgPool, username: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE LOWER(username) = LOWER($1)")
        .bind(username)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn post_json(db: &sqlx::PgPool, uri: &str, body: Value) -> (StatusCode, Value) {
    let app = build_test_app(db.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = support::body_to_bytes(response.into_body()).await;
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn login(db: &sqlx::PgPool, username: &str, password: &str) -> (StatusCode, Value) {
    post_json(
        db,
        "/auth/login",
        json!({ "username": username, "password": password }),
    )
    .await
}

// ============================================================================
// Existing accounts
// ============================================================================

#[tokio::test]
async fn test_login_existing_user_returns_200() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("login_ok");
    let password = "securePassword123";
    cleanup_test_user(&db, &username).await;
    create_test_user(&db, &username, password).await;

    let (status, json) = login(&db, &username, password).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.get("userId").is_some());
    assert_eq!(json["username"], username);
    assert!(json["token"].as_str().is_some());
    assert_eq!(json["accountCreated"], false);
    assert!(json.get("email").is_none());

    cleanup_test_user(&db, &username).await;
}

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("login_wrong");
    cleanup_test_user(&db, &username).await;
    create_test_user(&db, &username, "correctPassword123").await;

    let (status, json) = login(&db, &username, "wrongPassword123").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["code"], "INVALID_CREDENTIALS");
    assert_eq!(count_users(&db, &username).await, 1);

    cleanup_test_user(&db, &username).await;
}

#[tokio::test]
async fn test_login_is_case_insensitive_and_keeps_stored_case() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("Login_Case");
    let password = "securePassword123";
    cleanup_test_user(&db, &username).await;
    create_test_user(&db, &username, password).await;

    let (status, json) = login(&db, &username.to_lowercase(), password).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["username"], username);
    assert_eq!(json["accountCreated"], false);

    cleanup_test_user(&db, &username).await;
}

#[tokio::test]
async fn test_login_returns_display_name_when_present() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("login_display");
    let password = "securePassword123";
    cleanup_test_user(&db, &username).await;
    create_test_user_with_display_name(&db, &username, password, Some("张三")).await;

    let (status, json) = login(&db, &username, password).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["displayName"], "张三");

    cleanup_test_user(&db, &username).await;
}

#[tokio::test]
async fn test_login_legacy_email_account_still_works() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    // Accounts created before usernames keep their email as the username,
    // which the creation rules would reject.
    let username = format!("legacy_{}@example.com", Uuid::new_v4().simple());
    let password = "securePassword123";
    cleanup_test_user(&db, &username).await;
    create_test_user(&db, &username, password).await;

    let (status, json) = login(&db, &username, password).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["username"], username);

    cleanup_test_user(&db, &username).await;
}

#[tokio::test]
async fn test_login_empty_password_returns_400() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("login_empty");
    cleanup_test_user(&db, &username).await;
    create_test_user(&db, &username, "correctPassword123").await;

    let (status, _json) = login(&db, &username, "").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);

    cleanup_test_user(&db, &username).await;
}

// ============================================================================
// First login creates the account
// ============================================================================

#[tokio::test]
async fn test_first_login_creates_account_then_signs_in() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("New_User");
    let password = "securePassword123";
    cleanup_test_user(&db, &username).await;

    let (status, json) = login(&db, &username, password).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json["username"], username);
    assert_eq!(json["accountCreated"], true);
    assert!(json["token"].as_str().is_some());
    let user_id = json["userId"].as_str().unwrap().to_string();
    assert_eq!(count_users(&db, &username).await, 1);

    let (status, json) = login(&db, &username.to_uppercase(), password).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["userId"], user_id);
    assert_eq!(json["username"], username);
    assert_eq!(json["accountCreated"], false);

    // A taken username with another password does not create a second account.
    let (status, _json) = login(&db, &username.to_lowercase(), "otherPassword456").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(count_users(&db, &username).await, 1);

    cleanup_test_user(&db, &username).await;
}

#[tokio::test]
async fn test_first_login_rejects_weak_passwords() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let username = unique_username("weak_pwd");
    cleanup_test_user(&db, &username).await;

    for password in ["short1", "lettersonly", "1234567890", username.as_str()] {
        let (status, json) = login(&db, &username, password).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "password {:?}", password);
        assert_eq!(json["code"], "WEAK_PASSWORD", "password {:?}", password);
    }
    assert_eq!(count_users(&db, &username).await, 0);
}

#[tokio::test]
async fn test_first_login_rejects_invalid_usernames() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let too_long = "a".repeat(33);
    for username in ["a", "has space", "new@example.com", too_long.as_str()] {
        let (status, json) = login(&db, username, "securePassword123").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "username {:?}", username);
        assert_eq!(json["code"], "INVALID_USERNAME", "username {:?}", username);
        assert_eq!(count_users(&db, username).await, 0);
    }
}

#[tokio::test]
async fn test_login_requires_username_field() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let (status, _json) = post_json(
        &db,
        "/auth/login",
        json!({ "email": "user@example.com", "password": "securePassword123" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_registration_endpoints_are_gone() {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;

    let (status, _json) = post_json(
        &db,
        "/auth/register/request-code",
        json!({ "email": "user@example.com" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _json) = post_json(
        &db,
        "/auth/register",
        json!({ "email": "user@example.com", "code": "123456", "password": "securePassword123" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
