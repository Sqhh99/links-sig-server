//! Database test helpers
//!
//! Provides utilities for setting up and cleaning test databases.

use sqlx::PgPool;

/// Create a connection pool to the test database
pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://links_sig_test:links_sig_test_password@localhost:5433/links_sig_test".to_string());

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Run migrations on the test database
pub async fn run_test_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}

/// Clean up test data from specific tables
pub async fn cleanup_test_data(pool: &PgPool) {
    // Delete in reverse order of foreign key dependencies
    sqlx::query("DELETE FROM email_verification_codes")
        .execute(pool)
        .await
        .expect("Failed to clean email_verification_codes");

    sqlx::query("DELETE FROM users")
        .execute(pool)
        .await
        .expect("Failed to clean users");
}

/// Clean up test data for a specific email
pub async fn cleanup_test_user(pool: &PgPool, email: &str) {
    sqlx::query("DELETE FROM email_verification_codes WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await
        .expect("Failed to clean email_verification_codes");

    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await
        .expect("Failed to clean users");
}
