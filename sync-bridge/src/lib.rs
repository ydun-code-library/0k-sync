//! # sync-bridge
//!
//! FFI-friendly bridge layer for 0k-Sync multi-language bindings.
//!
//! This crate monomorphizes `SyncClient<IrohTransport>` into [`SyncHandle`],
//! providing flat, lifetime-free types that sync-node (napi-rs) and
//! sync-python (PyO3) can wrap directly.
//!
//! ## Design
//!
//! - All types are FFI-friendly: no generics, no lifetimes, no trait objects
//! - `String` instead of `&str`, `Vec<u8>` instead of `&[u8]`
//! - Errors flatten to human-readable strings
//! - Thin wrappers — all real logic lives in sync-client and sync-core

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod handle;
pub mod types;

pub use error::SyncBridgeError;
pub use handle::SyncHandle;
pub use types::{PushResult, SyncBlob, SyncHandleConfig, SyncInvite};
