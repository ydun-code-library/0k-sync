//! sync-relay binary entry point.
//!
//! Usage:
//! ```bash
//! sync-relay --config relay.toml
//! sync-relay --help
//! ```

use std::path::PathBuf;

fn main() {
    // Placeholder - will be replaced with tokio::main and full startup
    println!("sync-relay v{}", env!("CARGO_PKG_VERSION"));
    println!("Configuration file: {:?}", get_config_path());
    println!("Server not yet implemented - Phase 6 in progress");
}

fn get_config_path() -> PathBuf {
    std::env::args()
        .skip_while(|arg| arg != "--config")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("relay.toml"))
}
