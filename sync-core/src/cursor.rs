//! Cursor tracking for 0k-Sync.
//!
//! This module provides tracking of received message cursors with:
//! - Recording the highest cursor seen
//! - Detecting gaps in the sequence (missing messages)
//! - Supporting pull requests to fill gaps after reconnection
//!
//! Cursors in 0k-Sync are monotonically increasing integers assigned by the
//! relay. The cursor tracker helps clients identify what they've missed.

use std::collections::BTreeSet;
use zerok_sync_types::Cursor;

/// Tracks received cursors and detects gaps.
///
/// The tracker maintains:
/// - The highest cursor seen (for pull after-cursor)
/// - A set of received cursors (for gap detection)
///
/// Gap detection is useful when messages arrive out of order or when
/// reconnecting after a disconnect. The client can request pulls for
/// missing cursor ranges.
#[derive(Debug, Clone)]
pub struct CursorTracker {
    /// Set of all received cursor values.
    received: BTreeSet<u64>,
    /// The last contiguous cursor (no gaps up to this point).
    contiguous: u64,
}

impl CursorTracker {
    /// Create a new tracker starting at cursor 0.
    pub fn new() -> Self {
        Self {
            received: BTreeSet::new(),
            contiguous: 0,
        }
    }

    /// Create a new tracker starting at a specific cursor.
    ///
    /// This is useful when resuming from a persisted state.
    pub fn with_cursor(cursor: Cursor) -> Self {
        Self {
            received: BTreeSet::new(),
            contiguous: cursor.value(),
        }
    }

    /// Record receipt of a message with the given cursor.
    ///
    /// Call this for each message received from the relay.
    pub fn received(&mut self, cursor: Cursor) {
        let value = cursor.value();
        if value > self.contiguous {
            self.received.insert(value);
            // Update contiguous cursor if possible
            self.update_contiguous();
        }
    }

    /// Get the last cursor seen (highest value).
    pub fn last_cursor(&self) -> Cursor {
        self.received
            .last()
            .map(|&v| Cursor::new(v))
            .unwrap_or_else(|| Cursor::new(self.contiguous))
    }

    /// Get the last contiguous cursor (no gaps up to this point).
    ///
    /// This is useful for requesting pulls - everything up to this cursor
    /// has been received, so we need to pull from here.
    pub fn contiguous_cursor(&self) -> Cursor {
        Cursor::new(self.contiguous)
    }

    /// Check if there are any gaps in the received sequence.
    pub fn has_gaps(&self) -> bool {
        !self.received.is_empty()
    }

    /// Maximum gap size before we stop enumerating missing cursors.
    /// Prevents OOM if a malicious relay reports a huge cursor jump (F-019).
    const MAX_GAP: u64 = 10_000;

    /// Get the list of missing cursors (gaps in the sequence).
    ///
    /// Returns cursors between the contiguous point and the highest received
    /// that have not been received. Returns empty if the gap exceeds `MAX_GAP`.
    pub fn missing(&self) -> Vec<Cursor> {
        if self.received.is_empty() {
            return Vec::new();
        }

        let max_received = *self.received.last().unwrap();
        let gap = max_received.saturating_sub(self.contiguous);

        // F-019: Cap gap enumeration to prevent OOM
        if gap > Self::MAX_GAP {
            return Vec::new();
        }

        let mut missing = Vec::new();

        for cursor in (self.contiguous + 1)..=max_received {
            if !self.received.contains(&cursor) {
                missing.push(Cursor::new(cursor));
            }
        }

        missing
    }

    /// Acknowledge that gaps have been filled up to the given cursor.
    ///
    /// This clears the gap tracking state for cursors up to the given value.
    pub fn acknowledge_up_to(&mut self, cursor: Cursor) {
        let value = cursor.value();
        self.received.retain(|&c| c > value);
        if value > self.contiguous {
            self.contiguous = value;
        }
        self.update_contiguous();
    }

    /// Reset the tracker to a specific cursor.
    ///
    /// Clears all gap tracking and sets the contiguous point.
    pub fn reset(&mut self, cursor: Cursor) {
        self.received.clear();
        self.contiguous = cursor.value();
    }

    /// Update the contiguous cursor based on received set.
    fn update_contiguous(&mut self) {
        // Find the highest contiguous cursor
        let mut next = self.contiguous + 1;
        while self.received.remove(&next) {
            self.contiguous = next;
            next += 1;
        }
    }
}

