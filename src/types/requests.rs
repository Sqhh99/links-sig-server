//! Request DTOs - Incoming request body structures

use chrono::{DateTime, Utc};
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
    #[serde(default)]
    pub allow_guest_join: Option<bool>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub scheduled_start_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub no_join_auto_end_minutes: Option<u32>,
    #[serde(default)]
    pub empty_auto_end_minutes: Option<u32>,
}

/// Join meeting request
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingRequest {
    #[serde(default)]
    pub participant_name: String,
    #[serde(default)]
    pub meeting_password: Option<String>,
}

/// Guest join meeting request
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuestJoinMeetingRequest {
    #[serde(default)]
    pub participant_name: String,
    #[serde(default)]
    pub meeting_password: Option<String>,
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

/// Query params for listing host meetings created by current user.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMeetingsQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub time_from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub time_to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub include_ended: Option<bool>,
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
    /// Optional profile display name
    #[serde(default, rename = "displayName")]
    pub display_name: Option<String>,
}

/// Login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
}
