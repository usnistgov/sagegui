# SageGUI — Development Plan

**Goal:** A maintainable GUI for Sage that can stay up-to-date with official Sage releases.

**Approach:** Fork Sage, add library exports, maintain sync with upstream (Option A). *(locked — see NOTES.md.)*

---

## Status

- **Current phase:** Phase 5 in progress. Licensing resolved (Apache-2.0, LICENSE file added 2026-08-16). Async run-bar progress, prefilter controls, settings persistence, a Stop button, and a live Sage-log panel all landed 2026-08-21 — live-tested for the first time 2026-08-24, which found real bugs, all now fixed and live-tested same day: the Stop button (both the false completion message and, via a `neely/sage` fork patch, real mid-search cancellation — a cancelled run aborted after 2.2s of scoring vs. 60s+ to finish, wrote zero output) and settings persistence (modifications weren't surviving a restart — root cause was a `sync_from_ser()` call that was simply never made; fixed, confirmed). Templates and theme still open — modifications now correctly persisting raises the priority of the templates item (see below), since a stale mod from an unrelated prior search can now silently carry forward.
- **Last updated:** 2026-08-24
- **Next action (next session):** 2026-08-24 also reconciled PLAN against README.md's "To be added" list (the maintainer's own running wishlist) — found real gaps: Phase 6's export-format list was missing Perseus/ProteoPlotter, DIAgui, and PDV-import targets (now added, with the upstream feature-request links the maintainer had already found), and a new "iBAQ and other LFQ options" item was added, seeded with a concrete finding from the persistence audit (`LfqSettings.peak_scoring`/`integration` are hardcoded at launch regardless of the stored config; `mobility_pct_tolerance`/`peptide_q_value` have no UI at all). The Sage Log panel's two polish follow-ups were dropped from the backlog by maintainer decision (debugging aid, not a user feature — panel itself stays, just not investing more time). The macOS terminal-window bug and the app icon are now both fixed and locally verified same day (see checklist below for the mechanism and verification detail — real `.app` bundle packaging in CI, a cropped crab-wizard mascot as the icon). (The run-bar "Processing" label color was also flagged in that test, but the maintainer decided it's fine as-is — dropped.) **Not yet tested on Windows at all** — Mac-only so far, and the actual GitHub Actions macOS build hasn't run this recipe yet either (only verified by hand-building the identical bundle locally). Highest-priority open item given the persistence fix: **JSON-file templates** (bundled starting configs + a picker, replacing the inert archetype dropdown — see "Experiment templates" below) plus a `results.json` importer (README-requested; scoped in NOTES "UI-review feedback #1" as import-only, not a full Save/Load). Also open: the new Phase 6 export-format and LFQ-options items above. (The light theme and re-homing the Info/Help block off the Run tab were both dropped 2026-08-24 — maintainer confirmed neither is actually wanted; see NOTES UI-review #3 and #7.)
- **Released:** `v0.7.1` (2026-08-24) — Stop button now genuinely cancels an in-progress search (`neely/sage` commit `ed5f06c`), settings persistence fixed for modifications + fully audited, real app icon, macOS `.app` bundle packaging (fixes a terminal window opening alongside the GUI). Previous: `v0.7.0` — Multi-FASTA + on-the-fly concatenation. `v0.6.0` — Sage v0.15.0-beta.2 (commit `d74024df`).

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

**Key dependency:** Our fork of Sage at `github.com/neely/sage` (v0.15.0-beta.2, commit `ed5f06c` — carries two additive hand-written patches on top of upstream `d74024df`: the `Runner.progress` counter and the `Runner.cancel` cooperative-cancellation flag, see NOTES.md).

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

- [x] **Async execution + progress display** — Run search on a background thread so the GUI stays responsive; show current step, elapsed time, real progress. *(2026-08-21: real spectra-scored / pre-scanned-total percentage during search, via `neely/sage`'s `Runner.progress` — see NOTES.md. Named-step text and a live Sage-log panel added the same session. 2026-08-24 live test: confirmed working — phase text is described as "good" even while the bar isn't moving, and the log panel fills in real time once the search starts. Confirmed gap, not a bug: both the bar and the log panel stay silent for the first ~1-2 min during database build on a large FASTA — Sage itself only logs that phase at `trace` level. Two small issues found: the green "Processing" label reads poorly (color/contrast fix flagged, then reconsidered and dropped the same session — see below), and no estimated-remaining-time readout exists.)*
- [x] **Remember settings between sessions** — `Config`, the tolerance-type selections, experiment archetype, and active tab persist through eframe's `persistence` feature. *(Landed 2026-08-21. 2026-08-24 live test found modifications don't survive a restart. Root-caused and fixed same day: `StaticModConfig`/`VariableModConfig` keep a `#[serde(skip)]` live map alongside a serializable shadow map, and the `sync_from_ser()` method meant to rebuild the live map after deserializing was simply never called — the mods were saved correctly, just never rebuilt into the form the UI reads. Fixed in `SageLauncher::new`, live-tested and confirmed same day. Followed by a full field-by-field audit (same session, requested explicitly): a new test round-trips a `PersistedState` with every single field — not just mods — set to a non-default value through the real serde types and the real restore logic, sanity-checked by confirming it fails without the fix. All fields pass; no other gap exists. **New consequence, not a bug:** mods now correctly carrying over between sessions means a stale mod from an unrelated prior search can silently apply to the next one — raises the priority of the still-open "Experiment templates" item below as the planned reset-to-defaults mechanism.)*
- [x] **Stop button** — Cancels a run from the run bar. *(Landed 2026-08-21 for the pre-search phase. 2026-08-24 live test found a real bug: clicking Stop **during** an active search didn't stop it and falsely reported "No output files were written" even though the run completed and wrote output normally. Message bug fixed and live-tested same day. That test also surfaced that non-interruption itself was unacceptable, not just the message — so real cooperative cancellation was built the same day via a `neely/sage` fork patch (commit `ed5f06c`, see NOTES "Stop button" and "Custom patch carried on neely/sage: Runner.cancel"), then live-tested and confirmed: a mid-scoring Stop click aborted after 2.2s (vs. 60s+ to finish) and wrote zero output files. Both halves of the original bug are now fixed and confirmed.)*
- [x] **macOS: a terminal window opens alongside the GUI** — found in live testing 2026-08-24, fixed same day. Root cause confirmed: `build.yml` shipped the raw Mach-O binary in a `.tar.gz`, with no `.app` bundle structure — Finder/LaunchServices treats an unbundled executable as a plain Unix binary and runs it via Terminal.app. Fixed by adding a "Create macOS App Bundle" CI step that packages `target/release/sagegui` into `Sage Launcher.app` (`Contents/MacOS`, `Contents/Resources/AppIcon.icns`, a generated `Info.plist` with `CFBundlePackageType=APPL`) before archiving, for both macOS matrix legs. **Verified locally, not just built**: constructed the exact bundle by hand from the real release binary on this Mac, launched it with `open` (the same mechanism Finder uses), and confirmed zero Terminal windows opened before/after (`osascript -e 'tell application "Terminal" to count windows'` stayed at 0) while the process ran correctly. README's macOS Gatekeeper-bypass instructions need a follow-up check (`xattr -d com.apple.quarantine` on a `.app` may need `-r` for the bundle) — see NOTES.
- [x] **Run-bar "Processing" label color** — flagged in live testing 2026-08-24, briefly dropped the same session on a miscommunication (maintainer thought a different colored text was meant), then reopened and actually fixed after live-testing v0.7.1 on Windows and seeing the hardcoded `egui::Color32::GREEN` still looked wrong. Changed to a plain `ui.label(...)` so it inherits the theme's normal text color instead of a hardcoded pure green — matches the plain elapsed-time label already sitting right next to it. Not yet re-verified live (the fix landed after v0.7.1 shipped); low risk since it's identical to an adjacent already-correct label, but worth a glance next run.
- [x] **App icon** — was a placeholder (eframe's own default: a white "e" on black, per live-test feedback 2026-08-24). Fixed same day: cropped the crab-wizard mascot out of `assets/sagegui_logo-removebg.png` (flood-fill background removal from the corners, so enclosed white bits like the hat's stars survive), padded to a square `assets/icon-master.png` (1024×1024), derived `assets/icon-256.png` (runtime window/taskbar icon, wired via `eframe::ViewportBuilder::with_icon` in `src/main.rs`), `assets/AppIcon.icns` (macOS bundle icon, built with `iconutil`), and `assets/AppIcon.ico` (Windows, built but not yet wired into the `.exe` file icon — see NOTES). **Verified, not just built**: launched the debug binary and screenshotted the Dock — the crab wizard renders correctly, not the default "e".
- ~~**Sage Log panel: selectable/copyable text**~~ and ~~**Sage Log panel: confirm no real emptiness bug**~~ — dropped 2026-08-24. The panel was built as a debugging aid during the async-execution work, not a feature end users need; maintainer decided to keep it as-is (it works) but not invest further polish time. Not dead code, just no longer an active backlog item — see NOTES if it needs picking back up.
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
- ~~**High-contrast "Y2K" theme**~~ — dropped 2026-08-24. Originally flagged 2026-08-13 as "the default grey-on-grey is too faint," read at the time as wanting a dedicated flat-light theme. The dark-theme contrast pass landed 2026-08-21 (`dark_visuals()`, sage-green accent) fixed the actual complaint — the maintainer confirmed 2026-08-24 the original faintness was their system being in dark mode against egui's low-contrast default, not a request for a separate light theme. No further theme work needed. See NOTES → UI-review feedback #7 for the full history.

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

*(Reconciled 2026-08-24 against README.md's "To be added" list — README had
several targets PLAN didn't yet track. This is now the complete list; keep
both in sync going forward, per AGENTS.md.)*

Each of these requires understanding the target format and confirming Sage's output contains the required fields. Research is an action item per format before implementing.

- [ ] **pepXML / mzIdentML export** — the two standard interchange formats several downstream tools (Scaffold among them) ingest. Investigate whether spoofing pepXML from Sage results is complete enough to be useful; mzIdentML is the more modern/complete standard but more complex to produce correctly.
- [ ] **MSstats-compatible export** — Understand MSstats input format (feature-level TSV with specific column names); produce it from Sage results. Likely feasible with our existing data. **Preferred path:** contribute a Sage-input feature directly to [MSstatsConvert](https://github.com/Vitek-Lab/MSstatsConvert) rather than (or in addition to) exporting our own spoofed file — [feature already requested upstream](https://github.com/Vitek-Lab/MSstatsConvert/issues/143).
- [ ] **Perseus-format export** — for [Perseus](https://maxquant.net/perseus/) and [ProteoPlotter](https://github.com/JGM-Lab-UoG/ProteoPlotter). Format/column requirements not yet researched.
- [ ] **DIAgui-compatible export** — for [DIAgui](https://github.com/mgerault/DIAgui). Format/column requirements not yet researched.
- [ ] **FragPipe Analyst / LFQ-Analyst / *-Analyst export** — [LFQ-Analyst](https://github.com/MonashBioinformaticsPlatform/LFQ-Analyst), FragPipe-Analyst, and the other tools under the [*-Analyst suite](https://analyst-suites.org/) likely share a common input shape. Identify required format; map Sage output columns.
- [ ] **PDV import** — add SageGUI/Sage output as a supported import format in [PDV](https://github.com/wenbostar/PDV) (a spectrum/PSM viewer), rather than building our own viewer. [Feature already requested upstream](https://github.com/wenbostar/PDV/issues/110).
- [ ] **Scaffold-compatible export (?)** — Scaffold ingests pepXML or mzIdentML (see above), so this may fall out of that work rather than needing a dedicated exporter. Still marked uncertain (README: "Scaffold (?)") — confirm Scaffold's actual import requirements before committing effort here.

**Note on scope:** Format export is "spoof where we have the data, document gaps where we don't." We won't invent data that Sage doesn't produce. Where an upstream tool already has an open feature request for Sage support (MSstatsConvert #143, PDV #110), **contributing there may be less total work and more durable than a parallel SageGUI-side exporter** — worth a real "build vs. contribute upstream" decision per format before implementing, not just defaulting to building our own.

#### iBAQ and other LFQ options

Not yet in PLAN before this session (added from README's "To be added" list,
2026-08-24). **Concrete starting point found during the settings-persistence
audit the same day:** `sage_core::lfq::LfqSettings` (used internally by
`QuantType::Lfq`) already has `peak_scoring`, `integration`,
`mobility_pct_tolerance`, and `peptide_q_value` fields, but only
`ppm_tolerance`, `spectral_angle`, and `combine_charge_states` have UI
widgets (`QuantType::update_section`, `src/ui.rs`) — the other four are
never user-editable. Worse, `peak_scoring`/`integration` aren't even read
from the stored `LfqSettings` at launch: `From<QuantType> for QuantOptions`
(`src/ui.rs`) hardcodes `PeakScoringStrategy::Hybrid` and
`IntegrationStrategy::Sum` regardless of what's in the struct. iBAQ itself
(intensity-based absolute quantification — sum of peptide intensities
divided by the number of theoretically observable tryptic peptides for a
protein) isn't a `LfqSettings` field at all; check whether Sage computes it
internally anywhere, or whether this needs a rollup-script-style post-
processing step (ties to the Phase 5 "Port rollup scripts" item above).
Research needed before implementing: what `peak_scoring`/`integration`
options Sage actually supports and what tradeoff each represents (worth
tooltips, matching the existing pattern for other advanced fields), and
whether iBAQ is a Sage-side computation or a downstream one.

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
