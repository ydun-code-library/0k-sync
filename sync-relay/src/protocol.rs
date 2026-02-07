//! Protocol handler for /0k-sync/1 ALPN.
//!
//! Implements iroh's ProtocolHandler trait to accept incoming connections.

use crate::server::SyncRelay;
use crate::session::Session;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Protocol identifier for 0k-Sync.
pub const ALPN: &[u8] = b"/0k-sync/1";

/// Maximum message size (1MB per blob limit from spec).
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Protocol handler for accepting 0k-Sync connections.
#[derive(Clone, Debug)]
pub struct SyncProtocol {
    relay: Arc<SyncRelay>,
}

impl SyncProtocol {
    /// Create a new protocol handler.
    pub fn new(relay: Arc<SyncRelay>) -> Self {
        Self { relay }
    }
}

impl ProtocolHandler for SyncProtocol {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl std::future::Future<Output = Result<(), AcceptError>> + Send {
        let relay = self.relay.clone();
        async move {
            // Rate limit check: prevent connection flooding from single device
            let remote_id = connection.remote_id();
            if let Err(e) = relay.rate_limits().check_connection(remote_id.as_bytes()) {
                tracing::warn!("Connection rate limited for {}: {}", remote_id, e);
                relay.metrics().rate_limit_hits.fetch_add(1, Ordering::Relaxed);
                connection.close(1u32.into(), b"rate limited");
                return Ok(());
            }

            // F-007: Reject if at session capacity
            let max_sessions = relay.config().limits.max_concurrent_sessions;
            if relay.total_sessions() >= max_sessions {
                tracing::warn!(
                    "Session limit reached ({}/{}), rejecting {}",
                    relay.total_sessions(),
                    max_sessions,
                    remote_id
                );
                connection.close(2u32.into(), b"too many sessions");
                return Ok(());
            }

            relay.metrics().connections_total.fetch_add(1, Ordering::Relaxed);

            let session = Session::new(relay, connection);
            // Spawn session handler - don't block the accept loop
            tokio::spawn(async move {
                if let Err(e) = session.run().await {
                    tracing::warn!("Session error: {}", e);
                }
            });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpn_matches_client() {
        // This ALPN must match sync-client/src/transport/iroh.rs
        assert_eq!(ALPN, b"/0k-sync/1");
    }

    #[test]
    fn max_message_size_is_1mb() {
        assert_eq!(MAX_MESSAGE_SIZE, 1024 * 1024);
    }
}
