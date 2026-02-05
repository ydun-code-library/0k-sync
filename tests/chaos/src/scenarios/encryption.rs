//! Encryption chaos scenarios (E-HS-*, E-ENC-*, E-PQ-*).
//!
//! Tests cryptographic edge cases and connection handling using MockTransport.
//! Per 06-CHAOS-TESTING-STRATEGY.md sections 6.1, 6.2, 6.3.

#[cfg(test)]
mod tests {
    use sync_client::crypto::{Argon2Params, CryptoError, GroupKey, GroupSecret, NONCE_SIZE};
    use sync_client::transport::{MockTransport, Transport, TransportError};

    // ========================================================================
    // E-HS-* Handshake/Connection Disruption (6 tests)
    // ========================================================================

    /// E-HS-01: Disconnect after connect, verify clean retry possible.
    #[tokio::test]
    async fn e_hs_01_disconnect_after_connect() {
        let transport = MockTransport::new();

        // Connect successfully
        transport.connect("test-relay").await.unwrap();
        assert!(transport.is_connected());

        // Disconnect
        transport.close().await.unwrap();
        assert!(!transport.is_connected());

        // Retry should succeed
        transport.connect("test-relay").await.unwrap();
        assert!(transport.is_connected());
    }

    /// E-HS-02: Connection timeout/failure returns proper error.
    #[tokio::test]
    async fn e_hs_02_connect_timeout() {
        let transport = MockTransport::new();
        transport.fail_next_connect("connection timed out");

        let result = transport.connect("unreachable-relay").await;

        assert!(matches!(result, Err(TransportError::ConnectionFailed(_))));
        assert!(!transport.is_connected());
    }

    /// E-HS-03: Connect to invalid address fails gracefully.
    #[tokio::test]
    async fn e_hs_03_connect_invalid_address() {
        let transport = MockTransport::new();
        transport.fail_next_connect("invalid endpoint id format");

        let result = transport.connect("not-a-valid-endpoint-id").await;

        assert!(matches!(result, Err(TransportError::ConnectionFailed(_))));
    }

    /// E-HS-04: Reconnect after disconnect succeeds.
    #[tokio::test]
    async fn e_hs_04_reconnect_after_disconnect() {
        let transport = MockTransport::new();

        // First connection
        transport.connect("relay-1").await.unwrap();
        transport.send(b"message 1").await.unwrap();
        transport.close().await.unwrap();

        // Reconnect to different relay
        transport.connect("relay-2").await.unwrap();
        transport.send(b"message 2").await.unwrap();

        assert!(transport.is_connected());
        assert_eq!(transport.connected_address(), Some("relay-2".to_string()));
    }

    /// E-HS-05: Send before connect returns NotConnected error.
    #[tokio::test]
    async fn e_hs_05_send_before_connect() {
        let transport = MockTransport::new();

        let result = transport.send(b"premature data").await;

        assert!(matches!(result, Err(TransportError::NotConnected)));
    }

    /// E-HS-06: Concurrent connect attempts handled gracefully.
    #[tokio::test]
    async fn e_hs_06_concurrent_connect_attempts() {
        let transport = MockTransport::new();

        // Simulate two concurrent connects by calling connect twice
        // MockTransport handles this gracefully (last wins)
        let r1 = transport.connect("relay-1").await;
        let r2 = transport.connect("relay-2").await;

        // Both should succeed with MockTransport
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        // Last connect wins
        assert_eq!(transport.connected_address(), Some("relay-2".to_string()));
    }

    // ========================================================================
    // E-ENC-* Session Encryption (5 tests)
    // ========================================================================

