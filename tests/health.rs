//! Health endpoint integration tests
//!
//! Tests for GET /health/
//!
//! These tests require a running test database. Run with:
//! ```
//! docker compose up -d postgres-test
//! cargo test --test health
//! ```

mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use support::{body_to_json, build_test_app, run_test_migrations, setup_test_db};

async fn setup() -> sqlx::PgPool {
    let db = setup_test_db().await;
    run_test_migrations(&db).await;
    db
}

#[tokio::test]
async fn test_health_check_returns_200() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_check_returns_valid_json() {
    let db = setup().await;
    let app = build_test_app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;

    // Verify response structure
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(
        json.get("status").is_some(),
        "Response should have 'status' field"
    );
    assert!(
        json.get("time").is_some(),
        "Response should have 'time' field"
    );

    // Verify status value
    assert_eq!(json["status"], "ok");
}
