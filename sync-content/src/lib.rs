//! # sync-content
//!
//! Large content transfer for 0k-Sync using encrypt-then-hash.
//!
//! This crate handles content-addressed storage and transfer of large files
//! (photos, documents, audio). Small sync messages (<64KB) go through the
//! relay directly; large content is encrypted, hashed, and stored in a
//! content-addressed blob store.
//!
//! ## Encrypt-Then-Hash Pattern
//!
//! ```text
//! Plaintext → XChaCha20-Poly1305 → Ciphertext → BLAKE3 → Hash (content address)
//!                 ↑                                           ↓
//!          Content Key                               Blob Store
//!          (HKDF from GroupSecret + blob_id)
//! ```
//!
//! 1. Derive a unique content key from GroupSecret + blob_id using HKDF
//! 2. Encrypt plaintext with XChaCha20-Poly1305 (random 192-bit nonce)
//! 3. Hash the ciphertext with BLAKE3 to get the content address
//! 4. Store ciphertext in blob store using hash as key
//! 5. Return ContentRef with hash, nonce, and sizes
//!
//! ## Example
//!
//! ```rust,ignore
//! use zerok_sync_content::{ContentTransfer, MemoryStore, ContentError};
//!
//! # async fn example() -> Result<(), ContentError> {
//! // Create a content transfer handler with a memory store
//! let store = MemoryStore::new();
//! let group_secret = [0u8; 32]; // In practice, from GroupSecret
//! let transfer = ContentTransfer::new(store, group_secret);
//!
//! // Add content (encrypt and store)
//! let blob_id = b"unique-blob-identifier";
//! let plaintext = b"Large file content here...";
//! let content_ref = transfer.add(blob_id, plaintext).await?;
//!
//! // Retrieve content (fetch and decrypt)
//! let retrieved = transfer.get(blob_id, &content_ref).await?;
//! assert_eq!(retrieved, plaintext);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

mod encrypt;
mod error;
mod store;

pub use encrypt::{
    decrypt_content, derive_content_key, encrypt_content, EncryptedContent, CONTENT_KEY_SIZE,
    HASH_SIZE, NONCE_SIZE,
};
pub use error::ContentError;
pub use store::{BlobStore, MemoryStore};

use sync_types::ContentRef;

/// Handler for content transfer operations.
///
/// Manages the encrypt-then-hash pipeline and blob storage for large content.
/// Each instance is bound to a specific sync group via its `group_secret`.
pub struct ContentTransfer<S: BlobStore> {
    store: S,
    group_secret: [u8; 32],
}

impl<S: BlobStore> ContentTransfer<S> {
    /// Create a new content transfer handler.
    ///
    /// # Arguments
    ///
    /// * `store` - Blob store for encrypted content
    /// * `group_secret` - 32-byte group secret (from GroupSecret::as_bytes())
    pub fn new(store: S, group_secret: [u8; 32]) -> Self {
        Self {
            store,
            group_secret,
        }
    }

    /// Add plaintext content to the store.
    ///
    /// This encrypts the content, stores it, and returns a `ContentRef`
    /// that can be shared with other devices in the sync group.
    ///
    /// # Arguments
    ///
    /// * `blob_id` - Unique identifier for this content (used in key derivation)
    /// * `plaintext` - The content to encrypt and store
    ///
    /// # Returns
    ///
    /// A `ContentRef` containing the hash, nonce, and sizes needed to retrieve
    /// and decrypt the content.
    pub async fn add(&self, blob_id: &[u8], plaintext: &[u8]) -> Result<ContentRef, ContentError> {
        // Derive content-specific key
        let content_key = derive_content_key(&self.group_secret, blob_id);

        // Encrypt and hash
        let (ciphertext, nonce, hash) = encrypt_content(&content_key, plaintext)?;

        // Store ciphertext
        let stored_hash = self.store.put(&ciphertext).await?;

        // Verify hash matches (should always be true)
        if hash != stored_hash {
            return Err(ContentError::HashMismatch {
                expected: hex::encode(hash),
                actual: hex::encode(stored_hash),
            });
        }

        Ok(ContentRef {
            content_hash: hash,
            encryption_nonce: nonce,
            content_size: plaintext.len() as u64,
            encrypted_size: ciphertext.len() as u64,
        })
    }

    /// Retrieve and decrypt content using a ContentRef.
    ///
    /// # Arguments
    ///
    /// * `blob_id` - The same blob_id used when adding the content
    /// * `content_ref` - The ContentRef from a previous `add()` call
    ///
    /// # Returns
    ///
    /// The decrypted plaintext content.
    ///
    /// # Errors
    ///
    /// - `NotFound` if the content is not in the store
    /// - `HashMismatch` if the stored content has been corrupted
    /// - `DecryptionFailed` if decryption fails (wrong key or corrupted)
    pub async fn get(
        &self,
        blob_id: &[u8],
        content_ref: &ContentRef,
    ) -> Result<Vec<u8>, ContentError> {
        // Get ciphertext from store
        let ciphertext = self.store.get(&content_ref.content_hash).await?;

        // Verify hash (detect corruption)
        let actual_hash = *blake3::hash(&ciphertext).as_bytes();
        if actual_hash != content_ref.content_hash {
            return Err(ContentError::HashMismatch {
                expected: hex::encode(content_ref.content_hash),
                actual: hex::encode(actual_hash),
            });
        }

        // Derive content key and decrypt
        let content_key = derive_content_key(&self.group_secret, blob_id);
        decrypt_content(&content_key, &content_ref.encryption_nonce, &ciphertext)
    }

