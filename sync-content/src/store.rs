//! Content-addressed blob storage.
//!
//! This module provides a trait for storing encrypted content blobs
//! addressed by their BLAKE3 hash, plus a memory-based implementation
//! for testing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::error::ContentError;

/// Trait for content-addressed blob storage.
///
/// All implementations store encrypted ciphertext, addressed by its BLAKE3 hash.
/// This enables verify-on-read: if the hash doesn't match, the content is corrupted.
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Store ciphertext and return its BLAKE3 hash.
    ///
    /// The hash is computed from the ciphertext bytes and serves as the content address.
    async fn put(&self, ciphertext: &[u8]) -> Result<[u8; 32], ContentError>;

    /// Retrieve ciphertext by its BLAKE3 hash.
    ///
    /// Returns `NotFound` if the hash is not in the store.
    async fn get(&self, hash: &[u8; 32]) -> Result<Vec<u8>, ContentError>;

    /// Check if content exists in the store.
    async fn contains(&self, hash: &[u8; 32]) -> bool;

    /// Remove content from the store.
    ///
    /// Returns `Ok(true)` if removed, `Ok(false)` if not found.
    async fn remove(&self, hash: &[u8; 32]) -> Result<bool, ContentError>;
}

/// In-memory blob store for testing.
///
/// Stores blobs in a thread-safe HashMap. Not persistent - all data
/// is lost when the store is dropped.
#[derive(Default, Clone)]
pub struct MemoryStore {
    blobs: Arc<Mutex<HashMap<[u8; 32], Vec<u8>>>>,
}

impl MemoryStore {
    /// Create a new empty memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of blobs currently stored.
    pub fn len(&self) -> usize {
        self.blobs.lock().unwrap().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.blobs.lock().unwrap().is_empty()
    }

    /// Clear all blobs from the store.
    pub fn clear(&self) {
        self.blobs.lock().unwrap().clear();
    }
}

#[async_trait]
impl BlobStore for MemoryStore {
    async fn put(&self, ciphertext: &[u8]) -> Result<[u8; 32], ContentError> {
        let hash = *blake3::hash(ciphertext).as_bytes();
        self.blobs.lock().unwrap().insert(hash, ciphertext.to_vec());
        Ok(hash)
    }

    async fn get(&self, hash: &[u8; 32]) -> Result<Vec<u8>, ContentError> {
        self.blobs
            .lock()
            .unwrap()
            .get(hash)
            .cloned()
            .ok_or_else(|| ContentError::NotFound {
                hash: hex::encode(hash),
            })
    }

    async fn contains(&self, hash: &[u8; 32]) -> bool {
        self.blobs.lock().unwrap().contains_key(hash)
    }

    async fn remove(&self, hash: &[u8; 32]) -> Result<bool, ContentError> {
        Ok(self.blobs.lock().unwrap().remove(hash).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_store_put_get() {
        let store = MemoryStore::new();
        let ciphertext = b"encrypted content here";

        let hash = store.put(ciphertext).await.unwrap();
        let retrieved = store.get(&hash).await.unwrap();

        assert_eq!(retrieved, ciphertext);
    }

    #[tokio::test]
    async fn memory_store_not_found() {
        let store = MemoryStore::new();
        let nonexistent_hash = [0xFF; 32];

        let result = store.get(&nonexistent_hash).await;

        assert!(matches!(result, Err(ContentError::NotFound { .. })));
    }

    #[tokio::test]
    async fn memory_store_hash_is_blake3() {
        let store = MemoryStore::new();
        let ciphertext = b"test content for hashing";

        let hash = store.put(ciphertext).await.unwrap();
        let expected_hash = *blake3::hash(ciphertext).as_bytes();

        assert_eq!(hash, expected_hash);
    }

    #[tokio::test]
    async fn memory_store_contains() {
        let store = MemoryStore::new();
        let ciphertext = b"content to check";

        let hash = store.put(ciphertext).await.unwrap();

        assert!(store.contains(&hash).await);
        assert!(!store.contains(&[0x00; 32]).await);
    }

    #[tokio::test]
    async fn memory_store_remove() {
        let store = MemoryStore::new();
        let ciphertext = b"content to remove";

        let hash = store.put(ciphertext).await.unwrap();
        assert!(store.contains(&hash).await);

        let removed = store.remove(&hash).await.unwrap();
        assert!(removed);
        assert!(!store.contains(&hash).await);

        // Second remove returns false
        let removed_again = store.remove(&hash).await.unwrap();
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn memory_store_len_and_clear() {
        let store = MemoryStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.put(b"blob 1").await.unwrap();
        store.put(b"blob 2").await.unwrap();
        store.put(b"blob 3").await.unwrap();

        assert_eq!(store.len(), 3);
        assert!(!store.is_empty());

        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn memory_store_overwrite_same_hash() {
        let store = MemoryStore::new();
        let ciphertext = b"same content";

        // Put same content twice
        let hash1 = store.put(ciphertext).await.unwrap();
        let hash2 = store.put(ciphertext).await.unwrap();

        // Same hash, only stored once
        assert_eq!(hash1, hash2);
        assert_eq!(store.len(), 1);
    }
}
