//! Response DTOs - Outgoing response body structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// LiveKit/Meeting Responses
// ============================================================================

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

// ============================================================================
// Auth Responses
// ============================================================================

/// Response for verification code request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCodeResponse {
    pub message: String,
    /// Seconds until another code can be requested
    pub retry_after_secs: u64,
}

/// Response for successful registration
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub email: String,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Response for successful login
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub email: String,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Response for refreshing user token
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenResponse {
    pub user_id: Uuid,
    pub email: String,
    pub token: String,
    pub expires_in_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Response for creating a meeting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMeetingResponse {
    pub meeting_no: String,
    pub room_name: String,
    pub share_url: String,
    pub status: String,
    pub topic: String,
    pub scheduled_start_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    pub allow_guest_join: bool,
    pub requires_password: bool,
    pub no_join_auto_end_minutes: i32,
    pub empty_auto_end_minutes: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response for joining a meeting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinMeetingResponse {
    pub meeting_no: String,
    pub token: String,
    pub url: String,
    pub room_name: String,
    pub is_host: bool,
}

/// Response for leaving a meeting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaveMeetingResponse {
    pub message: String,
    pub meeting_no: String,
    pub room_name: String,
    pub identity: String,
    pub left: bool,
}

/// Meeting record list item for current user
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRecordItem {
    pub meeting_no: String,
    pub room_name: String,
    pub meeting_status: String,
    pub creator_user_id: Uuid,
    pub first_joined_at: DateTime<Utc>,
    pub last_joined_at: DateTime<Utc>,
    pub join_count: i32,
}

/// Paginated response for current user's meeting records
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRecordListResponse {
    pub records: Vec<MeetingRecordItem>,
    pub page: u32,
    pub page_size: u32,
}

/// Host meeting list item for current user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMeetingItem {
    pub meeting_no: String,
    pub room_name: String,
    pub topic: String,
    pub status: String,
    pub scheduled_start_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<DateTime<Utc>>,
    pub allow_guest_join: bool,
    pub requires_password: bool,
    pub no_join_auto_end_minutes: i32,
    pub empty_auto_end_minutes: i32,
    pub created_at: DateTime<Utc>,
}

/// Paginated response for host meetings.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMeetingListResponse {
    pub meetings: Vec<HostMeetingItem>,
    pub page: u32,
    pub page_size: u32,
}

/// User profile response (for future use)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UserProfileResponse {
    pub user_id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}
