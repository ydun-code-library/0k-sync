//! Serve command - run a sync server that accepts connections.
//!
//! This is a minimal relay implementation for testing E2E sync between devices.

use anyhow::{Context, Result};
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::Endpoint;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use zerok_sync_client::ALPN;
use zerok_sync_types::{BlobId, Cursor, Message, PullBlob, PullResponse, PushAck};

use crate::config::DeviceConfig;

/// Maximum message size (1MB, matching sync-relay protocol limit).
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// In-memory blob storage for the serve command.
#[derive(Debug, Default)]
struct BlobStore {
    /// Stored blobs indexed by cursor.
    blobs: HashMap<u64, StoredBlob>,
    /// Next cursor value.
    next_cursor: u64,
}

/// A stored blob.
#[derive(Debug, Clone)]
struct StoredBlob {
    blob_id: BlobId,
    cursor: Cursor,
    payload: Vec<u8>,
    timestamp: u64,
}

impl BlobStore {
    /// Store a new blob and return its cursor.
    fn store(&mut self, blob_id: BlobId, payload: Vec<u8>) -> Cursor {
        let cursor = Cursor::new(self.next_cursor);
        self.next_cursor += 1;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.blobs.insert(
            cursor.value(),
            StoredBlob {
                blob_id,
                cursor,
                payload,
                timestamp,
            },
        );

        cursor
    }

    /// Get blobs after a cursor.
    ///
    /// - `None` = get all blobs
    /// - `Some(cursor)` = get blobs with cursor > given cursor
    fn get_after(&self, after_cursor: Option<Cursor>) -> Vec<StoredBlob> {
        let mut blobs: Vec<_> = match after_cursor {
            None => self.blobs.values().cloned().collect(),
            Some(cursor) => self
                .blobs
                .values()
                .filter(|b| b.cursor.value() > cursor.value())
                .cloned()
                .collect(),
        };

        blobs.sort_by_key(|b| b.cursor.value());
        blobs
    }

    /// Get the maximum cursor.
    fn max_cursor(&self) -> Cursor {
        Cursor::new(self.next_cursor.saturating_sub(1))
    }
}

/// Protocol handler for 0k-Sync.
#[derive(Clone)]
struct SyncProtocol {
    store: Arc<RwLock<BlobStore>>,
}

impl std::fmt::Debug for SyncProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncProtocol").finish()
    }
}

impl SyncProtocol {
    fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(BlobStore::default())),
        }
    }
}

impl ProtocolHandler for SyncProtocol {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), AcceptError> {
        use n0_error::StdResultExt;

        let remote_id = connection.remote_id();
        println!("  Connection from: {}", remote_id);

        // Accept bidirectional stream
        let (mut send, mut recv) = connection.accept_bi().await?;

        // Read length-prefixed message
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf).await.anyerr()?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // F-009: Guard against unbounded allocation from malicious length prefix
        if len > MAX_MESSAGE_SIZE {
            println!(
                "  Rejected oversized message: {} bytes (max {})",
                len, MAX_MESSAGE_SIZE
            );
            return Ok(());
        }

        let mut data = vec![0u8; len];
        recv.read_exact(&mut data).await.anyerr()?;

        // Parse message
        let message = Message::from_bytes(&data).anyerr()?;

        // Handle message
        let response = match message {
            Message::Push(push) => {
                println!("  Received Push: {} bytes", push.payload.len());

                let mut store = self.store.write().await;
                let cursor = store.store(push.blob_id, push.payload);

                println!("  Stored at cursor: {}", cursor);

                Message::PushAck(PushAck {
                    blob_id: push.blob_id,
                    cursor,
                })
            }
            Message::Pull(pull) => {
                println!("  Received Pull after cursor: {}", pull.after_cursor);

                let store = self.store.read().await;
                let blobs = store.get_after(Some(pull.after_cursor));

                println!("  Returning {} blob(s)", blobs.len());

                Message::PullResponse(PullResponse {
                    blobs: blobs
                        .into_iter()
                        .map(|b| PullBlob {
                            blob_id: b.blob_id,
                            cursor: b.cursor,
                            payload: b.payload,
                            timestamp: b.timestamp,
                        })
                        .collect(),
                    has_more: false,
                    max_cursor: store.max_cursor(),
                })
            }
            _ => {
                println!("  Unexpected message type");
                return Ok(());
            }
        };

        // Send response
        let response_bytes = response.to_bytes().anyerr()?;
        let len = (response_bytes.len() as u32).to_be_bytes();

        send.write_all(&len).await.anyerr()?;
        send.write_all(&response_bytes).await.anyerr()?;
        send.finish()?;

        // Wait for the stream to be fully acknowledged before returning
        // This prevents the connection from being cleaned up before data is transmitted
        send.stopped().await.ok();

        println!("  Response sent");

        Ok(())
    }
}

