//! Email integration - SMTP email sending with trait abstraction
//!
//! Provides an `EmailSender` trait for abstraction and two implementations:
//! - `SmtpEmailSender`: Real SMTP email sending via lettre
//! - `FakeEmailSender`: In-memory mock for testing

use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use std::sync::{Arc, Mutex};

use crate::config::Config;

/// Email sending abstraction
#[async_trait]
pub trait EmailSender: Send + Sync {
    /// Send an email
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError>;
}

/// Email sending error
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Failed to build email: {0}")]
    BuildError(String),
    #[error("Failed to send email: {0}")]
    SendError(String),
    #[error("Invalid email address: {0}")]
    InvalidAddress(String),
}

/// Real SMTP email sender
pub struct SmtpEmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    sender: String,
}

impl SmtpEmailSender {
    /// Create a new SMTP email sender from configuration
    pub fn new(config: &Config) -> Result<Self, EmailError> {
        let creds = Credentials::new(config.smtp_sender.clone(), config.smtp_password.clone());

        let transport = if config.smtp_use_ssl {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                .map_err(|e| EmailError::SendError(e.to_string()))?
                .credentials(creds)
                .port(config.smtp_port)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
                .map_err(|e| EmailError::SendError(e.to_string()))?
                .credentials(creds)
                .port(config.smtp_port)
                .build()
        };

        Ok(Self {
            transport,
            sender: config.smtp_sender.clone(),
        })
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        let email = Message::builder()
            .from(
                self.sender
                    .parse()
                    .map_err(|e: lettre::address::AddressError| {
                        EmailError::InvalidAddress(e.to_string())
                    })?,
            )
            .to(to.parse().map_err(|e: lettre::address::AddressError| {
                EmailError::InvalidAddress(e.to_string())
            })?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| EmailError::BuildError(e.to_string()))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| EmailError::SendError(e.to_string()))?;

        Ok(())
    }
}

/// Recorded email for testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecordedEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Fake email sender for testing
///
/// Records all sent emails in memory for verification.
#[allow(dead_code)]
pub struct FakeEmailSender {
    emails: Arc<Mutex<Vec<RecordedEmail>>>,
    should_fail: Arc<Mutex<bool>>,
}

#[allow(dead_code)]
impl FakeEmailSender {
    /// Create a new fake email sender
    pub fn new() -> Self {
        Self {
            emails: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Configure the sender to fail on next send
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }

    /// Get all recorded emails
    pub fn get_emails(&self) -> Vec<RecordedEmail> {
        self.emails.lock().unwrap().clone()
    }

    /// Get the last recorded email
    pub fn last_email(&self) -> Option<RecordedEmail> {
        self.emails.lock().unwrap().last().cloned()
    }

    /// Clear all recorded emails
    pub fn clear(&self) {
        self.emails.lock().unwrap().clear();
    }

    /// Get emails sent to a specific address
    pub fn emails_to(&self, address: &str) -> Vec<RecordedEmail> {
        self.emails
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.to == address)
            .cloned()
            .collect()
    }
}

impl Default for FakeEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailSender for FakeEmailSender {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        if *self.should_fail.lock().unwrap() {
            return Err(EmailError::SendError("Simulated failure".to_string()));
        }

        self.emails.lock().unwrap().push(RecordedEmail {
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        });

        Ok(())
    }
}

/// Format verification code email body
pub fn format_verification_email(code: &str, expiration_minutes: u64) -> String {
    format!(
        "Your verification code is: {}\n\n\
         This code will expire in {} minutes.\n\n\
         If you did not request this code, please ignore this email.",
        code, expiration_minutes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_email_sender_records_emails() {
        let sender = FakeEmailSender::new();

        sender
            .send_email("test@example.com", "Test Subject", "Test Body")
            .await
            .unwrap();

        let emails = sender.get_emails();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].to, "test@example.com");
        assert_eq!(emails[0].subject, "Test Subject");
        assert_eq!(emails[0].body, "Test Body");
    }

    #[tokio::test]
    async fn test_fake_email_sender_can_fail() {
        let sender = FakeEmailSender::new();
        sender.set_should_fail(true);

        let result = sender.send_email("test@example.com", "Test", "Body").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fake_email_sender_filter_by_recipient() {
        let sender = FakeEmailSender::new();

        sender.send_email("a@test.com", "S1", "B1").await.unwrap();
        sender.send_email("b@test.com", "S2", "B2").await.unwrap();
        sender.send_email("a@test.com", "S3", "B3").await.unwrap();

        let emails_to_a = sender.emails_to("a@test.com");
        assert_eq!(emails_to_a.len(), 2);
    }

    #[test]
    fn test_format_verification_email() {
        let body = format_verification_email("123456", 10);
        assert!(body.contains("123456"));
        assert!(body.contains("10 minutes"));
    }
}
