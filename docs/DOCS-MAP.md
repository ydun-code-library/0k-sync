# Sync-Relay Documentation Map

**Version**: 1.7
**Last Updated**: 2026-02-07
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
| `../NEXT-SESSION-START-HERE.md` | Session continuity, Docker gotchas | Starting new session |
| `../JIMMYS-WORKFLOW.md` | Workflow system v2.1 | Before implementing anything |
| `../sync-mvp-roadmap.md` | Product tier roadmap | Understanding deployment strategy |
| `../appendix-b-hybrid-crypto.md` | Post-quantum crypto design | Security/crypto decisions |
| `../Dockerfile` | Production relay image (multi-stage) | Docker builds, deployment |
| `../.dockerignore` | Docker build context exclusions | Modifying Docker build |

### Core Documentation (docs/)

| File | Lines | Purpose | When to Read |
|------|-------|---------|--------------|
| `00-PLAN.md` | 174 | Documentation planning | Understanding doc structure |
| `01-EXECUTIVE-SUMMARY.md` | 265 | Technical overview (v2.4.0) | High-level understanding |
| `02-SPECIFICATION.md` | 2,200+ | **PRIMARY SPEC** — Full protocol (v2.5.0, incl. Section 18 bindings) | Implementation reference |
| `03-IMPLEMENTATION-PLAN.md` | 3,400+ | TDD implementation guide (v2.5.0, Phase 8A/8B/8C COMPLETE) | Before writing code |
| `04-RESEARCH-VALIDATION.md` | 652 | Technology choices | Understanding "why" decisions |
| `05-RELEASE-STRATEGY.md` | 930 | Versioning, publishing, CI/CD | Before release activities |
| `06-CHAOS-TESTING-STRATEGY.md` | 775 | Failure testing, 68 core + 17 binding scenarios | Building chaos test harness |
| `07-DISTRIBUTED-TESTING-GUIDE.md` | ~350 | Distributed testing across Q/Beast/Guardian mesh | Running distributed tests, relay ops |
| `E2E-TESTING-GUIDE.md` | ~150 | Manual E2E testing Q ↔ Beast | Running integration tests |
| `MULTI-RELAY-SPEC.md` | ~200 | Multi-relay fan-out architecture (Phase 6.5) | Multi-relay implementation |

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
| `pq-crypto-shake-mlkem.md` | 150 | SHAKE256 + ML-KEM architecture | PQ implementation |
| `oprf-passphrase-hardening.md` | 180 | OPRF for offline attack prevention | Enterprise tier design |

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
├── defines → 68 core chaos scenarios across 6 categories
├── defines → Test environment (The Beast)
├── integrates with → CI/CD pipeline (smoke chaos in PRs)
├── phased with → 03-IMPLEMENTATION-PLAN.md (chaos per impl phase)
└── extended by → 07-DISTRIBUTED-TESTING-GUIDE.md (37 multi-machine scenarios)

07-DISTRIBUTED-TESTING-GUIDE.md (Distributed Testing)
├── extends → 06-CHAOS-TESTING-STRATEGY.md (multi-machine layer)
├── defines → 37 distributed scenarios (MR, CM, EDGE, NET, CONV)
├── defines → 3-relay permanent infrastructure on Beast
├── defines → Relay observability (/health, /metrics, Prometheus)
├── operations → Relay startup, monitoring, troubleshooting
└── machines → Q (orchestrator), Beast (relays), Guardian (edge)

Dockerfile + Docker files (Containerization)
├── builds → sync-relay binary (multi-stage)
├── config → sync-relay/relay.docker.toml (database at /data/relay.db)
├── tested by → tests/docker-validate.sh (8 validation tests)
├── chaos infra → tests/chaos/docker-compose.chaos.yml
├── gotchas documented in → NEXT-SESSION-START-HERE.md (Docker Gotchas table)
└── build lessons in → AGENTS.md (Docker Build Notes section)
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
| snow → clatter migration | All docs | ✅ Docs updated | ⚠️ Code not yet implemented (F-002) |

**⚠️ Version Reality Check (2026-02-03):**

| Documented | Actual | Notes |
|------------|--------|-------|
| iroh 1.0 RC | **iroh 0.96** | 1.0 not yet released; 0.96 is latest on crates.io |
| iroh-blobs 1.0 | **iroh-blobs 0.98** | Same situation |

**Note:** Documentation references "iroh 1.0 RC" throughout but actual implementation uses iroh 0.96 with cargo patch for curve25519-dalek compatibility.

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
- `research/pq-crypto-shake-mlkem.md` — SHAKE256 + ML-KEM for PQ compliance
- `research/oprf-passphrase-hardening.md` — OPRF for offline attack prevention

### Release & Quality Documents
- `05-RELEASE-STRATEGY.md` — Versioning, publishing, CI/CD, quality gates
- `06-CHAOS-TESTING-STRATEGY.md` — Chaos testing, 68 core + 17 binding failure scenarios
- `07-DISTRIBUTED-TESTING-GUIDE.md` — Distributed testing across Q/Beast/Guardian, relay operations

### Docker & Deployment Files
- `../Dockerfile` — Production relay image (multi-stage build)
- `../.dockerignore` — Build context exclusions
- `../sync-relay/relay.docker.toml` — Docker config (database at /data/relay.db)
- `../sync-relay/relay.toml.example` — Config template for local development
- `../tests/docker-validate.sh` — Docker validation tests (8 tests)
- `../tests/chaos/Dockerfile.relay` — Relay image for chaos testing
- `../tests/chaos/Dockerfile.cli` — CLI image for chaos testing
- `../tests/chaos/docker-compose.chaos.yml` — Chaos testing topology (toxiproxy)
- `../tests/chaos/docker-compose.distributed.yml` — Distributed testing topology (3-relay on Beast)

### Workflow Documents
- `AGENTS.md` — AI guidelines
- `CLAUDE.md` — Quick reference
- `JIMMYS-WORKFLOW.md` — Implementation workflow
- `STATUS.md` — Progress tracking
- `NEXT-SESSION-START-HERE.md` — Session continuity

### Review Documents (docs/reviews/)
- `2026-02-05-security-audit-report.md` — Security audit v1 (41 findings, 21 fixed)
- `2026-02-05-security-audit-v2-report.md` — Security audit v2 post-remediation (35 findings, 4 MEDIUM fixed)

### Handoff Documents (docs/handoffs/)
- `P2-MONEY-Q-0k-sync-implementation-handoff.md` — Q's implementation handoff from Moneypenny
- `P2-MARKETING-Q-0k-sync-bindings-request.md` — Marketing's formal request for multi-language bindings
- `P3-Q-MARKETING-0k-sync-favicon-request.md` — Favicon asset request to marketing

### Amendment Documents (docs/)
- `WEBSOCKET-REMOVAL-AMENDMENT.md` — Unified transport to iroh QUIC (applied 2026-02-03)
- `PHASE-RECONCILIATION-AMENDMENT.md` — Phase reconciliation (applied 2026-02-03)

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

**Navigation Index Version**: 1.7
**Active Documents**: 32 (excludes archive/ and reference/; includes Docker files, audit reports, handoffs)
**Last Audit**: 2026-02-07
