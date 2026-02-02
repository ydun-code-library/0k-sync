# Sync-Relay Organization & Template Alignment Plan

**Created**: 2026-02-02
**Total Steps**: 5
**Estimated Time**: 2-3 hours
**Autonomous Mode**: ENABLED (with checkpoints for human review)
**Workflow Version**: Jimmy's Workflow v2.1

---

## Workflow Overview

**Goal**: Organize sync-relay project, align with templates v3.0, integrate DeadDrop research documents, and prepare for Q's implementation phase.

**Success Criteria**:
- All template files updated to current versions
- Research documents organized in docs/research/
- DOCS-MAP.md provides clear navigation
- STATUS.md reflects current project state
- Project ready for Q's implementation kickoff

**Dependencies**:
- DeadDrop documents available at `/home/jimmyb/DeadDrop/`
- Templates available at `/home/jimmyb/templates/`
- sync-relay project at `/home/jimmyb/crabnebula/sync-relay/`

---

## PRE-FLIGHT Assessment (COMPLETED)

### Context Inventory

**Requirements Clarity**:
- [x] Task clearly defined: Organize sync-relay, align templates, add research docs
- [x] Success criteria explicit: Template compliance, research docs in place, ready for Q
- [x] Edge cases identified: Amendment application scope (flag for Q vs apply now)

**Files Inventory**:

| Location | File | Status | Notes |
|----------|------|--------|-------|
| sync-relay | AGENTS.md | ✅ Have | v1.6.0 — needs update to v1.7.0 |
| sync-relay | CLAUDE.md | ✅ Have | v1.5.1 — review needed |
| sync-relay | JIMMYS-WORKFLOW.md | ✅ Have | v1.1 — **CRITICAL: needs v2.1** |
| sync-relay | STATUS.md | ✅ Have | Dated 2026-01-16, needs update |
| sync-relay | NEXT-SESSION-START-HERE.md | ✅ Have | Needs review |
| sync-relay/docs | 00-PLAN.md through 04-RESEARCH-VALIDATION.md | ✅ Have | Current |
| sync-relay/docs | DOCS-MAP.md | ❌ Need | Must create |
| sync-relay/docs | research/ | ❌ Need | Directory doesn't exist |
| DeadDrop | cn-sync-*.md | ✅ Have | **IDENTICAL** to sync-relay (checksums match) |
| DeadDrop | iroh-deep-dive-report.md | ✅ Have | 690 lines — goes to research/ |
| DeadDrop | tactical-mesh-profile-appendix-d.md | ✅ Have | 877 lines — goes to research/ |
| templates | AGENTS.md.template | ✅ Have | v1.7.0 |
| templates | JIMMYS-WORKFLOW.md | ✅ Have | v2.1 |

**Key Finding**: DeadDrop cn-sync-* files are **byte-identical** to sync-relay/docs/. No reconciliation needed.

**Domain Knowledge**:
- [x] Template structure understood (explored ~/templates/projects/)
- [x] Jimmy's Workflow v2.1 reviewed (PRE-FLIGHT phase is new)
- [x] Project structure understood (explored sync-relay)
- [x] Amendment scope understood (iroh deep dive informs spec changes)

**Pre-flight Decision**: 🟢 CLEAR

---

## Step 1: Update JIMMYS-WORKFLOW.md to v2.1

### 🔴 PRE-FLIGHT

**Status**: 🟢 CLEAR

**Rationale**: This is the most critical update. v1.1 → v2.1 adds the PRE-FLIGHT check phase which is foundational to all subsequent work. Must be done first.

**Source**: `/home/jimmyb/templates/projects/core/JIMMYS-WORKFLOW.md`
**Target**: `/home/jimmyb/crabnebula/sync-relay/JIMMYS-WORKFLOW.md`

---

### 🔴 IMPLEMENT

**Pre-flight**: 🟢 CLEAR

**Task**:
- [ ] Copy JIMMYS-WORKFLOW.md v2.1 from templates to sync-relay
- [ ] Verify version header shows v2.1

**Files to Modify**:
- `/home/jimmyb/crabnebula/sync-relay/JIMMYS-WORKFLOW.md`

**Expected Outcome**: sync-relay has current workflow system with PRE-FLIGHT phase

**Complexity**: 🟢 Simple
**Estimated Time**: 5 minutes

**Success Criteria**:
- [ ] File updated
- [ ] Version shows 2.1
- [ ] PRE-FLIGHT section present

**AI Assistance Disclosure**: Yes — Claude Code executing plan

---

### 🟢 VALIDATE

