//! SyncClient - the main interface for 0k-Sync.
//!
//! This module provides [`SyncClient`], the primary API for applications
//! to sync encrypted data using the 0k-Sync protocol.
//!
//! # Architecture
//!
//! SyncClient uses a pure state machine (from sync-core) for protocol logic
//! and interprets the actions to perform actual I/O via the Transport trait.
//!
//! ```text
//! Application → SyncClient → Transport → Network
//!                   ↓
//!              sync-core (pure state machine)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use sync_client::{SyncClient, SyncConfig, MockTransport};
//!
//! let transport = MockTransport::new();
//! let config = SyncConfig::new_with_salt("my-passphrase", b"random-salt-here", "node-address"); // single relay
//! let client = SyncClient::new(config, transport);
//!
//! // Connect and sync
//! client.connect().await?;
//! client.push(b"encrypted data").await?;
//! let blobs = client.pull().await?;
//! ```

use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use zerok_sync_core::{ConnectionState, CursorTracker, Event};
use zerok_sync_types::{
    BlobId, Cursor, GroupId, Hello, Message, Pull, PullResponse, Push, PushAck,
};

use crate::crypto::{CryptoError, GroupKey, GroupSecret};
use crate::transport::{Transport, TransportError};

/// Client errors.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Transport error.
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// Crypto error.
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Not connected to relay.
    #[error("not connected")]
    NotConnected,

    /// Connection failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// All configured relay addresses failed to connect.
    #[error("all relays failed: {0}")]
    AllRelaysFailed(String),
}

/// Configuration for SyncClient.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// The group secret (derived from passphrase).
    pub group_secret: GroupSecret,
    /// Addresses of relays to connect to (iroh NodeIds).
    /// First is primary; remaining are secondaries for fan-out.
    pub relay_addresses: Vec<String>,
    /// Human-readable device name.
    pub device_name: String,
    /// Time-to-live for pushed blobs (seconds, 0 = no expiry).
    pub default_ttl: u32,
}

impl SyncConfig {
    /// Create a new configuration from a passphrase (generates random salt).
    ///
    /// Returns the config and the 16-byte salt. The salt MUST be shared with
    /// other devices (e.g., in the Invite payload) so they can derive the same secret.
    pub fn new(passphrase: &str, relay_address: &str) -> (Self, [u8; 16]) {
        let (secret, salt) = GroupSecret::from_passphrase(passphrase);
        let config = Self {
            group_secret: secret,
            relay_addresses: vec![relay_address.to_string()],
            device_name: "0k-sync device".to_string(),
            default_ttl: 0,
        };
        (config, salt)
    }

    /// Create a configuration from a passphrase with an explicit salt.
    ///
    /// Use this when joining a group where the salt is known from the Invite.
    pub fn new_with_salt(passphrase: &str, salt: &[u8], relay_address: &str) -> Self {
        Self {
            group_secret: GroupSecret::from_passphrase_with_salt(passphrase, salt),
            relay_addresses: vec![relay_address.to_string()],
            device_name: "0k-sync device".to_string(),
            default_ttl: 0,
        }
    }

    /// Create a configuration from pre-derived secret bytes.
    pub fn from_secret_bytes(secret_bytes: &[u8; 32], relay_address: &str) -> Self {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(secret_bytes);
        Self {
            group_secret: GroupSecret::from_raw(bytes),
            relay_addresses: vec![relay_address.to_string()],
            device_name: "0k-sync device".to_string(),
            default_ttl: 0,
        }
    }

    /// Get the primary relay address (first in the list).
    pub fn primary_relay(&self) -> Option<&str> {
        self.relay_addresses.first().map(|s| s.as_str())
    }

    /// Set the device name.
    pub fn with_device_name(mut self, name: &str) -> Self {
        self.device_name = name.to_string();
        self
    }

