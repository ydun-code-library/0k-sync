//! Configuration management for sync-cli.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use zerok_sync_types::{Cursor, DeviceId};

/// Device configuration stored locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Unique device identifier.
    pub device_id: String,
    /// Human-readable device name.
    pub device_name: String,
    /// When the device was initialized.
    pub created_at: u64,
}

impl DeviceConfig {
    /// Create a new device configuration.
    pub fn new(name: &str) -> Self {
        let device_id = DeviceId::random();
        Self {
            device_id: device_id.to_string(),
            device_name: name.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Load device configuration from a directory.
    pub async fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("device.json");
        let contents = tokio::fs::read_to_string(&path)
            .await
            .context("Device not initialized. Run 'sync-cli init' first.")?;
        serde_json::from_str(&contents).context("Invalid device configuration")
    }

    /// Save device configuration to a directory.
    pub async fn save(&self, data_dir: &Path) -> Result<()> {
        let path = data_dir.join("device.json");
        let contents = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, contents)
            .await
            .context("Failed to save device configuration")?;
        Ok(())
    }

    /// Check if device is initialized.
    pub async fn exists(data_dir: &Path) -> bool {
        data_dir.join("device.json").exists()
    }
}

/// Sync group configuration stored locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    /// Group identifier (derived from passphrase).
    pub group_id: String,
    /// Relay address (iroh NodeId).
    pub relay_address: String,
    /// When the group was joined.
    pub joined_at: u64,
    /// Current sync cursor.
    pub cursor: u64,
}

impl GroupConfig {
    /// Create a new group configuration.
    pub fn new(group_id: &str, relay_address: &str) -> Self {
        Self {
            group_id: group_id.to_string(),
            relay_address: relay_address.to_string(),
            joined_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cursor: 0,
        }
    }

    /// Load group configuration from a directory.
    pub async fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("group.json");
        let contents = tokio::fs::read_to_string(&path)
            .await
            .context("Not paired. Run 'sync-cli pair' first.")?;
        serde_json::from_str(&contents).context("Invalid group configuration")
    }

    /// Save group configuration to a directory.
    pub async fn save(&self, data_dir: &Path) -> Result<()> {
        let path = data_dir.join("group.json");
        let contents = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, contents)
            .await
            .context("Failed to save group configuration")?;
        Ok(())
    }

    /// Check if group is configured.
    pub async fn exists(data_dir: &Path) -> bool {
        data_dir.join("group.json").exists()
    }

    /// Update cursor position.
    #[allow(dead_code)]
    pub fn update_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor.value();
    }
}

/// Full application configuration.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    /// Device configuration.
    pub device: DeviceConfig,
    /// Group configuration (if paired).
    pub group: Option<GroupConfig>,
}

#[allow(dead_code)]
impl AppConfig {
    /// Load full configuration from a directory.
    pub async fn load(data_dir: &Path) -> Result<Self> {
        let device = DeviceConfig::load(data_dir).await?;
        let group = GroupConfig::load(data_dir).await.ok();
        Ok(Self { device, group })
    }
}
