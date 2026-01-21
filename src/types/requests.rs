//! Request DTOs - Incoming request body structures

use serde::Deserialize;

/// Token request structure
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequest {
    #[serde(default)]
    pub room_name: String,
    #[serde(default)]
    pub participant_name: String,
    #[serde(default)]
    pub is_host: bool,
}

/// Create room request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRoomRequest {
    #[serde(default)]
    pub name: String,
}
