//! Device pairing and invite generation for 0k-Sync.
//!
//! This module provides:
//! - Invite creation with relay NodeId, GroupId, and GroupSecret
//! - QR code payload encoding/decoding (base64 JSON)
//! - Short code format (XXXX-XXXX-XXXX-XXXX)
//! - Time-limited invites with expiration
//!
//! The invite flow:
//! 1. Device A creates a group and generates an invite
//! 2. Invite is displayed as QR code or short code
//! 3. Device B scans/enters the invite
//! 4. Both devices now share the GroupSecret for E2E encryption

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::{Zeroize, ZeroizeOnDrop};
use zerok_sync_types::GroupId;

/// Default invite TTL (10 minutes).
pub const DEFAULT_INVITE_TTL: Duration = Duration::from_secs(600);

/// Error type for pairing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingError {
    /// The invite payload is invalid (bad base64 or JSON).
    InvalidPayload(String),
    /// The short code format is invalid.
    InvalidShortCode(String),
    /// The invite has expired.
    Expired,
    /// Version mismatch.
    UnsupportedVersion(u32),
}

impl std::fmt::Display for PairingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PairingError::InvalidPayload(msg) => write!(f, "invalid invite payload: {}", msg),
            PairingError::InvalidShortCode(msg) => write!(f, "invalid short code: {}", msg),
            PairingError::Expired => write!(f, "invite has expired"),
            PairingError::UnsupportedVersion(v) => write!(f, "unsupported invite version: {}", v),
        }
    }
}

impl std::error::Error for PairingError {}

/// A 32-byte iroh NodeId (public key).
///
/// This is stored as raw bytes in sync-core. The sync-client will
/// convert to/from iroh's `NodeId` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelayNodeId([u8; 32]);

impl RelayNodeId {
    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create a random NodeId (for testing).
    #[cfg(test)]
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }
}

impl std::fmt::Display for RelayNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", URL_SAFE_NO_PAD.encode(self.0))
    }
}

/// A 32-byte symmetric secret shared by all devices in a group.
///
/// Used for:
/// - Content encryption key derivation (via HKDF)
/// - Group membership proof
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct GroupSecret([u8; 32]);

impl GroupSecret {
    /// Create a new random group secret.
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Derive the GroupId from this secret.
    pub fn derive_group_id(&self) -> GroupId {
        GroupId::from_secret(&self.0)
    }
}

// Intentionally opaque debug to avoid logging secrets
impl std::fmt::Debug for GroupSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GroupSecret([REDACTED])")
    }
}

/// An invite to join a sync group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    /// Invite format version (2 = with salt; v1 rejected).
    pub version: u32,
    /// The iroh NodeId of the relay to connect to.
    pub relay_node_id: RelayNodeId,
    /// The group identifier.
    pub group_id: GroupId,
    /// The shared secret for E2E encryption.
    pub group_secret: GroupSecret,
    /// Argon2id salt used to derive the GroupSecret from the passphrase.
    /// Required in v2 invites. Empty for legacy v1 (which are rejected).
    #[serde(default)]
    pub salt: Vec<u8>,
    /// Unix timestamp when the invite was created.
    pub created_at: u64,
    /// Unix timestamp when the invite expires.
    pub expires_at: u64,
}

impl Invite {
    /// Create a new invite with default TTL (10 minutes).
    pub fn create(
        relay_node_id: RelayNodeId,
        group_id: GroupId,
        group_secret: GroupSecret,
        salt: Vec<u8>,
    ) -> Self {
        Self::create_with_ttl(
            relay_node_id,
            group_id,
            group_secret,
            salt,
            DEFAULT_INVITE_TTL,
        )
    }

