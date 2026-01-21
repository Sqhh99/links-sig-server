//! Database integration - PostgreSQL connection pool
//!
//! Provides database connection pool creation and management.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Create a PostgreSQL connection pool
///
/// # Arguments
/// * `database_url` - PostgreSQL connection string
///
/// # Returns
/// A configured PgPool ready for use
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await
}

/// Run database migrations
///
/// Executes all pending migrations from the migrations directory.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_create_pool() {
        let url = "postgres://links_sig_test:links_sig_test_password@localhost:5433/links_sig_test";
        let pool = create_pool(url).await;
        assert!(pool.is_ok());
    }
}
