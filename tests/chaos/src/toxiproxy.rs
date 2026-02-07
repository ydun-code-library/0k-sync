//! Toxiproxy HTTP API client for network chaos injection.
//!
//! Toxiproxy is a framework for simulating network conditions. This module
//! provides a client for its HTTP API to add/remove toxics during tests.
//!
//! **Note:** Toxiproxy only supports TCP. For QUIC/UDP chaos, use the
//! `netem` module with `tc qdisc` instead. Toxiproxy is still useful for
//! chaosing the relay's HTTP health/metrics endpoint.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when interacting with Toxiproxy.
#[derive(Debug, Error)]
pub enum ToxiproxyError {
    /// HTTP request failed
    #[error("http error: {0}")]
    Http(String),

    /// Proxy not found
    #[error("proxy not found: {0}")]
    ProxyNotFound(String),

    /// Invalid toxic configuration
    #[error("invalid toxic: {0}")]
    InvalidToxic(String),

    /// Connection to Toxiproxy failed
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
}

impl From<reqwest::Error> for ToxiproxyError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() {
            ToxiproxyError::ConnectionFailed(e.to_string())
        } else {
            ToxiproxyError::Http(e.to_string())
        }
    }
}

/// Direction for a toxic (upstream = client→server, downstream = server→client).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToxicDirection {
    /// Client to server
    Upstream,
    /// Server to client
    Downstream,
}

/// A toxic that can be applied to a proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toxic {
    /// Unique name for this toxic
    pub name: String,
    /// Type of toxic (latency, bandwidth, etc.)
    #[serde(rename = "type")]
    pub toxic_type: String,
    /// Direction to apply toxic
    pub stream: ToxicDirection,
    /// Probability of toxic being applied (0.0-1.0)
    pub toxicity: f64,
    /// Toxic-specific attributes
    pub attributes: ToxicAttributes,
}

/// Attributes for different toxic types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToxicAttributes {
    /// Latency toxic
    Latency {
        /// Latency in milliseconds
        latency: u64,
        /// Jitter in milliseconds
        jitter: u64,
    },
    /// Bandwidth toxic
    Bandwidth {
        /// Rate in KB/s
        rate: u64,
    },
    /// Packet loss toxic
    SlowClose {
        /// Delay in milliseconds before closing
        delay: u64,
    },
    /// Timeout toxic
    Timeout {
        /// Timeout in milliseconds
        timeout: u64,
    },
    /// Reset peer toxic
    ResetPeer {
        /// Timeout before reset in milliseconds
        timeout: u64,
    },
    /// Empty attributes
    Empty {},
}

/// Toxiproxy client configuration.
#[derive(Debug, Clone)]
pub struct ToxiproxyConfig {
    /// Base URL for Toxiproxy API
    pub base_url: String,
}

impl Default for ToxiproxyConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8474".into(),
        }
    }
}

/// Client for the Toxiproxy HTTP API.
pub struct ToxiproxyClient {
    config: ToxiproxyConfig,
    http: reqwest::Client,
}

impl ToxiproxyClient {
    /// Create a new Toxiproxy client.
    pub fn new(config: ToxiproxyConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Build the URL for a proxy's toxics endpoint.
    pub fn toxics_url(&self, proxy_name: &str) -> String {
        format!("{}/proxies/{}/toxics", self.config.base_url, proxy_name)
    }

    /// Build the URL for a specific toxic.
    pub fn toxic_url(&self, proxy_name: &str, toxic_name: &str) -> String {
        format!(
            "{}/proxies/{}/toxics/{}",
            self.config.base_url, proxy_name, toxic_name
        )
    }

    /// Add a toxic to a proxy.
    pub async fn add_toxic(&self, proxy_name: &str, toxic: &Toxic) -> Result<(), ToxiproxyError> {
        let url = self.toxics_url(proxy_name);
        let response = self.http.post(&url).json(toxic).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ToxiproxyError::ProxyNotFound(proxy_name.to_string()));
        }

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToxiproxyError::InvalidToxic(body));
        }

        Ok(())
    }

    /// Remove a toxic from a proxy.
    pub async fn remove_toxic(
        &self,
        proxy_name: &str,
        toxic_name: &str,
    ) -> Result<(), ToxiproxyError> {
        let url = self.toxic_url(proxy_name, toxic_name);
        let response = self.http.delete(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ToxiproxyError::ProxyNotFound(format!(
                "{}/{}",
                proxy_name, toxic_name
            )));
        }

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToxiproxyError::Http(body));
        }

        Ok(())
    }

    /// List all toxics on a proxy.
    pub async fn list_toxics(&self, proxy_name: &str) -> Result<Vec<Toxic>, ToxiproxyError> {
        let url = self.toxics_url(proxy_name);
        let response = self.http.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ToxiproxyError::ProxyNotFound(proxy_name.to_string()));
        }

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToxiproxyError::Http(body));
        }

        let toxics: Vec<Toxic> = response.json().await?;
        Ok(toxics)
    }
}

