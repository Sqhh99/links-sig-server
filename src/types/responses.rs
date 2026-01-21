//! Response DTOs - Outgoing response body structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Room information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub name: String,
    pub display_name: String,
    pub participants: i32,
    pub created_at: DateTime<Utc>,
}

/// Token response structure
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub token: String,
    pub url: String,
    pub room_name: String,
    pub is_host: bool,
}

/// Error response
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub time: String,
}

/// Generic message response
#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
}

/// LiveKit Room from API response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveKitRoom {
    pub sid: Option<String>,
    pub name: String,
    pub empty_timeout: Option<u32>,
    pub max_participants: Option<u32>,
    pub creation_time: Option<i64>,
    pub turn_password: Option<String>,
    pub enabled_codecs: Option<Vec<serde_json::Value>>,
    pub metadata: Option<String>,
    pub num_participants: Option<u32>,
    pub num_publishers: Option<u32>,
    pub active_recording: Option<bool>,
}

/// LiveKit Participant from API response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveKitParticipant {
    pub sid: Option<String>,
    pub identity: String,
    pub state: Option<i32>,
    pub tracks: Option<Vec<serde_json::Value>>,
    pub metadata: Option<String>,
    pub joined_at: Option<i64>,
    pub name: Option<String>,
    pub version: Option<u32>,
    pub permission: Option<serde_json::Value>,
    pub region: Option<String>,
    pub is_publisher: Option<bool>,
}

/// List rooms response
#[derive(Debug, Clone, Deserialize)]
pub struct ListRoomsResponse {
    #[serde(default)]
    pub rooms: Vec<LiveKitRoom>,
}

/// List participants response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParticipantsResponse {
    #[serde(default)]
    pub participants: Vec<LiveKitParticipant>,
}
