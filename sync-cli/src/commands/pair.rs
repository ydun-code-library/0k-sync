//! Pairing commands for creating and joining sync groups.

use anyhow::{Context, Result};
use std::path::Path;
use zerok_sync_client::GroupSecret as ClientGroupSecret;
use zerok_sync_core::{GroupSecret, Invite, RelayNodeId};
use zerok_sync_types::GroupId;

use crate::config::{DeviceConfig, GroupConfig};

/// Create a new sync group and display invite code.
pub async fn create(data_dir: &Path, passphrase: Option<&str>) -> Result<()> {
    // Ensure device is initialized
    let _device = DeviceConfig::load(data_dir).await?;

    // Check if already paired
    if GroupConfig::exists(data_dir).await {
        anyhow::bail!(
            "Already paired to a group. Delete {} to create a new group.",
            data_dir.join("group.json").display()
        );
    }

    // Get or prompt for passphrase
    let passphrase = match passphrase {
        Some(p) => p.to_string(),
        None => prompt_passphrase("Enter passphrase for new sync group: ")?,
    };

    // Use sync-client's GroupSecret for passphrase derivation
    let client_secret = ClientGroupSecret::from_passphrase(&passphrase);

    // Convert to sync-core's GroupSecret format
    let group_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
    let group_id = group_secret.derive_group_id();

    // For now, use placeholder relay node (will be replaced with actual iroh NodeId)
    let relay_bytes = [0u8; 32]; // Placeholder - in production this would be the relay's public key
    let relay_node_id = RelayNodeId::from_bytes(relay_bytes);

    // Create invite
    let invite = Invite::create(relay_node_id, group_id, group_secret);
    let short_code = invite.to_short_code();
    let qr_payload = invite.to_qr_payload();

    // Save group configuration
    let group_config = GroupConfig::new(&group_id.to_string(), &relay_node_id.to_string());
    group_config.save(data_dir).await?;

    println!("Sync group created!");
    println!();
    println!("  Group ID: {}", &group_id.to_string()[..16]);
    println!();
    println!("Share with other devices:");
    println!();
    println!("  Short code: {}", short_code);
    println!();
    println!("  QR payload: {}", qr_payload);
    println!();
    println!("Note: Other devices need the same passphrase to join.");
    println!("The invite expires in 10 minutes.");

    Ok(())
}

/// Join an existing sync group using a QR payload or passphrase.
pub async fn join(data_dir: &Path, code: &str, passphrase: Option<&str>) -> Result<()> {
    // Ensure device is initialized
    let _device = DeviceConfig::load(data_dir).await?;

    // Check if already paired
    if GroupConfig::exists(data_dir).await {
        anyhow::bail!(
            "Already paired to a group. Delete {} to join a different group.",
            data_dir.join("group.json").display()
        );
    }

    // Try to parse as QR payload first
    let invite = match Invite::from_qr_payload(code) {
        Ok(inv) => {
            // Check if invite is expired
            if inv.is_expired() {
                anyhow::bail!("Invite code has expired. Request a new one.");
            }
            inv
        }
        Err(_) => {
            // Not a QR payload - treat as short code hint
            // With short codes, we need the passphrase to reconstruct the group
            let passphrase = match passphrase {
                Some(p) => p.to_string(),
                None => prompt_passphrase("Enter group passphrase: ")?,
            };

            // Derive group from passphrase
            let client_secret = ClientGroupSecret::from_passphrase(&passphrase);
            let group_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
            let group_id = group_secret.derive_group_id();

            // For short code joining, we need relay info from elsewhere
            // In a real implementation, the short code would be used to look up
            // the relay info from a bootstrap server
            let relay_bytes = [0u8; 32]; // Placeholder
            let relay_node_id = RelayNodeId::from_bytes(relay_bytes);

            Invite::create(relay_node_id, group_id, group_secret)
        }
    };

    // If we got the invite from QR but also have a passphrase, verify it matches
    if let Some(passphrase) = passphrase {
        let client_secret = ClientGroupSecret::from_passphrase(passphrase);
        let derived_group_id = GroupId::from_secret(client_secret.as_bytes());

        if derived_group_id != invite.group_id {
            anyhow::bail!("Passphrase does not match the group in this invite");
        }
    }

    // Save group configuration
    let group_config = GroupConfig::new(
        &invite.group_id.to_string(),
        &invite.relay_node_id.to_string(),
    );
    group_config.save(data_dir).await?;

    println!("Joined sync group successfully!");
    println!();
    println!("  Group ID: {}", &invite.group_id.to_string()[..16]);
    println!();
    println!("Next steps:");
    println!("  1. Push data: sync-cli push \"Hello!\"");
    println!("  2. Pull data: sync-cli pull");

    Ok(())
}

/// Prompt for passphrase input.
fn prompt_passphrase(prompt: &str) -> Result<String> {
    // In a real implementation, use rpassword for secure input
    // For now, just read from stdin
    print!("{}", prompt);
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("Failed to read passphrase")?;

    Ok(input.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn init_device(dir: &Path) {
        let config = DeviceConfig::new("Test Device");
        config.save(dir).await.unwrap();
    }

    #[tokio::test]
    async fn create_requires_initialized_device() {
        let dir = tempdir().unwrap();

        // Should fail without device init
        let result = create(dir.path(), Some("test-passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_saves_group_config() {
        let dir = tempdir().unwrap();
        init_device(dir.path()).await;

        create(dir.path(), Some("test-passphrase")).await.unwrap();

        assert!(dir.path().join("group.json").exists());
        let config = GroupConfig::load(dir.path()).await.unwrap();
        assert!(!config.group_id.is_empty());
    }

    #[tokio::test]
    async fn create_fails_if_already_paired() {
        let dir = tempdir().unwrap();
        init_device(dir.path()).await;

        // First create should succeed
        create(dir.path(), Some("test-passphrase")).await.unwrap();

        // Second create should fail
        let result = create(dir.path(), Some("other-passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn join_with_qr_payload() {
        let dir = tempdir().unwrap();
        init_device(dir.path()).await;

        // Create an invite payload
        let client_secret = ClientGroupSecret::from_passphrase("test");
        let group_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
        let group_id = group_secret.derive_group_id();
        let relay = RelayNodeId::from_bytes([0u8; 32]);
        let invite = Invite::create(relay, group_id, group_secret);
        let qr = invite.to_qr_payload();

        // Join with the QR payload
        join(dir.path(), &qr, None).await.unwrap();

        assert!(dir.path().join("group.json").exists());
    }
}
