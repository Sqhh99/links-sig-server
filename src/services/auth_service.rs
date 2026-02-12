//! Auth service - Token generation and user state management
//!
//! Handles JWT token generation with LiveKit grants.

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
    /// 2. Determines if user should be host (first in room)
    /// 3. Creates appropriate video grants
    /// 4. Generates and returns the JWT token
    pub async fn generate_token<L: LiveKitService + ?Sized>(
        config: &Config,
        livekit: &L,
        mut req: TokenRequest,
    ) -> Result<TokenResponse, AppError> {
        // Set defaults
        if req.room_name.is_empty() {
            req.room_name = "default-room".to_string();
        }
        if req.participant_name.is_empty() {
            req.participant_name = format!("user-{}", chrono::Utc::now().timestamp());
        }

        // Trim whitespace
        req.room_name = req.room_name.trim().to_string();
        req.participant_name = req.participant_name.trim().to_string();

        // Check if user should be host
        let is_host = Self::determine_host_status(livekit, &req).await;

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

    /// Determine if the user should be marked as host
    ///
    /// User becomes host if:
    /// 1. Explicitly requested (`is_host = true`)
    /// 2. Room is empty or doesn't exist
    async fn determine_host_status<L: LiveKitService + ?Sized>(
        livekit: &L,
        req: &TokenRequest,
    ) -> bool {
        if req.is_host {
            return true;
        }

        // Check if room has participants
        match livekit.list_participants(&req.room_name).await {
            Ok(participants) => {
                if participants.participants.is_empty() {
                    info!(
                        "User '{}' is host of room '{}'",
                        req.participant_name, req.room_name
                    );
                    true
                } else {
                    false
                }
            }
            Err(_) => {
                // Room doesn't exist or error, user is host
                info!(
                    "User '{}' is host of room '{}'",
                    req.participant_name, req.room_name
                );
                true
            }
        }
    }
}
