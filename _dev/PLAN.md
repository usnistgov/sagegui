# SageGUI development plan

**Goal:** A maintainable GUI for Sage that can stay up-to-date with official Sage releases.

**Approach:** Compile Sage in as a library, vendored in `vendor/sage` (Option A). *(Locked. See NOTES.md.)*

---

## Status

- **Current phase:** Published as `usnistgov/sagegui` (2026-09-23), a GitHub fork of `jspaezp/sagegui`. Releases are tagged `nist-vX.Y.Z`. Actions is on (2026-09-25). On `main` but not yet in a release: vendored Sage with six patches (three code, three dependency), the reworked Info / Help block, the database cache (hidden by `index_cache::ENABLED`), the licence files and the `_dev/` move.
- **Last updated:** 2026-09-25
- **Next action (next session):** (1) Check the result of the first manual `build.yml` run on usnistgov (see JOURNAL 2026-09-25). (2) Cut the first NIST-hosted release as `nist-vX.Y.Z` (version not chosen). Decide first whether the cache ships in it. (3) Then the open items in Handoff below. Keep `neely/sage` public: commits before the vendoring still pin it.
- **Released:** `nist-v0.9.0` (2026-09-22): Convert results to mzIdentML and pepXML, a Results location, Write HTML report, a pre-run output-folder check, a Combine Charge States hover note. Earlier releases are in CHANGELOG.md.

Locked decisions, gotchas, and the API-change reference now live in `NOTES.md`. Session history is in `JOURNAL.md`.

---

## Mission

Provide a user-friendly graphical interface for Sage that:
1. Works with the latest Sage releases
2. Builds for Windows, macOS, and Linux
3. Exposes all important Sage parameters
4. Can be maintained long-term without excessive effort

---

## Non-goals (do not build)

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
│  │              sage-core / sage-cli (vendored)            ││
│  │  - Input struct (search parameters)                     ││
│  │  - Runner (search execution)                            ││
│  │  - Output (results)                                     ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

**Key dependency:** Sage, vendored in `vendor/sage/` since 2026-09-23: upstream `lazear/sage` commit `d74024df` (10 commits after v0.15.0-beta.2) plus six patches. Three are additive code patches (`Runner.progress`, `Runner.cancel`, `Runner::from_parts`). Three are dependency updates that clear security advisories. See `vendor/sage/VENDORED.md` and `PATCHES.md`. Before 2026-09-23 it was the Git fork `github.com/neely/sage` at `ed5f06c`.

---

## Phases

### Phases 0 to 4: done

- **Phase 0, bug fixes (done):** TMT 16/18-plex selection and the fragment tolerance type switch. Both in Sebastian's original code.
- **Phase 1, Sage v0.15.0-beta.2 (done):** API changes are in NOTES "API changes reference".
- **Phase 2, test on real data (done):** 60,672 PSMs and LFQ from one mzML file. TMT quantification is still untested (no TMT data).
- **Phase 3, CI and release (done):** fmt, clippy, test and release builds for Windows, Linux and macOS (Intel and Apple Silicon). `src/version.rs` holds the Sage version. Dependabot is on.
- **Phase 4, documentation (done):** README quick start, MAINTENANCE.md, macOS Gatekeeper instructions.

---

### Phase 5: core UX and input (largely done)

**Goals:** Address the blocking usability pain points before adding features. Async execution came first, because long searches froze the GUI.

#### UI/UX (priority order)

