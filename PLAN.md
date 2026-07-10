# SageGUI — Development Plan

**Goal:** A maintainable GUI for Sage that can stay up-to-date with official Sage releases.

**Approach:** Fork Sage, add library exports, maintain sync with upstream (Option A).

---

## Mission

Provide a user-friendly graphical interface for Sage that:
1. Works with the latest Sage releases
2. Builds for Windows, macOS, and Linux
3. Exposes all important Sage parameters
4. Can be maintained long-term without excessive effort

---

## Non-Goals (Do NOT Build)

- Not a full proteomics pipeline (just search configuration and execution)
- Not a results viewer beyond basic summary (use downstream tools)
- Not a parameter optimization engine
- Not a batch processing system (one search at a time)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        SageGUI                               │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    egui/eframe                          ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐ ││
│  │  │  Files   │ │  Search  │ │  Quant   │ │  Results   │ ││
│  │  │  Panel   │ │  Params  │ │  Params  │ │  Summary   │ ││
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────┘ ││
│  └─────────────────────────────────────────────────────────┘│
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              sage-core / sage-cli (our fork)            ││
│  │  - Input struct (search parameters)                     ││
│  │  - Runner (search execution)                            ││
│  │  - Output (results)                                     ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

**Key dependency:** Our fork of Sage at `github.com/neely/sage` (v0.15.0-beta.2, commit d74024df).

---

## Phases

### Phase 0 — Bug Fixes & Organization ✅ Complete

**Goals:**
- Fix known bugs in Sebastian's GUI
- Set up project documentation structure
- Push fixes to our fork

**Completed:**
- [x] Fixed TMT 16/18-plex selection bug
- [x] Fixed fragment tolerance type switching bug
- [x] Pushed to `neely/sagegui`
- [x] Created CONTEXT.md, GLOSSARY.md, PLAN.md, NOTES.md

---

### Phase 1 — Fork Sage & Update to v0.15.0-beta.2 ✅ Complete

**Goals:**
- Fork `lazear/sage` to `neely/sage`
- Update sagegui to use our fork with latest Sage version
- Fix all API compatibility issues

**Completed:**
- [x] Forked `lazear/sage` to `neely/sage`
- [x] Discovered `lib.rs` already exists in v0.15.0-beta.2 (no modifications needed!)
- [x] Updated `sagegui/Cargo.toml` to use our fork
- [x] Fixed API compatibility issues (see below)
- [x] Pinned to specific commit hash for reproducibility
- [x] Added Sage version display in GUI
- [x] Created CHANGELOG.md

**API Changes Fixed:**

| Issue | Fix Applied |
|-------|-------------|
| `restrict` type changed | `Option<char>` → `Option<String>` via `.map(\|c\| c.to_string())` |
| `Builder` missing fields | Added `prefilter: None`, `prefilter_chunk_size: None`, `prefilter_low_memory: None` |
| `LfqOptions` missing fields | Added `mobility_pct_tolerance: None`, `peptide_q_value: None` |
| `Input` field renamed | Changed `bruker_spectrum_processor` → `bruker_config: None` |
| `Input` new fields | Added `protein_grouping: None`, `protein_grouping_peptide_fdr: None`, `write_report: None` |
| `Runner::new` signature | Changed from `Runner::new(search)` to `Runner::new(search, parallel)` |

**Checkpoint:** ✅ `cargo check` passes, GUI launches successfully.

---

### Phase 2 — Test & Validate ✅ Complete

**Goals:**
- Run the GUI and verify all features work
- Test on real data
- Fix any runtime issues

**Test Cases:**
1. [x] Load mzML files
2. [x] Load FASTA database
3. [x] Configure search parameters
4. [x] Run search
5. [x] View results summary
6. [ ] TMT quantification (all plex sizes) — not tested (needs TMT data)
7. [x] LFQ quantification

**Test Results:**
- **60,672 PSMs** identified from single mzML file
- **LFQ quantification** working correctly
- Output files generated: `results.sage.tsv`, `lfq.tsv`, `results.json`

**Checkpoint:** ✅ Core functionality verified. TMT testing deferred.

---

### Phase 3 — CI/CD & Release

**Goals:**
- Verify GitHub Actions builds work
- Create first release from our fork
- Document release process

**Tasks:**
1. [ ] Push all changes
2. [ ] Verify CI builds pass on all platforms
3. [ ] Create tag `v0.6.0`
4. [ ] Verify release artifacts are created
5. [ ] Document how to update Sage version in future

**Checkpoint:** Release `v0.6.0` available with binaries for Windows, macOS, Linux.

---

### Phase 4 — Documentation & Handoff

**Goals:**
- Update README with installation instructions
- Document maintenance process
- Create "How to update Sage" guide

**Deliverables:**
- Updated README.md
- MAINTENANCE.md (how to sync with upstream Sage)
- Release notes

---

## Future Phases (Not Planned Yet)

- **Phase 5:** UI improvements (better layout, themes)
- **Phase 6:** Additional features (batch processing, presets)
- **Phase 7:** Expose new Sage v0.15 features (prefilter, protein grouping, write_report)
- **Phase 8:** Consider Option C (wrapper) if maintenance burden too high

---

## Maintenance Commitment

When Sage releases a new version:

1. **Merge upstream changes** into `neely/sage`:
   ```bash
   git fetch upstream
   git merge upstream/main
   ```

2. **Update sagegui Cargo.toml** to new commit hash

3. **Fix any API changes** (check for new/changed fields in Input, Builder, etc.)

4. **Test and release** new sagegui version

**Estimated effort:** 1-2 hours per Sage release (assuming no major API changes).

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-10 | Option A (fork Sage) over Option C (wrapper) | User preference for embedded approach despite maintenance burden |
| 2026-07-10 | Keep egui/eframe | Already working, good Rust GUI choice |
| 2026-07-10 | Single main.rs file | Keep Sebastian's structure for now |
| 2026-07-10 | Target v0.15.0-beta.2 instead of v0.14.7 | Latest version already has lib.rs, fewer modifications needed |
| 2026-07-10 | Pin to commit hash | Prevents unexpected breakage from upstream changes |
