//! Configuration loading for sync-relay.
//!
//! Configuration is loaded from a TOML file (default: `relay.toml`).

use serde::Deserialize;
use std::path::PathBuf;

/// Root configuration for sync-relay.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server configuration.
    pub server: ServerConfig,
    /// Storage configuration.
    pub storage: StorageConfig,
    /// Rate limiting configuration.
    pub limits: LimitsConfig,
    /// HTTP endpoints configuration.
    pub http: HttpConfig,
    /// Cleanup task configuration.
    pub cleanup: CleanupConfig,
}

/// Server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address for iroh endpoint (default: 0.0.0.0:4433).
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    /// Secret key path for iroh endpoint (optional, generates if missing).
    pub secret_key_path: Option<PathBuf>,
}

/// Storage configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// Path to SQLite database file.
    #[serde(default = "default_database_path")]
    pub database: PathBuf,
    /// Maximum blob size in bytes (default: 1MB).
    #[serde(default = "default_max_blob_size")]
    pub max_blob_size: usize,
    /// Maximum total storage per group in bytes (default: 100MB).
    #[serde(default = "default_max_group_storage")]
    pub max_group_storage: usize,
    /// Default TTL for blobs in seconds (default: 7 days).
    #[serde(default = "default_ttl")]
    pub default_ttl: u64,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    /// Maximum connections per IP address (default: 10).
    #[serde(default = "default_connections_per_ip")]
    pub connections_per_ip: usize,
    /// Maximum messages per device per minute (default: 100).
    #[serde(default = "default_messages_per_minute")]
    pub messages_per_minute: u32,
    /// Timeout in seconds for receiving HELLO after connection (default: 10).
    /// Connections that don't send HELLO within this time are dropped.
    #[serde(default = "default_hello_timeout_secs")]
    pub hello_timeout_secs: u64,
}

/// HTTP endpoints configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    /// Bind address for HTTP server (default: 0.0.0.0:8080).
    #[serde(default = "default_http_bind")]
    pub bind_address: String,
    /// Enable metrics endpoint (default: true).
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
}

/// Cleanup task configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CleanupConfig {
    /// Cleanup interval in seconds (default: 3600 = 1 hour).
    #[serde(default = "default_cleanup_interval")]
    pub interval_secs: u64,
    /// Enable cleanup task (default: true).
    #[serde(default = "default_cleanup_enabled")]
    pub enabled: bool,
}

// Default value functions
fn default_bind_address() -> String {
    "0.0.0.0:4433".to_string()
}

fn default_database_path() -> PathBuf {
    PathBuf::from("relay.db")
}

fn default_max_blob_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_max_group_storage() -> usize {
    100 * 1024 * 1024 // 100MB
}

fn default_ttl() -> u64 {
    7 * 24 * 60 * 60 // 7 days in seconds
}

fn default_connections_per_ip() -> usize {
    10
}

fn default_messages_per_minute() -> u32 {
    100
}

fn default_hello_timeout_secs() -> u64 {
    10
}

fn default_http_bind() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_cleanup_interval() -> u64 {
    3600 // 1 hour
}

fn default_cleanup_enabled() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind_address: default_bind_address(),
                secret_key_path: None,
            },
            storage: StorageConfig {
                database: default_database_path(),
                max_blob_size: default_max_blob_size(),
                max_group_storage: default_max_group_storage(),
                default_ttl: default_ttl(),
            },
            limits: LimitsConfig {
                connections_per_ip: default_connections_per_ip(),
                messages_per_minute: default_messages_per_minute(),
                hello_timeout_secs: default_hello_timeout_secs(),
            },
            http: HttpConfig {
                bind_address: default_http_bind(),
                metrics_enabled: default_metrics_enabled(),
            },
            cleanup: CleanupConfig {
                interval_secs: default_cleanup_interval(),
                enabled: default_cleanup_enabled(),
            },
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            path: path.to_path_buf(),
            source: e,
        })
    }
}

/// Configuration error types.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to read configuration file.
    #[error("failed to read config file {path}: {source}")]
    ReadError {
        /// Path to the configuration file.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Failed to parse configuration file.
    #[error("failed to parse config file {path}: {source}")]
    ParseError {
        /// Path to the configuration file.
        path: PathBuf,
        /// Underlying TOML parse error.
        source: toml::de::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.server.bind_address, "0.0.0.0:4433");
        assert_eq!(config.storage.max_blob_size, 1024 * 1024);
        assert_eq!(config.limits.connections_per_ip, 10);
    }

    #[test]
    fn config_from_toml_string() {
        let toml = r#"
[server]
bind_address = "127.0.0.1:5000"

[storage]
database = "/data/relay.db"
max_blob_size = 2097152

[limits]
connections_per_ip = 5

[http]
bind_address = "0.0.0.0:9090"

[cleanup]
interval_secs = 1800
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.bind_address, "127.0.0.1:5000");
        assert_eq!(config.storage.database, PathBuf::from("/data/relay.db"));
        assert_eq!(config.storage.max_blob_size, 2097152);
        assert_eq!(config.limits.connections_per_ip, 5);
        assert_eq!(config.http.bind_address, "0.0.0.0:9090");
        assert_eq!(config.cleanup.interval_secs, 1800);
    }

    #[test]
    fn hello_timeout_has_default() {
        // F-006: HELLO timeout must have a reasonable default
        let config = Config::default();
        assert_eq!(config.limits.hello_timeout_secs, 10);
    }

    #[test]
    fn hello_timeout_configurable_from_toml() {
        // F-006: HELLO timeout must be configurable
        let toml = r#"
[server]
[storage]
[limits]
hello_timeout_secs = 30
[http]
[cleanup]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.limits.hello_timeout_secs, 30);
    }

    #[test]
    fn config_missing_fields_use_defaults() {
        let toml = r#"
[server]
[storage]
[limits]
[http]
[cleanup]
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.storage.max_blob_size, 1024 * 1024);
        assert_eq!(config.storage.default_ttl, 7 * 24 * 60 * 60);
    }
}
