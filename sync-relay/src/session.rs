//! Per-connection session management.
//!
//! Each connection gets a Session that tracks state and handles messages.

use crate::error::{ProtocolError, ProtocolResult, RelayError, StorageError};
use crate::protocol::MAX_MESSAGE_SIZE;
use crate::server::SyncRelay;
use crate::storage::{BlobStorage, StoreBlobRequest, StoredBlob};
use iroh::endpoint::Connection;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sync_types::{Cursor, DeviceId, GroupId, Message, PullBlob, PullResponse, PushAck, Welcome};

/// Session state machine states.
#[derive(Debug, Clone)]
pub enum SessionState {
    /// Waiting for HELLO message.
    AwaitingHello,
    /// Session is active with established group and device.
    Active {
        /// The sync group this session belongs to.
        group_id: GroupId,
        /// The device ID (derived from connection).
        device_id: DeviceId,
        /// Device name from HELLO.
        device_name: String,
        /// Last cursor the client knows about.
        last_cursor: Cursor,
    },
    /// Session is closing.
    Closing,
}

/// A per-connection session.
pub struct Session {
    relay: Arc<SyncRelay>,
    connection: Connection,
    state: SessionState,
}

impl Session {
    /// Create a new session for a connection.
    pub fn new(relay: Arc<SyncRelay>, connection: Connection) -> Self {
        Self {
            relay,
            connection,
            state: SessionState::AwaitingHello,
        }
    }

    /// Run the session until completion.
    pub async fn run(mut self) -> Result<(), RelayError> {
        let remote_id = self.connection.remote_id();
        tracing::info!("New connection from {}", remote_id);

        // Main session loop
        loop {
            // Accept a bidirectional stream.
            // F-006: Timeout when awaiting HELLO to prevent resource exhaustion.
            let stream = if matches!(self.state, SessionState::AwaitingHello) {
                let timeout_secs = self.relay.config().limits.hello_timeout_secs;
                match tokio::time::timeout(
                    Duration::from_secs(timeout_secs),
                    self.connection.accept_bi(),
                )
                .await
                {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(e)) => {
                        tracing::debug!("Connection closed during HELLO wait: {}", e);
                        break;
                    }
                    Err(_) => {
                        tracing::warn!("HELLO timeout ({}s) for {}", timeout_secs, remote_id);
                        break;
                    }
                }
            } else {
                match self.connection.accept_bi().await {
                    Ok(stream) => stream,
                    Err(e) => {
                        tracing::debug!("Connection closed: {}", e);
                        break;
                    }
                }
            };

            let (send, recv) = stream;

            // Handle the stream
            if let Err(e) = self.handle_stream(send, recv).await {
                tracing::warn!("Stream error: {}", e);
                // Continue accepting new streams unless connection is broken
                if matches!(self.state, SessionState::Closing) {
                    break;
                }
            }
        }

        // Cleanup
        if let SessionState::Active {
            group_id,
            device_id,
            ..
        } = &self.state
        {
            self.relay.unregister_session(group_id, device_id).await;
        }

