//! Protocol messages for 0k-Sync.
//!
//! These are the inner payloads that get encrypted before being wrapped
//! in an [`Envelope`].

use serde::{Deserialize, Serialize};

use crate::{BlobId, Cursor, GroupId, SyncError};

// Re-export MessageType from envelope for convenience
pub use crate::envelope::MessageType;

/// All possible protocol messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    /// Initial handshake
    Hello(Hello),
    /// Server response to Hello
    Welcome(Welcome),
    /// Push a blob
    Push(Push),
    /// Acknowledge a push
    PushAck(PushAck),
    /// Request blobs
    Pull(Pull),
    /// Response to pull
    PullResponse(PullResponse),
    /// New data notification
    Notify(Notify),
    /// Graceful disconnect
    Bye(Bye),
    /// Reference to large content (stored in iroh-blobs)
    ContentRef(ContentRef),
    /// Acknowledge content transfer complete
    ContentAck(ContentAck),
}

impl Message {
    /// Serialize to MessagePack bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SyncError> {
        rmp_serde::to_vec(self).map_err(SyncError::Serialization)
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SyncError> {
        rmp_serde::from_slice(bytes).map_err(SyncError::Deserialization)
    }
}

/// Initial handshake message sent by client on connect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Protocol version (currently 1)
    pub version: u8,
    /// Human-readable device name
    pub device_name: String,
    /// Target sync group
    pub group_id: GroupId,
    /// Client's last known cursor (for resumption)
    pub last_cursor: Cursor,
}

/// Server response to Hello handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    /// Protocol version supported by server
    pub version: u8,
    /// Highest cursor in this group
    pub max_cursor: Cursor,
    /// Number of blobs pending for this device
    pub pending_count: u32,
}

/// Push a blob to the sync group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Push {
    /// Unique identifier for this blob
    pub blob_id: BlobId,
    /// Encrypted payload (opaque to relay)
    pub payload: Vec<u8>,
    /// Time-to-live in seconds (0 = no expiry)
    pub ttl: u32,
}

/// Acknowledgement that a push was received and stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushAck {
    /// The blob that was acknowledged
    pub blob_id: BlobId,
    /// Relay-assigned cursor for this blob
    pub cursor: Cursor,
}

/// Request blobs after a given cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pull {
    /// Return blobs with cursor > this value
    pub after_cursor: Cursor,
    /// Maximum number of blobs to return (0 = no limit)
    pub limit: u32,
}

/// A single blob in a pull response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullBlob {
    /// Blob identifier
    pub blob_id: BlobId,
    /// Relay-assigned cursor
    pub cursor: Cursor,
    /// Encrypted payload
    pub payload: Vec<u8>,
    /// Original timestamp from push
    pub timestamp: u64,
}

/// Response to a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullResponse {
    /// The blobs matching the request
    pub blobs: Vec<PullBlob>,
    /// Whether there are more blobs available
    pub has_more: bool,
    /// Highest cursor in this response
    pub max_cursor: Cursor,
}

/// Notification that new data is available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notify {
    /// Latest cursor available
    pub latest_cursor: Cursor,
    /// Number of new blobs since client's last known cursor
    pub count: u32,
}

/// Graceful disconnect message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bye {
    /// Optional reason for disconnect
    pub reason: Option<String>,
}

/// Reference to large encrypted content (stored in iroh-blobs).
///
/// Small sync messages (<64KB) go through the relay directly.
/// Large content (photos, documents, audio) is stored in iroh-blobs
/// and only the ContentRef is sent through the relay.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentRef {
    /// BLAKE3 hash of ciphertext (content address for iroh-blobs)
    pub content_hash: [u8; 32],
    /// XChaCha20-Poly1305 encryption nonce (24 bytes)
    pub encryption_nonce: [u8; 24],
    /// Original plaintext size in bytes
    pub content_size: u64,
    /// Ciphertext size in bytes (content_size + 16 byte auth tag)
    pub encrypted_size: u64,
}

impl std::fmt::Debug for ContentRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentRef")
            .field("content_hash", &"[REDACTED]")
            .field("encryption_nonce", &"[REDACTED]")
            .field("content_size", &self.content_size)
            .field("encrypted_size", &self.encrypted_size)
            .finish()
    }
}

