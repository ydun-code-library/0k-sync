# Next Session Start Here

<!--
TEMPLATE_VERSION: 1.0.0
TEMPLATE_SOURCE: /home/jimmyb/templates/NEXT-SESSION-START-HERE.md.template
LAST_SYNC: 2026-01-12
PURPOSE: Provide quick context and continuity between development sessions
-->

**Last Updated:** 2026-02-05
**Last Session:** Phase 6 Completion (Q)
**Current Phase:** PHASE 6 COMPLETE — All phases 1-6 done
**Session Summary:** See STATUS.md Sessions 16-17 for complete details
**Next Handler:** Q (Chaos harness buildout, multi-relay failover design)

---

## ✅ E2E INTEGRATION COMPLETE (2026-02-05)

**First real end-to-end test passed.** Two CLI instances syncing bidirectionally through the real relay on Beast.

**Three protocol gaps discovered and fixed via TDD:**

| # | Bug | Root Cause | Fix |
|---|-----|-----------|-----|
| 1 | **HELLO/Welcome handshake missing** | `SyncClient::connect()` skipped handshake — relay rejected all messages with `NotAuthenticated` | Client now sends `Message::Hello` with `GroupId` derived from secret, receives `Welcome` before Push/Pull |
| 2 | **Hardcoded passphrase** | CLI push/pull used `"placeholder-passphrase"` instead of real group secret | `GroupConfig` stores `group_secret_hex`, `pair --join` saves it, push/pull read it. Added `SyncConfig::from_secret_bytes()` and `GroupSecret::from_raw()` |
| 3 | **QUIC stream model mismatch** | `IrohTransport` reused one persistent bi-stream; relay expects one stream per request-response | `send()` now opens new bi-stream each time, stores recv half for paired `recv()` call |

**Key lesson:** All 279 unit tests passed with mocks, but the real protocol had never been tested E2E. MockTransport hid the stream-per-request pattern and missing handshake.

**Files Modified:**
- `sync-client/src/client.rs` — HELLO handshake in connect(), `SyncConfig::from_secret_bytes()`
- `sync-client/src/crypto.rs` — `GroupSecret::from_raw()`
- `sync-client/src/transport/iroh.rs` — Stream-per-request model
- `sync-cli/src/commands/push.rs` — Welcome mock, real passphrase from config
- `sync-cli/src/commands/pull.rs` — Welcome mock, real passphrase from config
- `sync-cli/src/commands/pair.rs` — Store group_secret_hex on join
- `sync-cli/src/config.rs` — `group_secret_hex` field, `with_secret()`, `group_secret_bytes()`

**Tests:** 280 passing (+1 new: `connect_sends_hello_with_group_id`), 34 ignored

**Also this session:** Cargo.lock committed (was gitignored), Beast repo cloned and building.

---

## ✅ DOCKER CONTAINERIZATION COMPLETE (2026-02-05)

**Files Created:**
- `Dockerfile` — Production multi-stage build for sync-relay
- `.dockerignore` — Excludes target/, .git/, .claude/, docs/, IDE files
- `sync-relay/relay.docker.toml` — Docker config with `database = "/data/relay.db"`
- `tests/docker-validate.sh` — TDD validation script (8 tests)

**Files Updated:**
- `tests/chaos/Dockerfile.relay` — Replaced stub with real multi-stage build
- `tests/chaos/Dockerfile.cli` — Replaced stub with real CLI build
- `tests/chaos/docker-compose.chaos.yml` — Real healthchecks, volumes, fixed stale WebSocket refs

**Validation:** 8/8 Docker tests pass, 279/279 Rust tests pass

**Docker Image Details:**
- Builder: `rust:1-slim-bookworm` + git, build-essential, pkg-config
- Runtime: `debian:bookworm-slim` + ca-certificates, curl
- Non-root user: `relay`
- Ports: 8080/tcp (HTTP health/metrics). QUIC port is ephemeral (see gotchas below).
- Volume: `/data` (SQLite DB)
- Config: `/etc/0k-sync/relay.toml` (override via bind mount)

