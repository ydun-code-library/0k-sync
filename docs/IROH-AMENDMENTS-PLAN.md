# iroh Deep Dive Amendments - Implementation Plan

**Date:** 2026-02-02
**Source:** `docs/research/iroh-deep-dive-report.md`
**Workflow:** Jimmy's Workflow v2.1 (PRE-FLIGHT → IMPLEMENT → VALIDATE → CHECKPOINT)

---

## PRE-FLIGHT Summary

### What the Report Recommends

| Decision | Status | Impact |
|----------|--------|--------|
| Keep custom sync protocol | CONFIRMED | No change needed |
| Integrate iroh-blobs for large content | **NEW - ADOPT** | Spec + Implementation Plan |
| Add mDNS local discovery | **NEW - ADOPT** | Spec + Implementation Plan |
| Target iroh 1.0 RC | **UPDATED** | Version references throughout |
| Add ContentReference to wire protocol | **NEW** | Spec Section 5 (Messages) |
| Add sync-content crate | **NEW** | Implementation Plan workspace |
| Self-host iroh-relay from day one | **NEW - ADOPT** | Deployment tiers |
| Self-host iroh-dns-server | **NEW - ADOPT** | Deployment tiers |

### Documents Requiring Amendments

1. **docs/02-SPECIFICATION.md** - Technical specification
2. **docs/03-IMPLEMENTATION-PLAN.md** - TDD implementation guide
3. **docs/01-EXECUTIVE-SUMMARY.md** - Protocol stack diagram
4. **AGENTS.md** - Crate structure reference

---

## Amendment Details

### 1. Specification (02-SPECIFICATION.md)

#### 1.1 Add Layer 3: Content Transfer (NEW SECTION)
- Insert new section after Section 6 (Client Library)
- Title: "Section 7: Large Content Transfer Protocol"
- Content: ContentReference message, encrypt-then-hash flow, iroh-blobs integration

#### 1.2 Add ContentReference Message Types
- Add to Section 5.2 (Message Types):
  - `CONTENT_REF` (0x70) - Content reference metadata
  - `CONTENT_ACK` (0x71) - Content transfer acknowledgment
- Add ContentReference struct to Section 5.3 (Message Structures)

#### 1.3 Update Protocol Stack (Section 3)
- Add Layer 3: Content Transfer between Application and Sync Messages
- Add mDNS to Layer 0 discovery list
- Add ALPN table showing both `/private-sync/1` and `/iroh-bytes/4`

#### 1.4 Update Transport Section
- Change iroh version from "v0.35.x" to "1.0 RC" (or stable when available)
- Add mDNS local discovery documentation
- Add self-hosted infrastructure section

#### 1.5 Add Content Key Derivation
- Add to Section 4 (Security Model):
  - Content key derivation from GroupSecret via HKDF-SHA256
  - Separate rotation lifecycle for content keys

#### 1.6 Renumber Sections
After adding new Section 7 (Content Transfer), renumber:
- Old Section 7 (Relay Server) → Section 8
- Old Section 8 (Pairing Flow) → Section 9
- etc.

### 2. Implementation Plan (03-IMPLEMENTATION-PLAN.md)

#### 2.1 Add sync-content Crate to Workspace
Update Section 2.1 (Cargo Workspace):
```
├── sync-content/        # NEW: Content transfer coordinator
│   ├── encrypt.rs       # Encrypt-then-hash pipeline
│   ├── transfer.rs      # iroh-blobs provider/requester wrapper
│   ├── thumbnail.rs     # Preview generation
│   └── lifecycle.rs     # GC coordination, quota management
```

#### 2.2 Update Phase 1 (sync-types)
- Add ContentReference type definition
- Add CONTENT_REF and CONTENT_ACK message types

#### 2.3 Update Phase 3 (sync-client)
- Add iroh-blobs dependency
- Add content encryption with content key derivation
- Add iroh-blobs store initialization

#### 2.4 Add Phase 3.5: sync-content
- NEW phase between sync-client and sync-cli
- Content transfer coordinator implementation
- Tests for encrypt-then-hash, iroh-blobs integration

#### 2.5 Update Phase 5 (Framework Integration)
- Add content transfer progress events
- Add thumbnail handling
- Add progressive loading strategy

#### 2.6 Update Workspace Cargo.toml Example
- Add iroh-blobs dependency
- Update iroh version to 1.0 RC
- Add sync-content to workspace members

### 3. Executive Summary (01-EXECUTIVE-SUMMARY.md)

#### 3.1 Update Protocol Stack Diagram
Add Layer 3: Content Transfer to the stack diagram

#### 3.2 Update Technology Choices Table
- iroh version: "1.0 RC" (not "v0.35.x")
- Add: iroh-blobs for content transfer

### 4. AGENTS.md

#### 4.1 Update Crate Structure
Add sync-content to the workspace structure

#### 4.2 Update Dependencies
- iroh version to 1.0 RC
- Add iroh-blobs dependency

---

## Validation Checklist

**Completed: 2026-02-02**

- [x] Protocol stack shows 5 layers (0-4) with Layer 3 = Content Transfer
- [x] ContentReference message type defined (0x70, 0x71)
- [x] sync-content crate in workspace structure
- [x] iroh version references updated to 1.0 RC
- [x] mDNS mentioned in transport/discovery
- [x] Content key derivation documented
- [x] Self-hosted infrastructure section present
- [x] Section numbers are sequential and TOC updated

---

## Rollback Procedure

If amendments cause issues:
```bash
git revert <commit-hash>
```

All changes are documentation-only, no code impact.

---

## Execution Order

1. **02-SPECIFICATION.md** - Primary spec (largest changes)
2. **03-IMPLEMENTATION-PLAN.md** - Implementation guide
3. **01-EXECUTIVE-SUMMARY.md** - Overview diagram
4. **AGENTS.md** - Reference update

---

*Plan created: 2026-02-02*
*Ready for IMPLEMENT phase*
