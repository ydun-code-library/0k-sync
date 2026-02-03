//! Identity and ordering types for 0k-Sync.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A unique identifier for a device in the sync network.
///
/// 32 bytes of random data, displayed as URL-safe base64.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId([u8; 32]);

impl DeviceId {
    /// Create a new random DeviceId.
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }

    /// Create a DeviceId from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(bytes);
            Some(Self(arr))
        } else {
            None
        }
    }

    /// Get the raw bytes of this DeviceId.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", URL_SAFE_NO_PAD.encode(self.0))
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({})", &self.to_string()[..8])
    }
}

/// A unique identifier for a sync group.
///
/// Derived from the group secret using a KDF.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId([u8; 32]);

impl GroupId {
    /// Create a GroupId from a secret passphrase.
    ///
    /// Uses SHA-256 for now; will use Argon2id in production.
    pub fn from_secret(secret: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"0k-sync-group-id-v1");
        hasher.update(secret);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }

    /// Create a random GroupId (for testing).
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }

    /// Create a GroupId from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(bytes);
            Some(Self(arr))
        } else {
            None
        }
    }

    /// Get the raw bytes of this GroupId.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", URL_SAFE_NO_PAD.encode(self.0))
    }
}

impl fmt::Debug for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GroupId({})", &self.to_string()[..8])
    }
}

/// A unique identifier for a blob (sync payload).
///
/// UUID v4 format (16 bytes).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobId(uuid::Uuid);

impl BlobId {
    /// Create a new random BlobId.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a BlobId from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        uuid::Uuid::from_slice(bytes).ok().map(Self)
    }

    /// Get the raw bytes of this BlobId.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Get the inner UUID.
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl Default for BlobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlobId({})", self.0)
    }
}

/// A monotonically increasing cursor for ordering sync operations.
///
/// Assigned by the relay, not by clients. Cursors are more reliable
/// than timestamps because device clocks can drift.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Cursor(u64);

impl Cursor {
    /// Create a new Cursor with the given value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the numeric value of this Cursor.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Create a Cursor representing "no data yet".
    pub fn zero() -> Self {
        Self(0)
    }

    /// Increment the cursor by one.
    pub fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cursor({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_roundtrip() {
        let original = DeviceId::random();
        let bytes = original.as_bytes();
        let restored = DeviceId::from_bytes(bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn device_id_base64_display() {
        let id = DeviceId::random();
        let display = id.to_string();
        assert_eq!(display.len(), 43); // 32 bytes = 43 base64 chars (no padding)
    }

    #[test]
    fn device_id_from_invalid_length_fails() {
        assert!(DeviceId::from_bytes(&[0u8; 16]).is_none());
        assert!(DeviceId::from_bytes(&[0u8; 64]).is_none());
    }

    #[test]
    fn group_id_from_secret() {
        let secret = b"test-passphrase-for-sync-group";
        let group = GroupId::from_secret(secret);
        assert_eq!(group.as_bytes().len(), 32);
    }

    #[test]
    fn group_id_deterministic() {
        let secret = b"same-secret";
        let group1 = GroupId::from_secret(secret);
        let group2 = GroupId::from_secret(secret);
        assert_eq!(group1, group2);
    }

    #[test]
    fn group_id_different_secrets_differ() {
        let group1 = GroupId::from_secret(b"secret-1");
        let group2 = GroupId::from_secret(b"secret-2");
        assert_ne!(group1, group2);
    }

    #[test]
    fn blob_id_is_uuid_v4() {
        let id = BlobId::new();
        assert_eq!(id.as_bytes().len(), 16);
        assert_eq!(id.as_uuid().get_version_num(), 4);
    }

    #[test]
    fn blob_id_roundtrip() {
        let original = BlobId::new();
        let bytes = original.as_bytes();
        let restored = BlobId::from_bytes(bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn cursor_ordering() {
        let c1 = Cursor::new(100);
        let c2 = Cursor::new(200);
        assert!(c1 < c2);
        assert!(c2 > c1);
    }

    #[test]
    fn cursor_next() {
        let c = Cursor::new(100);
        assert_eq!(c.next().value(), 101);
    }

    #[test]
    fn cursor_zero() {
        let c = Cursor::zero();
        assert_eq!(c.value(), 0);
    }

    #[test]
    fn cursor_saturating_add() {
        let c = Cursor::new(u64::MAX);
        assert_eq!(c.next().value(), u64::MAX); // Saturates, doesn't wrap
    }
}
