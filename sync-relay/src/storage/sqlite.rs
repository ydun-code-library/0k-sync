//! SQLite storage backend for sync-relay.

use super::{BlobStorage, StoreBlobRequest, StoredBlob};
use crate::error::StorageError;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use sync_types::{BlobId, Cursor, DeviceId, GroupId};

/// SQLite-based blob storage.
///
/// Uses WAL mode for concurrent reads/writes.
#[derive(Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage from a database path.
    ///
    /// Creates the database file if it doesn't exist.
    pub async fn new(path: &Path) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(path.to_str().unwrap_or("relay.db"))
            .map_err(StorageError::Database)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await
            .map_err(StorageError::Database)?;

        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    /// Create an in-memory SQLite storage (for testing).
    pub async fn in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(":memory:")
            .map_err(StorageError::Database)?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(StorageError::Database)?;

        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<(), StorageError> {
        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_cursors (
                group_id BLOB PRIMARY KEY,
                next_cursor INTEGER NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blobs (
                blob_id BLOB PRIMARY KEY,
                group_id BLOB NOT NULL,
                cursor INTEGER NOT NULL,
                sender_id BLOB NOT NULL,
                payload BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                UNIQUE(group_id, cursor)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deliveries (
                blob_id BLOB NOT NULL,
                device_id BLOB NOT NULL,
                delivered_at INTEGER,
                PRIMARY KEY (blob_id, device_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        // Create indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_blobs_group_cursor ON blobs(group_id, cursor)",
        )
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_blobs_expires ON blobs(expires_at)")
            .execute(&self.pool)
            .await
            .map_err(StorageError::Database)?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_blobs_group_id ON blobs(group_id)")
            .execute(&self.pool)
            .await
            .map_err(StorageError::Database)?;

        Ok(())
    }

    /// Atomically get and increment the cursor for a group.
    async fn next_cursor(&self, group_id: &GroupId) -> Result<Cursor, StorageError> {
        // Use INSERT OR REPLACE pattern for atomic cursor assignment
        let cursor: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO group_cursors (group_id, next_cursor)
            VALUES (?1, 2)
            ON CONFLICT(group_id) DO UPDATE SET next_cursor = next_cursor + 1
            RETURNING next_cursor - 1
            "#,
        )
        .bind(group_id.as_bytes().as_slice())
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(Cursor::new(cursor as u64))
    }

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

#[async_trait]
impl BlobStorage for SqliteStorage {
    async fn store_blob(&self, req: StoreBlobRequest) -> Result<Cursor, StorageError> {
        let cursor = self.next_cursor(&req.group_id).await?;
        let expires_at = Self::current_timestamp() + req.ttl_secs as i64;

        sqlx::query(
            r#"
            INSERT INTO blobs (blob_id, group_id, cursor, sender_id, payload, timestamp, expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(req.blob_id.as_bytes())
        .bind(req.group_id.as_bytes().as_slice())
        .bind(cursor.value() as i64)
        .bind(req.sender_id.as_bytes().as_slice())
        .bind(&req.payload)
        .bind(req.timestamp)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(cursor)
    }

    async fn get_blobs_after(
        &self,
        group_id: &GroupId,
        after: Cursor,
        limit: u32,
    ) -> Result<Vec<StoredBlob>, StorageError> {
        let rows = sqlx::query_as::<_, BlobRow>(
            r#"
            SELECT blob_id, group_id, cursor, sender_id, payload, timestamp, expires_at
            FROM blobs
            WHERE group_id = ?1 AND cursor > ?2
            ORDER BY cursor ASC
            LIMIT ?3
            "#,
        )
        .bind(group_id.as_bytes().as_slice())
        .bind(after.value() as i64)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        rows.into_iter().map(|row| row.try_into()).collect()
    }

    async fn get_max_cursor(&self, group_id: &GroupId) -> Result<Cursor, StorageError> {
        let cursor: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT MAX(cursor) FROM blobs WHERE group_id = ?1
            "#,
        )
        .bind(group_id.as_bytes().as_slice())
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(Cursor::new(cursor.unwrap_or(0) as u64))
    }

    async fn mark_delivered(
        &self,
        blob_id: &BlobId,
        device_id: &DeviceId,
    ) -> Result<(), StorageError> {
        let now = Self::current_timestamp();

        sqlx::query(
            r#"
            INSERT INTO deliveries (blob_id, device_id, delivered_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(blob_id, device_id) DO UPDATE SET delivered_at = ?3
            "#,
        )
        .bind(blob_id.as_bytes())
        .bind(device_id.as_bytes().as_slice())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(())
    }

    async fn mark_delivered_batch(
        &self,
        blob_ids: &[BlobId],
        device_id: &DeviceId,
    ) -> Result<(), StorageError> {
        if blob_ids.is_empty() {
            return Ok(());
        }

        let now = Self::current_timestamp();
        let device_bytes = device_id.as_bytes();

        // Use a transaction for batch insert
        let mut tx = self.pool.begin().await.map_err(StorageError::Database)?;

        for blob_id in blob_ids {
            sqlx::query(
                r#"
                INSERT INTO deliveries (blob_id, device_id, delivered_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(blob_id, device_id) DO UPDATE SET delivered_at = ?3
                "#,
            )
            .bind(blob_id.as_bytes())
            .bind(device_bytes.as_slice())
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(StorageError::Database)?;
        }

        tx.commit().await.map_err(StorageError::Database)?;
        Ok(())
    }

    async fn get_pending_count(
        &self,
        group_id: &GroupId,
        device_id: &DeviceId,
    ) -> Result<u32, StorageError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM blobs b
            WHERE b.group_id = ?1
              AND b.sender_id != ?2
              AND NOT EXISTS (
                  SELECT 1 FROM deliveries d
                  WHERE d.blob_id = b.blob_id AND d.device_id = ?2
              )
            "#,
        )
        .bind(group_id.as_bytes().as_slice())
        .bind(device_id.as_bytes().as_slice())
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(count as u32)
    }

    async fn cleanup_expired(&self) -> Result<u64, StorageError> {
        let now = Self::current_timestamp();

        // Delete deliveries for expired blobs using subquery (avoids N+1)
        sqlx::query(
            r#"
            DELETE FROM deliveries WHERE blob_id IN (
                SELECT blob_id FROM blobs WHERE expires_at <= ?1
            )
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        // Delete expired blobs
        let result = sqlx::query(
            r#"
            DELETE FROM blobs WHERE expires_at <= ?1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(result.rows_affected())
    }

    async fn get_group_storage(&self, group_id: &GroupId) -> Result<u64, StorageError> {
        let size: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT SUM(LENGTH(payload)) FROM blobs WHERE group_id = ?1
            "#,
        )
        .bind(group_id.as_bytes().as_slice())
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        Ok(size.unwrap_or(0) as u64)
    }

    async fn get_blob(&self, blob_id: &BlobId) -> Result<Option<StoredBlob>, StorageError> {
        let row = sqlx::query_as::<_, BlobRow>(
            r#"
            SELECT blob_id, group_id, cursor, sender_id, payload, timestamp, expires_at
            FROM blobs
            WHERE blob_id = ?1
            "#,
        )
        .bind(blob_id.as_bytes())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }
}

/// Internal row type for SQLite queries.
#[derive(sqlx::FromRow)]
struct BlobRow {
    blob_id: Vec<u8>,
    group_id: Vec<u8>,
    cursor: i64,
    sender_id: Vec<u8>,
    payload: Vec<u8>,
    timestamp: i64,
    expires_at: i64,
}

impl TryFrom<BlobRow> for StoredBlob {
    type Error = StorageError;

    fn try_from(row: BlobRow) -> Result<Self, Self::Error> {
        Ok(StoredBlob {
            blob_id: BlobId::from_bytes(&row.blob_id).ok_or_else(|| StorageError::NotFound {
                blob_id: hex::encode(&row.blob_id),
            })?,
            group_id: GroupId::from_bytes(&row.group_id).ok_or_else(|| StorageError::NotFound {
                blob_id: "invalid group_id".to_string(),
            })?,
            cursor: Cursor::new(row.cursor as u64),
            sender_id: DeviceId::from_bytes(&row.sender_id).ok_or_else(|| {
                StorageError::NotFound {
                    blob_id: "invalid sender_id".to_string(),
                }
            })?,
            payload: row.payload,
            timestamp: row.timestamp,
            expires_at: row.expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(group_id: &GroupId, sender_id: &DeviceId, payload: &[u8]) -> StoreBlobRequest {
        StoreBlobRequest {
            blob_id: BlobId::new(),
            group_id: *group_id,
            sender_id: *sender_id,
            payload: payload.to_vec(),
            timestamp: SqliteStorage::current_timestamp(),
            ttl_secs: 3600, // 1 hour
        }
    }

    #[tokio::test]
    async fn store_blob_assigns_cursor() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        let req = make_request(&group_id, &device_id, b"test payload");
        let cursor = storage.store_blob(req).await.unwrap();

        assert_eq!(cursor.value(), 1);
    }

    #[tokio::test]
    async fn cursors_are_monotonic_per_group() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        let c1 = storage
            .store_blob(make_request(&group_id, &device_id, b"one"))
            .await
            .unwrap();
        let c2 = storage
            .store_blob(make_request(&group_id, &device_id, b"two"))
            .await
            .unwrap();
        let c3 = storage
            .store_blob(make_request(&group_id, &device_id, b"three"))
            .await
            .unwrap();

        assert_eq!(c1.value(), 1);
        assert_eq!(c2.value(), 2);
        assert_eq!(c3.value(), 3);
    }

    #[tokio::test]
    async fn cursors_are_independent_per_group() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_a = GroupId::random();
        let group_b = GroupId::random();
        let device_id = DeviceId::random();

        let c_a1 = storage
            .store_blob(make_request(&group_a, &device_id, b"a1"))
            .await
            .unwrap();
        let c_b1 = storage
            .store_blob(make_request(&group_b, &device_id, b"b1"))
            .await
            .unwrap();
        let c_a2 = storage
            .store_blob(make_request(&group_a, &device_id, b"a2"))
            .await
            .unwrap();

        // Each group has its own cursor sequence
        assert_eq!(c_a1.value(), 1);
        assert_eq!(c_b1.value(), 1);
        assert_eq!(c_a2.value(), 2);
    }

    #[tokio::test]
    async fn get_blobs_after_cursor() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        // Store 5 blobs
        for i in 0..5 {
            storage
                .store_blob(make_request(
                    &group_id,
                    &device_id,
                    format!("blob-{}", i).as_bytes(),
                ))
                .await
                .unwrap();
        }

        // Get blobs after cursor 2
        let blobs = storage
            .get_blobs_after(&group_id, Cursor::new(2), 100)
            .await
            .unwrap();

        assert_eq!(blobs.len(), 3);
        assert_eq!(blobs[0].cursor.value(), 3);
        assert_eq!(blobs[1].cursor.value(), 4);
        assert_eq!(blobs[2].cursor.value(), 5);
    }

    #[tokio::test]
    async fn get_blobs_after_respects_limit() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        // Store 10 blobs
        for i in 0..10 {
            storage
                .store_blob(make_request(
                    &group_id,
                    &device_id,
                    format!("blob-{}", i).as_bytes(),
                ))
                .await
                .unwrap();
        }

        // Get with limit 3
        let blobs = storage
            .get_blobs_after(&group_id, Cursor::zero(), 3)
            .await
            .unwrap();

        assert_eq!(blobs.len(), 3);
    }

    #[tokio::test]
    async fn get_max_cursor() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        // Empty group has cursor 0
        let max = storage.get_max_cursor(&group_id).await.unwrap();
        assert_eq!(max.value(), 0);

        // Store some blobs
        storage
            .store_blob(make_request(&group_id, &device_id, b"one"))
            .await
            .unwrap();
        storage
            .store_blob(make_request(&group_id, &device_id, b"two"))
            .await
            .unwrap();

        let max = storage.get_max_cursor(&group_id).await.unwrap();
        assert_eq!(max.value(), 2);
    }

    #[tokio::test]
    async fn mark_delivered_tracks_delivery() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let sender = DeviceId::random();
        let receiver = DeviceId::random();

        // Store a blob from sender
        let req = make_request(&group_id, &sender, b"payload");
        let blob_id = req.blob_id;
        storage.store_blob(req).await.unwrap();

        // Initially pending for receiver
        let pending = storage.get_pending_count(&group_id, &receiver).await.unwrap();
        assert_eq!(pending, 1);

        // Mark as delivered
        storage.mark_delivered(&blob_id, &receiver).await.unwrap();

        // No longer pending
        let pending = storage.get_pending_count(&group_id, &receiver).await.unwrap();
        assert_eq!(pending, 0);
    }

    #[tokio::test]
    async fn mark_delivered_batch_tracks_multiple() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let sender = DeviceId::random();
        let receiver = DeviceId::random();

        // Store 3 blobs from sender
        let req1 = make_request(&group_id, &sender, b"one");
        let blob_id1 = req1.blob_id;
        storage.store_blob(req1).await.unwrap();

        let req2 = make_request(&group_id, &sender, b"two");
        let blob_id2 = req2.blob_id;
        storage.store_blob(req2).await.unwrap();

        let req3 = make_request(&group_id, &sender, b"three");
        let blob_id3 = req3.blob_id;
        storage.store_blob(req3).await.unwrap();

        // Initially 3 pending for receiver
        let pending = storage.get_pending_count(&group_id, &receiver).await.unwrap();
        assert_eq!(pending, 3);

        // Batch mark as delivered
        storage
            .mark_delivered_batch(&[blob_id1, blob_id2, blob_id3], &receiver)
            .await
            .unwrap();

        // No longer pending
        let pending = storage.get_pending_count(&group_id, &receiver).await.unwrap();
        assert_eq!(pending, 0);
    }

    #[tokio::test]
    async fn mark_delivered_batch_empty_is_noop() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let device_id = DeviceId::random();

        // Empty batch should succeed
        storage.mark_delivered_batch(&[], &device_id).await.unwrap();
    }

    #[tokio::test]
    async fn pending_count_excludes_own_blobs() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_a = DeviceId::random();
        let device_b = DeviceId::random();

        // Device A stores a blob
        storage
            .store_blob(make_request(&group_id, &device_a, b"from A"))
            .await
            .unwrap();

        // Device A doesn't see its own blob as pending
        let pending_a = storage.get_pending_count(&group_id, &device_a).await.unwrap();
        assert_eq!(pending_a, 0);

        // Device B sees it as pending
        let pending_b = storage.get_pending_count(&group_id, &device_b).await.unwrap();
        assert_eq!(pending_b, 1);
    }

    #[tokio::test]
    async fn get_group_storage() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        // Empty group
        let size = storage.get_group_storage(&group_id).await.unwrap();
        assert_eq!(size, 0);

        // Store some payloads
        storage
            .store_blob(make_request(&group_id, &device_id, &[0u8; 100]))
            .await
            .unwrap();
        storage
            .store_blob(make_request(&group_id, &device_id, &[0u8; 200]))
            .await
            .unwrap();

        let size = storage.get_group_storage(&group_id).await.unwrap();
        assert_eq!(size, 300);
    }

    #[tokio::test]
    async fn get_blob_by_id() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        let req = make_request(&group_id, &device_id, b"find me");
        let blob_id = req.blob_id;
        storage.store_blob(req).await.unwrap();

        let blob = storage.get_blob(&blob_id).await.unwrap();
        assert!(blob.is_some());
        assert_eq!(blob.unwrap().payload, b"find me");

        // Non-existent blob
        let missing = storage.get_blob(&BlobId::new()).await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn cleanup_expired_removes_old_blobs() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let device_id = DeviceId::random();

        // Store a blob with 0 TTL (already expired)
        let mut req = make_request(&group_id, &device_id, b"expired");
        req.ttl_secs = 0;
        let expired_id = req.blob_id;
        storage.store_blob(req).await.unwrap();

        // Store a blob with long TTL
        let req = make_request(&group_id, &device_id, b"fresh");
        let fresh_id = req.blob_id;
        storage.store_blob(req).await.unwrap();

        // Wait a moment to ensure expiration
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Cleanup
        let deleted = storage.cleanup_expired().await.unwrap();
        assert_eq!(deleted, 1);

        // Expired blob is gone, fresh blob remains
        assert!(storage.get_blob(&expired_id).await.unwrap().is_none());
        assert!(storage.get_blob(&fresh_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn cleanup_removes_deliveries_for_expired_blobs() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let group_id = GroupId::random();
        let sender = DeviceId::random();
        let receiver = DeviceId::random();

        // Store and deliver an expired blob
        let mut req = make_request(&group_id, &sender, b"expired");
        req.ttl_secs = 0;
        let blob_id = req.blob_id;
        storage.store_blob(req).await.unwrap();
        storage.mark_delivered(&blob_id, &receiver).await.unwrap();

        // Wait and cleanup
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        storage.cleanup_expired().await.unwrap();

        // Blob and delivery are gone
        assert!(storage.get_blob(&blob_id).await.unwrap().is_none());
    }
}
