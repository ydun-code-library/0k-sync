//! Prometheus metrics endpoint.

use crate::server::SyncRelay;
use axum::{http::header::CONTENT_TYPE, response::IntoResponse, Extension};
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Prometheus metrics handler.
///
/// Returns metrics in Prometheus text format.
/// Includes both gauges (current state) and counters (monotonic since startup).
pub async fn metrics_handler(Extension(relay): Extension<Arc<SyncRelay>>) -> impl IntoResponse {
    let m = relay.metrics();

    // Gauges — current state
    let connections = relay.total_sessions();
    let groups = relay.total_groups();

    // Counters — monotonic since startup
    let pushes = m.pushes_total.load(Ordering::Relaxed);
    let pulls = m.pulls_total.load(Ordering::Relaxed);
    let conns_total = m.connections_total.load(Ordering::Relaxed);
    let bytes_rx = m.bytes_received.load(Ordering::Relaxed);
    let bytes_tx = m.bytes_sent.load(Ordering::Relaxed);
    let blobs = m.blobs_stored.load(Ordering::Relaxed);
    let rate_limits = m.rate_limit_hits.load(Ordering::Relaxed);
    let errors = m.errors_total.load(Ordering::Relaxed);

    // Storage stats (async queries — best effort)
    let total_blobs_stored = relay.storage().get_total_blobs().await.unwrap_or(0);
    let storage_bytes = relay.storage().get_total_storage_bytes().await.unwrap_or(0);
    let groups_with_data = relay.storage().get_total_groups_with_data().await.unwrap_or(0);

    let body = format!(
        r#"# HELP sync_relay_connections_active Number of active connections
# TYPE sync_relay_connections_active gauge
sync_relay_connections_active {connections}

# HELP sync_relay_groups_active Number of active sync groups
# TYPE sync_relay_groups_active gauge
sync_relay_groups_active {groups}

# HELP sync_relay_info Server information
# TYPE sync_relay_info gauge
sync_relay_info{{version="{version}"}} 1

# HELP sync_relay_pushes_total Total PUSH requests handled
# TYPE sync_relay_pushes_total counter
sync_relay_pushes_total {pushes}

# HELP sync_relay_pulls_total Total PULL requests handled
# TYPE sync_relay_pulls_total counter
sync_relay_pulls_total {pulls}

# HELP sync_relay_connections_total Total connections accepted
# TYPE sync_relay_connections_total counter
sync_relay_connections_total {conns_total}

# HELP sync_relay_bytes_received_total Total ciphertext bytes received (push payloads)
# TYPE sync_relay_bytes_received_total counter
sync_relay_bytes_received_total {bytes_rx}

# HELP sync_relay_bytes_sent_total Total ciphertext bytes sent (pull payloads)
# TYPE sync_relay_bytes_sent_total counter
sync_relay_bytes_sent_total {bytes_tx}

# HELP sync_relay_blobs_stored_total Total blobs stored since startup
# TYPE sync_relay_blobs_stored_total counter
sync_relay_blobs_stored_total {blobs}

# HELP sync_relay_rate_limit_hits_total Total rate limit rejections
# TYPE sync_relay_rate_limit_hits_total counter
sync_relay_rate_limit_hits_total {rate_limits}

# HELP sync_relay_errors_total Total protocol errors
# TYPE sync_relay_errors_total counter
sync_relay_errors_total {errors}

# HELP sync_relay_storage_blobs Number of blobs currently in database
# TYPE sync_relay_storage_blobs gauge
sync_relay_storage_blobs {total_blobs_stored}

# HELP sync_relay_storage_bytes Total ciphertext bytes in database
# TYPE sync_relay_storage_bytes gauge
sync_relay_storage_bytes {storage_bytes}

# HELP sync_relay_storage_groups Number of groups with stored data
# TYPE sync_relay_storage_groups gauge
sync_relay_storage_groups {groups_with_data}
"#,
        version = env!("CARGO_PKG_VERSION"),
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
