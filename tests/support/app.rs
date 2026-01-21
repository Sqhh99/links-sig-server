//! Test application builder
//!
//! Provides helper functions to build test instances of the application.

use axum::Router;
use http_body_util::BodyExt;
use hyper::body::Bytes;

use links_sig_rust_server::{build_api_router, AppState, Config};

use super::fake_livekit::FakeLiveKitService;

/// Build a test configuration with default values
pub fn build_test_config() -> Config {
    Config::for_tests()
}

/// Build a test AppState with FakeLiveKitService
pub fn build_test_state() -> AppState {
    let config = build_test_config();
    let fake_livekit = FakeLiveKitService::new();
    AppState::with_livekit(config, fake_livekit)
}

/// Build a test AppState with a custom FakeLiveKitService
pub fn build_test_state_with_livekit(fake_livekit: FakeLiveKitService) -> AppState {
    let config = build_test_config();
    AppState::with_livekit(config, fake_livekit)
}

/// Build a test application router with state
pub fn build_test_app() -> Router {
    let state = build_test_state();
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