    /// Create a new invite with custom TTL.
    pub fn create_with_ttl(
        relay_node_id: RelayNodeId,
        group_id: GroupId,
        group_secret: GroupSecret,
        salt: Vec<u8>,
        ttl: Duration,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: 2,
            relay_node_id,
            group_id,
            group_secret,
            salt,
            created_at: now,
            expires_at: now + ttl.as_secs(),
        }
    }

    /// Check if the invite has expired.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.expires_at
    }

    /// Encode the invite as a base64 JSON payload for QR codes.
    ///
    /// Format: `BASE64(JSON({ version, relay_node_id, group_id, ... }))`
    pub fn to_qr_payload(&self) -> String {
        let json = serde_json::to_string(self).expect("invite serialization failed");
        URL_SAFE_NO_PAD.encode(json.as_bytes())
    }

    /// Decode an invite from a base64 JSON payload.
    ///
    /// Only version 2 invites (with salt) are accepted. Version 1 is rejected
    /// because it used a static Argon2id salt (security finding F-001).
    pub fn from_qr_payload(payload: &str) -> Result<Self, PairingError> {
        let json_bytes = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|e| PairingError::InvalidPayload(format!("base64 decode: {}", e)))?;

        let invite: Self = serde_json::from_slice(&json_bytes)
            .map_err(|e| PairingError::InvalidPayload(format!("json parse: {}", e)))?;

        if invite.version != 2 {
            return Err(PairingError::UnsupportedVersion(invite.version));
        }

        Ok(invite)
    }

    /// Encode the invite as a short code (XXXX-XXXX-XXXX-XXXX).
    ///
    /// The short code contains enough entropy for relay lookup and
    /// key derivation. The full invite data is stored server-side
    /// and retrieved using the lookup portion.
    ///
    /// Format: 16 base32 characters with dashes (19 chars total).
    pub fn to_short_code(&self) -> String {
        // Use first 10 bytes of the group secret for the short code
        // This provides 80 bits of entropy, enough for secure lookup
        let bytes = &self.group_secret.0[..10];
        let encoded = base32_encode(bytes);

        // Format as XXXX-XXXX-XXXX-XXXX
        format!(
            "{}-{}-{}-{}",
            &encoded[0..4],
            &encoded[4..8],
            &encoded[8..12],
            &encoded[12..16]
        )
    }

    /// Split a short code into lookup and decrypt portions.
    ///
    /// - Lookup (first 8 chars): Used to find the invite on the relay
    /// - Decrypt (last 8 chars): Used to decrypt the invite payload
    pub fn split_short_code(code: &str) -> Result<(String, String), PairingError> {
        // Remove dashes
        let clean: String = code.chars().filter(|c| *c != '-').collect();

        if clean.len() != 16 {
            return Err(PairingError::InvalidShortCode(format!(
                "expected 16 characters, got {}",
                clean.len()
            )));
        }

        // Verify all characters are valid base32
        if !clean.chars().all(is_base32_char) {
            return Err(PairingError::InvalidShortCode(
                "invalid characters (must be A-Z or 2-7)".into(),
            ));
        }

        let lookup = clean[..8].to_string();
        let decrypt = clean[8..].to_string();

        Ok((lookup, decrypt))
    }
}

/// Encode bytes as base32 (RFC 4648, uppercase, no padding).
fn base32_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut bits = 0u32;
    let mut bit_count = 0;

    for &byte in bytes {
        bits = (bits << 8) | (byte as u32);
        bit_count += 8;

        while bit_count >= 5 {
            bit_count -= 5;
            let index = ((bits >> bit_count) & 0x1F) as usize;
            result.push(ALPHABET[index] as char);
        }
    }

    // Handle remaining bits
    if bit_count > 0 {
        let index = ((bits << (5 - bit_count)) & 0x1F) as usize;
        result.push(ALPHABET[index] as char);
    }

    result
}

