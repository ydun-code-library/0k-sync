# 0k-Sync

**Zero-knowledge sync protocol for local-first applications**

> **0k** = Zero Knowledge — the relay never sees your data

[![Status](https://img.shields.io/badge/status-implementation-green)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

---

## Overview

0k-Sync is a secure, E2E encrypted synchronization protocol for local-first applications. The relay never sees plaintext data - it's a zero-knowledge pass-through that routes encrypted blobs between devices.

```
Device A                     RELAY                      Device B
    │                          │                           │
    │── encrypted blob ───────►│                           │
    │                          │── encrypted blob ────────►│
    │                          │                           │
    │                          │◄───── ACK ────────────────│
    │                          │   (blob deleted)          │
```

## Key Features

- **Zero-Knowledge** - Relay sees only ciphertext, never plaintext
- **E2E Encryption** - XChaCha20-Poly1305 (256-bit) over iroh QUIC transport
- **Post-Quantum Planned** - Hybrid Noise Protocol (ML-KEM-768 + X25519 via clatter) designed, not yet implemented
- **No Accounts** - Devices pair via QR code or short code
- **Local-First** - Apps work offline; sync is opportunistic
- **Framework Agnostic** - Works with any Rust application
- **100% Open Source** - MIT/Apache-2.0 dual licensed

## Deployment Tiers

| Tier | Name | Infrastructure | Cost |
|------|------|----------------|------|
| 1 | Hobbyist | iroh public network | Free |
| 2 | Self-Hosted | Docker on your server | Electricity |
| 3 | PaaS | Railway, Fly.io, etc. | ~$5-50/mo |
| 4 | Managed | Shared relay cluster | Free-$5/mo |
| 5 | Dedicated | Dedicated relay instance | Usage-based |
| 6 | Enterprise | Customer infrastructure | License |

**Key insight:** The client library stays constant. Only the relay tier changes.

## Quick Start

```rust
use sync_client::SyncClient;

// Connect to relay
let client = SyncClient::new(SyncConfig::default()).await?;

// Pair devices (one-time)
let invite = client.create_invite().await?;
// Share invite code with other device...

// Push encrypted data
client.push(encrypted_blob).await?;

// Pull new data
let blobs = client.pull().await?;
```

## Documentation

| Document | Purpose |
|----------|---------|
| [Executive Summary](docs/01-EXECUTIVE-SUMMARY.md) | Technical overview for decision makers |
| [Specification](docs/02-SPECIFICATION.md) | Detailed protocol and API specification |
| [Implementation Plan](docs/03-IMPLEMENTATION-PLAN.md) | TDD implementation approach |
| [Research Validation](docs/04-RESEARCH-VALIDATION.md) | Technology choices and justification |
| [Hybrid Crypto (Appendix B)](appendix-b-hybrid-crypto.md) | Post-quantum cryptography design (planned) |
| [iroh Deep Dive](docs/research/iroh-deep-dive-report.md) | iroh ecosystem audit |

## Project Structure

```
0k-sync/
├── docs/                     # Documentation
│   ├── 01-EXECUTIVE-SUMMARY.md
│   ├── 02-SPECIFICATION.md
│   ├── 03-IMPLEMENTATION-PLAN.md
│   ├── 04-RESEARCH-VALIDATION.md
│   ├── 05-RELEASE-STRATEGY.md
│   └── 06-CHAOS-TESTING-STRATEGY.md
├── sync-types/               # Wire format types (Phase 1) ✅ 32 tests
├── sync-core/                # Pure logic, no I/O (Phase 2) ✅ 60 tests
├── sync-client/              # Client library (Phase 3) ✅ 55 tests
├── sync-content/             # Encrypt-then-hash (Phase 3.5) ✅ 23 tests
├── sync-cli/                 # Testing tool (Phase 4) ✅ 20 tests
├── sync-relay/               # Relay server (Phase 6) 🟡 30 tests (MVP)
└── tests/chaos/              # Chaos testing harness (50 passing, 28 stubs)
```

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| P2P (Tier 1) | [iroh](https://github.com/n0-computer/iroh) | Public network relay |
| E2E Encryption | XChaCha20-Poly1305 | Blob encryption (256-bit) |
| Key Derivation | Argon2id | Passphrase to key |
| Transport | [iroh](https://github.com/n0-computer/iroh) | QUIC P2P + relay fallback (all tiers) |
| Transport Encryption | iroh QUIC (TLS 1.3) | Wire encryption |
| **Planned** | [clatter](https://github.com/jmlepisto/clatter) (Noise Protocol) | Hybrid post-quantum (ML-KEM-768 + X25519) |

**Post-quantum roadmap:** Hybrid Noise Protocol (ML-KEM-768 + X25519 via clatter) is designed but not yet implemented. See [Appendix B](appendix-b-hybrid-crypto.md) for the design.

## Current Status

**Phase: Implementation In Progress (99% Complete)**

- [x] Architecture design
- [x] Protocol specification
- [x] Documentation
- [x] sync-types crate (32 tests) - wire format types + Welcome message
- [x] sync-core crate (60 tests) - pure logic, zero I/O
- [x] sync-client crate (55 tests) - E2E encryption, transport abstraction
- [x] sync-content crate (23 tests) - encrypt-then-hash content transfer
- [x] sync-cli tool (20 tests) - CLI with 6 commands
- [x] IrohTransport (Phase 5) - E2E verified over iroh QUIC
- [x] Chaos scenarios (50 passing, 28 stubs for relay integration)
- [x] **sync-relay server (Phase 6 MVP - 30 tests)**
  - SQLite storage with WAL mode
  - Protocol handler on ALPN /0k-sync/1
  - Session management (HELLO→WELCOME, PUSH→PUSH_ACK, PULL→PULL_RESPONSE)
  - HTTP endpoints (/health, /metrics)
  - Background cleanup task
- [ ] Rate limiting, Dockerfile, integration tests (Phase 6 completion)

## Development

```bash
# Build workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace

# Format
cargo fmt --check
```

## Integration Examples

0k-Sync can be integrated with any framework:

- **Tauri** - Use as a plugin or direct library
- **Electron** - Via native Node.js bindings
- **Mobile** - iOS/Android via FFI
- **Web** - WebAssembly (future)

See [docs/03-IMPLEMENTATION-PLAN.md](docs/03-IMPLEMENTATION-PLAN.md) for integration patterns.

## Contributing

This project follows:
- **Jimmy's Workflow** (PRE-FLIGHT/RED/GREEN/CHECKPOINT)
- **TDD** - Tests first, always
- **KISS** - Keep it simple

See [AGENTS.md](AGENTS.md) for complete development guidelines.

## License

Dual-licensed under MIT and Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

---

**0k-Sync: Zero-knowledge sync for local-first apps.**