- [x] **Async execution and progress display.** Real percentage during scoring, named phases, a live Sage Log panel. The database build phase is silent for 1 to 2 minutes on a large FASTA, because Sage logs it only at `trace`. No time-remaining estimate. See NOTES "Sage patch 1".
- [x] **Remember settings between sessions.** Every field round-trips, checked by a test. Mods now carry over, so a stale mod can apply to the next search; re-applying a template resets it. See NOTES "Settings persistence".
- [x] **Stop button.** Cooperative cancellation, also during scoring (2.2 s to stop in the live test). See NOTES "Stop button".
- [x] **macOS terminal window.** Fixed with a real `.app` bundle. See NOTES "macOS terminal window + app icon".
- [x] **Run-bar "Processing" label colour.** Uses the theme text colour.
- [x] **App icon.** Crab-wizard mascot. `AppIcon.ico` is not yet wired into the Windows `.exe`. See NOTES.
- ~~**Sage Log panel polish**~~: dropped 2026-08-24. The panel works; no further work planned.
- [x] **Reset to defaults on numeric controls.** Hover text names each default; re-applying a template is the exact reset. A per-control reset button was rejected (see NOTES "A template is a complete state").
- [ ] **Session resilience / auto-recovery**: If the GUI is closed or crashes during a run, persist enough state to resume or at least report results.
- [ ] **Results summary panel**: After search completes, show PSM/peptide/protein counts at specified FDR threshold directly in GUI. *(Placeholder slot reserved on Run/Info tab.)*
- [ ] **Configuration export (save)**: Save the current config as JSON. *(Not built. It needs an exporter from `Config` back to Sage's schema. The import half is done: see "Load configuration from a Sage JSON file" below. Remembering last-used settings across sessions is also done: see "Remember settings between sessions" above.)*
- [ ] **Smarter output directory**: Default to timestamped subfolder near mzML files instead of current working directory.
- [x] **Modifications preset library.** Static and Variable boxes with a curated list (`MOD_PRESETS` in `src/ui.rs`, Unimod masses).
- [x] **Parameter hover text** on controls.
- [x] **Parameter presets.** Done as Experiment templates (see below).
- [x] **Load configuration from a Sage JSON file.** Import only, from `config.json` or `results.json`. Reports what it could not apply.
- [ ] Better error messages and validation
- [x] **Delta-mass display of the tolerance windows.** Display only: `Config` and everything sent to Sage keep the raw convention. The raw pair is printed under the control. See the STOP section in AGENTS.md and NOTES "Tolerance display is delta mass".
- ~~**High-contrast theme**~~: dropped 2026-08-24. The dark-theme contrast pass fixed the actual complaint. See NOTES UI-review feedback #7.

**UI restructure (landed 2026-08-13):** sidebar-nav + pinned run-bar, 6 tabs, UI extracted to `src/ui.rs`, the 6 previously-hidden Sage params surfaced, native `.d`/Bruker support dropped (mzML/.gz only). See NOTES "UI redesign".

#### Input: multi-FASTA and contaminants (done, v0.7.0)

- [x] **Multi-FASTA list**, concatenated to one temp file before the run. No header dedup. cRAP is added as a normal FASTA.

#### Experiment templates (done 2026-09-08)

- [x] Five bundled templates in `assets/templates/`, in Sage's own schema. See NOTES "The bundled set".
- [x] A Templates dropdown replaced the inert Experiment dropdown. `ExperimentType` stays in `PersistedState` so old settings load.
- [ ] **"Save current as template".** Needs an exporter from `Config` to Sage's schema (same as Configuration export above).

#### Governance and licensing

NIST FAIR governance files landed 2026-09-08. The licence is resolved (2026-09-23): NIST `LICENSE.md`, derived files listed in `THIRD_PARTY_LICENSES.md`. See NOTES "License and governance".

- [ ] **Mint a DOI**, then add a top-level `doi:` to `CITATION.cff`, README "Citation" and `sagegui_citation()` in `src/ui.rs`.
- [ ] **Confirm the CODEMETA `themes` nesting** with the code.nist.gov maintainers. Fix sageRecon and this repo together.
- ~~**Rename checklist**~~: superseded 2026-09-23. The repository kept the name as `usnistgov/sagegui`. See NOTES.

#### Enzyme presets (from sageRecon)

- [x] **14 enzyme presets from sageRecon.** Trypsin stays the default; the picker names the enzyme, and the cut side is a labelled radio pair. See NOTES "Enzyme presets".
- [ ] sageRecon's **MS2 tolerance by analyzer class** (Orbitrap / FT-ICR 20 ppm, Astral 20 ppm, TOF/QTOF 100 ppm, ion trap 1.0 Da) as hover text on the fragment-tolerance control.

#### Input: Thermo .raw conversion

- [ ] **ThermoRawFileParser integration**: Bundle or detect [ThermoRawFileParser](https://github.com/compomics/ThermoRawFileParser) and invoke it automatically when `.raw` files are selected, converting to mzML before the search. Saves users up to 1hr of manual conversion per batch.
- **Before implementing:** verify ThermoRawFileParser license compatibility with our Apache-2.0 (it is Apache-2.0 itself; confirm no distribution constraints for bundling a .NET binary).

#### New Sage v0.15 features to expose

- [x] **Prefilter options.** Defaults match what Sage resolves on its own. See `docs/PARAMETER_REFERENCE.md`.
- [ ] Protein grouping settings
- [ ] Write report option
- [ ] Bruker configuration (for timsTOF data)

#### Speed

- [x] **Database cache (built, hidden).** Saves about 3 s per repeat run on the maintainer's Mac (build 9.0 s, load 6.0 s); not timed on Windows. Hidden from the UI since 2026-09-25 by `index_cache::ENABLED`. See NOTES "Database cache" for how to ship it.

#### sageRecon integration

- [ ] **Port rollup scripts**: The peptide→protein rollup and LFQ aggregation scripts currently live in a separate project (not sageRecon). Action item: locate, read, and refactor them into a form SageGUI can call. (See Phase 6 for the GUI surface.)
- [ ] **Digestion Efficiency Report**: Port from [sageRecon](https://github.com/usnistgov/sageRecon): missed cleavages, semi-tryptic peptides, N/C ragged ratio.
- [ ] **Delta Mass Explorer**: Port from sageRecon: modification distribution from open search.
- [ ] **Link to sageRecon**: "Analyze with sageRecon" button for deeper analysis. The repo moved to `usnistgov/sageRecon` and was renamed from sagePreview.

---

### Phase 6: output formatting and downstream export (planned)

**Goals:** Let users get FDR-filtered protein/peptide tables and export to the formats their downstream tools expect. The rollup logic (peptide→protein at a specified FDR) comes from the scripts ported in Phase 5.

#### FDR-filtered rollup export

- [ ] **Peptide-level export at specified FDR**: User sets FDR threshold (default 1%); export filtered `results.sage.tsv`.
- [ ] **Protein-level rollup export**: Apply rollup scripts to produce a protein-level intensity table at the specified FDR.

#### Format spoofing for downstream tools

*(Reconciled 2026-08-24 against README.md's "To be added" list. README had
several targets PLAN didn't yet track. This is now the complete list; keep
both in sync going forward, per AGENTS.md.)*

Each of these requires understanding the target format and confirming Sage's output contains the required fields. Research is an action item per format before implementing.

- [x] **pepXML and mzIdentML export.** Built in SageGUI (not psm-utils, not shic) and shipped in `nist-v0.9.0`. mzIdentML 1.1.1 passes `xmllint` against the XSD. pepXML 1.23 passes except `search_engine="Sage"`, which the schema does not list (deliberate). pyteomics and psims read a real 9,392-entry output with zero errors. **Still open:** a test in a real target tool (Scaffold, Skyline, PeptideShaker or TPP); whether readers accept `MS:1001412` and `MS:1001413` in delta-mass terms; multi-file and multi-rank runs. See NOTES "Built here: mzIdentML 1.1.1 and pepXML 1.23 converters" and "Independent-reader verification"
- [ ] **MSstats**: **being built upstream, not by us.** The preferred path worked: [MSstatsConvert #143](https://github.com/Vitek-Lab/MSstatsConvert/issues/143) is in active development by @swaraj-neu, reading `lfq.tsv` rather than `results.sage.tsv` (Sage's `ms2_intensity` is a discriminant feature, not a quant channel). Planned defaults: `spectrum_q` at 0.01, `rank == 1`, drop `label == -1`, `ProteinName` from `proteins`, keep inline modification tags. **Done on our side:** SageGUI defaults `combine_charge_states` to true, which makes Sage write charge -1 and `PrecursorCharge` meaningless for MSstats. Documented in README → Downstream tools, and the Quant tab checkbox has a hover note (commit de1452b, 2026-09-21).
- [ ] **three-layer-ms1 report (idea, 2026-09-21)**: sageRecon's `_dev/extracted/three-layer-ms1` splits MS1 signal into non-peptidic, never sampled, sampled but not identified, and identified. Verified: it needs the mzML files, and Sage's HTML report has no MS1 TIC, so it cannot extend that report. Sage makes the report only during a search, from in-memory data, and there is no standalone function to remake it from `results.sage.tsv`. So this would be a port that reads `results.sage.tsv`, `results.json` and the mzML files, and writes its own page. Not started.
- [ ] **Perseus-format export**: for [Perseus](https://maxquant.net/perseus/) and [ProteoPlotter](https://github.com/JGM-Lab-UoG/ProteoPlotter). **Parked by maintainer decision (2026-09-21).** Researched: ProteoPlotter needs a Perseus-processed `.txt` with `#!{Type}` and `#!{C:Grouping}` rows and t-test columns, so a raw Sage table cannot feed it. A reformat-only peptide table from `lfq.tsv` is the safe first step. Unverified until test-loaded in Perseus: `#!{Type}` handling on a generic upload, and NaN and 0 handling. Details in NOTES "Perseus and ProteoPlotter".
- [ ] **DIAgui-compatible export**: for [DIAgui](https://github.com/mgerault/DIAgui). Format/column requirements not yet researched.
- [ ] **FragPipe Analyst / LFQ-Analyst / *-Analyst export**: [LFQ-Analyst](https://github.com/MonashBioinformaticsPlatform/LFQ-Analyst), FragPipe-Analyst, and the other tools under the [*-Analyst suite](https://analyst-suites.org/) likely share a common input shape. Identify required format; map Sage output columns.
- [x] **PDV import**: **done upstream, not by us.** [PDV v2.7.0](https://github.com/wenbostar/PDV/releases/tag/v2.7.0) (2026-08-14) reads `results.sage.tsv` with its mzML/mgf files, handles gzipped spectra, and can filter decoys and hits above 1% q-value on import. Nothing to build here. Listed under README → Downstream tools.
- [ ] **Scaffold-compatible export (?)**: Scaffold ingests pepXML or mzIdentML (see above), so this may fall out of that work rather than needing a dedicated exporter. Still marked uncertain (README: "Scaffold (?)"). Confirm Scaffold's actual import requirements before committing effort here.

**Note on scope:** Format export is "spoof where we have the data, document gaps where we don't." We won't invent data that Sage doesn't produce. Where an upstream tool already has an open feature request for Sage support (MSstatsConvert #143, PDV #110), **contributing there may be less total work and more durable than a parallel SageGUI-side exporter**: worth a real "build vs. contribute upstream" decision per format before implementing, not just defaulting to building our own.

#### iBAQ and other LFQ options

Added 2026-08-24 from README's "To be added" list. **Concrete starting point found during the settings-persistence
audit the same day:** `sage_core::lfq::LfqSettings` (used internally by
`QuantType::Lfq`) already has `peak_scoring`, `integration`,
`mobility_pct_tolerance`, and `peptide_q_value` fields, but only
`ppm_tolerance`, `spectral_angle`, and `combine_charge_states` have UI
widgets (`QuantType::update_section`, `src/ui.rs`). The other four are
never user-editable. Worse, `peak_scoring`/`integration` aren't even read
from the stored `LfqSettings` at launch: `From<QuantType> for QuantOptions`
(`src/ui.rs`) hardcodes `PeakScoringStrategy::Hybrid` and
`IntegrationStrategy::Sum` regardless of what's in the struct. iBAQ itself
(intensity-based absolute quantification: sum of peptide intensities
divided by the number of theoretically observable tryptic peptides for a
protein) isn't a `LfqSettings` field at all; check whether Sage computes it
internally anywhere, or whether this needs a rollup-script-style post-
processing step (ties to the Phase 5 "Port rollup scripts" item above).
Research needed before implementing: what `peak_scoring`/`integration`
options Sage actually supports and what tradeoff each represents (worth
tooltips, matching the existing pattern for other advanced fields), and
whether iBAQ is a Sage-side computation or a downstream one.

---

## Future phases (not planned yet)

### Distribution Improvements (Future)

#### macOS Code Signing

**Decision 2026-09-10: no Apple Developer account for now.** The maintainer chose not to pay for the Apple Developer Program. The app stays un-notarized, so macOS blocks a browser download.

- **What users see:** "Sage Launcher.app is damaged and can't be opened." Not "unidentified developer". Right-click then Open does not help for this message. The prominent button is Move to Trash, which deletes the user's copy; the maintainer clicked it by accident on 2026-09-10.
- **Workaround, verified 2026-09-10:** `xattr -dr com.apple.quarantine "Sage Launcher.app"`. Documented in README.
- **Done in CI since v0.8.2:** the bundle is sealed with an ad-hoc signature, and `codesign --verify --deep --strict` runs on the bundle and on the extracted release zip. This fixes a bundle that was internally invalid. It does not get past Gatekeeper.
- **The real fix, if an account is obtained:** Developer ID signing with the hardened runtime, then `xcrun notarytool submit --wait` and `xcrun stapler staple`. Put the signing where CI now ad-hoc signs, in "Create macOS App Bundle", on `target/<triple>/release/Sage Launcher.app`. Check for a NIST institutional account before paying. See NOTES, macOS Gatekeeper.

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

Locked decisions and their rationale have moved to **NOTES.md → Design decisions (locked)**. That is now the single source of truth. Do not duplicate them here. For the dated sequence of when things were decided, see **JOURNAL.md**.

---

## Handoff for the next session

**Start here:** AGENTS.md, `_dev/dev_AGENTS.md`, this status block, NOTES.md (locked decisions and dead-ends), then the top of JOURNAL.md.

**State:** Phases 0 to 4 done. `nist-v0.9.0` is the latest release (2026-09-22). Phase 5 is largely done. Phase 6 has the mzIdentML and pepXML converters; the rest is planned.

**Open items, in order (after the status-block next actions):**

1. **Open the converter output in a target tool** (Skyline, PeptideShaker, TPP or Scaffold). An independent parser and a rescoring tool confirm both files are well-formed (NOTES "Independent-reader verification"). No specific importer has opened them yet.
2. **Release checks not yet done:** Windows SmartScreen on the unsigned `.exe`; the Intel macOS app on Intel hardware or under Rosetta; the macOS dialog text after sealing.
3. **SageGUI versus Sage defaults audit.** Assert it as an invariant over an empty Sage config.
4. **Locate the rollup scripts.** They are in a separate project (not sageRecon). Record language and structure in NOTES before Phase 6 is scoped.
5. **Phase 6 format survey.** Sample inputs for MSstats, LFQ-Analyst and Scaffold; which columns Sage already writes; record the gap analysis in NOTES.

**Watch out for:**
- **The precursor window is written backwards.** Read the STOP section at the top of AGENTS.md before touching any tolerance value. Raw JSON `[lower, upper]` is the delta-mass range negated and swapped.
- Do not re-add `build.rs` version detection or switch Sage to a branch or Git dependency. Both are dead-ends (NOTES).
- Do not remove `ExperimentType` or `reuse_cached_index` from saved state. Dropping a field makes eframe fail to load the whole saved blob, and the user loses every setting.
- `cargo build` prints one `internal_eq_trait_method_impls` future-incompatibility warning from `vendor/sage/crates/sage/src/enzyme.rs`. It is upstream code (NOTES "Pin Sage to a commit hash"). A future Rust release makes it a hard error; then it needs a vendored patch or a re-vendor.
- ThermoRawFileParser: the licence is clear (Apache-2.0), but the .NET runtime on Linux and macOS is an open question. Do the cross-platform spike first.
- TMT quantification is still untested. LFQ is the only validated path. `tmt11.json` says so in its description.
- A behaviour change must update README, CHANGELOG and MAINTENANCE in the same session.

**Key files:** `src/main.rs` (app state, run thread), `src/ui.rs` (all tab rendering, Info / Help), `src/sage_json.rs` (template and config import), `src/export/` (mzIdentML and pepXML converters), `src/convert_job.rs` (runs a conversion on a thread), `src/index_cache.rs` (database cache, hidden), `assets/templates/` (bundled configs), `src/version.rs` (Sage version constants), `vendor/sage/` (vendored Sage, with VENDORED.md and PATCHES.md), `.github/workflows/` (build, badges, actionlint).
