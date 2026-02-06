# 0k-Sync - E2E encrypted sync protocol for local-first apps

<!--
TEMPLATE_VERSION: 1.7.0
TEMPLATE_SOURCE: /home/jimmyb/templates/AGENTS.md.template
LAST_SYNC: 2026-02-02
SYNC_CHECK: Run ~/templates/tools/check-version.sh to verify you have the latest template version
AUTO_SYNC: Run ~/templates/tools/sync-templates.sh to update (preserves your customizations)
CHANGELOG: See ~/templates/CHANGELOG.md for version history
-->

**STATUS: IN DEVELOPMENT** - Last Updated: 2026-02-06

## Repository Information
- **GitHub Repository**: https://github.com/ydun-code-library/0k-sync
- **Local Directory**: `/home/jimmyb/crabnebula/sync-relay`
- **Primary Purpose**: Provide E2E encrypted synchronization between local-first app instances

## Important Context

<!-- PROJECT_SPECIFIC START: IMPORTANT_CONTEXT -->
**0k-Sync** is a zero-knowledge sync protocol and relay server that enables secure synchronization between multiple instances of any local-first application without the relay ever seeing plaintext data.

**Key Design Principles:**
- **Zero Knowledge** — Relay cannot decrypt user data
- **Open Standards** — Built on iroh QUIC, XChaCha20-Poly1305, open source
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
- Accept HELLO/Welcome handshake for group authentication
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
✅ **Phases 1-6.5 COMPLETE** (2026-02-06)

- ✅ Phase 1: sync-types (33 tests) - wire format + Welcome message
- ✅ Phase 2: sync-core (70 tests) - pure logic + Invite v3 multi-relay
- ✅ Phase 3: sync-client (63 tests) - E2E encryption + connect failover
- ✅ Phase 3.5: sync-content (24 tests) - encrypt-then-hash
- ✅ Phase 4: sync-cli (30 tests) - CLI tool + multi-relay config
- ✅ Phase 5: IrohTransport + chaos scenarios (50 passing, 28 stubs)
- ✅ **Phase 6: sync-relay server (51 tests)**
  - ✅ SQLite storage with WAL mode
  - ✅ Protocol handler (ALPN /0k-sync/1)
  - ✅ Session management (HELLO, PUSH, PULL)
  - ✅ HTTP endpoints (/health, /metrics)
  - ✅ Background cleanup task
  - ✅ Rate limiting (governor crate — per-device + global)
  - ✅ Docker containerization (8/8 validation tests)
  - ✅ notify_group (server-push via uni streams)
  - ✅ Cross-machine E2E (Q ↔ Beast over Tailscale)
  - ✅ Security audit v1 + v2 remediation complete
- ✅ **Phase 6.5: Multi-relay fan-out (2026-02-06)**
  - ✅ Invite v3 with relay list (backward compat with v2)
  - ✅ GroupConfig multi-relay + per-relay cursors (serde OneOrMany compat)
  - ✅ SyncConfig multi-relay support
  - ✅ Connect failover (tries relays in order, AllRelaysFailed error)
  - ✅ Push fan-out (primary awaited, secondaries fire-and-forget)
  - ✅ Per-relay cursor tracking (HashMap<String, u64>)
  - ✅ E2E verified Q ↔ Beast
- ⚪ Phase 7: framework integrations (optional)

**Total: 321 tests passing, 34 ignored**
<!-- PROJECT_SPECIFIC END: CURRENT_STATUS -->

## Technology Stack

### Rust Workspace Structure

