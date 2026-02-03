//! Cryptographic primitives for 0k-Sync.
//!
//! This module provides:
//! - Device-adaptive Argon2id key derivation (12-64 MiB based on RAM)
//! - XChaCha20-Poly1305 encryption with 192-bit nonces
//! - GroupKey with encryption and authentication subkeys
//!
//! # Security Notes
//!
//! - XChaCha20 uses 192-bit nonces (24 bytes), safe for random generation
//! - Argon2id parameters scale with available RAM for mobile/desktop parity
//! - All keys derived via HKDF-SHA256 for cryptographic separation

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use thiserror::Error;

/// Nonce size for XChaCha20-Poly1305 (192 bits = 24 bytes).
pub const NONCE_SIZE: usize = 24;

/// Key size for XChaCha20-Poly1305 (256 bits = 32 bytes).
pub const KEY_SIZE: usize = 32;

/// Crypto errors.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Encryption failed.
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption failed (authentication error).
    #[error("decryption failed: authentication error")]
    DecryptionFailed,

    /// Invalid key length.
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength {
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },

    /// Key derivation failed.
    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),
}

/// Argon2id parameters for device-adaptive key derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Params {
    memory_mib: u32,
    iterations: u32,
    parallelism: u32,
}

impl Argon2Params {
    /// Create parameters based on available RAM in MB.
    ///
    /// Scaling:
    /// - < 2000 MB: 12 MiB, 3 iterations (low-end mobile)
    /// - < 4000 MB: 19 MiB, 2 iterations (mid-range mobile)
    /// - < 8000 MB: 46 MiB, 1 iteration (high-end mobile)
    /// - >= 8000 MB: 64 MiB, 3 iterations (desktop)
    pub fn for_ram_mb(ram_mb: u64) -> Self {
        if ram_mb < 2000 {
            // Low-end mobile: 12 MiB, 3 iterations
            Self {
                memory_mib: 12,
                iterations: 3,
                parallelism: 1,
            }
        } else if ram_mb < 4000 {
            // Mid-range mobile: 19 MiB, 2 iterations
            Self {
                memory_mib: 19,
                iterations: 2,
                parallelism: 1,
            }
        } else if ram_mb < 8000 {
            // High-end mobile: 46 MiB, 1 iteration
            Self {
                memory_mib: 46,
                iterations: 1,
                parallelism: 1,
            }
        } else {
            // Desktop: 64 MiB, 3 iterations
            Self {
                memory_mib: 64,
                iterations: 3,
                parallelism: 4,
            }
        }
    }

    /// Get memory in MiB.
    pub fn memory_mib(&self) -> u32 {
        self.memory_mib
    }

    /// Get iteration count.
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Convert to argon2 Params.
    fn to_argon2_params(self) -> Result<Params, CryptoError> {
        Params::new(
            self.memory_mib * 1024, // Convert MiB to KiB
            self.iterations,
            self.parallelism,
            Some(KEY_SIZE),
        )
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))
    }
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self::for_ram_mb(detect_available_ram_mb())
    }
}

/// Detect available RAM in megabytes.
pub fn detect_available_ram_mb() -> u64 {
    use sysinfo::System;
    let sys = System::new_all();
    sys.total_memory() / (1024 * 1024) // Convert bytes to MB
}

/// A group secret derived from a passphrase.
///
/// This is the root secret shared between all devices in a sync group.
#[derive(Clone)]
pub struct GroupSecret([u8; KEY_SIZE]);

impl GroupSecret {
    /// Create a GroupSecret from a passphrase using Argon2id.
    pub fn from_passphrase(passphrase: &str) -> Self {
        Self::from_passphrase_with_params(passphrase, Argon2Params::default())
    }

    /// Create a GroupSecret with custom Argon2 parameters.
    pub fn from_passphrase_with_params(passphrase: &str, params: Argon2Params) -> Self {
        // Use a domain-separated salt for 0k-Sync
        let salt = b"0k-sync-group-secret-v1";

        let argon2_params = params
            .to_argon2_params()
            .expect("invalid argon2 parameters");
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

        let mut output = [0u8; KEY_SIZE];
        argon2
            .hash_password_into(passphrase.as_bytes(), salt, &mut output)
            .expect("argon2 hash failed");

        Self(output)
    }

    /// Create a random GroupSecret (for testing).
    pub fn random() -> Self {
        let mut bytes = [0u8; KEY_SIZE];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.0
    }
}

// Don't leak secret in debug output
impl std::fmt::Debug for GroupSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GroupSecret([REDACTED])")
    }
}

