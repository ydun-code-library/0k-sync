# Sync Relay - E2E encrypted local-first sync for Tauri apps

<!--
TEMPLATE_VERSION: 1.6.0
TEMPLATE_SOURCE: /home/jimmyb/templates/AGENTS.md.template
LAST_SYNC: 2026-01-16
SYNC_CHECK: Run ~/templates/tools/check-version.sh to verify you have the latest template version
AUTO_SYNC: Run ~/templates/tools/sync-templates.sh to update (preserves your customizations)
CHANGELOG: See ~/templates/CHANGELOG.md for version history
-->

**STATUS: IN DEVELOPMENT** - Last Updated: 2026-01-16

## Repository Information
- **GitHub Repository**: https://github.com/Jimmyh-world/sync-relay
- **Local Directory**: `/home/jimmyb/crabnebula/sync-relay`
- **Primary Purpose**: Provide E2E encrypted synchronization between Tauri app instances

## Important Context

<!-- PROJECT_SPECIFIC START: IMPORTANT_CONTEXT -->
**Sync Relay** (also known as **tauri-secure-sync**) is a self-hosted relay server and client library that enables secure synchronization between multiple instances of a Tauri application without the relay ever seeing plaintext data.

**Key Design Principles:**
- **Zero Knowledge** — Relay cannot decrypt user data
- **Open Standards** — Built on Noise Protocol, open source
- **Local-First** — Apps work offline; sync is opportunistic
- **Rust Native** — Server and client libraries in Rust
- **Simple** — Relay is a dumb pipe; intelligence lives client-side

**Target Apps:**
- CashTable (accounting app)
- Regime Tracker (health tracking)
- Future Tauri applications

**Documentation:**
- [Executive Summary](docs/01-EXECUTIVE-SUMMARY.md) - Technical overview
- [Specification](docs/02-SPECIFICATION.md) - Detailed protocol spec
- [Implementation Plan](docs/03-IMPLEMENTATION-PLAN.md) - TDD approach
- [Research Validation](docs/04-RESEARCH-VALIDATION.md) - Technology justification
<!-- PROJECT_SPECIFIC END: IMPORTANT_CONTEXT -->

## Core Development Principles (MANDATORY)

### 1. KISS (Keep It Simple, Stupid)
- Avoid over-complication and over-engineering
- Choose simple solutions over complex ones
- Question every abstraction layer
- If a feature seems complex, ask: "Is there a simpler way?"

### 2. TDD (Test-Driven Development)
- Write tests first
- Run tests to ensure they fail (Red phase)
- Write minimal code to pass tests (Green phase)
- Refactor while keeping tests green
- Never commit code without tests

### 3. Separation of Concerns (SOC)
- Each module/component has a single, well-defined responsibility
- Clear boundaries between different parts of the system
- Services should be loosely coupled
- Avoid mixing business logic with UI or data access code

### 4. DRY (Don't Repeat Yourself)
- Eliminate code duplication
- Extract common functionality into reusable components
- Use configuration files for repeated settings
- Create shared libraries for common operations

### 5. Documentation Standards
- Always include the actual date when writing documentation
- Use objective, factual language only
- Avoid marketing terms
- State current development status clearly
- Document what IS, not what WILL BE

### 6. Jimmy's Workflow (Red/Green Checkpoints)
**MANDATORY for all implementation tasks**

- 🔴 **RED (IMPLEMENT)**: Write code, build features, make changes
- 🟢 **GREEN (VALIDATE)**: Run explicit validation commands, prove it works
- 🔵 **CHECKPOINT**: Mark completion with machine-readable status, document rollback

**Critical Rules:**
- NEVER skip validation phases
- NEVER proceed to next checkpoint without GREEN passing
- ALWAYS document rollback procedures
- ALWAYS use explicit validation commands (not assumptions)

**Reference**: See **JIMMYS-WORKFLOW.md** for complete workflow system

### 7. YAGNI (You Ain't Gonna Need It)
- Don't implement features until they're actually needed
- Build for current requirements, not hypothetical future ones
- Question every feature: "Do we need this NOW?"

