//! SyncHandle — concrete wrapper around `SyncClient<IrohTransport>`.
//!
//! Monomorphizes the generic client into an FFI-friendly handle.

use crate::error::SyncBridgeError;
use crate::types::{PushResult, SyncBlob, SyncHandleConfig, SyncInvite};
use zerok_sync_client::{IrohTransport, SyncClient, SyncConfig};
use zerok_sync_core::{GroupSecret, Invite, RelayNodeId};
use zerok_sync_types::GroupId;

/// Concrete sync handle for FFI consumers.
///
/// Wraps `SyncClient<IrohTransport>` with FFI-friendly methods.
/// All methods that cross FFI boundaries use owned types (`String`, `Vec<u8>`).
pub struct SyncHandle {
    client: SyncClient<IrohTransport>,
    config: SyncHandleConfig,
}

impl SyncHandle {
    /// Create a new SyncHandle from configuration.
    ///
    /// This binds a real iroh endpoint (async, may take 0-3s).
    pub async fn create(config: SyncHandleConfig) -> Result<Self, SyncBridgeError> {
        config.validate()?;
        let sync_config = to_sync_config(&config)?;
        let transport = IrohTransport::new()
            .await
            .map_err(|e| SyncBridgeError::TransportError(e.to_string()))?;
        let client = SyncClient::new(sync_config, transport);
        Ok(Self { client, config })
    }

    /// Check if connected to a relay.
    pub async fn is_connected(&self) -> bool {
        self.client.is_connected().await
    }

    /// Get the current cursor position.
    pub async fn current_cursor(&self) -> u64 {
        self.client.current_cursor().await.value()
    }

    /// Get the address of the active relay (if connected).
    pub async fn active_relay(&self) -> Option<String> {
        self.client.active_relay().await
    }

    /// Connect to the relay(s).
    pub async fn connect(&self) -> Result<(), SyncBridgeError> {
        self.client.connect().await?;
        Ok(())
    }

    /// Disconnect from the relay.
    pub async fn disconnect(&self) -> Result<(), SyncBridgeError> {
        self.client.disconnect().await?;
        Ok(())
    }

    /// Push encrypted data to the sync group.
    pub async fn push(&self, data: &[u8]) -> Result<PushResult, SyncBridgeError> {
        let (blob_id, cursor) = self.client.push(data).await?;
        Ok(PushResult {
            blob_id: blob_id.to_string(),
            cursor: cursor.value(),
        })
    }

    /// Pull new blobs from the sync group.
    pub async fn pull(&self) -> Result<Vec<SyncBlob>, SyncBridgeError> {
        let blobs = self.client.pull().await?;
        Ok(blobs.into_iter().map(received_to_sync_blob).collect())
    }

    /// Pull blobs after a specific cursor.
    pub async fn pull_after(&self, cursor: u64) -> Result<Vec<SyncBlob>, SyncBridgeError> {
        let blobs = self
            .client
            .pull_after(Some(zerok_sync_types::Cursor::new(cursor)))
            .await?;
        Ok(blobs.into_iter().map(received_to_sync_blob).collect())
    }

    /// Create an invite for sharing group access.
    pub fn create_invite(&self, relay_addresses: &[String]) -> Result<SyncInvite, SyncBridgeError> {
        if relay_addresses.is_empty() {
            return Err(SyncBridgeError::InvalidConfig(
                "relay_addresses must not be empty".to_string(),
            ));
        }

        // Get the group secret from config to build the invite
        let (core_secret, salt) = config_to_core_secret(&self.config)?;
        let group_id = core_secret.derive_group_id();

        // Parse relay addresses into RelayNodeId placeholders
        // For real usage, these would be parsed from the iroh NodeId format.
        // In the bridge layer, we store them as the raw address strings
        // and the invite carries the NodeId bytes.
        let relay_node_ids: Vec<RelayNodeId> = relay_addresses
            .iter()
            .map(|addr| {
                // Hash the address string to get 32 bytes for the RelayNodeId
                // This is a bridge-level convenience — real NodeIds come from iroh
                let hash = blake3::hash(addr.as_bytes());
                RelayNodeId::from_bytes(*hash.as_bytes())
            })
            .collect();

        let invite = Invite::create_multi_relay(relay_node_ids, group_id, core_secret, salt);

        Ok(invite_to_sync_invite(&invite, relay_addresses))
    }

