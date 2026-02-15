//! Auth service - Token generation and user state management
//!
//! Handles JWT token generation with LiveKit grants.

use chrono::Utc;
use tracing::info;

use crate::auth::jwt::{AccessToken, VideoGrant};
use crate::config::Config;
use crate::integrations::LiveKitService;
use crate::types::{AppError, TokenRequest, TokenResponse};

/// Auth service for token generation
pub struct AuthService;

impl AuthService {
    /// Generate a LiveKit access token for a user
    ///
    /// This method:
    /// 1. Validates and normalizes the request
    /// 2. Ensures the request is for a non-business room
    /// 3. Allows joining existing rooms only (no implicit room creation)
    /// 4. Always issues guest token as non-host
    /// 5. Generates and returns the JWT token
    pub async fn generate_token<L: LiveKitService + ?Sized>(
        config: &Config,
        livekit: &L,
        mut req: TokenRequest,
    ) -> Result<TokenResponse, AppError> {
        // Trim whitespace
        req.room_name = req.room_name.trim().to_string();
        req.participant_name = req.participant_name.trim().to_string();

        if req.room_name.is_empty() {
            return Err(AppError::bad_request("roomName is required"));
        }

        if Self::is_business_meeting_room(&req.room_name) {
            return Err(AppError::forbidden(
                "Business meetings must be joined via /api/meetings/{meeting_no}/join",
            ));
        }

        if req.participant_name.is_empty() {
            req.participant_name = format!("user-{}", Utc::now().timestamp());
        }

        if req.is_host {
            info!(
                "Ignoring isHost=true for guest token request in room '{}'",
                req.room_name
            );
        }

        if !Self::room_exists(livekit, &req.room_name).await? {
            return Err(AppError::not_found("Room not found"));
        }

        let is_host = false;

        let response = Self::generate_token_for_room(
            config,
            req.room_name.clone(),
            req.participant_name.clone(),
            is_host,
        )?;

        info!(
            "Token generated for user '{}' in room '{}' (is_host: {})",
            req.participant_name, req.room_name, is_host
        );

        Ok(response)
    }

    /// Generate a LiveKit token for a specific room and host status.
    ///
    /// This bypasses host auto-detection and is used by business-level meeting APIs.
    pub fn generate_token_for_room(
        config: &Config,
        room_name: String,
        participant_name: String,
        is_host: bool,
    ) -> Result<TokenResponse, AppError> {
        Self::generate_token_for_room_with_identity(
            config,
            room_name,
            participant_name.clone(),
            Some(participant_name),
            is_host,
        )
    }

    /// Generate a LiveKit token with explicit identity and optional display name.
    pub fn generate_token_for_room_with_identity(
        config: &Config,
        room_name: String,
        identity: String,
        display_name: Option<String>,
        is_host: bool,
    ) -> Result<TokenResponse, AppError> {
        let grant = VideoGrant {
            room_join: Some(true),
            room: Some(room_name.clone()),
            can_publish: Some(true),
            can_subscribe: Some(true),
            ..Default::default()
        };

        let metadata = format!(r#"{{"isHost":{}}}"#, is_host);

        let mut token_builder = AccessToken::new(&config.api_key, &config.api_secret)
            .set_identity(&identity)
            .set_metadata(&metadata)
            .set_video_grant(grant)
            .set_valid_for(24 * 60 * 60);

        if let Some(name) = display_name {
            let name = name.trim();
            if !name.is_empty() {
                token_builder = token_builder.set_name(name);
            }
        }

        let token = token_builder
            .to_jwt()
            .map_err(|e| AppError::internal(format!("Failed to generate token: {}", e)))?;

        Ok(TokenResponse {
            token,
            url: config.livekit_ws_url.clone(),
            room_name,
            is_host,
        })
    }

    /// Generate a guest token that can only subscribe in a room.
    ///
    /// Guest token is always non-host and carries `isGuest=true` metadata.
    pub fn generate_guest_token_for_room_with_identity(
        config: &Config,
        room_name: String,
        identity: String,
        display_name: Option<String>,
    ) -> Result<TokenResponse, AppError> {
        let grant = VideoGrant {
            room_join: Some(true),
            room: Some(room_name.clone()),
            can_publish: Some(false),
            can_subscribe: Some(true),
            can_publish_data: Some(false),
            ..Default::default()
        };

        let metadata = r#"{"isHost":false,"isGuest":true}"#;

        let mut token_builder = AccessToken::new(&config.api_key, &config.api_secret)
            .set_identity(&identity)
            .set_metadata(metadata)
            .set_video_grant(grant)
            .set_valid_for(24 * 60 * 60);

        if let Some(name) = display_name {
            let name = name.trim();
            if !name.is_empty() {
                token_builder = token_builder.set_name(name);
            }
        }

        let token = token_builder
            .to_jwt()
            .map_err(|e| AppError::internal(format!("Failed to generate token: {}", e)))?;

        Ok(TokenResponse {
            token,
            url: config.livekit_ws_url.clone(),
            room_name,
            is_host: false,
        })
    }

    async fn room_exists<L: LiveKitService + ?Sized>(
        livekit: &L,
        room_name: &str,
    ) -> Result<bool, AppError> {
        let rooms = livekit.list_rooms().await.map_err(AppError::internal)?;
        Ok(rooms.iter().any(|room| room.name == room_name))
    }

    fn is_business_meeting_room(room_name: &str) -> bool {
        if !room_name.starts_with("m-") {
            return false;
        }
        let meeting_no = &room_name[2..];
        meeting_no.len() == 9 && meeting_no.chars().all(|c| c.is_ascii_digit())
    }
}
