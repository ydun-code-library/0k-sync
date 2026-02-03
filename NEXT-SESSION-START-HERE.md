# Next Session Start Here

<!--
TEMPLATE_VERSION: 1.0.0
TEMPLATE_SOURCE: /home/jimmyb/templates/NEXT-SESSION-START-HERE.md.template
LAST_SYNC: 2026-01-12
PURPOSE: Provide quick context and continuity between development sessions
-->

**Last Updated:** 2026-02-03
**Last Session:** Phase 1 Implementation (Q)
**Current Phase:** PHASE 1 COMPLETE (sync-types + chaos harness skeleton)
**Session Summary:** See STATUS.md for complete details
**Next Handler:** Q (Phase 2: sync-core)

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

**Current Status:** 30% complete
- ✅ Documentation complete (~6,300 lines across 6 core docs)
- ✅ AGENTS.md compliant with template v1.7.0
- ✅ JIMMYS-WORKFLOW.md updated to v2.1 (PRE-FLIGHT phase)
- ✅ DOCS-MAP.md navigation index created
- ✅ Research documents added (iroh deep dive, tactical mesh)
- ✅ Release strategy documented (05-RELEASE-STRATEGY.md)
- ✅ Chaos testing integrated (06-CHAOS-TESTING-STRATEGY.md, 68 scenarios)
- ✅ README.md created
- ✅ GitHub repository: https://github.com/ydun-code-library/0k-sync
- ⚪ Implementation not started

---

## 🟢 Current Status Summary

### What's Been Completed ✅

**Phase 1: sync-types + Chaos Harness (Complete - 2026-02-03):**
- ✅ Cargo workspace with 7 crates
- ✅ `sync-types/` - Wire format types (DeviceId, GroupId, BlobId, Cursor, Envelope, Messages)
- ✅ MessagePack serialization (rmp-serde)
- ✅ 28 unit tests for sync-types
- ✅ `tests/chaos/` - Chaos harness skeleton (topology, toxiproxy, pumba, assertions)
- ✅ 24 unit tests for chaos harness
- ✅ docker-compose.chaos.yml for chaos testing
- ✅ Dockerfile stubs for relay and CLI

**Documentation (Complete):**
- ✅ `docs/01-EXECUTIVE-SUMMARY.md` - Technical overview
- ✅ `docs/02-SPECIFICATION.md` - Full protocol spec with mobile lifecycle
- ✅ `docs/03-IMPLEMENTATION-PLAN.md` - TDD implementation plan (v2.2.0, chaos integrated)
- ✅ `docs/04-RESEARCH-VALIDATION.md` - Technology validation
- ✅ `docs/05-RELEASE-STRATEGY.md` - Release playbook
- ✅ `docs/06-CHAOS-TESTING-STRATEGY.md` - 68 chaos scenarios (v1.5.0)
- ✅ `README.md` - Project overview
- ✅ `AGENTS.md` - Template v1.7.0 compliant
- ✅ `CLAUDE.md` - Updated with MCP server documentation
- ✅ `STATUS.md` / `NEXT-SESSION-START-HERE.md`

**Architecture Decisions:**
- ✅ 6 product tiers defined
- ✅ iroh for Tier 1 (MVP)
- ✅ Mobile lifecycle handling documented
- ✅ Zero-knowledge relay design

---

## 🎯 Current Task: Phase 2 - sync-core (0% Complete)

### Next Steps
- [ ] Implement ConnectionState state machine
- [ ] Implement MessageBuffer with pending tracking
- [ ] Implement CursorTracker with gap detection
- [ ] Implement Invite generation/parsing
- [ ] Add chaos assertion helpers to chaos-tests
- [ ] All tests must pass instantly (no I/O)

**Reference:** See `docs/03-IMPLEMENTATION-PLAN.md` Section 5 (Phase 2)

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

### Option 1: Implement sync-core ⭐ RECOMMENDED

**Goal:** Pure logic crate with zero I/O (instant tests)

**Tasks:**
- [ ] ConnectionState state machine (connect, disconnect, reconnect with backoff)
- [ ] MessageBuffer with pending tracking
- [ ] CursorTracker with gap detection
- [ ] Invite generation/parsing (QR code, short code)
- [ ] Chaos assertion helpers in chaos-tests

**Key Design:** All functions are pure - state in, (new_state, actions) out. No I/O!

**Reference:** See `docs/03-IMPLEMENTATION-PLAN.md` Section 5 (Phase 2)

---

### Option 2: Continue to sync-client (Phase 3)

**Prerequisites:** Phase 2 (sync-core) must be complete first

**Tasks:**
- [ ] Crypto module (Noise XX, XChaCha20-Poly1305, Argon2id)
- [ ] Transport abstraction (iroh, WebSocket)
- [ ] SyncClient implementation
- [ ] Integration tests

---

### ✅ Completed: Phase 1 (sync-types + chaos harness)

**Status:** Done (2026-02-03)

**Deliverables:**
- [x] Cargo workspace with 7 crates
- [x] sync-types crate (DeviceId, GroupId, BlobId, Cursor, Envelope, Messages)
- [x] 28 tests for sync-types
- [x] Chaos harness skeleton (topology, toxiproxy, pumba, assertions)
- [x] 24 tests for chaos harness
- [x] All validation passes (tests, clippy, fmt, doc)

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

### 1. Implementation Order Matters
```
sync-types → sync-core → sync-client → sync-content → sync-cli → tauri-plugin-sync → sync-relay
```
Each crate depends on previous ones being stable.

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

**Amendment Status:** The iroh-deep-dive-report.md contains spec changes that should be considered during implementation:
- iroh-blobs for content transfer (Layer 3)
- sync-content crate addition
- mDNS local discovery
- Self-hosted infrastructure

See `docs/DOCS-MAP.md` → Amendment Status section for full breakdown.
