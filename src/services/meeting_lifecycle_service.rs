//! Meeting lifecycle background service.
//!
//! Handles automatic meeting transitions:
//! - scheduled -> ended when nobody joins within timeout after scheduled start
//! - open -> ended when room stays empty for timeout

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use crate::integrations::LiveKitService;
use crate::services::{MeetingRegistryService, MeetingService};
use crate::state::AppState;
use crate::types::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
struct OpenMeetingRow {
    id: Uuid,
    room_name: String,
    empty_since: Option<DateTime<Utc>>,
    empty_auto_end_minutes: i32,
}

/// Periodic lifecycle runner for business meetings.
pub struct MeetingLifecycleService;

impl MeetingLifecycleService {
    /// Run lifecycle loop in the background.
    pub async fn run_loop(state: AppState) {
        let interval_secs = state.config.meeting_lifecycle_interval_secs.max(5);
        let mut ticker = interval(Duration::from_secs(interval_secs));

        loop {
            ticker.tick().await;
            if let Err(err) = Self::run_once(
                &state.db,
                &*state.livekit,
                state.config.meeting_lifecycle_lock_key,
            )
            .await
            {
                warn!("Meeting lifecycle tick failed: {}", err);
            }
        }
    }

    /// Run one lifecycle scan pass.
    pub async fn run_once<L: LiveKitService + ?Sized>(
        db: &PgPool,
        livekit: &L,
        advisory_lock_key: i64,
    ) -> Result<(), AppError> {
        // Advisory locks are session-scoped, so lock/unlock must use the same DB connection.
        let mut lock_conn = db
            .acquire()
            .await
            .map_err(|e| AppError::internal(format!("Failed to acquire DB connection: {}", e)))?;

        if !Self::try_acquire_lock(&mut lock_conn, advisory_lock_key).await? {
            return Ok(());
        }

        let result = async {
            Self::auto_end_no_join_scheduled_meetings(db).await?;
            Self::auto_end_or_track_empty_open_meetings(db, livekit).await?;
            Ok(())
        }
        .await;

        if let Err(err) = Self::release_lock(&mut lock_conn, advisory_lock_key).await {
            warn!("Failed to release meeting lifecycle advisory lock: {}", err);
        }

        result
    }

    async fn try_acquire_lock(
        conn: &mut PoolConnection<Postgres>,
        advisory_lock_key: i64,
    ) -> Result<bool, AppError> {
        sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(advisory_lock_key)
            .fetch_one(conn.as_mut())
            .await
            .map_err(|e| AppError::internal(format!("Failed to acquire advisory lock: {}", e)))
    }

    async fn release_lock(
        conn: &mut PoolConnection<Postgres>,
        advisory_lock_key: i64,
    ) -> Result<(), AppError> {
        sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock($1)")
            .bind(advisory_lock_key)
            .fetch_one(conn.as_mut())
            .await
            .map_err(|e| AppError::internal(format!("Failed to release advisory lock: {}", e)))?;
        Ok(())
    }

    async fn auto_end_no_join_scheduled_meetings(db: &PgPool) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'ended',
                ended_at = COALESCE(ended_at, NOW()),
                auto_end_reason = 'no_join_timeout'
            WHERE status = 'scheduled'
              AND first_participant_joined_at IS NULL
              AND NOW() >= scheduled_start_at + make_interval(mins => no_join_auto_end_minutes)
            "#,
        )
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to auto-end scheduled meetings: {}", e)))?;

        if result.rows_affected() > 0 {
            info!(
                "Auto-ended {} scheduled meetings by no-join timeout",
                result.rows_affected()
            );
        }

        Ok(())
    }

    async fn auto_end_or_track_empty_open_meetings<L: LiveKitService + ?Sized>(
        db: &PgPool,
        livekit: &L,
    ) -> Result<(), AppError> {
        let rows = sqlx::query_as::<_, OpenMeetingRow>(
            r#"
            SELECT id, room_name, empty_since, empty_auto_end_minutes
            FROM meetings
            WHERE status = 'open'
            "#,
        )
        .fetch_all(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query open meetings: {}", e)))?;

        let now = Utc::now();
        for row in rows {
            match livekit.list_participants(&row.room_name).await {
                Ok(resp) => {
                    if resp.participants.is_empty() {
                        if let Some(empty_since) = row.empty_since {
                            let elapsed = now.signed_duration_since(empty_since);
                            if elapsed >= ChronoDuration::minutes(row.empty_auto_end_minutes as i64)
                            {
                                // Confirm emptiness once more before forcing end to avoid
                                // transient list inconsistencies causing false positives.
                                if let Ok(confirm) = livekit.list_participants(&row.room_name).await
                                {
                                    if !confirm.participants.is_empty() {
                                        MeetingRegistryService::mark_successful_join(db, row.id)
                                            .await?;
                                        continue;
                                    }
                                }
                                if let Err(err) =
                                    MeetingService::end_meeting(livekit, &row.room_name).await
                                {
                                    warn!(
                                        "Failed to cleanup room '{}' while auto-ending: {}",
                                        row.room_name, err
                                    );
                                }
                                Self::mark_meeting_ended_with_reason(db, row.id, "empty_timeout")
                                    .await?;
                            }
                        } else {
                            MeetingRegistryService::set_empty_since_if_absent(db, row.id).await?;
                        }
                    } else {
                        MeetingRegistryService::mark_successful_join(db, row.id).await?;
                    }
                }
                Err(err) => {
                    warn!(
                        "Lifecycle list participants failed for room '{}': {}",
                        row.room_name, err
                    );
                    if Self::is_room_unavailable_error(&err) {
                        Self::mark_meeting_ended_with_reason(db, row.id, "empty_timeout").await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn mark_meeting_ended_with_reason(
        db: &PgPool,
        meeting_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE meetings
            SET
                status = 'ended',
                ended_at = COALESCE(ended_at, NOW()),
                auto_end_reason = $2,
                empty_since = NULL
            WHERE id = $1
              AND status IN ('scheduled', 'open')
            "#,
        )
        .bind(meeting_id)
        .bind(reason)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(format!("Failed to mark meeting ended: {}", e)))?;
        Ok(())
    }

    fn is_room_unavailable_error(err: &str) -> bool {
        let normalized = err.to_ascii_lowercase();
        normalized.contains("not found")
            || normalized.contains("does not exist")
            || normalized.contains("no such room")
    }
}