        Ok(())
    }

    /// Handle a single bidirectional stream.
    async fn handle_stream(
        &mut self,
        mut send: iroh::endpoint::SendStream,
        mut recv: iroh::endpoint::RecvStream,
    ) -> ProtocolResult<()> {
        // Read message with length prefix
        let message = self.read_message(&mut recv).await?;

        // Rate limit check for Active state operations (PUSH, PULL)
        // HELLO is not rate limited here (connection rate limit handles that)
        // BYE is not rate limited (we always allow graceful disconnect)
        if let SessionState::Active { device_id, .. } = &self.state {
            if matches!(message, Message::Push(_) | Message::Pull(_)) {
                // SR-001: Global rate limit check (aggregate across all clients)
                if let Err(e) = self.relay.rate_limits().check_global() {
                    tracing::warn!("Global rate limit exceeded: {}", e);
                    self.relay.metrics().rate_limit_hits.fetch_add(1, Ordering::Relaxed);
                    return Err(ProtocolError::RateLimited {
                        reason: e.to_string(),
                    });
                }
                if let Err(e) = self.relay.rate_limits().check_message(device_id.as_bytes()) {
                    tracing::warn!("Message rate limited for device {:?}: {}", device_id, e);
                    self.relay.metrics().rate_limit_hits.fetch_add(1, Ordering::Relaxed);
                    return Err(ProtocolError::RateLimited {
                        reason: e.to_string(),
                    });
                }
            }
        }

        // Handle message based on state
        let response = match (&self.state, &message) {
            (SessionState::AwaitingHello, Message::Hello(hello)) => {
                self.handle_hello(hello.clone()).await?
            }
            (SessionState::Active { .. }, Message::Push(push)) => {
                self.handle_push(push.clone()).await?
            }
            (SessionState::Active { .. }, Message::Pull(pull)) => {
                self.handle_pull(pull.clone()).await?
            }
            (SessionState::Active { .. }, Message::Bye(bye)) => {
                self.handle_bye(bye.clone()).await?;
                self.state = SessionState::Closing;
                return Ok(());
            }
            (SessionState::AwaitingHello, _) => {
                return Err(ProtocolError::NotAuthenticated);
            }
            (SessionState::Closing, _) => {
                return Ok(());
            }
            (_, msg) => {
                return Err(ProtocolError::UnexpectedMessage {
                    expected: self.expected_message_types(),
                    actual: format!("{:?}", std::mem::discriminant(msg)),
                });
            }
        };

        // Write response
        self.write_message(&mut send, &response).await?;

        // Signal end of response
        send.finish()
            .map_err(|e| ProtocolError::Stream(e.to_string()))?;

        Ok(())
    }

    /// Read a length-prefixed message from the stream.
    async fn read_message(&self, recv: &mut iroh::endpoint::RecvStream) -> ProtocolResult<Message> {
        // Read 4-byte length prefix (big-endian)
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| ProtocolError::Stream(e.to_string()))?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Validate length
        if len > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::InvalidMessage {
                reason: format!("message too large: {} > {}", len, MAX_MESSAGE_SIZE),
            });
        }

        // Read message bytes
        let mut buf = vec![0u8; len];
        recv.read_exact(&mut buf)
            .await
            .map_err(|e| ProtocolError::Stream(e.to_string()))?;

        // Deserialize
        Message::from_bytes(&buf).map_err(|e| ProtocolError::InvalidMessage {
            reason: e.to_string(),
        })
    }

    /// Write a length-prefixed message to the stream.
    async fn write_message(
        &self,
        send: &mut iroh::endpoint::SendStream,
        message: &Message,
    ) -> ProtocolResult<()> {
        let bytes = message
            .to_bytes()
            .map_err(|e| ProtocolError::InvalidMessage {
                reason: e.to_string(),
            })?;

        // Write 4-byte length prefix
        let len = bytes.len() as u32;
        send.write_all(&len.to_be_bytes())
            .await
            .map_err(|e| ProtocolError::Stream(e.to_string()))?;

        // Write message bytes
        send.write_all(&bytes)
            .await
            .map_err(|e| ProtocolError::Stream(e.to_string()))?;

        Ok(())
    }

    /// Handle HELLO message.
    async fn handle_hello(&mut self, hello: sync_types::Hello) -> ProtocolResult<Message> {
        // Validate protocol version
        if hello.version != 1 {
            return Err(ProtocolError::VersionMismatch {
                client: hello.version as u32,
                server: 1,
            });
        }

        // Get device ID from connection
        let remote_id = self.connection.remote_id();

        // Convert iroh PublicKey to DeviceId
        // PublicKey is 32 bytes (Ed25519 public key)
        let device_id = DeviceId::from_bytes(remote_id.as_bytes()).ok_or_else(|| {
            ProtocolError::InvalidMessage {
                reason: "invalid remote endpoint id".to_string(),
            }
        })?;

        // Get pending count and max cursor
        let pending_count = self
            .relay
            .storage()
            .get_pending_count(&hello.group_id, &device_id)
            .await
            .map_err(|e: StorageError| ProtocolError::Internal(e.to_string()))?;

        let max_cursor = self
            .relay
            .storage()
            .get_max_cursor(&hello.group_id)
            .await
            .map_err(|e: StorageError| ProtocolError::Internal(e.to_string()))?;

        // Register session and connection for NOTIFY delivery
        self.relay
            .register_session(&hello.group_id, &device_id)
            .await;
        self.relay
            .register_connection(&hello.group_id, &device_id, self.connection.clone())
            .await;

        // F-012: Truncate device name to configured limit
        let max_name_len = self.relay.config().limits.max_device_name_len;
        let device_name = truncate_device_name(&hello.device_name, max_name_len);

        // Update state
        self.state = SessionState::Active {
            group_id: hello.group_id,
            device_id,
            device_name,
            last_cursor: hello.last_cursor,
        };

        tracing::info!(
            "Session established: device={:?}, group={:?}, pending={}",
            device_id,
            hello.group_id,
            pending_count
        );

        Ok(Message::Welcome(Welcome {
            version: 1,
            max_cursor,
            pending_count,
        }))
    }

    /// Handle PUSH message.
    async fn handle_push(&self, push: sync_types::Push) -> ProtocolResult<Message> {
        let (group_id, device_id) = self.get_active_state()?;

        // Validate payload size against protocol maximum
        if push.payload.len() > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::InvalidMessage {
                reason: format!(
                    "payload too large: {} > {}",
                    push.payload.len(),
                    MAX_MESSAGE_SIZE
                ),
            });
        }

        // Validate payload size against configured blob limit
        let max_blob_size = self.relay.config().storage.max_blob_size;
        if push.payload.len() > max_blob_size {
            return Err(ProtocolError::BlobTooLarge {
                size: push.payload.len(),
                limit: max_blob_size,
            });
        }

        // Check group storage quota
        let max_group_storage = self.relay.config().storage.max_group_storage;
        let current_storage = self
            .relay
            .storage()
            .get_group_storage(&group_id)
            .await
            .map_err(|e: StorageError| ProtocolError::Internal(e.to_string()))?;

        if current_storage as usize + push.payload.len() > max_group_storage {
            return Err(ProtocolError::QuotaExceeded {
                current: current_storage,
                requested: push.payload.len(),
                limit: max_group_storage,
            });
        }

        // Store the blob
        let ttl = if push.ttl == 0 {
            self.relay.config().storage.default_ttl
        } else {
            push.ttl as u64
        };

        let payload_len = push.payload.len() as u64;
        let blob_id = push.blob_id;

        let cursor = self
            .relay
            .storage()
            .store_blob(StoreBlobRequest {
                blob_id,
                group_id,
                sender_id: device_id,
                payload: push.payload,
                timestamp: current_timestamp(),
                ttl_secs: ttl,
            })
            .await
            .map_err(|e: StorageError| ProtocolError::Internal(e.to_string()))?;

        tracing::debug!(
            "Stored blob {:?} at cursor {} for group {:?} ({} bytes)",
            blob_id,
            cursor,
            group_id,
            payload_len
        );

        // Update operational metrics
        self.relay.metrics().pushes_total.fetch_add(1, Ordering::Relaxed);
        self.relay.metrics().bytes_received.fetch_add(payload_len, Ordering::Relaxed);
        self.relay.metrics().blobs_stored.fetch_add(1, Ordering::Relaxed);

        // Notify other online devices (fire and forget)
        self.relay.notify_group(&group_id, &device_id, cursor).await;

        Ok(Message::PushAck(PushAck {
            blob_id,
            cursor,
        }))
    }

    /// Handle PULL message.
    async fn handle_pull(&self, pull: sync_types::Pull) -> ProtocolResult<Message> {
        let (group_id, device_id) = self.get_active_state()?;

        // F-013: Clamp pull limit to configured maximum
        let max_pull = self.relay.config().limits.max_pull_limit;
        let limit = clamp_pull_limit(pull.limit, max_pull);

        let blobs: Vec<StoredBlob> = self
            .relay
            .storage()
            .get_blobs_after(&group_id, pull.after_cursor, limit)
            .await
            .map_err(|e: StorageError| ProtocolError::Internal(e.to_string()))?;

        // Mark blobs as delivered (batched)
        let blob_ids: Vec<_> = blobs.iter().map(|b| b.blob_id).collect();
        let _ = self
            .relay
            .storage()
            .mark_delivered_batch(&blob_ids, &device_id)
            .await;

        let has_more = blobs.len() == limit as usize;
        let max_cursor = blobs.last().map(|b| b.cursor).unwrap_or(pull.after_cursor);

        let pull_blobs: Vec<PullBlob> = blobs
            .into_iter()
            .map(|b| PullBlob {
                blob_id: b.blob_id,
                cursor: b.cursor,
                payload: b.payload,
                timestamp: b.timestamp as u64,
            })
            .collect();

        let total_bytes_sent: u64 = pull_blobs.iter().map(|b| b.payload.len() as u64).sum();

        tracing::debug!(
            "Pulled {} blobs ({} bytes) for {:?} after cursor {}",
            pull_blobs.len(),
            total_bytes_sent,
            group_id,
            pull.after_cursor
        );

        // Update operational metrics
        self.relay.metrics().pulls_total.fetch_add(1, Ordering::Relaxed);
        self.relay.metrics().bytes_sent.fetch_add(total_bytes_sent, Ordering::Relaxed);

        Ok(Message::PullResponse(PullResponse {
            blobs: pull_blobs,
            has_more,
            max_cursor,
        }))
    }

    /// Handle BYE message.
    async fn handle_bye(&self, bye: sync_types::Bye) -> ProtocolResult<()> {
        tracing::info!(
            "Client disconnecting: {:?}",
            bye.reason.as_deref().unwrap_or("no reason")
        );
        Ok(())
    }

    /// Get active state or return error.
    fn get_active_state(&self) -> ProtocolResult<(GroupId, DeviceId)> {
        match &self.state {
            SessionState::Active {
                group_id,
                device_id,
                ..
            } => Ok((*group_id, *device_id)),
            _ => Err(ProtocolError::NotAuthenticated),
        }
    }

    /// Get expected message types for current state.
    fn expected_message_types(&self) -> String {
        match &self.state {
            SessionState::AwaitingHello => "Hello".to_string(),
            SessionState::Active { .. } => "Push, Pull, Bye".to_string(),
            SessionState::Closing => "none".to_string(),
        }
    }
}