#### Automated Checks
| Check | Command | Result | Proves |
|-------|---------|--------|--------|
| File exists | `ls -la JIMMYS-WORKFLOW.md` | Pending | File present |
| Version check | `grep "version.*2.1" JIMMYS-WORKFLOW.md` | Pending | Correct version |
| PRE-FLIGHT present | `grep "PRE-FLIGHT" JIMMYS-WORKFLOW.md` | Pending | New phase included |

#### Validation Reasoning

**Confidence**: HIGH (after execution)

**Why This Proves Correctness**:
1. Direct copy from authoritative template source
2. Version grep confirms correct version
3. PRE-FLIGHT grep confirms key v2.1 feature

**Weaknesses Acknowledged**:
- [ ] Does NOT verify all content intact (file size check recommended)

---

### 🔵 CHECKPOINT: JIMMYS-WORKFLOW Updated

**Status**: 🟡 IN_PROGRESS
**Confidence**: HIGH (expected)
**Dependencies**: None
**Blocks**: All subsequent steps (workflow is foundational)

**Rollback**:
```bash
git checkout HEAD -- JIMMYS-WORKFLOW.md
```

---

## Step 2: Update AGENTS.md to Template v1.7.0

### 🔴 PRE-FLIGHT

**Status**: 🟢 CLEAR

**Rationale**: AGENTS.md is the primary AI instruction file. Update preserves project-specific content while bringing template structure current.

**Current Version**: v1.6.0
**Target Version**: v1.7.0

**Changes in v1.7.0** (from CHANGELOG):
- AI-Optimized Documentation section (5.5)
- PRE-FLIGHT references in Jimmy's Workflow section
- Minor template improvements

---

### 🔴 IMPLEMENT

**Pre-flight**: 🟢 CLEAR

**Task**:
- [ ] Review current AGENTS.md project-specific sections (Important Context)
- [ ] Update template version header to 1.7.0
- [ ] Add section 5.5 (AI-Optimized Documentation) if missing
- [ ] Update Jimmy's Workflow section to reference PRE-FLIGHT
- [ ] Preserve all project-specific content

**Files to Modify**:
- `/home/jimmyb/crabnebula/sync-relay/AGENTS.md`

**Expected Outcome**: AGENTS.md at v1.7.0 with project-specific content preserved

**Complexity**: 🟡 Moderate (must preserve customizations)
**Estimated Time**: 20 minutes

**Success Criteria**:
- [ ] Template version shows 1.7.0
- [ ] Section 5.5 (AI-Optimized Documentation) present
- [ ] PRE-FLIGHT mentioned in Jimmy's Workflow section
- [ ] Project-specific Important Context preserved
- [ ] All 11 principles present (1-11)

**AI Assistance Disclosure**: Yes — Claude Code executing plan

---

### 🟢 VALIDATE

#### Manual Verification
| Check | Expected | Status |
|-------|----------|--------|
| Version header | TEMPLATE_VERSION: 1.7.0 | Pending |
| Section 5.5 | AI-Optimized Documentation present | Pending |
| Important Context | Sync Relay description preserved | Pending |
| Principles 9-11 | Measure Twice, No Shortcuts, Rules Persist | Pending |

#### Validation Reasoning

**Confidence**: MEDIUM

**Why This Proves Correctness**:
1. Version header confirms template version
2. Section checks confirm structural updates
3. Content preservation check confirms no data loss

**Weaknesses Acknowledged**:
- [ ] NOT verified: All minor template improvements captured
- [ ] Human spot-check recommended for Important Context preservation

---

### 🔵 CHECKPOINT: AGENTS.md Updated

**Status**: 🟡 IN_PROGRESS
**Confidence**: MEDIUM
**Dependencies**: Step 1 (JIMMYS-WORKFLOW)
**Blocks**: None

**Rollback**:
```bash
git checkout HEAD -- AGENTS.md
```

---

## Step 3: Create docs/research/ and Add Research Documents

### 🔴 PRE-FLIGHT

**Status**: 🟢 CLEAR

**Rationale**: Research documents belong in a dedicated subdirectory per briefing instructions. These are reference materials that inform the spec but are not part of the core documentation.

**Documents to Add**:
| Document | Lines | Purpose |
|----------|-------|---------|
| iroh-deep-dive-report.md | 690 | iroh ecosystem audit, integration recommendations |
| tactical-mesh-profile-appendix-d.md | 877 | Strategic portfolio document, defense/tactical applications |

---

### 🔴 IMPLEMENT

**Pre-flight**: 🟢 CLEAR

**Task**:
- [ ] Create `docs/research/` directory
- [ ] Copy `iroh-deep-dive-report.md` from DeadDrop
- [ ] Copy `tactical-mesh-profile-appendix-d.md` from DeadDrop
- [ ] Verify file integrity (line counts match)

