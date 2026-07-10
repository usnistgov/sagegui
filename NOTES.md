# SageGUI — Development Notes

This document tracks decisions, progress, and open questions across all phases.
Read this alongside `PLAN.md` to understand current project state.

---

## Current Status

**Active Phase:** Phase 3 — CI/CD & Release  
**Last Updated:** 2026-07-10  
**Next Action:** Verify CI builds, create release v0.6.0

---

## Phase 0 — Bug Fixes & Organization

**Status:** ✅ Complete  
**Started:** 2026-07-10  
**Completed:** 2026-07-10

### Goals
- Fix known bugs in Sebastian's GUI
- Set up project documentation structure
- Push fixes to our fork

### Bug Fixes Applied

#### 1. TMT Plex Selection Bug (Lines 421-423)

**Problem:** TMT 16-plex and 18-plex were incorrectly mapped to `Tmt11`.

**Before:**
```rust
"TMT-16" => Isobaric::Tmt11,
"TMT-18" => Isobaric::Tmt11,
```

**After:**
```rust
"TMT-16" => Isobaric::Tmt16,
"TMT-18" => Isobaric::Tmt18,
```

**Commit:** a225481

#### 2. Fragment Tolerance Type Bug (Lines 720-726)

**Problem:** When switching fragment tolerance type (ppm ↔ Da), the code was updating `precursor_tol` instead of `fragment_tol`.

**Before:**
```rust
if ui.selectable_label(self.fragment_tol_unit == "ppm", "ppm").clicked() {
    self.fragment_tol_unit = "ppm".to_string();
    self.precursor_tol = (-self.fragment_tol_value, self.fragment_tol_value);  // WRONG!
}
```

**After:**
```rust
if ui.selectable_label(self.fragment_tol_unit == "ppm", "ppm").clicked() {
    self.fragment_tol_unit = "ppm".to_string();
    self.fragment_tol = (-self.fragment_tol_value, self.fragment_tol_value);  // CORRECT
}
```

**Commit:** a225481

### Documentation Created

| File | Purpose |
|------|---------|
| `CONTEXT.md` | Background knowledge for working on this project |
| `PLAN.md` | Architecture, phases, requirements |
| `NOTES.md` | This file — progress log, decisions |
| `docs/GLOSSARY.md` | Term definitions |

### Decisions Made

1. **Option A chosen** — Fork Sage and maintain lib exports, rather than wrapper approach (Option C).

2. **Documentation structure** — Modeled after sagePreview project with CONTEXT.md, PLAN.md, NOTES.md, GLOSSARY.md.

3. **Keep Sebastian's structure** — Single `main.rs` file, egui/eframe framework.

---

## Phase 1 — Fork Sage & Update to v0.15.0-beta.2

**Status:** ✅ Complete  
**Started:** 2026-07-10  
**Completed:** 2026-07-10

### Goals
- Fork `lazear/sage` to `neely/sage`
- Update sagegui to use our fork with latest Sage version
- Fix all API compatibility issues

### Key Discovery

**Good news:** The official Sage v0.15.0-beta.2 already has `lib.rs` in `sage-cli`! We don't need to add it ourselves. The original plan assumed we'd need to create this file, but it already exports:
- `input`
- `output`
- `runner`
- `telemetry`

### API Changes Fixed

The jump from v0.14.7 to v0.15.0-beta.2 required these fixes:

| Issue | Location | Fix Applied |
|-------|----------|-------------|
| `restrict` type | `EnzymeBuilder` | Changed from `Option<char>` to `Option<String>` via `.map(\|c\| c.to_string())` |
| `Builder` missing fields | `DatabaseConfig → Builder` | Added `prefilter: None`, `prefilter_chunk_size: None`, `prefilter_low_memory: None` |
| `LfqOptions` missing fields | `QuantType → QuantOptions` | Added `mobility_pct_tolerance: None`, `peptide_q_value: None` |
| `Input` field renamed | `Config → Input` | Changed `bruker_spectrum_processor` → `bruker_config: None` |
| `Input` new fields | `Config → Input` | Added `protein_grouping: None`, `protein_grouping_peptide_fdr: None`, `write_report: None` |
| `Runner::new` signature | `run_sage()` | Changed from `input.build().and_then(Runner::new)` to `let search = input.build()?; Runner::new(search, parallel.into())` |

### Improvements Made

