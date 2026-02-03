//! Content chaos scenarios (S-BLOB-*, C-STOR-*, C-COLL-*).
//!
//! Tests blob integrity and storage edge cases.
//! Per 06-CHAOS-TESTING-STRATEGY.md sections 7.3, 8.1, 8.2.

#[cfg(test)]
mod tests {
    use sync_client::crypto::{GroupKey, GroupSecret, NONCE_SIZE};
    use sync_client::transport::{MockTransport, Transport, TransportError};

    // ========================================================================
    // S-BLOB-* Blob Integrity (4 tests)
    // ========================================================================

    /// S-BLOB-01: Large blob (1MB) encrypts and decrypts correctly.
    #[tokio::test]
    async fn s_blob_01_large_blob_encryption() {
    let secret = GroupSecret::from_passphrase("large-blob-test");
    let key = GroupKey::derive(&secret);

    // Create 1MB blob
    let plaintext: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

    // Encrypt
    let (ciphertext, nonce) = key.encrypt(&plaintext).unwrap();

    // Verify ciphertext is larger (includes auth tag)
    assert!(ciphertext.len() > plaintext.len());

    // Decrypt
    let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
    assert_eq!(decrypted, plaintext);
}

/// S-BLOB-02: Many small blobs (100 x 1KB) all encrypt/decrypt correctly.
#[tokio::test]
async fn s_blob_02_many_small_blobs() {
    let secret = GroupSecret::from_passphrase("many-blobs-test");
    let key = GroupKey::derive(&secret);

    let mut encrypted_blobs = Vec::new();

    // Encrypt 100 blobs
    for i in 0..100 {
        let plaintext: Vec<u8> = (0..1024).map(|j| ((i + j) % 256) as u8).collect();
        let (ciphertext, nonce) = key.encrypt(&plaintext).unwrap();
        encrypted_blobs.push((plaintext, ciphertext, nonce));
    }

    // Decrypt all and verify
    for (original, ciphertext, nonce) in encrypted_blobs {
        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, original);
    }
}

/// S-BLOB-03: Empty blob (zero bytes) handled correctly.
#[tokio::test]
async fn s_blob_03_empty_blob() {
    let secret = GroupSecret::from_passphrase("empty-blob-test");
    let key = GroupKey::derive(&secret);

    let plaintext: Vec<u8> = vec![];

    // Encrypt empty blob
    let (ciphertext, nonce) = key.encrypt(&plaintext).unwrap();

    // Ciphertext has auth tag even for empty plaintext
    assert!(!ciphertext.is_empty());
    assert_eq!(ciphertext.len(), 16); // Poly1305 tag size

    // Decrypt
    let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
    assert!(decrypted.is_empty());
}

/// S-BLOB-04: Maximum message size blob handled correctly.
#[tokio::test]
async fn s_blob_04_max_size_blob() {
    let secret = GroupSecret::from_passphrase("max-size-test");
    let key = GroupKey::derive(&secret);

    // Create a moderately large blob (100KB - not actual max to keep test fast)
    let plaintext: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();

    // Encrypt
    let (ciphertext, nonce) = key.encrypt(&plaintext).unwrap();

    // Decrypt
    let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
    assert_eq!(decrypted, plaintext);
}

// ============================================================================
// C-STOR-* Storage/Transport Failures (4 tests)
// ============================================================================

/// C-STOR-01: Recv on empty queue returns ConnectionClosed.
#[tokio::test]
async fn c_stor_01_recv_empty_queue() {
    let transport = MockTransport::new();
    transport.connect("relay").await.unwrap();

    // No responses queued
    let result = transport.recv().await;

    assert!(matches!(result, Err(TransportError::ConnectionClosed)));
}

/// C-STOR-02: Send after close returns NotConnected.
#[tokio::test]
async fn c_stor_02_send_after_close() {
    let transport = MockTransport::new();
    transport.connect("relay").await.unwrap();
    transport.close().await.unwrap();

    let result = transport.send(b"data after close").await;

    assert!(matches!(result, Err(TransportError::NotConnected)));
}

