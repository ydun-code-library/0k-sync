//! Chaos test scenarios for 0k-Sync.
//!
//! Organized by category per 06-CHAOS-TESTING-STRATEGY.md:
//!
//! - `encryption` - E-HS-*, E-ENC-*, E-PQ-* (16 tests) - Cryptographic edge cases
//! - `content` - S-BLOB-*, C-STOR-*, C-COLL-* (10 tests) - Blob integrity
//! - `transport` - T-LAT-*, T-LOSS-*, T-CONN-*, T-BW-* (16 stubs) - Network chaos
//! - `sync` - S-SM-*, S-CONC-*, S-CONV-* (12 stubs) - Protocol chaos
//!
//! ## Phase Status
//!
//! | Module | Tests | Status | Requirements |
//! |--------|-------|--------|--------------|
//! | encryption | 16 | ✅ Runnable | MockTransport |
//! | content | 10 | ✅ Runnable | MockTransport |
//! | transport | 16 | ⏳ Stubs | Docker + Toxiproxy + sync-relay |
//! | sync | 12 | ⏳ Stubs | Docker + sync-relay |
//!
//! ## Future Modules (Phase 6)
//!
//! - `adversarial` - A-PROTO-*, A-RES-* (10 scenarios) - Malicious input
//! - `crossplatform` - X-PLAT-* (4 scenarios) - Multi-OS testing

pub mod content;
pub mod encryption;
pub mod sync;
pub mod transport;