/// Check if a character is valid base32.
fn is_base32_char(c: char) -> bool {
    matches!(c, 'A'..='Z' | '2'..='7')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_relay_node_id() -> RelayNodeId {
        // Deterministic NodeId for testing
        RelayNodeId::from_bytes([0xAB; 32])
    }

    fn test_salt() -> Vec<u8> {
        b"test-salt-00000!".to_vec()
    }

    #[test]
    fn invite_roundtrip() {
        let invite = Invite::create(
            test_relay_node_id(),
            GroupId::random(),
            GroupSecret::random(),
            test_salt(),
        );

        let encoded = invite.to_qr_payload();
        let decoded = Invite::from_qr_payload(&encoded).unwrap();

        assert_eq!(invite.relay_node_id, decoded.relay_node_id);
        assert_eq!(invite.group_id, decoded.group_id);
        assert_eq!(decoded.salt, test_salt());
    }

    #[test]
    fn short_code_format() {
        let invite = Invite::create(
            test_relay_node_id(),
            GroupId::random(),
            GroupSecret::random(),
            test_salt(),
        );

        let code = invite.to_short_code();

        // Format: XXXX-XXXX-XXXX-XXXX
        assert_eq!(code.len(), 19);
        assert_eq!(&code[4..5], "-");
        assert_eq!(&code[9..10], "-");
        assert_eq!(&code[14..15], "-");
    }

    #[test]
    fn short_code_splits_correctly() {
        let code = "ABCD-EFGH-IJKL-MNOP";
        let (lookup, decrypt) = Invite::split_short_code(code).unwrap();

        assert_eq!(lookup, "ABCDEFGH");
        assert_eq!(decrypt, "IJKLMNOP");
    }

    #[test]
    fn short_code_without_dashes() {
        let code = "ABCDEFGHIJKLMNOP";
        let (lookup, decrypt) = Invite::split_short_code(code).unwrap();

        assert_eq!(lookup, "ABCDEFGH");
        assert_eq!(decrypt, "IJKLMNOP");
    }

    #[test]
    fn invite_expires() {
        let invite = Invite::create_with_ttl(
            test_relay_node_id(),
            GroupId::random(),
            GroupSecret::random(),
            test_salt(),
            Duration::from_secs(0), // Already expired
        );

        assert!(invite.is_expired());
    }

    #[test]
    fn invite_not_expired() {
        let invite = Invite::create(
            test_relay_node_id(),
            GroupId::random(),
            GroupSecret::random(),
            test_salt(),
        );

        assert!(!invite.is_expired());
    }

    #[test]
    fn invalid_short_code_length() {
        let result = Invite::split_short_code("ABCD-EFGH");
        assert!(matches!(result, Err(PairingError::InvalidShortCode(_))));
    }

    #[test]
    fn invalid_short_code_chars() {
        let result = Invite::split_short_code("ABCD-EFGH-IJKL-MN01"); // 0 and 1 are invalid
        assert!(matches!(result, Err(PairingError::InvalidShortCode(_))));
    }

    #[test]
    fn invalid_qr_payload_base64() {
        let result = Invite::from_qr_payload("not-valid-base64!!!");
        assert!(matches!(result, Err(PairingError::InvalidPayload(_))));
    }

    #[test]
    fn invalid_qr_payload_json() {
        let payload = URL_SAFE_NO_PAD.encode(b"not valid json");
        let result = Invite::from_qr_payload(&payload);
        assert!(matches!(result, Err(PairingError::InvalidPayload(_))));
    }

    #[test]
    fn group_secret_derives_group_id() {
        let secret = GroupSecret::random();
        let group_id = secret.derive_group_id();

        // Verify it's deterministic
        let group_id2 = secret.derive_group_id();
        assert_eq!(group_id, group_id2);
    }

    #[test]
    fn group_secret_debug_is_redacted() {
        let secret = GroupSecret::random();
        let debug = format!("{:?}", secret);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains(&format!("{:?}", secret.0)));
    }

    #[test]
    fn relay_node_id_display_is_base64() {
        let node_id = RelayNodeId::from_bytes([0xAB; 32]);
        let display = node_id.to_string();
        // 32 bytes = 43 base64 chars (URL-safe, no padding)
        assert_eq!(display.len(), 43);
    }

    #[test]
    fn base32_encoding_works() {
        // Test vector from RFC 4648
        assert_eq!(base32_encode(b""), "");
        assert_eq!(base32_encode(b"f"), "MY");
        assert_eq!(base32_encode(b"fo"), "MZXQ");
        assert_eq!(base32_encode(b"foo"), "MZXW6");
        assert_eq!(base32_encode(b"foob"), "MZXW6YQ");
        assert_eq!(base32_encode(b"fooba"), "MZXW6YTB");
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn short_code_is_deterministic() {
        let secret = GroupSecret::from_bytes([0x42; 32]);
        let invite1 = Invite::create(
            test_relay_node_id(),
            GroupId::from_secret(&[0x42; 32]),
            secret.clone(),
            test_salt(),
        );
        let invite2 = Invite::create(
            test_relay_node_id(),
            GroupId::from_secret(&[0x42; 32]),
            secret,
            test_salt(),
        );

        assert_eq!(invite1.to_short_code(), invite2.to_short_code());
    }

    #[test]
    fn reject_invite_v1() {
        // Create a v1-style invite (no salt) by constructing and then modifying
        let mut invite = Invite::create(
            test_relay_node_id(),
            GroupId::random(),
            GroupSecret::random(),
            test_salt(),
        );
        invite.version = 1; // Force v1

        let encoded = invite.to_qr_payload();
        let result = Invite::from_qr_payload(&encoded);
        assert!(matches!(result, Err(PairingError::UnsupportedVersion(1))));
    }

    #[test]
    fn invite_v2_includes_salt() {
        let salt = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let invite = Invite::create(
            test_relay_node_id(),
            GroupId::random(),
            GroupSecret::random(),
            salt.clone(),
        );
        assert_eq!(invite.version, 2);
        assert_eq!(invite.salt, salt);

        // Roundtrip preserves salt
        let encoded = invite.to_qr_payload();
        let decoded = Invite::from_qr_payload(&encoded).unwrap();
        assert_eq!(decoded.salt, salt);
        assert_eq!(decoded.version, 2);
    }
}
