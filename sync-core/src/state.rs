//! Connection state machine for 0k-Sync.
//!
//! This module provides a pure, side-effect-free state machine for managing
//! connection lifecycle. The state machine takes events as input and produces
//! a new state plus a list of actions to execute.
//!
//! The actual I/O (connecting, sending messages) is performed by sync-client,
//! not by this module. This enables instant unit testing without network mocks.

use std::time::Duration;
use zerok_sync_types::Cursor;

/// Connection state machine - NO I/O, just state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected to any peer/relay.
    Disconnected,
    /// Connection attempt in progress.
    Connecting,
    /// Connected, performing Noise handshake.
    Handshaking,
    /// Fully connected and authenticated.
    Connected {
        /// Last known cursor from the relay.
        cursor: Cursor,
    },
    /// Disconnected, waiting to reconnect.
    Reconnecting {
        /// Number of reconnection attempts so far.
        attempt: u32,
    },
}

impl ConnectionState {
    /// Create a new state machine in the Disconnected state.
    pub fn new() -> Self {
        Self::Disconnected
    }

    /// Process an event and return the new state plus actions to execute.
    ///
    /// This is a pure function - no side effects. The caller (sync-client)
    /// is responsible for executing the returned actions.
    pub fn on_event(self, event: Event) -> (Self, Vec<Action>) {
        match (self, event) {
            // From Disconnected
            (Self::Disconnected, Event::ConnectRequested) => {
                (Self::Connecting, vec![Action::Connect])
            }

            // From Connecting
            (Self::Connecting, Event::ConnectSucceeded) => {
                (Self::Handshaking, vec![Action::StartHandshake])
            }
            (Self::Connecting, Event::ConnectFailed { error }) => (
                Self::Reconnecting { attempt: 1 },
                vec![
                    Action::EmitEvent(SyncEvent::ConnectionFailed { error }),
                    Action::StartReconnectTimer {
                        delay: calculate_backoff(1),
                    },
                ],
            ),

            // From Handshaking
            (Self::Handshaking, Event::HandshakeCompleted { cursor }) => (
                Self::Connected { cursor },
                vec![Action::EmitEvent(SyncEvent::Connected { cursor })],
            ),
            (Self::Handshaking, Event::HandshakeFailed { error }) => (
                Self::Reconnecting { attempt: 1 },
                vec![
                    Action::EmitEvent(SyncEvent::ConnectionFailed { error }),
                    Action::StartReconnectTimer {
                        delay: calculate_backoff(1),
                    },
                ],
            ),

            // From Connected
            (Self::Connected { cursor: existing }, Event::MessageReceived { message }) => {
                let new_cursor = extract_cursor_from_message(&message).unwrap_or(existing);
                (
                    Self::Connected { cursor: new_cursor },
                    vec![Action::ProcessMessage { message }],
                )
            }
            (Self::Connected { cursor }, Event::Disconnected { reason }) => (
                Self::Reconnecting { attempt: 1 },
                vec![
                    Action::EmitEvent(SyncEvent::Disconnected {
                        reason,
                        last_cursor: cursor,
                    }),
                    Action::StartReconnectTimer {
                        delay: calculate_backoff(1),
                    },
                ],
            ),
            (Self::Connected { cursor }, Event::DisconnectRequested) => (
                Self::Disconnected,
                vec![
                    Action::SendBye,
                    Action::Disconnect,
                    Action::EmitEvent(SyncEvent::Disconnected {
                        reason: "user requested".into(),
                        last_cursor: cursor,
                    }),
                ],
            ),

            // From Reconnecting
            (Self::Reconnecting { attempt: _ }, Event::ReconnectTimer) => {
                (Self::Connecting, vec![Action::Connect])
            }
            (Self::Reconnecting { attempt: _ }, Event::ConnectSucceeded) => {
                (Self::Handshaking, vec![Action::StartHandshake])
            }
            (Self::Reconnecting { attempt }, Event::ConnectFailed { error }) => {
                let next_attempt = attempt.saturating_add(1);
                (
                    Self::Reconnecting {
                        attempt: next_attempt,
                    },
                    vec![
                        Action::EmitEvent(SyncEvent::ReconnectFailed {
                            attempt: next_attempt,
                            error,
                        }),
                        Action::StartReconnectTimer {
                            delay: calculate_backoff(next_attempt),
                        },
                    ],
                )
            }
            (Self::Reconnecting { .. }, Event::DisconnectRequested) => {
                (Self::Disconnected, vec![Action::CancelReconnect])
            }

            // Invalid transitions - stay in current state
            (state, _) => (state, vec![]),
        }
    }