    /// Encode an invite as a QR payload string.
    pub fn invite_to_qr(invite: &SyncInvite) -> String {
        invite.qr_payload.clone()
    }

    /// Decode an invite from a QR payload string.
    pub fn invite_from_qr(payload: &str) -> Result<SyncInvite, SyncBridgeError> {
        let invite = Invite::from_qr_payload(payload)?;
        let relay_addrs: Vec<String> = invite
            .relay_node_ids
            .iter()
            .map(|id| id.to_string())
            .collect();
        Ok(invite_to_sync_invite(&invite, &relay_addrs))
    }

    /// Get the short code from an invite.
    pub fn invite_to_short_code(invite: &SyncInvite) -> String {
        invite.short_code.clone()
    }

    /// Shut down the handle, closing the transport.
    pub async fn shutdown(self) -> Result<(), SyncBridgeError> {
        self.client.disconnect().await?;
        Ok(())
    }
}

/// Derive a group secret from a passphrase and salt.
///
/// Returns `(secret_bytes, group_id_bytes)`.
/// This is a standalone function for FFI consumers who need to pre-derive secrets.
pub fn derive_secret(passphrase: &str, salt: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let client_secret = zerok_sync_client::GroupSecret::from_passphrase_with_salt(passphrase, salt);
    let group_id = GroupId::from_secret(client_secret.as_bytes());
    (
        client_secret.as_bytes().to_vec(),
        group_id.as_bytes().to_vec(),
    )
}

// --- Internal conversion helpers ---

/// Convert `SyncHandleConfig` to `SyncConfig`.
fn to_sync_config(config: &SyncHandleConfig) -> Result<SyncConfig, SyncBridgeError> {
    let primary_relay = config.relay_addresses.first().ok_or_else(|| {
        SyncBridgeError::InvalidConfig("relay_addresses must not be empty".to_string())
    })?;

    let mut sync_config = if let Some(ref passphrase) = config.passphrase {
        let salt = config.salt.as_ref().ok_or_else(|| {
            SyncBridgeError::InvalidConfig("passphrase requires salt".to_string())
        })?;
        SyncConfig::new_with_salt(passphrase, salt, primary_relay)
    } else if let Some(ref secret_bytes) = config.secret_bytes {
        let bytes: [u8; 32] = secret_bytes.as_slice().try_into().map_err(|_| {
            SyncBridgeError::InvalidConfig(format!(
                "secret_bytes must be 32 bytes, got {}",
                secret_bytes.len()
            ))
        })?;
        SyncConfig::from_secret_bytes(&bytes, primary_relay)
    } else {
        return Err(SyncBridgeError::InvalidConfig(
            "must set either passphrase or secret_bytes".to_string(),
        ));
    };

    // Add remaining relay addresses if multi-relay
    if config.relay_addresses.len() > 1 {
        let all_addrs: Vec<&str> = config.relay_addresses.iter().map(|s| s.as_str()).collect();
        sync_config = sync_config.with_relay_addresses(&all_addrs);
    }

    if let Some(ref name) = config.device_name {
        sync_config = sync_config.with_device_name(name);
    }

    if let Some(ttl) = config.ttl {
        sync_config = sync_config.with_ttl(ttl);
    }

    Ok(sync_config)
}

