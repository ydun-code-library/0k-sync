# Claude AI Assistant Instructions

<!--
TEMPLATE_VERSION: 1.7.0
TEMPLATE_SOURCE: /home/jimmyb/templates/CLAUDE.md.template
LAST_SYNC: 2026-02-02
-->

Please refer to **AGENTS.md** for complete development guidelines and project context.

This project follows the [agents.md](https://agents.md/) standard for AI coding assistants.

## Quick Reference

### Core Development Principles
1. **KISS** - Keep It Simple, Stupid
2. **TDD** - Test-Driven Development
3. **SOC** - Separation of Concerns
4. **DRY** - Don't Repeat Yourself
5. **Documentation Standards** - Factual, dated, objective
6. **Jimmy's Workflow** - Red/Green Checkpoints (MANDATORY)
7. **YAGNI** - You Ain't Gonna Need It
8. **Fix Now** - Never defer known issues
9. **Measure Twice, Cut Once** - Verify before executing
10. **No Shortcuts** - Do it right, complete the job
11. **Rules Persist** - Principles apply even after context compression

### Jimmy's Workflow v2.1
Use for all implementation tasks:
```
🔴 PRE-FLIGHT → 🔴 IMPLEMENT → 🟢 VALIDATE → 🔵 CHECKPOINT
```
- 🔴 **PRE-FLIGHT**: Verify context, requirements, dependencies FIRST
- 🔴 **IMPLEMENT**: Write code, build features
- 🟢 **VALIDATE**: Run tests, prove it works
- 🔵 **CHECKPOINT**: Mark complete, document rollback

**Invoke**: *"Let's use Jimmy's Workflow to execute this plan"*

**Reference**: See **JIMMYS-WORKFLOW.md** for complete system (v2.1)

### Critical Rules
- ✅ Write tests FIRST (TDD)
- ✅ Run explicit validation commands
- ✅ Never skip checkpoints
- ✅ Document rollback procedures
- ✅ Include actual dates in documentation
- ✅ Use `gh` CLI for all GitHub operations
- ✅ Apply YAGNI - only build what's needed NOW
- ✅ Read specification first: `docs/02-SPECIFICATION.md`
- ✅ NEVER log plaintext or blob contents
- ❌ Never proceed without GREEN validation passing
- ❌ Never assume - always verify

### GitHub Operations
```bash
# Pull Requests & CI/CD
gh pr create --title "Title" --body "Description"
gh pr checks
gh pr list

# Issues
gh issue create --title "Bug" --body "Description"
gh issue list

# Workflow Monitoring
gh run list
gh run watch
```

### Common Commands
```bash
# Workspace Build
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --check

# Run Relay Server
cargo run -p sync-relay -- --config relay.toml

# Run CLI Tool (package name is zerok-sync-cli)
cargo run -p zerok-sync-cli -- push "message"
cargo run -p zerok-sync-cli -- pull --after-cursor 0
cargo run -p zerok-sync-cli -- pair --create

# Docker
docker build -t 0k-sync-relay .
docker run -d -p 8080:8080 -v relay-data:/data 0k-sync-relay
bash tests/docker-validate.sh  # 8 validation tests
```

### Project-Specific Notes

**This is a Rust Cargo workspace** with 6 core crates:
1. `sync-types` - Shared types (Envelope, Message, Welcome, etc.) - 33 tests
2. `sync-core` - Pure logic, no I/O (instant tests) - 65 tests
3. `sync-client` - Library for local-first apps - 59 tests
4. `sync-content` - Encrypt-then-hash content transfer - 24 tests
5. `sync-cli` - Testing/verification tool - 27 tests
6. `sync-relay` - **COMPLETE** (51 tests) - relay server with SQLite, HTTP endpoints, rate limiting, notify_group
7. Framework integrations - Optional wrappers (e.g., tauri-plugin-sync)

**Current Phase:** PHASE 6 COMPLETE — 309 tests passing, 34 ignored

**Key Files:**
- `docs/DOCS-MAP.md` - Navigation index (start here)
- `docs/02-SPECIFICATION.md` - Full technical specification
- `docs/03-IMPLEMENTATION-PLAN.md` - TDD implementation guide
- `docs/research/iroh-deep-dive-report.md` - Amendment source (spec changes)
- `Cargo.toml` (workspace root) - Workspace definition
- `sync-types/src/lib.rs` - Wire format types

**Architecture:**
```
Local-First App → sync-client → iroh (QUIC) → sync-relay → SQLite
                      ↓
               XChaCha20-Poly1305 (E2E encryption)
```

**Protocol Stack:**
```
Application Messages (Push/Pull)
        ↓
    Envelope (routing)
        ↓
XChaCha20-Poly1305 (E2E encryption)
        ↓
    iroh (QUIC + TLS 1.3)
```

**Security Reminders:**
- Relay is zero-knowledge (never sees plaintext)
- Use cursors for ordering, not timestamps
- Never log blob contents
- Rate limit everything

---

## MCP Servers (Q's Toolbox)

The following MCP servers are available for this project. All run on Beast (100.71.79.25) via systemd.

### Essential for 0k-Sync

| Server | Tool | Port | Purpose |
|--------|------|------|---------|
| `rust-rag` | `mcp__rust-rag__rust_dev_search` | 8005 | Rust patterns, serde, tokio, thiserror, async |
| `iroh-rag` | `mcp__iroh-rag__iroh_ecosystem_search` | 8008 | iroh P2P, QUIC, blobs, gossip, relay |
| `crypto-rag` | `mcp__crypto-rag__crypto_protocols_search` | 8009 | Noise Protocol, hybrid crypto, ML-KEM |
| `0k-sync-rag` | `mcp__0k-sync-rag__project_0k_sync_search` | 8101 | **Project MCP** — search this codebase |

### Future Phases

| Server | Tool | Port | Purpose |
|--------|------|------|---------|
| `tauri-rag` | `mcp__tauri-rag__tauri_dev_search` | 8004 | Tauri 2.x commands, plugins, state |

### Not Needed

| Server | Purpose | Why Not |
|--------|---------|---------|
| `zk-runtime-rag` | deno_core, arkworks ZK | Different project |
| `cardano-rag` | Cardano, Plutus, CIPs | Different project |
| `solidjs-rag` | SolidJS frontend | No frontend work |
| `lucid-rag` | Lucid Evolution | Different project |
| `aiken-rag` | Aiken validators | Different project |
| `vault-core-rag` | vault-core project | Different project |

### Re-indexing the Project MCP

When the 0k-sync codebase changes significantly (feature branches merged, major refactors, before code reviews), re-index from Q:

```bash
ssh jimmyb@100.71.79.25 "reingest-project 0k-sync"
```

Full guide: Beast at `/home/jimmyb/projects/RAGv1/docs/MCP-QUICK-START.md`

### Usage Examples

```
# Search this project's codebase via MCP
mcp__0k-sync-rag__project_0k_sync_search("notify_group implementation")

# Search Rust patterns
mcp__rust-rag__rust_dev_search("serde MessagePack serialization")

# Search iroh documentation
mcp__iroh-rag__iroh_ecosystem_search("iroh-blobs content-addressed storage")

# Search crypto protocols
mcp__crypto-rag__crypto_protocols_search("Noise Protocol XX handshake")
```

---

*Last updated: 2026-02-05*
*Template Version: 1.7.0*
