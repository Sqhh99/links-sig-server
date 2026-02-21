//! Meeting registry service
//!
//! Business-level meeting management using 9-digit meeting numbers.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::config::Config;
use crate::integrations::LiveKitService;
use crate::services::{AuthService, MeetingService};
use crate::types::{
    AppError, CreateMeetingResponse, GuestJoinMeetingRequest, HostMeetingItem,
    HostMeetingListResponse, HostMeetingsQuery, JoinMeetingRequest, JoinMeetingResponse,
    LeaveMeetingResponse, MeetingRecordItem, MeetingRecordListResponse, MessageResponse,
};

const MEETING_NUMBER_MAX: u32 = 1_000_000_000;
const MEETING_NUMBER_RETRY: usize = 8;
const PASSWORD_MIN_LEN: usize = 6;
const PASSWORD_MAX_LEN: usize = 32;

const CODE_MEETING_NOT_STARTED: &str = "MEETING_NOT_STARTED";
const CODE_HOST_NOT_JOINED: &str = "HOST_NOT_JOINED";
const CODE_PASSWORD_REQUIRED: &str = "PASSWORD_REQUIRED";
const CODE_PASSWORD_INVALID: &str = "PASSWORD_INVALID";
const CODE_GUEST_NOT_ALLOWED: &str = "GUEST_NOT_ALLOWED";
const CODE_MEETING_ENDED: &str = "MEETING_ENDED";
const CODE_MEETING_CANCELLED: &str = "MEETING_CANCELLED";
const CODE_MEETING_NOT_CANCELLABLE: &str = "MEETING_NOT_CANCELLABLE";
const CODE_INVALID_MEETING_STATUS: &str = "INVALID_MEETING_STATUS";

#[derive(Debug, Clone, sqlx::FromRow)]
struct MeetingRow {
    id: Uuid,
    meeting_no: String,
    room_name: String,
    creator_user_id: Uuid,
    allow_guest_join: bool,
    status: String,
    topic: String,
    scheduled_start_at: DateTime<Utc>,
    opened_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    requires_password: bool,
    password_hash: Option<String>,
    no_join_auto_end_minutes: i32,
    empty_auto_end_minutes: i32,
    first_participant_joined_at: Option<DateTime<Utc>>,
    empty_since: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, sqlx::FromRow)]
struct HostMeetingRow {
    meeting_no: String,
    room_name: String,
    topic: String,
    status: String,
    scheduled_start_at: DateTime<Utc>,
    opened_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    allow_guest_join: bool,
    requires_password: bool,
    no_join_auto_end_minutes: i32,
    empty_auto_end_minutes: i32,
    created_at: DateTime<Utc>,
}

/// Service for business meetings and per-user meeting records.
pub struct MeetingRegistryService;

