# Sync-Relay Documentation Map

**Version**: 1.0
**Last Updated**: 2026-02-02
**Purpose**: Navigation index for humans and AI assistants

---

## Quick Start Reading Order

### For New Developers
1. `../README.md` — Project overview, what this is
2. `01-EXECUTIVE-SUMMARY.md` — Technical overview, architecture
3. `02-SPECIFICATION.md` — Full protocol specification
4. `03-IMPLEMENTATION-PLAN.md` — TDD implementation guide

### For AI Assistants (Q's Entry Point)
1. `../AGENTS.md` — Development principles, project context
2. `../CLAUDE.md` — Quick reference, commands
3. `DOCS-MAP.md` — This file (navigation)
4. `02-SPECIFICATION.md` — Protocol details
5. `03-IMPLEMENTATION-PLAN.md` — What to build, in what order

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
```

---

## Amendment Status

**Pending amendments from iroh-deep-dive-report.md:**

These changes should be applied during implementation phase:

| Change | Target Document | Priority |
|--------|----------------|----------|
| iroh-blobs for content transfer (Layer 3) | 02-SPECIFICATION.md | HIGH |
| sync-content crate addition | 03-IMPLEMENTATION-PLAN.md | HIGH |
| mDNS local discovery | 02-SPECIFICATION.md | MEDIUM |
| Self-hosted infra (iroh-relay, iroh-dns-server) | 02-SPECIFICATION.md | MEDIUM |
| iroh 1.0 RC target version | 03-IMPLEMENTATION-PLAN.md | LOW |

**Note**: Amendments flagged for Q's implementation phase. Research docs available in `docs/research/` for reference.

---

## Document Categories

### Specification Documents
- `02-SPECIFICATION.md` — The authoritative protocol spec
- `appendix-b-hybrid-crypto.md` — Cryptographic design

### Planning Documents
- `00-PLAN.md` — Documentation planning
- `03-IMPLEMENTATION-PLAN.md` — Implementation approach
- `sync-mvp-roadmap.md` — Product tier strategy

### Context Documents
- `01-EXECUTIVE-SUMMARY.md` — Overview
- `04-RESEARCH-VALIDATION.md` — Technology rationale

### Research Documents
- `research/iroh-deep-dive-report.md` — iroh ecosystem
- `research/tactical-mesh-profile-appendix-d.md` — Tactical applications

### Workflow Documents
- `AGENTS.md` — AI guidelines
- `CLAUDE.md` — Quick reference
- `JIMMYS-WORKFLOW.md` — Implementation workflow
- `STATUS.md` — Progress tracking
- `NEXT-SESSION-START-HERE.md` — Session continuity

### Archive/Historical
- `reference/decentralised-sync-relay-spec.md`
- `reference/hosted-sync-relay-spec.md`

---

## Skip Guidance

**Skip 02-SPECIFICATION.md if:**
- Just need high-level understanding (read 01-EXECUTIVE-SUMMARY.md instead)
- Working on non-protocol tasks (config, deployment)

**Skip research/ documents if:**
- Implementing per existing spec (amendments not yet applied)
- Not making architectural decisions

**Skip reference/ documents if:**
- Not doing historical research
- Working on current implementation

---

**Navigation Index Version**: 1.0
**Document Count**: 14+ files mapped
**Last Audit**: 2026-02-02