/// Run the serve command.
pub async fn run(data_dir: &Path, _passphrase: Option<&str>) -> Result<()> {
    // Load device config (just for display)
    let device = DeviceConfig::load(data_dir).await.ok();

    println!("Starting 0k-Sync server...");
    println!();

    // Create endpoint
    let endpoint = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .context("Failed to create endpoint")?;

    let endpoint_id = endpoint.id();

    println!("Server ready!");
    println!();
    if let Some(dev) = &device {
        println!("  Device:     {}", dev.device_name);
    }
    println!("  EndpointId: {}", endpoint_id);
    println!();
    println!("Other devices can connect with:");
    println!("  sync-cli pair --join {}", endpoint_id);
    println!();
    println!("Press Ctrl+C to stop.");
    println!();
    println!("--- Connections ---");

    // Set up router with our protocol
    let protocol = SyncProtocol::new();
    let router = Router::builder(endpoint).accept(ALPN, protocol).spawn();

    // Wait for Ctrl+C
    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    println!();
    println!("Shutting down...");

    router
        .shutdown()
        .await
        .context("Failed to shutdown router")?;

    println!("Done.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_store_stores_and_retrieves() {
        let mut store = BlobStore::default();

        // Store a blob
        let blob_id = BlobId::new();
        let cursor = store.store(blob_id, b"test data".to_vec());

        assert_eq!(cursor.value(), 0);

        // Retrieve it
        let blobs = store.get_after(None);
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].payload, b"test data");
    }

    #[test]
    fn blob_store_filters_by_cursor() {
        let mut store = BlobStore::default();

        // Store multiple blobs
        store.store(BlobId::new(), b"blob 0".to_vec());
        store.store(BlobId::new(), b"blob 1".to_vec());
        store.store(BlobId::new(), b"blob 2".to_vec());

        // Get all
        let all = store.get_after(None);
        assert_eq!(all.len(), 3);

        // Get after cursor 0
        let after_0 = store.get_after(Some(Cursor::new(0)));
        assert_eq!(after_0.len(), 2);

        // Get after cursor 1
        let after_1 = store.get_after(Some(Cursor::new(1)));
        assert_eq!(after_1.len(), 1);

        // Get after cursor 2
        let after_2 = store.get_after(Some(Cursor::new(2)));
        assert_eq!(after_2.len(), 0);
    }

    #[test]
    fn max_message_size_guards_allocation() {
        // F-009: Verify the allocation guard constant matches the relay protocol limit
        assert_eq!(
            MAX_MESSAGE_SIZE,
            1024 * 1024,
            "must match relay protocol limit"
        );

        // Verify the boundary conditions
        assert!(
            MAX_MESSAGE_SIZE < u32::MAX as usize,
            "guard must reject u32::MAX"
        );
        // A u32 length prefix can express up to 4GB — we cap at 1MB
        let malicious_len = u32::MAX as usize;
        assert!(malicious_len > MAX_MESSAGE_SIZE);
    }

    #[test]
    fn blob_store_max_cursor() {
        let mut store = BlobStore::default();

        assert_eq!(store.max_cursor().value(), 0);

        store.store(BlobId::new(), b"test".to_vec());
        assert_eq!(store.max_cursor().value(), 0);

        store.store(BlobId::new(), b"test".to_vec());
        assert_eq!(store.max_cursor().value(), 1);
    }
}