**Docker Commands:**
```bash
# Build relay image
docker build -t 0k-sync-relay .

# Run relay
docker run -d -p 8080:8080 -v relay-data:/data 0k-sync-relay

# Check health
curl http://localhost:8080/health

# Run validation tests
bash tests/docker-validate.sh
```

### Docker Gotchas (Lessons Learned)

| Gotcha | Detail | Solution |
|--------|--------|----------|
| **SIGINT vs SIGTERM** | `tokio::signal::ctrl_c()` catches SIGINT only. Docker sends SIGTERM by default. | `STOPSIGNAL SIGINT` in Dockerfile, `stop_signal: SIGINT` in compose |
| **Ephemeral QUIC port** | `Endpoint::builder().bind()` ignores `config.server.bind_address`. Cannot EXPOSE fixed UDP port. | Relay is discovered via iroh NodeId/Pkarr, not direct port |
| **sqlx sqlite (no bundled)** | libsqlite3-sys compiles SQLite from C source. Needs `build-essential` + `pkg-config` in builder. No runtime lib needed. | Install build deps in builder stage |
| **curve25519-dalek patch** | `[patch.crates-io]` points to git fork. Fork must be PUBLIC. Builder needs `git`. | Fork at `ydun-code-library/curve25519-dalek` set to public (was accidentally private) |
| **Toxiproxy + QUIC** | Toxiproxy only supports TCP. iroh QUIC uses UDP. Cannot chaos-test QUIC path. | HTTP endpoint (8080) can still be chaosed |
| **Cargo.lock not in git** | Non-reproducible Docker builds. Different dep versions each build. | ✅ Committed (2026-02-05) |

---

## ✅ CODE REVIEW COMPLETE (2026-02-04)

**File:** `docs/reviews/2026-02-03-sync-relay-phase6-mvp-review.md`

James's code review issues have been addressed:

| # | Issue | Status |
|---|-------|--------|
| 1 | Quota enforcement | ✅ Fixed (max_blob_size + group quota checks) |
| 2 | Cleanup N+1 queries | ✅ Fixed (2 queries with subquery) |
| 3 | Pull delivery batching | ✅ Fixed (mark_delivered_batch with transaction) |
| 4 | Error variant for internal errors | ✅ Fixed (ProtocolError::Internal) |
| 5 | notify_group implementation | ⬜ Remaining (1-2 hrs) |
| 6 | total_sessions accuracy | ⬜ Optional |
| 7 | Graceful shutdown | ✅ Fixed (explicit task aborts) |

**Also completed:**
- ✅ sqlx upgraded 0.7 → 0.8 (fixes RUSTSEC-2024-0363)
- ✅ Excluded sqlx-mysql (fixes RUSTSEC-2023-0071 rsa vulnerability)
- ✅ **0 vulnerabilities** (was 2)
- ✅ 7 documentation files updated with sqlx 0.8

**Next:** Docker → Integration tests

---

## ✅ RATE LIMITING COMPLETE (2026-02-04)

**Files:**
- `sync-relay/src/limits.rs` - New module with RateLimits struct
- `sync-relay/src/protocol.rs` - Connection rate check added
- `sync-relay/src/session.rs` - Message rate check added
- `sync-relay/src/error.rs` - ProtocolError::RateLimited variant added

**Implementation:**
- Connection rate limiting by EndpointId (device public key)
- Message rate limiting by DeviceId (PUSH/PULL operations)
- Uses `governor` crate with keyed DashMap limiters
- Config: `connections_per_ip: 10` (per minute), `messages_per_minute: 100`

**Tests:** +7 new tests in limits.rs (279 total passing)

---

## 📋 Q's Handoff Document

**File:** `docs/handoffs/P2-MONEY-Q-0k-sync-implementation-handoff.md`

This handoff from Moneypenny contains:
- Pre-flight checklist (MCP server inventory required)
- Implementation order with chaos deliverables
- Key technical decisions already made
- Critical rules and first task instructions

---

## ⚡ Quick Context Load

### What This Project Is

**0k-Sync** (zero-knowledge sync) is a self-hosted relay server and Rust client library that enables E2E encrypted synchronization between local-first app instances.

**Your Role:** Developer / Implementer
- Implement Rust crates (sync-types, sync-core, sync-client, sync-content, sync-cli, sync-relay)
- Deploy relay server to Beast
- Create framework integrations as needed (e.g., Tauri plugin)
- Write tests and documentation