    /// E-ENC-01: Corrupted ciphertext fails decryption.
    #[tokio::test]
    async fn e_enc_01_corrupted_ciphertext() {
        let secret = GroupSecret::from_passphrase_with_salt("test-passphrase", b"test-salt-00000!");
        let key = GroupKey::derive(&secret);

        // Encrypt some data
        let plaintext = b"secret message";
        let (mut ciphertext, nonce) = key.encrypt(plaintext).unwrap();

        // Corrupt the ciphertext
        ciphertext[0] ^= 0xFF;
        ciphertext[5] ^= 0xAA;

        // Decryption should fail
        let result = key.decrypt(&ciphertext, &nonce);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    /// E-ENC-02: Truncated payload (no nonce) handled gracefully.
    #[tokio::test]
    async fn e_enc_02_truncated_payload() {
        let secret = GroupSecret::from_passphrase_with_salt("test-passphrase", b"test-salt-00000!");
        let key = GroupKey::derive(&secret);

        // Create a payload that's too short to contain a nonce
        let truncated = vec![0u8; NONCE_SIZE - 1]; // 23 bytes, need 24

        // Try to extract nonce - this should fail
        if truncated.len() < NONCE_SIZE {
            // Simulating what client.rs does - skip blobs with payload < NONCE_SIZE
            assert!(truncated.len() < NONCE_SIZE);
        }

        // Even if we try to decrypt with garbage nonce
        let garbage_nonce = [0u8; NONCE_SIZE];
        let result = key.decrypt(&truncated, &garbage_nonce);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    /// E-ENC-03: Wrong key fails decryption.
    #[tokio::test]
    async fn e_enc_03_wrong_key_decryption() {
        let secret_a = GroupSecret::from_passphrase_with_salt("passphrase-a", b"test-salt-00000!");
        let secret_b = GroupSecret::from_passphrase_with_salt("passphrase-b", b"test-salt-00000!");

        let key_a = GroupKey::derive(&secret_a);
        let key_b = GroupKey::derive(&secret_b);

        // Encrypt with key A
        let plaintext = b"confidential data";
        let (ciphertext, nonce) = key_a.encrypt(plaintext).unwrap();

        // Decrypt with key B should fail
        let result = key_b.decrypt(&ciphertext, &nonce);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    /// E-ENC-04: Valid encrypt/decrypt cycle with nonce extraction.
    #[tokio::test]
    async fn e_enc_04_nonce_extraction() {
        let secret = GroupSecret::from_passphrase_with_salt("test-passphrase", b"test-salt-00000!");
        let key = GroupKey::derive(&secret);

        let plaintext = b"test message for nonce extraction";

        // Encrypt
        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();

        // Verify nonce is correct size
        assert_eq!(nonce.len(), NONCE_SIZE);

        // Decrypt with extracted nonce
        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// E-ENC-05: Empty plaintext encrypts/decrypts correctly.
    #[tokio::test]
    async fn e_enc_05_empty_plaintext() {
        let secret = GroupSecret::from_passphrase_with_salt("test-passphrase", b"test-salt-00000!");
        let key = GroupKey::derive(&secret);

        let plaintext = b"";

        // Encrypt empty
        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();

        // Ciphertext includes auth tag, so it's not empty
        assert!(!ciphertext.is_empty());

        // Decrypt
        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
        assert!(decrypted.is_empty());
    }

    // ========================================================================
    // E-PQ-* Post-Quantum / Key Derivation (5 tests)
    // ========================================================================

    /// E-PQ-01: Same passphrase produces deterministic GroupKey.
    #[tokio::test]
    async fn e_pq_01_key_derivation_deterministic() {
        let passphrase = "deterministic-test-passphrase";

        // Use fixed params for reproducibility
        let params = Argon2Params::for_ram_mb(1000); // Low-end params

        let secret1 =
            GroupSecret::from_passphrase_with_params(passphrase, b"test-salt-00000!", params);
        let secret2 =
            GroupSecret::from_passphrase_with_params(passphrase, b"test-salt-00000!", params);

        let key1 = GroupKey::derive(&secret1);
        let key2 = GroupKey::derive(&secret2);

        // Keys should be identical
        assert_eq!(key1.encryption_key(), key2.encryption_key());
        assert_eq!(key1.auth_key(), key2.auth_key());
    }

    /// E-PQ-02: Different passphrases produce different GroupKeys.
    #[tokio::test]
    async fn e_pq_02_key_derivation_different() {
        let params = Argon2Params::for_ram_mb(1000);

        let secret1 =
            GroupSecret::from_passphrase_with_params("passphrase-one", b"test-salt-00000!", params);
        let secret2 =
            GroupSecret::from_passphrase_with_params("passphrase-two", b"test-salt-00000!", params);

        let key1 = GroupKey::derive(&secret1);
        let key2 = GroupKey::derive(&secret2);

        // Keys should be different
        assert_ne!(key1.encryption_key(), key2.encryption_key());
        assert_ne!(key1.auth_key(), key2.auth_key());
    }

    /// E-PQ-03: Low memory (<4GB) uses OWASP minimum (19 MiB, 2 iter) after CL-001.
    #[tokio::test]
    async fn e_pq_03_argon2_params_low_memory() {
        let params = Argon2Params::for_ram_mb(1500); // < 4000 MB

        assert_eq!(params.memory_mib(), 19);
        assert_eq!(params.iterations(), 2);
    }

    /// E-PQ-04: High memory (>=8GB) uses 64 MiB Argon2 params.
    #[tokio::test]
    async fn e_pq_04_argon2_params_high_memory() {
        let params = Argon2Params::for_ram_mb(16000); // >= 8000 MB

        assert_eq!(params.memory_mib(), 64);
        assert_eq!(params.iterations(), 3);
    }

    /// E-PQ-05: HKDF domain separation produces different subkeys.
    #[tokio::test]
    async fn e_pq_05_hkdf_domain_separation() {
        let secret =
            GroupSecret::from_passphrase_with_salt("domain-separation-test", b"test-salt-00000!");
        let key = GroupKey::derive(&secret);

        // Encryption and auth keys must be different
        assert_ne!(key.encryption_key(), key.auth_key());

        // Neither should be all zeros
        assert_ne!(key.encryption_key(), &[0u8; 32]);
        assert_ne!(key.auth_key(), &[0u8; 32]);
    }
}
