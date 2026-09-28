-- Accounts are identified by a username instead of an email address, and
-- there is no email verification any more.
--
-- Existing rows keep their email as the username, so those users can still
-- sign in with it.
ALTER TABLE users RENAME COLUMN email TO username;

-- Usernames keep the case they were created with but are unique regardless
-- of case, which replaces the plain unique constraint on the old column.
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_email_key;
DROP INDEX IF EXISTS idx_users_email;
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username_lower ON users (LOWER(username));

DROP TABLE IF EXISTS email_verification_codes;
