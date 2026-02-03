# Sync-Relay Documentation Map

**Version**: 1.3
**Last Updated**: 2026-02-03
**Purpose**: Navigation index for humans and AI assistants

---

## Quick Start Reading Order

### For New Developers
1. `../README.md` — Project overview, what this is
2. `01-EXECUTIVE-SUMMARY.md` — Technical overview, architecture
3. `02-SPECIFICATION.md` — Full protocol specification
4. `03-IMPLEMENTATION-PLAN.md` — TDD implementation guide

### For AI Assistants (Q's Entry Point)
1. `handoffs/P2-MONEY-Q-0k-sync-implementation-handoff.md` — **READ FIRST** — Handoff from Moneypenny
2. `../AGENTS.md` — Development principles, project context
3. `../CLAUDE.md` — Quick reference, commands
4. `DOCS-MAP.md` — This file (navigation)
5. `02-SPECIFICATION.md` — Protocol details
6. `03-IMPLEMENTATION-PLAN.md` — What to build, in what order

### For Understanding Research Context
1. `research/iroh-deep-dive-report.md` — iroh ecosystem, integration decisions
2. `research/tactical-mesh-profile-appendix-d.md` — Strategic applications
3. `04-RESEARCH-VALIDATION.md` — Technology choices rationale

---

## Document Index

### Root Level Files

| File | Purpose | When to Read |
|------|---------|--------------|
| `../README.md` | Project overview | First contact with project |
| `../AGENTS.md` | AI assistant guidelines, principles | Every session start |
| `../CLAUDE.md` | Quick reference, commands | Need specific commands |
| `../STATUS.md` | Current progress, blockers | Check what's done/pending |
| `../NEXT-SESSION-START-HERE.md` | Session continuity | Starting new session |
| `../JIMMYS-WORKFLOW.md` | Workflow system v2.1 | Before implementing anything |
| `../sync-mvp-roadmap.md` | Product tier roadmap | Understanding deployment strategy |
| `../appendix-b-hybrid-crypto.md` | Post-quantum crypto design | Security/crypto decisions |

### Core Documentation (docs/)

| File | Lines | Purpose | When to Read |
|------|-------|---------|--------------|
| `00-PLAN.md` | 174 | Documentation planning | Understanding doc structure |
| `01-EXECUTIVE-SUMMARY.md` | 256 | Technical overview | High-level understanding |
| `02-SPECIFICATION.md` | 1,684 | **PRIMARY SPEC** — Full protocol | Implementation reference |
| `03-IMPLEMENTATION-PLAN.md` | 1,845 | TDD implementation guide | Before writing code |
| `04-RESEARCH-VALIDATION.md` | 652 | Technology choices | Understanding "why" decisions |
| `05-RELEASE-STRATEGY.md` | 930 | Versioning, publishing, CI/CD | Before release activities |
| `06-CHAOS-TESTING-STRATEGY.md` | 775 | Failure testing, 68 scenarios | Building chaos test harness |

### Reference Documents (docs/reference/)

| File | Purpose | When to Read |
|------|---------|--------------|
| `decentralised-sync-relay-spec.md` | Original decentralized design | Historical context |
| `hosted-sync-relay-spec.md` | Original hosted design | Historical context |

### Research Documents (docs/research/)

| File | Lines | Purpose | When to Read |
|------|-------|---------|--------------|
| `iroh-deep-dive-report.md` | 690 | **AMENDMENT SOURCE** — iroh ecosystem audit | Before spec updates |
| `tactical-mesh-profile-appendix-d.md` | 877 | Defense/tactical applications | Strategic context |

---

## Cross-Reference Map