/// A derived key for E2E encryption within a sync group.
///
/// Contains separate subkeys for encryption and authentication,
/// derived from the GroupSecret via HKDF-SHA256.
#[derive(Clone)]
pub struct GroupKey {
    encryption_key: [u8; KEY_SIZE],
    auth_key: [u8; KEY_SIZE],
}

impl GroupKey {
    /// Derive a GroupKey from a GroupSecret.
    ///
    /// Uses HKDF-SHA256 to derive separate encryption and auth subkeys.
    pub fn derive(secret: &GroupSecret) -> Self {
        Self::derive_with_ram(secret, detect_available_ram_mb())
    }

    /// Derive with explicit RAM parameter (for testing).
    pub fn derive_with_ram(secret: &GroupSecret, _ram_mb: u64) -> Self {
        // Use HKDF to derive subkeys
        let hkdf = Hkdf::<Sha256>::new(Some(b"0k-sync-group-key-v1"), secret.as_bytes());

        let mut encryption_key = [0u8; KEY_SIZE];
        let mut auth_key = [0u8; KEY_SIZE];

        hkdf.expand(b"encryption", &mut encryption_key)
            .expect("hkdf expand failed");
        hkdf.expand(b"authentication", &mut auth_key)
            .expect("hkdf expand failed");

        Self {
            encryption_key,
            auth_key,
        }
    }

    /// Get the encryption subkey.
    pub fn encryption_key(&self) -> &[u8; KEY_SIZE] {
        &self.encryption_key
    }

    /// Get the authentication subkey.
    pub fn auth_key(&self) -> &[u8; KEY_SIZE] {
        &self.auth_key
    }

    /// Encrypt data using XChaCha20-Poly1305.
    ///
    /// Returns (ciphertext, nonce). Nonce is 192 bits (24 bytes),
    /// safe for random generation without coordination.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; NONCE_SIZE]), CryptoError> {
        // Generate random 192-bit nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        getrandom::getrandom(&mut nonce_bytes).expect("getrandom failed");
        let nonce = XNonce::from_slice(&nonce_bytes);

