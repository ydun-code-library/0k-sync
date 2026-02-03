# Next Session Start Here

<!--
TEMPLATE_VERSION: 1.0.0
TEMPLATE_SOURCE: /home/jimmyb/templates/NEXT-SESSION-START-HERE.md.template
LAST_SYNC: 2026-01-12
PURPOSE: Provide quick context and continuity between development sessions
-->

**Last Updated:** 2026-02-03
**Last Session:** Phase 4 Implementation (Q)
**Current Phase:** PHASE 4 COMPLETE (sync-types, sync-core, sync-client, sync-cli)
**Session Summary:** See STATUS.md for complete details
**Next Handler:** Q (Phase 5: iroh transport + transport chaos)

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

**Current Status:** 70% complete
- ✅ Documentation complete (~6,300 lines across 6 core docs)
- ✅ Phase 1: sync-types (28 tests) - wire format types
- ✅ Phase 2: sync-core (60 tests) - pure logic, zero I/O
- ✅ Phase 3: sync-client (42 tests) - E2E encryption, transport abstraction
- ✅ Phase 4: sync-cli (15 tests) - CLI with 5 commands
- ✅ Chaos harness skeleton (24 tests) - infrastructure ready
- ✅ 169 total tests passing
- ✅ GitHub repository: https://github.com/ydun-code-library/0k-sync
- ⚪ Phase 5: iroh transport integration (next)
- ⚪ Phase 6: sync-relay server (future)

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

## 🎯 Current Task: Phase 5 - iroh Transport + Transport Chaos (0% Complete)

### Next Steps
- [ ] Implement IrohTransport (implements Transport trait from sync-client)
- [ ] iroh Endpoint connection management
- [ ] Replace MockTransport with IrohTransport in sync-cli
- [ ] Test real P2P connections between devices
- [ ] Implement transport chaos scenarios:
  - [ ] Connection drops and reconnects
  - [ ] Timeout handling
  - [ ] Network partition simulation

**Reference:** See `docs/03-IMPLEMENTATION-PLAN.md` for Phase 5 details

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

### Option 1: Implement iroh Transport ⭐ RECOMMENDED

**Goal:** Real P2P transport replacing MockTransport

**Tasks:**
- [ ] IrohTransport struct implementing Transport trait
- [ ] iroh Endpoint connection lifecycle
- [ ] Connect to iroh public network (Tier 1)
- [ ] Update sync-cli to use IrohTransport
- [ ] Test real device-to-device sync

**Key Design:** Transport trait abstraction allows drop-in replacement.

**Reference:** Use `mcp__iroh-rag__iroh_ecosystem_search` for iroh patterns

---

### Option 2: Transport Chaos Scenarios

**Prerequisites:** IrohTransport working

**Tasks:**
- [ ] Connection drop scenarios
- [ ] Reconnect with backoff validation
- [ ] Timeout handling under load
- [ ] Network partition simulation

**Key Design:** Test ConnectionState machine under real network conditions.

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
cd /home/jimmyb/crabnebula/sync-relay

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
sync-types ✅ → sync-core ✅ → sync-client ✅ → sync-cli ✅ → iroh-transport ⬅️ NEXT → sync-relay → tauri-plugin
```
Phase 5 adds real transport to sync-client, then Phase 6 builds the relay.

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

**Phase 5 Focus:**
- IrohTransport implementation (Transport trait from sync-client)
- iroh Endpoint connection management
- Transport chaos scenarios (drops, reconnects, timeouts)

**Phase 6 (after Phase 5):**
- sync-relay server (iroh Endpoint + SQLite)
- Full topology chaos (multi-node, partitions, Toxiproxy)

**Chaos Testing Strategy:**
- Phase 5: Transport-level chaos (client-side)
- Phase 6: Full topology chaos (relay + multi-node)

**MCP Servers:**
- `mcp__iroh-rag__iroh_ecosystem_search` - iroh patterns
- `mcp__rust-rag__rust_dev_search` - Rust patterns
- `mcp__crypto-rag__crypto_protocols_search` - Noise Protocol
