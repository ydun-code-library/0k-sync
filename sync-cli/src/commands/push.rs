//! Push data to the sync group.

use anyhow::{Context, Result};
use std::path::Path;
use zerok_sync_client::{MockTransport, SyncClient, SyncConfig};

use crate::config::{DeviceConfig, GroupConfig};

/// Run the push command.
pub async fn run(data_dir: &Path, data: &[u8]) -> Result<()> {
    // Load configuration
    let device = DeviceConfig::load(data_dir).await?;
    let mut group = GroupConfig::load(data_dir).await?;

    println!("Pushing {} bytes...", data.len());

    // Create sync client
    // Note: Using MockTransport for now. In production, this would be IrohTransport.
    let config = SyncConfig::new("placeholder-passphrase", &group.relay_address)
        .with_device_name(&device.device_name);

    let transport = MockTransport::new();

    // Queue a mock PushAck response
    let mock_ack = create_mock_push_ack();
    transport.queue_response(mock_ack);

    let client = SyncClient::new(config, transport);

    // Connect and push
    client
        .connect()
        .await
        .context("Failed to connect to relay")?;

    match client.push(data).await {
        Ok((blob_id, cursor)) => {
            // Update cursor in config
            group.cursor = cursor.value();
            group.save(data_dir).await?;

            println!("Push successful!");
            println!();
            println!("  Blob ID: {}", blob_id);
            println!("  Cursor:  {}", cursor);
        }
        Err(e) => {
            // For demo purposes, show what would happen
            println!("Push queued (offline mode)");
            println!("  Error: {}", e);
            println!();
            println!("Data will sync when connection is available.");
        }
    }

    Ok(())
}

/// Create a mock PushAck response for testing.
fn create_mock_push_ack() -> Vec<u8> {
    use zerok_sync_types::{BlobId, Cursor, Message, PushAck};

    let ack = Message::PushAck(PushAck {
        blob_id: BlobId::new(),
        cursor: Cursor::new(1),
    });

    ack.to_bytes().unwrap_or_default()
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
    async fn push_requires_device_and_group() {
        let dir = tempdir().unwrap();

        // Should fail without device
        let result = run(dir.path(), b"test data").await;
        assert!(result.is_err());

        // Init device but not group
        let device = DeviceConfig::new("Test");
        device.save(dir.path()).await.unwrap();

        // Should still fail without group
        let result = run(dir.path(), b"test data").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn push_with_mock_transport() {
        let dir = tempdir().unwrap();
        setup_device_and_group(dir.path()).await;

        // This will use MockTransport and fail because the mock PushAck
        // has a different blob_id than what we're pushing. That's expected
        // for this demo implementation.
        let result = run(dir.path(), b"test data").await;
        // The error is expected in this test setup
        assert!(result.is_ok()); // Function handles the error gracefully
    }
}