    /// Check if content exists in the store.
    pub async fn contains(&self, content_ref: &ContentRef) -> bool {
        self.store.contains(&content_ref.content_hash).await
    }

    /// Remove content from the store.
    ///
    /// Returns `Ok(true)` if removed, `Ok(false)` if not found.
    pub async fn remove(&self, content_ref: &ContentRef) -> Result<bool, ContentError> {
        self.store.remove(&content_ref.content_hash).await
    }

    /// Get a reference to the underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn content_transfer_roundtrip() {
        let store = MemoryStore::new();
        let group_secret = [0xAB; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let blob_id = b"test-blob-001";
        let plaintext = b"This is the content to transfer";

        // Add content
        let content_ref = transfer.add(blob_id, plaintext).await.unwrap();

        // Verify content_ref
        assert_eq!(content_ref.content_size, plaintext.len() as u64);
        assert_eq!(content_ref.encrypted_size, plaintext.len() as u64 + 16); // + auth tag

        // Retrieve content
        let retrieved = transfer.get(blob_id, &content_ref).await.unwrap();
        assert_eq!(retrieved, plaintext);
    }

    #[tokio::test]
    async fn content_transfer_wrong_blob_id() {
        let store = MemoryStore::new();
        let group_secret = [0xCD; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let blob_id = b"original-blob-id";
        let wrong_blob_id = b"different-blob-id";
        let plaintext = b"Secret content";

        // Add with original blob_id
        let content_ref = transfer.add(blob_id, plaintext).await.unwrap();

        // Try to retrieve with wrong blob_id (different key derivation)
        let result = transfer.get(wrong_blob_id, &content_ref).await;
        assert!(matches!(result, Err(ContentError::DecryptionFailed)));
    }

    #[tokio::test]
    async fn content_transfer_large_content_1mb() {
        let store = MemoryStore::new();
        let group_secret = [0xEF; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let blob_id = b"large-blob";
        // 1MB of data
        let plaintext: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        let content_ref = transfer.add(blob_id, &plaintext).await.unwrap();
        let retrieved = transfer.get(blob_id, &content_ref).await.unwrap();

        assert_eq!(retrieved, plaintext);
    }

    #[tokio::test]
    async fn content_transfer_empty_content() {
        let store = MemoryStore::new();
        let group_secret = [0x11; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let blob_id = b"empty-blob";
        let plaintext = b"";

        let content_ref = transfer.add(blob_id, plaintext).await.unwrap();
        assert_eq!(content_ref.content_size, 0);
        assert_eq!(content_ref.encrypted_size, 16); // Just auth tag

        let retrieved = transfer.get(blob_id, &content_ref).await.unwrap();
        assert!(retrieved.is_empty());
    }

    #[tokio::test]
    async fn content_transfer_contains_and_remove() {
        let store = MemoryStore::new();
        let group_secret = [0x22; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let blob_id = b"removable-blob";
        let plaintext = b"Content to remove";

        let content_ref = transfer.add(blob_id, plaintext).await.unwrap();

        assert!(transfer.contains(&content_ref).await);

        let removed = transfer.remove(&content_ref).await.unwrap();
        assert!(removed);

        assert!(!transfer.contains(&content_ref).await);
    }

    #[tokio::test]
    async fn content_transfer_multiple_blobs() {
        let store = MemoryStore::new();
        let group_secret = [0x33; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let contents = vec![
            (b"blob-1".as_slice(), b"Content one".as_slice()),
            (b"blob-2".as_slice(), b"Content two".as_slice()),
            (b"blob-3".as_slice(), b"Content three".as_slice()),
        ];

        let mut refs = Vec::new();
        for (blob_id, plaintext) in &contents {
            let content_ref = transfer.add(blob_id, plaintext).await.unwrap();
            refs.push(content_ref);
        }

        assert_eq!(transfer.store().len(), 3);

        // Retrieve all in reverse order
        for (i, (blob_id, plaintext)) in contents.iter().enumerate().rev() {
            let retrieved = transfer.get(blob_id, &refs[i]).await.unwrap();
            assert_eq!(retrieved, *plaintext);
        }
    }

    #[tokio::test]
    async fn content_ref_hash_matches_blake3() {
        let store = MemoryStore::new();
        let group_secret = [0x44; 32];
        let transfer = ContentTransfer::new(store, group_secret);

        let blob_id = b"hash-test-blob";
        let plaintext = b"Content for hash verification";

        let content_ref = transfer.add(blob_id, plaintext).await.unwrap();

        // We can't verify exact ciphertext due to random nonce, but we can verify
        // the hash in content_ref matches what's stored
        let ciphertext = transfer.store().get(&content_ref.content_hash).await.unwrap();
        let expected_hash = *blake3::hash(&ciphertext).as_bytes();

        assert_eq!(content_ref.content_hash, expected_hash);
    }
}
