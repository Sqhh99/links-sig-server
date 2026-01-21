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
use tower::ServiceExt;

use support::{body_to_json, build_test_app, setup_test_db, run_test_migrations};

async fn setup() -> sqlx::PgPool {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    db
}

// ============================================================================
// POST /token
// ============================================================================

#[tokio::test]
async fn test_get_token_returns_200() {
    let db = setup().await;
    let app = build_test_app(db);

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
    let app = build_test_app(db);

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
    assert!(json.get("token").is_some(), "Response should have 'token' field");
    assert!(json.get("url").is_some(), "Response should have 'url' field");
    assert!(json.get("roomName").is_some(), "Response should have 'roomName' field");
    assert!(json.get("isHost").is_some(), "Response should have 'isHost' field");

    // Verify token is a non-empty string (JWT format)
    let token = json["token"].as_str().unwrap();
    assert!(!token.is_empty(), "Token should not be empty");
    assert!(
        token.split('.').count() == 3,
        "Token should be in JWT format (3 parts separated by dots)"
    );
}

#[tokio::test]
async fn test_get_token_as_host() {
    let db = setup().await;
    let app = build_test_app(db);

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

    // Verify isHost is reflected in response
    assert_eq!(json["isHost"], true, "isHost should be true for host request");
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
async fn test_get_token_with_empty_fields_uses_defaults() {
    let db = setup().await;
    let app = build_test_app(db);

    // Send minimal JSON - fields should use defaults
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

    // Should still work with default values (based on TokenRequest #[serde(default)])
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    assert!(json.get("token").is_some(), "Should still return a token");
}
