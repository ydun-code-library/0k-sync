//! Message buffer for 0k-Sync.
//!
//! This module provides a queue for outgoing messages with:
//! - FIFO ordering for delivery
//! - Pending tracking (messages sent but not yet acknowledged)
//! - Max size limits to prevent unbounded memory growth
//!
//! The buffer is used by sync-client to manage outgoing Push messages.
//! Messages are enqueued, dequeued for sending, and remain "pending"
//! until an acknowledgement is received from the relay.

use std::collections::{HashMap, VecDeque};
use zerok_sync_types::BlobId;

/// Error type for buffer operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferError {
    /// Buffer is at capacity.
    Full {
        /// Current buffer capacity.
        capacity: usize,
    },
}

impl std::fmt::Display for BufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferError::Full { capacity } => {
                write!(f, "buffer full (capacity: {})", capacity)
            }
        }
    }
}

impl std::error::Error for BufferError {}

/// A queued message waiting to be sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedMessage {
    /// Unique identifier for the blob being pushed.
    pub blob_id: BlobId,
    /// The encrypted payload.
    pub payload: Vec<u8>,
    /// Time-to-live in seconds (0 = forever).
    pub ttl: u32,
}

impl QueuedMessage {
    /// Create a new queued message.
    pub fn new(blob_id: BlobId, payload: Vec<u8>, ttl: u32) -> Self {
        Self {
            blob_id,
            payload,
            ttl,
        }
    }
}

/// Message buffer with pending tracking.
///
/// Messages flow through the buffer in this order:
/// 1. `enqueue()` - add to the queue
/// 2. `dequeue()` - remove from queue, move to pending
/// 3. `ack()` - remove from pending (delivery confirmed)
///
/// If delivery fails, call `nack()` to move back to the front of the queue.
#[derive(Debug)]
pub struct MessageBuffer {
    /// Maximum number of messages (queued + pending).
    max_size: usize,
    /// Messages waiting to be sent.
    queue: VecDeque<QueuedMessage>,
    /// Messages sent but not yet acknowledged.
    pending: HashMap<BlobId, QueuedMessage>,
}

