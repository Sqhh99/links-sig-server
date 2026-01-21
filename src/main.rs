//! LiveKit Signaling Server
//!
//! A Rust backend server for LiveKit video conferencing, built with Axum.
//!
//! # Architecture
//!
//! - `main.rs` - Application entry point, server initialization
//! - `config.rs` - Configuration loading from environment
//! - `state.rs` - Shared application state (AppState)
//! - `routes.rs` - Centralized route definitions
//! - `handlers/` - HTTP request handlers (thin layer)
//! - `services/` - Business logic layer
//! - `integrations/` - External service adapters (LiveKit, Database, Email)
//! - `auth/` - JWT authentication and extractors
//! - `types/` - Request/Response DTOs and error types

mod auth;
mod config;
mod handlers;
mod integrations;
mod routes;
mod services;
mod state;
mod types;

use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::Config;
use crate::integrations::{create_pool, run_migrations, SmtpEmailSender};
use crate::routes::build_api_router;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    init_logging();

    // Load .env file
    if let Err(e) = dotenvy::dotenv() {
        info!(".env file not found, using default config: {}", e);
    }

    // Load configuration
    let config = Config::from_env();
    log_startup_info(&config);

    // Initialize database connection pool
    info!("Connecting to database...");
    let db = create_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");
    info!("Database connection established");

    // Run migrations
    info!("Running database migrations...");
    if let Err(e) = run_migrations(&db).await {
        error!("Failed to run migrations: {}", e);
        // Continue anyway - migrations might already be applied
    } else {
        info!("Database migrations completed");
    }

    // Initialize email sender
    let email = Arc::new(
        SmtpEmailSender::new(&config)
            .expect("Failed to initialize email sender"),
    );

    // Create application state
    let state = AppState::new(config.clone(), db, email);

    // Build and run server
    let app = build_app(state);
    run_server(app, &config).await;
}

/// Initialize the logging subsystem
fn init_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

/// Log startup information
fn log_startup_info(config: &Config) {
    let protocol = if config.enable_https { "https" } else { "http" };

    info!("LiveKit Signaling Server started successfully");
    info!(
        "Server address: {}://{}:{}",
        protocol, config.server_host, config.server_port
    );
    info!("LiveKit API: {}", config.livekit_url);
    info!("LiveKit WebSocket: {}", config.livekit_ws_url);
    info!("API Key: {}", config.api_key);
    info!("HTTPS enabled: {}", config.enable_https);
    info!("Database: {}", mask_connection_string(&config.database_url));
    info!("SMTP Host: {}", config.smtp_host);
}

/// Mask sensitive parts of connection string
fn mask_connection_string(url: &str) -> String {
    // postgres://user:password@host:port/db -> postgres://user:***@host:port/db
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let prefix = &url[..colon_pos + 1];
            let suffix = &url[at_pos..];
            return format!("{}***{}", prefix, suffix);
        }
    }
    url.to_string()
}

/// Build the complete application router
fn build_app(state: AppState) -> Router {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(false); // Note: Can't use credentials with Any origin

    // Static files path - look in parent directory's static folder
    let static_path = PathBuf::from("../static");
    let index_file = static_path.join("index.html");

    // Build API routes
    let api_routes = build_api_router();

    // Create main router with static file serving
    Router::new()
        .nest("/api", api_routes)
        .fallback_service(
            ServeDir::new(&static_path).not_found_service(ServeFile::new(&index_file)),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Run the HTTP/HTTPS server
async fn run_server(app: Router, config: &Config) {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));

    if config.enable_https {
        info!("Using TLS certificate: {}", config.ssl_cert_file);

        // HTTPS server with rustls
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &config.ssl_cert_file,
            &config.ssl_key_file,
        )
        .await
        .expect("Failed to load TLS certificates");

        axum_server::bind_rustls(addr, rustls_config)
            .serve(app.into_make_service())
            .await
            .expect("HTTPS server failed");
    } else {
        // HTTP server
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind address");
        axum::serve(listener, app)
            .await
            .expect("HTTP server failed");
    }
}
