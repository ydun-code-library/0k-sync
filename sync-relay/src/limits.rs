//! Rate limiting for sync-relay.
//!
//! Provides protection against connection flooding and message spam.
//!
//! ## Design Notes
//!
//! In iroh, connections may come through relay servers, so we cannot reliably
//! identify clients by IP address. Instead, we rate limit by:
//! - **EndpointId** (32-byte public key) for connection attempts
//! - **DeviceId** (32-byte identifier) for message operations
//!
//! Both use the governor crate's keyed rate limiters backed by DashMap.

use crate::config::LimitsConfig;
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Type alias for a keyed rate limiter using DashMap.
type KeyedLimiter<K> = RateLimiter<
    K,
    dashmap::DashMap<K, InMemoryState>,
    DefaultClock,
    NoOpMiddleware<governor::clock::QuantaInstant>,
>;

/// Type alias for a direct (non-keyed) rate limiter.
type DirectLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Rate limiters for the relay server.
#[derive(Clone)]
pub struct RateLimits {
    /// Limits connection attempts per EndpointId.
    ///
    /// Configured via `limits.connections_per_ip` (repurposed for device identity
    /// since iroh abstracts over IP addresses).
    connection_limiter: Arc<KeyedLimiter<[u8; 32]>>,

    /// Limits message operations per DeviceId.
    ///
    /// Configured via `limits.messages_per_minute`.
    message_limiter: Arc<KeyedLimiter<[u8; 32]>>,

    /// Global rate limiter across all clients (F-014).
    ///
    /// Prevents aggregate overload even if individual clients are within limits.
    global_limiter: Arc<DirectLimiter>,
}

impl std::fmt::Debug for RateLimits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimits")
            .field("connection_limiter", &"KeyedLimiter<[u8;32]>")
            .field("message_limiter", &"KeyedLimiter<[u8;32]>")
            .field("global_limiter", &"DirectLimiter")
            .finish()
    }
}

impl RateLimits {
    /// Create rate limiters from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Rate limiting configuration
    ///
    /// # Panics
    ///
    /// Panics if the configured values are zero.
    pub fn new(config: &LimitsConfig) -> Self {
        // Connection rate: allow `connections_per_ip` per minute
        // (e.g., 10 connections/minute = 1 every 6 seconds)
        let connections_per_minute = NonZeroU32::new(config.connections_per_ip as u32)
            .expect("connections_per_ip must be > 0");
        let connection_quota = Quota::per_minute(connections_per_minute);

        // Message rate: allow `messages_per_minute` per minute
        let messages_per_minute =
            NonZeroU32::new(config.messages_per_minute).expect("messages_per_minute must be > 0");
        let message_quota = Quota::per_minute(messages_per_minute);

        // Global rate: allow `global_requests_per_second` per second across all clients
        let global_rps = NonZeroU32::new(config.global_requests_per_second)
            .expect("global_requests_per_second must be > 0");
        let global_quota = Quota::per_second(global_rps);

        Self {
            connection_limiter: Arc::new(RateLimiter::keyed(connection_quota)),
            message_limiter: Arc::new(RateLimiter::keyed(message_quota)),
            global_limiter: Arc::new(RateLimiter::direct(global_quota)),
        }
    }

    /// Check if a connection attempt is allowed.
    ///
    /// # Arguments
    ///
    /// * `endpoint_id` - The 32-byte EndpointId (public key) of the connecting device
    ///
    /// # Returns
    ///
    /// `Ok(())` if allowed, `Err` with reason if rate limited.
    pub fn check_connection(&self, endpoint_id: &[u8; 32]) -> Result<(), RateLimitError> {
        self.connection_limiter
            .check_key(endpoint_id)
            .map_err(|_| RateLimitError::ConnectionLimitExceeded)
    }

    /// Check if a message operation is allowed.
    ///
    /// # Arguments
    ///
    /// * `device_id` - The 32-byte DeviceId of the device sending the message
    ///
    /// # Returns
    ///
    /// `Ok(())` if allowed, `Err` with reason if rate limited.
    pub fn check_message(&self, device_id: &[u8; 32]) -> Result<(), RateLimitError> {
        self.message_limiter
            .check_key(device_id)
            .map_err(|_| RateLimitError::MessageLimitExceeded)
    }

    /// Check if the global request rate is within limits.
    ///
    /// This is a server-wide rate limit that caps aggregate throughput
    /// regardless of individual client limits.
    pub fn check_global(&self) -> Result<(), RateLimitError> {
        self.global_limiter
            .check()
            .map_err(|_| RateLimitError::GlobalLimitExceeded)
    }

    /// Get the number of tracked connection keys (for metrics).
    pub fn connection_keys_count(&self) -> usize {
        self.connection_limiter.len()
    }

    /// Get the number of tracked message keys (for metrics).
    pub fn message_keys_count(&self) -> usize {
        self.message_limiter.len()
    }

    /// Evict stale entries from the keyed rate limiter DashMaps (F-015).
    ///
    /// Over time, disconnected clients leave entries in the DashMap.
    /// `retain_recent()` removes entries whose rate limit cells have fully
    /// recharged (i.e., idle clients). Call periodically from cleanup task.
    pub fn shrink(&self) {
        self.connection_limiter.retain_recent();
        self.message_limiter.retain_recent();
    }
}