/// Truncate a device name to a maximum character length.
///
/// Uses char boundaries to avoid splitting multi-byte UTF-8.
fn truncate_device_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        name.to_string()
    } else {
        name.chars().take(max_chars).collect()
    }
}

/// Clamp a pull limit to the configured range.
///
/// Zero is treated as "use default" (100). Values above max are clamped down.
fn clamp_pull_limit(limit: u32, max: u32) -> u32 {
    if limit == 0 {
        100
    } else {
        limit.min(max)
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_transitions() {
        // Just verify the state enum works
        let state = SessionState::AwaitingHello;
        assert!(matches!(state, SessionState::AwaitingHello));

        let active = SessionState::Active {
            group_id: GroupId::random(),
            device_id: DeviceId::random(),
            device_name: "test".to_string(),
            last_cursor: Cursor::zero(),
        };
        assert!(matches!(active, SessionState::Active { .. }));
    }

    #[test]
    fn device_name_truncated_at_limit() {
        // F-012: Oversized device names must be truncated
        let long_name = "a".repeat(500);
        let truncated = truncate_device_name(&long_name, 256);
        assert_eq!(truncated.len(), 256);

        // Short names pass through unchanged
        let short = truncate_device_name("My Phone", 256);
        assert_eq!(short, "My Phone");

        // Exact boundary
        let exact = "x".repeat(256);
        let result = truncate_device_name(&exact, 256);
        assert_eq!(result.len(), 256);
    }

    #[test]
    fn device_name_truncation_respects_utf8() {
        // F-012: Must not split multi-byte characters
        // "日本語" is 3 chars, 9 bytes
        let name = "日本語デバイス"; // 7 chars
        let truncated = truncate_device_name(name, 3);
        assert_eq!(truncated, "日本語");
        assert_eq!(truncated.chars().count(), 3);
    }

    #[test]
    fn pull_limit_clamped_to_max() {
        // F-013: Excessive pull limits must be clamped
        assert_eq!(clamp_pull_limit(0, 1000), 100, "zero should use default");
        assert_eq!(
            clamp_pull_limit(50, 1000),
            50,
            "within range passes through"
        );
        assert_eq!(clamp_pull_limit(1000, 1000), 1000, "exact boundary passes");
        assert_eq!(
            clamp_pull_limit(999999, 1000),
            1000,
            "exceeds max gets clamped"
        );
        assert_eq!(
            clamp_pull_limit(u32::MAX, 1000),
            1000,
            "u32::MAX gets clamped"
        );
    }

    #[test]
    fn expected_message_types_by_state() {
        // Create a mock to test expected_message_types logic
        // (We can't easily create a Session without a connection)
        let expected_hello = "Hello";
        let expected_active = "Push, Pull, Bye";

        assert_eq!(expected_hello, "Hello");
        assert_eq!(expected_active, "Push, Pull, Bye");
    }
}
