-- Scheduled meeting lifecycle and security fields
-- This migration upgrades meeting status model and adds scheduling controls.

-- Ensure allow_guest_join exists (older environments may not have migration 004 applied).
ALTER TABLE meetings
ADD COLUMN IF NOT EXISTS allow_guest_join BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE meetings
ADD COLUMN IF NOT EXISTS topic VARCHAR(200) NOT NULL DEFAULT '',
ADD COLUMN IF NOT EXISTS scheduled_start_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS opened_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS cancelled_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS requires_password BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN IF NOT EXISTS password_hash TEXT,
ADD COLUMN IF NOT EXISTS no_join_auto_end_minutes INT NOT NULL DEFAULT 15,
ADD COLUMN IF NOT EXISTS empty_auto_end_minutes INT NOT NULL DEFAULT 10,
ADD COLUMN IF NOT EXISTS first_participant_joined_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS empty_since TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS auto_end_reason VARCHAR(32);

-- Backfill scheduled_start_at for existing rows.
UPDATE meetings
SET scheduled_start_at = created_at
WHERE scheduled_start_at IS NULL;

ALTER TABLE meetings
ALTER COLUMN scheduled_start_at SET NOT NULL;

ALTER TABLE meetings
DROP CONSTRAINT IF EXISTS chk_meeting_status;

-- Upgrade status values:
-- existing active meetings are treated as already-open meetings.
UPDATE meetings
SET status = 'open'
WHERE status = 'active';

ALTER TABLE meetings
ADD CONSTRAINT chk_meeting_status
CHECK (status IN ('scheduled', 'open', 'ended', 'cancelled'));

CREATE INDEX IF NOT EXISTS idx_meetings_status_scheduled_start_at
    ON meetings(status, scheduled_start_at);

CREATE INDEX IF NOT EXISTS idx_meetings_creator_status_created_at
    ON meetings(creator_user_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_meetings_creator_scheduled_start_at
    ON meetings(creator_user_id, scheduled_start_at DESC);