/// Helper to create common toxics.
pub mod toxics {
    use super::*;

    /// Create a latency toxic.
    pub fn latency(
        name: &str,
        direction: ToxicDirection,
        latency_ms: u64,
        jitter_ms: u64,
    ) -> Toxic {
        Toxic {
            name: name.into(),
            toxic_type: "latency".into(),
            stream: direction,
            toxicity: 1.0,
            attributes: ToxicAttributes::Latency {
                latency: latency_ms,
                jitter: jitter_ms,
            },
        }
    }

    /// Create a bandwidth toxic.
    pub fn bandwidth(name: &str, direction: ToxicDirection, rate_kbps: u64) -> Toxic {
        Toxic {
            name: name.into(),
            toxic_type: "bandwidth".into(),
            stream: direction,
            toxicity: 1.0,
            attributes: ToxicAttributes::Bandwidth { rate: rate_kbps },
        }
    }

    /// Create a timeout toxic.
    pub fn timeout(name: &str, direction: ToxicDirection, timeout_ms: u64) -> Toxic {
        Toxic {
            name: name.into(),
            toxic_type: "timeout".into(),
            stream: direction,
            toxicity: 1.0,
            attributes: ToxicAttributes::Timeout {
                timeout: timeout_ms,
            },
        }
    }

    /// Create a reset peer toxic.
    pub fn reset_peer(name: &str, direction: ToxicDirection, timeout_ms: u64) -> Toxic {
        Toxic {
            name: name.into(),
            toxic_type: "reset_peer".into(),
            stream: direction,
            toxicity: 1.0,
            attributes: ToxicAttributes::ResetPeer {
                timeout: timeout_ms,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default() {
        let config = ToxiproxyConfig::default();
        assert_eq!(config.base_url, "http://localhost:8474");
    }

    #[test]
    fn client_urls() {
        let client = ToxiproxyClient::new(ToxiproxyConfig::default());

        assert_eq!(
            client.toxics_url("relay"),
            "http://localhost:8474/proxies/relay/toxics"
        );

        assert_eq!(
            client.toxic_url("relay", "latency-downstream"),
            "http://localhost:8474/proxies/relay/toxics/latency-downstream"
        );
    }

    #[test]
    fn latency_toxic() {
        let toxic = toxics::latency("test-latency", ToxicDirection::Downstream, 100, 20);

        assert_eq!(toxic.name, "test-latency");
        assert_eq!(toxic.toxic_type, "latency");
        assert_eq!(toxic.stream, ToxicDirection::Downstream);

        if let ToxicAttributes::Latency { latency, jitter } = toxic.attributes {
            assert_eq!(latency, 100);
            assert_eq!(jitter, 20);
        } else {
            panic!("Expected Latency attributes");
        }
    }

    #[test]
    fn bandwidth_toxic() {
        let toxic = toxics::bandwidth("test-bw", ToxicDirection::Upstream, 1024);

        assert_eq!(toxic.toxic_type, "bandwidth");
        if let ToxicAttributes::Bandwidth { rate } = toxic.attributes {
            assert_eq!(rate, 1024);
        } else {
            panic!("Expected Bandwidth attributes");
        }
    }

    #[test]
    fn toxic_serializes() {
        let toxic = toxics::latency("test", ToxicDirection::Downstream, 50, 10);
        let json = serde_json::to_string(&toxic).unwrap();
        assert!(json.contains("\"latency\":50"));
        assert!(json.contains("\"jitter\":10"));
    }
}
