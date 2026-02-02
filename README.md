# 0k-Sync

**Zero-knowledge sync relay for Tauri applications**

> **0k** = Zero Knowledge — the relay never sees your data

[![Status](https://img.shields.io/badge/status-design%20phase-yellow)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

---

## Overview

0k-Sync is a secure, E2E encrypted synchronization system for local-first Tauri applications. The relay never sees plaintext data - it's a zero-knowledge pass-through that routes encrypted blobs between devices.

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
- **E2E Encryption** - ChaCha20-Poly1305 with Noise Protocol XX handshake
- **No Accounts** - Devices pair via QR code or short code
- **Local-First** - Apps work offline; sync is opportunistic
- **100% Open Source** - MIT/Apache-2.0 dual licensed

## Product Tiers

| Tier | Name | Infrastructure | Cost |
|------|------|----------------|------|
| 1 | Vibe Coder | iroh public network | Free |
| 2 | Home Developer | Self-hosted Docker | Electricity |
| 3 | Vercel-style | PaaS (Railway, Fly.io) | ~$5-50/mo |
| 4 | Community Sync | CrabNebula shared | Free-$5/mo |
| 5 | Cloud | CrabNebula dedicated | Usage-based |
| 6 | Enterprise | Customer infrastructure | License |

**Key insight:** The client library stays constant. Only the relay tier changes.

## Quick Start

```rust
// Add sync to any Tauri app
tauri::Builder::default()
    .plugin(tauri_plugin_sync::init())
    .run(tauri::generate_context!())
```

```typescript
// Frontend usage
await sync.enable();
await sync.push(encryptedBlob);
const blobs = await sync.pull();
```

## Documentation

| Document | Purpose |
|----------|---------|
| [Executive Summary](docs/01-EXECUTIVE-SUMMARY.md) | Technical overview for decision makers |
| [Specification](docs/02-SPECIFICATION.md) | Detailed protocol and API specification |
| [Implementation Plan](docs/03-IMPLEMENTATION-PLAN.md) | TDD implementation approach |
| [Research Validation](docs/04-RESEARCH-VALIDATION.md) | Technology choices and justification |

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
├── tauri-plugin-sync/        # Tauri plugin (Phase 5)
└── sync-relay/               # Custom relay (Phase 6, future)
```

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| P2P (Tier 1) | [iroh](https://github.com/n0-computer/iroh) | Public network relay |
| Transport Encryption | [snow](https://github.com/mcginty/snow) (Noise Protocol) | E2E encrypted channel |
| E2E Encryption | ChaCha20-Poly1305 | Blob encryption |
| Key Derivation | Argon2id | Passphrase to key |
| WebSocket | tokio-tungstenite | Transport (Tiers 2-6) |
| Plugin Framework | Tauri 2.0 | App integration |

## Current Status

**Phase: Design Complete, Implementation Pending**

- [x] Architecture design
- [x] Protocol specification
- [x] Documentation
- [ ] sync-types crate
- [ ] sync-core crate
- [ ] sync-client crate
- [ ] sync-cli tool
- [ ] tauri-plugin-sync
- [ ] sync-relay (custom, future)

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

## Contributing

This project follows:
- **Jimmy's Workflow** (RED/GREEN/CHECKPOINT)
- **TDD** - Tests first, always
- **KISS** - Keep it simple

See [AGENTS.md](AGENTS.md) for complete development guidelines.

## License

Dual-licensed under MIT and Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

---

**0k-Sync: Build. Distribute. Sync.**
