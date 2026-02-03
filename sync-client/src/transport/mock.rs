//! Mock transport for testing.
//!
//! Allows queueing responses and capturing sent messages for verification.

use super::{Transport, TransportError};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Mock transport for testing.
///
/// Allows queueing responses and capturing sent messages for verification.
#[derive(Debug, Default)]
pub struct MockTransport {
    inner: Arc<Mutex<MockTransportInner>>,
}

#[derive(Debug, Default)]
struct MockTransportInner {
    connected: bool,
    connected_address: Option<String>,
    sent_messages: Vec<Vec<u8>>,
    receive_queue: VecDeque<Vec<u8>>,
    fail_next_connect: Option<String>,
    fail_next_send: Option<String>,
    fail_next_recv: Option<String>,
}

impl MockTransport {
    /// Create a new mock transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a message to be returned by the next `recv()` call.
    pub fn queue_response(&self, data: Vec<u8>) {
        let mut inner = self.inner.lock().unwrap();
        inner.receive_queue.push_back(data);
    }

    /// Get all messages that were sent.
    pub fn sent_messages(&self) -> Vec<Vec<u8>> {
        let inner = self.inner.lock().unwrap();
        inner.sent_messages.clone()
    }

    /// Get the last message that was sent.
    pub fn last_sent(&self) -> Option<Vec<u8>> {
        let inner = self.inner.lock().unwrap();
        inner.sent_messages.last().cloned()
    }

    /// Get the address that was connected to.
    pub fn connected_address(&self) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        inner.connected_address.clone()
    }

    /// Cause the next connect() to fail with the given error.
    pub fn fail_next_connect(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.fail_next_connect = Some(error.to_string());
    }

    /// Cause the next send() to fail with the given error.
    pub fn fail_next_send(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.fail_next_send = Some(error.to_string());
    }

    /// Cause the next recv() to fail with the given error.
    pub fn fail_next_recv(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.fail_next_recv = Some(error.to_string());
    }

    /// Clear all state (messages, queue, connection).
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        *inner = MockTransportInner::default();
    }
}

impl Clone for MockTransport {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn connect(&self, address: &str) -> Result<(), TransportError> {
        let mut inner = self.inner.lock().unwrap();

        // Check for forced failure
        if let Some(error) = inner.fail_next_connect.take() {
            return Err(TransportError::ConnectionFailed(error));
        }

        inner.connected = true;
        inner.connected_address = Some(address.to_string());
        Ok(())
    }

    async fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.connected {
            return Err(TransportError::NotConnected);
        }

        // Check for forced failure
        if let Some(error) = inner.fail_next_send.take() {
            return Err(TransportError::SendFailed(error));
        }

        inner.sent_messages.push(data.to_vec());
        Ok(())
    }

    async fn recv(&self) -> Result<Vec<u8>, TransportError> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.connected {
            return Err(TransportError::NotConnected);
        }

        // Check for forced failure
        if let Some(error) = inner.fail_next_recv.take() {
            return Err(TransportError::ReceiveFailed(error));
        }

        inner
            .receive_queue
            .pop_front()
            .ok_or(TransportError::ConnectionClosed)
    }

    fn is_connected(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.connected
    }

    async fn close(&self) -> Result<(), TransportError> {
        let mut inner = self.inner.lock().unwrap();
        inner.connected = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // MockTransport Basic Tests
    // ===========================================

    #[tokio::test]
    async fn mock_transport_connects() {
        let transport = MockTransport::new();
        assert!(!transport.is_connected());

        transport.connect("test-node-id").await.unwrap();

        assert!(transport.is_connected());
        assert_eq!(
            transport.connected_address(),
            Some("test-node-id".to_string())
        );
    }

    #[tokio::test]
    async fn mock_transport_sends_messages() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();

        transport.send(b"message 1").await.unwrap();
        transport.send(b"message 2").await.unwrap();

        let sent = transport.sent_messages();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0], b"message 1");
        assert_eq!(sent[1], b"message 2");
    }

    #[tokio::test]
    async fn mock_transport_receives_queued_messages() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();

        transport.queue_response(b"response 1".to_vec());
        transport.queue_response(b"response 2".to_vec());

        let r1 = transport.recv().await.unwrap();
        let r2 = transport.recv().await.unwrap();

        assert_eq!(r1, b"response 1");
        assert_eq!(r2, b"response 2");
    }

    #[tokio::test]
    async fn mock_transport_recv_empty_returns_closed() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();

        let result = transport.recv().await;
        assert!(matches!(result, Err(TransportError::ConnectionClosed)));
    }

    #[tokio::test]
    async fn mock_transport_closes() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();
        assert!(transport.is_connected());

        transport.close().await.unwrap();
        assert!(!transport.is_connected());
    }

    // ===========================================
    // Error Condition Tests
    // ===========================================

    #[tokio::test]
    async fn send_without_connect_fails() {
        let transport = MockTransport::new();

        let result = transport.send(b"data").await;
        assert!(matches!(result, Err(TransportError::NotConnected)));
    }

    #[tokio::test]
    async fn recv_without_connect_fails() {
        let transport = MockTransport::new();

        let result = transport.recv().await;
        assert!(matches!(result, Err(TransportError::NotConnected)));
    }

    #[tokio::test]
    async fn forced_connect_failure() {
        let transport = MockTransport::new();
        transport.fail_next_connect("network unreachable");

        let result = transport.connect("node").await;
        assert!(matches!(result, Err(TransportError::ConnectionFailed(_))));
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn forced_send_failure() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();
        transport.fail_next_send("buffer full");

        let result = transport.send(b"data").await;
        assert!(matches!(result, Err(TransportError::SendFailed(_))));

        // Next send should work
        transport.send(b"data").await.unwrap();
    }

    #[tokio::test]
    async fn forced_recv_failure() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();
        transport.queue_response(b"data".to_vec());
        transport.fail_next_recv("timeout");

        let result = transport.recv().await;
        assert!(matches!(result, Err(TransportError::ReceiveFailed(_))));

        // Next recv should work (and get the queued data)
        let data = transport.recv().await.unwrap();
        assert_eq!(data, b"data");
    }

    // ===========================================
    // Clone and Shared State Tests
    // ===========================================

    #[tokio::test]
    async fn mock_transport_clone_shares_state() {
        let transport1 = MockTransport::new();
        let transport2 = transport1.clone();

        transport1.connect("node").await.unwrap();
        assert!(transport2.is_connected());

        transport1.send(b"from t1").await.unwrap();
        transport2.send(b"from t2").await.unwrap();

        let sent = transport1.sent_messages();
        assert_eq!(sent.len(), 2);
    }

    #[tokio::test]
    async fn mock_transport_reset_clears_all() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();
        transport.send(b"data").await.unwrap();
        transport.queue_response(b"response".to_vec());

        transport.reset();

        assert!(!transport.is_connected());
        assert!(transport.sent_messages().is_empty());
        assert!(transport.connected_address().is_none());
    }

    // ===========================================
    // Last Sent Helper Test
    // ===========================================

    #[tokio::test]
    async fn last_sent_returns_most_recent() {
        let transport = MockTransport::new();
        transport.connect("node").await.unwrap();

        assert!(transport.last_sent().is_none());

        transport.send(b"first").await.unwrap();
        assert_eq!(transport.last_sent(), Some(b"first".to_vec()));

        transport.send(b"second").await.unwrap();
        assert_eq!(transport.last_sent(), Some(b"second".to_vec()));
    }
}
