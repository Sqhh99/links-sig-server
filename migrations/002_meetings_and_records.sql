-- Meetings table
-- Stores business-level meetings mapped to LiveKit room names
CREATE TABLE IF NOT EXISTS meetings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    meeting_no CHAR(9) NOT NULL UNIQUE,
    room_name VARCHAR(64) NOT NULL UNIQUE,
    creator_user_id UUID NOT NULL REFERENCES users(id),
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_meeting_no_format CHECK (meeting_no ~ '^[0-9]{9}$'),
    CONSTRAINT chk_meeting_status CHECK (status IN ('active', 'ended'))
);

CREATE INDEX IF NOT EXISTS idx_meetings_creator_user_id
    ON meetings(creator_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_meetings_status ON meetings(status);

-- Meeting participants table
-- One row per (meeting, user), with join counters
CREATE TABLE IF NOT EXISTS meeting_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    meeting_id UUID NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    first_joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    join_count INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_meeting_user UNIQUE (meeting_id, user_id),
    CONSTRAINT chk_join_count_positive CHECK (join_count > 0)
);

CREATE INDEX IF NOT EXISTS idx_meeting_participants_user_id
    ON meeting_participants(user_id, last_joined_at DESC);
CREATE INDEX IF NOT EXISTS idx_meeting_participants_meeting_id
    ON meeting_participants(meeting_id);

-- Keep updated_at in sync
DROP TRIGGER IF EXISTS update_meetings_updated_at ON meetings;
CREATE TRIGGER update_meetings_updated_at
    BEFORE UPDATE ON meetings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_meeting_participants_updated_at ON meeting_participants;
CREATE TRIGGER update_meeting_participants_updated_at
    BEFORE UPDATE ON meeting_participants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
