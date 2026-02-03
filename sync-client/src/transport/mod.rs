//! Transport abstraction for 0k-Sync.
//!
//! This module provides a pluggable transport layer that abstracts
//! the underlying connection mechanism (iroh QUIC, mock for testing).
//!
//! # Design
//!
//! The transport trait is async and connection-oriented:
//! - `connect()` establishes a connection
//! - `send()` transmits encrypted envelope bytes
//! - `recv()` receives envelope bytes
//! - `close()` gracefully terminates
//!
//! # Example
//!
//! ```ignore
//! let transport = MockTransport::new();
//! transport.connect("node_id").await?;
//! transport.send(envelope_bytes).await?;
//! let response = transport.recv().await?;
//! ```

mod mock;

pub use mock::MockTransport;

use async_trait::async_trait;
use thiserror::Error;

/// Transport errors.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Connection failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Not connected.
    #[error("not connected")]
    NotConnected,

    /// Connection closed.
    #[error("connection closed")]
    ConnectionClosed,

    /// Send failed.
    #[error("send failed: {0}")]
    SendFailed(String),

    /// Receive failed.
    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    /// Connection timeout.
    #[error("connection timeout")]
    Timeout,
}

/// Transport trait for sending and receiving sync protocol messages.
///
/// Implementations handle the underlying connection mechanism
/// (iroh QUIC, WebSocket, mock, etc).
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to a relay/peer identified by the given address.
    ///
    /// For iroh, this would be a NodeId. For testing, it's arbitrary.
    async fn connect(&self, address: &str) -> Result<(), TransportError>;

    /// Send bytes over the connection.
    ///
    /// The bytes are typically an encrypted envelope.
    async fn send(&self, data: &[u8]) -> Result<(), TransportError>;

    /// Receive bytes from the connection.
    ///
    /// Blocks until data is available or connection closes.
    async fn recv(&self) -> Result<Vec<u8>, TransportError>;

    /// Check if currently connected.
    fn is_connected(&self) -> bool;

    /// Close the connection gracefully.
    async fn close(&self) -> Result<(), TransportError>;
}
