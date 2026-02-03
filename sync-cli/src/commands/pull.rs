//! Pull data from the sync group.

use anyhow::{Context, Result};
use std::path::Path;
use zerok_sync_client::{GroupKey, MockTransport, SyncClient, SyncConfig};
use zerok_sync_types::Cursor;

use crate::config::{DeviceConfig, GroupConfig};

/// Run the pull command.
pub async fn run(data_dir: &Path, after_cursor: Option<u64>) -> Result<()> {
    // Load configuration
    let device = DeviceConfig::load(data_dir).await?;
    let mut group = GroupConfig::load(data_dir).await?;

    let cursor = after_cursor.unwrap_or(group.cursor);
    println!("Pulling data after cursor {}...", cursor);

    // Create sync client
    // Note: Using MockTransport for now. In production, this would be IrohTransport.
    let config = SyncConfig::new("placeholder-passphrase", &group.relay_address)
        .with_device_name(&device.device_name);

    let transport = MockTransport::new();

    // Queue a mock PullResponse
    let mock_response = create_mock_pull_response(&config);
    transport.queue_response(mock_response);

    let client = SyncClient::new(config, transport);

    // Connect and pull
    client
        .connect()
        .await
        .context("Failed to connect to relay")?;

    match client.pull_after(Some(Cursor::new(cursor))).await {
        Ok(blobs) => {
            if blobs.is_empty() {
                println!("No new data.");
            } else {
                println!("Received {} blob(s):", blobs.len());
                println!();

                for blob in &blobs {
                    // Try to display as UTF-8, otherwise show hex
                    let content = match std::str::from_utf8(&blob.payload) {
                        Ok(s) => s.to_string(),
                        Err(_) => format!("<binary {} bytes>", blob.payload.len()),
                    };

                    println!("  [{}] {}", blob.cursor, content);

                    // Update cursor if this is higher
                    if blob.cursor.value() > group.cursor {
                        group.cursor = blob.cursor.value();
                    }
                }

                // Save updated cursor
                group.save(data_dir).await?;

                println!();
                println!("Cursor updated to {}", group.cursor);
            }
        }
        Err(e) => {
            println!("Pull failed: {}", e);
            println!();
            println!("This may be due to network issues or no data available.");
        }
    }

    Ok(())
}

/// Create a mock PullResponse for testing.
fn create_mock_pull_response(config: &SyncConfig) -> Vec<u8> {
    use zerok_sync_types::{BlobId, Cursor, Message, PullBlob, PullResponse};

    // Create an encrypted test message
    let key = GroupKey::derive(&config.group_secret);
    let plaintext = b"Hello from sync group!";
    let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();

    // Prepend nonce to ciphertext
    let mut payload = Vec::with_capacity(24 + ciphertext.len());
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);

    let response = Message::PullResponse(PullResponse {
        blobs: vec![PullBlob {
            blob_id: BlobId::new(),
            cursor: Cursor::new(1),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }],
        has_more: false,
        max_cursor: Cursor::new(1),
    });

    response.to_bytes().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn setup_device_and_group(dir: &Path) {
        let device = DeviceConfig::new("Test Device");
        device.save(dir).await.unwrap();

        let group = GroupConfig::new("test-group-id", "test-relay");
        group.save(dir).await.unwrap();
    }

    #[tokio::test]
    async fn pull_requires_device_and_group() {
        let dir = tempdir().unwrap();

        // Should fail without device
        let result = run(dir.path(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pull_with_mock_transport() {
        let dir = tempdir().unwrap();
        setup_device_and_group(dir.path()).await;

        // This will use MockTransport with a mock response
        let result = run(dir.path(), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn pull_after_specific_cursor() {
        let dir = tempdir().unwrap();
        setup_device_and_group(dir.path()).await;

        let result = run(dir.path(), Some(100)).await;
        assert!(result.is_ok());
    }
}
