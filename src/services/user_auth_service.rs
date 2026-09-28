//! User authentication service - Username/password sign-in
//!
//! There is no separate registration step: the first login with an unused
//! username creates the account, after the username and password pass the
//! rules below. Later logins with that username must present the same password.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::auth::encode_user_token;
use crate::config::Config;
use crate::types::{AppError, LoginRequest, LoginResponse, RefreshTokenResponse};

pub const CODE_INVALID_CREDENTIALS: &str = "INVALID_CREDENTIALS";
pub const CODE_INVALID_USERNAME: &str = "INVALID_USERNAME";
pub const CODE_WEAK_PASSWORD: &str = "WEAK_PASSWORD";

const USERNAME_MIN_CHARS: usize = 2;
const USERNAME_MAX_CHARS: usize = 32;
const PASSWORD_MIN_CHARS: usize = 8;
const PASSWORD_MAX_CHARS: usize = 128;

/// User record from database
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User authentication service
pub struct UserAuthService;

impl UserAuthService {
    // ========================================================================
    // Login
    // ========================================================================

    /// Sign in with username and password, creating the account on first use.
    ///
    /// Usernames are matched case-insensitively and keep the case they were
    /// created with. Only new usernames are checked against the naming rules,
    /// so accounts created before those rules existed can still sign in.
    pub async fn login(
        db: &PgPool,
        config: &Config,
        req: LoginRequest,
    ) -> Result<LoginResponse, AppError> {
        let username = req.username.trim();
        let password = req.password.as_str();

        if username.is_empty() || password.is_empty() {
            return Err(AppError::bad_request("Username and password are required"));
        }

        if let Some(user) = Self::find_user(db, username).await? {
            return Self::sign_in(config, user, password);
        }

        Self::validate_new_username(username)?;
        Self::validate_new_password(username, password)?;
        let password_hash = Self::hash_password(password)?;

        let created: Option<User> = sqlx::query_as(
            r#"
            INSERT INTO users (username, password_hash)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            RETURNING id, username, display_name, password_hash, created_at, updated_at
            "#,
        )
        .bind(username)
        .bind(&password_hash)
        .fetch_optional(db)
        .await?;

        let Some(user) = created else {
            // Another request created this username between the lookup and the
            // insert; treat this one as a normal login against that account.
            let user = Self::find_user(db, username)
                .await?
                .ok_or_else(|| AppError::internal("Failed to create user"))?;
            return Self::sign_in(config, user, password);
        };

        info!(
            "User created on first login: {} ({})",
            user.username, user.id
        );

        let token = encode_user_token(
            user.id,
            &user.username,
            &config.jwt_secret,
            config.jwt_expiration_secs,
        )?;

        Ok(LoginResponse {
            user_id: user.id,
            username: user.username,
            token,
            display_name: user.display_name,
            account_created: true,
        })
    }

    /// Refresh user JWT using current authenticated user identity.
    pub async fn refresh_token(
        db: &PgPool,
        config: &Config,
        user_id_str: &str,
    ) -> Result<RefreshTokenResponse, AppError> {
        let user_id = Uuid::parse_str(user_id_str)
            .map_err(|_| AppError::unauthorized("Invalid user token"))?;

        let user_row = sqlx::query_as::<_, (String, Option<String>)>(
            r#"
            SELECT username, display_name
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("User not found"))?;
        let (username, display_name) = user_row;

        let token = encode_user_token(
            user_id,
            &username,
            &config.jwt_secret,
            config.jwt_expiration_secs,
        )?;

        Ok(RefreshTokenResponse {
            user_id,
            username,
            token,
            expires_in_secs: config.jwt_expiration_secs,
            display_name,
        })
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Look up a user by username, ignoring case
    async fn find_user(db: &PgPool, username: &str) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            r#"
            SELECT id, username, display_name, password_hash, created_at, updated_at
            FROM users
            WHERE LOWER(username) = LOWER($1)
            "#,
        )
        .bind(username)
        .fetch_optional(db)
        .await?;

