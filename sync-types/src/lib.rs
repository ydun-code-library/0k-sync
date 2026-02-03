//! # sync-types
//!
//! Wire format types for the 0k-Sync zero-knowledge sync protocol.
//!
//! This crate provides the foundational types used across all 0k-Sync crates:
//! - [`DeviceId`], [`GroupId`], [`BlobId`], [`Cursor`] - Identity and ordering types
//! - [`Envelope`] - Message wrapper with routing metadata
//! - [`Message`] - Protocol messages (Hello, Push, Pull, etc.)
//! - [`SyncError`] - Error types

#![warn(missing_docs)]
#![warn(clippy::all)]

mod envelope;
mod error;
mod ids;
mod messages;

pub use envelope::Envelope;
pub use error::SyncError;
pub use ids::{BlobId, Cursor, DeviceId, GroupId};
pub use messages::{
    Bye, ContentAck, ContentRef, Hello, Message, MessageType, Notify, Pull, PullBlob,
    PullResponse, Push, PushAck, Welcome,
};