    /// Check if currently connected.
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    /// Check if currently trying to connect.
    pub fn is_connecting(&self) -> bool {
        matches!(
            self,
            Self::Connecting | Self::Handshaking | Self::Reconnecting { .. }
        )
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Events that can occur in the connection lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// User requested connection.
    ConnectRequested,
    /// Transport connection succeeded.
    ConnectSucceeded,
    /// Transport connection failed.
    ConnectFailed {
        /// Error message describing the failure.
        error: String,
    },
    /// Noise handshake completed successfully.
    HandshakeCompleted {
        /// Initial cursor from the relay.
        cursor: Cursor,
    },
    /// Noise handshake failed.
    HandshakeFailed {
        /// Error message describing the failure.
        error: String,
    },
    /// Message received from peer/relay.
    MessageReceived {
        /// The received message.
        message: ReceivedMessage,
    },
    /// Connection was lost.
    Disconnected {
        /// Reason for disconnection.
        reason: String,
    },
    /// User requested disconnect.
    DisconnectRequested,
    /// Reconnect timer fired.
    ReconnectTimer,
}

/// A received message (simplified for state machine purposes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceivedMessage {
    /// Notification of new data.
    Notify {
        /// The cursor of the new data.
        cursor: Cursor,
    },
    /// Response to a pull request.
    PullResponse {
        /// Maximum cursor in the response.
        max_cursor: Cursor,
    },
    /// Acknowledgement of a push.
    PushAck {
        /// Cursor assigned to the pushed data.
        cursor: Cursor,
    },
    /// Other message types.
    Other,
}

/// Extract cursor from a received message if applicable.
fn extract_cursor_from_message(msg: &ReceivedMessage) -> Option<Cursor> {
    match msg {
        ReceivedMessage::Notify { cursor } => Some(*cursor),
        ReceivedMessage::PullResponse { max_cursor } => Some(*max_cursor),
        ReceivedMessage::PushAck { cursor } => Some(*cursor),
        ReceivedMessage::Other => None,
    }
}

/// Actions to be executed by the sync-client.
///
/// These are instructions, not side effects. The sync-client interprets
/// these and performs the actual I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Initiate transport connection.
    Connect,
    /// Disconnect the transport.
    Disconnect,
    /// Start the Noise handshake.
    StartHandshake,
    /// Send a Bye message before disconnecting.
    SendBye,
    /// Process a received message.
    ProcessMessage {
        /// The message to process.
        message: ReceivedMessage,
    },
    /// Start a timer for reconnection.
    StartReconnectTimer {
        /// Delay before attempting reconnection.
        delay: Duration,
    },
    /// Cancel any pending reconnect timer.
    CancelReconnect,
    /// Emit an event to the application.
    EmitEvent(SyncEvent),
}

/// Events emitted to the application layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    /// Successfully connected.
    Connected {
        /// Current cursor from the relay.
        cursor: Cursor,
    },
    /// Connection failed.
    ConnectionFailed {
        /// Error message describing the failure.
        error: String,
    },
    /// Disconnected from peer/relay.
    Disconnected {
        /// Reason for disconnection.
        reason: String,
        /// Last known cursor before disconnect.
        last_cursor: Cursor,
    },
    /// Reconnection attempt failed.
    ReconnectFailed {
        /// Which reconnection attempt this was.
        attempt: u32,
        /// Error message describing the failure.
        error: String,
    },
}

/// Calculate reconnection backoff with jitter.
///
/// Uses exponential backoff with random jitter to prevent thundering herd
/// when many clients reconnect simultaneously after a relay restart.
///
/// Formula: min(30s, 2^attempt seconds) + random(0..5000ms)
fn calculate_backoff(attempt: u32) -> Duration {
    // Base: 2^attempt seconds, capped at 30 seconds
    let base_secs = 2u64.pow(attempt.min(5)).min(30);
    let base = Duration::from_secs(base_secs);

    // Jitter: 0-5000ms random
    let jitter_ms = random_jitter_ms();
    let jitter = Duration::from_millis(jitter_ms);

    base + jitter
}

