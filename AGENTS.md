# 0k-Sync - E2E encrypted sync protocol for local-first apps

<!--
TEMPLATE_VERSION: 1.7.0
TEMPLATE_SOURCE: /home/jimmyb/templates/AGENTS.md.template
LAST_SYNC: 2026-02-02
SYNC_CHECK: Run ~/templates/tools/check-version.sh to verify you have the latest template version
AUTO_SYNC: Run ~/templates/tools/sync-templates.sh to update (preserves your customizations)
CHANGELOG: See ~/templates/CHANGELOG.md for version history
-->

**STATUS: IN DEVELOPMENT** - Last Updated: 2026-02-02

## Repository Information
- **GitHub Repository**: https://github.com/ydun-code-library/0k-sync
- **Local Directory**: `/home/jimmyb/crabnebula/sync-relay`
- **Primary Purpose**: Provide E2E encrypted synchronization between local-first app instances

## Important Context

<!-- PROJECT_SPECIFIC START: IMPORTANT_CONTEXT -->
**0k-Sync** is a zero-knowledge sync protocol and relay server that enables secure synchronization between multiple instances of any local-first application without the relay ever seeing plaintext data.

**Key Design Principles:**
- **Zero Knowledge** — Relay cannot decrypt user data
- **Open Standards** — Built on Noise Protocol, open source
- **Local-First** — Apps work offline; sync is opportunistic
- **Rust Native** — Server and client libraries in Rust
- **Simple** — Relay is a dumb pipe; intelligence lives client-side

**Target Apps:**
- CashTable (accounting app)
- Regime Tracker (health tracking)
- Future local-first applications

**Documentation:**
- [DOCS-MAP](docs/DOCS-MAP.md) - Navigation index (start here)
- [Executive Summary](docs/01-EXECUTIVE-SUMMARY.md) - Technical overview
- [Specification](docs/02-SPECIFICATION.md) - Detailed protocol spec
- [Implementation Plan](docs/03-IMPLEMENTATION-PLAN.md) - TDD approach
- [Research Validation](docs/04-RESEARCH-VALIDATION.md) - Technology justification

**Research Documents:**
- [iroh Deep Dive](docs/research/iroh-deep-dive-report.md) - iroh ecosystem audit
- [Tactical Mesh Profile](docs/research/tactical-mesh-profile-appendix-d.md) - Defense/tactical applications
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

### 5.5. AI-Optimized Documentation
**CRITICAL**: Documentation is structured data for both humans AND AI consumption

- Use consistent header hierarchy (H1 for title, H2 for sections, H3 for subsections)
- Include machine-parseable metadata (version, date, status)
- Use tables for structured data (dependencies, commands, file lists)
- Mark project-specific sections with HTML comments for template sync
- Structure documents for searchability (clear section names, bullet points)
- Include "When to read this" and "Skip if" guidance where helpful

### 6. Jimmy's Workflow v2.1 (PRE-FLIGHT/Red/Green Checkpoints)
**MANDATORY for all implementation tasks**

```
🔴 PRE-FLIGHT → 🔴 IMPLEMENT → 🟢 VALIDATE → 🔵 CHECKPOINT
```

- 🔴 **PRE-FLIGHT**: Verify context, dependencies, requirements BEFORE starting
- 🔴 **IMPLEMENT**: Write code, build features, make changes
- 🟢 **VALIDATE**: Run explicit validation commands, prove it works
- 🔵 **CHECKPOINT**: Mark completion with machine-readable status, document rollback

**Critical Rules:**
- NEVER skip PRE-FLIGHT - verify context first
- NEVER skip validation phases
- NEVER proceed to next checkpoint without GREEN passing
- ALWAYS document rollback procedures
- ALWAYS use explicit validation commands (not assumptions)

**Confidence Levels:**
- **HIGH**: Proceed automatically
- **MEDIUM**: Pause for human spot-check
- **LOW**: Stop, require human validation

**Reference**: See **JIMMYS-WORKFLOW.md** for complete workflow system (v2.1)

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
**Sync Relay** is a lightweight, self-hosted relay server and Rust client library that enables secure synchronization between multiple instances of a local-first application.

**Key Responsibilities:**
- Accept iroh connections (QUIC) from local-first apps
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
🔄 **Architecture Defined, Implementation Pending** - 20%

- ✅ Protocol design complete (Noise XX)
- ✅ Message specification defined
- ✅ Storage schema designed (SQLite)
- ✅ Client API designed
- ✅ Pairing flow designed (QR/short code)
- ⚪ sync-types crate implementation
- ⚪ sync-relay server implementation
- ⚪ sync-client library implementation
- ⚪ sync-cli testing tool
- ⚪ framework integrations (optional)
<!-- PROJECT_SPECIFIC END: CURRENT_STATUS -->

## Technology Stack

### Rust Workspace Structure

