//! LiveKit API integration
//!
//! Provides a trait-based abstraction over the LiveKit API for easier testing
//! and a concrete implementation using the official LiveKit SDK.

use async_trait::async_trait;
use livekit_api::services::room::{CreateRoomOptions, RoomClient};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::types::{ListParticipantsResponse, LiveKitParticipant, LiveKitRoom};

/// LiveKit service trait - abstraction for LiveKit operations
///
/// This trait allows for easy mocking in tests while providing
/// a clean interface for LiveKit operations.
#[async_trait]
pub trait LiveKitService: Send + Sync {
    /// List all rooms
    async fn list_rooms(&self) -> Result<Vec<LiveKitRoom>, String>;

    /// Create a new room
    async fn create_room(
        &self,
        name: &str,
        empty_timeout: u32,
        max_participants: u32,
    ) -> Result<LiveKitRoom, String>;

    /// Delete a room
    async fn delete_room(&self, room_name: &str) -> Result<(), String>;

    /// List participants in a room
    async fn list_participants(&self, room_name: &str) -> Result<ListParticipantsResponse, String>;

    /// Remove a participant from a room
    async fn remove_participant(&self, room_name: &str, identity: &str) -> Result<(), String>;
}

/// LiveKit API client using official SDK
#[derive(Clone)]
pub struct LiveKitClient {
    #[allow(dead_code)]
    config: Arc<Config>,
    room_client: Arc<RoomClient>,
}

impl LiveKitClient {
    /// Create a new LiveKit client
    pub fn new(config: Arc<Config>) -> Self {
        let room_client =
            RoomClient::with_api_key(&config.livekit_url, &config.api_key, &config.api_secret);
        Self {
            config,
            room_client: Arc::new(room_client),
        }
    }
}

#[async_trait]
impl LiveKitService for LiveKitClient {
    async fn list_rooms(&self) -> Result<Vec<LiveKitRoom>, String> {
        match self.room_client.list_rooms(vec![]).await {
            Ok(rooms) => {
                let room_list: Vec<LiveKitRoom> = rooms
                    .into_iter()
                    .map(|r| LiveKitRoom {
                        sid: Some(r.sid),
                        name: r.name,
                        empty_timeout: Some(r.empty_timeout),
                        max_participants: Some(r.max_participants),
                        creation_time: Some(r.creation_time),
                        turn_password: None,
                        enabled_codecs: None,
                        metadata: Some(r.metadata),
                        num_participants: Some(r.num_participants),
                        num_publishers: Some(r.num_publishers),
                        active_recording: Some(r.active_recording),
                    })
                    .collect();
                Ok(room_list)
            }
            Err(e) => {
                error!("Failed to list rooms: {}", e);
                Err(format!("Failed to list rooms: {}", e))
            }
        }
    }

    async fn create_room(
        &self,
        name: &str,
        empty_timeout: u32,
        max_participants: u32,
    ) -> Result<LiveKitRoom, String> {
        let options = CreateRoomOptions {
            empty_timeout,
            max_participants,
            ..Default::default()
        };

        match self.room_client.create_room(name, options).await {
            Ok(room) => {
                info!("Room created: {}", room.name);
                Ok(LiveKitRoom {
                    sid: Some(room.sid),
                    name: room.name,
                    empty_timeout: Some(room.empty_timeout),
                    max_participants: Some(room.max_participants),
                    creation_time: Some(room.creation_time),
                    turn_password: None,
                    enabled_codecs: None,
                    metadata: Some(room.metadata),
                    num_participants: Some(room.num_participants),
                    num_publishers: Some(room.num_publishers),
                    active_recording: Some(room.active_recording),
                })
            }
            Err(e) => {
                error!("Failed to create room: {}", e);
                Err(format!("Failed to create room: {}", e))
            }
        }
    }

    async fn delete_room(&self, room_name: &str) -> Result<(), String> {
        match self.room_client.delete_room(room_name).await {
            Ok(()) => {
                info!("Room deleted: {}", room_name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete room: {}", e);
                Err(format!("Failed to delete room: {}", e))
            }
        }
    }

    async fn list_participants(&self, room_name: &str) -> Result<ListParticipantsResponse, String> {
        match self.room_client.list_participants(room_name).await {
            Ok(participants) => {
                let participant_list: Vec<LiveKitParticipant> = participants
                    .into_iter()
                    .map(|p| LiveKitParticipant {
                        sid: Some(p.sid),
                        identity: p.identity,
                        state: Some(p.state),
                        tracks: None,
                        metadata: Some(p.metadata),
                        joined_at: Some(p.joined_at),
                        name: Some(p.name),
                        version: Some(p.version),
                        permission: None,
                        region: Some(p.region),
                        is_publisher: Some(p.is_publisher),
                    })
                    .collect();
                Ok(ListParticipantsResponse {
                    participants: participant_list,
                })
            }
            Err(e) => {
                // This might fail if room doesn't exist yet, which is expected
                warn!("List participants failed (room may not exist): {}", e);
                Err(format!("Failed to list participants: {}", e))
            }
        }
    }

    async fn remove_participant(&self, room_name: &str, identity: &str) -> Result<(), String> {
        match self
            .room_client
            .remove_participant(room_name, identity)
            .await
        {
            Ok(()) => {
                info!(
                    "Participant '{}' removed from room '{}'",
                    identity, room_name
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove participant: {}", e);
                Err(format!("Failed to remove participant: {}", e))
            }
        }
    }
}