**Files to Create**:
- `/home/jimmyb/crabnebula/sync-relay/docs/research/` (directory)
- `/home/jimmyb/crabnebula/sync-relay/docs/research/iroh-deep-dive-report.md`
- `/home/jimmyb/crabnebula/sync-relay/docs/research/tactical-mesh-profile-appendix-d.md`

**Expected Outcome**: Research documents organized in dedicated subdirectory

**Complexity**: 🟢 Simple
**Estimated Time**: 10 minutes

**Success Criteria**:
- [ ] Directory exists
- [ ] iroh-deep-dive-report.md: 690 lines
- [ ] tactical-mesh-profile-appendix-d.md: 877 lines

**AI Assistance Disclosure**: Yes — Claude Code executing plan

---

### 🟢 VALIDATE

#### Automated Checks
| Check | Command | Result | Proves |
|-------|---------|--------|--------|
| Directory exists | `ls -d docs/research/` | Pending | Structure correct |
| iroh report lines | `wc -l docs/research/iroh-deep-dive-report.md` | Pending | 690 lines |
| tactical mesh lines | `wc -l docs/research/tactical-mesh-profile-appendix-d.md` | Pending | 877 lines |

#### Validation Reasoning

**Confidence**: HIGH

**Why This Proves Correctness**:
1. Line counts match source documents
2. Direct copy preserves content integrity

**Weaknesses Acknowledged**:
- [ ] Does NOT verify content semantically (line count sufficient for copy operation)

---

### 🔵 CHECKPOINT: Research Documents Added

**Status**: 🟡 IN_PROGRESS
**Confidence**: HIGH
**Dependencies**: None
**Blocks**: Step 4 (DOCS-MAP needs to reference these)

**Rollback**:
```bash
rm -rf docs/research/
```

---

## Step 4: Create DOCS-MAP.md

### 🔴 PRE-FLIGHT

**Status**: 🟢 CLEAR

**Rationale**: Project now has >5 documentation files. DOCS-MAP.md provides AI-optimized navigation per documentation standards.

**Documents to Map**:
- docs/00-PLAN.md
- docs/01-EXECUTIVE-SUMMARY.md
- docs/02-SPECIFICATION.md
- docs/03-IMPLEMENTATION-PLAN.md
- docs/04-RESEARCH-VALIDATION.md
- docs/reference/decentralised-sync-relay-spec.md
- docs/reference/hosted-sync-relay-spec.md
- docs/research/iroh-deep-dive-report.md
- docs/research/tactical-mesh-profile-appendix-d.md
- Root: README.md, AGENTS.md, CLAUDE.md, STATUS.md, NEXT-SESSION-START-HERE.md

---

### 🔴 IMPLEMENT

**Pre-flight**: 🟢 CLEAR

