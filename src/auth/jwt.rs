//! JWT token handling for LiveKit access tokens
//!
//! Provides token generation with VideoGrant permissions.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Video grant for LiveKit access token
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoGrant {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_create: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_list: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_record: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_admin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_join: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_publish: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_publish_data: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_publish_sources: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_update_own_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingress_admin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<bool>,
}

impl Default for VideoGrant {
    fn default() -> Self {
        Self {
            room_create: None,
            room_list: None,
            room_record: None,
            room_admin: None,
            room_join: None,
            room: None,
            can_publish: None,
            can_subscribe: None,
            can_publish_data: None,
            can_publish_sources: None,
            can_update_own_metadata: None,
            ingress_admin: None,
            hidden: None,
            recorder: None,
            agent: None,
        }
    }
}

/// LiveKit access token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at time (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<u64>,
    /// Not before time (Unix timestamp)
    pub nbf: u64,
    /// Issuer (API Key)
    pub iss: String,
    /// Subject (Participant identity)
    pub sub: String,
    /// Video grant
    pub video: VideoGrant,
    /// Participant metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    /// Participant name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Access token builder
pub struct AccessToken {
    api_key: String,
    api_secret: String,
    identity: String,
    name: Option<String>,
    metadata: Option<String>,
    video_grant: VideoGrant,
    valid_for_seconds: u64,
}

impl AccessToken {
    /// Create a new access token builder
    pub fn new(api_key: &str, api_secret: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            identity: String::new(),
            name: None,
            metadata: None,
            video_grant: VideoGrant::default(),
            valid_for_seconds: 6 * 60 * 60, // 6 hours default
        }
    }

    /// Set participant identity
    pub fn set_identity(mut self, identity: &str) -> Self {
        self.identity = identity.to_string();
        self
    }

    /// Set participant name
    #[allow(dead_code)]
    pub fn set_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Set participant metadata
    pub fn set_metadata(mut self, metadata: &str) -> Self {
        self.metadata = Some(metadata.to_string());
        self
    }

    /// Set video grant
    pub fn set_video_grant(mut self, grant: VideoGrant) -> Self {
        self.video_grant = grant;
        self
    }

    /// Set token validity duration in seconds
    pub fn set_valid_for(mut self, seconds: u64) -> Self {
        self.valid_for_seconds = seconds;
        self
    }

    /// Generate JWT token
    pub fn to_jwt(&self) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = AccessTokenClaims {
            exp: now + self.valid_for_seconds,
            iat: None, // Go SDK doesn't include iat
            nbf: now,
            iss: self.api_key.clone(),
            sub: self.identity.clone(),
            video: self.video_grant.clone(),
            metadata: self.metadata.clone(),
            name: self.name.clone(),
        };

        let header = Header::new(Algorithm::HS256);
        let key = EncodingKey::from_secret(self.api_secret.as_bytes());

        encode(&header, &claims, &key)
    }
}

/// Decode and validate a JWT token
pub fn decode_token(
    token: &str,
    api_secret: &str,
) -> Result<AccessTokenClaims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(api_secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    // Don't validate iss/sub as they vary
    validation.set_required_spec_claims(&["exp", "nbf"]);

    let token_data = decode::<AccessTokenClaims>(token, &key, &validation)?;
    Ok(token_data.claims)
}

/// Create an access token for room operations (admin)
#[allow(dead_code)]
pub fn create_admin_token(
    api_key: &str,
    api_secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let grant = VideoGrant {
        room_create: Some(true),
        room_list: Some(true),
        room_record: Some(true),
        room_admin: Some(true),
        room_join: Some(true),
        can_publish: Some(true),
        can_subscribe: Some(true),
        can_publish_data: Some(true),
        ..Default::default()
    };

    AccessToken::new(api_key, api_secret)
        .set_identity("admin-service")
        .set_video_grant(grant)
        .set_valid_for(300) // 5 minutes for admin operations
        .to_jwt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_encode_decode() {
        let api_key = "test-api-key";
        let api_secret = "test-api-secret";

        let grant = VideoGrant {
            room_join: Some(true),
            room: Some("test-room".to_string()),
            can_publish: Some(true),
            can_subscribe: Some(true),
            ..Default::default()
        };

        let token = AccessToken::new(api_key, api_secret)
            .set_identity("test-user")
            .set_metadata(r#"{"isHost":true}"#)
            .set_video_grant(grant)
            .set_valid_for(3600)
            .to_jwt()
            .expect("Failed to create token");

        // Verify token can be decoded
        let claims = decode_token(&token, api_secret).expect("Failed to decode token");

        assert_eq!(claims.iss, api_key);
        assert_eq!(claims.sub, "test-user");
        assert_eq!(claims.video.room, Some("test-room".to_string()));
        assert_eq!(claims.video.room_join, Some(true));
        assert_eq!(claims.metadata, Some(r#"{"isHost":true}"#.to_string()));
    }

    #[test]
    fn test_token_with_invalid_secret() {
        let api_key = "test-api-key";
        let api_secret = "test-api-secret";

        let token = AccessToken::new(api_key, api_secret)
            .set_identity("test-user")
            .set_valid_for(3600)
            .to_jwt()
            .expect("Failed to create token");

        // Decoding with wrong secret should fail
        let result = decode_token(&token, "wrong-secret");
        assert!(result.is_err());
    }
}
