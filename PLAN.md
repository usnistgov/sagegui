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

### Phase 3 — CI/CD & Release ✅ Complete

**Goals:**
- Verify GitHub Actions builds work
- Create first release from our fork
- Document release process
- Add version tracking and badges

**Completed:**
- [x] Push all changes
- [x] Add automated testing to CI (`cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release`)
- [x] Verify CI builds pass on all platforms (Windows, Linux, macOS x64/ARM64)
- [x] Create tag `v0.6.0`
- [x] Verify release artifacts are created
- [x] Add version badge to README (Sage version, build status, release)
- [x] Add links to release binaries in README (download table)
- [x] Document how to update Sage version in future (CHANGELOG.md)
- [x] Implement version sync (simplified to `src/version.rs` constants)
- [x] Configure Dependabot for Cargo and GitHub Actions
- [x] Add automatic release notes generation
- [x] Add version badge automation workflow
- [x] Add structured logging (`log` crate)

**Implementation Details (v0.6.0):**
- Simplified version sync: `src/version.rs` contains all Sage version constants (removed `build.rs`)
- Dependabot configured to auto-update dependencies (except pinned Sage)
- GitHub Actions `generate_release_notes: true` for automatic release notes
- New workflow `update-badges.yml` auto-updates README badge when `version.rs` changes
- Added `log` crate for structured logging (replacing `println!`)

**Checkpoint:** ✅ Release `v0.6.0` available with binaries for Windows, macOS, Linux. README shows current Sage version badge.

---

### Phase 4 — Documentation & Handoff ✅ Complete

**Goals:**
- Update README with installation instructions
- Document maintenance process
- Create "How to update Sage" guide

**Completed:**
- [x] Updated README.md with Quick Start guide
- [x] Created MAINTENANCE.md (how to sync with upstream Sage)
- [x] Added macOS Gatekeeper bypass instructions
- [x] Linked MAINTENANCE.md from README documentation section
- [x] Release notes (auto-generated via GitHub)

**Checkpoint:** ✅ Documentation complete. Project ready for handoff.

---

---

### Phase 5 — GUI Improvements & Feature Discussion (Planned)

**Goals:**
- Discuss and prioritize GUI improvements
- Plan integration of post-analysis features from sagePreview
- Improve user experience

**Potential Improvements:**

#### UI/UX Improvements (Priority)
- [ ] **Better progress display** — Show current step (building database, searching, scoring), elapsed time, estimated remaining
- [ ] **Results summary panel** — After search completes, show PSM/peptide/protein counts at 1% FDR directly in GUI
- [ ] **Configuration persistence** — Save last-used settings so users don't re-enter everything each time
- [ ] **Smarter output directory** — Default to timestamped subfolder near mzML files instead of current working directory
- [ ] Parameter presets (default, open search, semi-enzymatic)
- [ ] Save/load configuration files (JSON export/import)
- [ ] Dark mode / theme support
- [ ] Better error messages and validation

#### Post-Analysis Features (from sagePreview)
- [ ] **Digestion Efficiency Report** — Analyze missed cleavages, semi-tryptic peptides, N/C ragged ratio
- [ ] **Delta Mass Explorer** — Visualize modification distribution from open search
- [ ] **Results Summary** — Quick stats panel (PSMs, peptides, proteins at various FDR)
- [ ] **LFQ/Rollup scripts** — Integration with sagePreview LFQ analysis tools (to be discussed)
- [ ] **Export to sagePreview** — Generate config for deeper analysis

#### New Sage v0.15 Features to Expose
- [ ] Prefilter options (for large databases)
- [ ] Protein grouping settings
- [ ] Write report option
- [ ] Bruker configuration (for timsTOF data)

#### Integration Ideas
- [ ] **Link to sagePreview** — "Analyze with sagePreview" button that opens results in sagePreview
- [ ] One-click "analyze results" button
- [ ] Built-in quality metrics dashboard

**Discussion Points:**
1. Which features are highest priority?
2. Should post-analysis be built into GUI or remain separate tools?
3. How to handle the sagePreview relationship (link vs embed)?
4. LFQ/rollup scripts from sagePreview — what should be integrated?

---

## Future Phases (Not Planned Yet)

### Distribution Improvements (Future)

#### macOS Code Signing
- **Problem:** Unsigned apps trigger Gatekeeper warnings ("unidentified developer")
- **Solution:** Apple Developer Program ($99/year) + code signing in CI
- **Implementation:**
  ```yaml
  # Future workflow addition
  - name: Sign macOS Binary
    if: matrix.os == 'macos-latest'
    run: codesign --sign "${{ secrets.APPLE_DEVELOPER_ID }}" target/release/sagegui
  ```
- **Alternative:** Document how users can bypass Gatekeeper (`xattr -d com.apple.quarantine`)

#### Windows MSI Installer
- **Problem:** Raw .exe requires manual extraction, no Start Menu integration
- **Solution:** Add WiX-based MSI installer alongside .exe
- **Implementation:** Use `cargo-wix` crate
  ```yaml
  # Future workflow addition
  - name: Build MSI Installer
    if: matrix.os == 'windows-latest'
    run: |
      cargo install cargo-wix
      cargo wix --nocapture
  ```
- **Note:** Keep .exe.zip for users who prefer portable apps

---

- **Phase 6:** Batch processing (multiple files, queue system)
- **Phase 7:** Advanced visualization (spectra viewer, modification heatmaps)
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
