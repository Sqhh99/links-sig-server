//! Test application builder
//!
//! Provides helper functions to build test instances of the application.

use axum::Router;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use sqlx::PgPool;

use links_sig_rust_server::{build_api_router, AppState, Config};

use super::fake_livekit::FakeLiveKitService;

/// Build a test configuration with default values
pub fn build_test_config() -> Config {
    Config::for_tests()
}

/// Build a test AppState with FakeLiveKitService
///
/// Requires a database connection pool (use `setup_test_db` to get one)
pub fn build_test_state(db: PgPool) -> AppState {
    build_test_state_with_livekit(db, FakeLiveKitService::new())
}

/// Build a test AppState with a custom FakeLiveKitService
pub fn build_test_state_with_livekit(db: PgPool, fake_livekit: FakeLiveKitService) -> AppState {
    AppState::with_livekit(build_test_config(), db, fake_livekit)
}

/// Build a test application router with state
pub fn build_test_app(db: PgPool) -> Router {
    let state = build_test_state(db);
    build_api_router().with_state(state)
}

/// Build a test application router with a custom state
pub fn build_test_app_with_state(state: AppState) -> Router {
    build_api_router().with_state(state)
}

/// Helper to read response body as bytes
pub async fn body_to_bytes(body: axum::body::Body) -> Bytes {
    body.collect()
        .await
        .expect("Failed to collect body")
        .to_bytes()
}

/// Helper to read response body as JSON Value
pub async fn body_to_json(body: axum::body::Body) -> serde_json::Value {
    let bytes = body_to_bytes(body).await;
    serde_json::from_slice(&bytes).expect("Failed to parse JSON from response body")
}
