//! Main SyncRelay server coordination.
//!
//! SyncRelay manages storage, active sessions, and coordinates message routing.

use crate::config::Config;
use crate::limits::RateLimits;
use crate::storage::SqliteStorage;
use dashmap::DashMap;
use iroh::endpoint::Connection;
use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use sync_types::{Cursor, DeviceId, GroupId, Message, Notify};
use tokio::sync::RwLock;

/// Operational metrics for monitoring relay activity.
///
/// All counters are monotonically increasing (reset only on restart).
/// Thread-safe via `AtomicU64` — no locks needed for incrementing.
#[derive(Debug, Default)]
pub struct RelayMetrics {
    /// Total PUSH requests handled successfully.
    pub pushes_total: AtomicU64,
    /// Total PULL requests handled successfully.
    pub pulls_total: AtomicU64,
    /// Total connections accepted (before session establishment).
    pub connections_total: AtomicU64,
    /// Total ciphertext bytes received (push payloads).
    pub bytes_received: AtomicU64,
    /// Total ciphertext bytes sent (pull response payloads).
    pub bytes_sent: AtomicU64,
    /// Total blobs stored in the database.
    pub blobs_stored: AtomicU64,
    /// Total rate limit rejections (connection + message + global).
    pub rate_limit_hits: AtomicU64,
    /// Total protocol errors (invalid messages, auth failures, etc.).
    pub errors_total: AtomicU64,
}

/// Active session tracking for a group.
#[derive(Debug, Default)]
struct GroupSessions {
    /// Set of device IDs with active connections.
    devices: HashSet<DeviceId>,
}

/// Main relay server.
pub struct SyncRelay {
    config: Config,
    storage: Arc<SqliteStorage>,
    /// Rate limiters for connections and messages.
    rate_limits: RateLimits,
    /// Operational metrics (counters, gauges).
    metrics: RelayMetrics,
    /// Active sessions per group.
    sessions: DashMap<GroupId, Arc<RwLock<GroupSessions>>>,
    /// Active connections for NOTIFY delivery (stored separately to preserve Debug on GroupSessions).
    notify_connections: DashMap<(GroupId, DeviceId), Connection>,
}

impl std::fmt::Debug for SyncRelay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncRelay")
            .field("config", &self.config)
            .field("rate_limits", &self.rate_limits)
            .field("metrics", &self.metrics)
            .field("sessions_count", &self.sessions.len())
            .finish_non_exhaustive()
    }
}

impl SyncRelay {
    /// Create a new SyncRelay with the given config and storage.
    pub fn new(config: Config, storage: SqliteStorage) -> Self {
        let rate_limits = RateLimits::new(&config.limits);
        Self {
            config,
            storage: Arc::new(storage),
            rate_limits,
            metrics: RelayMetrics::default(),
            sessions: DashMap::new(),
            notify_connections: DashMap::new(),
        }
    }

    /// Get the relay configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get access to the storage layer.
    pub fn storage(&self) -> &SqliteStorage {
        &self.storage
    }

    /// Get a clone of the storage Arc for background tasks.
    pub fn storage_arc(&self) -> Arc<SqliteStorage> {
        self.storage.clone()
    }

    /// Get access to the rate limiters.
    pub fn rate_limits(&self) -> &RateLimits {
        &self.rate_limits
    }

    /// Get access to the operational metrics.
    pub fn metrics(&self) -> &RelayMetrics {
        &self.metrics
    }

    /// Register a session (device connected to a group).
    pub async fn register_session(&self, group_id: &GroupId, device_id: &DeviceId) {
        let sessions = self
            .sessions
            .entry(*group_id)
            .or_insert_with(|| Arc::new(RwLock::new(GroupSessions::default())));

        let mut guard = sessions.write().await;
        guard.devices.insert(*device_id);

        tracing::debug!(
            "Registered session: device={:?} in group={:?} (total: {})",
            device_id,
            group_id,
            guard.devices.len()
        );
    }

