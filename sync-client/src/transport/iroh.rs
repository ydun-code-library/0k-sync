//! IrohTransport - Real P2P transport using iroh QUIC.
//!
//! This transport implements the Transport trait using iroh's peer-to-peer
//! QUIC connections with automatic relay and discovery.

use super::{Transport, TransportError};
use async_trait::async_trait;
use iroh::{endpoint::Connection, Endpoint, EndpointAddr, EndpointId};
use std::sync::Arc;
use std::time::Duration;
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
        let endpoint = Endpoint::builder().bind().await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to bind endpoint: {e}"))
        })?;

        Ok(Self {
            endpoint,
            connection: Arc::new(Mutex::new(None)),
            config,
        })
    }

    /// Get our EndpointId for sharing with peers.
    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    /// Get our full EndpointAddr for sharing with peers.
    pub fn endpoint_addr(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    /// Get the endpoint for advanced usage (e.g., accepting connections).
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Parse address string to EndpointAddr.
    ///
    /// Accepts an EndpointId (base32-encoded public key).
    fn parse_address(address: &str) -> Result<EndpointAddr, TransportError> {
        address
            .parse::<EndpointId>()
            .map(EndpointAddr::from)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid EndpointId: {e}")))
    }
}

#[async_trait]
impl Transport for IrohTransport {
    async fn connect(&self, address: &str) -> Result<(), TransportError> {
        let addr = Self::parse_address(address)?;

        // Close existing connection if any
        self.close().await.ok();

        // Connect with timeout
        let conn: Connection = tokio::time::timeout(self.config.connect_timeout, async {
            self.endpoint
                .connect(addr, ALPN)
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
        use iroh::endpoint::ReadExactError;

        let mut guard = self.connection.lock().await;
        let conn = guard.as_mut().ok_or(TransportError::NotConnected)?;

        // Read length prefix (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        conn.recv
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| match e {
                ReadExactError::FinishedEarly(_) => TransportError::ConnectionClosed,
                ReadExactError::ReadError(e) => {
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
    async fn endpoint_id_is_available() {
        let transport = IrohTransport::new().await.unwrap();
        let id = transport.endpoint_id();
        // EndpointId should be a valid 32-byte public key
        assert!(!id.to_string().is_empty());
    }

    #[tokio::test]
    async fn two_transports_have_different_endpoint_ids() {
        let transport1 = IrohTransport::new().await.unwrap();
        let transport2 = IrohTransport::new().await.unwrap();

        assert_ne!(transport1.endpoint_id(), transport2.endpoint_id());
    }

    // ===========================================
    // Integration Tests (Two Endpoints)
    // ===========================================

    /// Echo protocol handler for testing
    #[derive(Debug, Clone)]
    struct EchoProtocol;

    impl iroh::protocol::ProtocolHandler for EchoProtocol {
        async fn accept(
            &self,
            connection: iroh::endpoint::Connection,
        ) -> Result<(), iroh::protocol::AcceptError> {
            use n0_error::StdResultExt;

            let (mut send, mut recv) = connection.accept_bi().await?;

            // Echo server: read length-prefixed message, send it back
            let mut len_buf = [0u8; 4];
            recv.read_exact(&mut len_buf).await.anyerr()?;
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut data = vec![0u8; len];
            recv.read_exact(&mut data).await.anyerr()?;

            // Echo back with same framing
            send.write_all(&len_buf).await.anyerr()?;
            send.write_all(&data).await.anyerr()?;
            send.finish()?;

            Ok(())
        }
    }

    #[tokio::test]
    #[ignore = "Requires relay/discovery - run with --ignored for E2E tests"]
    async fn two_endpoints_can_connect() {
        use iroh::protocol::Router;

        // Create accepting endpoint with Router
        let acceptor = IrohTransport::new().await.unwrap();
        let acceptor_id = acceptor.endpoint_id().to_string();

        // Set up Router to accept connections with our echo protocol
        let router = Router::builder(acceptor.endpoint().clone())
            .accept(ALPN, EchoProtocol)
            .spawn();

        // Create connecting transport
        let connector = IrohTransport::new().await.unwrap();

        // Connect to acceptor (requires relay/discovery for address resolution)
        connector.connect(&acceptor_id).await.unwrap();
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
