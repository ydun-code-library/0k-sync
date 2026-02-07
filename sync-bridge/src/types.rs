//! FFI-friendly types for sync-bridge.
//!
//! All types here are flat — no generics, no lifetimes, no trait objects.
//! `String` instead of `&str`, `Vec<u8>` instead of `&[u8]`.

use crate::error::SyncBridgeError;

/// Configuration for creating a [`SyncHandle`](crate::SyncHandle).
///
/// Supports two modes:
/// - **Passphrase mode**: Set `passphrase` + `salt` (Argon2id derivation)
/// - **Secret bytes mode**: Set `secret_bytes` (pre-derived 32-byte key)
///
/// Exactly one mode must be used. `relay_addresses` must be non-empty.
#[derive(Debug, Clone)]
pub struct SyncHandleConfig {
    /// Passphrase for Argon2id key derivation (mode 1).
    pub passphrase: Option<String>,
    /// Salt for Argon2id derivation (required with passphrase).
    pub salt: Option<Vec<u8>>,
    /// Pre-derived 32-byte secret (mode 2).
    pub secret_bytes: Option<Vec<u8>>,
    /// Relay addresses (iroh NodeIds). First is primary.
    pub relay_addresses: Vec<String>,
    /// Human-readable device name.
    pub device_name: Option<String>,
    /// Time-to-live for pushed blobs (seconds, 0 = no expiry).
    pub ttl: Option<u32>,
}

impl SyncHandleConfig {
    /// Create config from a passphrase and salt.
    pub fn from_passphrase(passphrase: &str, salt: &[u8], relay_address: &str) -> Self {
        Self {
            passphrase: Some(passphrase.to_string()),
            salt: Some(salt.to_vec()),
            secret_bytes: None,
            relay_addresses: vec![relay_address.to_string()],
            device_name: None,
            ttl: None,
        }
    }

    /// Create config from pre-derived secret bytes.
    pub fn from_secret_bytes(secret_bytes: &[u8], relay_address: &str) -> Self {
        Self {
            passphrase: None,
            salt: None,
            secret_bytes: Some(secret_bytes.to_vec()),
            relay_addresses: vec![relay_address.to_string()],
            device_name: None,
            ttl: None,
        }
    }

    /// Validate the configuration.
    ///
    /// Returns an error if:
    /// - Both passphrase and secret_bytes are set
    /// - Neither passphrase nor secret_bytes is set
    /// - Passphrase is set without salt
    /// - relay_addresses is empty
    /// - secret_bytes is not 32 bytes
    pub fn validate(&self) -> Result<(), SyncBridgeError> {
        // Must have exactly one secret source
        if self.passphrase.is_some() && self.secret_bytes.is_some() {
            return Err(SyncBridgeError::InvalidConfig(
                "cannot set both passphrase and secret_bytes".to_string(),
            ));
        }

        if self.passphrase.is_none() && self.secret_bytes.is_none() {
            return Err(SyncBridgeError::InvalidConfig(
                "must set either passphrase or secret_bytes".to_string(),
            ));
        }

        // Passphrase requires salt
        if self.passphrase.is_some() && self.salt.is_none() {
            return Err(SyncBridgeError::InvalidConfig(
                "passphrase requires salt".to_string(),
            ));
        }

        // Secret bytes must be 32 bytes
        if let Some(ref bytes) = self.secret_bytes {
            if bytes.len() != 32 {
                return Err(SyncBridgeError::InvalidConfig(format!(
                    "secret_bytes must be 32 bytes, got {}",
                    bytes.len()
                )));
            }
        }

        // Must have at least one relay
        if self.relay_addresses.is_empty() {
            return Err(SyncBridgeError::InvalidConfig(
                "relay_addresses must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Result of a push operation.
#[derive(Debug, Clone)]
pub struct PushResult {
    /// The blob identifier (UUID string).
    pub blob_id: String,
    /// The assigned cursor position.
    pub cursor: u64,
}

/// A received blob from a pull operation.
#[derive(Debug, Clone)]
pub struct SyncBlob {
    /// The blob identifier (UUID string).
    pub blob_id: String,
    /// The decrypted payload bytes.
    pub data: Vec<u8>,
    /// The cursor position.
    pub cursor: u64,
    /// Original timestamp.
    pub timestamp: u64,
}

/// An invite for sharing group access.
#[derive(Debug, Clone)]
pub struct SyncInvite {
    /// Invite format version.
    pub version: u32,
    /// Relay addresses (base64-encoded NodeIds).
    pub relay_addresses: Vec<String>,
    /// The group secret bytes (32 bytes).
    pub group_secret: Vec<u8>,
    /// Argon2id salt.
    pub salt: Vec<u8>,
    /// QR payload string (base64-encoded JSON).
    pub qr_payload: String,
    /// Short code (XXXX-XXXX-XXXX-XXXX).
    pub short_code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_passphrase_constructs_correctly() {
        let config = SyncHandleConfig::from_passphrase(
            "my-passphrase",
            b"test-salt-00000!",
            "relay-node-id",
        );
        assert_eq!(config.passphrase.as_deref(), Some("my-passphrase"));
        assert_eq!(config.salt.as_deref(), Some(b"test-salt-00000!".as_slice()));
        assert!(config.secret_bytes.is_none());
        assert_eq!(config.relay_addresses, vec!["relay-node-id"]);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_from_secret_bytes_constructs_correctly() {
        let secret = [0x42u8; 32];
        let config = SyncHandleConfig::from_secret_bytes(&secret, "relay-node-id");
        assert!(config.passphrase.is_none());
        assert_eq!(config.secret_bytes.as_deref(), Some(secret.as_slice()));
        assert_eq!(config.relay_addresses, vec!["relay-node-id"]);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_both_passphrase_and_secret_bytes() {
        let config = SyncHandleConfig {
            passphrase: Some("pass".to_string()),
            salt: Some(b"salt".to_vec()),
            secret_bytes: Some(vec![0u8; 32]),
            relay_addresses: vec!["relay".to_string()],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SyncBridgeError::InvalidConfig(_)));
        assert!(err.to_string().contains("both"));
    }

    #[test]
    fn validate_rejects_empty_relay_addresses() {
        let config = SyncHandleConfig {
            passphrase: Some("pass".to_string()),
            salt: Some(b"salt".to_vec()),
            secret_bytes: None,
            relay_addresses: vec![],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_rejects_passphrase_without_salt() {
        let config = SyncHandleConfig {
            passphrase: Some("pass".to_string()),
            salt: None,
            secret_bytes: None,
            relay_addresses: vec!["relay".to_string()],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("salt"));
    }

    #[test]
    fn validate_rejects_wrong_length_secret_bytes() {
        let config = SyncHandleConfig::from_secret_bytes(&[0u8; 16], "relay");
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("32 bytes"));
    }

    #[test]
    fn push_result_holds_correct_data() {
        let result = PushResult {
            blob_id: "test-blob-id".to_string(),
            cursor: 42,
        };
        assert_eq!(result.blob_id, "test-blob-id");
        assert_eq!(result.cursor, 42);
    }

    #[test]
    fn sync_blob_holds_correct_data() {
        let blob = SyncBlob {
            blob_id: "test-blob-id".to_string(),
            data: vec![1, 2, 3],
            cursor: 10,
            timestamp: 1705000000,
        };
        assert_eq!(blob.blob_id, "test-blob-id");
        assert_eq!(blob.data, vec![1, 2, 3]);
        assert_eq!(blob.cursor, 10);
        assert_eq!(blob.timestamp, 1705000000);
    }
}
