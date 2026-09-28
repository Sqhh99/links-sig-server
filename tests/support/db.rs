//! Database test helpers
//!
//! Provides utilities for setting up and cleaning test databases.

use sqlx::PgPool;
use std::time::Duration;

/// Create a connection pool to the test database
pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://links_sig_test:links_sig_test_password@127.0.0.1:5433/links_sig_test"
            .to_string()
    });

    let mut last_error = None;

    for attempt in 1..=10 {
        match sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await
        {
            Ok(pool) => return pool,
            Err(err) => {
                last_error = Some(err);
                if attempt < 10 {
                    tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                }
            }
        }
    }

    panic!(
        "Failed to connect to test database after retries. \
TEST_DATABASE_URL={}.\n\
Hint: start test DB with `docker compose up -d postgres-test`.\n\
Last error: {}",
        database_url,
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
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
    sqlx::query("DELETE FROM meeting_participants")
        .execute(pool)
        .await
        .expect("Failed to clean meeting_participants");

    sqlx::query("DELETE FROM meetings")
        .execute(pool)
        .await
        .expect("Failed to clean meetings");

    sqlx::query("DELETE FROM users")
        .execute(pool)
        .await
        .expect("Failed to clean users");
}

/// Clean up test data for a specific username
pub async fn cleanup_test_user(pool: &PgPool, username: &str) {
    sqlx::query(
        r#"
        DELETE FROM meeting_participants mp
        USING users u
        WHERE mp.user_id = u.id
          AND LOWER(u.username) = LOWER($1)
        "#,
    )
    .bind(username)
    .execute(pool)
    .await
    .expect("Failed to clean meeting_participants");

    sqlx::query(
        r#"
        DELETE FROM meetings m
        USING users u
        WHERE m.creator_user_id = u.id
          AND LOWER(u.username) = LOWER($1)
        "#,
    )
    .bind(username)
    .execute(pool)
    .await
    .expect("Failed to clean meetings");

    sqlx::query("DELETE FROM users WHERE LOWER(username) = LOWER($1)")
        .bind(username)
        .execute(pool)
        .await
        .expect("Failed to clean users");
}
