//! Meeting service - Room and participant management
//!
//! Handles all meeting-related business logic including room CRUD
//! and participant management.

use chrono::{TimeZone, Utc};
use tracing::{error, info};

use crate::integrations::LiveKitService;
use crate::types::{AppError, CreateRoomRequest, ListParticipantsResponse, MessageResponse, Room};

/// Meeting service for room and participant operations
pub struct MeetingService;

impl MeetingService {
    fn is_room_unavailable_error(err: &str) -> bool {
        let normalized = err.to_ascii_lowercase();
        normalized.contains("not_found")
            || normalized.contains("not found")
            || normalized.contains("does not exist")
            || normalized.contains("requested room does not exist")
            || normalized.contains("no such room")
    }

    /// List all active rooms
    pub async fn list_rooms<L: LiveKitService + ?Sized>(
        livekit: &L,
    ) -> Result<Vec<Room>, AppError> {
        let rooms = livekit.list_rooms().await.map_err(AppError::internal)?;

        let room_list: Vec<Room> = rooms
            .into_iter()
            .map(|r| Room {
                name: r.name.clone(),
                display_name: r.name.clone(),
                participants: r.num_participants.unwrap_or(0) as i32,
                created_at: r
                    .creation_time
                    .map(|t| Utc.timestamp_opt(t, 0).unwrap())
                    .unwrap_or_else(Utc::now),
            })
            .collect();

        Ok(room_list)
    }

    /// Create a new room
    pub async fn create_room<L: LiveKitService + ?Sized>(
        livekit: &L,
        mut req: CreateRoomRequest,
    ) -> Result<Room, AppError> {
        if req.name.is_empty() {
            req.name = format!("room-{}", Utc::now().timestamp());
        }

        let room = livekit
            .create_room(&req.name, 300, 50)
            .await
            .map_err(AppError::internal)?;

        Ok(Room {
            name: room.name.clone(),
            display_name: room.name.clone(),
            participants: room.num_participants.unwrap_or(0) as i32,
            created_at: room
                .creation_time
                .map(|t| Utc.timestamp_opt(t, 0).unwrap())
                .unwrap_or_else(Utc::now),
        })
    }

    /// Delete a room
    pub async fn delete_room<L: LiveKitService + ?Sized>(
        livekit: &L,
        room_name: &str,
    ) -> Result<MessageResponse, AppError> {
        livekit
            .delete_room(room_name)
            .await
            .map_err(AppError::internal)?;

        Ok(MessageResponse {
            message: "Room deleted".to_string(),
            identity: None,
        })
    }

    /// List participants in a room
    pub async fn list_participants<L: LiveKitService + ?Sized>(
        livekit: &L,
        room_name: &str,
    ) -> Result<ListParticipantsResponse, AppError> {
        livekit
            .list_participants(room_name)
            .await
            .map_err(AppError::internal)
    }

    /// Kick a participant from a room
    pub async fn kick_participant<L: LiveKitService + ?Sized>(
        livekit: &L,
        room_name: &str,
        identity: &str,
    ) -> Result<MessageResponse, AppError> {
        livekit
            .remove_participant(room_name, identity)
            .await
            .map_err(AppError::internal)?;

        Ok(MessageResponse {
            message: "Participant removed".to_string(),
            identity: Some(identity.to_string()),
        })
    }

    /// End a meeting (kick all participants and delete room)
    pub async fn end_meeting<L: LiveKitService + ?Sized>(
        livekit: &L,
        room_name: &str,
    ) -> Result<MessageResponse, AppError> {
        // Get all participants
        let participants = livekit
            .list_participants(room_name)
            .await
            .map_err(|e| AppError::internal(format!("Failed to end meeting: {}", e)))?;

        // Kick all participants
        for p in participants.participants {
            if let Err(e) = livekit.remove_participant(room_name, &p.identity).await {
                error!("Failed to remove participant '{}': {}", p.identity, e);
            }
        }

        // Delete the room
        if let Err(e) = livekit.delete_room(room_name).await {
            if Self::is_room_unavailable_error(&e) {
                info!(
                    "Room '{}' already absent while ending meeting, treated as ended",
                    room_name
                );
            } else {
                error!("Failed to delete room: {}", e);
            }
        }

        info!("Meeting '{}' ended, all participants removed", room_name);

        Ok(MessageResponse {
            message: "Meeting ended".to_string(),
            identity: None,
        })
    }
}
