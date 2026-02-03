//! Error types for sync-content.

use thiserror::Error;

/// Errors that can occur during content operations.
#[derive(Error, Debug)]
pub enum ContentError {
    /// Encryption operation failed.
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption failed (authentication error).
    /// No details provided to prevent timing attacks.
    #[error("decryption failed")]
    DecryptionFailed,

    /// Content not found in store.
    #[error("content not found: {hash}")]
    NotFound {
        /// Hex-encoded hash of the missing content.
        hash: String,
    },

    /// Store operation failed.
    #[error("store error: {0}")]
    StoreError(String),

    /// Hash verification failed.
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        /// Expected hash (hex-encoded).
        expected: String,
        /// Actual hash (hex-encoded).
        actual: String,
    },
}