**Task**:
- [ ] Create DOCS-MAP.md in docs/
- [ ] Include reading order for new developers
- [ ] Include reading order for AI assistants (Q's entry point)
- [ ] Map all documents with purpose and when-to-read
- [ ] Include cross-reference relationships

**Files to Create**:
- `/home/jimmyb/crabnebula/sync-relay/docs/DOCS-MAP.md`

**Expected Outcome**: Clear navigation for humans and AI assistants

**Complexity**: 🟡 Moderate
**Estimated Time**: 30 minutes

**Success Criteria**:
- [ ] File created
- [ ] All docs listed with purpose
- [ ] Reading order specified
- [ ] Cross-references documented
- [ ] Research documents included

**AI Assistance Disclosure**: Yes — Claude Code executing plan

---

### 🟢 VALIDATE

#### Manual Verification
| Check | Expected | Status |
|-------|----------|--------|
| File exists | docs/DOCS-MAP.md present | Pending |
| All docs listed | 14+ documents referenced | Pending |
| Reading order | Clear entry points defined | Pending |

#### Validation Reasoning

**Confidence**: MEDIUM

**Why This Proves Correctness**:
1. Presence check confirms file created
2. Document count confirms coverage
3. Human review recommended for navigation quality

**Weaknesses Acknowledged**:
- [ ] NOT verified: Navigation actually helpful (subjective)
- [ ] Human spot-check recommended

---

### 🔵 CHECKPOINT: DOCS-MAP Created

**Status**: 🟡 IN_PROGRESS
**Confidence**: MEDIUM
**Dependencies**: Step 3 (research docs must exist first)
**Blocks**: None

**Rollback**:
```bash
rm docs/DOCS-MAP.md
```

---

## Step 5: Update STATUS.md and NEXT-SESSION-START-HERE.md

### 🔴 PRE-FLIGHT

**Status**: 🟢 CLEAR

**Rationale**: Project status files need to reflect current state (post-organization) and provide correct entry point for Q's implementation phase.

---

### 🔴 IMPLEMENT

**Pre-flight**: 🟢 CLEAR

**Task**:
- [ ] Update STATUS.md with:
  - Current date (2026-02-02)
  - Organization work completed
  - New research documents added
  - Template versions updated
  - Ready for Q handoff
- [ ] Update NEXT-SESSION-START-HERE.md with:
  - Entry point for Q
  - Reference to DOCS-MAP.md
  - Priority: sync-types crate implementation
  - Amendment notes (iroh deep dive findings)

**Files to Modify**:
- `/home/jimmyb/crabnebula/sync-relay/STATUS.md`
- `/home/jimmyb/crabnebula/sync-relay/NEXT-SESSION-START-HERE.md`

**Expected Outcome**: Status files reflect current state, Q has clear entry point

**Complexity**: 🟡 Moderate
**Estimated Time**: 30 minutes

**Success Criteria**:
- [ ] STATUS.md date updated to 2026-02-02
- [ ] STATUS.md reflects organization work
- [ ] NEXT-SESSION-START-HERE.md references DOCS-MAP.md
- [ ] Q entry point clearly defined

**AI Assistance Disclosure**: Yes — Claude Code executing plan

---

### 🟢 VALIDATE

#### Manual Verification
| Check | Expected | Status |
|-------|----------|--------|
| STATUS.md date | 2026-02-02 | Pending |
| Organization noted | Template updates, research docs | Pending |
| Q entry point | Clear implementation starting point | Pending |

#### Validation Reasoning

**Confidence**: MEDIUM

**Why This Proves Correctness**:
1. Date check confirms currency
2. Content review confirms completeness

**Weaknesses Acknowledged**:
- [ ] NOT verified: Q will find this helpful (requires Q's feedback)
- [ ] Human review recommended before Q handoff

---

### 🔵 CHECKPOINT: Status Files Updated

**Status**: 🟡 IN_PROGRESS
**Confidence**: MEDIUM
**Dependencies**: Steps 1-4
**Blocks**: Q handoff

**Rollback**:
```bash
git checkout HEAD -- STATUS.md NEXT-SESSION-START-HERE.md
```

---

## Final Integration Checkpoint

### 🔵 CHECKPOINT: Sync-Relay Organization Complete

#### All Steps Status
- ⚪ Step 1: JIMMYS-WORKFLOW v2.1 (PENDING)
- ⚪ Step 2: AGENTS.md v1.7.0 (PENDING)
- ⚪ Step 3: Research Documents (PENDING)
- ⚪ Step 4: DOCS-MAP.md (PENDING)
- ⚪ Step 5: Status Files (PENDING)

#### Overall Success Criteria
| Criterion | Status |
|-----------|--------|
| JIMMYS-WORKFLOW at v2.1 | Pending |
| AGENTS.md at v1.7.0 | Pending |
| Research docs in docs/research/ | Pending |
| DOCS-MAP.md provides navigation | Pending |
| STATUS.md current | Pending |
| Ready for Q implementation phase | Pending |

#### What This Plan Does NOT Include

**Flagged for Q's Phase** (per briefing — amendments are implementation-level):
- [ ] Apply iroh-deep-dive amendments to 02-SPECIFICATION.md
- [ ] Add sync-content crate to implementation plan
- [ ] Update Layer 3 (content transfer) in protocol stack
- [ ] Add mDNS local discovery to spec
- [ ] Add self-hosted infrastructure section

**Rationale**: The iroh deep dive contains spec amendments that require implementation decisions. These belong in Q's implementation phase, not Moneypenny's organization phase. The research documents are now available in docs/research/ for Q to reference.

---

## Rollback (Full)

If entire plan needs reverting:

```bash
cd /home/jimmyb/crabnebula/sync-relay
git checkout HEAD -- JIMMYS-WORKFLOW.md AGENTS.md STATUS.md NEXT-SESSION-START-HERE.md
rm -rf docs/research/
rm -f docs/DOCS-MAP.md
```

---

## Notes for Execution

1. **Execute in order** — Steps have dependencies
2. **Checkpoint after each step** — Verify before proceeding
3. **Preserve project-specific content** — Especially in AGENTS.md
4. **Human review at MEDIUM confidence** — Steps 2, 4, 5 warrant spot-checks
5. **Git commit after completion** — Single commit for organization work

---

**Plan Version**: 1.0
**Created By**: Miss Moneypenny (Claude Code)
**Workflow Version**: Jimmy's Workflow v2.1
**Date**: 2026-02-02