impl MessageBuffer {
    /// Create a new buffer with the given maximum size.
    ///
    /// The max size includes both queued and pending messages.
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            queue: VecDeque::new(),
            pending: HashMap::new(),
        }
    }

    /// Add a message to the queue.
    ///
    /// Returns an error if the buffer is full (queued + pending >= max_size).
    pub fn enqueue(&mut self, msg: QueuedMessage) -> Result<(), BufferError> {
        if self.total_count() >= self.max_size {
            return Err(BufferError::Full {
                capacity: self.max_size,
            });
        }
        self.queue.push_back(msg);
        Ok(())
    }

    /// Remove and return the next message from the queue.
    ///
    /// The message is moved to the pending set until acknowledged.
    pub fn dequeue(&mut self) -> Option<QueuedMessage> {
        let msg = self.queue.pop_front()?;
        self.pending.insert(msg.blob_id, msg.clone());
        Some(msg)
    }

    /// Acknowledge successful delivery of a message.
    ///
    /// Removes the message from the pending set.
    pub fn ack(&mut self, blob_id: &BlobId) {
        self.pending.remove(blob_id);
    }

    /// Negative acknowledge - move a message back to the front of the queue.
    ///
    /// Used when delivery fails and the message should be retried.
    pub fn nack(&mut self, blob_id: &BlobId) {
        if let Some(msg) = self.pending.remove(blob_id) {
            self.queue.push_front(msg);
        }
    }

    /// Check if a message is pending (sent but not acknowledged).
    pub fn is_pending(&self, blob_id: &BlobId) -> bool {
        self.pending.contains_key(blob_id)
    }

    /// Number of messages in the queue (not including pending).
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Number of pending messages.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Total number of messages (queued + pending).
    pub fn total_count(&self) -> usize {
        self.queue.len() + self.pending.len()
    }

    /// Get all pending blob IDs for persistence.
    pub fn pending_blob_ids(&self) -> Vec<BlobId> {
        self.pending.keys().copied().collect()
    }

    /// Clear all messages (both queued and pending).
    pub fn clear(&mut self) {
        self.queue.clear();
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_push() -> QueuedMessage {
        QueuedMessage::new(BlobId::new(), vec![1, 2, 3], 0)
    }

    fn make_push_with_payload(payload: Vec<u8>) -> QueuedMessage {
        QueuedMessage::new(BlobId::new(), payload, 0)
    }

    #[test]
    fn buffer_queues_messages() {
        let mut buffer = MessageBuffer::new(100);
        let msg = QueuedMessage::new(BlobId::new(), vec![1, 2, 3], 0);

        buffer.enqueue(msg).unwrap();

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn buffer_respects_max_size() {
        let mut buffer = MessageBuffer::new(2);

        buffer.enqueue(make_push()).unwrap();
        buffer.enqueue(make_push()).unwrap();
        let overflow = buffer.enqueue(make_push());

        assert!(overflow.is_err());
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn buffer_dequeues_in_order() {
        let mut buffer = MessageBuffer::new(100);
        let msg1 = make_push_with_payload(vec![1]);
        let msg2 = make_push_with_payload(vec![2]);

        buffer.enqueue(msg1).unwrap();
        buffer.enqueue(msg2).unwrap();

        let first = buffer.dequeue().unwrap();
        assert_eq!(first.payload, vec![1]);
    }

    #[test]
    fn buffer_marks_pending_until_ack() {
        let mut buffer = MessageBuffer::new(100);
        let msg = make_push();
        let blob_id = msg.blob_id;

        buffer.enqueue(msg).unwrap();
        let _pending = buffer.dequeue().unwrap();

        assert!(buffer.is_pending(&blob_id));

        buffer.ack(&blob_id);
        assert!(!buffer.is_pending(&blob_id));
    }

    #[test]
    fn dequeue_returns_none_when_empty() {
        let mut buffer = MessageBuffer::new(100);
        assert!(buffer.dequeue().is_none());
    }

    #[test]
    fn pending_counts_toward_max_size() {
        let mut buffer = MessageBuffer::new(2);

        buffer.enqueue(make_push()).unwrap();
        buffer.enqueue(make_push()).unwrap();

        // Dequeue one - moves to pending
        let _ = buffer.dequeue();

        // Try to enqueue - should fail because pending + queued = 2
        let overflow = buffer.enqueue(make_push());
        assert!(overflow.is_err());
    }

    #[test]
    fn ack_frees_space() {
        let mut buffer = MessageBuffer::new(2);

        let msg = make_push();
        let blob_id = msg.blob_id;

        buffer.enqueue(msg).unwrap();
        buffer.enqueue(make_push()).unwrap();

        // Dequeue and ack one
        let _ = buffer.dequeue();
        buffer.ack(&blob_id);

        // Now we should have space
        assert!(buffer.enqueue(make_push()).is_ok());
    }

    #[test]
    fn nack_moves_to_front_of_queue() {
        let mut buffer = MessageBuffer::new(100);

        let msg1 = make_push_with_payload(vec![1]);
        let blob_id1 = msg1.blob_id;
        let msg2 = make_push_with_payload(vec![2]);

        buffer.enqueue(msg1).unwrap();
        buffer.enqueue(msg2).unwrap();

        // Dequeue first message
        let _ = buffer.dequeue();
        assert!(buffer.is_pending(&blob_id1));

        // Nack it - should go back to front
        buffer.nack(&blob_id1);
        assert!(!buffer.is_pending(&blob_id1));

        // Next dequeue should return the same message
        let retry = buffer.dequeue().unwrap();
        assert_eq!(retry.payload, vec![1]);
    }

    #[test]
    fn is_empty_works() {
        let mut buffer = MessageBuffer::new(100);
        assert!(buffer.is_empty());

        buffer.enqueue(make_push()).unwrap();
        assert!(!buffer.is_empty());

        buffer.dequeue();
        assert!(buffer.is_empty());
    }

    #[test]
    fn pending_count_tracks_correctly() {
        let mut buffer = MessageBuffer::new(100);
        assert_eq!(buffer.pending_count(), 0);

        buffer.enqueue(make_push()).unwrap();
        buffer.enqueue(make_push()).unwrap();

        let msg1 = buffer.dequeue().unwrap();
        assert_eq!(buffer.pending_count(), 1);

        let msg2 = buffer.dequeue().unwrap();
        assert_eq!(buffer.pending_count(), 2);

        buffer.ack(&msg1.blob_id);
        assert_eq!(buffer.pending_count(), 1);

        buffer.ack(&msg2.blob_id);
        assert_eq!(buffer.pending_count(), 0);
    }

    #[test]
    fn total_count_includes_both() {
        let mut buffer = MessageBuffer::new(100);

        buffer.enqueue(make_push()).unwrap();
        buffer.enqueue(make_push()).unwrap();
        assert_eq!(buffer.total_count(), 2);

        buffer.dequeue();
        assert_eq!(buffer.total_count(), 2); // 1 queued + 1 pending
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.pending_count(), 1);
    }

    #[test]
    fn pending_blob_ids_returns_all_pending() {
        let mut buffer = MessageBuffer::new(100);

        let msg1 = make_push();
        let msg2 = make_push();
        let id1 = msg1.blob_id;
        let id2 = msg2.blob_id;

        buffer.enqueue(msg1).unwrap();
        buffer.enqueue(msg2).unwrap();
        buffer.dequeue();
        buffer.dequeue();

        let ids = buffer.pending_blob_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn clear_removes_all() {
        let mut buffer = MessageBuffer::new(100);

        buffer.enqueue(make_push()).unwrap();
        buffer.enqueue(make_push()).unwrap();
        buffer.dequeue();

        buffer.clear();

        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.pending_count(), 0);
        assert_eq!(buffer.total_count(), 0);
    }

    #[test]
    fn ack_nonexistent_is_no_op() {
        let mut buffer = MessageBuffer::new(100);
        let fake_id = BlobId::new();

        // Should not panic
        buffer.ack(&fake_id);
        assert_eq!(buffer.pending_count(), 0);
    }

    #[test]
    fn nack_nonexistent_is_no_op() {
        let mut buffer = MessageBuffer::new(100);
        let fake_id = BlobId::new();

        // Should not panic
        buffer.nack(&fake_id);
        assert_eq!(buffer.len(), 0);
    }
}
