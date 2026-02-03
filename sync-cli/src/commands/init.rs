//! Initialize device identity.

use anyhow::Result;
use std::path::Path;

use crate::config::DeviceConfig;

/// Run the init command.
pub async fn run(data_dir: &Path, name: &str) -> Result<()> {
    // Check if already initialized
    if DeviceConfig::exists(data_dir).await {
        anyhow::bail!(
            "Device already initialized. Delete {} to reinitialize.",
            data_dir.join("device.json").display()
        );
    }

    // Create new device configuration
    let config = DeviceConfig::new(name);
    config.save(data_dir).await?;

    println!("Device initialized successfully!");
    println!();
    println!("  Device ID: {}", &config.device_id[..16]);
    println!("  Name:      {}", config.device_name);
    println!("  Data dir:  {}", data_dir.display());
    println!();
    println!("Next steps:");
    println!("  1. Create a sync group: sync-cli pair --create");
    println!("  2. Or join an existing group: sync-cli pair --join <code>");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn init_creates_device_config() {
        let dir = tempdir().unwrap();
        run(dir.path(), "Test Device").await.unwrap();

        assert!(dir.path().join("device.json").exists());

        let config = DeviceConfig::load(dir.path()).await.unwrap();
        assert_eq!(config.device_name, "Test Device");
        assert!(!config.device_id.is_empty());
    }

    #[tokio::test]
    async fn init_fails_if_already_initialized() {
        let dir = tempdir().unwrap();

        // First init should succeed
        run(dir.path(), "Device 1").await.unwrap();

        // Second init should fail
        let result = run(dir.path(), "Device 2").await;
        assert!(result.is_err());
    }
}
