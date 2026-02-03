//! Main SyncRelay server coordination.
//!
//! SyncRelay manages storage, active sessions, and coordinates message routing.

use crate::config::Config;
use crate::storage::SqliteStorage;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use sync_types::{Cursor, DeviceId, GroupId};
use tokio::sync::RwLock;

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
    /// Active sessions per group.
    sessions: DashMap<GroupId, Arc<RwLock<GroupSessions>>>,
}

impl std::fmt::Debug for SyncRelay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncRelay")
            .field("config", &self.config)
            .field("sessions_count", &self.sessions.len())
            .finish_non_exhaustive()
    }
}

impl SyncRelay {
    /// Create a new SyncRelay with the given config and storage.
    pub fn new(config: Config, storage: SqliteStorage) -> Self {
        Self {
            config,
            storage: Arc::new(storage),
            sessions: DashMap::new(),
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
    /// This is fire-and-forget - we don't wait for delivery.
    pub async fn notify_group(&self, group_id: &GroupId, sender: &DeviceId, cursor: Cursor) {
        let online = self.get_online_devices(group_id, sender).await;

        if online.is_empty() {
            return;
        }

        // For now, we just log the notification intent
        // Full implementation will push NOTIFY messages to connected sessions
        tracing::debug!(
            "Would notify {} devices in {:?} about cursor {}",
            online.len(),
            group_id,
            cursor
        );

        // TODO: Implement actual notification delivery
        // This requires tracking send channels per session, which is more complex.
        // For Phase 6 MVP, clients will poll with PULL.
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
