//! Configuration constants for distributed chaos testing.
//!
//! Defines SSH targets, file paths, and port mappings for the
//! multi-machine Tailscale mesh topology.

use super::ssh::SshTarget;

/// Beast — 91GB RAM Linux server (Docker host, relays, cross-compilation).
pub const BEAST: SshTarget = SshTarget {
    host: "100.71.79.25",
    user: "jimmyb",
};

/// Guardian — Raspberry Pi ARM device (edge client).
pub const GUARDIAN: SshTarget = SshTarget {
    host: "100.115.186.91",
    user: "jamesb",
};

/// Q's Tailscale IP (Mac Mini — test orchestrator).
pub const Q_IP: &str = "100.114.70.54";

/// Beast's Tailscale IP.
pub const BEAST_IP: &str = "100.71.79.25";

/// Guardian's Tailscale IP.
pub const GUARDIAN_IP: &str = "100.115.186.91";

/// Path to the 0k-sync repo on Beast.
pub const BEAST_REPO: &str = "~/0k-sync";

/// Path to the release CLI binary on Beast.
pub const BEAST_CLI: &str = "~/0k-sync/target/release/sync-cli";

/// Get the path to the release CLI binary on Q (Mac Mini).
/// Resolves relative to the workspace root at runtime.
pub fn q_cli_path() -> std::path::PathBuf {
    // Navigate from chaos-tests manifest to workspace root
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // tests/
        .and_then(|p| p.parent()) // workspace root
        .map(|p| p.join("target/release/sync-cli"))
        .unwrap_or_else(|| std::path::PathBuf::from("target/release/sync-cli"))
}

/// Path to the cross-compiled ARM binary on Guardian.
pub const GUARDIAN_CLI: &str = "/tmp/sync-cli-arm";

/// Guardian data directory for test state.
pub const GUARDIAN_DATA_DIR: &str = "/tmp/0k-sync-test";

/// HTTP ports for the 3 relay instances on Beast.
pub const RELAY_HTTP_PORTS: [u16; 3] = [8090, 8091, 8092];

/// Docker compose file for distributed relay topology.
pub const DISTRIBUTED_COMPOSE: &str = "tests/chaos/docker-compose.distributed.yml";

/// SSH connection timeout in seconds.
pub const SSH_TIMEOUT_SECS: u64 = 30;

/// Relay startup timeout in seconds (includes Docker build on first run).
pub const RELAY_STARTUP_TIMEOUT_SECS: u64 = 300;

/// Endpoint ID discovery timeout in seconds.
pub const ENDPOINT_ID_TIMEOUT_SECS: u64 = 60;
