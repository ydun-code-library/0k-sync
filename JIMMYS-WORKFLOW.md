# Jimmy's Workflow: Red/Green Checkpoint System

<!--
TEMPLATE_VERSION: 1.1
TEMPLATE_SOURCE: /home/jimmyb/templates/JIMMYS-WORKFLOW.md
DISTRIBUTION: Copy to project root OR reference from platform root
LAST_SYNC: See version check below
-->

**Created**: 2025-10-02
**Version**: 1.1 (Enhanced with Time Tracking, Complexity Rating, Lessons Learned, Micro-Checkpoints)
**Status**: Active Platform-Wide Standard

---

## What's New in v1.1

**Enhanced Checkpoint Tracking:**
- ✅ Time tracking (estimated vs actual)
- ✅ Complexity rating (simple/moderate/complex)
- ✅ Lessons learned capture
- ✅ Variance analysis

**Micro-Checkpoint Variant:**
- ✅ Streamlined format for trivial changes
- ✅ Guard rails to prevent misuse
- ✅ Still requires verification and rollback

**Benefits:**
- Improves estimation accuracy over time
- Helps identify bottlenecks and patterns
- Captures knowledge for future iterations
- Reduces verbosity for simple fixes

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Core Philosophy](#core-philosophy)
3. [Color-Coded Phases](#color-coded-phases)
4. [Machine-Readable Checkpoint Format](#machine-readable-checkpoint-format)
5. [Enhanced Validation Templates](#enhanced-validation-templates)
6. [Rollback Procedures](#rollback-procedures)
7. [Micro-Checkpoint Variant](#micro-checkpoint-variant)
8. [Common Workflow Patterns](#common-workflow-patterns)
9. [Autonomous Execution Mode](#autonomous-execution-mode)
10. [AI Assistant Integration Guide](#ai-assistant-integration-guide)
11. [Failure Recovery Patterns](#failure-recovery-patterns)
12. [State Tracking](#state-tracking)
13. [Quality Gates](#quality-gates)
14. [Integration with TodoWrite](#integration-with-todowrite)
15. [Quick Reference](#quick-reference)

---

## System Overview

**Jimmy's Workflow** is a validation-gate system designed to combat AI hallucination and ensure robust implementation through mandatory checkpoints. It integrates seamlessly with Test-Driven Development (TDD) while adding explicit validation and rollback procedures.

### Core Problem Solved

**The AI Hallucination Problem:**
- AI says "done" ≠ Actually done
- Documentation claims it works ≠ It actually works
- Configuration looks correct ≠ Configuration is correct
- Test passes in theory ≠ Test passes in practice

### Core Solution

**Mandatory Validation Gates:**
```
🔴 RED: IMPLEMENT → 🟢 GREEN: VALIDATE → 🔵 CHECKPOINT: GATE
```

Every implementation step MUST pass through all three phases. No exceptions.

---

## Core Philosophy

### Principle 1: ASSUME NOTHING
- Verify every claim
- Test every assumption
- Prove every feature works
- Document actual results, not intentions

### Principle 2: EXPLICIT OVER IMPLICIT
- Write down exact validation commands
- Define quantifiable success criteria
- Document rollback procedures upfront
- Make checkpoints machine-readable

### Principle 3: FAIL FAST, FAIL SAFE
- Catch problems at validation gates
- Never proceed with broken code
- Always have rollback path ready
- Block progression until GREEN passes

### Principle 4: AUTOMATION-FRIENDLY
- Clear, deterministic validation steps
- Machine-readable status tracking
- Enables autonomous execution
- Supports parallel workflows

---

## Color-Coded Phases

### 🔴 RED: IMPLEMENT Phase

**Purpose**: Write code, build features, make changes

**Required Documentation:**
```markdown
🔴 **IMPLEMENT:**
- Clear task description
- Expected outcome defined
- Estimated time/complexity
- Dependencies identified
- Files to modify listed
- Success criteria previewed
```

**Example:**
```markdown
🔴 **IMPLEMENT:**
- Update service interface to accept simplified parameters
- Remove verbose parameter passing
- Files to modify:
  - src/services/example-service.ts (signature change)
  - src/components/ComponentA.tsx (call site)
  - src/components/ComponentB.tsx (call site)
- Expected: TypeScript compilation succeeds with new signature
- Complexity: 🟡 Moderate (type changes propagate)
- Estimated: 15 minutes
- Dependencies: None
```

**Best Practices:**
- List ALL files that will be touched
- Identify breaking changes upfront
- Note any dependencies on other steps
- Estimate time realistically
- Rate complexity: 🟢 Simple | 🟡 Moderate | 🔴 Complex

---

### 🟢 GREEN: VALIDATE Phase

**Purpose**: Prove the implementation actually works through explicit validation

**Required Documentation:**
```markdown
🟢 **VALIDATE:**
- Run: `specific command` (expect: specific result)
- Check: Manual verification steps
- Performance: Benchmarks if applicable
- Security: Audit checks if applicable
```

**Comprehensive Validation Template:**
```markdown
🟢 **VALIDATE:**

**Automated Tests:**
- Run: `npm test` (expect: all tests passing, no new failures)
- Run: `npm run build` (expect: exit code 0, no errors)
- Run: `npm run typecheck` (expect: 0 TypeScript errors)
- Run: `npm run lint` (expect: no new warnings)
- Run: `npm audit` (expect: 0 vulnerabilities)

**Code Quality:**
- Check: No console.log statements in production code
- Check: No @ts-expect-error directives added
- Check: No hardcoded credentials
- Check: No commented-out code blocks
- Check: Functions under 20 lines
- Check: Files under 200 lines

**Manual Verification:**
- Test: [Specific user scenario in browser]
- Test: [Specific API call with expected response]
- Test: [Database query returns expected data]
- Verify: UI renders correctly
- Verify: Error handling works

**Performance (if applicable):**
- Benchmark: Page load < 2 seconds
- Benchmark: API response < 500ms
- Benchmark: No memory leaks

**Security (if applicable):**
- Check: No exposed API keys
- Check: RLS policies enforced
- Check: Input validation present
- Check: XSS protection in place
```

**Best Practices:**
- Always include automated tests first
- Add manual verification for UX changes
- Define quantifiable success criteria
- Document what "passing" means for each check

---

### 🔵 CHECKPOINT: GATE Phase

**Purpose**: Lock in validated progress, document rollback, gate next step

**Required Documentation:**
```markdown
🔵 **CHECKPOINT:** [Descriptive Name]
**Status**: 🔵 COMPLETE | 🟡 IN_PROGRESS | 🔴 BLOCKED
**Complexity**: 🟢 Simple | 🟡 Moderate | 🔴 Complex
**Validated**: [ISO 8601 timestamp]
**Estimated**: [minutes] (optional but recommended)
**Actual**: [minutes] (populate at completion)
**Tests**: ✅ [N/N passing] or ❌ [failed details]
**Build**: ✅ Success or ❌ [error details]
**Rollback**: `git revert [commit-hash]` or [detailed steps]
**Dependencies**: [List of prerequisite steps] (status)
**Blockers**: [None or list of issues]
**Notes**: [Any important observations]
**Lessons Learned**: [Insights for future improvements] (optional)
```

**Complete Example:**
```markdown
🔵 **CHECKPOINT:** Service Interface Refactor

**Status**: 🔵 COMPLETE
**Complexity**: 🟡 Moderate
**Validated**: 2025-10-02T14:30:00Z
**Estimated**: 15 minutes
**Actual**: 30 minutes (+15 min variance)
**Tests**: ✅ 23/23 passing
**Build**: ✅ Success (0 errors, 1 warning: bundle size - acceptable)
**Rollback**: `git revert 7a3b9c2` or restore from `backup/service.ts.backup`
**Dependencies**: None
**Blockers**: None
**Notes**: Bundle size warning is acceptable - below 500KB threshold. All TypeScript errors resolved.
**Lessons Learned**: Type changes propagated more than expected. Should run `tsc --watch` during refactoring for faster feedback.
```

**Checkpoint Status Definitions:**
- 🔵 **COMPLETE**: All validation passed, ready to proceed
- 🟡 **IN_PROGRESS**: Implementation started, validation pending
- 🔴 **BLOCKED**: Validation failed or dependencies not met - DO NOT PROCEED

**Critical Rule:**
> **NEVER proceed past a checkpoint until status is 🔵 COMPLETE**

---

## Machine-Readable Checkpoint Format

For automation and AI assistant tracking, use this JSON-compatible format:

```json
{
  "workflow_name": "Service Interface Refactor",
  "started": "2025-10-02T14:00:00Z",
  "current_step": 2,
  "total_steps": 4,
  "overall_status": "IN_PROGRESS",
  "checkpoints": [
    {
      "step_number": 1,
      "name": "Simplify Service Interface",
      "complexity": "moderate",
      "status": "COMPLETE",
      "started": "2025-10-02T14:00:00Z",
      "completed": "2025-10-02T14:30:00Z",
      "estimated_minutes": 15,
      "actual_minutes": 30,
      "variance_minutes": 15,
      "validation": {
        "automated_tests": {"status": "PASS", "details": "23/23 passing"},
        "build": {"status": "PASS", "details": "0 errors"},
        "manual": {"status": "PASS", "details": "TypeScript compilation verified"}
      },
      "rollback": {
        "method": "git",
        "command": "git revert 7a3b9c2",
        "backup_location": "backup/service.ts.backup"
      },
      "files_modified": [
        "src/services/example-service.ts",
        "src/components/ComponentA.tsx",
        "src/components/ComponentB.tsx"
      ],
      "lessons_learned": [
        "TypeScript refactoring took longer than expected due to type propagation",
        "Should have run type checker incrementally during changes"
      ]
    },
    {
      "step_number": 2,
      "name": "Update Backend Integration",
      "complexity": "simple",
      "status": "IN_PROGRESS",
      "started": "2025-10-02T14:31:00Z",
      "estimated_minutes": 20,
      "dependencies": [1],
      "blockers": []
    }
  ]
}
```

**Usage**: Save as `workflow-state.json` for autonomous execution tracking

**Enhanced Fields (v1.1)**:
- `complexity`: "simple" | "moderate" | "complex" - Helps with planning and risk assessment
- `estimated_minutes`: Planned duration
- `actual_minutes`: Real duration (populated at completion)
- `variance_minutes`: Difference between estimated and actual
- `lessons_learned`: Array of insights for future improvements

**Complexity Guidelines**:
- **simple**: Single file, < 50 lines changed, no breaking changes
- **moderate**: Multiple files, refactoring, type changes, moderate risk
- **complex**: Architecture changes, database migrations, high risk, multiple dependencies

---

## Enhanced Validation Templates

### Template 1: Frontend Component Change

```markdown
🟢 **VALIDATE:**

**Build & Tests:**
- Run: `npm test -- ComponentName` (expect: all component tests pass)
- Run: `npm run build` (expect: successful build)
- Run: `npm run typecheck` (expect: 0 errors)

**Manual UI Testing:**
- Test: Component renders without errors in browser
- Test: All interactive elements respond correctly
- Test: Responsive design works (mobile, tablet, desktop)
- Test: Accessibility: keyboard navigation works
- Test: Accessibility: screen reader compatible

**Code Quality:**
- Check: No prop-types warnings
- Check: No React warnings in console
- Check: Proper TypeScript types (no `any`)
- Check: Component file < 150 lines
```

### Template 2: Backend/Edge Function Change

```markdown
🟢 **VALIDATE:**

**Deployment:**
- Run: `supabase functions deploy function-name` (expect: success)
- Check: Function logs show no errors

**API Testing:**
- Test: `curl -X POST [url] -d '[test-data]'` (expect: 200 OK)
- Test: Error handling - send invalid data (expect: 4xx with clear message)
- Test: Authentication - send without auth (expect: 401)
- Verify: Response structure matches expected schema

**Database:**
- Check: Data persisted correctly
- Check: RLS policies enforced
- Check: No orphaned records
- Query: `SELECT * FROM table WHERE id = 'test-id'` (verify data)

**Security:**
- Check: API keys not exposed in logs
- Check: Input validation working
- Check: SQL injection prevention in place
```

### Template 3: Database Migration

```markdown
🟢 **VALIDATE:**

**Migration Safety:**
- Run: `supabase db diff` (expect: only intended changes)
- Check: Migration is reversible (down migration exists)
- Verify: No data loss in migration
- Verify: Existing data migrates correctly

**Testing:**
- Run: `supabase db reset` (expect: clean reset works)
- Run: All migrations in sequence (expect: no errors)
- Query: Test queries work with new schema
- Check: RLS policies updated for new columns

**Performance:**
- Check: No missing indexes on new columns
- Check: Query performance acceptable
- Benchmark: Common queries < 100ms

**Rollback Test:**
- Run: Down migration (expect: clean rollback)
- Verify: System still functional after rollback
```

---

## Rollback Procedures

### Git-Based Rollback (Preferred)

**For Single Commit:**
```bash
# Identify commit to revert
git log --oneline -5

# Revert specific commit (creates new commit)
git revert [commit-hash]

# Verify rollback
npm test && npm run build
```

**For Multiple Commits:**
```bash
# Revert range of commits
git revert [oldest-commit]..[newest-commit]

# Or reset to specific point (WARNING: destructive)
git reset --hard [safe-commit-hash]
```

**Document in Checkpoint:**
```markdown
**Rollback**: `git revert 7a3b9c2` - restores service to original signature
```

---

### File-Based Rollback

**Create Backup Before Changes:**
```bash
# Backup single file
cp src/services/example-service.ts backup/example-service.ts.backup-2025-10-02

# Backup multiple files
tar -czf backup/checkpoint-1-2025-10-02.tar.gz src/services/ src/components/
```

**Restore from Backup:**
```bash
# Restore single file
cp backup/example-service.ts.backup-2025-10-02 src/services/example-service.ts

# Restore multiple files
tar -xzf backup/checkpoint-1-2025-10-02.tar.gz
```

**Document in Checkpoint:**
```markdown
**Rollback**:
1. `cp backup/example-service.ts.backup-2025-10-02 src/services/example-service.ts`
2. `npm install` (restore dependencies)
3. `npm test` (verify restoration)
```

---

### Database Migration Rollback

**Supabase Down Migration:**
```bash
# Rollback last migration
supabase migration down

# Verify database state
supabase db diff
```

**Manual SQL Rollback:**
```sql
-- Document exact rollback SQL
ALTER TABLE example_table DROP COLUMN IF EXISTS new_column_1;
ALTER TABLE example_table DROP COLUMN IF EXISTS new_column_2;
```

**Document in Checkpoint:**
```markdown
**Rollback**:
1. `supabase migration down` or
2. Execute SQL: `ALTER TABLE example_table DROP COLUMN new_column_1, new_column_2;`
3. Verify: `SELECT * FROM example_table LIMIT 1` (columns removed)
```

---

### Configuration Rollback

**Environment Variables:**
```bash
# Backup .env file
cp .env .env.backup-2025-10-02

# Restore
cp .env.backup-2025-10-02 .env
```

**Supabase Secrets:**
```bash
# Document previous value
echo "Previous API_KEY: [old-key]" > backup/secrets-2025-10-02.txt

# Restore
supabase secrets set API_KEY="[old-key]"
```

---

## Micro-Checkpoint Variant (v1.1)

**Purpose**: Streamlined format for trivial changes (typos, documentation, simple config)

**When to Use**:
- Single file changes
- < 10 lines modified
- No breaking changes
- No tests needed (documentation only)
- Low risk

**Format**:
```markdown
🔴🟢🔵 **Quick Fix**: [Description]
- **File**: [filename]
- **Change**: [what changed]
- **Verified**: [how you verified]
- **Rollback**: [git command or undo steps]
- **Time**: [actual minutes]

Example:
🔴🟢🔵 **Quick Fix**: Fixed typo in README.md
- **File**: README.md
- **Change**: "teh" → "the" in installation section
- **Verified**: `git diff` shows only README change, markdown syntax valid
- **Rollback**: `git revert abc123`
- **Time**: 2 minutes
```

**Rules**:
- ✅ Use for: Typos, doc updates, simple config changes
- ❌ Don't use for: Code changes, breaking changes, anything requiring tests
- Must still include verification and rollback
- Must justify why full checkpoint not needed

**Guard Rails**:
```markdown
🔴🟢🔵 **Quick Fix**: Update database connection string
- ⚠️ **Justification for Micro-Checkpoint**: Config only, no code changes, tested in dev
- **File**: .env
- **Change**: Updated DATABASE_URL to new endpoint
- **Verified**: App starts successfully, database connection test passes
- **Rollback**: `cp .env.backup .env`
- **Time**: 5 minutes
```

---

## Common Workflow Patterns

### Pattern 1: Single Feature Implementation

**Use Case**: Adding a new feature with isolated changes

```markdown
## Feature: Add Client Export to CSV

### Step 1: Implement Export Function
🔴 **IMPLEMENT:**
- Create exportToCSV utility function
- Add Export button to Clients page
- Wire up button to download CSV
- Files: src/utils/csv-export.ts (new), src/pages/Clients.tsx

🟢 **VALIDATE:**
- Run: `npm test -- csv-export` (expect: utility tests pass)
- Test: Click Export button → CSV downloads
- Verify: CSV contains all client data
- Verify: Special characters escaped correctly

🔵 **CHECKPOINT:** CSV Export Feature Complete
**Status**: 🔵 COMPLETE
**Rollback**: `git revert [hash]`
```

---

### Pattern 2: Multi-Step Refactor

**Use Case**: Large refactor requiring multiple coordinated changes

```markdown
## Refactor: Extract Shared Badge Utility

### Step 1: Extract Utility Function
🔴 **IMPLEMENT:**
- Create src/utils/badge-helpers.ts
- Add getStateBadgeClass(state: string): string
- Add comprehensive tests
- Files: src/utils/badge-helpers.ts (new), src/utils/badge-helpers.test.ts (new)

🟢 **VALIDATE:**
- Run: `npm test -- badge-helpers` (expect: 10/10 tests pass)
- Check: Full coverage (all states tested)

🔵 **CHECKPOINT:** Utility Extracted
**Status**: 🔵 COMPLETE
**Dependencies**: None
**Rollback**: `rm src/utils/badge-helpers.{ts,test.ts} && git checkout .`

---

### Step 2: Update First Component
🔴 **IMPLEMENT:**
- Update Dashboard.tsx to use getStateBadgeClass()
- Remove inline badge logic
- Verify imports work
- Files: src/pages/Dashboard.tsx

🟢 **VALIDATE:**
- Run: `npm test` (expect: all tests pass, no regressions)
- Test: Dashboard badges still render correctly
- Compare: Screenshots before/after (identical)

🔵 **CHECKPOINT:** Dashboard Migrated
**Status**: 🔵 COMPLETE
**Dependencies**: Step 1 (COMPLETE)
**Rollback**: `git revert [hash]` (restores inline logic)

---

### Step 3: Update Remaining Components
🔴 **IMPLEMENT:**
- Update Reports.tsx to use utility
- Update ComponentC.tsx to use utility
- Remove all inline badge logic
- Files: src/pages/Reports.tsx, src/components/ComponentC.tsx

🟢 **VALIDATE:**
- Run: `npm test` (expect: all pass)
- Run: `npm run build` (expect: success)
- Grep: `className={.*state.*}` in modified files (expect: 0 matches)
- Test: All badge displays correct in browser

🔵 **CHECKPOINT:** Refactor Complete - All Components Migrated
**Status**: 🔵 COMPLETE
**Dependencies**: Step 1, Step 2 (COMPLETE)
**Rollback**: `git revert [hash-step-3] [hash-step-2]`
**Notes**: Successfully eliminated 3 instances of duplicate code (DRY achieved)
```

---

### Pattern 3: Parallel Execution

**Use Case**: Independent tasks that can run simultaneously

```markdown
## Parallel Implementation: UI Polish + Backend Optimization

### Step 1A: Add Loading Spinners (Independent)
🔴 **IMPLEMENT:**
- Add LoadingSpinner component
- Add to Dashboard, Clients, Reports pages
- Files: src/components/LoadingSpinner.tsx (new), [3 pages]

🟢 **VALIDATE:**
- Run: `npm test -- LoadingSpinner`
- Test: Spinners appear during data fetching
- Verify: Spinners disappear when loaded

---

### Step 1B: Optimize Database Queries (Independent)
🔴 **IMPLEMENT:**
- Add indexes to example_table
- Optimize JOIN query in stats function
- Files: supabase/migrations/[timestamp]_add_indexes.sql, supabase/functions/stats/index.ts

🟢 **VALIDATE:**
- Run: `EXPLAIN ANALYZE [query]` (expect: uses index)
- Benchmark: Query time before/after (expect: >30% improvement)
- Test: Dashboard loads with real data

---

### Parallel Checkpoint
🔵 **CHECKPOINT:** UI + Backend Improvements Complete

**Step 1A Status**: 🔵 COMPLETE (UI loading states working)
**Step 1B Status**: 🔵 COMPLETE (Queries 40% faster)

**Combined Validation:**
- Run: `npm test` (expect: all pass)
- Test: Full application flow (expect: faster + better UX)

**Rollback**:
- 1A: `git revert [hash-ui]`
- 1B: `supabase migration down && git revert [hash-backend]`

**Notes**: Both tasks completed independently, no conflicts
```

---

### Pattern 4: Dependent Chain

**Use Case**: Steps that must execute in strict order

```markdown
## Multi-Tier Feature Implementation (Dependent Chain)

### Step 1: Add Database Columns (MUST BE FIRST)
🔴 **IMPLEMENT:**
- Add new columns to example_table
- Migration file with up/down
- Files: supabase/migrations/[timestamp]_add_columns.sql

🟢 **VALIDATE:**
- Run: `supabase db push`
- Query: `\d example_table` (expect: new columns present)
- Check: Existing data intact

🔵 **CHECKPOINT:** Database Schema Updated
**Status**: 🔵 COMPLETE
**Rollback**: `supabase migration down`
**BLOCK NEXT STEPS UNTIL COMPLETE** ⚠️

---

### Step 2: Update Frontend (DEPENDS ON STEP 1)
🔴 **IMPLEMENT:**
- Update service to use new database columns
- Files: src/services/example-service.ts, [3 components]

**Dependency Check**: ✅ Step 1 COMPLETE (database columns exist)

🟢 **VALIDATE:**
- Run: `npm test && npm run build`
- Check: TypeScript compiles with new signature

🔵 **CHECKPOINT:** Frontend Updated
**Status**: 🔵 COMPLETE
**Dependencies**: Step 1 ✅ COMPLETE
**Rollback**: `git revert [hash]`
**BLOCK NEXT STEPS UNTIL COMPLETE** ⚠️

---

### Step 3: Update Backend (DEPENDS ON STEPS 1 & 2)
🔴 **IMPLEMENT:**
- Update Edge Function to query new database columns
- Files: supabase/functions/example-function/index.ts

**Dependency Check**:
- ✅ Step 1 COMPLETE (columns exist in database)
- ✅ Step 2 COMPLETE (frontend uses new API)

🟢 **VALIDATE:**
- Deploy: `supabase functions deploy example-function`
- Test: Trigger function via UI (expect: success)
- Check: Database queried correctly (logs)

🔵 **CHECKPOINT:** Backend Database-Driven
**Status**: 🔵 COMPLETE
**Dependencies**: Steps 1, 2 ✅ COMPLETE
**Rollback**: Redeploy previous function version

---

### Final Integration Checkpoint
🔵 **CHECKPOINT:** Complete System Refactor

**All Steps Status**: 🔵 COMPLETE
**End-to-End Test**: ✅ Generate → Approve → Send → Email arrives
**Performance**: ✅ Within acceptable limits
**Rollback**: Multi-step (see individual checkpoint rollbacks above)
```

---

## Autonomous Execution Mode

**Use Case**: Long-running tasks executed by AI assistants with minimal human intervention

### Setup for Autonomous Execution

1. **Define Complete Workflow Upfront**
   - All steps documented
   - All validation criteria explicit
   - All rollback procedures ready
   - All dependencies mapped

2. **Set Human Intervention Triggers**
   ```markdown
   **Autonomous Execution Rules:**
   - Proceed automatically if: All GREEN validations pass
   - PAUSE for human if: Any validation fails after 2 retries
   - PAUSE for human if: Unexpected error occurs
   - PAUSE for human if: Checkpoint blocked by external dependency
   - REPORT progress: After each checkpoint completion
   ```

3. **Enable Progress Tracking**
   - Use `workflow-state.json` for machine-readable state
   - Update TodoWrite tool after each checkpoint
   - Log all validation results
   - Document all decisions made

### Autonomous Execution Example

```markdown
## Autonomous Task: Clean Up Console Logs

**Autonomous Mode**: ENABLED
**Human Intervention**: ON_FAILURE
**Progress Reporting**: PER_CHECKPOINT

---

### Step 1: Find All Console Logs
🔴 **IMPLEMENT (Autonomous):**
- Run: `grep -r "console.log" src/ --exclude-dir=node_modules`
- Document: All locations found
- Create: Cleanup plan

🟢 **VALIDATE (Autonomous):**
- Check: Grep results are complete
- Verify: No false positives

🔵 **CHECKPOINT:** Console Logs Identified
**Status**: 🔵 COMPLETE (Autonomous)
**Found**: 23 console.log statements across 8 files
**Next**: Proceed to Step 2

---

### Step 2: Remove Non-Production Logs
🔴 **IMPLEMENT (Autonomous):**
- Remove console.log from production code
- Keep debug logs in development-only blocks
- Update 8 files identified

🟢 **VALIDATE (Autonomous):**
- Run: `npm test` (expect: all pass)
- Run: `npm run build` (expect: success)
- Run: `grep -r "console.log" src/` (expect: 0 in production paths)

**VALIDATION RESULT**: ❌ FAILED (Tests failing: 2/23 failed)

**AUTONOMOUS ACTION**: PAUSE - Human intervention required
**REASON**: Validation failed - tests breaking
**REPORT**:
- Issue: Removed console.logs that tests were checking for
- Files affected: src/services/example-service.ts, src/utils/logger.ts
- Recommended fix: Update tests to not rely on console.logs
- Awaiting human decision: Fix tests or restore logs?

**Rollback Ready**: `git stash` (changes preserved for review)
```

### Autonomous Reporting Template

```markdown
## Autonomous Execution Report

**Task**: [Name]
**Started**: [Timestamp]
**Current Status**: [IN_PROGRESS / PAUSED / COMPLETE]

**Checkpoints Completed**:
- ✅ Step 1: [Name] (Completed: [timestamp])
- ✅ Step 2: [Name] (Completed: [timestamp])
- 🟡 Step 3: [Name] (In Progress)

**Current Step Details**:
- Phase: 🟢 VALIDATE
- Action: Running automated tests
- Expected: All tests pass
- Actual: [result]

**Decisions Made (Autonomous)**:
1. [Decision 1]: Chose [option] because [reason]
2. [Decision 2]: Skipped [step] because [criteria met]

**Human Intervention Required**:
- [None / Issue description]

**Next Steps**:
- If GREEN passes: Proceed to Step 4
- If GREEN fails: Pause and report to human
```

---

## AI Assistant Integration Guide

### How to Invoke Jimmy's Workflow

**User Says:** *"Let's use Jimmy's Workflow to execute this plan"*

**AI Assistant Should:**

1. **Acknowledge Workflow Activation**
   ```
   ✅ Activating Jimmy's Workflow (Red/Green Checkpoints)

   I will:
   - Execute in 🔴 RED → 🟢 GREEN → 🔵 CHECKPOINT phases
   - Validate explicitly at each step
   - Document rollback procedures
   - Report progress at each checkpoint
   - BLOCK progression if validation fails
   ```

2. **Break Down Task into Checkpointed Steps**
   ```markdown
   ## Workflow Plan: [Task Name]

   ### Step 1: [Name]
   🔴 IMPLEMENT: [What I will do]
   🟢 VALIDATE: [How I will verify]
   🔵 CHECKPOINT: [Success criteria]

   ### Step 2: [Name]
   ...

   **Total Steps**: [N]
   **Estimated Time**: [Duration]
   **Dependencies**: [List]
   ```

3. **Execute with Explicit Reporting**
   ```markdown
   ### Executing Step 1

   🔴 **IMPLEMENTING...**
   - Creating file X
   - Modifying file Y
   - Running command Z

   [Execute actual changes]

   🟢 **VALIDATING...**
   - Running: `npm test`
   - Result: ✅ 23/23 tests passing
   - Running: `npm run build`
   - Result: ✅ Build successful

   🔵 **CHECKPOINT REACHED**: Step 1 Complete
   **Status**: 🔵 COMPLETE
   **Rollback**: `git revert abc123`
   **Ready to proceed to Step 2**: ✅ YES
   ```

4. **Handle Validation Failures**
   ```markdown
   🟢 **VALIDATING...**
   - Running: `npm test`
   - Result: ❌ FAILED (3/23 tests failing)

   🔴 **CHECKPOINT BLOCKED**
   **Status**: 🔴 BLOCKED
   **Issue**: Tests failing in email service
   **Details**:
     - example-service.test.ts: Expected exampleId parameter, got undefined
     - Affected: 3 tests

   **Actions Taken**:
   - ❌ Did NOT proceed to next step
   - ✅ Preserved changes (git stash)
   - ✅ Documented failure details

   **Recommended Fix**:
   [Specific fix suggestion]

   **Awaiting**: Human decision or retry approval
   ```

### Progress Reporting Format

**After Each Checkpoint:**
```markdown
## Progress Update

**Workflow**: [Name]
**Completed**: Steps 1, 2, 3 (🔵 COMPLETE)
**Current**: Step 4 (🟡 IN_PROGRESS)
**Remaining**: Steps 5, 6
**Overall**: 50% complete (3 of 6 checkpoints)

**Latest Checkpoint**:
🔵 Step 3: Email Service Updated
- Validated: 2025-10-02 14:45:00
- Tests: ✅ 23/23 passing
- Build: ✅ Success
- Ready for Step 4: ✅ YES
```

### When to Ask for Human Input

**MUST Pause for Human If:**
1. ❌ Validation fails after 2 retry attempts
2. ❌ Unexpected error occurs (not in planned failure scenarios)
3. ❌ Breaking change detected that wasn't anticipated
4. ❌ Security risk identified
5. ❌ External dependency unavailable (API down, service offline)
6. ❌ Ambiguous requirement needs clarification

**Can Proceed Autonomously If:**
1. ✅ All validation passes
2. ✅ Expected errors handled by documented recovery
3. ✅ Dependencies met
4. ✅ No security concerns
5. ✅ Clear next step defined

---

## Failure Recovery Patterns

### Pattern 1: Test Failure Recovery

```markdown
**Scenario**: Tests fail during GREEN phase

**Recovery Steps**:

1. **Document Failure**
   ```markdown
   🟢 **VALIDATION FAILED**
   - Command: `npm test`
   - Expected: All tests pass
   - Actual: 3 tests failing
   - Files: example-service.test.ts (3 failures)
   - Error: "Expected exampleId, got undefined"
   ```

2. **Analyze Root Cause**
   - Check: Are tests outdated?
   - Check: Did implementation break existing functionality?
   - Check: Are test expectations wrong?

3. **Choose Recovery Path**

   **Path A: Fix Implementation**
   ```markdown
   🔴 **FIX IMPLEMENTATION**
   - Issue: Forgot to update test mocks
   - Action: Update mock data to include exampleId
   - Files: src/services/__mocks__/example-service.ts

   🟢 **RE-VALIDATE**
   - Run: `npm test`
   - Result: ✅ All pass now

   🔵 **CHECKPOINT UNBLOCKED**
   ```

   **Path B: Update Tests**
   ```markdown
   🔴 **FIX TESTS**
   - Issue: Tests expecting old API signature
   - Action: Update test expectations
   - Files: src/services/example-service.test.ts

   🟢 **RE-VALIDATE**
   - Run: `npm test`
   - Result: ✅ All pass now
   ```

   **Path C: Rollback**
   ```markdown
   🔴 **ROLLBACK IMPLEMENTATION**
   - Decision: Implementation approach was wrong
   - Action: `git revert [hash]`
   - Files: Restored to previous state

   🟢 **VALIDATE ROLLBACK**
   - Run: `npm test`
   - Result: ✅ Back to working state

   🔴 **CHECKPOINT RESET**
   **Status**: 🔴 BLOCKED
   **Next**: Re-plan implementation approach
   ```

4. **Retry Limit**
   - Max retries: 2 attempts
   - After 2 failures: PAUSE for human intervention
```

---

### Pattern 2: Build Failure Recovery

```markdown
**Scenario**: TypeScript compilation fails

**Recovery Steps**:

1. **Capture Error Details**
   ```markdown
   🟢 **VALIDATION FAILED**
   - Command: `npm run build`
   - Exit Code: 1
   - Error: TypeScript error in example-service.ts:42
   - Message: "Property 'content' does not exist on type 'EmailParams'"
   ```

2. **Quick Fix Attempt**
   ```markdown
   🔴 **APPLY FIX**
   - Issue: Removed property still referenced
   - Action: Update type definition
   - File: src/services/example-service.ts (line 42)

   🟢 **RE-VALIDATE**
   - Run: `npm run build`
   - Result: ✅ Build successful
   ```

3. **If Fix Fails**
   ```markdown
   **Retry 2**:
   🔴 **APPLY ALTERNATIVE FIX**
   - Action: Different approach to type safety

   🟢 **RE-VALIDATE**
   - Result: ❌ Still failing

   **Max Retries Reached**: PAUSE for human
   **Status**: 🔴 BLOCKED
   **Preserved**: Changes in git stash
   ```

---

### Pattern 3: Integration Failure Recovery

```markdown
**Scenario**: API call fails during manual validation

**Recovery Steps**:

1. **Document Integration Failure**
   ```markdown
   🟢 **VALIDATION FAILED** (Manual Test)
   - Test: Send example-item via UI
   - Expected: Action completed successfully
   - Actual: 500 Internal Server Error
   - Edge Function: send-example-item
   - Logs: "Cannot read property 'id' of null"
   ```

2. **Check Dependencies**
   ```markdown
   **Dependency Check**:
   - ✅ Database migration complete (Step 1)
   - ✅ Frontend updated (Step 2)
   - ❌ Edge Function deployed: Unknown (checking...)

   **Finding**: Edge Function not deployed!
   ```

3. **Fix Dependency**
   ```markdown
   🔴 **FIX MISSING DEPLOYMENT**
   - Action: `supabase functions deploy send-example-item`
   - Result: Deployed successfully

   🟢 **RE-VALIDATE**
   - Test: Send example-item via UI
   - Result: ✅ Action completed successfully

   🔵 **CHECKPOINT UNBLOCKED**
   ```

---

### Pattern 4: External Dependency Failure

```markdown
**Scenario**: External service unavailable (API down, database unreachable)

**Recovery Steps**:

1. **Identify External Issue**
   ```markdown
   🟢 **VALIDATION FAILED**
   - Test: Generate example-item with Claude AI
   - Error: "ConnectTimeout: Connection refused"
   - Service: Anthropic API
   - Status: Checking status.anthropic.com...
   ```

2. **Pause Execution**
   ```markdown
   🔴 **EXTERNAL BLOCKER**
   **Status**: 🔴 BLOCKED (External)
   **Issue**: Anthropic API unavailable
   **Type**: Temporary service outage

   **Actions**:
   - ❌ Cannot proceed with validation
   - ✅ Changes preserved (uncommitted)
   - ✅ Rollback available

   **Options**:
   1. Wait for service recovery (check status page)
   2. Test with fallback provider (if configured)
   3. Skip AI validation, proceed with manual review

   **Awaiting**: Human decision
   ```

3. **Resume After Resolution**
   ```markdown
   **Update**: Anthropic API restored

   🟢 **RE-VALIDATE**
   - Test: Generate example-item with Claude AI
   - Result: ✅ Digest generated successfully

   🔵 **CHECKPOINT UNBLOCKED**
   ```

---

## State Tracking

### Workflow State File Format

Save as `workflow-state.json` in project root:

```json
{
  "workflow": {
    "name": "Service Architecture Refactor",
    "description": "Decouple frontend from backend logic, make database-driven",
    "started": "2025-10-02T14:00:00Z",
    "updated": "2025-10-02T15:30:00Z",
    "status": "IN_PROGRESS",
    "current_step": 3,
    "total_steps": 4,
    "completion_percentage": 50
  },
  "checkpoints": [
    {
      "step": 1,
      "name": "Add Database Columns",
      "status": "COMPLETE",
      "phase": "CHECKPOINT",
      "started": "2025-10-02T14:00:00Z",
      "completed": "2025-10-02T14:20:00Z",
      "duration_minutes": 20,
      "validation": {
        "automated_tests": {
          "status": "PASS",
          "command": "supabase db push",
          "result": "Migration successful",
          "timestamp": "2025-10-02T14:18:00Z"
        },
        "manual_checks": {
          "status": "PASS",
          "checks": [
            "Columns exist in database ✅",
            "Existing data intact ✅"
          ]
        }
      },
      "rollback": {
        "method": "migration",
        "command": "supabase migration down",
        "backup": "supabase/migrations/[timestamp]_add_columns.sql"
      },
      "files_modified": [
        "supabase/migrations/20251002140500_add_columns.sql"
      ],
      "notes": "Migration reversible, no data loss"
    },
    {
      "step": 2,
      "name": "Simplify Frontend Service Interface",
      "status": "COMPLETE",
      "phase": "CHECKPOINT",
      "started": "2025-10-02T14:21:00Z",
      "completed": "2025-10-02T14:45:00Z",
      "duration_minutes": 24,
      "dependencies": [1],
      "validation": {
        "automated_tests": {
          "status": "PASS",
          "command": "npm test",
          "result": "23/23 tests passing",
          "timestamp": "2025-10-02T14:43:00Z"
        },
        "build": {
          "status": "PASS",
          "command": "npm run build",
          "result": "Build successful (0 errors, 1 warning)",
          "warnings": ["Bundle size 489KB - acceptable"]
        }
      },
      "rollback": {
        "method": "git",
        "command": "git revert 7a3b9c2",
        "backup": "backup/example-service-2025-10-02.tar.gz"
      },
      "files_modified": [
        "src/services/example-service.ts",
        "src/components/ComponentA.tsx",
        "src/components/ComponentC.tsx",
        "src/pages/Dashboard.tsx"
      ]
    },
    {
      "step": 3,
      "name": "Update send-example-item Edge Function",
      "status": "IN_PROGRESS",
      "phase": "VALIDATE",
      "started": "2025-10-02T14:46:00Z",
      "dependencies": [1, 2],
      "validation": {
        "deployment": {
          "status": "PASS",
          "command": "supabase functions deploy send-example-item",
          "result": "Deployed successfully",
          "timestamp": "2025-10-02T15:10:00Z"
        },
        "api_test": {
          "status": "IN_PROGRESS",
          "command": "curl -X POST [url]",
          "expected": "200 OK with email sent"
        }
      },
      "files_modified": [
        "supabase/functions/send-example-item/index.ts"
      ]
    },
    {
      "step": 4,
      "name": "End-to-End Integration Test",
      "status": "PENDING",
      "phase": "NOT_STARTED",
      "dependencies": [1, 2, 3]
    }
  ],
  "metrics": {
    "checkpoints_complete": 2,
    "checkpoints_in_progress": 1,
    "checkpoints_pending": 1,
    "total_duration_minutes": 44,
    "validation_failures": 0,
    "rollbacks_performed": 0
  }
}
```

### State Tracking Commands

**Update Status:**
```bash
# Mark checkpoint complete
jq '.checkpoints[2].status = "COMPLETE"' workflow-state.json > tmp.json && mv tmp.json workflow-state.json

# Update current step
jq '.workflow.current_step = 4' workflow-state.json > tmp.json && mv tmp.json workflow-state.json
```

**Query Status:**
```bash
# Get current step
jq '.workflow.current_step' workflow-state.json

# Get completion percentage
jq '.workflow.completion_percentage' workflow-state.json

# List incomplete checkpoints
jq '.checkpoints[] | select(.status != "COMPLETE") | .name' workflow-state.json
```

---

## Quality Gates

Jimmy's Workflow integrates with TDD quality gates:

### TDD Cycle Mapping

```
┌─────────────────────────────────────┐
│  TRADITIONAL TDD                    │
├─────────────────────────────────────┤
│  🔴 RED: Write failing test         │
│  🟢 GREEN: Make test pass           │
│  🔵 REFACTOR: Improve code          │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│  JIMMY'S WORKFLOW                   │
├─────────────────────────────────────┤
│  🔴 RED: IMPLEMENT feature          │
│  🟢 GREEN: VALIDATE (run tests)     │
│  🔵 CHECKPOINT: Lock in progress    │
└─────────────────────────────────────┘
```

### Combined TDD + Checkpoint Workflow

```markdown
## Feature: Add CSV Export

### TDD Phase 1: Write Tests
🔴 **IMPLEMENT (TDD RED):**
- Write test: `should export clients to CSV`
- Write test: `should handle empty client list`
- Write test: `should escape special characters`
- Run tests: EXPECT FAILURES (not implemented yet)

🟢 **VALIDATE (TDD RED):**
- Run: `npm test -- csv-export`
- Result: ✅ Tests fail as expected (RED achieved)

🔵 **CHECKPOINT:** Tests Written
**TDD Phase**: RED ✅
**Next**: Implement feature to make tests pass

---

### TDD Phase 2: Make Tests Pass
🔴 **IMPLEMENT (TDD GREEN):**
- Create exportToCSV() function
- Add minimal implementation
- Run tests after each change

🟢 **VALIDATE (TDD GREEN):**
- Run: `npm test -- csv-export`
- Result: ✅ All 3 tests passing (GREEN achieved)

🔵 **CHECKPOINT:** Feature Implemented
**TDD Phase**: GREEN ✅
**Next**: Refactor if needed

---

### TDD Phase 3: Refactor
🔴 **IMPLEMENT (TDD REFACTOR):**
- Extract CSV header generation
- Simplify escape logic
- Add TypeScript types

🟢 **VALIDATE (TDD REFACTOR):**
- Run: `npm test -- csv-export`
- Result: ✅ Still passing (GREEN maintained)
- Check: Code cleaner, more maintainable

🔵 **CHECKPOINT:** Feature Complete & Refactored
**TDD Cycle**: Complete ✅
**Quality**: High
```

---

## Integration with TodoWrite

Jimmy's Workflow maps naturally to the TodoWrite tool for progress tracking:

### Mapping Checkpoints to Todos

```markdown
**Before Workflow Starts:**

TodoWrite:
- [ ] Add multi-language support (4 steps planned)

**After Planning:**

TodoWrite:
- [ ] Step 1: Add translation files
- [ ] Step 2: Configure i18next
- [ ] Step 3: Update components to use translations
- [ ] Step 4: Test in browser (EN + SV)

**During Execution (Step 1):**

TodoWrite:
- [IN_PROGRESS] Step 1: Add translation files ← Currently here
- [PENDING] Step 2: Configure i18next
- [PENDING] Step 3: Update components
- [PENDING] Step 4: Test in browser

**After Step 1 Checkpoint:**

TodoWrite:
- [COMPLETE] Step 1: Add translation files ✅
- [IN_PROGRESS] Step 2: Configure i18next ← Now here
- [PENDING] Step 3: Update components
- [PENDING] Step 4: Test in browser
```

### TodoWrite Update Pattern

**At Each Checkpoint:**
```typescript
// Mark current step complete
todoWrite.update(currentStep, 'COMPLETE')

// Mark next step in_progress
todoWrite.update(nextStep, 'IN_PROGRESS')
```

**On Checkpoint Failure:**
```typescript
// Keep current step in_progress
// Add new todo for fix if needed
todoWrite.add('Fix failing tests in Step 2', 'IN_PROGRESS')
```

---

## Quick Reference

### Commands Cheat Sheet

```bash
# Validation Commands
npm test                    # Run all tests
npm test -- ComponentName   # Run specific tests
npm run build              # Build for production
npm run typecheck          # TypeScript check only
npm run lint               # ESLint check
npm audit                  # Security vulnerabilities

# Rollback Commands
git revert [hash]          # Revert specific commit
git stash                  # Preserve uncommitted changes
git checkout [file]        # Restore single file
git reset --hard [hash]    # Hard reset (DESTRUCTIVE)

# Supabase Commands
supabase functions deploy [name]     # Deploy Edge Function
supabase functions logs [name]       # View logs
supabase db push                     # Run migrations
supabase migration down              # Rollback migration
supabase db diff                     # Check schema changes

# Search Commands
grep -r "pattern" src/              # Find in source
grep -r "console.log" src/          # Find console logs
grep -c "pattern" [file]            # Count matches
```

### Checkpoint Status Icons

- 🔵 **COMPLETE**: All green, proceed
- 🟡 **IN_PROGRESS**: Currently executing
- 🔴 **BLOCKED**: Validation failed, DO NOT PROCEED
- ⚠️ **DEPENDENCY**: Waiting for prerequisite
- 🔄 **RETRYING**: Attempting fix (count: 1/2)

### Validation Checklist Template

```markdown
🟢 **VALIDATE:**
- [ ] Run: `npm test` (all pass)
- [ ] Run: `npm run build` (success)
- [ ] Run: `npm run typecheck` (0 errors)
- [ ] Check: No console.log in code
- [ ] Check: No security issues
- [ ] Test: [Manual scenario] (works)
- [ ] Verify: Database state correct
```

### Rollback Template

```markdown
**Rollback**:
1. `git revert [hash]` OR `cp backup/[file] src/`
2. `npm install` (if dependencies changed)
3. `npm test && npm run build` (verify restoration)
4. Redeploy if needed: `supabase functions deploy [name]`
```

---

## Usage Summary

**To invoke this workflow:**
> "Let's use Jimmy's Workflow to execute this plan"

**Core pattern:**
```
🔴 IMPLEMENT → 🟢 VALIDATE → 🔵 CHECKPOINT
```

**Critical rules:**
1. NEVER skip validation
2. NEVER proceed without GREEN passing
3. ALWAYS document rollback
4. ALWAYS use explicit commands

**Benefits:**
- ✅ Prevents AI hallucination
- ✅ Forces validation at every step
- ✅ Enables autonomous execution
- ✅ Provides safety nets (rollback)
- ✅ Integrates with TDD seamlessly

---

## Version Check (For Projects That Copy This File)

**If you COPIED this file to your project** (not referencing from platform root):

```bash
# Check if your copy is up to date
~/templates/tools/check-version.sh
```

**If out of date:**
- Your copy: v1.0 (example)
- Master version: v1.1
- What's new: See ~/templates/CHANGELOG.md
- To update: Replace this file with latest from ~/templates/JIMMYS-WORKFLOW.md

**If you REFERENCE this file** (e.g., `../JIMMYS-WORKFLOW.md`):
- ✅ Always up to date automatically
- No sync needed

---

**Document Version**: 1.1
**Last Updated**: 2025-01-04
**Status**: Active Platform Standard
**Maintained By**: Jimmy + AI Coding Assistants

