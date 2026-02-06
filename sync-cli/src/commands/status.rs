//! Show sync status.

use anyhow::Result;
use std::path::Path;

use crate::config::{DeviceConfig, GroupConfig};

/// Run the status command.
pub async fn run(data_dir: &Path) -> Result<()> {
    println!("=== sync-cli status ===");
    println!();

    // Check device
    match DeviceConfig::load(data_dir).await {
        Ok(device) => {
            println!("Device:");
            println!("  ID:   {}", &device.device_id[..16]);
            println!("  Name: {}", device.device_name);
            println!("  Init: {}", format_timestamp(device.created_at));
        }
        Err(_) => {
            println!("Device: NOT INITIALIZED");
            println!();
            println!("Run 'sync-cli init --name <name>' to initialize.");
            return Ok(());
        }
    }

    println!();

    // Check group
    match GroupConfig::load(data_dir).await {
        Ok(group) => {
            let group_id_display = if group.group_id.len() > 16 {
                &group.group_id[..16]
            } else {
                &group.group_id
            };

            println!("Sync Group:");
            println!("  ID:     {}", group_id_display);
            println!("  Relays: {} configured", group.relay_addresses.len());
            for (i, addr) in group.relay_addresses.iter().enumerate() {
                let label = if i == 0 { "primary" } else { "secondary" };
                let display = if addr.len() > 16 { &addr[..16] } else { addr };
                let cursor = group.cursor_for_relay(addr);
                println!("    [{}] {}... (cursor: {})", label, display, cursor);
            }
            println!("  Joined: {}", format_timestamp(group.joined_at));
        }
        Err(_) => {
            println!("Sync Group: NOT PAIRED");
            println!();
            println!("Run 'sync-cli pair --create' or 'sync-cli pair --join <code>'");
            return Ok(());
        }
    }

    println!();

    // Connection status (placeholder for now)
    println!("Connection:");
    println!("  Status: OFFLINE (mock transport)");
    println!();
    println!("Note: Real network connection requires iroh transport (Phase 5+)");

    Ok(())
}

/// Format a Unix timestamp as a human-readable string.
fn format_timestamp(ts: u64) -> String {
    // Simple formatting without external dependencies
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let diff = now.saturating_sub(ts);

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn status_without_init() {
        let dir = tempdir().unwrap();

        // Should succeed but show "not initialized"
        let result = run(dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn status_with_device() {
        let dir = tempdir().unwrap();

        let device = DeviceConfig::new("Test Device");
        device.save(dir.path()).await.unwrap();

        let result = run(dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn status_with_device_and_group() {
        let dir = tempdir().unwrap();

        let device = DeviceConfig::new("Test Device");
        device.save(dir.path()).await.unwrap();

        let group = GroupConfig::new("test-group", &["test-relay"]);
        group.save(dir.path()).await.unwrap();

        let result = run(dir.path()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn format_timestamp_works() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert_eq!(format_timestamp(now), "just now");
        assert!(format_timestamp(now - 120).contains("minutes"));
        assert!(format_timestamp(now - 7200).contains("hours"));
        assert!(format_timestamp(now - 172800).contains("days"));
    }
}
