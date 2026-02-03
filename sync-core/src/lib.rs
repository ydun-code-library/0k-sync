//! # sync-core
//!
//! Pure logic for 0k-Sync (no I/O, instant tests).
//!
//! This crate implements the state machines and algorithms for sync
//! without any network or disk I/O, enabling fast unit tests.
//!
//! ## Design Philosophy
//!
//! All modules in this crate are **pure** - they take input and produce output
//! without side effects. This enables:
//! - Instant unit tests (no mocks, no async)
//! - Deterministic behavior (same input → same output)
//! - Easy reasoning about state transitions
//!
//! The actual I/O (network, disk) is performed by `sync-client`, which
//! interprets the actions produced by these state machines.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod buffer;
pub mod cursor;
pub mod pairing;
pub mod state;

pub use buffer::{BufferError, MessageBuffer, QueuedMessage};
pub use cursor::CursorTracker;
pub use pairing::{GroupSecret, Invite, PairingError, RelayNodeId, DEFAULT_INVITE_TTL};
pub use state::{Action, ConnectionState, Event, ReceivedMessage, SyncEvent};
