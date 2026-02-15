-- Add optional user display name for profile and meeting defaults
ALTER TABLE users
ADD COLUMN IF NOT EXISTS display_name VARCHAR(64);