**Current Status:** 99% complete
- ✅ Documentation complete (~6,300 lines across 6 core docs)
- ✅ Phase 1: sync-types (32 tests) - wire format types
- ✅ Phase 2: sync-core (60 tests) - pure logic, zero I/O
- ✅ Phase 3: sync-client (55 tests) - E2E encryption, transport abstraction
- ✅ Phase 3.5: sync-content (23 tests) - encrypt-then-hash content transfer
- ✅ Phase 4: sync-cli (20 tests) - CLI with 6 commands
- ✅ Phase 5: IrohTransport (E2E verified Mac Mini ↔ Beast)
- ✅ Chaos scenarios (78 tests: 50 passing, 28 stubs for Phase 6)
- ✅ **Phase 6: sync-relay (43 tests) - COMPLETE (E2E verified, Docker, notify_group)**
- ✅ 284 tests total (284 passing, 34 ignored)
- ✅ **0 vulnerabilities** (sqlx 0.8, no mysql)
- ✅ GitHub repository: https://github.com/ydun-code-library/0k-sync

**⚠️ Critical Dependency Note:**
iroh 0.96 requires a cargo patch for curve25519-dalek. This is already configured in workspace Cargo.toml:
```toml
[patch.crates-io]
curve25519-dalek = { git = "https://github.com/ydun-code-library/curve25519-dalek", branch = "fix/digest-import-5.0.0-pre.1" }
```

---

## 🟢 Current Status Summary

### What's Been Completed ✅

**Phase 1: sync-types + Chaos Harness (v0.1.0-phase1):**
- ✅ Cargo workspace with 7 crates
- ✅ Wire format types (DeviceId, GroupId, BlobId, Cursor, Envelope, Messages)
- ✅ MessagePack serialization (rmp-serde)
- ✅ 28 unit tests for sync-types
- ✅ Chaos harness skeleton (topology, toxiproxy, pumba, assertions) - 24 tests

**Phase 2: sync-core (v0.1.0-phase2):**
- ✅ ConnectionState state machine with exponential backoff
- ✅ MessageBuffer with pending message tracking
- ✅ CursorTracker with gap detection
- ✅ Invite generation/parsing (QR payload + short codes)
- ✅ GroupSecret from passphrase with GroupId derivation
- ✅ 60 unit tests (all instant, no I/O)

**Phase 3: sync-client (v0.1.0-phase3):**
- ✅ GroupKey E2E encryption (XChaCha20-Poly1305, 192-bit nonces)
- ✅ Device-adaptive Argon2id (12-64 MiB based on available RAM)
- ✅ Transport trait abstraction for pluggable transports
- ✅ MockTransport for testing without network
- ✅ SyncClient API (connect, push, pull)
- ✅ 42 unit tests

**Phase 4: sync-cli (v0.1.0-phase4):**
- ✅ init command (device identity)
- ✅ pair --create/--join (sync groups via passphrase or QR)
- ✅ push command (encrypted data)
- ✅ pull command (after cursor)
- ✅ status command (device/group/connection state)
- ✅ JSON config persistence (device.json, group.json)
- ✅ 15 unit tests

**Documentation (Complete):**
- ✅ `docs/01-EXECUTIVE-SUMMARY.md` - Technical overview
- ✅ `docs/02-SPECIFICATION.md` - Full protocol spec
- ✅ `docs/03-IMPLEMENTATION-PLAN.md` - TDD implementation plan
- ✅ `docs/06-CHAOS-TESTING-STRATEGY.md` - 68 chaos scenarios
- ✅ `AGENTS.md`, `CLAUDE.md`, `README.md`

---

## 🎯 Current Task: Phase 6 - sync-relay