/// Acknowledge that content transfer is complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAck {
    /// BLAKE3 hash of the acknowledged content
    pub content_hash: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_roundtrip() {
        let hello = Hello {
            version: 1,
            device_name: "Test Device".into(),
            group_id: GroupId::from_secret(b"test"),
            last_cursor: Cursor::new(0),
        };

        let bytes = rmp_serde::to_vec(&hello).unwrap();
        let restored: Hello = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(hello.device_name, restored.device_name);
        assert_eq!(hello.version, restored.version);
    }

    #[test]
    fn welcome_roundtrip() {
        let welcome = Welcome {
            version: 1,
            max_cursor: Cursor::new(42),
            pending_count: 5,
        };

        let bytes = rmp_serde::to_vec(&welcome).unwrap();
        let restored: Welcome = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(welcome.version, restored.version);
        assert_eq!(welcome.max_cursor, restored.max_cursor);
        assert_eq!(welcome.pending_count, restored.pending_count);
    }

    #[test]
    fn push_with_payload() {
        let push = Push {
            blob_id: BlobId::new(),
            payload: vec![0u8; 1000],
            ttl: 3600,
        };

        let bytes = rmp_serde::to_vec(&push).unwrap();
        let restored: Push = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(push.payload.len(), restored.payload.len());
        assert_eq!(push.ttl, restored.ttl);
    }

    #[test]
    fn push_ack_roundtrip() {
        let ack = PushAck {
            blob_id: BlobId::new(),
            cursor: Cursor::new(42),
        };

        let bytes = rmp_serde::to_vec(&ack).unwrap();
        let restored: PushAck = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(ack.cursor, restored.cursor);
    }

    #[test]
    fn pull_roundtrip() {
        let pull = Pull {
            after_cursor: Cursor::new(100),
            limit: 50,
        };

        let bytes = rmp_serde::to_vec(&pull).unwrap();
        let restored: Pull = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(pull.after_cursor, restored.after_cursor);
        assert_eq!(pull.limit, restored.limit);
    }

    #[test]
    fn pull_response_with_blobs() {
        let response = PullResponse {
            blobs: vec![
                PullBlob {
                    blob_id: BlobId::new(),
                    cursor: Cursor::new(1),
                    payload: vec![1, 2, 3],
                    timestamp: 1705000000,
                },
                PullBlob {
                    blob_id: BlobId::new(),
                    cursor: Cursor::new(2),
                    payload: vec![4, 5, 6],
                    timestamp: 1705000001,
                },
            ],
            has_more: true,
            max_cursor: Cursor::new(2),
        };

        let bytes = rmp_serde::to_vec(&response).unwrap();
        let restored: PullResponse = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(response.blobs.len(), restored.blobs.len());
        assert_eq!(response.has_more, restored.has_more);
    }

    #[test]
    fn notify_roundtrip() {
        let notify = Notify {
            latest_cursor: Cursor::new(500),
            count: 10,
        };

        let bytes = rmp_serde::to_vec(&notify).unwrap();
        let restored: Notify = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(notify.latest_cursor, restored.latest_cursor);
        assert_eq!(notify.count, restored.count);
    }

    #[test]
    fn bye_with_reason() {
        let bye = Bye {
            reason: Some("client shutdown".into()),
        };

        let bytes = rmp_serde::to_vec(&bye).unwrap();
        let restored: Bye = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(bye.reason, restored.reason);
    }

    #[test]
    fn bye_without_reason() {
        let bye = Bye { reason: None };

        let bytes = rmp_serde::to_vec(&bye).unwrap();
        let restored: Bye = rmp_serde::from_slice(&bytes).unwrap();

        assert!(restored.reason.is_none());
    }

    #[test]
    fn message_enum_roundtrip() {
        let msg = Message::Push(Push {
            blob_id: BlobId::new(),
            payload: vec![1, 2, 3],
            ttl: 0,
        });

        let bytes = msg.to_bytes().unwrap();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert!(matches!(restored, Message::Push(_)));
    }

    #[test]
    fn content_ref_roundtrip() {
        let content_ref = ContentRef {
            content_hash: [0xAB; 32],
            encryption_nonce: [0xCD; 24],
            content_size: 1024 * 1024,        // 1MB
            encrypted_size: 1024 * 1024 + 16, // + auth tag
        };

        let bytes = rmp_serde::to_vec(&content_ref).unwrap();
        let restored: ContentRef = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(content_ref.content_hash, restored.content_hash);
        assert_eq!(content_ref.encryption_nonce, restored.encryption_nonce);
        assert_eq!(content_ref.content_size, restored.content_size);
        assert_eq!(content_ref.encrypted_size, restored.encrypted_size);
    }

    #[test]
    fn content_ack_roundtrip() {
        let content_ack = ContentAck {
            content_hash: [0xEF; 32],
        };

        let bytes = rmp_serde::to_vec(&content_ack).unwrap();
        let restored: ContentAck = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(content_ack.content_hash, restored.content_hash);
    }

    #[test]
    fn message_content_ref_roundtrip() {
        let msg = Message::ContentRef(ContentRef {
            content_hash: [0x11; 32],
            encryption_nonce: [0x22; 24],
            content_size: 5000,
            encrypted_size: 5016,
        });

        let bytes = msg.to_bytes().unwrap();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert!(matches!(restored, Message::ContentRef(_)));
    }

    #[test]
    fn content_ref_debug_redacts_sensitive_fields() {
        // F-016: ContentRef Debug must not leak content_hash or encryption_nonce
        let content_ref = ContentRef {
            content_hash: [0xAB; 32],
            encryption_nonce: [0xCD; 24],
            content_size: 1024,
            encrypted_size: 1040,
        };
        let debug = format!("{:?}", content_ref);
        assert!(
            debug.contains("REDACTED"),
            "hash and nonce should be redacted"
        );
        assert!(!debug.contains("171"), "raw byte values must not appear"); // 0xAB = 171
                                                                            // Size fields are fine to display (metadata, not secrets)
        assert!(debug.contains("1024"), "content_size should be visible");
    }
}