### 8. Fix Now, Not Later
- Fix vulnerabilities immediately when discovered
- Fix warnings immediately (don't suppress or accumulate)
- Fix failing tests immediately
- Don't use suppressions without documented justification

### 9. Measure Twice, Cut Once
- Always verify your understanding before executing
- Double-check file paths, command syntax, and target locations
- Review the plan before implementation begins
- Confirm assumptions with explicit checks (read the file, run the test)
- When in doubt, investigate first - don't guess

### 10. No Shortcuts (Do It Right)
- Complete the job properly - no half-arsed work
- Don't skip steps to save time
- Implement the full solution, not a "good enough" hack
- If something needs 5 steps, do all 5 steps
- Quality over speed - cutting corners creates debt

### 11. Rules Persist (Context Compression Immunity)
- **ALL rules remain in effect after auto-compact/context summarization**
- Core principles are NEVER optional, regardless of context length
- If you can't remember a rule, re-read AGENTS.md
- Summarization does not equal permission to skip validation
- Jimmy's Workflow gates apply to EVERY task, not just "important" ones

## GitHub Workflow

### Use GitHub CLI (gh) for All GitHub Operations

```bash
# Pull Requests
gh pr create --title "Feature" --body "Description"
gh pr list
gh pr checks

# CI/CD Monitoring
gh run list
gh run watch

# Issues
gh issue create --title "Bug" --body "Description"
gh issue list
```

## Service Overview

<!-- PROJECT_SPECIFIC START: SERVICE_OVERVIEW -->
**Sync Relay** is a lightweight, self-hosted relay server and Rust client library that enables secure synchronization between multiple instances of a Tauri application.

**Key Responsibilities:**
- Accept WebSocket connections from Tauri apps
- Perform Noise Protocol XX handshake for E2E encryption
- Route encrypted blobs between devices in same sync group
- Store blobs temporarily for offline devices
- Clean up expired blobs automatically

**What the Relay NEVER Sees:**
- Plaintext content
- What app is syncing
- Semantic meaning of data
- User passwords or credentials

**Important Distinctions:**
- **Relay** vs **Client**: Relay is server, client is library for apps
- **Sync** vs **Storage**: This is sync infrastructure, not permanent storage
- **Zero Knowledge**: Relay handles ciphertext only
<!-- PROJECT_SPECIFIC END: SERVICE_OVERVIEW -->

## Current Status

<!-- PROJECT_SPECIFIC START: CURRENT_STATUS -->
🔄 **Architecture Defined, Implementation Pending** - 0%

- ✅ Protocol design complete (Noise XX)
- ✅ Message specification defined
- ✅ Storage schema designed (SQLite)
- ✅ Client API designed
- ✅ Pairing flow designed (QR/short code)
- ⚪ sync-types crate implementation
- ⚪ sync-relay server implementation
- ⚪ sync-client library implementation
- ⚪ sync-cli testing tool
- ⚪ tauri-plugin-sync wrapper
<!-- PROJECT_SPECIFIC END: CURRENT_STATUS -->

## Technology Stack

### Rust Workspace Structure

**sync-types/** (shared types):
- Wire format, message structs, crypto primitives
- Dependencies: serde, snow, uuid

**sync-relay/** (server binary):
- WebSocket server, SQLite storage
- Dependencies: tokio, tokio-tungstenite, sqlx, axum

**sync-client/** (library for apps):
- Connection management, encryption layer
- Dependencies: tokio, snow, argon2

**sync-cli/** (testing tool):
- Command-line push/pull/pair commands
- Dependencies: clap, dialoguer

**tauri-plugin-sync/** (optional Tauri wrapper):
- Exposes sync-client as Tauri commands
- Dependencies: tauri, sync-client

### Protocol Stack

```
Layer 4: Application Messages (Push, Pull, Ack)
Layer 3: Envelope (Device ID, Cursor, Blob)
Layer 2: Noise Protocol (XX handshake)
Layer 1: WebSocket (Binary frames)
Layer 0: TLS 1.3 (Cloudflare Tunnel)
```

### Cryptographic Primitives

| Function | Algorithm |
|----------|-----------|
| DH | Curve25519 |
| Cipher | ChaChaPoly |
| Hash | BLAKE2s |
| KDF | Argon2id |

## Build & Test Commands

### Development
```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run specific crate tests
cargo test -p sync-types
cargo test -p sync-relay

# Type checking and linting
cargo clippy --workspace
cargo fmt --check
```

### Running Services
```bash
# Start relay server
cargo run -p sync-relay -- --config relay.toml

# Run CLI tool
cargo run -p sync-cli -- push "test message"
cargo run -p sync-cli -- pull --after-cursor 0
```

## Repository Structure

```
crabnebula-sync/
├── Cargo.toml                 # Workspace definition
├── README.md                  # Project overview
├── docs/                      # Documentation
│   ├── 00-PLAN.md             # Documentation planning
│   ├── 01-EXECUTIVE-SUMMARY.md
│   ├── 02-SPECIFICATION.md
│   ├── 03-IMPLEMENTATION-PLAN.md
│   ├── 04-RESEARCH-VALIDATION.md
│   └── reference/             # Original specifications (archive)
├── sync-types/                # Shared types (Phase 1)
│   ├── Cargo.toml
│   └── src/lib.rs             # Envelope, Message, DeviceId, Cursor
├── sync-core/                 # Pure logic, no I/O (Phase 2)
│   ├── Cargo.toml
│   └── src/lib.rs
├── sync-client/               # Client library (Phase 3)
│   ├── Cargo.toml
│   └── src/lib.rs
├── sync-cli/                  # Testing tool (Phase 4)
│   ├── Cargo.toml
│   └── src/main.rs
├── tauri-plugin-sync/         # Tauri plugin (Phase 5)
│   ├── Cargo.toml
│   └── src/lib.rs
├── sync-relay/                # Custom relay (Phase 6, future)
│   ├── Cargo.toml
│   └── Dockerfile
├── AGENTS.md                  # This file
├── CLAUDE.md                  # AI assistant quick reference
├── STATUS.md                  # Project status
├── NEXT-SESSION-START-HERE.md # Session continuity
└── JIMMYS-WORKFLOW.md         # Workflow reference
```

## Development Workflow

### Starting Work on a Task
1. Read this AGENTS.md file for context
2. Review the specification (`sync-relay-spec.md`)
3. Check current implementation status above
4. **Use Jimmy's Workflow**: Plan → Implement → Validate → Checkpoint
5. Follow TDD approach - write tests first

### Before Committing Code
1. Run all tests: `cargo test --workspace`
2. Build: `cargo build --workspace`
3. Run linter: `cargo clippy --workspace`
4. Format: `cargo fmt`
5. Update documentation if needed

## Known Issues & Technical Debt

<!-- PROJECT_SPECIFIC START: KNOWN_ISSUES -->
### 🔴 Critical Issues
None at this time (project not yet started)

### 🟡 Important Issues
1. iroh dependency - verify compatibility with latest version before starting

### 📝 Technical Debt
None at this time
<!-- PROJECT_SPECIFIC END: KNOWN_ISSUES -->

## Project-Specific Guidelines

<!-- PROJECT_SPECIFIC START: PROJECT_SPECIFIC_GUIDELINES -->
### Code Style
- **Rust**: Follow standard Rust conventions (rustfmt, clippy)
- **Async**: Use tokio for all async operations
- **Error Handling**: Use thiserror for error types

### Security Guidelines
- NEVER log blob contents (encrypted or not)
- Use constant-time comparison for security-sensitive comparisons
- Verify Noise handshake before accepting any data
- Rate limit connections and messages

### Testing Requirements
- Unit tests for all public APIs
- Integration tests for protocol flows
- Property-based tests for serialization
- sync-cli used for end-to-end testing

### Deployment
- Target deployment: Beast (home server) with Docker
- Cloudflare Tunnel for public access
- SQLite for storage (simple, file-based)
<!-- PROJECT_SPECIFIC END: PROJECT_SPECIFIC_GUIDELINES -->

## Dependencies & Integration

<!-- PROJECT_SPECIFIC START: DEPENDENCIES -->
### External Services
- **Cloudflare Tunnel**: TLS termination, public endpoint
- **SQLite**: Local storage for relay

### Related Projects
- **CashTable**: Primary consumer - accounting app
- **health-tracker (Regime Tracker)**: Secondary consumer - health tracking
- **health-plugin**: Health data that may sync via this relay

### Rust Dependencies (planned)
```toml
# sync-relay
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
snow = "0.9"  # Noise Protocol
sqlx = { version = "0.7", features = ["sqlite"] }
axum = "0.7"  # Health endpoints

# sync-client
argon2 = "0.5"  # Key derivation
keyring = "2"   # OS keychain
```
<!-- PROJECT_SPECIFIC END: DEPENDENCIES -->

## Environment Variables

<!-- PROJECT_SPECIFIC START: ENVIRONMENT_VARIABLES -->
```bash
# Relay Server
RELAY_BIND=127.0.0.1:8080
RELAY_DATABASE=/data/relay.db
RUST_LOG=info

# Client
SYNC_RELAY_URL=wss://sync.yourdomain.com
SYNC_GROUP_PASSPHRASE=user-provided
```
<!-- PROJECT_SPECIFIC END: ENVIRONMENT_VARIABLES -->

## Troubleshooting

<!-- PROJECT_SPECIFIC START: TROUBLESHOOTING -->
### Common Issues

**Issue**: Connection refused to relay
**Solution**: Check relay is running, Cloudflare tunnel is active

**Issue**: Decryption failed
**Solution**: Verify both devices have same group passphrase

**Issue**: Blobs not syncing
**Solution**: Check cursor values, ensure pull is using correct after_cursor
<!-- PROJECT_SPECIFIC END: TROUBLESHOOTING -->

## Resources & References

### Documentation
- **Full Specification**: `sync-relay-spec.md`
- **Noise Protocol**: https://noiseprotocol.org/noise.html
- **snow Rust crate**: https://github.com/mcginty/snow

### Related Projects
- Syncthing BEP: https://docs.syncthing.net/specs/bep-v1.html
- Any-Sync: https://github.com/anyproto/any-sync
- iroh: https://github.com/n0-computer/iroh

## Important Reminders for AI Assistants

1. **Always use Jimmy's Workflow** for implementation tasks
2. **Follow TDD** - Write tests before implementation
3. **Read the spec first** - `sync-relay-spec.md` has all details
4. **Apply YAGNI** - Only implement what's needed for current phase
5. **Use GitHub CLI** - Use `gh` for all GitHub operations
6. **Fix Now** - Never defer fixes
7. **Document dates** - Include actual dates in all documentation
8. **Never log plaintext** - Security is paramount
9. **Cursor > Timestamp** - Use cursors for ordering, not wall clock time

---

**This document follows the [agents.md](https://agents.md/) standard for AI coding assistants.**

**Template Version**: 1.6.0
**Last Updated**: 2026-01-16
