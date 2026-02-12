//! Meeting registry service
//!
//! Business-level meeting management using 9-digit meeting numbers.

use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::config::Config;
use crate::integrations::LiveKitService;
use crate::services::AuthService;
use crate::types::{
    AppError, CreateMeetingResponse, JoinMeetingRequest, JoinMeetingResponse, LeaveMeetingResponse,
    MeetingRecordItem, MeetingRecordListResponse,
};

const MEETING_NUMBER_MAX: u32 = 1_000_000_000;
const MEETING_NUMBER_RETRY: usize = 8;

#[derive(Debug, Clone, sqlx::FromRow)]
struct MeetingRow {
    id: Uuid,
    meeting_no: String,
    room_name: String,
    creator_user_id: Uuid,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct MeetingRecordRow {
    meeting_no: String,
    room_name: String,
    meeting_status: String,
    creator_user_id: Uuid,
    first_joined_at: DateTime<Utc>,
    last_joined_at: DateTime<Utc>,
    join_count: i32,
}

/// Service for business meetings and per-user meeting records.
pub struct MeetingRegistryService;

impl MeetingRegistryService {
    /// Create a meeting with a unique 9-digit meeting number.
    pub async fn create_meeting(
        db: &PgPool,
        config: &Config,
        creator_user_id: Uuid,
        display_name: Option<String>,
    ) -> Result<CreateMeetingResponse, AppError> {
        let display_name = display_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        for _ in 0..MEETING_NUMBER_RETRY {
            let meeting_no = {
                let mut rng = rand::thread_rng();
                format!("{:09}", rng.gen_range(0..MEETING_NUMBER_MAX))
            };
            let room_name = format!("m-{}", meeting_no);

            let inserted = sqlx::query_as::<_, MeetingRow>(
                r#"
                INSERT INTO meetings (meeting_no, room_name, creator_user_id)
                VALUES ($1, $2, $3)
                RETURNING id, meeting_no, room_name, creator_user_id, status, created_at
                "#,
            )
            .bind(&meeting_no)
            .bind(&room_name)
            .bind(creator_user_id)
            .fetch_one(db)
            .await;

            match inserted {
                Ok(row) => {
                    return Ok(CreateMeetingResponse {
                        meeting_no: row.meeting_no.clone(),
                        room_name: row.room_name,
                        share_url: Self::build_share_url(config, &row.meeting_no),
                        display_name: display_name.clone(),
                        created_at: row.created_at,
                    });
                }
                Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                    continue;
                }
                Err(err) => {
                    return Err(AppError::internal(format!(
                        "Failed to create meeting: {}",
                        err
                    )));
                }
            }
        }

        Err(AppError::internal(
            "Failed to generate unique meeting number",
        ))
    }