1. **Pinned to specific commit** — Changed from `branch = "master"` to `rev = "d74024df..."` for reproducibility
2. **Added version display** — GUI now shows "Sage Engine Version: v0.15.0-beta.2" in Info/Help
3. **Updated repository links** — Changed from jspaezp to neely
4. **Created CHANGELOG.md** — Tracks all changes for release notes

### Checkpoint Status
- [x] Fork lazear/sage to neely/sage
- [x] Clone fork locally
- [x] Discovered lib.rs already exists (no modifications needed!)
- [x] Update sagegui Cargo.toml to use our fork
- [x] Fix API compatibility issues
- [x] `cargo check` passes
- [x] GUI launches successfully

---

## Phase 2 — Test & Validate

**Status:** ✅ Complete  
**Started:** 2026-07-10  
**Completed:** 2026-07-10

### Test Cases
1. [x] Load mzML files
2. [x] Load FASTA database
3. [x] Configure search parameters
4. [x] Run search
5. [x] View results summary
6. [ ] TMT quantification (all plex sizes) — not tested yet
7. [x] LFQ quantification

### Test Results

**Test Data:**
- mzML: `B.naive_01steady-state.mzML.gz` (from sagePreview testing)
- FASTA: `UniProt-Human-UP000005640_canonical-2023_05.fasta`

**Search Parameters:**
- Precursor tolerance: ±10 ppm
- Fragment tolerance: ±10 ppm
- Enzyme: Trypsin (KR, not P), 2 missed cleavages
- Static mods: C+57.021 (carbamidomethyl)
- Variable mods: M+15.995 (oxidation)
- LFQ enabled

**Results:**
- **60,672 PSMs** identified
- **LFQ quantification** working (lfq.tsv generated)
- Output files: `test/results.sage.tsv`, `test/lfq.tsv`, `test/results.json`

### Notes
- GUI launches successfully ✅
- File selection works ✅
- Search execution works ✅
- LFQ quantification works ✅
- TMT not tested (would need TMT-labeled data)

---

## Reference Information

### Sage Versions

| Version | Status | Notes |
|---------|--------|-------|
| v0.14.7 | Old (Sebastian's) | What the original GUI used |
| v0.15.0-beta.2 | Current | What we're using now (commit d74024df) |

### Our Forks
- sagegui: `github.com/neely/sagegui`
- sage: `github.com/neely/sage` (forked from lazear/sage)

### Key Files in Sage

| File | Purpose |
|------|---------|
| `crates/sage-cli/src/lib.rs` | Exports input, output, runner, telemetry |
| `crates/sage-cli/src/input.rs` | `Input`, `LfqOptions`, `QuantOptions`, etc. |
| `crates/sage-cli/src/runner.rs` | `Runner::new()`, `Runner::run()` |
| `crates/sage/src/database.rs` | `Builder`, `EnzymeBuilder` |

---

## Session Log

### 2026-07-10 Session 1

**Accomplished:**
1. Cloned and analyzed Sebastian's sagegui
2. Identified and fixed two bugs (TMT plex, fragment tolerance)
3. Pushed fixes to neely/sagegui
4. Analyzed API differences between Sebastian's Sage fork and official v0.14.7
5. Decided on Option A (fork Sage) approach
6. Created project documentation (CONTEXT.md, PLAN.md, NOTES.md, GLOSSARY.md)

**Key Decisions:**
- Option A over Option C (user preference)
- Will fork official Sage and add lib exports
- Accept maintenance burden of keeping fork in sync

### 2026-07-10 Session 2

**Accomplished:**
1. Forked lazear/sage to neely/sage
2. Discovered lib.rs already exists in v0.15.0-beta.2 (pleasant surprise!)
3. Updated sagegui Cargo.toml to use neely/sage
4. Fixed 6 API compatibility issues
5. Pinned dependency to specific commit hash
6. Added Sage version display in GUI
7. Created CHANGELOG.md
8. Updated PLAN.md and NOTES.md with accurate information
9. Confirmed GUI launches successfully

**Key Learnings:**
- The plan predicted we'd need to add lib.rs, but it already existed
- The plan mentioned v0.14.7, but we used v0.15.0-beta.2 (current master)
- API changes were different from what was predicted in the plan
- Pinning to commit hash is better than branch for reproducibility

**Next Steps:**
1. Commit and push all changes
2. Test with real data (Phase 2)
3. Set up CI/CD (Phase 3)
