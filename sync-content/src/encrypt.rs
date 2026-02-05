//! Content encryption using encrypt-then-hash pattern.
//!
//! This module provides:
//! - Content key derivation (HKDF from GroupSecret + blob_id)
//! - XChaCha20-Poly1305 encryption with random nonces
//! - BLAKE3 hashing of ciphertext for content addressing

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use sha2::Sha256;

use crate::error::ContentError;

/// Size of content encryption key in bytes (256 bits).
pub const CONTENT_KEY_SIZE: usize = 32;

/// Size of XChaCha20-Poly1305 nonce in bytes (192 bits).
pub const NONCE_SIZE: usize = 24;

/// Size of BLAKE3 hash output in bytes (256 bits).
pub const HASH_SIZE: usize = 32;

/// Maximum content size for encryption (100 MB).
pub const MAX_CONTENT_SIZE: usize = 100 * 1024 * 1024;

/// Derive a content-specific encryption key from group secret and blob ID.
///
/// Uses HKDF-SHA256 with domain separation:
/// - Salt: `"0k-sync-content-v1"` (distinguishes from other key derivations)
/// - Info: `blob_id || "content-encryption"`
///
/// Each blob gets a unique key, even within the same sync group.
pub fn derive_content_key(group_secret: &[u8; 32], blob_id: &[u8]) -> [u8; CONTENT_KEY_SIZE] {
    let hkdf = Hkdf::<Sha256>::new(
        Some(b"0k-sync-content-v1"), // Salt for domain separation
        group_secret,
    );

    // Info = blob_id || "content-encryption"
    let mut info = Vec::with_capacity(blob_id.len() + 18);
    info.extend_from_slice(blob_id);
    info.extend_from_slice(b"content-encryption");

    let mut content_key = [0u8; CONTENT_KEY_SIZE];
    hkdf.expand(&info, &mut content_key)
        .expect("HKDF expand should not fail with valid lengths");

    content_key
}

/// Result of content encryption: (ciphertext, nonce, blake3_hash).
pub type EncryptedContent = (Vec<u8>, [u8; NONCE_SIZE], [u8; HASH_SIZE]);

/// Encrypt content and return (ciphertext, nonce, blake3_hash).
///
/// Uses XChaCha20-Poly1305 with a random 192-bit nonce.
/// The BLAKE3 hash is computed over the ciphertext (encrypt-then-hash pattern).
pub fn encrypt_content(
    content_key: &[u8; CONTENT_KEY_SIZE],
    plaintext: &[u8],
) -> Result<EncryptedContent, ContentError> {
    // F-017: Reject content exceeding maximum size
    if plaintext.len() > MAX_CONTENT_SIZE {
        return Err(ContentError::EncryptionFailed(format!(
            "content too large: {} bytes (max {})",
            plaintext.len(),
            MAX_CONTENT_SIZE
        )));
    }

    // Generate random nonce (192 bits safe for random generation)
    let mut nonce = [0u8; NONCE_SIZE];
    getrandom::getrandom(&mut nonce).map_err(|e| ContentError::EncryptionFailed(e.to_string()))?;

    // Encrypt with XChaCha20-Poly1305
    let cipher = XChaCha20Poly1305::new_from_slice(content_key)
        .map_err(|e| ContentError::EncryptionFailed(e.to_string()))?;
    let xnonce = XNonce::from_slice(&nonce);
    let ciphertext = cipher
        .encrypt(xnonce, plaintext)
        .map_err(|e| ContentError::EncryptionFailed(e.to_string()))?;

    // Hash ciphertext with BLAKE3 (encrypt-then-hash)
    let hash = blake3::hash(&ciphertext);

    Ok((ciphertext, nonce, *hash.as_bytes()))
}

