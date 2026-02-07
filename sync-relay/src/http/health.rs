//! Health check endpoint.

use crate::server::SyncRelay;
use axum::{Extension, Json};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

/// Global start time for uptime calculation.
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Initialize the start time (call once at startup).
pub fn init_start_time() {
    START_TIME.get_or_init(Instant::now);
}

/// Health status response.
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    /// Overall status.
    pub status: String,
    /// Server version.
    pub version: String,
    /// Number of active connections.
    pub connections: usize,
    /// Number of active groups (with sessions).
    pub groups: usize,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// Total blobs stored in the database.
    pub total_blobs: u64,
    /// Total storage used across all groups (bytes).
    pub storage_bytes: u64,
    /// Total groups with stored data.
    pub groups_with_data: u64,
}

/// Health check handler.
pub async fn health_handler(Extension(relay): Extension<Arc<SyncRelay>>) -> Json<HealthStatus> {
    let uptime = START_TIME
        .get()
        .map(|start| start.elapsed().as_secs())
        .unwrap_or(0);

    let total_blobs = relay.storage().get_total_blobs().await.unwrap_or(0);
    let storage_bytes = relay.storage().get_total_storage_bytes().await.unwrap_or(0);
    let groups_with_data = relay.storage().get_total_groups_with_data().await.unwrap_or(0);

    Json(HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        connections: relay.total_sessions(),
        groups: relay.total_groups(),
        uptime_seconds: uptime,
        total_blobs,
        storage_bytes,
        groups_with_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_serializes() {
        let status = HealthStatus {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            connections: 42,
            groups: 15,
            uptime_seconds: 3600,
            total_blobs: 100,
            storage_bytes: 51200,
            groups_with_data: 5,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"connections\":42"));
        assert!(json.contains("\"total_blobs\":100"));
        assert!(json.contains("\"storage_bytes\":51200"));
    }
}