    /// Join a meeting by meeting number and update per-user join record.
    pub async fn join_meeting(
        db: &PgPool,
        config: &Config,
        meeting_no: &str,
        user_id: Uuid,
        user_email: &str,
        req: JoinMeetingRequest,
    ) -> Result<JoinMeetingResponse, AppError> {
        if !Self::is_valid_meeting_no(meeting_no) {
            return Err(AppError::bad_request("meeting_no must be 9 digits"));
        }

        let meeting = sqlx::query_as::<_, MeetingRow>(
            r#"
            SELECT id, meeting_no, room_name, creator_user_id, status, created_at
            FROM meetings
            WHERE meeting_no = $1
            "#,
        )
        .bind(meeting_no)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query meeting: {}", e)))?
        .ok_or_else(|| AppError::not_found("Meeting not found"))?;

        if meeting.status != "active" {
            return Err(AppError::conflict("Meeting has ended"));
        }

        let mut participant_name = req.participant_name.trim().to_string();
        if participant_name.is_empty() {
            participant_name = user_email.to_string();
        }

        let is_host = meeting.creator_user_id == user_id;
        let identity = user_id.to_string();

        let token = AuthService::generate_token_for_room_with_identity(
            config,
            meeting.room_name.clone(),
            identity,
            Some(participant_name),
            is_host,
        )?;

        sqlx::query(
            r#"
            INSERT INTO meeting_participants (
                meeting_id, user_id, first_joined_at, last_joined_at, join_count
            )
            VALUES ($1, $2, NOW(), NOW(), 1)
            ON CONFLICT (meeting_id, user_id)
            DO UPDATE SET
                last_joined_at = NOW(),
                join_count = meeting_participants.join_count + 1
            "#,
        )
        .bind(meeting.id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to upsert meeting participant: {}", e)))?;

        Ok(JoinMeetingResponse {
            meeting_no: meeting.meeting_no,
            token: token.token,
            url: token.url,
            room_name: token.room_name,
            is_host: token.is_host,
        })
    }

    /// Leave a meeting by removing current user's participant identity from LiveKit room.
    ///
    /// The operation is idempotent:
    /// - If participant exists in room, it is removed and `left = true`
    /// - If participant does not exist (or room is already gone), `left = false`
    pub async fn leave_meeting<L: LiveKitService + ?Sized>(
        db: &PgPool,
        livekit: &L,
        meeting_no: &str,
        user_id: Uuid,
    ) -> Result<LeaveMeetingResponse, AppError> {
        if !Self::is_valid_meeting_no(meeting_no) {
            return Err(AppError::bad_request("meeting_no must be 9 digits"));
        }

        let meeting = sqlx::query_as::<_, MeetingRow>(
            r#"
            SELECT id, meeting_no, room_name, creator_user_id, status, created_at
            FROM meetings
            WHERE meeting_no = $1
            "#,
        )
        .bind(meeting_no)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query meeting: {}", e)))?
        .ok_or_else(|| AppError::not_found("Meeting not found"))?;

        let identity = user_id.to_string();

        let participants_before = match livekit.list_participants(&meeting.room_name).await {
            Ok(resp) => Some(resp.participants),
            Err(err) => {
                // Room may already be gone. Keep leave idempotent.
                warn!(
                    "Leave meeting list participants failed for room '{}': {}",
                    meeting.room_name, err
                );
                if Self::is_room_unavailable_error(&err) {
                    Self::mark_meeting_ended_by_id(db, meeting.id).await?;
                }
                None
            }
        };

        let already_present = participants_before
            .as_ref()
            .is_some_and(|participants| participants.iter().any(|p| p.identity == identity));
        if already_present {
            livekit
                .remove_participant(&meeting.room_name, &identity)
                .await
                .map_err(AppError::internal)?;
        }

        let room_is_empty = if already_present {
            match livekit.list_participants(&meeting.room_name).await {
                Ok(resp) => Some(resp.participants.is_empty()),
                Err(err) => {
                    warn!(
                        "Leave meeting post-remove list participants failed for room '{}': {}",
                        meeting.room_name, err
                    );
                    Self::is_room_unavailable_error(&err).then_some(true)
                }
            }
        } else {
            participants_before.as_ref().map(Vec::is_empty)
        };

        if room_is_empty == Some(true) {
            Self::mark_meeting_ended_by_id(db, meeting.id).await?;
        }

        Ok(LeaveMeetingResponse {
            message: if already_present {
                "Left meeting".to_string()
            } else {
                "Already left meeting".to_string()
            },
            meeting_no: meeting.meeting_no,
            room_name: meeting.room_name,
            identity,
            left: already_present,
        })
    }

    /// Returns creator user id for a room if it is a business meeting room.
    pub async fn get_meeting_creator_by_room_name(
        db: &PgPool,
        room_name: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let creator_user_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT creator_user_id
            FROM meetings
            WHERE room_name = $1
            LIMIT 1
            "#,
        )
        .bind(room_name)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query meeting by room: {}", e)))?;

        Ok(creator_user_id)
    }

    /// Mark a meeting as ended by meeting id.
    ///
    /// This transition is one-way: only `active -> ended`.
    pub async fn mark_meeting_ended_by_id(db: &PgPool, meeting_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'ended',
                ended_at = COALESCE(ended_at, NOW())
            WHERE id = $1
              AND status = 'active'
            "#,
        )
        .bind(meeting_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to mark meeting ended: {}", e)))?;

        Ok(())
    }

    /// Mark a meeting as ended by room name.
    ///
    /// No-op if room is not a business meeting room.
    pub async fn mark_meeting_ended_by_room_name(
        db: &PgPool,
        room_name: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'ended',
                ended_at = COALESCE(ended_at, NOW())
            WHERE room_name = $1
              AND status = 'active'
            "#,
        )
        .bind(room_name)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to mark meeting ended: {}", e)))?;

        Ok(())
    }

    /// List current user's meeting records.
    pub async fn list_user_records(
        db: &PgPool,
        user_id: Uuid,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<MeetingRecordListResponse, AppError> {
        let page = page.unwrap_or(1).max(1);
        let page_size = page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * page_size) as i64;

        let rows = sqlx::query_as::<_, MeetingRecordRow>(
            r#"
            SELECT
                m.meeting_no,
                m.room_name,
                m.status AS meeting_status,
                m.creator_user_id,
                mp.first_joined_at,
                mp.last_joined_at,
                mp.join_count
            FROM meeting_participants mp
            INNER JOIN meetings m ON m.id = mp.meeting_id
            WHERE mp.user_id = $1
            ORDER BY mp.last_joined_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list meeting records: {}", e)))?;

        let records = rows
            .into_iter()
            .map(|row| MeetingRecordItem {
                meeting_no: row.meeting_no,
                room_name: row.room_name,
                meeting_status: row.meeting_status,
                creator_user_id: row.creator_user_id,
                first_joined_at: row.first_joined_at,
                last_joined_at: row.last_joined_at,
                join_count: row.join_count,
            })
            .collect();

        Ok(MeetingRecordListResponse {
            records,
            page,
            page_size,
        })
    }

    fn build_share_url(config: &Config, meeting_no: &str) -> String {
        let base = config.app_base_url.trim_end_matches('/');
        format!("{}/join?meetingNo={}", base, meeting_no)
    }

    fn is_valid_meeting_no(meeting_no: &str) -> bool {
        meeting_no.len() == 9 && meeting_no.chars().all(|c| c.is_ascii_digit())
    }

    fn is_room_unavailable_error(err: &str) -> bool {
        let normalized = err.to_ascii_lowercase();
        normalized.contains("not found")
            || normalized.contains("does not exist")
            || normalized.contains("no such room")
    }
}
