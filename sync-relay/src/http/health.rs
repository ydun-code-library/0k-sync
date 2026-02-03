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
    /// Number of active groups.
    pub groups: usize,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
}

/// Health check handler.
pub async fn health_handler(Extension(relay): Extension<Arc<SyncRelay>>) -> Json<HealthStatus> {
    let uptime = START_TIME
        .get()
        .map(|start| start.elapsed().as_secs())
        .unwrap_or(0);

    Json(HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        connections: relay.total_sessions(),
        groups: relay.total_groups(),
        uptime_seconds: uptime,
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
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"connections\":42"));
    }
}