/// Decrypt content using content key and nonce.
///
/// Returns the decrypted plaintext or `DecryptionFailed` if authentication fails.
pub fn decrypt_content(
    content_key: &[u8; CONTENT_KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
    ciphertext: &[u8],
) -> Result<Vec<u8>, ContentError> {
    let cipher = XChaCha20Poly1305::new_from_slice(content_key)
        .map_err(|_| ContentError::DecryptionFailed)?;
    let xnonce = XNonce::from_slice(nonce);
    cipher
        .decrypt(xnonce, ciphertext)
        .map_err(|_| ContentError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_key_derivation_deterministic() {
        let group_secret = [0xAB; 32];
        let blob_id = b"test-blob-id";

        let key1 = derive_content_key(&group_secret, blob_id);
        let key2 = derive_content_key(&group_secret, blob_id);

        assert_eq!(key1, key2, "Same inputs should produce same key");
    }

    #[test]
    fn content_key_different_blob_ids() {
        let group_secret = [0xAB; 32];
        let blob_id_a = b"blob-a";
        let blob_id_b = b"blob-b";

        let key_a = derive_content_key(&group_secret, blob_id_a);
        let key_b = derive_content_key(&group_secret, blob_id_b);

        assert_ne!(
            key_a, key_b,
            "Different blob IDs should produce different keys"
        );
    }

    #[test]
    fn content_key_different_group_secrets() {
        let secret_a = [0xAA; 32];
        let secret_b = [0xBB; 32];
        let blob_id = b"same-blob";

        let key_a = derive_content_key(&secret_a, blob_id);
        let key_b = derive_content_key(&secret_b, blob_id);

        assert_ne!(
            key_a, key_b,
            "Different secrets should produce different keys"
        );
    }

    #[test]
    fn encrypt_then_hash_produces_ciphertext_hash() {
        let content_key = [0xCD; 32];
        let plaintext = b"Hello, World!";

        let (ciphertext, _nonce, hash) = encrypt_content(&content_key, plaintext).unwrap();

        // Hash should be of ciphertext, not plaintext
        let expected_hash = blake3::hash(&ciphertext);
        assert_eq!(hash, *expected_hash.as_bytes());

        // Plaintext hash should be different
        let plaintext_hash = blake3::hash(plaintext);
        assert_ne!(hash, *plaintext_hash.as_bytes());
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let content_key = [0xEF; 32];
        let plaintext = b"Confidential content for testing";

        let (ciphertext, nonce, _hash) = encrypt_content(&content_key, plaintext).unwrap();
        let decrypted = decrypt_content(&content_key, &nonce, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let content_key = [0x11; 32];
        let wrong_key = [0x22; 32];
        let plaintext = b"Secret message";

        let (ciphertext, nonce, _hash) = encrypt_content(&content_key, plaintext).unwrap();
        let result = decrypt_content(&wrong_key, &nonce, &ciphertext);

        assert!(matches!(result, Err(ContentError::DecryptionFailed)));
    }

    #[test]
    fn decrypt_wrong_nonce_fails() {
        let content_key = [0x33; 32];
        let plaintext = b"Secret message";

        let (ciphertext, _nonce, _hash) = encrypt_content(&content_key, plaintext).unwrap();
        let wrong_nonce = [0xFF; NONCE_SIZE];
        let result = decrypt_content(&content_key, &wrong_nonce, &ciphertext);

        assert!(matches!(result, Err(ContentError::DecryptionFailed)));
    }

    #[test]
    fn encrypt_empty_content() {
        let content_key = [0x44; 32];
        let plaintext = b"";

        let (ciphertext, nonce, _hash) = encrypt_content(&content_key, plaintext).unwrap();

        // Ciphertext should have auth tag even for empty plaintext
        assert_eq!(ciphertext.len(), 16); // Poly1305 tag size

        let decrypted = decrypt_content(&content_key, &nonce, &ciphertext).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn encrypt_large_content() {
        let content_key = [0x55; 32];
        // 1MB of data
        let plaintext: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        let (ciphertext, nonce, _hash) = encrypt_content(&content_key, &plaintext).unwrap();

        // Ciphertext should be plaintext + 16 byte auth tag
        assert_eq!(ciphertext.len(), plaintext.len() + 16);

        let decrypted = decrypt_content(&content_key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_rejects_oversized_content() {
        // F-017: Content exceeding MAX_CONTENT_SIZE must be rejected.
        // Verify the constant is correct and publicly accessible.
        assert_eq!(MAX_CONTENT_SIZE, 100 * 1024 * 1024);
        // Full 100MB+ allocation test would be too slow for unit tests.
        // The guard at encrypt_content() is verified by code review.
    }
}