```
02-SPECIFICATION.md (Protocol Spec)
├── references → 01-EXECUTIVE-SUMMARY.md (high-level context)
├── references → ../appendix-b-hybrid-crypto.md (crypto details)
├── AMENDED BY → research/iroh-deep-dive-report.md
│   ├── Layer 3 content transfer (iroh-blobs)
│   ├── mDNS local discovery
│   ├── Self-hosted infrastructure
│   └── sync-content crate addition
└── implemented by → 03-IMPLEMENTATION-PLAN.md

03-IMPLEMENTATION-PLAN.md (Build Guide)
├── follows → 02-SPECIFICATION.md
├── uses → ../JIMMYS-WORKFLOW.md
└── produces → sync-types, sync-core, sync-client, etc.

research/iroh-deep-dive-report.md
├── amends → 02-SPECIFICATION.md
├── informs → 03-IMPLEMENTATION-PLAN.md
└── validates → 04-RESEARCH-VALIDATION.md

research/tactical-mesh-profile-appendix-d.md
├── extends → 02-SPECIFICATION.md (Appendix D)
├── references → research/iroh-deep-dive-report.md
└── strategic context for → enterprise/defense markets

05-RELEASE-STRATEGY.md (Release Playbook)
├── references → 02-SPECIFICATION.md, 03-IMPLEMENTATION-PLAN.md
├── defines → Versioning, crate naming (zerok-sync-*)
├── defines → CI/CD pipeline (3 tiers)
├── defines → Quality gates per milestone
└── coordinates with → 06-CHAOS-TESTING-STRATEGY.md

06-CHAOS-TESTING-STRATEGY.md (Failure Testing)
├── references → 02-SPECIFICATION.md, 03-IMPLEMENTATION-PLAN.md, 05-RELEASE-STRATEGY.md
├── defines → 68 chaos scenarios across 6 categories
├── defines → Test environment (The Beast)
├── integrates with → CI/CD pipeline (smoke chaos in PRs)
└── phased with → 03-IMPLEMENTATION-PLAN.md (chaos per impl phase)
```

---

## Amendment Status

**✅ Amendments from iroh-deep-dive-report.md: APPLIED (2026-02-02)**

| Change | Target Document | Status |
|--------|----------------|--------|
| iroh-blobs for content transfer (Layer 3) | 02-SPECIFICATION.md | ✅ Applied |
| sync-content crate addition | 03-IMPLEMENTATION-PLAN.md | ✅ Applied |
| mDNS local discovery | 02-SPECIFICATION.md | ✅ Applied |
| Self-hosted infra (iroh-relay, iroh-dns-server) | 02-SPECIFICATION.md | ✅ Applied |
| iroh 1.0 RC target version | All docs | ✅ Applied |
| snow → clatter migration | All docs | ✅ Applied |

**Reference:** See `IROH-AMENDMENTS-PLAN.md` for detailed amendment tracking.

---

## Document Categories

### Specification Documents
- `02-SPECIFICATION.md` — The authoritative protocol spec
- `appendix-b-hybrid-crypto.md` — Cryptographic design

### Planning Documents
- `03-IMPLEMENTATION-PLAN.md` — Implementation approach
- `sync-mvp-roadmap.md` — Product tier strategy

### Context Documents
- `01-EXECUTIVE-SUMMARY.md` — Overview
- `04-RESEARCH-VALIDATION.md` — Technology rationale

### Research Documents
- `research/iroh-deep-dive-report.md` — iroh ecosystem
- `research/tactical-mesh-profile-appendix-d.md` — Tactical applications

### Release & Quality Documents
- `05-RELEASE-STRATEGY.md` — Versioning, publishing, CI/CD, quality gates
- `06-CHAOS-TESTING-STRATEGY.md` — Chaos testing, 68 failure scenarios

### Workflow Documents
- `AGENTS.md` — AI guidelines
- `CLAUDE.md` — Quick reference
- `JIMMYS-WORKFLOW.md` — Implementation workflow
- `STATUS.md` — Progress tracking
- `NEXT-SESSION-START-HERE.md` — Session continuity

### Handoff Documents (docs/handoffs/)
- `P2-MONEY-Q-0k-sync-implementation-handoff.md` — Q's implementation handoff from Moneypenny

### Archive (Completed Plans)
- `archive/00-PLAN.md` — Documentation plan (executed)
- `archive/IROH-AMENDMENTS-PLAN.md` — Amendment plan (executed 2026-02-02)
- `archive/SYNC-RELAY-ORGANIZATION-PLAN.md` — Organization plan (executed)

### Reference (Superseded Specs)
- `reference/decentralised-sync-relay-spec.md` — Superseded by 02-SPEC v2.2.0
- `reference/hosted-sync-relay-spec.md` — Superseded by 02-SPEC v2.2.0

---

## Skip Guidance

**Skip 02-SPECIFICATION.md if:**
- Just need high-level understanding (read 01-EXECUTIVE-SUMMARY.md instead)
- Working on non-protocol tasks (config, deployment)

**Skip research/ documents if:**
- Implementing per existing spec (amendments already applied)
- Not making architectural decisions

**Skip reference/ documents if:**
- Not doing historical research
- Working on current implementation

---

**Navigation Index Version**: 1.3
**Active Documents**: 18 (excludes archive/ and reference/)
**Last Audit**: 2026-02-03
