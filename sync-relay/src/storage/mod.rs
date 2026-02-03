//! Storage layer for sync-relay.
//!
//! Provides blob storage with cursor-based ordering.

mod sqlite;

pub use sqlite::SqliteStorage;

use crate::error::StorageError;
use async_trait::async_trait;
use sync_types::{BlobId, Cursor, DeviceId, GroupId};

/// A blob stored in the relay with cursor ordering.
#[derive(Debug, Clone)]
pub struct StoredBlob {
    /// Unique identifier for this blob.
    pub blob_id: BlobId,
    /// Group this blob belongs to.
    pub group_id: GroupId,
    /// Relay-assigned cursor for ordering.
    pub cursor: Cursor,
    /// Device that sent this blob.
    pub sender_id: DeviceId,
    /// Encrypted payload (relay cannot decrypt).
    pub payload: Vec<u8>,
    /// Unix timestamp when blob was created.
    pub timestamp: i64,
    /// Unix timestamp when blob expires.
    pub expires_at: i64,
}

/// Request to store a new blob.
#[derive(Debug, Clone)]
pub struct StoreBlobRequest {
    /// Unique identifier for this blob.
    pub blob_id: BlobId,
    /// Group this blob belongs to.
    pub group_id: GroupId,
    /// Device sending this blob.
    pub sender_id: DeviceId,
    /// Encrypted payload.
    pub payload: Vec<u8>,
    /// Unix timestamp when blob was created.
    pub timestamp: i64,
    /// TTL in seconds (added to current time for expires_at).
    pub ttl_secs: u64,
}

/// Trait for blob storage backends.
#[async_trait]
pub trait BlobStorage: Send + Sync {
    /// Store a blob and assign it a cursor.
    ///
    /// Returns the assigned cursor.
    async fn store_blob(&self, req: StoreBlobRequest) -> Result<Cursor, StorageError>;

    /// Get all blobs after the given cursor for a group.
    ///
    /// Returns up to `limit` blobs, ordered by cursor.
    async fn get_blobs_after(
        &self,
        group_id: &GroupId,
        after: Cursor,
        limit: u32,
    ) -> Result<Vec<StoredBlob>, StorageError>;

    /// Get the maximum cursor value for a group.
    ///
    /// Returns Cursor(0) if no blobs exist for the group.
    async fn get_max_cursor(&self, group_id: &GroupId) -> Result<Cursor, StorageError>;

    /// Mark a blob as delivered to a device.
    async fn mark_delivered(
        &self,
        blob_id: &BlobId,
        device_id: &DeviceId,
    ) -> Result<(), StorageError>;

    /// Get count of pending (undelivered) blobs for a device in a group.
    async fn get_pending_count(
        &self,
        group_id: &GroupId,
        device_id: &DeviceId,
    ) -> Result<u32, StorageError>;

    /// Remove expired blobs.
    ///
    /// Returns the number of blobs deleted.
    async fn cleanup_expired(&self) -> Result<u64, StorageError>;

    /// Get total storage used by a group in bytes.
    async fn get_group_storage(&self, group_id: &GroupId) -> Result<u64, StorageError>;

    /// Get a specific blob by ID.
    async fn get_blob(&self, blob_id: &BlobId) -> Result<Option<StoredBlob>, StorageError>;
}
