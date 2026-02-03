# Phase Reconciliation Amendment - Execution Plan

**Date:** 2026-02-03
**Target:** docs/03-IMPLEMENTATION-PLAN.md v2.3.0 → v2.4.0
**Author:** Q

---

## Pre-Flight Checklist

- [x] Read full document structure (2,226 lines)
- [x] Identified all amendment locations
- [x] Checked 06-CHAOS-TESTING-STRATEGY.md - no conflicting phase refs found
- [x] Documented execution plan (this file)

---

## Amendment Locations

| Amendment | Location | Action |
|-----------|----------|--------|
| A1a | Lines 244-271 | Replace ASCII diagram |
| A1b | Lines 276-284 | Replace dependencies table |
| A2 | After line 1233 | Insert Phase 3 scope note |
| A3 | Lines 1546-1696 | Replace entire Phase 5 section |
| A4 | Line 1698 | Update Phase 6 header (content OK) |
| A5 | After line 1812 | Insert new Phase 7 section |
| A6a | Line 3 | Update version to 2.4.0 |
| A6b | After line 2226 | Add changelog entry |
| TOC | Lines 18-21 | Update section numbers |
| Summary | Lines 2200-2206 | Update summary table |

---

## Execution Order

1. **A6a** - Version bump (line 3)
2. **TOC** - Table of contents (lines 18-21)
3. **A1a** - ASCII diagram (lines 244-271)
4. **A1b** - Dependencies table (lines 276-284)
5. **A2** - Phase 3 scope note (after line 1233)
6. **A3** - Phase 5 section replacement (lines 1546-1696)
7. **A4** - Phase 6 header update (line 1698)
8. **A5** - Phase 7 new section (after current Phase 6)
9. **Summary** - Summary table (lines 2200-2206)
10. **A6b** - Changelog entry (end of file)

---

## Validation Checklist

After all amendments:

- [ ] Version shows 2.4.0
- [ ] TOC has Phase 5 = IrohTransport, Phase 6 = sync-relay, Phase 7 = tauri-plugin
- [ ] ASCII diagram shows correct flow
- [ ] Dependencies table updated
- [ ] Phase 3 scope note present
- [ ] Phase 5 section is IrohTransport (not Tauri)
- [ ] Phase 6 header says "sync-relay + Full Topology Chaos"
- [ ] Phase 7 section exists (Tauri)
- [ ] Summary table updated
- [ ] Changelog has v2.4.0 entry
- [ ] No broken markdown (run `cat` test)

---

## Rollback

If amendments fail validation:
```bash
git checkout docs/03-IMPLEMENTATION-PLAN.md
```

---

*Execution begins after this plan is reviewed.*
