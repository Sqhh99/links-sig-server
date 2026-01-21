//! Health endpoint integration tests
//!
//! Tests for GET /health/

mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use support::{body_to_json, build_test_app};

#[tokio::test]
async fn test_health_check_returns_200() {
    let app = build_test_app();

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
    let app = build_test_app();

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
    assert!(json.get("status").is_some(), "Response should have 'status' field");
    assert!(json.get("time").is_some(), "Response should have 'time' field");

    // Verify status value
    assert_eq!(json["status"], "ok");
}