### Phase 6 MVP + Code Review ✅
- [x] Crate scaffold with dependencies
- [x] SQLite storage layer with WAL mode
- [x] Protocol handler ALPN /0k-sync/1
- [x] Session state machine (HELLO, PUSH, PULL, BYE)
- [x] Message handlers with cursor assignment
- [x] HTTP endpoints: /health, /metrics
- [x] Main entry point with graceful shutdown
- [x] Background cleanup task
- [x] **Code review fixes (2026-02-04):**
  - [x] Issue #1: Quota enforcement (max_blob_size + group quota)
  - [x] Issue #2: Batch cleanup queries (N+1 → 2)
  - [x] Issue #3: Batch delivery marking (transaction)
  - [x] Issue #4: ProtocolError::Internal variant
  - [x] Issue #7: Improved graceful shutdown
- [x] **sqlx 0.8 upgrade** (0 vulnerabilities)
- [x] 32 tests in sync-relay

### Phase 6 Remaining Tasks

**Next Up:**
- [x] Rate limiting (connections per IP, messages per minute) ✅
- [x] Docker containerization ✅ (8/8 validation tests)
- [x] Integration tests (two CLI instances through relay) ✅ (bidirectional push/pull on Beast)
- [x] Commit Cargo.lock to git ✅ (reproducible builds)
- [ ] Docker build on Beast (test containerized relay) ⬅️ START HERE
- [ ] Issue #5: Implement `notify_group` (1-2 hrs)
- [ ] Implement 28 ignored chaos stubs (T-*, S-SM-*, S-CONC-*, S-CONV-*)
- [ ] Cross-machine E2E (Q ↔ Beast over Tailscale)

**Reference:** See `docs/03-IMPLEMENTATION-PLAN.md` for Phase 6 details
**Reference:** See `docs/06-CHAOS-TESTING-STRATEGY.md` for chaos scenarios

---

## 📁 Key Project Files (Quick Access)