/// Extract core GroupSecret + salt from config for invite creation.
fn config_to_core_secret(
    config: &SyncHandleConfig,
) -> Result<(GroupSecret, Vec<u8>), SyncBridgeError> {
    if let Some(ref passphrase) = config.passphrase {
        let salt = config.salt.as_ref().ok_or_else(|| {
            SyncBridgeError::InvalidConfig("passphrase requires salt".to_string())
        })?;
        // Derive the client-layer secret, then map to core GroupSecret
        let client_secret =
            zerok_sync_client::GroupSecret::from_passphrase_with_salt(passphrase, salt);
        let core_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
        Ok((core_secret, salt.clone()))
    } else if let Some(ref secret_bytes) = config.secret_bytes {
        let bytes: [u8; 32] = secret_bytes.as_slice().try_into().map_err(|_| {
            SyncBridgeError::InvalidConfig("secret_bytes must be 32 bytes".to_string())
        })?;
        let core_secret = GroupSecret::from_bytes(bytes);
        Ok((core_secret, vec![]))
    } else {
        Err(SyncBridgeError::InvalidConfig(
            "must set either passphrase or secret_bytes".to_string(),
        ))
    }
}

/// Convert a `ReceivedBlob` to an FFI-friendly `SyncBlob`.
fn received_to_sync_blob(blob: zerok_sync_client::ReceivedBlob) -> SyncBlob {
    SyncBlob {
        blob_id: blob.blob_id.to_string(),
        data: blob.payload,
        cursor: blob.cursor.value(),
        timestamp: blob.timestamp,
    }
}

