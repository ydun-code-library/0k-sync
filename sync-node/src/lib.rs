//! # sync-node
//!
//! Node.js/Bun native addon for 0k-Sync via napi-rs.
//!
//! Wraps [`zerok_sync_bridge::SyncHandle`] into a JavaScript class with
//! async methods that return Promises.

#![warn(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use zerok_sync_bridge::{
    error::SyncBridgeError, handle::derive_secret as bridge_derive_secret, SyncHandle,
    SyncHandleConfig,
};

// ============================================================
// FFI types — #[napi(object)] maps to plain JS objects
// ============================================================

/// Configuration for creating a SyncClient.
#[napi(object)]
pub struct JsSyncConfig {
    /// Passphrase for Argon2id key derivation.
    pub passphrase: Option<String>,
    /// Salt for Argon2id derivation (required with passphrase).
    pub salt: Option<Buffer>,
    /// Pre-derived 32-byte secret.
    pub secret_bytes: Option<Buffer>,
    /// Relay addresses (iroh NodeIds). First is primary.
    pub relay_addresses: Vec<String>,
    /// Human-readable device name.
    pub device_name: Option<String>,
    /// Time-to-live for pushed blobs (seconds, 0 = no expiry).
    pub ttl: Option<u32>,
}

/// Result of a push operation.
#[napi(object)]
pub struct JsPushResult {
    /// The blob identifier (UUID string).
    pub blob_id: String,
    /// The assigned cursor position.
    pub cursor: i64,
}

/// A received blob from a pull operation.
#[napi(object)]
pub struct JsSyncBlob {
    /// The blob identifier (UUID string).
    pub blob_id: String,
    /// The decrypted payload bytes.
    pub data: Buffer,
    /// The cursor position.
    pub cursor: i64,
    /// Original timestamp.
    pub timestamp: i64,
}

/// An invite for sharing group access.
#[napi(object)]
pub struct JsSyncInvite {
    /// Invite format version.
    pub version: u32,
    /// Relay addresses.
    pub relay_addresses: Vec<String>,
    /// The group secret bytes (32 bytes).
    pub group_secret: Buffer,
    /// Argon2id salt.
    pub salt: Buffer,
    /// QR payload string (base64-encoded JSON).
    pub qr_payload: String,
    /// Short code (XXXX-XXXX-XXXX-XXXX).
    pub short_code: String,
}

/// Result of deriving a group secret.
#[napi(object)]
pub struct JsDeriveResult {
    /// The 32-byte derived secret.
    pub secret_bytes: Buffer,
    /// The 32-byte group identifier.
    pub group_id: Buffer,
}

// ============================================================
// Internal conversion helpers (testable without napi env)
// ============================================================

fn js_config_to_bridge(config: &JsSyncConfig) -> Result<SyncHandleConfig> {
    let bridge = SyncHandleConfig {
        passphrase: config.passphrase.clone(),
        salt: config.salt.as_ref().map(|b| b.to_vec()),
        secret_bytes: config.secret_bytes.as_ref().map(|b| b.to_vec()),
        relay_addresses: config.relay_addresses.clone(),
        device_name: config.device_name.clone(),
        ttl: config.ttl,
    };
    bridge.validate().map_err(to_napi_error)?;
    Ok(bridge)
}

fn bridge_push_to_js(result: zerok_sync_bridge::PushResult) -> JsPushResult {
    JsPushResult {
        blob_id: result.blob_id,
        cursor: result.cursor as i64,
    }
}

fn bridge_blob_to_js(blob: zerok_sync_bridge::SyncBlob) -> JsSyncBlob {
    JsSyncBlob {
        blob_id: blob.blob_id,
        data: blob.data.into(),
        cursor: blob.cursor as i64,
        timestamp: blob.timestamp as i64,
    }
}

fn bridge_invite_to_js(invite: zerok_sync_bridge::SyncInvite) -> JsSyncInvite {
    JsSyncInvite {
        version: invite.version,
        relay_addresses: invite.relay_addresses,
        group_secret: invite.group_secret.into(),
        salt: invite.salt.into(),
        qr_payload: invite.qr_payload,
        short_code: invite.short_code,
    }
}

