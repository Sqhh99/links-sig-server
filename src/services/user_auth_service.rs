//! User authentication service - Registration, login, and verification
//!
//! Handles user account management with email verification.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::encode_user_token;
use crate::config::Config;
use crate::integrations::email::{format_verification_email, EmailSender};
use crate::types::{
    AppError, LoginRequest, LoginResponse, RefreshTokenResponse, RegisterRequest, RegisterResponse,
    RequestCodeResponse, RequestRegisterCodeRequest,
};

type HmacSha256 = Hmac<Sha256>;

/// User record from database
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Verification code record
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct VerificationCode {
    pub id: Uuid,
    pub email: String,
    pub code_hash: String,
    pub purpose: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// User authentication service
pub struct UserAuthService;

impl UserAuthService {
    // ========================================================================
    // Request Verification Code
    // ========================================================================

    /// Request a verification code for registration
    ///
    /// Steps:
    /// 1. Validate email format
    /// 2. Check if email is already registered
    /// 3. Check rate limit (60s between requests)
    /// 4. Generate and hash verification code
    /// 5. Store in database
    /// 6. Send email
    pub async fn request_register_code<E: EmailSender + ?Sized>(
        db: &PgPool,
        email_sender: &E,
        config: &Config,
        req: RequestRegisterCodeRequest,
    ) -> Result<RequestCodeResponse, AppError> {
        let email = req.email.trim().to_lowercase();

        // Validate email format
        if !Self::is_valid_email(&email) {
            return Err(AppError::bad_request("Invalid email format"));
        }

        // Check if email is already registered
        if Self::email_exists(db, &email).await? {
            return Err(AppError::conflict("Email is already registered"));
        }

        // Check rate limit
        if let Some(last_code) = Self::get_last_verification_code(db, &email, "register").await? {
            let elapsed = Utc::now() - last_code.created_at;
            let rate_limit = Duration::seconds(config.code_rate_limit_secs as i64);

            if elapsed < rate_limit {
                let remaining = (rate_limit - elapsed).num_seconds() as u64;
                return Err(AppError::too_many_requests(format!(
                    "Please wait {} seconds before requesting another code",
                    remaining
                )));
            }
        }

        // Generate verification code
        let code = Self::generate_code(config.code_length);
        let code_hash = Self::hash_code(&code, &config.code_hmac_secret);

        // Calculate expiration
        let expires_at = Utc::now() + Duration::seconds(config.code_expiration_secs as i64);

        // Store in database
        sqlx::query(
            r#"
            INSERT INTO email_verification_codes (email, code_hash, purpose, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(&email)
        .bind(&code_hash)
        .bind("register")
        .bind(expires_at)
        .execute(db)
        .await?;

        // Send email
        let subject = "Your verification code";
        let body = format_verification_email(&code, config.code_expiration_secs / 60);

        email_sender
            .send_email(&email, subject, &body)
            .await
            .map_err(|e| {
                warn!("Failed to send verification email to {}: {}", email, e);
                AppError::internal("Failed to send verification email")
            })?;

        info!("Verification code sent to {}", email);

        Ok(RequestCodeResponse {
            message: "Verification code sent to your email".to_string(),
            retry_after_secs: config.code_rate_limit_secs,
        })
    }

    // ========================================================================
    // Register
    // ========================================================================

    /// Complete registration with verification code
    ///
    /// Steps:
    /// 1. Validate inputs
    /// 2. Verify the code
    /// 3. Check email not already registered
    /// 4. Hash password
    /// 5. Create user
    /// 6. Mark code as used
    /// 7. Generate JWT
    pub async fn register(
        db: &PgPool,
        config: &Config,
        req: RegisterRequest,
    ) -> Result<RegisterResponse, AppError> {
        let email = req.email.trim().to_lowercase();
        let code = req.code.trim();
        let password = &req.password;
        let display_name = Self::normalize_display_name(req.display_name)?;

        // Validate inputs
        if !Self::is_valid_email(&email) {
            return Err(AppError::bad_request("Invalid email format"));
        }

        if password.len() < 8 {
            return Err(AppError::bad_request(
                "Password must be at least 8 characters",
            ));
        }

        // Verify code
        let verification =
            Self::verify_code(db, &email, code, "register", &config.code_hmac_secret)
                .await?
                .ok_or_else(|| AppError::bad_request("Invalid or expired verification code"))?;

        // Double-check email is not registered (race condition protection)
        if Self::email_exists(db, &email).await? {
            return Err(AppError::conflict("Email is already registered"));
        }

        // Hash password
        let password_hash = Self::hash_password(password)?;

        // Create user
        let user_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO users (email, password_hash, display_name)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(&email)
        .bind(&password_hash)
        .bind(&display_name)
        .fetch_one(db)
        .await?;

        // Mark verification code as used
        sqlx::query(
            r#"
            UPDATE email_verification_codes
            SET used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(verification.id)
        .execute(db)
        .await?;

        // Generate JWT
        let token = encode_user_token(
            user_id,
            &email,
            &config.jwt_secret,
            config.jwt_expiration_secs,
        )?;

        info!("User registered: {} ({})", email, user_id);

        Ok(RegisterResponse {
            user_id,
            email,
            token,
            display_name,
        })
    }

    // ========================================================================
    // Login
    // ========================================================================

    /// Login with email and password
    pub async fn login(
        db: &PgPool,
        config: &Config,
        req: LoginRequest,
    ) -> Result<LoginResponse, AppError> {
        let email = req.email.trim().to_lowercase();
        let password = &req.password;

        // Find user
        let user: Option<User> = sqlx::query_as(
            r#"
            SELECT id, email, display_name, password_hash, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(&email)
        .fetch_optional(db)
        .await?;

        let user = user.ok_or_else(|| AppError::unauthorized("Invalid email or password"))?;

        // Verify password
        if !Self::verify_password(password, &user.password_hash)? {
            return Err(AppError::unauthorized("Invalid email or password"));
        }

        // Generate JWT
        let token = encode_user_token(
            user.id,
            &user.email,
            &config.jwt_secret,
            config.jwt_expiration_secs,
        )?;

        info!("User logged in: {} ({})", user.email, user.id);

        Ok(LoginResponse {
            user_id: user.id,
            email: user.email,
            token,
            display_name: user.display_name,
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
            SELECT email, display_name
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("User not found"))?;
        let (email, display_name) = user_row;

        let token = encode_user_token(
            user_id,
            &email,
            &config.jwt_secret,
            config.jwt_expiration_secs,
        )?;

        Ok(RefreshTokenResponse {
            user_id,
            email,
            token,
            expires_in_secs: config.jwt_expiration_secs,
            display_name,
        })
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Check if email exists in database
    async fn email_exists(db: &PgPool, email: &str) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)
            "#,
        )
        .bind(email)
        .fetch_one(db)
        .await?;

        Ok(exists)
    }

    /// Get the last verification code for an email and purpose
    async fn get_last_verification_code(
        db: &PgPool,
        email: &str,
        purpose: &str,
    ) -> Result<Option<VerificationCode>, AppError> {
        let code: Option<VerificationCode> = sqlx::query_as(
            r#"
            SELECT id, email, code_hash, purpose, expires_at, created_at, used_at
            FROM email_verification_codes
            WHERE email = $1 AND purpose = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(email)
        .bind(purpose)
        .fetch_optional(db)
        .await?;

        Ok(code)
    }

    /// Verify a code against the database
    async fn verify_code(
        db: &PgPool,
        email: &str,
        code: &str,
        purpose: &str,
        hmac_secret: &str,
    ) -> Result<Option<VerificationCode>, AppError> {
        let code_hash = Self::hash_code(code, hmac_secret);

        let verification: Option<VerificationCode> = sqlx::query_as(
            r#"
            SELECT id, email, code_hash, purpose, expires_at, created_at, used_at
            FROM email_verification_codes
            WHERE email = $1 
              AND purpose = $2 
              AND code_hash = $3 
              AND expires_at > NOW()
              AND used_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(email)
        .bind(purpose)
        .bind(&code_hash)
        .fetch_optional(db)
        .await?;

        Ok(verification)
    }

    /// Generate a random numeric verification code
    fn generate_code(length: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| rng.gen_range(0..10).to_string())
            .collect()
    }

    /// Hash a verification code using HMAC-SHA256
    fn hash_code(code: &str, secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(code.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
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

    /// Simple email validation
    fn is_valid_email(email: &str) -> bool {
        // Basic validation: contains @ with non-empty local part, and at least one . after @
        if let Some(at_pos) = email.find('@') {
            // Local part (before @) must not be empty
            if at_pos == 0 {
                return false;
            }
            let domain = &email[at_pos + 1..];
            return domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.');
        }
        false
    }

    fn normalize_display_name(display_name: Option<String>) -> Result<Option<String>, AppError> {
        let Some(display_name) = display_name else {
            return Ok(None);
        };

        let normalized = display_name.trim().to_string();
        if normalized.is_empty() {
            return Ok(None);
        }

        if normalized.chars().count() > 64 {
            return Err(AppError::bad_request(
                "displayName must be at most 64 characters",
            ));
        }

        Ok(Some(normalized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_length() {
        let code = UserAuthService::generate_code(6);
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_hash_code_deterministic() {
        let code = "123456";
        let secret = "test-secret";

        let hash1 = UserAuthService::hash_code(code, secret);
        let hash2 = UserAuthService::hash_code(code, secret);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_code_different_codes() {
        let secret = "test-secret";

        let hash1 = UserAuthService::hash_code("123456", secret);
        let hash2 = UserAuthService::hash_code("654321", secret);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_password_and_verify() {
        let password = "secure_password_123";
        let hash = UserAuthService::hash_password(password).unwrap();

        assert!(UserAuthService::verify_password(password, &hash).unwrap());
        assert!(!UserAuthService::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_is_valid_email() {
        assert!(UserAuthService::is_valid_email("test@example.com"));
        assert!(UserAuthService::is_valid_email("user.name@sub.domain.com"));
        assert!(!UserAuthService::is_valid_email("invalid"));
        assert!(!UserAuthService::is_valid_email("invalid@"));
        assert!(!UserAuthService::is_valid_email("@domain.com"));
        assert!(!UserAuthService::is_valid_email("test@.com"));
        assert!(!UserAuthService::is_valid_email("test@domain."));
    }
}