    /// Register a connection for NOTIFY delivery.
    ///
    /// Called alongside `register_session` when a device completes HELLO.
    pub async fn register_connection(
        &self,
        group_id: &GroupId,
        device_id: &DeviceId,
        connection: Connection,
    ) {
        self.notify_connections
            .insert((*group_id, *device_id), connection);
    }

    /// Unregister a session (device disconnected).
    pub async fn unregister_session(&self, group_id: &GroupId, device_id: &DeviceId) {
        if let Some(sessions) = self.sessions.get(group_id) {
            let mut guard = sessions.write().await;
            guard.devices.remove(device_id);

            tracing::debug!(
                "Unregistered session: device={:?} from group={:?} (remaining: {})",
                device_id,
                group_id,
                guard.devices.len()
            );
        }

        // Remove connection for notification delivery
        self.notify_connections.remove(&(*group_id, *device_id));
    }

    /// Get online device IDs for a group (excluding sender).
    pub async fn get_online_devices(
        &self,
        group_id: &GroupId,
        exclude: &DeviceId,
    ) -> Vec<DeviceId> {
        if let Some(sessions) = self.sessions.get(group_id) {
            let guard = sessions.read().await;
            guard
                .devices
                .iter()
                .filter(|d| *d != exclude)
                .copied()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Count active sessions for a group.
    pub async fn session_count(&self, group_id: &GroupId) -> usize {
        if let Some(sessions) = self.sessions.get(group_id) {
            let guard = sessions.read().await;
            guard.devices.len()
        } else {
            0
        }
    }

    /// Notify other online devices in a group about a new blob.
    ///
    /// This is fire-and-forget — we don't wait for delivery confirmation.
    /// Each notification is sent via a server-opened unidirectional QUIC stream.
    /// Clients that don't yet have a NOTIFY listener will have these streams
    /// buffered by the QUIC stack (harmless for small messages).
    pub async fn notify_group(&self, group_id: &GroupId, sender: &DeviceId, cursor: Cursor) {
        let online = self.get_online_devices(group_id, sender).await;

        if online.is_empty() {
            return;
        }

        let notify = Message::Notify(Notify {
            latest_cursor: cursor,
            count: 1,
        });

        let bytes = match notify.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to serialize NOTIFY: {}", e);
                return;
            }
        };

        let mut sent = 0;
        for device_id in &online {
            if let Some(conn) = self.notify_connections.get(&(*group_id, *device_id)) {
                let connection = conn.value().clone();
                let bytes = bytes.clone();
                let did = *device_id;
                tokio::spawn(async move {
                    if let Err(e) = deliver_notify(&connection, &bytes).await {
                        tracing::debug!("Failed to notify {:?}: {}", did, e);
                    }
                });
                sent += 1;
            }
        }

        tracing::debug!(
            "Sent NOTIFY to {}/{} devices in {:?} about cursor {}",
            sent,
            online.len(),
            group_id,
            cursor
        );
    }

    /// Get total active sessions across all groups.
    pub fn total_sessions(&self) -> usize {
        self.sessions
            .iter()
            .map(|entry| {
                // We can't easily await here, so use try_read
                entry
                    .value()
                    .try_read()
                    .map(|g| g.devices.len())
                    .unwrap_or(0)
            })
            .sum()
    }

    /// Get total active groups.
    pub fn total_groups(&self) -> usize {
        self.sessions.len()
    }
}