### Start Here Tomorrow
1. **NEXT-SESSION-START-HERE.md** - This file (you're reading it)
2. **STATUS.md** - Current progress and metrics
3. **docs/DOCS-MAP.md** - Navigation index
4. **AGENTS.md** - Development guidelines and context

### Implementation Files (Rate Limiting)
- **sync-relay/src/limits.rs** - Create with governor crate
- **sync-relay/src/session.rs** - Wire in rate checks
- **sync-relay/src/lib.rs** - Uncomment limits module

---

## 🎯 Immediate Next Steps

### Step 1: Rate Limiting ✅ COMPLETE

**Goal:** Implement connection and message rate limits

**Tasks:**
- [x] Create `limits.rs` with governor crate ✅
- [x] Connections per EndpointId (max 10/minute) ✅
- [x] Messages per DeviceId per minute (max 100) ✅
- [x] Wire into protocol.rs and session.rs ✅
- [x] 7 unit tests ✅

---

### Step 2: Docker ✅ COMPLETE

**Completed:** 2026-02-05

- [x] Root Dockerfile (multi-stage, non-root, HEALTHCHECK)
- [x] .dockerignore
- [x] relay.docker.toml (database = /data/relay.db)
- [x] tests/chaos/Dockerfile.relay (replaced stub)
- [x] tests/chaos/Dockerfile.cli (replaced stub)
- [x] docker-compose.chaos.yml (real healthchecks, volumes, fixed stale refs)
- [x] tests/docker-validate.sh (8/8 passing)

**Note:** QUIC port is ephemeral (iroh binds randomly). Only HTTP 8080 is exposed. See Docker Gotchas above.

---

### Step 3: Integration Tests ✅ COMPLETE

**Completed:** 2026-02-05

- [x] Cargo.lock committed for reproducible builds
- [x] Repo cloned and built on Beast (279 tests passing)
- [x] 3 protocol gaps found and fixed via TDD (see E2E INTEGRATION section above)
- [x] Bidirectional push/pull verified through real relay on Beast
- [x] 280 tests passing (+1 new: `connect_sends_hello_with_group_id`)

---

### Step 4: Docker on Beast + notify_group ✅ COMPLETE

**Completed:** 2026-02-05

- [x] Docker image built on Beast (77s build)
- [x] Containerized relay E2E: bidirectional push/pull through Docker relay
- [x] Cross-machine E2E: Q (Mac Mini) ↔ Beast over Tailscale
- [x] notify_group implemented (server opens uni stream per target, fire-and-forget)
- [x] +4 tests in sync-relay (43 total)
- [x] 28 chaos stubs updated with infrastructure requirements → separate work item

---

### Next: Chaos Harness + Multi-Relay ⭐ START HERE

**Tasks:**
- [ ] Build chaos test harness (Docker + `tc netem` for QUIC-compatible fault injection)
- [ ] Implement 28 chaos test stubs using harness
- [ ] Design multi-relay failover (Phase 6.5 — brought forward from Beta)
- [ ] Implement multi-relay in client + relay

---

### ✅ Completed: Phases 1-5 + Phase 6 MVP

**Status:** Done (2026-02-03)

**Phase 6 MVP:** sync-relay with 30 tests
- SQLite storage with WAL mode
- Protocol handler on ALPN /0k-sync/1
- Session state machine
- HTTP endpoints (/health, /metrics)
- Background cleanup task

**Total:** 270 tests passing, 34 ignored, clippy clean

---

## 🔑 Quick Reference

### Access Project
```bash
# On Q (Mac Mini):
cd /Users/ydun.io/Projects/Personal/0k-sync

# On Beast:
cd /home/jimmyb/projects/0k-sync

# Read session context
cat NEXT-SESSION-START-HERE.md
cat STATUS.md

# Check git status
git status
git log --oneline -5
```

### Common Commands
```bash
# Workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace

# Run server (after implementation)
cargo run -p sync-relay

# Run CLI (after implementation)
cargo run -p sync-cli -- push "test"
```

### Docker
```bash
# Build relay image
docker build -t 0k-sync-relay .

# Run relay (local)
docker run -d -p 8080:8080 -v relay-data:/data --name relay 0k-sync-relay

# Health check
curl http://localhost:8080/health

# Run Docker validation tests (8 tests)
bash tests/docker-validate.sh

# Chaos testing topology
cd tests/chaos && docker compose -f docker-compose.chaos.yml up --build
```

### Deployment (Beast)
```bash
# SSH to Beast (use Tailscale IP — hostname "beast" doesn't resolve via SSH)
ssh jimmyb@100.71.79.25

# Rust not on PATH for non-interactive SSH — prefix commands:
export PATH=$HOME/.cargo/bin:$PATH

# Repo location
cd ~/0k-sync

# Docker (port 8080 may be in use — use 8090 if needed)
docker run -d -p 8080:8080 -v relay-data:/data --name relay 0k-sync-relay
```

---

## 💡 Key Insights (Quick Recap)

### Why Cursors Over Timestamps?
- **Problem:** Device clocks drift, especially on mobile
- **Solution:** Relay assigns monotonic cursor to each blob
- **Benefit:** "Give me everything after cursor 500" is always reliable

### Why Noise XX Pattern?
- **Problem:** Need mutual authentication
- **Solution:** Noise XX proves both parties' identity
- **Benefit:** Forward secrecy from message 2 onwards

### Why Standalone Workspace?
- **Problem:** Embedding in app couples sync to app-specific types
- **Solution:** Sync only sees `Blob`, guarantees zero knowledge
- **Benefit:** `cargo add sync-client` works for any local-first app

---

## ⚠️ Important Reminders

### 1. Implementation Order (Current Progress)
```
sync-types ✅ → sync-core ✅ → sync-client ✅ → sync-cli ✅ → IrohTransport ✅ → chaos-tests ✅ → sync-relay ✅ → PHASE 6 COMPLETE ✅ → Chaos harness ⬅️ NEXT → multi-relay → tauri-plugin
```
Phase 6 complete (43 relay tests, 284 total). Next: Chaos harness buildout, multi-relay failover (Phase 6.5).

### 2. Security is Paramount
- NEVER log blob contents (even encrypted)
- Use constant-time comparisons
- Rate limit everything
- Verify Noise handshake before accepting data

### 3. Beast Deployment
- Docker container for relay
- Cloudflare Tunnel for public access
- SQLite for storage (file-based, simple)

### 4. Jimmy's Workflow v2.1
**ALWAYS follow PRE-FLIGHT/RED/GREEN/CHECKPOINT:**
```
🔴 PRE-FLIGHT → 🔴 IMPLEMENT → 🟢 VALIDATE → 🔵 CHECKPOINT
```
- 🔴 **PRE-FLIGHT:** Verify context, requirements, dependencies FIRST
- 🔴 **IMPLEMENT:** Write code, build features
- 🟢 **VALIDATE:** Run tests, prove it works
- 🔵 **CHECKPOINT:** Mark complete, document rollback

---

## 🎬 Ready to Continue!

**Tomorrow's First Actions:**
```bash
cd /Users/ydun.io/Projects/Personal/0k-sync

# 1. Verify green state
cargo test --workspace
cargo audit

# 2. Docker build on Beast
ssh jimmyb@100.71.79.25
export PATH=$HOME/.cargo/bin:$PATH
cd ~/0k-sync && git pull
docker build -t 0k-sync-relay .

# 3. Test containerized relay with CLI clients
```

**Then:** Chaos harness → multi-relay failover → tauri-plugin

**Good luck!**

---

**This file is updated at the end of each session for continuity.**

**Last Updated:** 2026-02-05
**Template Version:** 1.0.0
**Next Handler:** Q (Chaos harness, multi-relay failover)

---

## Note for Q

**✅ CODE REVIEW FIXES COMPLETE (2026-02-04)**

All quick-win code review issues addressed + sqlx security upgrade:

| # | Issue | Status |
|---|-------|--------|
| 1 | Quota enforcement | ✅ Done |
| 2 | Cleanup N+1 queries | ✅ Done |
| 3 | Pull delivery batching | ✅ Done |
| 4 | ProtocolError::Internal | ✅ Done |
| 5 | notify_group | ⬜ Later |
| 6 | total_sessions accuracy | ⬜ Optional |
| 7 | Graceful shutdown | ✅ Done |

**Security:** sqlx 0.8 upgrade - 0 vulnerabilities

---

**Phase 6 Status:**
- sync-relay crate: 32 tests passing
- Code review fixes: 5/7 complete (remaining are optional/later)
- sqlx 0.8: 0 vulnerabilities
- SQLite storage with WAL mode, atomic cursor assignment
- Protocol handler on ALPN /0k-sync/1
- Session management: AwaitingHello → Active → Closing
- Message handlers: HELLO→WELCOME, PUSH→PUSH_ACK, PULL→PULL_RESPONSE
- HTTP endpoints: /health (JSON), /metrics (Prometheus)
- Background cleanup task for TTL-based expiration

**Phase 6 Remaining:**
- [x] Rate limiting (limits.rs) ✅
- [x] Docker containerization ✅ (8/8 validation tests, 2026-02-05)
- [x] Integration tests (CLI through relay) ✅ (bidirectional push/pull on Beast, 2026-02-05)
- [x] Commit Cargo.lock to git ✅ (2026-02-05)
- [x] Docker build on Beast + containerized relay E2E ✅ (2026-02-05)
- [x] Cross-machine E2E (Q ↔ Beast over Tailscale) ✅ (2026-02-05)
- [x] Issue #5: notify_group ✅ (server-side, uni stream delivery, 2026-02-05)
- [x] Chaos test stubs updated (28 stubs → separate chaos harness work item)

**Test Summary:**
- sync-relay: 43 tests (+4 from notify_group)
- sync-client: 56 tests (+1 from HELLO handshake)
- sync-types: 32 tests
- Workspace total: 284 passing, 34 ignored

**Key Commits (Recent First):**
- `9fe34db` - curve25519-dalek patch comment update
- `0643db9` - Docker containerization with TDD validation
- `c2fcb11` - Rate limiting with governor crate
- `531e225` - sqlx 0.8 upgrade + docs
- `05db253` - Code review fixes
- Plus 3 commits from this session: Cargo.lock, HELLO handshake, stream-per-request fix

**curve25519-dalek Patch:**
- Fork: `ydun-code-library/curve25519-dalek` (PUBLIC, was accidentally private)
- Our PR #878: still open upstream
- Upstream PR #875 merged: may make our patch redundant
- Pre-releases pre.2–pre.6 exist; test removing patch when iroh updates

**MCP Servers:**
- `mcp__iroh-rag__iroh_ecosystem_search` - iroh server patterns
- `mcp__rust-rag__rust_dev_search` - Rust patterns (axum, sqlx, governor)
