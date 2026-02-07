//! Error types for sync-bridge.
//!
//! All lower-layer errors flatten to human-readable strings.
//! FFI consumers get string messages, not Rust enum internals.

use thiserror::Error;
use zerok_sync_client::ClientError;
use zerok_sync_core::PairingError;

/// Errors from sync-bridge operations.
#[derive(Debug, Error)]
pub enum SyncBridgeError {
    /// Invalid configuration.
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// Not connected to any relay.
    #[error("not connected")]
    NotConnected,

    /// Connection to relay failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// All configured relays failed.
    #[error("all relays failed: {0}")]
    AllRelaysFailed(String),

    /// Cryptographic operation failed.
    #[error("crypto error: {0}")]
    CryptoError(String),

    /// Transport-level error.
    #[error("transport error: {0}")]
    TransportError(String),

    /// Protocol-level error (serialization, unexpected message, etc).
    #[error("protocol error: {0}")]
    ProtocolError(String),
}

impl From<ClientError> for SyncBridgeError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::NotConnected => SyncBridgeError::NotConnected,
            ClientError::ConnectionFailed(msg) => SyncBridgeError::ConnectionFailed(msg),
            ClientError::AllRelaysFailed(msg) => SyncBridgeError::AllRelaysFailed(msg),
            ClientError::Crypto(e) => SyncBridgeError::CryptoError(e.to_string()),
            ClientError::Transport(e) => SyncBridgeError::TransportError(e.to_string()),
            ClientError::Protocol(msg) => SyncBridgeError::ProtocolError(msg),
            ClientError::Serialization(msg) => SyncBridgeError::ProtocolError(msg),
        }
    }
}

impl From<PairingError> for SyncBridgeError {
    fn from(err: PairingError) -> Self {
        SyncBridgeError::InvalidConfig(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerok_sync_client::CryptoError;
    use zerok_sync_client::TransportError;

    #[test]
    fn client_not_connected_maps_correctly() {
        let err: SyncBridgeError = ClientError::NotConnected.into();
        assert!(matches!(err, SyncBridgeError::NotConnected));
        assert_eq!(err.to_string(), "not connected");
    }

    #[test]
    fn client_connection_failed_maps_correctly() {
        let err: SyncBridgeError = ClientError::ConnectionFailed("timeout".to_string()).into();
        assert!(matches!(err, SyncBridgeError::ConnectionFailed(_)));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn client_all_relays_failed_maps_correctly() {
        let err: SyncBridgeError =
            ClientError::AllRelaysFailed("relay-a: down; relay-b: down".to_string()).into();
        assert!(matches!(err, SyncBridgeError::AllRelaysFailed(_)));
        assert!(err.to_string().contains("relay-a"));
    }

    #[test]
    fn client_crypto_error_maps_correctly() {
        let err: SyncBridgeError = ClientError::Crypto(CryptoError::DecryptionFailed).into();
        assert!(matches!(err, SyncBridgeError::CryptoError(_)));
        assert!(err.to_string().contains("crypto"));
    }

    #[test]
    fn client_transport_error_maps_correctly() {
        let err: SyncBridgeError = ClientError::Transport(TransportError::Timeout).into();
        assert!(matches!(err, SyncBridgeError::TransportError(_)));
        assert!(err.to_string().contains("transport"));
    }

    #[test]
    fn client_protocol_and_serialization_map_to_protocol_error() {
        let err1: SyncBridgeError = ClientError::Protocol("unexpected message".to_string()).into();
        assert!(matches!(err1, SyncBridgeError::ProtocolError(_)));

        let err2: SyncBridgeError = ClientError::Serialization("bad msgpack".to_string()).into();
        assert!(matches!(err2, SyncBridgeError::ProtocolError(_)));
    }

    #[test]
    fn pairing_error_maps_to_invalid_config() {
        let err: SyncBridgeError = PairingError::Expired.into();
        assert!(matches!(err, SyncBridgeError::InvalidConfig(_)));
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn display_is_human_readable() {
        let err = SyncBridgeError::InvalidConfig("missing salt".to_string());
        assert_eq!(err.to_string(), "invalid config: missing salt");

        let err = SyncBridgeError::NotConnected;
        assert_eq!(err.to_string(), "not connected");

        let err = SyncBridgeError::CryptoError("decryption failed".to_string());
        assert_eq!(err.to_string(), "crypto error: decryption failed");
    }
}