fn to_napi_error(err: SyncBridgeError) -> Error {
    Error::from_reason(err.to_string())
}

// ============================================================
// SyncClient — the main napi class
// ============================================================

/// The main sync client for JavaScript/TypeScript.
///
/// All async methods return Promises. Create via `SyncClient.create()`.
#[napi]
pub struct SyncClient {
    handle: SyncHandle,
}

#[napi]
impl SyncClient {
    /// Create a new SyncClient from configuration.
    ///
    /// This binds a real iroh endpoint (async, may take 0-3s).
    #[napi(factory)]
    pub async fn create(config: JsSyncConfig) -> Result<Self> {
        let bridge_config = js_config_to_bridge(&config)?;
        let handle = SyncHandle::create(bridge_config)
            .await
            .map_err(to_napi_error)?;
        Ok(Self { handle })
    }

    /// Check if connected to a relay.
    #[napi]
    pub async fn is_connected(&self) -> Result<bool> {
        Ok(self.handle.is_connected().await)
    }

    /// Get the current cursor position.
    #[napi]
    pub async fn current_cursor(&self) -> Result<i64> {
        Ok(self.handle.current_cursor().await as i64)
    }

    /// Get the address of the active relay (if connected).
    #[napi]
    pub async fn active_relay(&self) -> Result<Option<String>> {
        Ok(self.handle.active_relay().await)
    }

    /// Connect to the relay(s).
    #[napi]
    pub async fn connect(&self) -> Result<()> {
        self.handle.connect().await.map_err(to_napi_error)
    }

    /// Disconnect from the relay.
    #[napi]
    pub async fn disconnect(&self) -> Result<()> {
        self.handle.disconnect().await.map_err(to_napi_error)
    }

    /// Push encrypted data to the sync group.
    #[napi]
    pub async fn push(&self, data: Buffer) -> Result<JsPushResult> {
        let result = self.handle.push(&data).await.map_err(to_napi_error)?;
        Ok(bridge_push_to_js(result))
    }

    /// Pull new blobs from the sync group.
    #[napi]
    pub async fn pull(&self) -> Result<Vec<JsSyncBlob>> {
        let blobs = self.handle.pull().await.map_err(to_napi_error)?;
        Ok(blobs.into_iter().map(bridge_blob_to_js).collect())
    }

    /// Pull blobs after a specific cursor.
    #[napi]
    pub async fn pull_after(&self, cursor: i64) -> Result<Vec<JsSyncBlob>> {
        let blobs = self
            .handle
            .pull_after(cursor as u64)
            .await
            .map_err(to_napi_error)?;
        Ok(blobs.into_iter().map(bridge_blob_to_js).collect())
    }

    /// Create an invite for sharing group access.
    #[napi]
    pub fn create_invite(&self, relay_addresses: Vec<String>) -> Result<JsSyncInvite> {
        let invite = self
            .handle
            .create_invite(&relay_addresses)
            .map_err(to_napi_error)?;
        Ok(bridge_invite_to_js(invite))
    }

    /// Disconnect and release resources.
    #[napi]
    pub async fn shutdown(&self) -> Result<()> {
        self.handle.disconnect().await.map_err(to_napi_error)
    }
}

// ============================================================
// Standalone functions (not tied to a client instance)
// ============================================================

/// Decode an invite from a QR payload string.
#[napi]
pub fn invite_from_qr(payload: String) -> Result<JsSyncInvite> {
    let invite = SyncHandle::invite_from_qr(&payload).map_err(to_napi_error)?;
    Ok(bridge_invite_to_js(invite))
}

/// Derive a group secret from a passphrase and salt.
///
/// Returns the 32-byte secret and 32-byte group ID.
#[napi]
pub fn derive_secret(passphrase: String, salt: Buffer) -> JsDeriveResult {
    let (secret_bytes, group_id) = bridge_derive_secret(&passphrase, &salt);
    JsDeriveResult {
        secret_bytes: secret_bytes.into(),
        group_id: group_id.into(),
    }
}

// ============================================================
// Tests — bridge-level only (no napi Buffer in tests)
//
// napi's Buffer type calls napi FFI in Drop, so test binaries
// can't link against it. We test bridge conversions here;
// napi-specific glue is covered by JS integration tests.
// ============================================================