**sync-types/** (shared types):
- Wire format, message structs, crypto primitives
- Dependencies: serde, clatter, uuid

**sync-relay/** (server binary):
- iroh Endpoint server, SQLite storage
- Dependencies: tokio, iroh, sqlx, axum

**sync-client/** (library for apps):
- Connection management, encryption layer
- Dependencies: tokio, clatter (hybrid Noise), argon2, iroh

**sync-content/** (large content transfer):
- Encrypt-then-hash, iroh-blobs integration, content lifecycle
- Dependencies: iroh-blobs, blake3, chacha20poly1305, hkdf

**sync-cli/** (testing tool):
- Command-line push/pull/pair commands
- Dependencies: clap, dialoguer

**Framework Integrations** (optional, built on sync-client):
- Tauri plugin, Electron bindings, mobile FFI, etc.
- Each integration wraps sync-client for specific frameworks

### Protocol Stack

```
Layer 4: Application Sync Logic (Push, Pull, Ack)
Layer 3: Content Transfer (iroh-blobs, encrypt-then-hash)
Layer 2: Sync Protocol (Envelope, routing, cursor)
Layer 1: Transport Security (Hybrid Noise XX via clatter)
Layer 0: Transport (iroh QUIC, mDNS discovery, DHT)
```

### Cryptographic Primitives

| Function | Algorithm |
|----------|-----------|
| Key Exchange | Hybrid: ML-KEM-768 + X25519 (via clatter) |
| DH Fallback | Curve25519 |
| Cipher | XChaCha20-Poly1305 (192-bit nonces) |
| Hash | BLAKE3 (content addressing), BLAKE2s (Noise internal) |
| KDF | HKDF-SHA256 (session keys), Argon2id (passphrase) |

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
0k-sync/
├── Cargo.toml                 # Workspace definition
├── README.md                  # Project overview
├── docs/                      # Documentation
│   ├── DOCS-MAP.md            # Navigation index
│   ├── 01-EXECUTIVE-SUMMARY.md
│   ├── 02-SPECIFICATION.md    # Primary protocol spec
│   ├── 03-IMPLEMENTATION-PLAN.md
│   ├── 04-RESEARCH-VALIDATION.md
│   ├── 05-RELEASE-STRATEGY.md # Versioning, CI/CD, publishing
│   ├── 06-CHAOS-TESTING-STRATEGY.md # Failure testing (68 scenarios)
│   ├── archive/               # Completed plans
│   ├── reference/             # Superseded specifications
│   └── research/              # Research documents
│       ├── iroh-deep-dive-report.md
│       └── tactical-mesh-profile-appendix-d.md
├── sync-types/                # Shared types (Phase 1)
│   ├── Cargo.toml
│   └── src/lib.rs             # Envelope, Message, DeviceId, Cursor
├── sync-core/                 # Pure logic, no I/O (Phase 2)
│   ├── Cargo.toml
│   └── src/lib.rs
├── sync-client/               # Client library (Phase 3)
│   ├── Cargo.toml
│   └── src/lib.rs
├── sync-content/              # Large content transfer (Phase 3.5)
│   ├── Cargo.toml
│   └── src/lib.rs             # iroh-blobs, encrypt-then-hash
├── sync-cli/                  # Testing tool (Phase 4)
│   ├── Cargo.toml
│   └── src/main.rs
├── tauri-plugin-sync/         # Tauri integration (Phase 5)
│   ├── Cargo.toml
│   └── src/lib.rs
├── sync-relay/                # Custom relay (Phase 6)
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
2. Review the specification (`docs/02-SPECIFICATION.md`)
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
1. iroh dependency - compatibility verified (see docs/research/iroh-deep-dive-report.md, 2026-02-02)

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
iroh = "1.0"  # QUIC transport (all tiers)
clatter = "2.1"  # Hybrid Noise Protocol (ML-KEM-768 + X25519)
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
SYNC_RELAY_NODE_ID=your-sync-relay-node-id
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
- **Full Specification**: `docs/02-SPECIFICATION.md`
- **Noise Protocol**: https://noiseprotocol.org/noise.html
- **clatter Rust crate**: https://github.com/jmwample/clatter (hybrid Noise protocol)

### Related Projects
- Syncthing BEP: https://docs.syncthing.net/specs/bep-v1.html
- Any-Sync: https://github.com/anyproto/any-sync
- iroh: https://github.com/n0-computer/iroh

## Important Reminders for AI Assistants

1. **Always use Jimmy's Workflow** for implementation tasks
2. **Follow TDD** - Write tests before implementation
3. **Read the spec first** - `docs/02-SPECIFICATION.md` has all details
4. **Apply YAGNI** - Only implement what's needed for current phase
5. **Use GitHub CLI** - Use `gh` for all GitHub operations
6. **Fix Now** - Never defer fixes
7. **Document dates** - Include actual dates in all documentation
8. **Never log plaintext** - Security is paramount
9. **Cursor > Timestamp** - Use cursors for ordering, not wall clock time

---

**This document follows the [agents.md](https://agents.md/) standard for AI coding assistants.**

**Template Version**: 1.7.0
**Last Updated**: 2026-02-02
