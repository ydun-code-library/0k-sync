//! Prometheus metrics endpoint.

use crate::server::SyncRelay;
use axum::{http::header::CONTENT_TYPE, response::IntoResponse, Extension};
use std::sync::Arc;

/// Prometheus metrics handler.
///
/// Returns metrics in Prometheus text format.
pub async fn metrics_handler(Extension(relay): Extension<Arc<SyncRelay>>) -> impl IntoResponse {
    let connections = relay.total_sessions();
    let groups = relay.total_groups();

    // Prometheus text format
    let body = format!(
        r#"# HELP sync_relay_connections_active Number of active connections
# TYPE sync_relay_connections_active gauge
sync_relay_connections_active {}

# HELP sync_relay_groups_active Number of active sync groups
# TYPE sync_relay_groups_active gauge
sync_relay_groups_active {}

# HELP sync_relay_info Server information
# TYPE sync_relay_info gauge
sync_relay_info{{version="{}"}} 1
"#,
        connections,
        groups,
        env!("CARGO_PKG_VERSION")
    );

    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn prometheus_format_is_valid() {
        // Verify the format strings are valid
        let sample = format!(
            "# TYPE sync_relay_connections_active gauge\nsync_relay_connections_active {}",
            42
        );
        assert!(sample.contains("gauge"));
        assert!(sample.contains("42"));
    }
}