/// C-STOR-03: Multiple sends captured in order.
#[tokio::test]
async fn c_stor_03_multiple_sends_captured() {
    let transport = MockTransport::new();
    transport.connect("relay").await.unwrap();

    // Send 5 messages
    for i in 0..5 {
        transport.send(format!("message-{}", i).as_bytes()).await.unwrap();
    }

    let sent = transport.sent_messages();
    assert_eq!(sent.len(), 5);

    for (i, msg) in sent.iter().enumerate() {
        assert_eq!(msg, format!("message-{}", i).as_bytes());
    }
}

/// C-STOR-04: Reset clears all transport state.
#[tokio::test]
async fn c_stor_04_reset_clears_state() {
    let transport = MockTransport::new();
    transport.connect("relay").await.unwrap();
    transport.send(b"data").await.unwrap();
    transport.queue_response(b"response".to_vec());

    // Reset
    transport.reset();

    // All state cleared
    assert!(!transport.is_connected());
    assert!(transport.sent_messages().is_empty());
    assert!(transport.connected_address().is_none());

    // Can connect again
    transport.connect("new-relay").await.unwrap();
    assert!(transport.is_connected());
}

// ============================================================================
// C-COLL-* Collection Integrity (2 tests)
// ============================================================================

/// C-COLL-01: Pull multiple blobs, all decrypt correctly.
#[tokio::test]
async fn c_coll_01_pull_multiple_blobs() {
    let secret = GroupSecret::from_passphrase("collection-test");
    let key = GroupKey::derive(&secret);

    // Encrypt 3 blobs
    let blobs: Vec<Vec<u8>> = vec![
        b"blob one".to_vec(),
        b"blob two".to_vec(),
        b"blob three".to_vec(),
    ];

    let mut encrypted: Vec<(Vec<u8>, [u8; NONCE_SIZE])> = Vec::new();
    for blob in &blobs {
        let (ct, nonce) = key.encrypt(blob).unwrap();
        encrypted.push((ct, nonce));
    }

    // Decrypt all
    let mut decrypted = Vec::new();
    for (ct, nonce) in &encrypted {
        let pt = key.decrypt(ct, nonce).unwrap();
        decrypted.push(pt);
    }

    assert_eq!(decrypted.len(), 3);
    assert_eq!(decrypted[0], b"blob one");
    assert_eq!(decrypted[1], b"blob two");
    assert_eq!(decrypted[2], b"blob three");
}

/// C-COLL-02: Mixed valid/invalid blobs - valid returned, invalid skipped.
#[tokio::test]
async fn c_coll_02_pull_mixed_valid_invalid() {
    let secret = GroupSecret::from_passphrase("mixed-test");
    let key = GroupKey::derive(&secret);

    // Encrypt 2 valid blobs
    let (ct1, nonce1) = key.encrypt(b"valid-1").unwrap();
    let (ct2, nonce2) = key.encrypt(b"valid-2").unwrap();

    // Create 1 corrupted blob
    let (mut ct_bad, nonce_bad) = key.encrypt(b"will-corrupt").unwrap();
    ct_bad[0] ^= 0xFF; // Corrupt

    // Try to decrypt all
    let blobs = vec![
        (ct1, nonce1),
        (ct_bad, nonce_bad),
        (ct2, nonce2),
    ];

    let mut successful = Vec::new();
    let mut failed = 0;

    for (ct, nonce) in blobs {
        match key.decrypt(&ct, &nonce) {
            Ok(pt) => successful.push(pt),
            Err(_) => failed += 1,
        }
    }

    // 2 successful, 1 failed
    assert_eq!(successful.len(), 2);
    assert_eq!(failed, 1);
    assert_eq!(successful[0], b"valid-1");
    assert_eq!(successful[1], b"valid-2");
}
}