impl MeetingRegistryService {
    /// Create a meeting with a unique 9-digit meeting number.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_meeting(
        db: &PgPool,
        config: &Config,
        creator_user_id: Uuid,
        display_name: Option<String>,
        allow_guest_join: bool,
        topic: Option<String>,
        scheduled_start_at: Option<DateTime<Utc>>,
        password: Option<String>,
        no_join_auto_end_minutes: Option<u32>,
        empty_auto_end_minutes: Option<u32>,
    ) -> Result<CreateMeetingResponse, AppError> {
        let display_name = display_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let topic = Self::normalize_topic(topic)?;

        let no_join_auto_end_minutes = no_join_auto_end_minutes.unwrap_or(15) as i32;
        let empty_auto_end_minutes = empty_auto_end_minutes.unwrap_or(10) as i32;
        if no_join_auto_end_minutes <= 0 || empty_auto_end_minutes <= 0 {
            return Err(AppError::bad_request(
                "auto end minutes must be greater than 0",
            ));
        }

        let password = password
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let (requires_password, password_hash) = match password {
            Some(raw) => {
                Self::validate_meeting_password(raw)?;
                (true, Some(Self::hash_meeting_password(raw)?))
            }
            None => (false, None),
        };

        for _ in 0..MEETING_NUMBER_RETRY {
            let meeting_no = {
                let mut rng = rand::thread_rng();
                format!("{:09}", rng.gen_range(0..MEETING_NUMBER_MAX))
            };
            let room_name = format!("m-{}", meeting_no);

            let inserted = sqlx::query_as::<_, MeetingRow>(
                r#"
                INSERT INTO meetings (
                    meeting_no,
                    room_name,
                    creator_user_id,
                    allow_guest_join,
                    status,
                    topic,
                    scheduled_start_at,
                    requires_password,
                    password_hash,
                    no_join_auto_end_minutes,
                    empty_auto_end_minutes
                )
                VALUES ($1, $2, $3, $4, 'scheduled', $5, COALESCE($6, NOW()), $7, $8, $9, $10)
                RETURNING
                    id, meeting_no, room_name, creator_user_id, allow_guest_join, status, topic,
                    scheduled_start_at, opened_at, ended_at, cancelled_at, requires_password,
                    password_hash, no_join_auto_end_minutes, empty_auto_end_minutes,
                    first_participant_joined_at, empty_since, created_at
                "#,
            )
            .bind(&meeting_no)
            .bind(&room_name)
            .bind(creator_user_id)
            .bind(allow_guest_join)
            .bind(&topic)
            .bind(scheduled_start_at)
            .bind(requires_password)
            .bind(password_hash.as_deref())
            .bind(no_join_auto_end_minutes)
            .bind(empty_auto_end_minutes)
            .fetch_one(db)
            .await;

            match inserted {
                Ok(row) => {
                    return Ok(CreateMeetingResponse {
                        meeting_no: row.meeting_no.clone(),
                        room_name: row.room_name,
                        share_url: Self::build_share_url(config, &row.meeting_no),
                        status: row.status,
                        topic: row.topic,
                        scheduled_start_at: row.scheduled_start_at,
                        opened_at: row.opened_at,
                        ended_at: row.ended_at,
                        allow_guest_join: row.allow_guest_join,
                        requires_password: row.requires_password,
                        no_join_auto_end_minutes: row.no_join_auto_end_minutes,
                        empty_auto_end_minutes: row.empty_auto_end_minutes,
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

        let mut meeting = Self::find_meeting_by_no(db, meeting_no).await?;
        let now = Self::current_db_time(db).await?;

        if now < meeting.scheduled_start_at {
            return Err(AppError::conflict_code(
                "Meeting has not reached scheduled start time",
                CODE_MEETING_NOT_STARTED,
            ));
        }

        let is_host = meeting.creator_user_id == user_id;
        match meeting.status.as_str() {
            "ended" => {
                return Err(AppError::conflict_code(
                    "Meeting has ended",
                    CODE_MEETING_ENDED,
                ));
            }
            "cancelled" => {
                return Err(AppError::conflict_code(
                    "Meeting has been cancelled",
                    CODE_MEETING_CANCELLED,
                ));
            }
            "scheduled" => {
                if !is_host {
                    return Err(AppError::conflict_code(
                        "Meeting host has not opened the meeting yet",
                        CODE_HOST_NOT_JOINED,
                    ));
                }

                let opened = Self::try_open_meeting_by_host(db, meeting_no, user_id).await?;
                if opened {
                    meeting = Self::find_meeting_by_no(db, meeting_no).await?;
                } else {
                    let refreshed = Self::find_meeting_by_no(db, meeting_no).await?;
                    meeting = refreshed;
                    if meeting.status == "scheduled" {
                        return Err(AppError::conflict_code(
                            "Meeting host has not opened the meeting yet",
                            CODE_HOST_NOT_JOINED,
                        ));
                    }
                }
            }
            "open" => {}
            _ => return Err(AppError::internal("Unsupported meeting status")),
        }

        if meeting.status != "open" {
            return Err(AppError::conflict_code(
                "Meeting host has not opened the meeting yet",
                CODE_HOST_NOT_JOINED,
            ));
        }

        if meeting.requires_password && !is_host {
            let input_password = req
                .meeting_password
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::forbidden_code("Meeting password is required", CODE_PASSWORD_REQUIRED)
                })?;
            let hash = meeting
                .password_hash
                .as_deref()
                .ok_or_else(|| AppError::internal("Meeting password hash is missing"))?;
            if !Self::verify_meeting_password(input_password, hash)? {
                return Err(AppError::forbidden_code(
                    "Meeting password is invalid",
                    CODE_PASSWORD_INVALID,
                ));
            }
        }

        let mut participant_name = req.participant_name.trim().to_string();
        if participant_name.is_empty() {
            participant_name = Self::get_user_display_name(db, user_id)
                .await?
                .unwrap_or_else(|| user_email.to_string());
        }

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

        Self::mark_successful_join(db, meeting.id).await?;

        Ok(JoinMeetingResponse {
            meeting_no: meeting.meeting_no,
            token: token.token,
            url: token.url,
            room_name: token.room_name,
            is_host: token.is_host,
        })
    }

    /// Guest joins a meeting by meeting number.
    ///
    /// Guest access is controlled by `meetings.allow_guest_join`.
    pub async fn guest_join_meeting(
        db: &PgPool,
        config: &Config,
        meeting_no: &str,
        req: GuestJoinMeetingRequest,
    ) -> Result<JoinMeetingResponse, AppError> {
        if !Self::is_valid_meeting_no(meeting_no) {
            return Err(AppError::bad_request("meeting_no must be 9 digits"));
        }

        let meeting = Self::find_meeting_by_no(db, meeting_no).await?;
        let now = Self::current_db_time(db).await?;

        if now < meeting.scheduled_start_at {
            return Err(AppError::conflict_code(
                "Meeting has not reached scheduled start time",
                CODE_MEETING_NOT_STARTED,
            ));
        }

        match meeting.status.as_str() {
            "scheduled" => {
                return Err(AppError::conflict_code(
                    "Meeting host has not opened the meeting yet",
                    CODE_HOST_NOT_JOINED,
                ));
            }
            "ended" => {
                return Err(AppError::conflict_code(
                    "Meeting has ended",
                    CODE_MEETING_ENDED,
                ));
            }
            "cancelled" => {
                return Err(AppError::conflict_code(
                    "Meeting has been cancelled",
                    CODE_MEETING_CANCELLED,
                ));
            }
            "open" => {}
            _ => return Err(AppError::internal("Unsupported meeting status")),
        }

        if !meeting.allow_guest_join {
            return Err(AppError::forbidden_code(
                "Guest join is not allowed for this meeting",
                CODE_GUEST_NOT_ALLOWED,
            ));
        }

        if meeting.requires_password {
            let input_password = req
                .meeting_password
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::forbidden_code("Meeting password is required", CODE_PASSWORD_REQUIRED)
                })?;
            let hash = meeting
                .password_hash
                .as_deref()
                .ok_or_else(|| AppError::internal("Meeting password hash is missing"))?;
            if !Self::verify_meeting_password(input_password, hash)? {
                return Err(AppError::forbidden_code(
                    "Meeting password is invalid",
                    CODE_PASSWORD_INVALID,
                ));
            }
        }

        let mut participant_name = req.participant_name.trim().to_string();
        if participant_name.is_empty() {
            participant_name = format!("GUEST-{}", Uuid::new_v4().simple());
        }

        let identity = format!("GUEST-{}", Uuid::new_v4().simple());
        let token = AuthService::generate_guest_token_for_room_with_identity(
            config,
            meeting.room_name.clone(),
            identity,
            Some(participant_name),
        )?;

        Self::mark_successful_join(db, meeting.id).await?;

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

        let meeting = Self::find_meeting_by_no(db, meeting_no).await?;
        let identity = user_id.to_string();
        let is_host = meeting.creator_user_id == user_id;

        if meeting.status != "open" {
            return Ok(LeaveMeetingResponse {
                message: "Already left meeting".to_string(),
                meeting_no: meeting.meeting_no,
                room_name: meeting.room_name,
                identity,
                left: false,
            });
        }

        let participants_before = match livekit.list_participants(&meeting.room_name).await {
            Ok(resp) => Some(resp.participants),
            Err(err) => {
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
        let participants_before_count = participants_before.as_ref().map(Vec::len);

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
                    if Self::is_room_unavailable_error(&err) {
                        Self::mark_meeting_ended_by_id(db, meeting.id).await?;
                    }
                    None
                }
            }
        } else {
            participants_before.as_ref().map(Vec::is_empty)
        };

        let should_end_immediately = is_host
            || (already_present
                && participants_before_count == Some(1)
                && room_is_empty == Some(true));

        if should_end_immediately {
            Self::try_cleanup_room_as_ended(livekit, &meeting.room_name).await;
            Self::mark_meeting_ended_by_id(db, meeting.id).await?;
        } else {
            match room_is_empty {
                Some(true) => Self::set_empty_since_if_absent(db, meeting.id).await?,
                Some(false) => Self::clear_empty_since(db, meeting.id).await?,
                None => {}
            }
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

    /// Cancel a scheduled meeting as host.
    pub async fn cancel_meeting(
        db: &PgPool,
        meeting_no: &str,
        user_id: Uuid,
    ) -> Result<MessageResponse, AppError> {
        if !Self::is_valid_meeting_no(meeting_no) {
            return Err(AppError::bad_request("meeting_no must be 9 digits"));
        }

        let meeting = Self::find_meeting_by_no(db, meeting_no).await?;
        if meeting.creator_user_id != user_id {
            return Err(AppError::forbidden(
                "Only meeting host can perform this action",
            ));
        }
        if meeting.status != "scheduled" {
            return Err(AppError::conflict_code(
                "Meeting cannot be cancelled in current status",
                CODE_MEETING_NOT_CANCELLABLE,
            ));
        }

        sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'cancelled',
                cancelled_at = COALESCE(cancelled_at, NOW())
            WHERE id = $1
              AND status = 'scheduled'
            "#,
        )
        .bind(meeting.id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to cancel meeting: {}", e)))?;

        Ok(MessageResponse {
            message: "Meeting cancelled".to_string(),
            identity: None,
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
    pub async fn mark_meeting_ended_by_id(db: &PgPool, meeting_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'ended',
                ended_at = COALESCE(ended_at, NOW())
            WHERE id = $1
              AND status IN ('scheduled', 'open')
            "#,
        )
        .bind(meeting_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to mark meeting ended: {}", e)))?;

        Ok(())
    }

    /// Mark a meeting as ended by room name.
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
              AND status IN ('scheduled', 'open')
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

    /// List meetings created by current user.
    pub async fn list_host_meetings(
        db: &PgPool,
        user_id: Uuid,
        query: HostMeetingsQuery,
    ) -> Result<HostMeetingListResponse, AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * page_size) as i64;

        let status_filter = query
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        if let Some(ref status) = status_filter {
            if !Self::is_supported_status(status) {
                return Err(AppError::bad_request_code(
                    "status must be one of scheduled/open/ended/cancelled",
                    CODE_INVALID_MEETING_STATUS,
                ));
            }
        }

        let include_ended = query.include_ended.unwrap_or(false);
        let skip_active_status_filter = status_filter.is_some() || include_ended;

        let rows = sqlx::query_as::<_, HostMeetingRow>(
            r#"
            SELECT
                meeting_no,
                room_name,
                topic,
                status,
                scheduled_start_at,
                opened_at,
                ended_at,
                cancelled_at,
                allow_guest_join,
                requires_password,
                no_join_auto_end_minutes,
                empty_auto_end_minutes,
                created_at
            FROM meetings
            WHERE creator_user_id = $1
              AND ($2 OR status IN ('scheduled', 'open'))
              AND ($3::text IS NULL OR status = $3)
              AND ($4::timestamptz IS NULL OR scheduled_start_at >= $4)
              AND ($5::timestamptz IS NULL OR scheduled_start_at <= $5)
            ORDER BY scheduled_start_at DESC, created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(user_id)
        .bind(skip_active_status_filter)
        .bind(status_filter.as_deref())
        .bind(query.time_from)
        .bind(query.time_to)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list host meetings: {}", e)))?;

        let meetings = rows
            .into_iter()
            .map(|row| HostMeetingItem {
                meeting_no: row.meeting_no,
                room_name: row.room_name,
                topic: row.topic,
                status: row.status,
                scheduled_start_at: row.scheduled_start_at,
                opened_at: row.opened_at,
                ended_at: row.ended_at,
                cancelled_at: row.cancelled_at,
                allow_guest_join: row.allow_guest_join,
                requires_password: row.requires_password,
                no_join_auto_end_minutes: row.no_join_auto_end_minutes,
                empty_auto_end_minutes: row.empty_auto_end_minutes,
                created_at: row.created_at,
            })
            .collect();

        Ok(HostMeetingListResponse {
            meetings,
            page,
            page_size,
        })
    }

    pub(crate) async fn set_empty_since_if_absent(
        db: &PgPool,
        meeting_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET empty_since = COALESCE(empty_since, NOW())
            WHERE id = $1
              AND status = 'open'
            "#,
        )
        .bind(meeting_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to set empty_since: {}", e)))?;
        Ok(())
    }

    pub(crate) async fn clear_empty_since(db: &PgPool, meeting_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET empty_since = NULL
            WHERE id = $1
              AND status = 'open'
            "#,
        )
        .bind(meeting_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to clear empty_since: {}", e)))?;
        Ok(())
    }

    pub(crate) async fn mark_successful_join(
        db: &PgPool,
        meeting_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET
                first_participant_joined_at = COALESCE(first_participant_joined_at, NOW()),
                empty_since = NULL
            WHERE id = $1
            "#,
        )
        .bind(meeting_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to mark meeting join state: {}", e)))?;
        Ok(())
    }

    fn build_share_url(config: &Config, meeting_no: &str) -> String {
        let base = config.app_base_url.trim_end_matches('/');
        format!("{}/join?meetingNo={}", base, meeting_no)
    }

    async fn find_meeting_by_no(db: &PgPool, meeting_no: &str) -> Result<MeetingRow, AppError> {
        let meeting = sqlx::query_as::<_, MeetingRow>(
            r#"
            SELECT
                id,
                meeting_no,
                room_name,
                creator_user_id,
                allow_guest_join,
                status,
                topic,
                scheduled_start_at,
                opened_at,
                ended_at,
                cancelled_at,
                requires_password,
                password_hash,
                no_join_auto_end_minutes,
                empty_auto_end_minutes,
                first_participant_joined_at,
                empty_since,
                created_at
            FROM meetings
            WHERE meeting_no = $1
            "#,
        )
        .bind(meeting_no)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query meeting: {}", e)))?
        .ok_or_else(|| AppError::not_found("Meeting not found"))?;

        Ok(meeting)
    }

    async fn current_db_time(db: &PgPool) -> Result<DateTime<Utc>, AppError> {
        sqlx::query_scalar::<_, DateTime<Utc>>("SELECT NOW()")
            .fetch_one(db)
            .await
            .map_err(|e| AppError::internal(format!("Failed to query database time: {}", e)))
    }

    async fn try_cleanup_room_as_ended<L: LiveKitService + ?Sized>(livekit: &L, room_name: &str) {
        if let Err(err) = MeetingService::end_meeting(livekit, room_name).await {
            let err_text = err.to_string();
            if !Self::is_room_unavailable_error(&err_text) {
                warn!(
                    "Failed to cleanup room '{}' while ending meeting: {}",
                    room_name, err_text
                );
            }
        }
    }

    async fn try_open_meeting_by_host(
        db: &PgPool,
        meeting_no: &str,
        user_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'open',
                opened_at = COALESCE(opened_at, NOW())
            WHERE meeting_no = $1
              AND status = 'scheduled'
              AND creator_user_id = $2
              AND NOW() >= scheduled_start_at
            "#,
        )
        .bind(meeting_no)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to open meeting: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_user_display_name(db: &PgPool, user_id: Uuid) -> Result<Option<String>, AppError> {
        let display_name = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT display_name
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query user display name: {}", e)))?
        .flatten();

        Ok(display_name)
    }

    fn normalize_topic(topic: Option<String>) -> Result<String, AppError> {
        let topic = topic.unwrap_or_default();
        let topic = topic.trim().to_string();
        if topic.len() > 200 {
            return Err(AppError::bad_request(
                "topic must be at most 200 characters",
            ));
        }
        Ok(topic)
    }

    fn validate_meeting_password(password: &str) -> Result<(), AppError> {
        if !(PASSWORD_MIN_LEN..=PASSWORD_MAX_LEN).contains(&password.len()) {
            return Err(AppError::bad_request(format!(
                "password length must be {}-{} characters",
                PASSWORD_MIN_LEN, PASSWORD_MAX_LEN
            )));
        }
        Ok(())
    }

    fn hash_meeting_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::internal(format!("Failed to hash meeting password: {}", e)))?;
        Ok(hash.to_string())
    }

    fn verify_meeting_password(password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed = PasswordHash::new(hash)
            .map_err(|e| AppError::internal(format!("Invalid meeting password hash: {}", e)))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    fn is_supported_status(status: &str) -> bool {
        matches!(status, "scheduled" | "open" | "ended" | "cancelled")
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
