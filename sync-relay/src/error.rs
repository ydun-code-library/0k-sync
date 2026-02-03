//! Error types for sync-relay.

use std::path::PathBuf;

/// Main error type for sync-relay operations.
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// Protocol error.
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Rate limit exceeded.
    #[error("rate limit exceeded: {reason}")]
    RateLimited {
        /// Reason for rate limiting.
        reason: String,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Storage layer errors.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Database error.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Blob not found.
    #[error("blob not found: {blob_id}")]
    NotFound {
        /// The blob ID that was not found.
        blob_id: String,
    },

    /// Group storage quota exceeded.
    #[error("group storage quota exceeded: {group_id} (limit: {limit} bytes)")]
    QuotaExceeded {
        /// The group ID.
        group_id: String,
        /// The storage limit in bytes.
        limit: usize,
    },

    /// Blob too large.
    #[error("blob too large: {size} bytes (limit: {limit} bytes)")]
    BlobTooLarge {
        /// Actual size of the blob.
        size: usize,
        /// Maximum allowed size.
        limit: usize,
    },

    /// Migration error.
    #[error("migration error: {0}")]
    Migration(String),

    /// Database path error.
    #[error("invalid database path: {path}")]
    InvalidPath {
        /// The invalid path.
        path: PathBuf,
    },
}

/// Protocol layer errors.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// Invalid message format.
    #[error("invalid message format: {reason}")]
    InvalidMessage {
        /// Reason the message is invalid.
        reason: String,
    },

    /// Message deserialization failed.
    #[error("message deserialization failed: {0}")]
    Deserialization(#[from] rmp_serde::decode::Error),

    /// Message serialization failed.
    #[error("message serialization failed: {0}")]
    Serialization(#[from] rmp_serde::encode::Error),

    /// Unexpected message type.
    #[error("unexpected message type: expected {expected}, got {actual}")]
    UnexpectedMessage {
        /// Expected message type.
        expected: String,
        /// Actual message type received.
        actual: String,
    },

    /// Session not authenticated.
    #[error("session not authenticated: HELLO required first")]
    NotAuthenticated,

    /// Connection error.
    #[error("connection error: {0}")]
    Connection(String),

    /// Stream error.
    #[error("stream error: {0}")]
    Stream(String),

    /// Protocol version mismatch.
    #[error("protocol version mismatch: client={client}, server={server}")]
    VersionMismatch {
        /// Client protocol version.
        client: u32,
        /// Server protocol version.
        server: u32,
    },
}

/// Result type alias for relay operations.
pub type Result<T> = std::result::Result<T, RelayError>;

/// Result type alias for storage operations.
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Result type alias for protocol operations.
pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;
