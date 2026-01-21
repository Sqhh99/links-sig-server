//! Health check handler

use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;

use crate::types::HealthResponse;

/// Health check endpoint
///
/// GET /api/health
///
/// Returns server health status.
pub async fn handle_health() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        time: Utc::now().to_rfc3339(),
    };
    (StatusCode::OK, Json(response))
}
