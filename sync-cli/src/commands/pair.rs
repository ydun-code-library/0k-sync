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

    // Use sync-client's GroupSecret for passphrase derivation (with random salt)
    let (client_secret, salt) = ClientGroupSecret::from_passphrase(&passphrase);

    // Convert to sync-core's GroupSecret format
    let group_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
    let group_id = group_secret.derive_group_id();

    // For now, use placeholder relay node (will be replaced with actual iroh NodeId)
    let relay_bytes = [0u8; 32]; // Placeholder - in production this would be the relay's public key
    let relay_node_id = RelayNodeId::from_bytes(relay_bytes);

    // Create invite with salt for v2 format
    let invite = Invite::create(relay_node_id, group_id, group_secret, salt.to_vec());
    let short_code = invite.to_short_code();
    let qr_payload = invite.to_qr_payload();

    // Save group configuration with secret and salt for encryption
    let group_config = GroupConfig::with_secret_and_salt(
        &group_id.to_string(),
        &relay_node_id.to_string(),
        client_secret.as_bytes(),
        &salt,
    );
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

/// Join an existing sync group using a QR payload, EndpointId, or passphrase.
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
    if let Ok(invite) = Invite::from_qr_payload(code) {
        // Check if invite is expired
        if invite.is_expired() {
            anyhow::bail!("Invite code has expired. Request a new one.");
        }

        // If we got the invite from QR but also have a passphrase, verify it matches
        if let Some(passphrase) = passphrase {
            let client_secret =
                ClientGroupSecret::from_passphrase_with_salt(passphrase, &invite.salt);
            let derived_group_id = GroupId::from_secret(client_secret.as_bytes());

            if derived_group_id != invite.group_id {
                anyhow::bail!("Passphrase does not match the group in this invite");
            }
        }

        // Save group configuration WITH secret and salt (the invite carries both)
        let group_config = GroupConfig::with_secret_and_salt(
            &invite.group_id.to_string(),
            &invite.relay_node_id.to_string(),
            invite.group_secret.as_bytes(),
            &invite.salt,
        );
        group_config.save(data_dir).await?;

        println!("Joined sync group successfully!");
        println!();
        println!("  Group ID: {}", &invite.group_id.to_string()[..16]);
        println!();
        println!("Next steps:");
        println!("  1. Push data: sync-cli push \"Hello!\"");
        println!("  2. Pull data: sync-cli pull");

        return Ok(());
    }

    // Check if code looks like an EndpointId (64-char hex string)
    let is_endpoint_id = code.len() == 64 && code.chars().all(|c| c.is_ascii_hexdigit());

    if is_endpoint_id {
        // Direct EndpointId join - requires passphrase for group derivation.
        // Uses a fixed salt because there's no invite to carry a random salt.
        // This is weaker than the QR path — prefer QR/short code for production use.
        const ENDPOINT_JOIN_SALT: &[u8; 16] = b"0k-endpt-join-v2";

        let passphrase = match passphrase {
            Some(p) => p.to_string(),
            None => prompt_passphrase("Enter group passphrase: ")?,
        };

        // Derive group from passphrase with fixed salt
        let client_secret =
            ClientGroupSecret::from_passphrase_with_salt(&passphrase, ENDPOINT_JOIN_SALT);
        let group_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
        let group_id = group_secret.derive_group_id();

        // Use the provided EndpointId as the relay address, store secret and salt
        let group_config = GroupConfig::with_secret_and_salt(
            &group_id.to_string(),
            code,
            client_secret.as_bytes(),
            ENDPOINT_JOIN_SALT,
        );
        group_config.save(data_dir).await?;

        println!("Joined sync group successfully!");
        println!();
        println!("  Group ID:    {}", &group_id.to_string()[..16]);
        println!("  EndpointId:  {}...{}", &code[..8], &code[56..]);
        println!();
        println!("Next steps:");
        println!("  1. Push data: sync-cli push \"Hello!\"");
        println!("  2. Pull data: sync-cli pull");

        return Ok(());
    }

    // Fall back to short code handling
    // With short codes, we need the passphrase to reconstruct the group
    let passphrase = match passphrase {
        Some(p) => p.to_string(),
        None => prompt_passphrase("Enter group passphrase: ")?,
    };

    // For short code joining, we would need relay info from a bootstrap server
    // This is not yet implemented
    let _ = (&passphrase, code); // Suppress unused warnings
    anyhow::bail!(
        "Short code lookup not yet implemented. Use EndpointId directly:\n  \
         sync-cli pair --join <endpoint-id>\n\n\
         Get the EndpointId from the server running 'sync-cli serve'"
    );
}

/// Prompt for passphrase input with echo suppression.
fn prompt_passphrase(prompt: &str) -> Result<String> {
    let passphrase = rpassword::prompt_password(prompt)
        .context("Failed to read passphrase")?;

    // F-035: Reject empty or too-short passphrases
    let trimmed = passphrase.trim().to_string();
    if trimmed.len() < 8 {
        anyhow::bail!("Passphrase must be at least 8 characters");
    }

    Ok(trimmed)
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

        // Create a v2 invite payload with salt
        let salt = b"test-salt-00000!";
        let client_secret = ClientGroupSecret::from_passphrase_with_salt("test", salt);
        let group_secret = GroupSecret::from_bytes(*client_secret.as_bytes());
        let group_id = group_secret.derive_group_id();
        let relay = RelayNodeId::from_bytes([0u8; 32]);
        let invite = Invite::create(relay, group_id, group_secret, salt.to_vec());
        let qr = invite.to_qr_payload();

        // Join with the QR payload
        join(dir.path(), &qr, None).await.unwrap();

        assert!(dir.path().join("group.json").exists());

        // Verify the group_secret was saved (fixes pre-existing bug)
        let config = GroupConfig::load(dir.path()).await.unwrap();
        assert!(config.group_secret_hex.is_some());
        assert!(config.salt_hex.is_some());
    }

    #[tokio::test]
    async fn join_with_endpoint_id() {
        let dir = tempdir().unwrap();
        init_device(dir.path()).await;

        // Use a 64-char hex string as EndpointId
        let endpoint_id = "aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444";
        let passphrase = "test-passphrase";

        // Join with EndpointId
        join(dir.path(), endpoint_id, Some(passphrase)).await.unwrap();

        // Verify group.json was created with correct relay_address
        let config = GroupConfig::load(dir.path()).await.unwrap();
        assert_eq!(config.relay_address, endpoint_id);
        assert!(!config.group_id.is_empty());
    }

    #[tokio::test]
    async fn join_with_short_code_fails() {
        let dir = tempdir().unwrap();
        init_device(dir.path()).await;

        // Short codes are not yet implemented
        let result = join(dir.path(), "ABCD-EFGH", Some("test")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Short code lookup not yet implemented"));
    }
}