**sync-types/** (shared types):
- Wire format, message structs, crypto primitives
- Dependencies: serde, uuid

**sync-relay/** (server binary):
- iroh Endpoint server, SQLite storage
- Dependencies: tokio, iroh, sqlx, axum

**sync-client/** (library for apps):
- Connection management, encryption layer
- Dependencies: tokio, chacha20poly1305, argon2, iroh

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
Layer 1: Transport Security (iroh QUIC TLS 1.3) — Noise XX via clatter planned
Layer 0: Transport (iroh QUIC, mDNS discovery, DHT)
```

### Cryptographic Primitives

| Function | Algorithm |
|----------|-----------|
| E2E Cipher | XChaCha20-Poly1305 (192-bit nonces) |
| Transport | iroh QUIC (TLS 1.3) |
| Hash | BLAKE3 (content addressing) |
| KDF | HKDF-SHA256 (session keys), Argon2id (passphrase) |
| **Planned** | Hybrid Key Exchange: ML-KEM-768 + X25519 (via clatter, not yet implemented) |

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

### Docker
```bash
# Build relay image
docker build -t 0k-sync-relay .

# Run relay container
docker run -d -p 8080:8080 -v relay-data:/data 0k-sync-relay

# Health check
curl http://localhost:8080/health

# Run Docker validation tests (8 tests)
bash tests/docker-validate.sh

# Chaos testing topology
cd tests/chaos && docker compose -f docker-compose.chaos.yml up --build
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
│   ├── relay.toml.example     # Config template
│   └── relay.docker.toml      # Docker-specific config (/data/relay.db)
├── Dockerfile                 # Production relay image (multi-stage)
├── .dockerignore              # Docker build context exclusions
├── tests/
│   ├── docker-validate.sh     # Docker validation tests (8 tests)
│   └── chaos/
│       ├── Dockerfile.relay   # Relay image for chaos testing
│       ├── Dockerfile.cli     # CLI image for chaos testing
│       └── docker-compose.chaos.yml  # Chaos topology (toxiproxy)
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
None

### 🟡 Important Issues
1. **iroh 0.96** requires cargo patch for curve25519-dalek (configured in Cargo.toml)
2. **QUIC port is ephemeral** — `config.server.bind_address` is logged but never passed to `Endpoint::builder().bind()`. Cannot expose fixed UDP port in Docker.
3. **SIGINT only** — `tokio::signal::ctrl_c()` catches SIGINT, not SIGTERM. Docker workaround: `STOPSIGNAL SIGINT`.

### ✅ Resolved Issues
1. **curve25519-dalek build failure** — Fixed with cargo patch (PR #878 upstream)
2. **Stream acknowledgment race** — Fixed with `send.stopped().await`
3. **pair --join EndpointId** — Now properly saves EndpointId as relay_address
4. **curve25519-dalek fork visibility** — Fork was accidentally private, blocking Docker builds. Made public 2026-02-05.
5. **Cargo.lock committed** — Now tracked in git for reproducible builds (2026-02-05)

### 📝 Technical Debt
1. iroh 0.96 is pre-1.0 — minor API changes possible
2. **curve25519-dalek patch may be droppable** — Upstream merged PR #875 ("Update digest and sha2 deps") and released pre.2–pre.6. Our PR #878 still open. Test removing `[patch.crates-io]` when iroh updates.
3. **Toxiproxy can't chaos QUIC** — UDP not supported. HTTP path (8080) can still be chaosed.
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

### Docker Build Notes (Lessons Learned 2026-02-05)

**Builder stage requirements:**
- `git` — curve25519-dalek `[patch.crates-io]` clones from GitHub fork
- `build-essential` — `cc` crate compiles libsqlite3-sys, ring, blake3
- `pkg-config` — libsqlite3-sys probes for system SQLite
- `libssl-dev` NOT needed — iroh uses rustls, not OpenSSL

**Runtime stage requirements:**
- `ca-certificates` — iroh relay discovery uses HTTPS
- `curl` — Docker HEALTHCHECK
- No `libsqlite3` needed at runtime (statically compiled)

**Key gotchas:**
- Use `STOPSIGNAL SIGINT` (not SIGTERM) — binary only handles SIGINT
- QUIC port is ephemeral — only EXPOSE 8080 (HTTP)
- Fork `ydun-code-library/curve25519-dalek` must be PUBLIC
- Bash `((VAR++))` returns exit code 1 when VAR=0 with `set -e`
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

### Rust Dependencies (actual)
```toml
# P2P networking (requires cargo patch - see workspace Cargo.toml)
iroh = "0.96"                    # QUIC transport
iroh-blobs = "0.98"              # Content-addressed storage

# Encryption
# clatter = "2.2"                # Hybrid Noise Protocol — PLANNED, not yet implemented
chacha20poly1305 = "0.10"        # XChaCha20-Poly1305
argon2 = "0.5"                   # Key derivation

# Async runtime
tokio = { version = "1", features = ["full"] }

# Future (sync-relay)
sqlx = { version = "0.8", default-features = false, features = ["sqlite", "runtime-tokio", "derive"] }
axum = "0.7"  # Health endpoints
```

**⚠️ Cargo Patch Required:**
```toml
[patch.crates-io]
curve25519-dalek = { git = "https://github.com/ydun-code-library/curve25519-dalek", branch = "fix/digest-import-5.0.0-pre.1" }
```
<!-- PROJECT_SPECIFIC END: DEPENDENCIES -->

## Environment Variables

<!-- PROJECT_SPECIFIC START: ENVIRONMENT_VARIABLES -->
```bash
# Relay Server
RELAY_BIND=127.0.0.1:8080
RELAY_DATABASE=/data/relay.db
RUST_LOG=info

# Client (supports multiple relays since Phase 6.5)
SYNC_RELAY_NODE_IDS=primary-node-id,secondary-node-id
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
- **Noise Protocol**: https://noiseprotocol.org/noise.html (planned, not yet implemented)
- **clatter Rust crate**: https://github.com/jmwample/clatter (hybrid Noise — planned)

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
**Last Updated**: 2026-02-06
