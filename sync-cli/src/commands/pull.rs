//! Pull data from the sync group.

use anyhow::{Context, Result};
use std::path::Path;
use zerok_sync_client::{
    GroupKey, IrohTransport, MockTransport, SyncClient, SyncConfig, Transport,
};
use zerok_sync_types::Cursor;

use crate::config::{DeviceConfig, GroupConfig};

/// Run the pull command.
pub async fn run(data_dir: &Path, after_cursor: Option<u64>, use_mock: bool) -> Result<()> {
    // Load configuration
    let device = DeviceConfig::load(data_dir).await?;
    let mut group = GroupConfig::load(data_dir).await?;

    let primary_relay = group
        .primary_relay()
        .ok_or_else(|| anyhow::anyhow!("No relay addresses configured"))?
        .to_string();
    let cursor = after_cursor.unwrap_or_else(|| group.cursor_for_relay(&primary_relay));
    println!("Pulling data after cursor {}...", cursor);

    // Create sync client config from stored secret (F-003: no placeholder fallback)
    let secret_bytes = group
        .group_secret_bytes()
        .filter(|b| b.len() == 32)
        .ok_or_else(|| anyhow::anyhow!("Group secret not found. Run 'sync-cli pair' first."))?;
    let bytes: [u8; 32] = secret_bytes.try_into().unwrap();
    let config =
        SyncConfig::from_secret_bytes(&bytes, &primary_relay).with_device_name(&device.device_name);

    // Create transport and client based on mode
    if use_mock {
        run_with_mock(config, &mut group, data_dir, cursor).await
    } else {
        run_with_iroh(config, &mut group, data_dir, cursor).await
    }
}

/// Run pull with MockTransport (for testing/demo).
async fn run_with_mock(
    config: SyncConfig,
    group: &mut GroupConfig,
    data_dir: &Path,
    cursor: u64,
) -> Result<()> {
    let transport = MockTransport::new();

    // Queue Welcome response for HELLO handshake
    transport.queue_response(create_mock_welcome());

    // Queue a mock PullResponse
    let mock_response = create_mock_pull_response(&config);
    transport.queue_response(mock_response);

    let client = SyncClient::new(config, transport);
    do_pull(client, group, data_dir, cursor).await
}

/// Run pull with IrohTransport (real P2P).
async fn run_with_iroh(
    config: SyncConfig,
    group: &mut GroupConfig,
    data_dir: &Path,
    cursor: u64,
) -> Result<()> {
    let primary = group.primary_relay().unwrap_or("unknown");
    println!("Connecting to peer: {}", primary);

    let transport = IrohTransport::new()
        .await
        .context("Failed to create iroh transport")?;

    println!("  Our EndpointId: {}", transport.endpoint_id());

    let client = SyncClient::new(config, transport);
    do_pull(client, group, data_dir, cursor).await
}

/// Common pull logic for any transport.
async fn do_pull<T: Transport>(
    client: SyncClient<T>,
    group: &mut GroupConfig,
    data_dir: &Path,
    cursor: u64,
) -> Result<()> {
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

                // Use active relay for cursor tracking (handles failover)
                let relay_addr = client
                    .active_relay()
                    .await
                    .or_else(|| group.primary_relay().map(|s| s.to_string()))
                    .unwrap_or_default();
                let current_cursor = group.cursor_for_relay(&relay_addr);

                for blob in &blobs {
                    // Try to display as UTF-8, otherwise show hex
                    let content = match std::str::from_utf8(&blob.payload) {
                        Ok(s) => s.to_string(),
                        Err(_) => format!("<binary {} bytes>", blob.payload.len()),
                    };

                    println!("  [{}] {}", blob.cursor, content);

                    // Update cursor if this is higher
                    if blob.cursor.value() > current_cursor {
                        group.set_cursor_for_relay(&relay_addr, blob.cursor.value());
                    }
                }

                // Save updated cursor
                group.save(data_dir).await?;

                println!();
                println!("Cursor updated to {}", group.cursor_for_relay(&relay_addr));
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

/// Create a mock Welcome response for HELLO handshake.
fn create_mock_welcome() -> Vec<u8> {
    use zerok_sync_types::{Cursor, Message, Welcome};
    Message::Welcome(Welcome {
        version: 1,
        max_cursor: Cursor::zero(),
        pending_count: 0,
    })
    .to_bytes()
    .unwrap_or_default()
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

        let group = GroupConfig::with_secret("test-group-id", &["test-relay"], &[0x42u8; 32]);
        group.save(dir).await.unwrap();
    }

    async fn setup_device_and_group_no_secret(dir: &Path) {
        let device = DeviceConfig::new("Test Device");
        device.save(dir).await.unwrap();

        let group = GroupConfig::new("test-group-id", &["test-relay"]);
        group.save(dir).await.unwrap();
    }

    #[tokio::test]
    async fn pull_requires_device_and_group() {
        let dir = tempdir().unwrap();

        // Should fail without device (use_mock=true)
        let result = run(dir.path(), None, true).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pull_with_mock_transport() {
        let dir = tempdir().unwrap();
        setup_device_and_group(dir.path()).await;

        // This will use MockTransport with a mock response
        let result = run(dir.path(), None, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn pull_after_specific_cursor() {
        let dir = tempdir().unwrap();
        setup_device_and_group(dir.path()).await;

        let result = run(dir.path(), Some(100), true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn pull_fails_without_secret() {
        // F-003: no placeholder fallback — missing secret must error
        let dir = tempdir().unwrap();
        setup_device_and_group_no_secret(dir.path()).await;

        let result = run(dir.path(), None, true).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Group secret not found"), "got: {}", err);
    }
}
