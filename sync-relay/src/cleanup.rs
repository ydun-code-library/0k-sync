//! Background cleanup task for expired blobs.
//!
//! Runs periodically to delete blobs that have exceeded their TTL.

use crate::config::CleanupConfig;
use crate::storage::{BlobStorage, SqliteStorage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

/// Spawn a background cleanup task.
///
/// Returns a handle that can be used to abort the task.
pub fn spawn_cleanup_task(
    storage: Arc<SqliteStorage>,
    config: CleanupConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !config.enabled {
            tracing::info!("Cleanup task disabled");
            return;
        }

        let interval_secs = config.interval_secs;
        tracing::info!("Cleanup task started (interval: {}s)", interval_secs);

        let mut timer = interval(Duration::from_secs(interval_secs));

        loop {
            timer.tick().await;

            match storage.cleanup_expired().await {
                Ok(deleted) => {
                    if deleted > 0 {
                        tracing::info!("Cleanup: deleted {} expired blobs", deleted);
                    } else {
                        tracing::debug!("Cleanup: no expired blobs");
                    }
                }
                Err(e) => {
                    tracing::error!("Cleanup error: {}", e);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CleanupConfig;
    use crate::storage::{BlobStorage, SqliteStorage, StoreBlobRequest};
    use sync_types::{BlobId, DeviceId, GroupId};

    fn test_cleanup_config(interval_secs: u64) -> CleanupConfig {
        CleanupConfig {
            interval_secs,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn cleanup_task_removes_expired_blobs() {
        let storage = Arc::new(SqliteStorage::in_memory().await.unwrap());
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        // Store a blob with 0 TTL (already expired)
        storage
            .store_blob(StoreBlobRequest {
                blob_id: BlobId::new(),
                group_id,
                sender_id: device_id,
                payload: b"expired".to_vec(),
                timestamp: 0,
                ttl_secs: 0,
            })
            .await
            .unwrap();

        // Wait briefly for expiration
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Run cleanup directly
        let deleted = storage.cleanup_expired().await.unwrap();
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn cleanup_task_disabled() {
        let storage = Arc::new(SqliteStorage::in_memory().await.unwrap());
        let config = CleanupConfig {
            interval_secs: 1,
            enabled: false,
        };

        let handle = spawn_cleanup_task(storage, config);

        // Task should complete immediately when disabled
        tokio::time::timeout(Duration::from_millis(100), handle)
            .await
            .expect("Task should complete when disabled")
            .expect("Task should not panic");
    }
}
