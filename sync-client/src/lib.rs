//! # sync-client
//!
//! Client library for 0k-Sync zero-knowledge sync protocol.
//!
//! This is the main library that applications use to sync data.
//!
//! ## Features
//!
//! - **E2E Encryption**: XChaCha20-Poly1305 with 192-bit nonces
//! - **Device-Adaptive Key Derivation**: Argon2id scales with available RAM
//! - **Transport Abstraction**: Pluggable transport layer (iroh, mock)
//! - **Pure State Machine**: Uses sync-core for side-effect-free logic
//!
//! ## Example
//!
//! ```ignore
//! use sync_client::{SyncClient, SyncConfig};
//!
//! let config = SyncConfig::default();
//! let client = SyncClient::new(config).await?;
//!
//! // Push encrypted data
//! client.push(b"my data").await?;
//!
//! // Pull new data
//! let blobs = client.pull().await?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod client;
pub mod crypto;
pub mod transport;

pub use client::{ClientError, ReceivedBlob, SyncClient, SyncConfig};
pub use crypto::{Argon2Params, CryptoError, GroupKey, GroupSecret, KEY_SIZE, NONCE_SIZE};
// NOTE: IrohTransport disabled due to iroh 0.96 dependency bug
// pub use transport::{IrohTransport, IrohTransportConfig, ...}
pub use transport::{MockTransport, Transport, TransportError, ALPN, MAX_MESSAGE_SIZE};