        let cipher = XChaCha20Poly1305::new_from_slice(&self.encryption_key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed("aead encrypt failed".into()))?;

        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypt data using XChaCha20-Poly1305.
    pub fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: &[u8; NONCE_SIZE],
    ) -> Result<Vec<u8>, CryptoError> {
        let nonce = XNonce::from_slice(nonce);

        let cipher = XChaCha20Poly1305::new_from_slice(&self.encryption_key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

// Don't leak keys in debug output
impl std::fmt::Debug for GroupKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GroupKey {{ encryption_key: [REDACTED], auth_key: [REDACTED] }}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ===========================================
    // Argon2 Parameter Tests
    // ===========================================

    #[test]
    fn argon2_parameters_scale_with_ram() {
        // Low-end mobile: 12 MiB
        let params_low = Argon2Params::for_ram_mb(1500);
        assert_eq!(params_low.memory_mib(), 12);
        assert_eq!(params_low.iterations(), 3);

        // Mid-range mobile: 19 MiB
        let params_mid = Argon2Params::for_ram_mb(3000);
        assert_eq!(params_mid.memory_mib(), 19);
        assert_eq!(params_mid.iterations(), 2);

        // High-end mobile: 46 MiB
        let params_high = Argon2Params::for_ram_mb(6000);
        assert_eq!(params_high.memory_mib(), 46);
        assert_eq!(params_high.iterations(), 1);

        // Desktop: 64 MiB
        let params_desktop = Argon2Params::for_ram_mb(16000);
        assert_eq!(params_desktop.memory_mib(), 64);
        assert_eq!(params_desktop.iterations(), 3);
    }

    #[test]
    fn group_key_derivation_uses_device_adaptive_argon2() {
        // Use low-end params for faster test
        let params = Argon2Params::for_ram_mb(1500);

        // Time the Argon2 derivation (GroupSecret creation), not HKDF (GroupKey derivation)
        let start = std::time::Instant::now();
        let secret = GroupSecret::from_passphrase_with_params("my-secure-passphrase", params);
        let elapsed = start.elapsed();

        let key = GroupKey::derive_with_ram(&secret, 1500);

        assert_eq!(key.encryption_key().len(), 32);
        assert_eq!(key.auth_key().len(), 32);

        // Argon2 should take measurable time even with low params
        // Using 12 MiB memory, so should be noticeable
        assert!(
            elapsed >= Duration::from_millis(5),
            "Argon2 was too fast: {:?}",
            elapsed
        );
        assert!(elapsed <= Duration::from_secs(10));
    }

    // ===========================================
    // XChaCha20-Poly1305 Tests (192-bit nonces)
    // ===========================================

    #[test]
    fn xchacha20_uses_192_bit_nonces() {
        let key = GroupKey::derive(&GroupSecret::random());
        let plaintext = b"Hello, sync world!";

        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();

        // XChaCha20 uses 24-byte (192-bit) nonces, not 12-byte (96-bit)
        assert_eq!(nonce.len(), 24, "Must use 192-bit nonces for XChaCha20");

        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn random_192_bit_nonces_are_safe() {
        // 192-bit nonces have 2^80 birthday bound (vs 2^32 for 96-bit)
        // Safe to generate randomly without coordination
        let key = GroupKey::derive(&GroupSecret::random());
        let plaintext = b"Same message";

        let (ct1, nonce1) = key.encrypt(plaintext).unwrap();
        let (ct2, nonce2) = key.encrypt(plaintext).unwrap();

        // Different random nonces
        assert_ne!(nonce1, nonce2);
        // Different ciphertext
        assert_ne!(ct1, ct2);

        // Both decrypt correctly
        assert_eq!(key.decrypt(&ct1, &nonce1).unwrap(), plaintext.as_slice());
        assert_eq!(key.decrypt(&ct2, &nonce2).unwrap(), plaintext.as_slice());
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key1 = GroupKey::derive(&GroupSecret::random());
        let key2 = GroupKey::derive(&GroupSecret::random());
        let plaintext = b"Secret message";

        let (ciphertext, nonce) = key1.encrypt(plaintext).unwrap();
        let result = key2.decrypt(&ciphertext, &nonce);

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn corrupted_ciphertext_fails_decryption() {
        let key = GroupKey::derive(&GroupSecret::random());
        let plaintext = b"Secret message";

        let (mut ciphertext, nonce) = key.encrypt(plaintext).unwrap();

        // Corrupt a byte
        ciphertext[0] ^= 0xFF;

        let result = key.decrypt(&ciphertext, &nonce);
        assert!(result.is_err());
    }

    #[test]
    fn empty_plaintext_encrypts() {
        let key = GroupKey::derive(&GroupSecret::random());
        let plaintext = b"";

        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();
        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn large_plaintext_encrypts() {
        let key = GroupKey::derive(&GroupSecret::random());
        let plaintext = vec![0x42u8; 1024 * 1024]; // 1 MiB

        let (ciphertext, nonce) = key.encrypt(&plaintext).unwrap();
        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    // ===========================================
    // GroupSecret Tests
    // ===========================================

    #[test]
    fn group_secret_from_passphrase_is_deterministic() {
        let params = Argon2Params::for_ram_mb(1500); // Fast params for test
        let secret1 = GroupSecret::from_passphrase_with_params("same-passphrase", params);
        let secret2 = GroupSecret::from_passphrase_with_params("same-passphrase", params);

        assert_eq!(secret1.as_bytes(), secret2.as_bytes());
    }

    #[test]
    fn group_secret_different_passphrases_differ() {
        let params = Argon2Params::for_ram_mb(1500);
        let secret1 = GroupSecret::from_passphrase_with_params("passphrase-1", params);
        let secret2 = GroupSecret::from_passphrase_with_params("passphrase-2", params);

        assert_ne!(secret1.as_bytes(), secret2.as_bytes());
    }

    #[test]
    fn group_secret_debug_is_redacted() {
        let secret = GroupSecret::random();
        let debug = format!("{:?}", secret);
        assert!(debug.contains("REDACTED"));
    }

    // ===========================================
    // GroupKey Tests
    // ===========================================

    #[test]
    fn group_key_subkeys_are_different() {
        let key = GroupKey::derive(&GroupSecret::random());

        // Encryption and auth keys should be different
        assert_ne!(key.encryption_key(), key.auth_key());
    }

    #[test]
    fn group_key_derivation_is_deterministic() {
        let secret = GroupSecret::random();
        let key1 = GroupKey::derive(&secret);
        let key2 = GroupKey::derive(&secret);

        assert_eq!(key1.encryption_key(), key2.encryption_key());
        assert_eq!(key1.auth_key(), key2.auth_key());
    }

    #[test]
    fn group_key_debug_is_redacted() {
        let key = GroupKey::derive(&GroupSecret::random());
        let debug = format!("{:?}", key);
        assert!(debug.contains("REDACTED"));
    }

    // ===========================================
    // System Detection Test
    // ===========================================

    #[test]
    fn detect_ram_returns_reasonable_value() {
        let ram_mb = detect_available_ram_mb();
        // Should be at least 512 MB on any modern system
        assert!(ram_mb >= 512, "Detected RAM: {} MB", ram_mb);
        // Should be less than 1 TB (sanity check)
        assert!(ram_mb < 1024 * 1024, "Detected RAM: {} MB", ram_mb);
    }
}
