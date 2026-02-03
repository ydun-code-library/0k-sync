# Handoff: 0k-Sync Implementation Phase

**From:** Moneypenny
**To:** Q
**Date:** 2026-02-03
**Project:** 0k-Sync (sync-relay)
**Priority:** P2 (Normal)

---

## Summary

Design phase complete. Documentation suite ready. Chaos testing integrated into implementation plan. Your turn to build.

---

## Project Location

```
Local:  /home/jimmyb/crabnebula/sync-relay
Remote: https://github.com/ydun-code-library/0k-sync
Branch: main (clean)
```

---

## Pre-Flight Checklist (MANDATORY)

Before writing any code, you MUST:

1. **Check your available MCP servers** — inventory what tools you have access to
2. **Read the documentation in order:**
   - `docs/DOCS-MAP.md` — Navigation index (start here)
   - `docs/01-EXECUTIVE-SUMMARY.md` — High-level overview
   - `docs/02-SPECIFICATION.md` — Full protocol spec (1,684 lines)
   - `docs/03-IMPLEMENTATION-PLAN.md` — TDD plan with chaos (2,213 lines, v2.2.0)
   - `docs/06-CHAOS-TESTING-STRATEGY.md` — 68 scenarios (778 lines, v1.5.0)
3. **Verify everything** — Don't trust summaries, read the source
4. **Note any issues** — Flag anything unclear or inconsistent

---

## What's Been Done

| Item | Status | Notes |
|------|--------|-------|
| Architecture design | ✅ Complete | 6 product tiers, client constant |
| Protocol specification | ✅ Complete | Noise XX, cursor-based ordering |
| Implementation plan | ✅ Complete | TDD approach, v2.2.0 |
| Chaos testing strategy | ✅ Complete | 68 scenarios, 5 invariants |
| Release strategy | ✅ Complete | SemVer, CI/CD tiers |
| GitHub repo | ✅ Created | ydun-code-library/0k-sync |
| Pre-flight audit | ✅ Fixed | 14 issues resolved |
| Chaos integration | ✅ Applied | 13 amendments, 100 "chaos" refs |

---

## What You're Building

**6 Crates (in order):**

1. `sync-types` — Wire format, message definitions (START HERE)
2. `sync-core` — Pure logic, no I/O
3. `sync-client` — Library for local-first apps
4. `sync-content` — Content-addressable storage
5. `sync-cli` — Testing/verification tool
6. `sync-relay` — Custom relay server

**Crate Naming for crates.io:** `zerok-sync-*` prefix (e.g., `zerok-sync-types`)

---

## Implementation Order (Phase 1 First)

```
Phase 1: sync-types     → Wire format types
Phase 2: sync-core      → Pure sync logic
Phase 3: sync-client    → iroh integration
Phase 3.5: sync-content → Content-addressable blobs
Phase 4: sync-cli       → CLI testing tool
Phase 5: Framework      → Optional wrappers
Phase 6: sync-relay     → Custom relay (future)
```

Each phase includes chaos deliverables. See implementation plan for details.

---

## Key Technical Decisions (Already Made)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Transport | WebSocket over TLS | Universal, firewall-friendly |
| Encryption | Noise XX (clatter 2.1) | Mutual auth, hybrid post-quantum |
| Ordering | Cursors, not timestamps | Relay-assigned, monotonic |
| Storage | SQLite (sqlx 0.7) | Embedded, reliable |
| Serialization | MessagePack | Compact, schema-flexible |
| Relay model | Zero-knowledge | Pass-through only, never sees plaintext |

---

## Critical Rules

1. **NEVER log plaintext or blob contents**
2. **Use cursors for ordering, not timestamps**
3. **Relay is zero-knowledge** — pass-through encryption only
4. **TDD approach** — tests first, then implementation
5. **Jimmy's Workflow** — PRE-FLIGHT → IMPLEMENT → VALIDATE → CHECKPOINT
6. **Rate limit everything**

---

## Chaos Testing (Integrated)

Chaos infrastructure builds alongside code, not after:

- Phase 1: Harness skeleton
- Phase 2: Assertion helpers
- Phase 3: 32 scenario stubs (16 encryption + 16 transport)
- Phase 3.5: 10 content chaos scenarios
- Phase 4: 12 sync chaos scenarios
- Phase 6: Full 68-scenario activation

Test environment: The Beast (96GB RAM server)

---

## Open Questions

None blocking. All architectural decisions documented.

---

## Your First Task

1. Inventory your MCP servers
2. Read `docs/DOCS-MAP.md`
3. Read `docs/03-IMPLEMENTATION-PLAN.md` Section 4 (Phase 1)
4. Create Cargo workspace structure
5. Implement `sync-types` crate skeleton
6. Write tests first (TDD)

---

## Files to Know

| File | Purpose |
|------|---------|
| `AGENTS.md` | AI assistant guidelines |
| `CLAUDE.md` | Quick reference |
| `STATUS.md` | Project status tracker |
| `JIMMYS-WORKFLOW.md` | Workflow system (v2.1) |
| `docs/DOCS-MAP.md` | Documentation index |
| `docs/02-SPECIFICATION.md` | Protocol spec |
| `docs/03-IMPLEMENTATION-PLAN.md` | TDD plan (v2.2.0) |

---

## Success Criteria (Phase 1)

- [ ] Cargo workspace created
- [ ] All message types defined
- [ ] Serialization round-trip tests pass
- [ ] Types are ergonomic to use
- [ ] Chaos harness skeleton in place

---

## Contact

Issues or blockers → Flag to Moneypenny via Dead Drop
Format: `P1-Q-MONEY-0k-sync-[description].md`

---

*Clean handoff. Measure twice, cut once. Good luck, Q.*

**— Moneypenny**
