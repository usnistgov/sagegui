# SageGUI — Development Plan

**Goal:** A maintainable GUI for Sage that can stay up-to-date with official Sage releases.

**Approach:** Fork Sage, add library exports, maintain sync with upstream (Option A). *(locked — see NOTES.md.)*

---

## Status

- **Current phase:** None — Phases 0–4 complete. Phases 5 & 6 are planned, not started.
- **Last updated:** 2026-07-24
- **Next action:** Start Phase 5 — async execution is the highest-priority item (blocks everything else UX-wise). Also locate the rollup scripts before Phase 6 can be scoped.
- **Released:** `v0.6.0` — Sage v0.15.0-beta.2 (commit `d74024df`), binaries for Windows / macOS (x64+ARM64) / Linux.

Locked decisions, gotchas, and the API-change reference now live in `NOTES.md`. Session history is in `JOURNAL.md`.

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

### Phase 5 — Core UX & Input Improvements (Planned)

**Goals:** Address the blocking usability pain points before adding features. Async execution is highest priority — long searches currently freeze the GUI.

#### UI/UX (priority order)

- [ ] **Async execution + progress display** — Run search on a background thread so the GUI stays responsive; show current step (building DB, searching, scoring), elapsed time, estimated remaining. Prevents "not responding" on 1hr+ jobs.
- [ ] **Session resilience / auto-recovery** — If the GUI is closed or crashes during a run, persist enough state to resume or at least report results.
- [ ] **Results summary panel** — After search completes, show PSM/peptide/protein counts at specified FDR threshold directly in GUI.
- [ ] **Configuration persistence** — Save last-used settings; don't make users re-enter everything each session.
- [ ] **Smarter output directory** — Default to timestamped subfolder near mzML files instead of current working directory.
- [ ] **Expanded modifications preset library** — Dropdown of common variable and static mods so users aren't typing them manually.
- [ ] Parameter presets (default, open search, semi-enzymatic)
- [ ] Save/load configuration files (JSON export/import)
- [ ] Better error messages and validation

#### Input: multi-FASTA & contaminants

- [ ] **Multi-FASTA selection** — Allow selecting multiple `.fasta` files; concatenate on-the-fly before passing to Sage (e.g., target organism + contaminants + spike-ins).
- [ ] **Built-in cRAP toggle** — Bundle the cRAP contaminants database; one checkbox appends it to the search without users managing the file.

#### Input: Thermo .raw conversion

- [ ] **ThermoRawFileParser integration** — Bundle or detect [ThermoRawFileParser](https://github.com/compomics/ThermoRawFileParser) and invoke it automatically when `.raw` files are selected, converting to mzML before the search. Saves users up to 1hr of manual conversion per batch.
- **Before implementing:** verify ThermoRawFileParser license compatibility with our Apache-2.0 (it's Apache-2.0 itself — confirm no distribution constraints for bundling a .NET binary).

#### New Sage v0.15 features to expose

- [ ] Prefilter options (for large databases)
- [ ] Protein grouping settings
- [ ] Write report option
- [ ] Bruker configuration (for timsTOF data)

#### sagePreview integration

- [ ] **Port rollup scripts** — The peptide→protein rollup and LFQ aggregation scripts currently live in a separate project (not sagePreview). Action item: locate, read, and refactor them into a form SageGUI can call. (See Phase 6 for the GUI surface.)
- [ ] **Digestion Efficiency Report** — Port from sagePreview: missed cleavages, semi-tryptic peptides, N/C ragged ratio.
- [ ] **Delta Mass Explorer** — Port from sagePreview: modification distribution from open search.
- [ ] **Link to sagePreview** — "Analyze with sagePreview" button for deeper analysis.

---

### Phase 6 — Output Formatting & Downstream Export (Planned)

**Goals:** Let users get FDR-filtered protein/peptide tables and export to the formats their downstream tools expect. The rollup logic (peptide→protein at a specified FDR) comes from the scripts ported in Phase 5.

#### FDR-filtered rollup export

- [ ] **Peptide-level export at specified FDR** — User sets FDR threshold (default 1%); export filtered `results.sage.tsv`.
- [ ] **Protein-level rollup export** — Apply rollup scripts to produce a protein-level intensity table at the specified FDR.

#### Format spoofing for downstream tools

Each of these requires understanding the target format and confirming Sage's output contains the required fields. Research is an action item per format before implementing.

- [ ] **MSstats-compatible export** — Understand MSstats input format (feature-level TSV with specific column names); produce it from Sage results. Likely feasible with our existing data.
- [ ] **FragPipe Analyst / LFQ-analyst export** — Identify required format; map Sage output columns.
- [ ] **Scaffold-compatible export** — Scaffold ingests pepXML or mzIdentML. Investigate whether spoofing pepXML from Sage results is complete enough to be useful.

**Note on scope:** Format export is "spoof where we have the data, document gaps where we don't." We won't invent data that Sage doesn't produce.

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

When Sage releases a new version, sync the fork, bump the pinned commit, fix any API changes, test, and release. Full step-by-step procedure lives in **MAINTENANCE.md**; the v0.14.7→v0.15.0-beta.2 fixes are the worked example in **NOTES.md** (API changes reference).

**Estimated effort:** 1–2 hours per Sage release (assuming no major API changes).

---

## Decision Log

Locked decisions and their rationale have moved to **NOTES.md → Design decisions (locked)**. That is now the single source of truth — don't duplicate them here. For the dated sequence of when things were decided, see **JOURNAL.md**.

---

## Handoff — for the next session

**Start here:** read AGENTS.md, then this status block, then NOTES.md (locked decisions + dead-ends), then the top of JOURNAL.md.

**State:** Project is at a clean stopping point — Phases 0–4 done, `v0.6.0` released and building on all platforms. Nothing is in flight.

**If picking up Phase 5:** it hasn't been scoped with the user yet. Resolve the four "Discussion Points" under Phase 5 *before* coding — especially the sagePreview relationship (link vs. embed) and which improvements are highest priority.

**Watch out for:**
- Don't re-add `build.rs` version detection, don't add `lib.rs` to sage-cli, don't switch the Sage dep back to a branch — all dead-ends (NOTES.md).
- TMT quant is still untested (no TMT data). LFQ is the only validated path.
- Any behavior change must sync README / CHANGELOG / MAINTENANCE in the same session.

**Key files:** `src/main.rs` (all GUI), `src/version.rs` (Sage version constants), `Cargo.toml` (pinned Sage commit), `.github/workflows/` (build + badges).
