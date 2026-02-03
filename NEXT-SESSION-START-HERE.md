# Next Session Start Here

<!--
TEMPLATE_VERSION: 1.0.0
TEMPLATE_SOURCE: /home/jimmyb/templates/NEXT-SESSION-START-HERE.md.template
LAST_SYNC: 2026-01-12
PURPOSE: Provide quick context and continuity between development sessions
-->

**Last Updated:** 2026-02-03
**Last Session:** Phase 3.5 Complete - sync-content Implementation (Q)
**Current Phase:** PHASES 1-5 + 3.5 COMPLETE (ready for Phase 6)
**Session Summary:** See STATUS.md for complete details
**Next Handler:** Q (Phase 6: sync-relay)

---

## 📋 Q's Handoff Document (READ FIRST)

**File:** `docs/handoffs/P2-MONEY-Q-0k-sync-implementation-handoff.md`

This handoff from Moneypenny contains:
- Pre-flight checklist (MCP server inventory required)
- Implementation order with chaos deliverables
- Key technical decisions already made
- Critical rules and first task instructions

**⚠️ Q must verify everything and inventory available MCP servers before starting.**

---

## ⚡ Quick Context Load

### What This Project Is

**0k-Sync** (zero-knowledge sync) is a self-hosted relay server and Rust client library that enables E2E encrypted synchronization between local-first app instances.

**Your Role:** Developer / Implementer
- Implement Rust crates (sync-types, sync-core, sync-client, sync-content, sync-cli, sync-relay)
- Deploy relay server to Beast
- Create framework integrations as needed (e.g., Tauri plugin)
- Write tests and documentation

**Current Status:** 98% complete
- ✅ Documentation complete (~6,300 lines across 6 core docs)
- ✅ Phase 1: sync-types (31 tests) - wire format types
- ✅ Phase 2: sync-core (60 tests) - pure logic, zero I/O
- ✅ Phase 3: sync-client (60 tests) - E2E encryption, transport abstraction
- ✅ Phase 3.5: sync-content (23 tests) - encrypt-then-hash content transfer
- ✅ Phase 4: sync-cli (20 tests) - CLI with 6 commands
- ✅ Phase 5: IrohTransport (E2E verified Mac Mini ↔ Beast)
- ✅ Chaos scenarios (78 tests: 50 passing, 28 stubs for Phase 6)
- ✅ 269 tests total (235 passing, 34 ignored)
- ✅ GitHub repository: https://github.com/ydun-code-library/0k-sync
- ⚪ **Phase 6: sync-relay server (NEXT)**

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

### Phase 5 Complete ✅
- [x] IrohTransport implementing Transport trait
- [x] iroh 0.96 Endpoint connection management
- [x] Replace MockTransport with IrohTransport (--mock fallback available)
- [x] `serve` command for E2E testing
- [x] E2E test: Mac Mini ↔ Beast over iroh QUIC ✓
- [x] curve25519-dalek dependency resolved (cargo patch)
- [x] Chaos scenarios (26 tests: E-HS-*, E-ENC-*, E-PQ-*, S-BLOB-*, C-STOR-*, C-COLL-*)
- [x] Transport/sync stubs (28 ignored tests for Phase 6)

### Phase 6 Tasks
- [ ] iroh Endpoint server (accept connections)
- [ ] Noise XX handshake implementation
- [ ] SQLite storage layer
- [ ] Message routing logic
- [ ] Health/metrics endpoints (axum)
- [ ] Docker containerization
- [ ] Implement 28 ignored chaos stubs (T-*, S-SM-*, S-CONC-*, S-CONV-*)

**Reference:** See `docs/03-IMPLEMENTATION-PLAN.md` for Phase 6 details
**Reference:** See `docs/06-CHAOS-TESTING-STRATEGY.md` for chaos scenarios

---

## 📁 Key Project Files (Quick Access)

