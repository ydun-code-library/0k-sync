//! # sync-relay
//!
//! Zero-knowledge relay server for 0k-Sync.
//!
//! This crate implements a relay server that:
//! - Accepts iroh QUIC connections from multiple devices
//! - Routes encrypted blobs between sync groups
//! - Provides temporary buffering for offline devices
//! - Never sees plaintext (relay is a "dumb pipe")
//!
//! ## Architecture
//!
//! ```text
//! Device A ──┐                    ┌── Device B
//!            │    iroh QUIC       │
//!            ├───────────────────►│
//!            │                    │
//!        ┌───┴────────────────────┴───┐
//!        │        sync-relay          │
//!        │  ┌─────────────────────┐   │
//!        │  │   SQLite (blobs)    │   │
//!        │  └─────────────────────┘   │
//!        └────────────────────────────┘
//! ```
//!
//! ## Protocol
//!
//! The relay uses ALPN `/0k-sync/1` and handles these messages:
//! - HELLO → WELCOME (handshake)
//! - PUSH → PUSH_ACK (store blob)
//! - PULL → PULL_RESPONSE (retrieve blobs)
//! - NOTIFY (server → client, new blob available)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod cleanup;
pub mod config;
pub mod error;
pub mod http;
pub mod limits;
pub mod protocol;
pub mod server;
pub mod session;
pub mod storage;
