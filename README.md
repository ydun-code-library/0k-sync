# 0k-Sync

**Zero-knowledge sync protocol for local-first applications**

> **0k** = Zero Knowledge — the relay never sees your data

[![Status](https://img.shields.io/badge/status-design%20phase-yellow)]()
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
- **Post-Quantum Ready** - Hybrid cryptography combining classical + ML-KEM-768 (NIST Level 3)
- **E2E Encryption** - XChaCha20-Poly1305 with Noise Protocol XX hybrid handshake
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
| [Hybrid Crypto (Appendix B)](appendix-b-hybrid-crypto.md) | Post-quantum cryptography design |
| [iroh Deep Dive](docs/research/iroh-deep-dive-report.md) | iroh ecosystem audit |

## Project Structure

```
0k-sync/
├── docs/                     # Documentation
│   ├── 01-EXECUTIVE-SUMMARY.md
│   ├── 02-SPECIFICATION.md
│   ├── 03-IMPLEMENTATION-PLAN.md
│   └── 04-RESEARCH-VALIDATION.md
├── sync-types/               # Wire format types (Phase 1)
├── sync-core/                # Pure logic, no I/O (Phase 2)
├── sync-client/              # Client library (Phase 3)
├── sync-cli/                 # Testing tool (Phase 4)
└── sync-relay/               # Custom relay server (Phase 5)
```

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| P2P (Tier 1) | [iroh](https://github.com/n0-computer/iroh) | Public network relay |
| Transport Encryption | [clatter](https://github.com/jmlepisto/clatter) (Noise Protocol) | Hybrid E2E channel |
| Post-Quantum KEM | ML-KEM-768 | Quantum-resistant key exchange |
| E2E Encryption | XChaCha20-Poly1305 | Blob encryption (256-bit) |
| Key Derivation | Argon2id | Passphrase to key |
| Transport | [iroh](https://github.com/n0-computer/iroh) | QUIC P2P + relay fallback (all tiers) |

**Why hybrid cryptography?** Classical algorithms (X25519) are vulnerable to future quantum computers. 0k-Sync combines classical + post-quantum algorithms so security holds if either is broken. See [Appendix B](appendix-b-hybrid-crypto.md) for details.

## Current Status

**Phase: Design Complete, Implementation Pending**

- [x] Architecture design
- [x] Protocol specification
- [x] Documentation
- [ ] sync-types crate
- [ ] sync-core crate
- [ ] sync-client crate
- [ ] sync-cli tool
- [ ] sync-relay server

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