impl Default for CursorTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_tracker_starts_at_zero() {
        let tracker = CursorTracker::new();
        assert_eq!(tracker.last_cursor(), Cursor::new(0));
    }

    #[test]
    fn cursor_tracker_updates_on_receive() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(5));
        tracker.received(Cursor::new(3)); // Out of order

        // last_cursor returns the highest
        assert_eq!(tracker.last_cursor(), Cursor::new(5));
    }

    #[test]
    fn cursor_tracker_detects_gaps() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(2));
        tracker.received(Cursor::new(5)); // Gap: 3, 4 missing

        assert!(tracker.has_gaps());
        assert_eq!(tracker.missing(), vec![Cursor::new(3), Cursor::new(4)]);
    }

    #[test]
    fn no_gaps_when_contiguous() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(2));
        tracker.received(Cursor::new(3));

        assert!(!tracker.has_gaps());
        assert!(tracker.missing().is_empty());
    }

    #[test]
    fn contiguous_cursor_tracks_correctly() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(1));

        tracker.received(Cursor::new(2));
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(2));

        tracker.received(Cursor::new(5)); // Gap
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(2));

        tracker.received(Cursor::new(3));
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(3));

        tracker.received(Cursor::new(4)); // Fills the gap
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(5));
    }

    #[test]
    fn filling_gaps_updates_contiguous() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(3));
        tracker.received(Cursor::new(5));

        assert!(tracker.has_gaps());
        assert_eq!(tracker.missing(), vec![Cursor::new(2), Cursor::new(4)]);

        // Fill gap at 2
        tracker.received(Cursor::new(2));
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(3));
        assert_eq!(tracker.missing(), vec![Cursor::new(4)]);

        // Fill gap at 4
        tracker.received(Cursor::new(4));
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(5));
        assert!(!tracker.has_gaps());
    }

    #[test]
    fn with_cursor_initializes_correctly() {
        let tracker = CursorTracker::with_cursor(Cursor::new(100));
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(100));
        assert_eq!(tracker.last_cursor(), Cursor::new(100));
        assert!(!tracker.has_gaps());
    }

    #[test]
    fn acknowledge_up_to_clears_gaps() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(5));
        assert!(tracker.has_gaps());

        // Acknowledge up to 5 - assumes gaps were filled elsewhere
        tracker.acknowledge_up_to(Cursor::new(5));
        assert!(!tracker.has_gaps());
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(5));
    }

    #[test]
    fn reset_clears_everything() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(5));

        tracker.reset(Cursor::new(10));

        assert_eq!(tracker.contiguous_cursor(), Cursor::new(10));
        assert_eq!(tracker.last_cursor(), Cursor::new(10));
        assert!(!tracker.has_gaps());
    }

    #[test]
    fn out_of_order_receives_work() {
        let mut tracker = CursorTracker::new();

        // Receive completely out of order
        tracker.received(Cursor::new(5));
        tracker.received(Cursor::new(2));
        tracker.received(Cursor::new(4));
        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(3));

        // Should be fully contiguous now
        assert!(!tracker.has_gaps());
        assert_eq!(tracker.contiguous_cursor(), Cursor::new(5));
        assert_eq!(tracker.last_cursor(), Cursor::new(5));
    }

    #[test]
    fn duplicate_receives_are_idempotent() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(2));
        tracker.received(Cursor::new(1)); // Duplicate
        tracker.received(Cursor::new(2)); // Duplicate

        assert_eq!(tracker.contiguous_cursor(), Cursor::new(2));
        assert!(!tracker.has_gaps());
    }

    #[test]
    fn old_cursors_are_ignored() {
        let mut tracker = CursorTracker::with_cursor(Cursor::new(100));

        tracker.received(Cursor::new(50)); // Below contiguous
        tracker.received(Cursor::new(100)); // At contiguous

        assert!(!tracker.has_gaps());
        assert_eq!(tracker.last_cursor(), Cursor::new(100));
    }

    #[test]
    fn cursor_gap_cap_prevents_oom() {
        // F-019: Gaps larger than MAX_GAP must not enumerate missing cursors
        let mut tracker = CursorTracker::new();
        tracker.received(Cursor::new(1));
        // Create a gap larger than MAX_GAP (10_000)
        tracker.received(Cursor::new(20_000));

        assert!(tracker.has_gaps());
        // missing() should return empty to prevent OOM allocation
        let missing = tracker.missing();
        assert!(
            missing.is_empty(),
            "gaps > MAX_GAP should return empty vec, got {} entries",
            missing.len()
        );
    }
}