#[cfg(test)]
mod tests {
    use zerok_sync_bridge::{
        error::SyncBridgeError, handle::derive_secret as bridge_derive_secret, PushResult,
        SyncBlob, SyncHandleConfig, SyncInvite,
    };

    // --- Config validation (bridge types, no napi) ---

    #[test]
    fn config_from_passphrase_validates() {
        let config =
            SyncHandleConfig::from_passphrase("test-pass", b"test-salt-00000!", "relay-node");
        config.validate().unwrap();
        assert_eq!(config.passphrase, Some("test-pass".to_string()));
        assert_eq!(config.relay_addresses, vec!["relay-node"]);
    }

    #[test]
    fn config_from_secret_bytes_validates() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay");
        config.validate().unwrap();
        assert_eq!(config.secret_bytes.as_deref(), Some([0x42; 32].as_slice()));
    }

    #[test]
    fn config_rejects_both_passphrase_and_secret() {
        let config = SyncHandleConfig {
            passphrase: Some("pass".to_string()),
            salt: Some(b"salt".to_vec()),
            secret_bytes: Some(vec![0u8; 32]),
            relay_addresses: vec!["relay".to_string()],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("both"));
    }

    #[test]
    fn config_rejects_empty_relays() {
        let config = SyncHandleConfig {
            passphrase: None,
            salt: None,
            secret_bytes: Some(vec![0u8; 32]),
            relay_addresses: vec![],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    // --- Bridge type conversions (no Buffer) ---

    #[test]
    fn push_result_fields_map_correctly() {
        let result = PushResult {
            blob_id: "abc-123".to_string(),
            cursor: 42,
        };
        assert_eq!(result.blob_id, "abc-123");
        assert_eq!(result.cursor as i64, 42i64);
    }

    #[test]
    fn sync_blob_fields_map_correctly() {
        let blob = SyncBlob {
            blob_id: "def-456".to_string(),
            data: vec![1, 2, 3],
            cursor: 10,
            timestamp: 1705000000,
        };
        assert_eq!(blob.blob_id, "def-456");
        assert_eq!(blob.data, vec![1, 2, 3]);
        assert_eq!(blob.cursor as i64, 10i64);
        assert_eq!(blob.timestamp as i64, 1705000000i64);
    }

    #[test]
    fn sync_invite_fields_map_correctly() {
        let invite = SyncInvite {
            version: 3,
            relay_addresses: vec!["relay-a".to_string(), "relay-b".to_string()],
            group_secret: vec![0x42; 32],
            salt: vec![0x01; 16],
            qr_payload: "encoded-payload".to_string(),
            short_code: "ABCD-EFGH-IJKL-MNOP".to_string(),
        };
        assert_eq!(invite.version, 3);
        assert_eq!(invite.relay_addresses.len(), 2);
        assert_eq!(invite.group_secret.len(), 32);
        assert_eq!(invite.qr_payload, "encoded-payload");
        assert_eq!(invite.short_code, "ABCD-EFGH-IJKL-MNOP");
    }

    // --- Error conversion ---

    #[test]
    fn bridge_error_to_string_is_human_readable() {
        let err = SyncBridgeError::NotConnected;
        assert_eq!(err.to_string(), "not connected");

        let err = SyncBridgeError::InvalidConfig("missing salt".to_string());
        assert!(err.to_string().contains("missing salt"));
    }

    // --- derive_secret ---

    #[test]
    fn derive_secret_is_deterministic() {
        let r1 = bridge_derive_secret("pass", b"salt-00000000000!");
        let r2 = bridge_derive_secret("pass", b"salt-00000000000!");
        assert_eq!(r1.0, r2.0);
        assert_eq!(r1.1, r2.1);
        assert_eq!(r1.0.len(), 32);
        assert!(!r1.1.is_empty());
    }

    #[test]
    fn derive_secret_different_passphrases_differ() {
        let r1 = bridge_derive_secret("pass-a", b"salt-00000000000!");
        let r2 = bridge_derive_secret("pass-b", b"salt-00000000000!");
        assert_ne!(r1.0, r2.0);
    }
}