/// Convert a core `Invite` to an FFI-friendly `SyncInvite`.
fn invite_to_sync_invite(invite: &Invite, relay_addresses: &[String]) -> SyncInvite {
    SyncInvite {
        version: invite.version,
        relay_addresses: relay_addresses.to_vec(),
        group_secret: invite.group_secret.as_bytes().to_vec(),
        salt: invite.salt.clone(),
        qr_payload: invite.to_qr_payload(),
        short_code: invite.to_short_code(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config conversion tests ---

    #[test]
    fn config_with_passphrase_converts_to_sync_config() {
        let config =
            SyncHandleConfig::from_passphrase("test-pass", b"test-salt-00000!", "relay-node");
        let sync_config = to_sync_config(&config).unwrap();
        assert_eq!(sync_config.primary_relay(), Some("relay-node"));
        assert_eq!(sync_config.device_name, "0k-sync device");
    }

    #[test]
    fn config_with_secret_bytes_converts_to_sync_config() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let sync_config = to_sync_config(&config).unwrap();
        assert_eq!(sync_config.primary_relay(), Some("relay-node"));
    }

    #[test]
    fn config_passphrase_without_salt_fails() {
        let config = SyncHandleConfig {
            passphrase: Some("pass".to_string()),
            salt: None,
            secret_bytes: None,
            relay_addresses: vec!["relay".to_string()],
            device_name: None,
            ttl: None,
        };
        let err = to_sync_config(&config).unwrap_err();
        assert!(err.to_string().contains("salt"));
    }

    #[test]
    fn config_wrong_length_secret_bytes_fails() {
        let config = SyncHandleConfig::from_secret_bytes(&[0u8; 16], "relay");
        let err = to_sync_config(&config).unwrap_err();
        assert!(err.to_string().contains("32 bytes"));
    }

    #[test]
    fn config_with_device_name_and_ttl() {
        let mut config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay");
        config.device_name = Some("My Device".to_string());
        config.ttl = Some(3600);
        let sync_config = to_sync_config(&config).unwrap();
        assert_eq!(sync_config.device_name, "My Device");
        assert_eq!(sync_config.default_ttl, 3600);
    }

    #[test]
    fn config_with_multi_relay() {
        let mut config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-a");
        config.relay_addresses = vec![
            "relay-a".to_string(),
            "relay-b".to_string(),
            "relay-c".to_string(),
        ];
        let sync_config = to_sync_config(&config).unwrap();
        assert_eq!(sync_config.relay_addresses.len(), 3);
        assert_eq!(sync_config.relay_addresses[0], "relay-a");
        assert_eq!(sync_config.relay_addresses[1], "relay-b");
        assert_eq!(sync_config.relay_addresses[2], "relay-c");
    }

    // --- SyncHandle creation (integration-ish — binds real iroh endpoint) ---

    #[tokio::test]
    async fn sync_handle_create_succeeds() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        assert!(!handle.is_connected().await);
    }

    #[tokio::test]
    async fn sync_handle_initial_cursor_is_zero() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        assert_eq!(handle.current_cursor().await, 0);
    }

    #[tokio::test]
    async fn sync_handle_active_relay_is_none_when_disconnected() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        assert!(handle.active_relay().await.is_none());
    }

    // --- Push/Pull on disconnected handle ---

    #[tokio::test]
    async fn push_on_disconnected_returns_not_connected() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        let err = handle.push(b"data").await.unwrap_err();
        assert!(matches!(err, SyncBridgeError::NotConnected));
    }

    #[tokio::test]
    async fn pull_on_disconnected_returns_not_connected() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        let err = handle.pull().await.unwrap_err();
        assert!(matches!(err, SyncBridgeError::NotConnected));
    }

    // --- Invite operations ---

    #[tokio::test]
    async fn create_invite_returns_valid_structure() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();

        let invite = handle.create_invite(&["relay-node".to_string()]).unwrap();

        assert_eq!(invite.version, 3);
        assert_eq!(invite.relay_addresses.len(), 1);
        assert_eq!(invite.group_secret.len(), 32);
        assert!(!invite.qr_payload.is_empty());
        assert!(!invite.short_code.is_empty());
    }

    #[test]
    fn qr_roundtrip_preserves_data() {
        // Create an invite using core directly to test bridge conversion
        let secret = GroupSecret::random();
        let group_id = secret.derive_group_id();
        let relay = RelayNodeId::from_bytes([0xAB; 32]);
        let salt = b"test-salt-00000!".to_vec();

        let invite =
            Invite::create_multi_relay(vec![relay], group_id, secret.clone(), salt.clone());

        let qr = invite.to_qr_payload();
        let decoded = SyncHandle::invite_from_qr(&qr).unwrap();

        assert_eq!(decoded.version, 3);
        assert_eq!(decoded.group_secret, secret.as_bytes().to_vec());
        assert_eq!(decoded.salt, salt);
    }

    #[test]
    fn short_code_format_is_correct() {
        let secret = GroupSecret::random();
        let group_id = secret.derive_group_id();
        let relay = RelayNodeId::from_bytes([0xAB; 32]);

        let invite = Invite::create(relay, group_id, secret, b"test-salt-00000!".to_vec());
        let relay_addrs = vec!["relay".to_string()];
        let sync_invite = invite_to_sync_invite(&invite, &relay_addrs);

        let code = &sync_invite.short_code;
        assert_eq!(code.len(), 19, "short code should be 19 chars: {}", code);
        assert_eq!(&code[4..5], "-");
        assert_eq!(&code[9..10], "-");
        assert_eq!(&code[14..15], "-");
    }

    #[tokio::test]
    async fn create_invite_rejects_empty_relay_addresses() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        let err = handle.create_invite(&[]).unwrap_err();
        assert!(matches!(err, SyncBridgeError::InvalidConfig(_)));
    }

    // --- derive_secret standalone ---

    #[test]
    fn derive_secret_returns_correct_lengths() {
        let (secret_bytes, group_id_bytes) = derive_secret("my-passphrase", b"test-salt-00000!");
        assert_eq!(secret_bytes.len(), 32);
        assert!(!group_id_bytes.is_empty());
    }

    #[test]
    fn derive_secret_is_deterministic() {
        let (s1, g1) = derive_secret("same-pass", b"same-salt-000000");
        let (s2, g2) = derive_secret("same-pass", b"same-salt-000000");
        assert_eq!(s1, s2);
        assert_eq!(g1, g2);
    }

    // --- Disconnect/shutdown ---

    #[tokio::test]
    async fn disconnect_on_disconnected_handle() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay-node");
        let handle = SyncHandle::create(config).await.unwrap();
        // Disconnect when not connected — should succeed gracefully
        // (transport close is idempotent)
        let result = handle.disconnect().await;
        // This may or may not error depending on transport state,
        // but it should not panic
        let _ = result;
    }
}
