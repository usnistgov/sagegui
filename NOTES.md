# SageGUI — Development Notes

This document tracks decisions, progress, and open questions across all phases.
Read this alongside `PLAN.md` to understand current project state.

---

## Current Status

**Active Phase:** Phase 0 — Bug Fixes & Organization  
**Last Updated:** 2026-07-10  
**Next Action:** Complete documentation, then start Phase 1 (Fork Sage)

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

### API Analysis (for Phase 1-2)

Analyzed differences between Sebastian's Sage fork and official Sage v0.14.7:

| Component | Sebastian's Fork | Official v0.14.7 | Impact |
|-----------|-----------------|------------------|--------|
| `sage-cli` | Has `lib.rs` with exports | No lib target | Must add lib.rs |
| `Kind` enum | Implements `Hash`/`Eq` | Does not | Must add derives or change data structure |
| `BrukerSpectrumProcessor` | Old name | Now `BrukerProcessingConfig` | Simple rename |
| `Builder` struct | Old fields | New fields (`fragment_min_mz`, etc.) | Add new fields |
| `variable_mods` | `Vec<(String, f32)>` | `HashMap<String, Vec<f32>>` | Type change |
| `timsrust` | Old API | New API | May need updates |

### Checkpoint Status
- [x] TMT bug fixed
- [x] Fragment tolerance bug fixed
- [x] Pushed to neely/sagegui
- [x] CONTEXT.md created
- [x] PLAN.md created
- [x] NOTES.md created
- [x] GLOSSARY.md created
- [ ] README.md updated (deferred to Phase 5)

### Open Questions
- None for Phase 0

---

## Phase 1 — Fork Sage & Add Lib Exports

**Status:** Not Started  
**Planned Start:** After Phase 0 complete

### Pre-Work Completed
- Analyzed official Sage v0.14.7 source code
- Identified required lib exports
- Documented API differences

### Tasks
1. [ ] Fork `lazear/sage` to `neely/sage`
2. [ ] Create branch `gui-lib-exports`
3. [ ] Add `crates/sage-cli/src/lib.rs`
4. [ ] Modify `crates/sage-cli/Cargo.toml`
5. [ ] Test `cargo check`
6. [ ] Tag release `v0.14.7-gui`
7. [ ] Update sagegui Cargo.toml

---

## Phase 2 — Fix API Compatibility Issues

**Status:** Not Started

### Known Issues (from Phase 0 analysis)

1. **`Kind` not hashable**
   - Location: `main.rs:110`
   - Current: `HashMap<Kind, bool>`
   - Fix options:
     - A) Add `#[derive(Hash, Eq)]` to `Kind` in our Sage fork
     - B) Change to `Vec<(Kind, bool)>` in GUI

2. **`BrukerSpectrumProcessor` renamed**
   - Location: `main.rs:27`
   - Fix: Update import to `BrukerProcessingConfig`

3. **`Builder` missing fields**
   - Location: `main.rs:171`
   - Fix: Add `fragment_min_mz`, `fragment_max_mz` fields

4. **`variable_mods` type change**
   - Location: `main.rs:183`
   - Fix: Update to `HashMap<String, Vec<f32>>`

5. **`ScoreType` location**
   - Location: `main.rs:19`
   - Fix: Find new import path or remove

---

## Reference Information

### Sebastian's Sage Fork
- Repo: `github.com/jspaezp/sage`
- Key modification: Added `lib.rs` to `sage-cli`
- Last updated: ~2 years ago (stuck on old Sage version)

### Official Sage
- Repo: `github.com/lazear/sage`
- Current version: v0.14.7
- Key crates: `sage-core`, `sage-cli`, `sage-cloudpath`

### Our Forks
- sagegui: `github.com/neely/sagegui`
- sage (to be created): `github.com/neely/sage`

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

**Next Steps:**
1. Fork lazear/sage to neely/sage
2. Add lib exports
3. Update sagegui to use our fork
4. Fix compilation errors
