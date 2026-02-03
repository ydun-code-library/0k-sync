//! Envelope - the wire format wrapper for all sync messages.

use serde::{Deserialize, Serialize};

use crate::{Cursor, DeviceId, GroupId, SyncError};

/// Message type discriminator for envelope routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    /// Initial handshake message
    Hello = 1,
    /// Push a blob to the group
    Push = 2,
    /// Acknowledgement of a push
    PushAck = 3,
    /// Request blobs after a cursor
    Pull = 4,
    /// Response to a pull request
    PullResponse = 5,
    /// Notification of new data available
    Notify = 6,
    /// Graceful disconnect
    Bye = 7,
}

impl TryFrom<u8> for MessageType {
    type Error = SyncError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Hello),
            2 => Ok(MessageType::Push),
            3 => Ok(MessageType::PushAck),
            4 => Ok(MessageType::Pull),
            5 => Ok(MessageType::PullResponse),
            6 => Ok(MessageType::Notify),
            7 => Ok(MessageType::Bye),
            _ => Err(SyncError::InvalidMessageType(value)),
        }
    }
}

/// The envelope wraps all protocol messages with routing metadata.
///
/// This is the outer layer that the relay sees. The payload is encrypted
/// and opaque to the relay (zero-knowledge).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    /// Protocol version (currently 1)
    pub version: u8,
    /// Message type discriminator
    pub msg_type: u8,
    /// Sender's device ID
    pub sender_id: DeviceId,
    /// Target sync group
    pub group_id: GroupId,
    /// Relay-assigned cursor (0 for client-originated messages)
    pub cursor: Cursor,
    /// Unix timestamp (seconds) - informational only, not trusted
    pub timestamp: u64,
    /// Encryption nonce (24 bytes for XChaCha20)
    pub nonce: [u8; 24],
    /// Encrypted payload (MessagePack-encoded inner message)
    pub payload: Vec<u8>,
}

impl Envelope {
    /// Create a new envelope for sending.
    pub fn new(
        msg_type: MessageType,
        sender_id: DeviceId,
        group_id: GroupId,
        nonce: [u8; 24],
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: 1,
            msg_type: msg_type as u8,
            sender_id,
            group_id,
            cursor: Cursor::zero(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            nonce,
            payload,
        }
    }

    /// Create a minimal envelope for testing.
    pub fn minimal() -> Self {
        Self {
            version: 1,
            msg_type: MessageType::Hello as u8,
            sender_id: DeviceId::random(),
            group_id: GroupId::random(),
            cursor: Cursor::zero(),
            timestamp: 0,
            nonce: [0u8; 24],
            payload: vec![],
        }
    }

    /// Serialize to MessagePack bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SyncError> {
        rmp_serde::to_vec(self).map_err(SyncError::Serialization)
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SyncError> {
        rmp_serde::from_slice(bytes).map_err(SyncError::Deserialization)
    }

    /// Get the message type as an enum.
    pub fn message_type(&self) -> Result<MessageType, SyncError> {
        MessageType::try_from(self.msg_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialize_roundtrip() {
        let envelope = Envelope {
            version: 1,
            msg_type: MessageType::Push as u8,
            sender_id: DeviceId::random(),
            group_id: GroupId::from_secret(b"test"),
            cursor: Cursor::new(42),
            timestamp: 1705000000,
            nonce: [0u8; 24],
            payload: vec![1, 2, 3, 4],
        };

        let bytes = envelope.to_bytes().unwrap();
        let restored = Envelope::from_bytes(&bytes).unwrap();

        assert_eq!(envelope.version, restored.version);
        assert_eq!(envelope.cursor, restored.cursor);
        assert_eq!(envelope.payload, restored.payload);
    }

    #[test]
    fn envelope_msgpack_is_compact() {
        let envelope = Envelope::minimal();
        let bytes = envelope.to_bytes().unwrap();
        // MessagePack should be much smaller than JSON equivalent
        assert!(bytes.len() < 200);
    }

    #[test]
    fn message_type_roundtrip() {
        for val in 1..=7u8 {
            let mt = MessageType::try_from(val).unwrap();
            assert_eq!(mt as u8, val);
        }
    }

    #[test]
    fn invalid_message_type_fails() {
        assert!(MessageType::try_from(0).is_err());
        assert!(MessageType::try_from(8).is_err());
        assert!(MessageType::try_from(255).is_err());
    }

    #[test]
    fn envelope_new_sets_timestamp() {
        let envelope = Envelope::new(
            MessageType::Hello,
            DeviceId::random(),
            GroupId::random(),
            [0u8; 24],
            vec![],
        );
        // Timestamp should be recent (within last minute)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(envelope.timestamp <= now);
        assert!(envelope.timestamp >= now - 60);
    }
}
