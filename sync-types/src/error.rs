//! Error types for 0k-Sync.

use thiserror::Error;

/// Errors that can occur in 0k-Sync operations.
#[derive(Debug, Error)]
pub enum SyncError {
    /// MessagePack serialization failed
    #[error("serialization failed: {0}")]
    Serialization(#[source] rmp_serde::encode::Error),

    /// MessagePack deserialization failed
    #[error("deserialization failed: {0}")]
    Deserialization(#[source] rmp_serde::decode::Error),

    /// Invalid message type discriminator
    #[error("invalid message type: {0}")]
    InvalidMessageType(u8),

    /// Invalid protocol version
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u8),

    /// Invalid data format
    #[error("invalid data: {0}")]
    InvalidData(String),

    /// Encryption/decryption failed
    #[error("crypto error: {0}")]
    Crypto(String),

    /// Connection error
    #[error("connection error: {0}")]
    Connection(String),

    /// Timeout
    #[error("operation timed out")]
    Timeout,

    /// Rate limit exceeded
    #[error("rate limit exceeded")]
    RateLimited,

    /// Group not found
    #[error("group not found")]
    GroupNotFound,

    /// Not authenticated
    #[error("not authenticated")]
    NotAuthenticated,

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = SyncError::InvalidMessageType(99);
        assert_eq!(err.to_string(), "invalid message type: 99");
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SyncError>();
    }
}
