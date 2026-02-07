//! Request DTOs - Incoming request body structures

use serde::Deserialize;

// ============================================================================
// LiveKit/Meeting Requests
// ============================================================================

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

/// Create meeting request
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateMeetingRequest {
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Join meeting request
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingRequest {
    #[serde(default)]
    pub participant_name: String,
}

/// Query params for listing current user's meeting records
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRecordsQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
}

// ============================================================================
// Auth Requests
// ============================================================================

/// Request verification code for registration
#[derive(Debug, Clone, Deserialize)]
pub struct RequestRegisterCodeRequest {
    /// Email address to send the verification code to
    pub email: String,
}

/// Complete registration with verification code
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    /// Email address
    pub email: String,
    /// Verification code received via email
    pub code: String,
    /// Password (will be hashed)
    pub password: String,
}

/// Login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
}
