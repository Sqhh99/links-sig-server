-- Add meeting-level guest access policy
ALTER TABLE meetings
ADD COLUMN IF NOT EXISTS allow_guest_join BOOLEAN NOT NULL DEFAULT false;
