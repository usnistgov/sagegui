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

**Key dependency:** Our fork of Sage at `github.com/neely/sage` with lib exports.

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

### Phase 1 — Fork Sage & Add Lib Exports

**Goals:**
- Fork `lazear/sage` to `neely/sage`
- Add `lib.rs` to `sage-cli` crate to expose internals
- Tag a release (e.g., `v0.14.7-gui`)
- Update sagegui to use our fork

**Tasks:**
1. Fork `lazear/sage` on GitHub
2. Create branch `gui-lib-exports`
3. Add `crates/sage-cli/src/lib.rs`:
   ```rust
   pub mod input;
   pub mod output;
   pub mod runner;
   pub mod telemetry;
   ```
4. Modify `crates/sage-cli/Cargo.toml` to include lib target
5. Test that `cargo check` passes
6. Tag release `v0.14.7-gui`
7. Update `sagegui/Cargo.toml` to point to our fork

**Checkpoint:** `cargo check` passes on sagegui with our Sage fork.

---

### Phase 2 — Fix API Compatibility Issues

**Goals:**
- Resolve all compilation errors from API changes
- Update GUI code to work with official Sage v0.14.7 types

**Known Issues to Fix:**

| Issue | Location | Fix |
|-------|----------|-----|
| `Kind` not hashable | `main.rs:110` | Replace `HashMap<Kind, bool>` with `Vec<(Kind, bool)>` or add derives to our fork |
| `BrukerSpectrumProcessor` renamed | `main.rs:27` | Update to `BrukerProcessingConfig` |
| `Builder` missing fields | `main.rs:171` | Add `fragment_min_mz`, `fragment_max_mz` |
| `variable_mods` type change | `main.rs:183` | Update to `HashMap<String, Vec<f32>>` |
| `ScoreType` moved | `main.rs:19` | Find new location or remove |

**Checkpoint:** `cargo build --release` succeeds.

---

### Phase 3 — Test & Validate

**Goals:**
- Run the GUI and verify all features work
- Test on real data
- Fix any runtime issues

**Test Cases:**
1. Load mzML files
2. Load FASTA database
3. Configure search parameters
4. Run search
5. View results summary
6. TMT quantification (all plex sizes)
7. LFQ quantification

**Checkpoint:** All test cases pass.

---

### Phase 4 — CI/CD & Release

**Goals:**
- Verify GitHub Actions builds work
- Create first release from our fork
- Document release process

**Tasks:**
1. Push all changes
2. Verify CI builds pass on all platforms
3. Create tag `v0.5.1`
4. Verify release artifacts are created
5. Document how to update Sage version in future

**Checkpoint:** Release `v0.5.1` available with binaries for Windows, macOS, Linux.

---

### Phase 5 — Documentation & Handoff

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

- **Phase 6:** UI improvements (better layout, themes)
- **Phase 7:** Additional features (batch processing, presets)
- **Phase 8:** Consider Option C (wrapper) if maintenance burden too high

---

## Maintenance Commitment

When Sage releases a new version:

1. **Merge upstream changes** into `neely/sage`:
   ```bash
   git fetch upstream
   git merge upstream/main
   ```

2. **Re-apply lib exports** if needed (usually just resolving conflicts in `lib.rs`)

3. **Update sagegui** to use new tag

4. **Test and release** new sagegui version

**Estimated effort:** 1-2 hours per Sage release (assuming no major API changes).

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-10 | Option A (fork Sage) over Option C (wrapper) | User preference for embedded approach despite maintenance burden |
| 2026-07-10 | Keep egui/eframe | Already working, good Rust GUI choice |
| 2026-07-10 | Single main.rs file | Keep Sebastian's structure for now |