        Ok(user)
    }

    /// Verify the password of an existing account and issue a JWT
    fn sign_in(config: &Config, user: User, password: &str) -> Result<LoginResponse, AppError> {
        if !Self::verify_password(password, &user.password_hash)? {
            // The username exists, so this is either a wrong password or
            // someone picking a name that is already taken.
            return Err(AppError::unauthorized_code(
                "Username is taken or password is incorrect",
                CODE_INVALID_CREDENTIALS,
            ));
        }

        let token = encode_user_token(
            user.id,
            &user.username,
            &config.jwt_secret,
            config.jwt_expiration_secs,
        )?;

        info!("User logged in: {} ({})", user.username, user.id);

        Ok(LoginResponse {
            user_id: user.id,
            username: user.username,
            token,
            display_name: user.display_name,
            account_created: false,
        })
    }

    /// Rules for a username that is about to be created: 2-32 characters of
    /// letters (any script), digits, `_`, `-` or `.`.
    fn validate_new_username(username: &str) -> Result<(), AppError> {
        let length = username.chars().count();
        if length < USERNAME_MIN_CHARS || length > USERNAME_MAX_CHARS {
            return Err(AppError::bad_request_code(
                format!(
                    "Username must be {}-{} characters",
                    USERNAME_MIN_CHARS, USERNAME_MAX_CHARS
                ),
                CODE_INVALID_USERNAME,
            ));
        }

        let allowed = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.');
        if !username.chars().all(allowed) {
            return Err(AppError::bad_request_code(
                "Username may only contain letters, digits, '_', '-' and '.'",
                CODE_INVALID_USERNAME,
            ));
        }

        Ok(())
    }

    /// Rules for the password of a new account: 8-128 characters, at least one
    /// ASCII letter and one digit, and not the username itself.
    fn validate_new_password(username: &str, password: &str) -> Result<(), AppError> {
        let length = password.chars().count();
        if length < PASSWORD_MIN_CHARS || length > PASSWORD_MAX_CHARS {
            return Err(AppError::bad_request_code(
                format!(
                    "Password must be {}-{} characters",
                    PASSWORD_MIN_CHARS, PASSWORD_MAX_CHARS
                ),
                CODE_WEAK_PASSWORD,
            ));
        }

        let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        if !has_letter || !has_digit {
            return Err(AppError::bad_request_code(
                "Password must contain both letters and digits",
                CODE_WEAK_PASSWORD,
            ));
        }

        if password.to_lowercase() == username.to_lowercase() {
            return Err(AppError::bad_request_code(
                "Password must not be the same as the username",
                CODE_WEAK_PASSWORD,
            ));
        }

        Ok(())
    }

    /// Hash a password using Argon2
    fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::internal(format!("Failed to hash password: {}", e)))?;
        Ok(password_hash.to_string())
    }

    /// Verify a password against a hash
    fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AppError::internal(format!("Invalid password hash: {}", e)))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_and_verify() {
        let password = "secure_password_123";
        let hash = UserAuthService::hash_password(password).unwrap();

        assert!(UserAuthService::verify_password(password, &hash).unwrap());
        assert!(!UserAuthService::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_validate_new_username() {
        assert!(UserAuthService::validate_new_username("alice").is_ok());
        assert!(UserAuthService::validate_new_username("Bob_the-builder.2").is_ok());
        assert!(UserAuthService::validate_new_username("张三").is_ok());
        assert!(UserAuthService::validate_new_username("a").is_err());
        assert!(UserAuthService::validate_new_username(&"a".repeat(33)).is_err());
        assert!(UserAuthService::validate_new_username("has space").is_err());
        assert!(UserAuthService::validate_new_username("user@example.com").is_err());
    }

    #[test]
    fn test_validate_new_password() {
        assert!(UserAuthService::validate_new_password("alice", "abcdefg1").is_ok());
        assert!(UserAuthService::validate_new_password("alice", "short1").is_err());
        assert!(UserAuthService::validate_new_password("alice", "lettersonly").is_err());
        assert!(UserAuthService::validate_new_password("alice", "12345678").is_err());
        assert!(UserAuthService::validate_new_password("alice", "密码密码1234").is_err());
        assert!(UserAuthService::validate_new_password("alice", &"a1".repeat(65)).is_err());
        assert!(UserAuthService::validate_new_password("alice123", "ALICE123").is_err());
    }
}
