//! IrohTransport - Real P2P transport using iroh QUIC.
//!
//! This transport implements the Transport trait using iroh's peer-to-peer
//! QUIC connections with automatic relay and discovery.

use super::{Transport, TransportError};
use async_trait::async_trait;
use iroh::{endpoint::Connection, Endpoint, NodeId};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

/// Protocol identifier for 0k-Sync over iroh.
pub const ALPN: &[u8] = b"/0k-sync/1";

/// Maximum message size (1MB per blob limit from spec).
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Configuration for IrohTransport.
#[derive(Clone, Debug)]
pub struct IrohTransportConfig {
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Send/recv operation timeout.
    pub operation_timeout: Duration,
}

impl Default for IrohTransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            operation_timeout: Duration::from_secs(60),
        }
    }
}

/// Active connection state including bidirectional stream.
struct ActiveConnection {
    /// The QUIC connection.
    #[allow(dead_code)]
    conn: Connection,
    /// Send half of bidirectional stream.
    send: iroh::endpoint::SendStream,
    /// Receive half of bidirectional stream.
    recv: iroh::endpoint::RecvStream,
}

/// IrohTransport implements the Transport trait using iroh QUIC connections.
///
/// # Example
///
/// ```ignore
/// let transport = IrohTransport::new().await?;
/// println!("My NodeId: {}", transport.node_id());
///
/// transport.connect("peer-node-id").await?;
/// transport.send(b"hello").await?;
/// let response = transport.recv().await?;
/// ```
pub struct IrohTransport {
    /// Our local endpoint for making connections.
    endpoint: Endpoint,
    /// Active connection (if connected).
    connection: Arc<Mutex<Option<ActiveConnection>>>,
    /// Configuration options.
    config: IrohTransportConfig,
}

impl IrohTransport {
    /// Create a new IrohTransport.
    ///
    /// This binds to the network and may take 0-3 seconds to probe relays.
    pub async fn new() -> Result<Self, TransportError> {
        Self::with_config(IrohTransportConfig::default()).await
    }

    /// Create a new IrohTransport with custom configuration.
    pub async fn with_config(config: IrohTransportConfig) -> Result<Self, TransportError> {
        let endpoint = Endpoint::builder()
            .bind()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to bind endpoint: {e}")))?;

        Ok(Self {
            endpoint,
            connection: Arc::new(Mutex::new(None)),
            config,
        })
    }

    /// Get our NodeId for sharing with peers.
    pub fn node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    /// Get the endpoint for advanced usage (e.g., accepting connections).
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Parse address string to NodeId.
    fn parse_address(address: &str) -> Result<NodeId, TransportError> {
        address
            .parse::<NodeId>()
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid NodeId: {e}")))
    }
}

#[async_trait]
impl Transport for IrohTransport {
    async fn connect(&self, address: &str) -> Result<(), TransportError> {
        let node_id = Self::parse_address(address)?;

        // Close existing connection if any
        self.close().await.ok();

        // Connect with timeout
        let conn = tokio::time::timeout(self.config.connect_timeout, async {
            self.endpoint
                .connect(node_id, ALPN)
                .await
                .map_err(|e| TransportError::ConnectionFailed(format!("Connection failed: {e}")))
        })
        .await
        .map_err(|_| TransportError::Timeout)??;

        // Open bidirectional stream for request-response
        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to open stream: {e}")))?;

        // Store connection state
        let mut state = self.connection.lock().await;
        *state = Some(ActiveConnection { conn, send, recv });

        Ok(())
    }