/// Generate random jitter between 0 and 5000 milliseconds.
fn random_jitter_ms() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    let random = u64::from_le_bytes(bytes);
    random % 5001 // 0..5000 inclusive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_disconnected() {
        let state = ConnectionState::new();
        assert!(matches!(state, ConnectionState::Disconnected));
    }

    #[test]
    fn connect_request_transitions_to_connecting() {
        let state = ConnectionState::Disconnected;
        let (new_state, actions) = state.on_event(Event::ConnectRequested);

        assert!(matches!(new_state, ConnectionState::Connecting));
        assert!(actions.iter().any(|a| matches!(a, Action::Connect)));
    }

    #[test]
    fn connect_success_transitions_to_handshaking() {
        let state = ConnectionState::Connecting;
        let (new_state, actions) = state.on_event(Event::ConnectSucceeded);

        assert!(matches!(new_state, ConnectionState::Handshaking));
        assert!(actions.iter().any(|a| matches!(a, Action::StartHandshake)));
    }

    #[test]
    fn handshake_complete_transitions_to_connected() {
        let state = ConnectionState::Handshaking;
        let (new_state, actions) = state.on_event(Event::HandshakeCompleted {
            cursor: Cursor::new(100),
        });

        assert!(
            matches!(new_state, ConnectionState::Connected { cursor } if cursor == Cursor::new(100))
        );
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::EmitEvent(SyncEvent::Connected { .. }))));
    }

    #[test]
    fn connect_failure_triggers_reconnect() {
        let state = ConnectionState::Connecting;
        let (new_state, actions) = state.on_event(Event::ConnectFailed {
            error: "timeout".into(),
        });

        assert!(matches!(
            new_state,
            ConnectionState::Reconnecting { attempt: 1 }
        ));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::StartReconnectTimer { .. })));
    }

    #[test]
    fn reconnect_timer_transitions_to_connecting() {
        let state = ConnectionState::Reconnecting { attempt: 1 };
        let (new_state, actions) = state.on_event(Event::ReconnectTimer);

        assert!(matches!(new_state, ConnectionState::Connecting));
        assert!(actions.iter().any(|a| matches!(a, Action::Connect)));
    }

    #[test]
    fn reconnect_failure_increments_attempt() {
        let state = ConnectionState::Reconnecting { attempt: 2 };
        let (new_state, actions) = state.on_event(Event::ConnectFailed {
            error: "timeout".into(),
        });

        assert!(matches!(
            new_state,
            ConnectionState::Reconnecting { attempt: 3 }
        ));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::StartReconnectTimer { .. })));
    }

    #[test]
    fn reconnect_backoff_increases_with_attempt() {
        // Attempt 1: base = 2s
        let delay1 = calculate_backoff(1);
        // Attempt 3: base = 8s
        let delay3 = calculate_backoff(3);

        // delay3 should be significantly larger (even accounting for jitter variance)
        // Base difference is 6 seconds, jitter adds up to 5s each
        // So delay3 - jitter should still be > delay1 + jitter in most cases
        // We just verify the base calculation is working
        assert!(delay1 >= Duration::from_secs(2));
        assert!(delay3 >= Duration::from_secs(8));
    }

    #[test]
    fn reconnect_jitter_creates_variance() {
        // Run multiple times and verify we get different values
        let mut delays: Vec<Duration> = Vec::new();

        for _ in 0..20 {
            delays.push(calculate_backoff(3));
        }

        // With 0-5000ms jitter, we should see some variance
        let min = delays.iter().min().unwrap();
        let max = delays.iter().max().unwrap();

        // Allow for possibility of same values but expect some variance
        // This is a probabilistic test - with 20 samples and 5001 possible
        // jitter values, collision is very unlikely
        assert!(
            max.as_millis() - min.as_millis() >= 100,
            "Expected jitter variance, got min={:?} max={:?}",
            min,
            max
        );
    }

    #[test]
    fn reconnect_delay_capped_at_30_seconds_plus_jitter() {
        // Even with high attempt count, base should be capped at 30s
        let delay = calculate_backoff(10);

        // Max possible: 30s base + 5s jitter = 35s
        assert!(
            delay <= Duration::from_secs(35),
            "Reconnect delay must be capped at ~35s (30s base + 5s jitter), got {:?}",
            delay
        );
    }

    #[test]
    fn successful_connect_from_reconnecting_works() {
        let state = ConnectionState::Reconnecting { attempt: 5 };
        let (new_state, _) = state.on_event(Event::ConnectSucceeded);

        assert!(matches!(new_state, ConnectionState::Handshaking));
    }

    #[test]
    fn handshake_complete_from_reconnecting_flow() {
        // Full reconnection flow
        let state = ConnectionState::Reconnecting { attempt: 3 };

        // Timer fires -> Connecting
        let (state, _) = state.on_event(Event::ReconnectTimer);
        assert!(matches!(state, ConnectionState::Connecting));

        // Connect succeeds -> Handshaking
        let (state, _) = state.on_event(Event::ConnectSucceeded);
        assert!(matches!(state, ConnectionState::Handshaking));

        // Handshake completes -> Connected
        let (state, _) = state.on_event(Event::HandshakeCompleted {
            cursor: Cursor::new(200),
        });
        assert!(
            matches!(state, ConnectionState::Connected { cursor } if cursor == Cursor::new(200))
        );
    }

    #[test]
    fn disconnect_request_from_connected() {
        let state = ConnectionState::Connected {
            cursor: Cursor::new(50),
        };
        let (new_state, actions) = state.on_event(Event::DisconnectRequested);

        assert!(matches!(new_state, ConnectionState::Disconnected));
        assert!(actions.iter().any(|a| matches!(a, Action::SendBye)));
        assert!(actions.iter().any(|a| matches!(a, Action::Disconnect)));
    }

    #[test]
    fn disconnect_request_from_reconnecting_cancels() {
        let state = ConnectionState::Reconnecting { attempt: 2 };
        let (new_state, actions) = state.on_event(Event::DisconnectRequested);

        assert!(matches!(new_state, ConnectionState::Disconnected));
        assert!(actions.iter().any(|a| matches!(a, Action::CancelReconnect)));
    }

    #[test]
    fn is_connected_helper() {
        assert!(!ConnectionState::Disconnected.is_connected());
        assert!(!ConnectionState::Connecting.is_connected());
        assert!(!ConnectionState::Handshaking.is_connected());
        assert!(ConnectionState::Connected {
            cursor: Cursor::new(0)
        }
        .is_connected());
        assert!(!ConnectionState::Reconnecting { attempt: 1 }.is_connected());
    }

    #[test]
    fn is_connecting_helper() {
        assert!(!ConnectionState::Disconnected.is_connecting());
        assert!(ConnectionState::Connecting.is_connecting());
        assert!(ConnectionState::Handshaking.is_connecting());
        assert!(!ConnectionState::Connected {
            cursor: Cursor::new(0)
        }
        .is_connecting());
        assert!(ConnectionState::Reconnecting { attempt: 1 }.is_connecting());
    }

    #[test]
    fn message_received_updates_cursor() {
        let state = ConnectionState::Connected {
            cursor: Cursor::new(10),
        };
        let (new_state, _) = state.on_event(Event::MessageReceived {
            message: ReceivedMessage::Notify {
                cursor: Cursor::new(20),
            },
        });

        match new_state {
            ConnectionState::Connected { cursor } => {
                assert_eq!(cursor, Cursor::new(20));
            }
            _ => panic!("Expected Connected state"),
        }
    }

    #[test]
    fn cursor_preserved_on_unknown_message() {
        // F-008: Unknown messages must NOT reset cursor to zero.
        // ReceivedMessage::Other has no cursor — the existing cursor must be preserved.
        let state = ConnectionState::Connected {
            cursor: Cursor::new(5),
        };
        let (new_state, actions) = state.on_event(Event::MessageReceived {
            message: ReceivedMessage::Other,
        });

        match new_state {
            ConnectionState::Connected { cursor } => {
                assert_eq!(
                    cursor,
                    Cursor::new(5),
                    "cursor must be preserved when message has no cursor"
                );
            }
            _ => panic!("Expected Connected state"),
        }
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::ProcessMessage { .. })));
    }

    #[test]
    fn unexpected_disconnect_triggers_reconnect() {
        let state = ConnectionState::Connected {
            cursor: Cursor::new(100),
        };
        let (new_state, actions) = state.on_event(Event::Disconnected {
            reason: "connection lost".into(),
        });

        assert!(matches!(
            new_state,
            ConnectionState::Reconnecting { attempt: 1 }
        ));
        assert!(actions.iter().any(|a| matches!(
            a,
            Action::EmitEvent(SyncEvent::Disconnected { last_cursor, .. }) if *last_cursor == Cursor::new(100)
        )));
    }
}