/// Rate limit error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitError {
    /// Too many connection attempts from this device.
    ConnectionLimitExceeded,
    /// Too many messages from this device.
    MessageLimitExceeded,
    /// Global request rate exceeded across all clients (F-014).
    GlobalLimitExceeded,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionLimitExceeded => {
                write!(f, "connection rate limit exceeded")
            }
            Self::MessageLimitExceeded => {
                write!(f, "message rate limit exceeded")
            }
            Self::GlobalLimitExceeded => {
                write!(f, "global rate limit exceeded")
            }
        }
    }
}

impl std::error::Error for RateLimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LimitsConfig {
        LimitsConfig {
            connections_per_ip: 5,
            messages_per_minute: 10,
            hello_timeout_secs: 10,
            max_concurrent_sessions: 10_000,
            max_device_name_len: 256,
            max_pull_limit: 1000,
            global_requests_per_second: 1000,
        }
    }

    #[test]
    fn create_rate_limits() {
        let limits = RateLimits::new(&test_config());
        assert_eq!(limits.connection_keys_count(), 0);
        assert_eq!(limits.message_keys_count(), 0);
    }

    #[test]
    fn connection_limit_allows_within_quota() {
        let config = LimitsConfig {
            connections_per_ip: 5,
            messages_per_minute: 100,
            hello_timeout_secs: 10,
            max_concurrent_sessions: 10_000,
            max_device_name_len: 256,
            max_pull_limit: 1000,
            global_requests_per_second: 1000,
        };
        let limits = RateLimits::new(&config);
        let endpoint_id = [1u8; 32];

        // First 5 should succeed
        for _ in 0..5 {
            assert!(limits.check_connection(&endpoint_id).is_ok());
        }

        // 6th should fail
        assert_eq!(
            limits.check_connection(&endpoint_id),
            Err(RateLimitError::ConnectionLimitExceeded)
        );
    }

    #[test]
    fn message_limit_allows_within_quota() {
        let config = LimitsConfig {
            connections_per_ip: 100,
            messages_per_minute: 5,
            hello_timeout_secs: 10,
            max_concurrent_sessions: 10_000,
            max_device_name_len: 256,
            max_pull_limit: 1000,
            global_requests_per_second: 1000,
        };
        let limits = RateLimits::new(&config);
        let device_id = [2u8; 32];

        // First 5 should succeed
        for _ in 0..5 {
            assert!(limits.check_message(&device_id).is_ok());
        }

        // 6th should fail
        assert_eq!(
            limits.check_message(&device_id),
            Err(RateLimitError::MessageLimitExceeded)
        );
    }

    #[test]
    fn different_keys_have_independent_limits() {
        let config = LimitsConfig {
            connections_per_ip: 2,
            messages_per_minute: 2,
            hello_timeout_secs: 10,
            max_concurrent_sessions: 10_000,
            max_device_name_len: 256,
            max_pull_limit: 1000,
            global_requests_per_second: 1000,
        };
        let limits = RateLimits::new(&config);

        let device_a = [1u8; 32];
        let device_b = [2u8; 32];

        // Device A uses its quota
        assert!(limits.check_message(&device_a).is_ok());
        assert!(limits.check_message(&device_a).is_ok());
        assert!(limits.check_message(&device_a).is_err());

        // Device B still has full quota
        assert!(limits.check_message(&device_b).is_ok());
        assert!(limits.check_message(&device_b).is_ok());
        assert!(limits.check_message(&device_b).is_err());
    }

    #[test]
    fn rate_limits_are_clone() {
        let limits = RateLimits::new(&test_config());
        let _cloned = limits.clone();
    }

    #[test]
    fn rate_limits_are_debug() {
        let limits = RateLimits::new(&test_config());
        let debug = format!("{:?}", limits);
        assert!(debug.contains("RateLimits"));
    }

    #[test]
    fn rate_limit_error_display() {
        assert_eq!(
            RateLimitError::ConnectionLimitExceeded.to_string(),
            "connection rate limit exceeded"
        );
        assert_eq!(
            RateLimitError::MessageLimitExceeded.to_string(),
            "message rate limit exceeded"
        );
        assert_eq!(
            RateLimitError::GlobalLimitExceeded.to_string(),
            "global rate limit exceeded"
        );
    }

    #[test]
    fn global_rate_limiter_rejects_excess() {
        // F-014: Global rate limiter must cap aggregate throughput
        let config = LimitsConfig {
            connections_per_ip: 100,
            messages_per_minute: 100,
            hello_timeout_secs: 10,
            max_concurrent_sessions: 10_000,
            max_device_name_len: 256,
            max_pull_limit: 1000,
            global_requests_per_second: 5,
        };
        let limits = RateLimits::new(&config);

        // First 5 should succeed
        for _ in 0..5 {
            assert!(limits.check_global().is_ok());
        }

        // 6th should fail
        assert_eq!(
            limits.check_global(),
            Err(RateLimitError::GlobalLimitExceeded)
        );
    }

    #[test]
    fn shrink_does_not_panic() {
        // F-015: Shrink via retain_recent() must be callable without error.
        // retain_recent() evicts entries whose rate limit cells have fully
        // recharged (idle clients). Freshly-used entries are kept.
        let limits = RateLimits::new(&test_config());

        // Create some entries
        let endpoint_a = [1u8; 32];
        let endpoint_b = [2u8; 32];
        let _ = limits.check_connection(&endpoint_a);
        let _ = limits.check_connection(&endpoint_b);
        let _ = limits.check_message(&endpoint_a);

        assert!(limits.connection_keys_count() > 0);

        // Shrink should not panic — freshly used entries may or may not
        // be evicted depending on timing, so we only assert no panic
        limits.shrink();
    }
}
