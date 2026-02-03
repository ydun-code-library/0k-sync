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

# Run CLI Tool
cargo run -p sync-cli -- push "message"
cargo run -p sync-cli -- pull --after-cursor 0
cargo run -p sync-cli -- pair --create
```

### Project-Specific Notes

**This is a Rust Cargo workspace** with 5 core crates:
1. `sync-types` - Shared types (Envelope, Message, etc.)
2. `sync-core` - Pure logic, no I/O (instant tests)
3. `sync-client` - Library for local-first apps
4. `sync-cli` - Testing/verification tool
5. `sync-relay` - Custom relay (future, Tiers 2-6)
6. Framework integrations - Optional wrappers (e.g., tauri-plugin-sync)

**Implementation Order:**
1. sync-types first (everything depends on wire format)
2. sync-core second (pure logic, no I/O)
3. sync-client third (iroh integration)
4. sync-cli fourth (fastest way to test protocol)
5. sync-relay fifth (custom relay, future)
6. Framework integrations last (thin wrappers around sync-client)

**Key Files:**
- `docs/DOCS-MAP.md` - Navigation index (start here)
- `docs/02-SPECIFICATION.md` - Full technical specification
- `docs/03-IMPLEMENTATION-PLAN.md` - TDD implementation guide
- `docs/research/iroh-deep-dive-report.md` - Amendment source (spec changes)
- `Cargo.toml` (workspace root) - Workspace definition
- `sync-types/src/lib.rs` - Wire format types

**Architecture:**
```
Local-First App → sync-client → WebSocket → sync-relay → SQLite
                      ↓
               Noise Protocol (E2E encryption)
```

**Protocol Stack:**
```
Application Messages (Push/Pull)
        ↓
    Envelope (routing)
        ↓
Noise Protocol (encryption)
        ↓
    WebSocket
        ↓
      TLS
```

**Security Reminders:**
- Relay is zero-knowledge (never sees plaintext)
- Use cursors for ordering, not timestamps
- Never log blob contents
- Rate limit everything

---

## MCP Servers (Q's Toolbox)

The following MCP servers are available for this project. Check these at session start.

### Essential for 0k-Sync

| Server | Tool | Purpose |
|--------|------|---------|
| `rust-rag` | `mcp__rust-rag__rust_dev_search` | Rust patterns, serde, tokio, thiserror, async |
| `iroh-rag` | `mcp__iroh-rag__iroh_ecosystem_search` | iroh P2P, QUIC, blobs, gossip, relay |
| `crypto-rag` | `mcp__crypto-rag__crypto_protocols_search` | Noise Protocol, hybrid crypto, ML-KEM |

### Future Phases

| Server | Tool | Purpose |
|--------|------|---------|
| `tauri-rag` | `mcp__tauri-rag__tauri_dev_search` | Tauri 2.x commands, plugins, state (Phase 5) |

### Not Needed

| Server | Purpose | Why Not |
|--------|---------|---------|
| `zk-runtime-rag` | deno_core, arkworks ZK | Different project |
| `cardano-rag` | Cardano, Plutus, CIPs | Different project |
| `solidjs-rag` | SolidJS frontend | No frontend work |
| `lucid-rag` | Lucid Evolution | Different project |
| `aiken-rag` | Aiken validators | Different project |
| `vault-core-rag` | vault-core project | Different project |

### Usage Examples

```
# Search Rust patterns
mcp__rust-rag__rust_dev_search("serde MessagePack serialization")

# Search iroh documentation
mcp__iroh-rag__iroh_ecosystem_search("iroh-blobs content-addressed storage")

# Search crypto protocols
mcp__crypto-rag__crypto_protocols_search("Noise Protocol XX handshake")
```

---

*Last updated: 2026-02-03*
*Template Version: 1.7.0*
