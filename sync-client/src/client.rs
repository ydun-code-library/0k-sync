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
//! let config = SyncConfig::new_with_salt("my-passphrase", b"random-salt-here", "node-address");
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
}

/// Configuration for SyncClient.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// The group secret (derived from passphrase).
    pub group_secret: GroupSecret,
    /// Address of the relay/peer to connect to (iroh NodeId).
    pub relay_address: String,
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
            relay_address: relay_address.to_string(),
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
            relay_address: relay_address.to_string(),
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
            relay_address: relay_address.to_string(),
            device_name: "0k-sync device".to_string(),
            default_ttl: 0,
        }
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
        }
    }

    /// Connect to the relay and perform HELLO/Welcome handshake.
    pub async fn connect(&self) -> Result<(), ClientError> {
        // Update state machine
        {
            let mut state = self.state.lock().await;
            let (new_state, _actions) = state.clone().on_event(Event::ConnectRequested);
            *state = new_state;
        }

        // Perform transport connection
        self.transport
            .connect(&self.config.relay_address)
            .await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        // Update state machine for successful connection
        {
            let mut state = self.state.lock().await;
            let (new_state, _actions) = state.clone().on_event(Event::ConnectSucceeded);
            *state = new_state;
        }

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

        let server_cursor = match welcome {
            Message::Welcome(w) => w.max_cursor,
            _ => return Err(ClientError::Protocol("expected Welcome response".into())),
        };

        // Complete handshake
        {
            let mut state = self.state.lock().await;
            let (final_state, _actions) = state.clone().on_event(Event::HandshakeCompleted {
                cursor: server_cursor,
            });
            *state = final_state;
        }

        Ok(())
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        let state = self.state.lock().await;
        state.is_connected()
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

        // Serialize and send
        let bytes = push
            .to_bytes()
            .map_err(|e| ClientError::Serialization(e.to_string()))?;
        self.transport.send(&bytes).await?;

        // Receive acknowledgement
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

        // Serialize and send
        let bytes = pull
            .to_bytes()
            .map_err(|e| ClientError::Serialization(e.to_string()))?;
        self.transport.send(&bytes).await?;

        // Receive response
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
    async fn connect_failure_returns_error() {
        let transport = MockTransport::new();
        transport.fail_next_connect("network unreachable");

        let client = SyncClient::new(test_config(), transport);

        let result = client.connect().await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ClientError::ConnectionFailed(_))));
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
}
