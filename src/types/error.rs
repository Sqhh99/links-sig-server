//! Unified error type with HTTP response mapping.
//!
//! All service errors are converted to AppError to ensure consistent responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use super::ErrorResponse;

/// Application error type with stable machine-readable code support.
#[derive(Debug, Clone)]
pub struct AppError {
    status: StatusCode,
    message: String,
    code: Option<&'static str>,
}

impl AppError {
    fn new(status: StatusCode, msg: impl Into<String>) -> Self {
        Self {
            status,
            message: msg.into(),
            code: None,
        }
    }

    fn new_with_code(status: StatusCode, msg: impl Into<String>, code: &'static str) -> Self {
        Self {
            status,
            message: msg.into(),
            code: Some(code),
        }
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }

    /// Create a bad request error.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }

    /// Create a bad request error with a stable code.
    pub fn bad_request_code(msg: impl Into<String>, code: &'static str) -> Self {
        Self::new_with_code(StatusCode::BAD_REQUEST, msg, code)
    }

    /// Create a not found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, msg)
    }

    /// Create a not found error with a stable code.
    pub fn not_found_code(msg: impl Into<String>, code: &'static str) -> Self {
        Self::new_with_code(StatusCode::NOT_FOUND, msg, code)
    }

    /// Create an unauthorized error.
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, msg)
    }

    /// Create an unauthorized error with a stable code.
    pub fn unauthorized_code(msg: impl Into<String>, code: &'static str) -> Self {
        Self::new_with_code(StatusCode::UNAUTHORIZED, msg, code)
    }

    /// Create a forbidden error.
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, msg)
    }

    /// Create a forbidden error with a stable code.
    pub fn forbidden_code(msg: impl Into<String>, code: &'static str) -> Self {
        Self::new_with_code(StatusCode::FORBIDDEN, msg, code)
    }

    /// Create a conflict error.
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, msg)
    }

    /// Create a conflict error with a stable code.
    pub fn conflict_code(msg: impl Into<String>, code: &'static str) -> Self {
        Self::new_with_code(StatusCode::CONFLICT, msg, code)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.code {
            Some(code) => write!(f, "{} ({}): {}", self.status, code, self.message),
            None => write!(f, "{}: {}", self.status, self.message),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
                code: self.code.map(str::to_string),
            }),
        )
            .into_response()
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::internal(format!("JWT error: {}", err))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::not_found("Resource not found"),
            sqlx::Error::Database(db_err) => {
                if let Some(code) = db_err.code() {
                    if code == "23505" {
                        return AppError::conflict("Resource already exists");
                    }
                }
                AppError::internal(format!("Database error: {}", db_err))
            }
            _ => AppError::internal(format!("Database error: {}", err)),
        }
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError::internal(format!("Password hashing error: {}", err))
    }
}
