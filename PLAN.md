# SageGUI — Development Plan

**Goal:** A maintainable GUI for Sage that can stay up-to-date with official Sage releases.

**Approach:** Fork Sage, add library exports, maintain sync with upstream (Option A). *(locked — see NOTES.md.)*

---

## Status

- **Current phase:** Phase 5 in progress. Licensing resolved (Apache-2.0, LICENSE file added 2026-08-16). Async run-bar progress, prefilter controls, settings persistence, a Stop button, and a live Sage-log panel all landed 2026-08-21 — live-tested for the first time 2026-08-24, which found real bugs in two of them. The Stop-button bug is fixed (same day). Settings-persistence audit and templates/theme still open.
- **Last updated:** 2026-08-24
- **Next action (next session):** The Stop-button false "no output written" message is **fixed** — `check_thread_status` now trusts the run's actual result instead of the Stop flag alone (see NOTES "Stop button"), covered by three new unit tests, `cargo build`/`test`/`clippy`/`fmt` all clean. **Not yet confirmed in the live GUI** — no accessibility/computer-use tool for a native macOS app was available this session either, so the maintainer should re-run the click-Stop-mid-search reproduction from 2026-08-24 to confirm end-to-end, including whether the "run bar left in a confusing state" symptom is fully explained (see NOTES for the current best guess: it's the long wait while a real search finishes, not a separate bug) or needs its own follow-up. After that: **(2) Settings persistence** — modifications don't survive a restart, and it's unconfirmed what else might not; needs a full field-by-field audit, not just a mods patch. Smaller items also logged: a macOS-only terminal window opens alongside the GUI (Windows already fixed), the green "Processing" run-bar label reads poorly, and the app icon needs real branding. **Not yet tested on Windows at all** — Mac-only so far. Also still open: Load Config round-trip with `fasta_paths`, JSON-file templates, the high-contrast flat light theme (NOTES UI-review #7), and re-homing the Info/Help block off the Run screen (NOTES UI-review #3).
- **Released:** `v0.7.0` — Multi-FASTA + on-the-fly concatenation. Previous: `v0.6.0` — Sage v0.15.0-beta.2 (commit `d74024df`).

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

- [x] **Async execution + progress display** — Run search on a background thread so the GUI stays responsive; show current step, elapsed time, real progress. *(2026-08-21: real spectra-scored / pre-scanned-total percentage during search, via `neely/sage`'s `Runner.progress` — see NOTES.md. Named-step text and a live Sage-log panel added the same session. 2026-08-24 live test: confirmed working — phase text is described as "good" even while the bar isn't moving, and the log panel fills in real time once the search starts. Confirmed gap, not a bug: both the bar and the log panel stay silent for the first ~1-2 min during database build on a large FASTA — Sage itself only logs that phase at `trace` level. Two small issues found: the green "Processing" label reads poorly (color/contrast fix needed), and no estimated-remaining-time readout exists.)*
- [ ] **Remember settings between sessions** — `Config`, the tolerance-type selections, experiment archetype, and active tab persist through eframe's `persistence` feature. *(Landed 2026-08-21. 2026-08-24 live test: file paths, FASTA, and output directory correctly survive a restart — **modifications do not**, and it's unconfirmed what else might not. Reopened pending a full field-by-field audit; see NOTES "Settings persistence" for where to start looking.)*
- [x] **Stop button** — Cancels a run from the run bar. *(Landed 2026-08-21 for the pre-search phase — confirmed working. 2026-08-24 live test found a real bug: clicking Stop **during** an active search doesn't stop it (expected, by design) but falsely reported "No output files were written" even though the run completed and wrote output normally. Fixed same day: `check_thread_status` now trusts the actual run result instead of the Stop flag alone — see NOTES "Stop button" for the mechanism and three new regression tests in `src/main.rs`. Not yet re-tested in the live GUI — the "run bar left in a confusing state" symptom is probably just the long wait while a real search finishes, not a separate bug, but needs the maintainer's live re-test to confirm.)*
- [ ] **macOS: a terminal window opens alongside the GUI** — found in live testing 2026-08-24. Windows already suppresses this via `windows_subsystem = "windows"` (macOS/Linux no-op); macOS needs proper `.app` bundle investigation (check CI's macOS packaging step in `build.yml`, likely missing an `Info.plist`/bundle structure that lets Finder launch it without attaching a console).
- [ ] **Run-bar "Processing" label color** — found in live testing 2026-08-24: the green text reads poorly, especially against the new dark theme. Needs a better color choice (candidate: the theme's own sage-green accent rather than plain `egui::Color32::GREEN`).
- [ ] **App icon** — current icon is a placeholder ("just an E on a black background" per live-test feedback 2026-08-24). Needs real branding before any wider release.
- [ ] **Session resilience / auto-recovery** — If the GUI is closed or crashes during a run, persist enough state to resume or at least report results.
- [ ] **Results summary panel** — After search completes, show PSM/peptide/protein counts at specified FDR threshold directly in GUI. *(Placeholder slot reserved on Run/Info tab.)*
- [x] **Configuration persistence (save/load)** — Save/Load Config as JSON on the Experiment tab. *(Remembering last-used settings across sessions is still open.)*
- [ ] **Smarter output directory** — Default to timestamped subfolder near mzML files instead of current working directory.
- [x] **Expanded modifications preset library** — Modifications tab redesigned as a two-box (Static/Variable) list-picker with a curated "Common modifications" master list + transfer arrows and a "+ Custom…" escape hatch. Multi-residue presets insert as separate rows; Static/Variable mutually exclusive. *(Landed 2026-08-13. Presets are hardcoded in `src/ui.rs` `MOD_PRESETS`; masses are Unimod monoisotopic deltas.)*
- [x] **Parameter documentation in the GUI** — Inline `on_hover_text` tooltips on controls (copy sourced from `docs/ui-spec.md` §3 / NOTES).
- [ ] Parameter presets (default, open search, semi-enzymatic) *(Experiment tab dropdown exists but is **inert** — selecting an archetype does nothing to the other tabs; confirmed 2026-08-13. Needs `apply_archetype`. See NOTES → UI-review feedback #6.)*
- [x] Save/load configuration files (JSON export/import) — **removed from v0.7.0** pending schema-alignment design (NOTES UI-review #1). Placeholder left on Experiment tab.
- [ ] Better error messages and validation
- [ ] **Delta-mass framing for the Da tolerance window (behavior change — DEFERRED, caveats)** — Optionally let the user enter the precursor Da window in **delta-mass / modification space** (type `+500` for "find IDs carrying a +500 Da mod") instead of Sage's raw `(lower, upper)` relative to the experimental mass, where a `-500` lower bound is what actually finds a +500 Da mod. This is the sign-flip Michael flagged. **Currently NOT done** — the GUI passes the two boxes through verbatim as Sage's `(lower, upper)`, and we added Lower/Upper labels + hover text + an inverted-window warning to explain the raw convention (see NOTES → "Precursor/fragment tolerance window — sign & delta-mass convention"). **Caveats before building this:**
  - **Divergence from Sage.** Every Sage `config.json`, the CLI, and the docs use the raw `(center + lower, center + upper)` convention. A delta-mass GUI would flip signs, so a value shown in SageGUI would not match the number in a Sage config file — confusing for users who cross-reference, and a Save/Load Config round-trip would need to convert both ways without drift.
  - **Only the Da precursor window has an intuitive delta-mass reading.** ppm and fragment tolerances don't; a partial re-framing (Da-precursor only) risks being *more* confusing than a consistent raw convention.
  - **Two-number asymmetry doesn't collapse to one.** Delta-mass framing is cleanest for a single offset, but the window is still a `(lower, upper)` pair — a delta-mass UI still has to present two bounds, so the win is mainly sign intuition, not simplicity.
  - **Preferred shape if pursued:** a display-only toggle ("show as delta mass") that flips signs/labels in the widget but keeps `Config`/serialization in Sage's raw convention — never store the flipped values. Decide dedup/round-trip semantics first. Lower priority than the labels+hover already shipped.
- [ ] **High-contrast "Y2K" theme** — the default grey-on-grey is too faint. Build a custom `egui::Visuals` (high-contrast, larger font, visible borders). Pure styling, next-session item. See NOTES → UI-review feedback #7.

**UI restructure (landed 2026-08-13):** sidebar-nav + pinned run-bar, 6 tabs, UI extracted to `src/ui.rs`, the 6 previously-hidden Sage params surfaced, native `.d`/Bruker support dropped (mzML/.gz only). See NOTES → "UI redesign" and CHANGELOG [Unreleased].

#### Input: multi-FASTA & contaminants — **NEXT SESSION, targets v0.7.0 release**

The top remaining file-input item. cRAP is just another FASTA, so both share one
concat mechanism. Concrete spec:

- [x] **Multi-FASTA selection** — replace the single `fasta: String` text/browse
  box on Files & Database with an **add/select list**: "Add FASTA…" (multi-pick
  append), a list showing each picked file with a per-row remove, in selection
  order (target organism + contaminants + spike-ins). Data model: change
  `DatabaseConfig.fasta: String` → `fasta_paths: Vec<PathBuf>` (keep a serde
  migration path / default so old config JSONs still load). *(Landed 2026-08-13.)*
- [x] **On-the-fly concatenation** — before launching Sage, concatenate all
  selected FASTAs into a **single temp file**, pass its path as Sage's `fasta`.
  Sage takes one DB. Clean up the temp file after the run. Single-file case
  bypasses the copy. *(Landed 2026-08-13. No dedup of identical headers — just
  concatenate; documented decision in NOTES.)*
- ~~**Built-in cRAP toggle**~~ — dropped in favour of **just more FASTA slots**;
  user adds their cRAP file like any other FASTA (decision 2026-08-13).

#### Experiment templates (JSON-file approach) — secondary next session, if quick

Chosen 2026-08-13 over hardcoded `apply_archetype`. **A template *is* a saved
config JSON** — reuse the existing Save/Load Config plumbing:

- [ ] Ship bundled example templates in `assets/templates/` (e.g.
  `tryptic-lfq.json`, `phospho.json`, `wide-open.json`), built from real
  `results.json` / settings the lab uses.
- [ ] Replace the inert Experiment dropdown with a **Templates** dropdown that
  loads a bundled JSON into `self.config` (same code path as Load Config), plus a
  **"Save current as template"** that writes the current config to a user file.
- [ ] Keep `Custom` = "no template applied." Removes the need for hardcoded
  archetype values entirely. If this isn't fast, **leave the dropdown in-progress**
  and ship v0.7.0 on multi-FASTA alone.

#### Input: Thermo .raw conversion

- [ ] **ThermoRawFileParser integration** — Bundle or detect [ThermoRawFileParser](https://github.com/compomics/ThermoRawFileParser) and invoke it automatically when `.raw` files are selected, converting to mzML before the search. Saves users up to 1hr of manual conversion per batch.
- **Before implementing:** verify ThermoRawFileParser license compatibility with our Apache-2.0 (it's Apache-2.0 itself — confirm no distribution constraints for bundling a .NET binary).

#### New Sage v0.15 features to expose

- [x] **Prefilter options (for large databases)** — Surfaced `prefilter`, `prefilter_chunk_size` and `prefilter_low_memory` on Files & Database, between the FASTA list and the Advanced block, with a contextual hint when semi-enzymatic digestion is on. Defaults all three to what Sage resolves to on its own (`false` / `0` / `true`), so a SageGUI run matches a Sage CLI run with the same visible config. The run bar shows a distinct "Prefiltering database in chunks…" phase message (no percentage — see NOTES → Database prefiltering, the pass has no progress signal). `docs/PARAMETER_REFERENCE.md` rewritten with the corrected `prefilter_low_memory` default. *(Landed 2026-08-21.)*
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

- **Phase 7:** Batch processing (multiple files, queue system)
- **Phase 8:** Advanced visualization (spectra viewer, modification heatmaps)
- **Phase 9:** Consider Option C (wrapper) if maintenance burden too high

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

**State:** Phases 0–4 done, `v0.6.0` released. Phases 5 & 6 are fully planned — nothing is implemented yet.

**Immediate next actions (in order):**

1. **Async execution** (Phase 5, #1 priority) — move the search onto a background thread; this unblocks all other UX work.
2. **Locate rollup scripts** — they exist in a separate project (not sagePreview). Find them, read them, record language + structure in NOTES before Phase 6 can be scoped accurately.
3. **Phase 6 format survey** — 30-min spike: find a sample input file for MSstats, LFQ-analyst, and Scaffold; identify which columns Sage already produces vs. what needs synthesizing; record gap analysis in NOTES under "Output format reference."

**Watch out for:**
- Don't re-add `build.rs` version detection, don't add `lib.rs` to sage-cli, don't switch the Sage dep back to a branch — all dead-ends (NOTES.md).
- ThermoRawFileParser: license is clear (Apache-2.0), but the .NET runtime dependency on Linux/macOS is an open question. Do the cross-platform spike before committing to that design.
- Rollup scripts may be R, not Python — changes the end-user dependency story even if the call strategy is the same.
- TMT quant is still untested (no TMT data). LFQ is the only validated path.
- Any behavior change must sync README / CHANGELOG / MAINTENANCE in the same session.

**Key files:** `src/main.rs` (all GUI), `src/version.rs` (Sage version constants), `Cargo.toml` (pinned Sage commit), `.github/workflows/` (build + badges).