/// Deliver a NOTIFY message via a unidirectional QUIC stream.
///
/// Opens a new uni stream on the connection, writes the length-prefixed
/// NOTIFY message, and finishes the stream. Fire-and-forget.
async fn deliver_notify(connection: &Connection, message_bytes: &[u8]) -> Result<(), String> {
    let mut send = connection
        .open_uni()
        .await
        .map_err(|e| format!("open_uni failed: {e}"))?;

    // Length-prefixed framing (4 bytes, big-endian)
    let len = (message_bytes.len() as u32).to_be_bytes();
    send.write_all(&len)
        .await
        .map_err(|e| format!("write length failed: {e}"))?;

    send.write_all(message_bytes)
        .await
        .map_err(|e| format!("write payload failed: {e}"))?;

    send.finish().map_err(|e| format!("finish failed: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config::default()
    }

    #[tokio::test]
    async fn register_and_unregister_session() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group = GroupId::random();
        let device = DeviceId::random();

        // Register
        relay.register_session(&group, &device).await;
        assert_eq!(relay.session_count(&group).await, 1);

        // Unregister
        relay.unregister_session(&group, &device).await;
        assert_eq!(relay.session_count(&group).await, 0);
    }

    #[tokio::test]
    async fn multiple_devices_in_group() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group = GroupId::random();
        let device1 = DeviceId::random();
        let device2 = DeviceId::random();
        let device3 = DeviceId::random();

        relay.register_session(&group, &device1).await;
        relay.register_session(&group, &device2).await;
        relay.register_session(&group, &device3).await;

        assert_eq!(relay.session_count(&group).await, 3);

        // Get online devices excluding device1
        let online = relay.get_online_devices(&group, &device1).await;
        assert_eq!(online.len(), 2);
        assert!(!online.contains(&device1));
        assert!(online.contains(&device2));
        assert!(online.contains(&device3));
    }

    #[tokio::test]
    async fn sessions_isolated_per_group() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group_a = GroupId::random();
        let group_b = GroupId::random();
        let device1 = DeviceId::random();
        let device2 = DeviceId::random();

        relay.register_session(&group_a, &device1).await;
        relay.register_session(&group_b, &device2).await;

        assert_eq!(relay.session_count(&group_a).await, 1);
        assert_eq!(relay.session_count(&group_b).await, 1);
        assert_eq!(relay.total_groups(), 2);
    }

    #[tokio::test]
    async fn unregister_cleans_up_connection_slot() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group = GroupId::random();
        let device = DeviceId::random();

        relay.register_session(&group, &device).await;
        assert_eq!(relay.session_count(&group).await, 1);

        // Unregister should clean up both session and connection slot
        relay.unregister_session(&group, &device).await;
        assert_eq!(relay.session_count(&group).await, 0);
        // Connection map should also be empty (no connection was registered, but no panic)
        assert!(!relay.notify_connections.contains_key(&(group, device)));
    }

    #[tokio::test]
    async fn notify_group_skips_when_no_devices() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group = GroupId::random();
        let sender = DeviceId::random();

        // Should not panic or error when no devices online
        relay.notify_group(&group, &sender, Cursor::new(1)).await;
    }

    #[tokio::test]
    async fn notify_group_excludes_sender() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group = GroupId::random();
        let sender = DeviceId::random();

        // Only the sender is online — should skip notification
        relay.register_session(&group, &sender).await;
        relay.notify_group(&group, &sender, Cursor::new(1)).await;

        // No crash, no notification attempted (no connections stored)
    }

    #[test]
    fn notify_message_serializes_correctly() {
        let notify = Message::Notify(Notify {
            latest_cursor: Cursor::new(42),
            count: 3,
        });

        let bytes = notify.to_bytes().unwrap();
        let decoded = Message::from_bytes(&bytes).unwrap();

        assert_eq!(decoded, notify);
        if let Message::Notify(n) = decoded {
            assert_eq!(n.latest_cursor, Cursor::new(42));
            assert_eq!(n.count, 3);
        } else {
            panic!("Expected Notify message");
        }
    }

    #[tokio::test]
    async fn total_sessions_counts_all() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let relay = SyncRelay::new(test_config(), storage);

        let group = GroupId::random();
        relay.register_session(&group, &DeviceId::random()).await;
        relay.register_session(&group, &DeviceId::random()).await;
        relay.register_session(&group, &DeviceId::random()).await;

        assert_eq!(relay.total_sessions(), 3);
    }
}