    /// Set the default TTL for pushed blobs.
    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set multiple relay addresses (for multi-relay fan-out/failover).
    pub fn with_relay_addresses(mut self, addresses: &[&str]) -> Self {
        self.relay_addresses = addresses.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// A received blob from the sync group.
#[derive(Clone)]
pub struct ReceivedBlob {
    /// The blob identifier.
    pub blob_id: BlobId,
    /// The decrypted payload.
    pub payload: Vec<u8>,
    /// The cursor position of this blob.
    pub cursor: Cursor,
    /// Original timestamp.
    pub timestamp: u64,
}

impl std::fmt::Debug for ReceivedBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReceivedBlob")
            .field("blob_id", &self.blob_id)
            .field(
                "payload",
                &format!("[{} bytes REDACTED]", self.payload.len()),
            )
            .field("cursor", &self.cursor)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

/// The main sync client.
///
/// Manages connection, encryption, and sync operations.
pub struct SyncClient<T: Transport> {
    config: SyncConfig,
    transport: T,
    key: GroupKey,
    state: Arc<Mutex<ConnectionState>>,
    cursor: Arc<Mutex<CursorTracker>>,
    active_relay: Arc<Mutex<Option<String>>>,
}

impl<T: Transport> SyncClient<T> {
    /// Create a new SyncClient.
    pub fn new(config: SyncConfig, transport: T) -> Self {
        let key = GroupKey::derive(&config.group_secret);
        Self {
            config,
            transport,
            key,
            state: Arc::new(Mutex::new(ConnectionState::new())),
            cursor: Arc::new(Mutex::new(CursorTracker::new())),
            active_relay: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the relay and perform HELLO/Welcome handshake.
    ///
    /// Tries each relay address in order until one succeeds the full
    /// transport connect + HELLO/Welcome handshake. The first successful
    /// relay becomes the "active" relay for this session.
    pub async fn connect(&self) -> Result<(), ClientError> {
        // Update state machine
        {
            let mut state = self.state.lock().await;
            let (new_state, _actions) = state.clone().on_event(Event::ConnectRequested);
            *state = new_state;
        }

        if self.config.relay_addresses.is_empty() {
            return Err(ClientError::AllRelaysFailed(
                "no relay addresses configured".to_string(),
            ));
        }

        let mut errors: Vec<(String, String)> = Vec::new();

        for address in &self.config.relay_addresses {
            match self.try_connect_relay(address).await {
                Ok(server_cursor) => {
                    // Update state machine for successful connection
                    {
                        let mut state = self.state.lock().await;
                        let (new_state, _actions) = state.clone().on_event(Event::ConnectSucceeded);
                        *state = new_state;
                    }

                    // Complete handshake
                    {
                        let mut state = self.state.lock().await;
                        let (final_state, _actions) =
                            state.clone().on_event(Event::HandshakeCompleted {
                                cursor: server_cursor,
                            });
                        *state = final_state;
                    }

                    // Track the active relay
                    {
                        let mut active = self.active_relay.lock().await;
                        *active = Some(address.clone());
                    }

                    return Ok(());
                }
                Err(e) => {
                    // Clean up partial connection before trying next relay
                    let _ = self.transport.close().await;
                    errors.push((address.clone(), e.to_string()));
                }
            }
        }

        // All relays failed
        let error_details: Vec<String> = errors
            .iter()
            .map(|(addr, err)| format!("{}: {}", addr, err))
            .collect();
        Err(ClientError::AllRelaysFailed(error_details.join("; ")))
    }

    /// Try connecting to a single relay: transport connect + HELLO/Welcome handshake.
    async fn try_connect_relay(&self, address: &str) -> Result<Cursor, ClientError> {
        // Transport-level connection
        self.transport
            .connect(address)
            .await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        // Send HELLO with group identity
        let group_id = GroupId::from_secret(self.config.group_secret.as_bytes());
        let last_cursor = self.cursor.lock().await.last_cursor();
        let hello = Message::Hello(Hello {
            version: 1,
            device_name: self.config.device_name.clone(),
            group_id,
            last_cursor,
        });
        let hello_bytes = hello
            .to_bytes()
            .map_err(|e| ClientError::Serialization(e.to_string()))?;
        self.transport.send(&hello_bytes).await?;

        // Receive Welcome response
        let welcome_bytes = self.transport.recv().await?;
        let welcome = Message::from_bytes(&welcome_bytes)
            .map_err(|e| ClientError::Serialization(e.to_string()))?;

        match welcome {
            Message::Welcome(w) => Ok(w.max_cursor),
            _ => Err(ClientError::Protocol("expected Welcome response".into())),
        }
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        let state = self.state.lock().await;
        state.is_connected()
    }

    /// Get the address of the relay we're connected to.
    pub async fn active_relay(&self) -> Option<String> {
        self.active_relay.lock().await.clone()
    }

    /// Try to reconnect to a different relay after the current one failed.
    ///
    /// Attempts each relay address after the current active one. If the current
    /// relay is unknown or at the end of the list, starts from the beginning
    /// (excluding the failed relay).
    async fn try_reconnect(&self) -> Result<(), ClientError> {
        // Close the failed connection
        let _ = self.transport.close().await;

        // Update state to disconnected
        {
            let mut state = self.state.lock().await;
            let (new_state, _) = state.clone().on_event(Event::DisconnectRequested);
            *state = new_state;
        }

        let current_relay = self.active_relay.lock().await.clone();
        let addresses = &self.config.relay_addresses;

        if addresses.is_empty() {
            return Err(ClientError::AllRelaysFailed(
                "no relay addresses configured".to_string(),
            ));
        }

        // Find the index of the current relay
        let current_index = current_relay
            .as_ref()
            .and_then(|r| addresses.iter().position(|a| a == r));

        // Collect relays to try: all relays after current, then wrap around (excluding current)
        let mut relays_to_try: Vec<&String> = Vec::new();
        if let Some(idx) = current_index {
            // Add relays after current
            relays_to_try.extend(addresses.iter().skip(idx + 1));
            // Add relays before current (wrap around)
            relays_to_try.extend(addresses.iter().take(idx));
        } else {
            // Current relay unknown, try all
            relays_to_try.extend(addresses.iter());
        }

        let mut errors: Vec<(String, String)> = Vec::new();

        for address in relays_to_try {
            match self.try_connect_relay(address).await {
                Ok(server_cursor) => {
                    // Update state machine
                    {
                        let mut state = self.state.lock().await;
                        let (new_state, _) = state.clone().on_event(Event::ConnectSucceeded);
                        *state = new_state;
                    }
                    {
                        let mut state = self.state.lock().await;
                        let (final_state, _) = state.clone().on_event(Event::HandshakeCompleted {
                            cursor: server_cursor,
                        });
                        *state = final_state;
                    }
                    // Track the new active relay
                    {
                        let mut active = self.active_relay.lock().await;
                        *active = Some(address.clone());
                    }
                    return Ok(());
                }
                Err(e) => {
                    let _ = self.transport.close().await;
                    errors.push((address.clone(), e.to_string()));
                }
            }
        }

        // All remaining relays failed
        let error_details: Vec<String> = errors
            .iter()
            .map(|(addr, err)| format!("{}: {}", addr, err))
            .collect();
        Err(ClientError::AllRelaysFailed(error_details.join("; ")))
    }

    /// Disconnect from the relay.
    pub async fn disconnect(&self) -> Result<(), ClientError> {
        {
            let mut state = self.state.lock().await;
            let (new_state, _actions) = state.clone().on_event(Event::DisconnectRequested);
            *state = new_state;
        }

        self.transport.close().await?;
        Ok(())
    }

    /// Push encrypted data to the sync group.
    ///
    /// Returns the blob ID and assigned cursor.
    /// On transport failure, automatically fails over to another relay and retries.
    pub async fn push(&self, plaintext: &[u8]) -> Result<(BlobId, Cursor), ClientError> {
        if !self.is_connected().await {
            return Err(ClientError::NotConnected);
        }

        // Encrypt the payload
        let (ciphertext, nonce) = self.key.encrypt(plaintext)?;

        // Prepend nonce to ciphertext for self-describing format
        let mut payload = Vec::with_capacity(crate::NONCE_SIZE + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);

        // Create push message
        let blob_id = BlobId::new();
        let push = Message::Push(Push {
            blob_id,
            payload,
            ttl: self.config.default_ttl,
        });

        // Serialize
        let bytes = push
            .to_bytes()
            .map_err(|e| ClientError::Serialization(e.to_string()))?;

        // Try to push, with failover on transport error
        match self.try_push_bytes(&bytes, blob_id).await {
            Ok(result) => Ok(result),
            Err(ClientError::Transport(_)) => {
                // Transport failed, try to reconnect to another relay
                self.try_reconnect().await?;
                // Retry the push on the new relay
                self.try_push_bytes(&bytes, blob_id).await
            }
            Err(e) => Err(e),
        }
    }

    /// Internal: attempt to send push message and receive ack.
    async fn try_push_bytes(
        &self,
        bytes: &[u8],
        blob_id: BlobId,
    ) -> Result<(BlobId, Cursor), ClientError> {
        self.transport.send(bytes).await?;

        let response_bytes = self.transport.recv().await?;
        let response = Message::from_bytes(&response_bytes)
            .map_err(|e| ClientError::Serialization(e.to_string()))?;

        match response {
            Message::PushAck(PushAck {
                blob_id: ack_id,
                cursor,
            }) if ack_id == blob_id => {
                // Update cursor tracker
                let mut cursor_tracker = self.cursor.lock().await;
                cursor_tracker.received(cursor);
                Ok((blob_id, cursor))
            }
            _ => Err(ClientError::Protocol("unexpected response to push".into())),
        }
    }

    /// Pull new blobs from the sync group.
    ///
    /// Returns blobs with cursor > the last known cursor.
    pub async fn pull(&self) -> Result<Vec<ReceivedBlob>, ClientError> {
        self.pull_after(None).await
    }

    /// Pull blobs after a specific cursor.
    ///
    /// If `after` is None, uses the last known cursor.
    /// On transport failure, automatically fails over to another relay and retries.
    pub async fn pull_after(
        &self,
        after: Option<Cursor>,
    ) -> Result<Vec<ReceivedBlob>, ClientError> {
        if !self.is_connected().await {
            return Err(ClientError::NotConnected);
        }

        let after_cursor = match after {
            Some(c) => c,
            None => {
                let cursor_tracker = self.cursor.lock().await;
                cursor_tracker.last_cursor()
            }
        };

        // Create pull request
        let pull = Message::Pull(Pull {
            after_cursor,
            limit: 100, // Default batch size
        });

        // Serialize
        let bytes = pull
            .to_bytes()
            .map_err(|e| ClientError::Serialization(e.to_string()))?;

        // Try to pull, with failover on transport error
        match self.try_pull_bytes(&bytes).await {
            Ok(result) => Ok(result),
            Err(ClientError::Transport(_)) => {
                // Transport failed, try to reconnect to another relay
                self.try_reconnect().await?;
                // Retry the pull on the new relay
                self.try_pull_bytes(&bytes).await
            }
            Err(e) => Err(e),
        }
    }

    /// Internal: attempt to send pull request and receive/decrypt response.
    async fn try_pull_bytes(&self, bytes: &[u8]) -> Result<Vec<ReceivedBlob>, ClientError> {
        self.transport.send(bytes).await?;

        let response_bytes = self.transport.recv().await?;
        let response = Message::from_bytes(&response_bytes)
            .map_err(|e| ClientError::Serialization(e.to_string()))?;

        match response {
            Message::PullResponse(PullResponse {
                blobs,
                has_more: _,
                max_cursor,
            }) => {
                let mut received = Vec::with_capacity(blobs.len());

                for blob in blobs {
                    // Decrypt payload (nonce is prepended)
                    if blob.payload.len() < crate::NONCE_SIZE {
                        // Skip malformed blobs
                        continue;
                    }

                    let nonce: [u8; crate::NONCE_SIZE] =
                        blob.payload[..crate::NONCE_SIZE].try_into().unwrap();
                    let ciphertext = &blob.payload[crate::NONCE_SIZE..];

                    match self.key.decrypt(ciphertext, &nonce) {
                        Ok(plaintext) => {
                            received.push(ReceivedBlob {
                                blob_id: blob.blob_id,
                                payload: plaintext,
                                cursor: blob.cursor,
                                timestamp: blob.timestamp,
                            });
                        }
                        Err(_) => {
                            // Skip blobs we can't decrypt (wrong key)
                            continue;
                        }
                    }
                }

                // Update cursor tracker
                {
                    let mut cursor_tracker = self.cursor.lock().await;
                    cursor_tracker.received(max_cursor);
                }

                Ok(received)
            }
            _ => Err(ClientError::Protocol("unexpected response to pull".into())),
        }
    }

    /// Get the current cursor position.
    pub async fn current_cursor(&self) -> Cursor {
        let cursor_tracker = self.cursor.lock().await;
        cursor_tracker.last_cursor()
    }

    /// Get a reference to the underlying transport (for testing).
    pub fn transport(&self) -> &T {
        &self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use zerok_sync_types::{GroupId, PullBlob, Welcome};

    fn test_config() -> SyncConfig {
        // Fixed salt for deterministic test configs
        SyncConfig::new_with_salt("test-passphrase", b"test-salt-00000!", "test-node")
    }

    // ===========================================
    // Configuration Tests
    // ===========================================

    #[test]
    fn config_creates_group_secret() {
        let salt = b"test-salt-00000!";
        let config = SyncConfig::new_with_salt("my-passphrase", salt, "node-address");
        // Secret should be deterministic given same passphrase + salt
        let config2 = SyncConfig::new_with_salt("my-passphrase", salt, "node-address");
        assert_eq!(
            config.group_secret.as_bytes(),
            config2.group_secret.as_bytes()
        );
    }

    #[test]
    fn config_builder_pattern() {
        let config = SyncConfig::new_with_salt("pass", b"test-salt-00000!", "node")
            .with_device_name("My Device")
            .with_ttl(3600);

        assert_eq!(config.device_name, "My Device");
        assert_eq!(config.default_ttl, 3600);
    }

    // ===========================================
    // Connection Tests
    // ===========================================

    /// Create a mock Welcome response for connect handshake.
    fn mock_welcome(max_cursor: u64, pending_count: u32) -> Vec<u8> {
        Message::Welcome(Welcome {
            version: 1,
            max_cursor: Cursor::new(max_cursor),
            pending_count,
        })
        .to_bytes()
        .unwrap()
    }

    #[tokio::test]
    async fn client_connects_via_transport() {
        let transport = MockTransport::new();
        // Queue Welcome response for HELLO handshake
        transport.queue_response(mock_welcome(0, 0));
        let client = SyncClient::new(test_config(), transport.clone());

        assert!(!client.is_connected().await);

        client.connect().await.unwrap();

        assert!(client.is_connected().await);
        assert_eq!(transport.connected_address(), Some("test-node".to_string()));
    }

    #[tokio::test]
    async fn connect_sends_hello_with_group_id() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let config = test_config();
        let expected_group_id = GroupId::from_secret(config.group_secret.as_bytes());
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();

        // Verify HELLO was sent
        let sent = transport.sent_messages();
        assert_eq!(sent.len(), 1);
        let msg = Message::from_bytes(&sent[0]).unwrap();
        match msg {
            Message::Hello(hello) => {
                assert_eq!(hello.version, 1);
                assert_eq!(hello.group_id, expected_group_id);
                assert_eq!(hello.device_name, "0k-sync device");
                assert_eq!(hello.last_cursor, Cursor::zero());
            }
            other => panic!("Expected Hello, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn client_disconnects() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let client = SyncClient::new(test_config(), transport.clone());

        client.connect().await.unwrap();
        assert!(client.is_connected().await);

        client.disconnect().await.unwrap();
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn connect_failure_returns_all_relays_failed() {
        let transport = MockTransport::new();
        transport.fail_next_connect("network unreachable");

        let client = SyncClient::new(test_config(), transport);

        let result = client.connect().await;
        assert!(result.is_err());
        assert!(
            matches!(result, Err(ClientError::AllRelaysFailed(_))),
            "expected AllRelaysFailed, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn connect_fails_first_relay_succeeds_on_second() {
        let transport = MockTransport::new();
        // First connect (relay-a) fails, second (relay-b) succeeds
        transport.fail_next_connect("relay-a unreachable");
        // Queue Welcome for the successful relay-b handshake
        transport.queue_response(mock_welcome(5, 0));

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();

        assert!(client.is_connected().await);
        assert_eq!(
            transport.connected_address(),
            Some("relay-b".to_string()),
            "should be connected to second relay"
        );
        assert_eq!(
            client.active_relay().await,
            Some("relay-b".to_string()),
            "active_relay should track the successful relay"
        );
    }

    #[tokio::test]
    async fn connect_fails_all_relays_returns_error() {
        let transport = MockTransport::new();
        // Both relays fail at transport level
        transport.fail_next_n_connects(3, "unreachable");

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b", "relay-c"]);
        let client = SyncClient::new(config, transport);

        let result = client.connect().await;
        assert!(result.is_err());
        match &result {
            Err(ClientError::AllRelaysFailed(msg)) => {
                assert!(msg.contains("relay-a"), "should mention relay-a: {}", msg);
                assert!(msg.contains("relay-b"), "should mention relay-b: {}", msg);
                assert!(msg.contains("relay-c"), "should mention relay-c: {}", msg);
            }
            other => panic!("expected AllRelaysFailed, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn connect_tracks_active_relay() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let client = SyncClient::new(test_config(), transport);

        assert!(client.active_relay().await.is_none());

        client.connect().await.unwrap();

        assert_eq!(client.active_relay().await, Some("test-node".to_string()));
    }

    #[tokio::test]
    async fn connect_with_empty_relays_fails() {
        let transport = MockTransport::new();
        let config = test_config().with_relay_addresses(&[]);
        let client = SyncClient::new(config, transport);

        let result = client.connect().await;
        assert!(matches!(result, Err(ClientError::AllRelaysFailed(_))));
    }

    // ===========================================
    // Push Tests
    // ===========================================

    #[tokio::test]
    async fn push_without_connect_fails() {
        let transport = MockTransport::new();
        let client = SyncClient::new(test_config(), transport);

        let result = client.push(b"data").await;
        assert!(matches!(result, Err(ClientError::NotConnected)));
    }

    #[tokio::test]
    async fn push_encrypts_and_sends() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let client = SyncClient::new(test_config(), transport.clone());
        client.connect().await.unwrap();

        // Queue a PushAck response
        let ack = Message::PushAck(PushAck {
            blob_id: BlobId::new(), // Will be replaced
            cursor: Cursor::new(1),
        });
        transport.queue_response(ack.to_bytes().unwrap());

        // This will fail because the blob_id won't match
        // Let's verify the message was sent at least
        let result = client.push(b"test data").await;

        // The push will fail because mock returns wrong blob_id
        assert!(result.is_err());

        // Verify we sent HELLO + Push (2 messages)
        let sent = transport.sent_messages();
        assert_eq!(sent.len(), 2);

        // First message is HELLO, second is Push
        let msg = Message::from_bytes(&sent[1]).unwrap();
        assert!(matches!(msg, Message::Push(_)));
    }

    #[tokio::test]
    async fn push_payload_is_encrypted() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let config = test_config();
        let key = GroupKey::derive(&config.group_secret);
        let client = SyncClient::new(config, transport.clone());
        client.connect().await.unwrap();

        // Create a properly matching PushAck by parsing the sent message first
        // For this test, we just verify encryption format
        let plaintext = b"secret message";

        // We need to intercept and respond properly
        // Since MockTransport doesn't support this, just verify format
        transport.queue_response(vec![]); // Will cause recv to fail
        let _ = client.push(plaintext).await; // Ignore result

        let sent = transport.sent_messages();
        // sent[0] is HELLO, sent[1] is Push
        let msg = Message::from_bytes(&sent[1]).unwrap();

        if let Message::Push(push) = msg {
            // Payload should be nonce + ciphertext
            assert!(push.payload.len() > crate::NONCE_SIZE);

            // Extract nonce and ciphertext
            let nonce: [u8; crate::NONCE_SIZE] =
                push.payload[..crate::NONCE_SIZE].try_into().unwrap();
            let ciphertext = &push.payload[crate::NONCE_SIZE..];

            // Should be decryptable with the same key
            let decrypted = key.decrypt(ciphertext, &nonce).unwrap();
            assert_eq!(decrypted, plaintext);
        } else {
            panic!("Expected Push message");
        }
    }

    // ===========================================
    // Pull Tests
    // ===========================================

    #[tokio::test]
    async fn pull_without_connect_fails() {
        let transport = MockTransport::new();
        let client = SyncClient::new(test_config(), transport);

        let result = client.pull().await;
        assert!(matches!(result, Err(ClientError::NotConnected)));
    }

    #[tokio::test]
    async fn pull_decrypts_received_blobs() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let config = test_config();
        let key = GroupKey::derive(&config.group_secret);
        let client = SyncClient::new(config, transport.clone());
        client.connect().await.unwrap();

        // Create encrypted blob payload
        let plaintext = b"received secret data";
        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();
        let mut payload = Vec::new();
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);

        // Queue a PullResponse
        let response = Message::PullResponse(PullResponse {
            blobs: vec![PullBlob {
                blob_id: BlobId::new(),
                cursor: Cursor::new(10),
                payload,
                timestamp: 1705000000,
            }],
            has_more: false,
            max_cursor: Cursor::new(10),
        });
        transport.queue_response(response.to_bytes().unwrap());

        let blobs = client.pull().await.unwrap();

        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].payload, plaintext);
        assert_eq!(blobs[0].cursor, Cursor::new(10));
    }

    #[tokio::test]
    async fn pull_updates_cursor() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let config = test_config();
        let key = GroupKey::derive(&config.group_secret);
        let client = SyncClient::new(config, transport.clone());
        client.connect().await.unwrap();

        assert_eq!(client.current_cursor().await, Cursor::zero());

        // Create encrypted blob
        let (ciphertext, nonce) = key.encrypt(b"data").unwrap();
        let mut payload = Vec::new();
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);

        let response = Message::PullResponse(PullResponse {
            blobs: vec![PullBlob {
                blob_id: BlobId::new(),
                cursor: Cursor::new(42),
                payload,
                timestamp: 1705000000,
            }],
            has_more: false,
            max_cursor: Cursor::new(42),
        });
        transport.queue_response(response.to_bytes().unwrap());

        client.pull().await.unwrap();

        assert_eq!(client.current_cursor().await, Cursor::new(42));
    }

    #[tokio::test]
    async fn pull_skips_undecryptable_blobs() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let config = test_config();
        let key = GroupKey::derive(&config.group_secret);
        let client = SyncClient::new(config, transport.clone());
        client.connect().await.unwrap();

        // Create one valid encrypted blob
        let (ciphertext, nonce) = key.encrypt(b"valid").unwrap();
        let mut valid_payload = Vec::new();
        valid_payload.extend_from_slice(&nonce);
        valid_payload.extend_from_slice(&ciphertext);

        // Create one blob with garbage (wrong key)
        let other_key = GroupKey::derive(&GroupSecret::random());
        let (ciphertext2, nonce2) = other_key.encrypt(b"invalid").unwrap();
        let mut invalid_payload = Vec::new();
        invalid_payload.extend_from_slice(&nonce2);
        invalid_payload.extend_from_slice(&ciphertext2);

        let response = Message::PullResponse(PullResponse {
            blobs: vec![
                PullBlob {
                    blob_id: BlobId::new(),
                    cursor: Cursor::new(1),
                    payload: valid_payload,
                    timestamp: 1705000000,
                },
                PullBlob {
                    blob_id: BlobId::new(),
                    cursor: Cursor::new(2),
                    payload: invalid_payload,
                    timestamp: 1705000001,
                },
            ],
            has_more: false,
            max_cursor: Cursor::new(2),
        });
        transport.queue_response(response.to_bytes().unwrap());

        let blobs = client.pull().await.unwrap();

        // Only the valid blob should be returned
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].payload, b"valid");
    }

    #[tokio::test]
    async fn pull_after_specific_cursor() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        let client = SyncClient::new(test_config(), transport.clone());
        client.connect().await.unwrap();

        // Queue empty response
        let response = Message::PullResponse(PullResponse {
            blobs: vec![],
            has_more: false,
            max_cursor: Cursor::new(100),
        });
        transport.queue_response(response.to_bytes().unwrap());

        client.pull_after(Some(Cursor::new(50))).await.unwrap();

        // Verify the Pull request had the right cursor
        let sent = transport.sent_messages();
        // sent[0] is HELLO, sent[1] is Pull
        let msg = Message::from_bytes(&sent[1]).unwrap();

        if let Message::Pull(pull) = msg {
            assert_eq!(pull.after_cursor, Cursor::new(50));
        } else {
            panic!("Expected Pull message");
        }
    }

    // ===========================================
    // Transport Access Tests
    // ===========================================

    #[tokio::test]
    async fn transport_accessible_for_testing() {
        let transport = MockTransport::new();
        let client = SyncClient::new(test_config(), transport);

        // Can access transport for test verification
        assert!(!client.transport().is_connected());
    }

    // ===========================================
    // Debug Redaction Tests (F-011)
    // ===========================================

    #[test]
    fn received_blob_debug_redacts_payload() {
        let blob = ReceivedBlob {
            blob_id: BlobId::new(),
            payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
            cursor: Cursor::new(42),
            timestamp: 1705000000,
        };
        let debug = format!("{:?}", blob);
        assert!(
            debug.contains("[4 bytes REDACTED]"),
            "payload should be redacted, got: {}",
            debug
        );
        assert!(
            !debug.contains("DEAD") && !debug.contains("dead"),
            "payload bytes must not appear in Debug output"
        );
    }

    // ===========================================
    // Failover Tests (Option A: transparent retry)
    // ===========================================

    #[tokio::test]
    async fn push_reconnects_on_send_failure() {
        let transport = MockTransport::new();
        // Queue Welcome for initial connect to relay-a
        transport.queue_response(mock_welcome(0, 0));
        // Queue Welcome for reconnect to relay-b
        transport.queue_response(mock_welcome(0, 0));

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        // Connect initially to relay-a
        client.connect().await.unwrap();
        assert_eq!(client.active_relay().await, Some("relay-a".to_string()));

        // Simulate relay-a dying: next send will fail
        transport.fail_next_send("connection reset");

        // Queue PushAck for the retried push on relay-b
        // We need to know the blob_id, but we can't predict it
        // So let's just verify the reconnect happened by checking active_relay
        let ack = Message::PushAck(PushAck {
            blob_id: BlobId::new(), // Won't match, but we test reconnect
            cursor: Cursor::new(1),
        });
        transport.queue_response(ack.to_bytes().unwrap());

        // Push should fail on relay-a, reconnect to relay-b, and retry
        let _result = client.push(b"test data").await;

        // The push will fail due to blob_id mismatch, but we should have reconnected
        // Verify we switched to relay-b
        assert_eq!(
            client.active_relay().await,
            Some("relay-b".to_string()),
            "should have failed over to relay-b"
        );
    }

    #[tokio::test]
    async fn push_reconnects_on_recv_failure() {
        let transport = MockTransport::new();
        // Queue Welcome for initial connect
        transport.queue_response(mock_welcome(0, 0));
        // Queue Welcome for reconnect
        transport.queue_response(mock_welcome(0, 0));

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();
        assert_eq!(client.active_relay().await, Some("relay-a".to_string()));

        // Simulate relay-a dying after send but before recv
        transport.fail_next_recv("connection reset");

        // Queue PushAck for the retried push
        let ack = Message::PushAck(PushAck {
            blob_id: BlobId::new(),
            cursor: Cursor::new(1),
        });
        transport.queue_response(ack.to_bytes().unwrap());

        let _ = client.push(b"test data").await;

        // Verify failover occurred
        assert_eq!(
            client.active_relay().await,
            Some("relay-b".to_string()),
            "should have failed over to relay-b after recv failure"
        );
    }

    #[tokio::test]
    async fn pull_reconnects_on_send_failure() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        transport.queue_response(mock_welcome(0, 0));

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();
        assert_eq!(client.active_relay().await, Some("relay-a".to_string()));

        // Simulate relay dying on pull's send
        transport.fail_next_send("connection reset");

        // Queue empty PullResponse for retry
        let response = Message::PullResponse(PullResponse {
            blobs: vec![],
            has_more: false,
            max_cursor: Cursor::new(0),
        });
        transport.queue_response(response.to_bytes().unwrap());

        let blobs = client.pull().await.unwrap();

        assert!(blobs.is_empty());
        assert_eq!(
            client.active_relay().await,
            Some("relay-b".to_string()),
            "should have failed over to relay-b"
        );
    }

    #[tokio::test]
    async fn pull_reconnects_on_recv_failure() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        transport.queue_response(mock_welcome(0, 0));

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();

        transport.fail_next_recv("connection reset");

        let response = Message::PullResponse(PullResponse {
            blobs: vec![],
            has_more: false,
            max_cursor: Cursor::new(0),
        });
        transport.queue_response(response.to_bytes().unwrap());

        let blobs = client.pull().await.unwrap();

        assert!(blobs.is_empty());
        assert_eq!(
            client.active_relay().await,
            Some("relay-b".to_string()),
            "should have failed over to relay-b"
        );
    }

    #[tokio::test]
    async fn push_fails_when_all_relays_exhausted() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));
        // No more welcomes - reconnect attempts will fail

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();

        // Fail send on relay-a
        transport.fail_next_send("connection reset");
        // Fail connect to relay-b
        transport.fail_next_connect("unreachable");

        let result = client.push(b"test data").await;

        assert!(
            matches!(result, Err(ClientError::AllRelaysFailed(_))),
            "should return AllRelaysFailed when no relays left, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn pull_fails_when_all_relays_exhausted() {
        let transport = MockTransport::new();
        transport.queue_response(mock_welcome(0, 0));

        let config = test_config().with_relay_addresses(&["relay-a", "relay-b"]);
        let client = SyncClient::new(config, transport.clone());

        client.connect().await.unwrap();

        transport.fail_next_send("connection reset");
        transport.fail_next_connect("unreachable");

        let result = client.pull().await;

        assert!(
            matches!(result, Err(ClientError::AllRelaysFailed(_))),
            "should return AllRelaysFailed when no relays left, got: {:?}",
            result
        );
    }
}
