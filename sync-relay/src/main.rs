//! sync-relay binary entry point.
//!
//! Usage:
//! ```bash
//! sync-relay --config relay.toml
//! sync-relay --help
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use zerok_sync_relay::config::Config;
use zerok_sync_relay::http;
use zerok_sync_relay::protocol::{SyncProtocol, ALPN};
use zerok_sync_relay::server::SyncRelay;
use zerok_sync_relay::storage::SqliteStorage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sync_relay=info".parse().unwrap())
                .add_directive("iroh=warn".parse().unwrap()),
        )
        .init();

    // Initialize health check start time
    http::health::init_start_time();

    // Parse arguments
    let config_path = get_config_path();

    // Load config
    let config = if config_path.exists() {
        tracing::info!("Loading config from {:?}", config_path);
        Config::from_file(&config_path)?
    } else {
        tracing::info!("Using default config (no config file found)");
        Config::default()
    };

    tracing::info!("sync-relay v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("iroh bind: {}", config.server.bind_address);
    tracing::info!("HTTP bind: {}", config.http.bind_address);
    tracing::info!("Database: {:?}", config.storage.database);

    // Initialize storage
    let storage = SqliteStorage::new(&config.storage.database).await?;
    tracing::info!("Storage initialized");

    // Create relay
    let relay = Arc::new(SyncRelay::new(config.clone(), storage));

    // Create iroh endpoint with default discovery (DNS + Pkarr)
    let endpoint = iroh::Endpoint::builder().bind().await?;

    let endpoint_id = endpoint.id();
    tracing::info!("Endpoint ID: {}", endpoint_id);

    // Create protocol handler
    let protocol = SyncProtocol::new(relay.clone());

    // Start iroh Router
    let router = iroh::protocol::Router::builder(endpoint)
        .accept(ALPN, protocol)
        .spawn();

    tracing::info!("iroh router started, accepting connections on ALPN {:?}",
        std::str::from_utf8(ALPN).unwrap_or("?"));

    // Start HTTP server
    let http_addr: SocketAddr = config.http.bind_address.parse()?;
    let http_router = http::build_router(relay.clone());

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    tracing::info!("HTTP server listening on {}", http_addr);

    // Run HTTP server in background
    let http_handle = tokio::spawn(async move {
        axum::serve(http_listener, http_router).await
    });

    // Print connection info
    println!();
    println!("=== sync-relay running ===");
    println!("Endpoint ID: {}", endpoint_id);
    println!("HTTP: http://{}/health", http_addr);
    println!();
    println!("Press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal");
        }
        result = http_handle => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
    }

    // Graceful shutdown
    tracing::info!("Shutting down...");
    router.shutdown().await?;
    tracing::info!("Goodbye!");

    Ok(())
}

fn get_config_path() -> PathBuf {
    std::env::args()
        .skip_while(|arg| arg != "--config")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("relay.toml"))
}
