//! Distributed chaos testing infrastructure.
//!
//! Multi-machine testing across the Tailscale mesh:
//! - Q (Mac Mini) — test orchestrator + client
//! - Beast (91GB server) — 3 relay instances + client container
//! - Guardian (Raspberry Pi) — ARM edge client
//!
//! All tests are `#[ignore = "requires distributed"]`.

pub mod config;
pub mod harness;
pub mod ssh;