### Start Here if You're New (Q's Entry Point)
1. **docs/DOCS-MAP.md** - Navigation index (READ FIRST)
2. **AGENTS.md** - Development guidelines and context
3. **docs/02-SPECIFICATION.md** - Full technical specification
4. **docs/03-IMPLEMENTATION-PLAN.md** - TDD implementation guide
5. **docs/research/iroh-deep-dive-report.md** - Amendment source (spec changes pending)
6. **STATUS.md** - Current progress and metrics

### Implementation Files (after setup)
5. **Cargo.toml** - Workspace root
6. **sync-types/src/lib.rs** - Wire format types
7. **sync-core/src/lib.rs** - Pure logic (no I/O)

---

## 🎯 Immediate Next Steps

### Option 1: Implement sync-relay Server ⭐ RECOMMENDED

**Goal:** Custom relay server for multi-device sync

**Tasks:**
- [ ] iroh Endpoint server (accept incoming connections)
- [ ] Noise XX handshake for mutual authentication
- [ ] SQLite storage layer (blob persistence)
- [ ] Message routing (Push/Pull/Notify)
- [ ] Health/metrics endpoints (axum)

**Key Design:** Relay is zero-knowledge - sees only ciphertext.

**Reference:** Use `mcp__iroh-rag__iroh_ecosystem_search` for iroh server patterns

---

### Option 2: Full Topology Chaos (Docker)

**Prerequisites:** sync-relay working

**Tasks:**
- [ ] Docker containerization (Dockerfile.relay)
- [ ] docker-compose.chaos.yml topology
- [ ] Toxiproxy network fault injection
- [ ] Implement 28 ignored chaos stubs
- [ ] Multi-node convergence testing

**Key Design:** Test full system under network chaos.

---

### ✅ Completed: Phases 1-4

**Status:** Done (2026-02-03)

**Tags:**
- v0.1.0-phase1: sync-types (28 tests)
- v0.1.0-phase2: sync-core (60 tests)
- v0.1.0-phase3: sync-client (42 tests)
- v0.1.0-phase4: sync-cli (15 tests)

**Total:** 169 tests passing, clippy clean, fmt clean

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

### Deployment (Beast)
```bash
# SSH to Beast
ssh jamesb@192.168.68.100

# Docker (after containerization)
docker-compose up -d
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
sync-types ✅ → sync-core ✅ → sync-client ✅ → sync-cli ✅ → IrohTransport ✅ → chaos-tests ✅ → sync-relay ⬅️ NEXT → tauri-plugin
```
Phase 5 complete (E2E verified + chaos scenarios). Next: Phase 6 sync-relay.

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

**Most Common Next Action:**
```bash
cd /home/jimmyb/crabnebula/sync-relay
cat docs/03-IMPLEMENTATION-PLAN.md | head -200  # Review implementation phases
git status
```

**Then:** Start Option 1 (Create Workspace Structure)

**Good luck!**

---

**This file is updated at the end of each session for continuity.**

**Last Updated:** 2026-02-03
**Template Version:** 1.0.0
**Next Handler:** Q (implementation phase)

---

## Note for Q

**Phase 5 Complete ✅:**
- IrohTransport implementation (Transport trait from sync-client)
- iroh Endpoint connection management (iroh 0.96)
- E2E verified (Mac Mini ↔ Beast)
- Chaos scenarios implemented (26 passing + 28 stubs)

**Phase 6 Focus (NEXT):**
- sync-relay server (iroh Endpoint + SQLite)
- Noise XX handshake implementation (use clatter 2.2)
- Full topology chaos (multi-node, partitions, Toxiproxy)
- Implement 28 ignored chaos test stubs

**Chaos Testing Status:**
- Phase 3/3.5: 26 tests passing (encryption + content scenarios)
- Phase 4/5/6: 28 stubs ready (transport + sync scenarios)
- Total chaos-tests: 78 tests (50 passing, 28 ignored)

**MCP Servers:**
- `mcp__iroh-rag__iroh_ecosystem_search` - iroh server patterns
- `mcp__rust-rag__rust_dev_search` - Rust patterns (axum, sqlx)
- `mcp__crypto-rag__crypto_protocols_search` - Noise Protocol XX
