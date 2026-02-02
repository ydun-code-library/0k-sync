# 0k-Sync - Documentation Plan

**Created:** 2026-01-16
**Status:** Planning Phase
**Workflow:** Jimmy's Workflow (RED/GREEN/CHECKPOINT)

---

## Objective

Create 4 comprehensive documents for 0k-Sync:

| Doc | Purpose | Status | Quality |
|-----|---------|--------|---------|
| 01-EXECUTIVE-SUMMARY.md | Technical executive overview (what/why) | Pending | Final draft |
| 02-SPECIFICATION.md | Detailed technical spec (what) | Pending | Final draft |
| 03-IMPLEMENTATION-PLAN.md | TDD implementation approach (how) | Pending | Final draft |
| 04-RESEARCH-VALIDATION.md | Justification & references (why) | Pending | Base + research points |

---

## Key Architecture Decisions (Validated via Research)

### Core Insight
**Client stays constant. Relay tier changes.**

```
┌─────────────────────────────────────────────────────────────────┐
│                 tauri-plugin-sync (CONSTANT)                     │
│  • E2E encryption (Group Key)                                   │
│  • Pairing (QR/short code)                                      │
│  • Push/Pull API                                                │
│  • Cursor tracking                                              │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                          Same wire protocol
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        ▼                         ▼                         ▼
   Tier 1: iroh            Tier 2-3: Self-hosted      Tier 4-6: Managed
   (Vibe coder)            (Home dev / Vercel)        (CrabNebula)
```

### Five Product Tiers

| Tier | Name | Relay | User Pays |
|------|------|-------|-----------|
| 1 | Vibe Coder | iroh public network | Nothing |
| 2 | Home Developer | Self-hosted container | Electricity |
| 3 | Vercel-style | PaaS container | Platform fees |
| 4 | Community Sync | CrabNebula shared pool | Free/cheap |
| 5 | Cloud | CrabNebula dedicated | Usage-based |
| 6 | Enterprise | Customer deploys | License |

### What We're NOT Building

- ❌ Data storage (relay is pass-through only)
- ❌ Hole punching (not needed - all outbound to relay)
- ❌ User accounts (zero-knowledge, QR pairing)
- ❌ Proprietary dependencies (100% open source)

### Technology Stack (Validated)

| Component | Choice | Version | Source |
|-----------|--------|---------|--------|
| P2P (Tier 1) | iroh | 0.32.0 | [n0-computer/iroh](https://github.com/n0-computer/iroh) |
| Noise Protocol | snow | latest | [mcginty/snow](https://github.com/mcginty/snow) |
| WebSocket | tokio-tungstenite | 0.21+ | [snapview/tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) |
| Key Derivation | argon2 (RustCrypto) | 0.5+ | [RustCrypto/password-hashes](https://github.com/RustCrypto/password-hashes) |
| Tauri Plugin | tauri-plugin | 2.0 | [tauri.app](https://v2.tauri.app/develop/plugins/) |
| Public Endpoint | Cloudflare Tunnel | Free tier | [cloudflare.com](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) |

---

## Document Outlines

### 01-EXECUTIVE-SUMMARY.md (2-3 pages)

**Audience:** Technical executives, architects, decision makers

1. **Problem Statement** - Why sync is hard, why Tauri apps need it
2. **Solution Overview** - Zero-knowledge relay, plugin architecture
3. **Product Tiers** - 6 tiers from hobbyist to enterprise
4. **Architecture Summary** - Client constant, relay variable
5. **Technology Choices** - Open source stack, no lock-in
6. **Business Model** - How CrabNebula monetizes (Tiers 4-6)
7. **Competitive Position** - vs Firebase, Supabase, custom solutions

### 02-SPECIFICATION.md (Detailed)

**Audience:** Implementers, developers

1. **Protocol Stack** - Layers, wire format, message types
2. **Security Model** - Noise XX, Group Key, threat model
3. **Client Library** - Public API, encryption layer, Tauri integration
4. **Relay Server** - Responsibilities, storage (temp only), rate limits
5. **Pairing Flow** - QR codes, short codes, no accounts
6. **Tier-Specific Details** - What changes per tier
7. **Configuration** - Config format, defaults, overrides

### 03-IMPLEMENTATION-PLAN.md (TDD)

**Audience:** Implementing developers

1. **Phase Overview** - 6 phases, dependencies
2. **Phase 1: sync-types** - Wire format, tests first
3. **Phase 2: sync-core** - Pure logic, no I/O, instant tests
4. **Phase 3: sync-client** - iroh integration, E2E encryption
5. **Phase 4: sync-cli** - Headless testing tool
6. **Phase 5: tauri-plugin-sync** - Tauri commands
7. **Phase 6: sync-relay** - Custom relay (future)
8. **Testing Strategy** - Unit, integration, E2E
9. **Validation Gates** - What must pass before proceeding

### 04-RESEARCH-VALIDATION.md (Base + Research Points)

**Audience:** Deep research session, auditors

1. **Technology Choices** - Why each choice, alternatives considered
2. **Security Analysis** - Noise Protocol justification, threat model validation
3. **Performance Considerations** - Benchmarks needed, scale limits
4. **Competitive Analysis** - How this compares to alternatives
5. **Open Questions** - Points needing deeper research
6. **References** - Academic papers, specs, documentation

---

## Execution Plan (Jimmy's Workflow)

### 🔴 RED Phase: Implementation

| Step | Task | Validation |
|------|------|------------|
| 1 | Write 01-EXECUTIVE-SUMMARY.md | Covers all key points |
| 2 | Write 02-SPECIFICATION.md | Technically complete |
| 3 | Write 03-IMPLEMENTATION-PLAN.md | TDD approach clear |
| 4 | Write 04-RESEARCH-VALIDATION.md | Research points identified |

### 🟢 GREEN Phase: Validation

| Check | Criteria |
|-------|----------|
| Consistency | All docs use same terminology |
| Completeness | All tiers covered in each doc |
| Accuracy | Tech versions match research |
| Cross-references | Docs reference each other correctly |

### 🔵 CHECKPOINT

- All 4 docs created
- Consistency verified
- Ready for deep research (doc 4)
- Ready for Beast audit

---

## File Structure

```
sync-relay/
├── docs/
│   ├── 00-PLAN.md                    ← This file
│   ├── 01-EXECUTIVE-SUMMARY.md       ← Technical executive overview
│   ├── 02-SPECIFICATION.md           ← Detailed spec
│   ├── 03-IMPLEMENTATION-PLAN.md     ← TDD plan
│   └── 04-RESEARCH-VALIDATION.md     ← Research base
├── hosted-sync-relay-spec.md         ← Original spec (reference)
├── decentralised-sync-relay-spec.md  ← iroh-first spec (reference)
└── AGENTS.md                         ← Project guidelines
```

---

**Next:** Execute RED phase - write documents in order.
