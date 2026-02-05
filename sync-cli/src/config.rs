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
        set_file_permissions_0600(&path).await?;
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
    /// Hex-encoded group secret for encryption (derived from passphrase).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_secret_hex: Option<String>,
    /// Hex-encoded Argon2id salt used to derive the group secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salt_hex: Option<String>,
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
            group_secret_hex: None,
            salt_hex: None,
        }
    }

    /// Create a new group configuration with secret.
    pub fn with_secret(group_id: &str, relay_address: &str, secret_bytes: &[u8]) -> Self {
        Self {
            group_id: group_id.to_string(),
            relay_address: relay_address.to_string(),
            joined_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cursor: 0,
            group_secret_hex: Some(hex::encode(secret_bytes)),
            salt_hex: None,
        }
    }

    /// Create a new group configuration with secret and salt.
    pub fn with_secret_and_salt(
        group_id: &str,
        relay_address: &str,
        secret_bytes: &[u8],
        salt: &[u8],
    ) -> Self {
        Self {
            group_id: group_id.to_string(),
            relay_address: relay_address.to_string(),
            joined_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cursor: 0,
            group_secret_hex: Some(hex::encode(secret_bytes)),
            salt_hex: Some(hex::encode(salt)),
        }
    }

    /// Get the group secret bytes, if stored.
    pub fn group_secret_bytes(&self) -> Option<Vec<u8>> {
        self.group_secret_hex
            .as_ref()
            .and_then(|h| hex::decode(h).ok())
    }

    /// Get the salt bytes, if stored.
    pub fn salt_bytes(&self) -> Option<Vec<u8>> {
        self.salt_hex.as_ref().and_then(|h| hex::decode(h).ok())
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
        set_file_permissions_0600(&path).await?;
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

/// Set file permissions to 0600 (owner read/write only) on Unix.
/// No-op on non-Unix platforms.
async fn set_file_permissions_0600(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .await
            .context("Failed to set file permissions")?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Set directory permissions to 0700 (owner only) on Unix.
/// No-op on non-Unix platforms.
pub async fn set_dir_permissions_0700(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .await
            .context("Failed to set directory permissions")?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn group_config_with_salt_roundtrip() {
        let dir = tempdir().unwrap();
        let config = GroupConfig::with_secret_and_salt(
            "group-123",
            "relay-addr",
            &[0x42u8; 32],
            &[0xABu8; 16],
        );
        config.save(dir.path()).await.unwrap();

        let loaded = GroupConfig::load(dir.path()).await.unwrap();
        assert_eq!(loaded.group_id, "group-123");
        assert!(loaded.group_secret_hex.is_some());
        assert!(loaded.salt_hex.is_some());
        assert_eq!(loaded.salt_bytes().unwrap(), vec![0xABu8; 16]);
        assert_eq!(loaded.group_secret_bytes().unwrap(), vec![0x42u8; 32]);
    }

    #[tokio::test]
    async fn group_config_without_salt_loads() {
        // Backwards compatibility: old config without salt_hex
        let dir = tempdir().unwrap();
        let config = GroupConfig::with_secret("group-old", "relay", &[1u8; 32]);
        config.save(dir.path()).await.unwrap();

        let loaded = GroupConfig::load(dir.path()).await.unwrap();
        assert!(loaded.salt_hex.is_none());
        assert!(loaded.salt_bytes().is_none());
        assert!(loaded.group_secret_bytes().is_some());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn group_config_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let config = GroupConfig::with_secret("test", "relay", &[0u8; 32]);
        config.save(dir.path()).await.unwrap();

        let path = dir.path().join("group.json");
        let perms = tokio::fs::metadata(&path).await.unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600, "file should be 0600");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn data_dir_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let data_dir = dir.path().join("test-data");
        tokio::fs::create_dir_all(&data_dir).await.unwrap();
        set_dir_permissions_0700(&data_dir).await.unwrap();

        let perms = tokio::fs::metadata(&data_dir).await.unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o700, "dir should be 0700");
    }
}
