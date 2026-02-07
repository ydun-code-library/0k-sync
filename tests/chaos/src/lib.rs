//! # chaos-tests
//!
//! Chaos testing harness for 0k-Sync.
//!
//! This crate provides infrastructure for testing 0k-Sync under adverse conditions:
//! - Network disruption (latency, packet loss, disconnects)
//! - Encryption edge cases
//! - Concurrent operations
//! - Resource exhaustion

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod assertions;
pub mod distributed;
pub mod harness;
pub mod netem;
pub mod pumba;
pub mod topology;
pub mod toxiproxy;

pub mod scenarios;
