# Claude AI Assistant Instructions

<!--
TEMPLATE_VERSION: 1.5.1
TEMPLATE_SOURCE: /home/jimmyb/templates/CLAUDE.md.template
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

### Jimmy's Workflow
Use for all implementation tasks:
- 🔴 **RED**: IMPLEMENT (write code, build features)
- 🟢 **GREEN**: VALIDATE (run tests, prove it works)
- 🔵 **CHECKPOINT**: GATE (mark complete, document rollback)

**Invoke**: *"Let's use Jimmy's Workflow to execute this plan"*

**Reference**: See **JIMMYS-WORKFLOW.md** for complete system

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

**This is a Rust Cargo workspace** with 6 crates:
1. `sync-types` - Shared types (Envelope, Message, etc.)
2. `sync-core` - Pure logic, no I/O (instant tests)
3. `sync-client` - Library for Tauri apps
4. `sync-cli` - Testing/verification tool
5. `tauri-plugin-sync` - Tauri plugin wrapper
6. `sync-relay` - Custom relay (future, Tiers 2-6)

**Implementation Order:**
1. sync-types first (everything depends on wire format)
2. sync-core second (pure logic, no I/O)
3. sync-client third (iroh integration)
4. sync-cli fourth (fastest way to test protocol)
5. tauri-plugin-sync fifth (thin wrapper)
6. sync-relay sixth (custom relay, future)

**Key Files:**
- `docs/02-SPECIFICATION.md` - Full technical specification
- `docs/03-IMPLEMENTATION-PLAN.md` - TDD implementation guide
- `Cargo.toml` (workspace root) - Workspace definition
- `sync-types/src/lib.rs` - Wire format types

**Architecture:**
```
Tauri App → sync-client → WebSocket → sync-relay → SQLite
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

*Last updated: 2026-01-16*
*Template Version: 1.6.0*
