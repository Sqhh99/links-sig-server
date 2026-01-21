//! Fake LiveKit service for testing
//!
//! Provides a mock implementation of LiveKitService that doesn't require
//! network access and allows configuring responses for different scenarios.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use links_sig_rust_server::integrations::LiveKitService;
use links_sig_rust_server::types::{ListParticipantsResponse, LiveKitParticipant, LiveKitRoom};

/// Fake LiveKit service for testing
///
/// This implementation stores rooms in memory and allows tests to
/// configure specific behaviors without network calls.
#[derive(Clone)]
pub struct FakeLiveKitService {
    /// In-memory rooms storage
    rooms: Arc<Mutex<Vec<LiveKitRoom>>>,
    /// In-memory participants storage (room_name -> participants)
    participants: Arc<Mutex<std::collections::HashMap<String, Vec<LiveKitParticipant>>>>,
    /// Optional error to return on next call
    next_error: Arc<Mutex<Option<String>>>,
}

impl FakeLiveKitService {
    /// Create a new fake LiveKit service
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(Mutex::new(Vec::new())),
            participants: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Add a pre-existing room for testing
    #[allow(dead_code)]
    pub fn with_room(self, room: LiveKitRoom) -> Self {
        self.rooms.lock().unwrap().push(room);
        self
    }

    /// Add a participant to a room for testing
    #[allow(dead_code)]
    pub fn with_participant(self, room_name: &str, participant: LiveKitParticipant) -> Self {
        {
            let mut participants = self.participants.lock().unwrap();
            participants
                .entry(room_name.to_string())
                .or_insert_with(Vec::new)
                .push(participant);
        }
        self
    }

    /// Set the next call to return an error
    #[allow(dead_code)]
    pub fn set_next_error(&self, error: &str) {
        *self.next_error.lock().unwrap() = Some(error.to_string());
    }

    /// Check and consume any pending error
    fn check_error(&self) -> Result<(), String> {
        let mut error = self.next_error.lock().unwrap();
        if let Some(e) = error.take() {
            return Err(e);
        }
        Ok(())
    }
}

impl Default for FakeLiveKitService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LiveKitService for FakeLiveKitService {
    async fn list_rooms(&self) -> Result<Vec<LiveKitRoom>, String> {
        self.check_error()?;
        let rooms = self.rooms.lock().unwrap();
        Ok(rooms.clone())
    }

    async fn create_room(
        &self,
        name: &str,
        empty_timeout: u32,
        max_participants: u32,
    ) -> Result<LiveKitRoom, String> {
        self.check_error()?;

        let room = LiveKitRoom {
            sid: Some(format!("RM_{}", uuid_simple())),
            name: name.to_string(),
            empty_timeout: Some(empty_timeout),
            max_participants: Some(max_participants),
            creation_time: Some(current_timestamp()),
            turn_password: None,
            enabled_codecs: None,
            metadata: Some(String::new()),
            num_participants: Some(0),
            num_publishers: Some(0),
            active_recording: Some(false),
        };

        self.rooms.lock().unwrap().push(room.clone());
        Ok(room)
    }

    async fn delete_room(&self, room_name: &str) -> Result<(), String> {
        self.check_error()?;

        let mut rooms = self.rooms.lock().unwrap();
        rooms.retain(|r| r.name != room_name);
        
        // Also remove participants
        let mut participants = self.participants.lock().unwrap();
        participants.remove(room_name);
        
        Ok(())
    }

    async fn list_participants(&self, room_name: &str) -> Result<ListParticipantsResponse, String> {
        self.check_error()?;

        let participants = self.participants.lock().unwrap();
        let room_participants = participants
            .get(room_name)
            .cloned()
            .unwrap_or_default();

        Ok(ListParticipantsResponse {
            participants: room_participants,
        })
    }

    async fn remove_participant(&self, room_name: &str, identity: &str) -> Result<(), String> {
        self.check_error()?;

        let mut participants = self.participants.lock().unwrap();
        if let Some(room_participants) = participants.get_mut(room_name) {
            room_participants.retain(|p| p.identity != identity);
        }
        Ok(())
    }
}

/// Generate a simple UUID-like string for testing
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

/// Get current timestamp in seconds
fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_livekit_create_room() {
        let service = FakeLiveKitService::new();
        
        let room = service.create_room("test-room", 300, 10).await.unwrap();
        
        assert_eq!(room.name, "test-room");
        assert_eq!(room.empty_timeout, Some(300));
        assert_eq!(room.max_participants, Some(10));
        assert!(room.sid.is_some());
    }

    #[tokio::test]
    async fn test_fake_livekit_list_rooms() {
        let service = FakeLiveKitService::new();
        
        // Initially empty
        let rooms = service.list_rooms().await.unwrap();
        assert!(rooms.is_empty());
        
        // Create a room
        service.create_room("test-room", 300, 10).await.unwrap();
        
        // Now should have one room
        let rooms = service.list_rooms().await.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "test-room");
    }

    #[tokio::test]
    async fn test_fake_livekit_error() {
        let service = FakeLiveKitService::new();
        service.set_next_error("Test error");
        
        let result = service.list_rooms().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Test error");
        
        // Next call should succeed
        let result = service.list_rooms().await;
        assert!(result.is_ok());
    }
}