    async fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        // Validate message size
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(TransportError::SendFailed(format!(
                "Message too large: {} > {}",
                data.len(),
                MAX_MESSAGE_SIZE
            )));
        }

        let mut guard = self.connection.lock().await;
        let conn = guard.as_mut().ok_or(TransportError::NotConnected)?;

        // Length-prefixed framing (4 bytes, big-endian)
        let len = (data.len() as u32).to_be_bytes();
        conn.send
            .write_all(&len)
            .await
            .map_err(|e| TransportError::SendFailed(format!("Failed to write length: {e}")))?;

        // Write payload
        conn.send
            .write_all(data)
            .await
            .map_err(|e| TransportError::SendFailed(format!("Failed to write data: {e}")))?;

        Ok(())
    }

    async fn recv(&self) -> Result<Vec<u8>, TransportError> {
        let mut guard = self.connection.lock().await;
        let conn = guard.as_mut().ok_or(TransportError::NotConnected)?;

        // Read length prefix (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        conn.recv.read_exact(&mut len_buf).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                TransportError::ConnectionClosed
            } else {
                TransportError::ReceiveFailed(format!("Failed to read length: {e}"))
            }
        })?;

        let len = u32::from_be_bytes(len_buf) as usize;

        // Validate length
        if len > MAX_MESSAGE_SIZE {
            return Err(TransportError::ReceiveFailed(format!(
                "Message too large: {} > {}",
                len, MAX_MESSAGE_SIZE
            )));
        }

        // Read payload
        let mut data = vec![0u8; len];
        conn.recv
            .read_exact(&mut data)
            .await
            .map_err(|e| TransportError::ReceiveFailed(format!("Failed to read data: {e}")))?;

        Ok(data)
    }

    fn is_connected(&self) -> bool {
        self.connection
            .try_lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    async fn close(&self) -> Result<(), TransportError> {
        if let Some(mut conn) = self.connection.lock().await.take() {
            // Signal end of stream
            conn.send.finish().ok();
            // Close connection gracefully
            conn.conn.close(0u32.into(), b"closing");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // Trait Compliance Tests
    // ===========================================

    #[test]
    fn iroh_transport_implements_transport_trait() {
        fn assert_transport<T: Transport>() {}
        assert_transport::<IrohTransport>();
    }

    // ===========================================
    // Address Parsing Tests
    // ===========================================

    #[test]
    fn parse_invalid_node_id_returns_error() {
        let result = IrohTransport::parse_address("not-a-valid-node-id");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TransportError::ConnectionFailed(_)));
    }

    #[test]
    fn parse_empty_address_returns_error() {
        let result = IrohTransport::parse_address("");
        assert!(result.is_err());
    }

    // ===========================================
    // Message Size Validation Tests
    // ===========================================

    #[tokio::test]
    async fn send_oversized_message_fails() {
        let transport = IrohTransport::new().await.unwrap();

        // Create oversized message (> 1MB)
        let oversized = vec![0u8; MAX_MESSAGE_SIZE + 1];

        // This should fail even without connection due to size check
        let result = transport.send(&oversized).await;

        // Should fail with SendFailed (size check happens before connection check)
        assert!(matches!(result, Err(TransportError::SendFailed(_))));
    }

    // ===========================================
    // Connection State Tests
    // ===========================================

    #[tokio::test]
    async fn not_connected_initially() {
        let transport = IrohTransport::new().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn send_without_connect_fails() {
        let transport = IrohTransport::new().await.unwrap();
        let result = transport.send(b"data").await;
        assert!(matches!(result, Err(TransportError::NotConnected)));
    }

    #[tokio::test]
    async fn recv_without_connect_fails() {
        let transport = IrohTransport::new().await.unwrap();
        let result = transport.recv().await;
        assert!(matches!(result, Err(TransportError::NotConnected)));
    }

    #[tokio::test]
    async fn close_without_connect_succeeds() {
        let transport = IrohTransport::new().await.unwrap();
        let result = transport.close().await;
        assert!(result.is_ok());
    }

    // ===========================================
    // NodeId Access Tests
    // ===========================================

    #[tokio::test]
    async fn node_id_is_available() {
        let transport = IrohTransport::new().await.unwrap();
        let node_id = transport.node_id();
        // NodeId should be a valid 32-byte public key
        assert!(!node_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn two_transports_have_different_node_ids() {
        let transport1 = IrohTransport::new().await.unwrap();
        let transport2 = IrohTransport::new().await.unwrap();

        assert_ne!(transport1.node_id(), transport2.node_id());
    }

    // ===========================================
    // Integration Tests (Two Endpoints)
    // ===========================================

    #[tokio::test]
    async fn two_endpoints_can_connect() {
        use iroh::protocol::Router;

        // Create accepting endpoint with Router
        let acceptor = IrohTransport::new().await.unwrap();
        let acceptor_node_id = acceptor.node_id().to_string();

        // Set up Router to accept connections
        let router = Router::builder(acceptor.endpoint().clone())
            .accept(ALPN, |incoming| {
                Box::pin(async move {
                    let conn = incoming.await?;
                    let (mut send, mut recv) = conn.accept_bi().await?;

                    // Echo server: read message, send it back
                    let mut len_buf = [0u8; 4];
                    recv.read_exact(&mut len_buf).await?;
                    let len = u32::from_be_bytes(len_buf) as usize;
                    let mut data = vec![0u8; len];
                    recv.read_exact(&mut data).await?;

                    // Echo back
                    send.write_all(&len_buf).await?;
                    send.write_all(&data).await?;
                    send.finish()?;

                    Ok(())
                })
            })
            .spawn()
            .await
            .unwrap();

        // Create connecting transport
        let connector = IrohTransport::new().await.unwrap();

        // Connect to acceptor
        connector.connect(&acceptor_node_id).await.unwrap();
        assert!(connector.is_connected());

        // Send a message
        let message = b"Hello, iroh!";
        connector.send(message).await.unwrap();

        // Receive echo
        let response = connector.recv().await.unwrap();
        assert_eq!(response, message);

        // Cleanup
        connector.close().await.unwrap();
        router.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connection_close_is_graceful() {
        let transport = IrohTransport::new().await.unwrap();

        // Connect to nothing (will fail, but that's ok for this test)
        // We're just testing that close() works

        transport.close().await.unwrap();
        assert!(!transport.is_connected());

        // Close again should still succeed
        transport.close().await.unwrap();
    }

    // ===========================================
    // Framing Tests
    // ===========================================

    #[test]
    fn length_prefix_encodes_correctly() {
        // Test that our framing format is correct
        let len: u32 = 1024;
        let bytes = len.to_be_bytes();
        assert_eq!(bytes, [0, 0, 4, 0]); // 1024 in big-endian

        let decoded = u32::from_be_bytes(bytes);
        assert_eq!(decoded, 1024);
    }

    #[test]
    fn max_message_size_is_one_megabyte() {
        assert_eq!(MAX_MESSAGE_SIZE, 1024 * 1024);
    }
}
