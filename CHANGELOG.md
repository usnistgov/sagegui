# Changelog

All notable changes to SageGUI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Default values in the tooltips** — hovering a numeric control now tells you its default, so nudging a slider and forgetting where it was is recoverable. Where every bundled template uses a different value from the default (Min Length, Min Matched Peaks, Max Variable Mods), the tooltip says both. The Experiment tab also notes that applying a template again restores every setting it covers.
- **Enzyme presets** — the Search tab has an Enzyme picker with 14 curated proteases: trypsin, trypsin/P, Arg-C, Asp-N, Asp-N/ambic, chymotrypsin, CNBr, Lys-C, Lys-C/P, Lys-N, pepsin-A, Trypchymo, Glu-C and Glu-C/DE. Ported from [sageRecon](https://github.com/usnistgov/sageRecon), whose source is Mascot's published enzyme list. A preset sets the cut rule only: missed cleavages, the length range and semi-enzymatic digestion are left alone. The picker names the current enzyme even if you set it by hand or by loading a template, and reads "Custom" when the rule is not one of the presets.
- **Cut side is now a labelled choice** — the old "C-Terminal" checkbox is a pair of buttons, "After the residue" and "Before the residue". Asp-N, Asp-N/ambic and Lys-N cut before their residue, which the checkbox gave no way to see.
- **Citation metadata** — a `CITATION.cff` file, so GitHub's "Cite this repository" control produces a reference for SageGUI in BibTeX or APA. The README gains a Citation section with the same reference. Cite the version you ran: the pinned Sage engine and the bundled templates move between releases.
- **NIST governance files** — `CODEOWNERS`, `CODEMETA.yaml` (indexing for the NIST Open Source Portal), and `fair-software.md` (NIST FAIR practices), matching the layout of [sageRecon](https://github.com/usnistgov/sageRecon).
- **Experiment templates** — the Experiment tab now has a working Templates picker with five bundled starting configurations: tryptic wide MS1 / tight MS2, tryptic tight, tryptic open, tryptic biofluid, and TMT 11-plex. Applying one replaces your search parameters. It never touches your selected files or output folder. This replaces the old Experiment Type dropdown, which stored its selection and changed nothing else.
- **Load settings from a Sage file** — point SageGUI at a `config.json` you gave the Sage command line, or at the `results.json` written into a past run's output folder, and it loads the search parameters. One reader handles both formats. Anything it could not apply is reported on screen, not dropped silently.
- **Re-select notes on import** — an imported file records where its own data lived. Those paths are never applied, because they can be from another machine. The import now lists what it left behind, says whether each path still exists here, and points at the tab where you pick it again.

### Changed
- **Tolerance windows are now shown as a delta-mass range** — the precursor and fragment controls read "Delta mass from X to Y", which is how the window is normally described: set 500 to find IDs carrying up to a +500 Da modification. Sage's own config files store this pair negated and swapped; the hover text says so, for anyone cross-checking a `config.json`. Only the display changed. What is saved and what is sent to Sage are unchanged.

### Fixed
- **Templates inherited settings from the template applied before them** — applying the biofluid template switched database chunking on, and applying another template afterwards left it on. The enzyme and the quantification method carried over the same way, so a tryptic template applied after the TMT one kept TMT quantification. Every template now states every setting it cares about, and a test checks that applying any template lands in the same place regardless of what came before.
- **The proline restriction was read differently from Sage** — a Sage file that leaves `restrict` unset inside its enzyme block means "no restriction" to Sage, but SageGUI kept whatever restriction was already set. Loading Michael Lazear's TMT config therefore ran trypsin where the Sage command line would run trypsin/P, with nothing on screen showing the difference. SageGUI now resolves it exactly as Sage does.
- **A settings error stayed on the run bar after it was fixed** — correcting an invalid Cleave At residue left the red error message in place. Pre-flight errors now clear as soon as the cause is gone.
- **A failed search could leave the run bar spinning forever** — if anything in the search panicked, the progress bar kept animating and the elapsed time kept climbing, with no error, no output, and no way to recover except restarting the app. The run thread now reports its own failure instead of relying on the message channel to notice. On Windows the message had been hidden entirely.
- **An invalid character in "Cleave At" hung the app** — Sage rejects anything that is not one of its accepted amino acids, and it did so in a way the interface could not see. Enzyme residues are now checked before a run starts, and a bad character is reported on the run bar with the list of accepted residues.
- **Run-bar "Processing" label used a hardcoded pure green** — switched to a plain label so it inherits the theme's normal text color, matching the elapsed-time label next to it.

## [0.7.1] - 2026-08-24

### Added
- **App icon** — the crab-wizard mascot cropped from the existing logo now appears as the window/taskbar icon (`assets/icon-256.png`, via `eframe::ViewportBuilder::with_icon`) and the macOS app bundle icon (`assets/AppIcon.icns`), replacing eframe's default "e on black" placeholder. A Windows `.ico` was also built (`assets/AppIcon.ico`) but isn't wired into the `.exe` file icon yet — only the running-app icon is fixed so far.
- **Real run-bar progress** — the progress bar now shows the actual fraction of spectra scored (live count from Sage / pre-scanned mzML total), instead of a static placeholder. Status text also now names the current phase ("Building peptide database…" / "Reading and searching spectra…") so the quiet build phase doesn't read as frozen.
- **Tolerance Lower/Upper labels** — Da and ppm precursor/fragment tolerance fields now show "Lower" and "Upper" labels with hover text explaining the sign convention.
- **Inverted-window warning** — a non-blocking ⚠ label appears when the lower bound exceeds the upper bound (empty search range).
- **Database prefiltering controls** — `prefilter`, `prefilter_chunk_size`, and `prefilter_low_memory` are now exposed on the Files & Database tab, with a contextual hint when semi-enzymatic digestion is on. Bounds peak memory on semi-enzymatic/non-specific searches, large databases, or heavily modified searches, at the cost of extra CPU time. Defaults match Sage's own resolved defaults (off; low-memory mode on when enabled).
- **Settings persistence** — the GUI now remembers your configuration, tolerance-type selections, experiment archetype, and active tab between sessions (via eframe's `persistence` feature), auto-saving every 30 seconds and on exit. Closing the window to start over no longer loses your parameters. A bug where static/variable modifications didn't survive a restart (found in live testing 2026-08-24) is fixed and live-tested as of the same day — see Fixed below.
- **Stop button** — cancels a run from the run bar. Initially landed 2026-08-21 covering only the pre-search phase; extended 2026-08-24 with real cooperative cancellation of an in-progress search too (a `neely/sage` fork patch — see Fixed below and NOTES.md). Not an instant kill: a parallel scoring batch already in flight finishes its cheap remainder, and the FASTA-digest phase still can't be shortened once started. Live-tested 2026-08-24: a mid-scoring Stop aborted the search in 2.2s (vs. 60s+ to finish normally) and wrote zero output files.
- **Live Sage log panel** — a "Sage Log" group on Run/Info shows Sage's own log output as it happens, auto-scrolled, capped at 500 lines. No changes to the Sage fork needed — a wrapped logger forwards matching records onto the existing run-bar channel. Confirmed working in live testing, with a caveat: it stays empty for the first couple of minutes on a large database (Sage's own build-phase logging is sparse), matching the progress bar's known gap during that phase.

### Changed
- **Apache 2.0 attribution notices** — files derived from jspaezp/sagegui now carry a short notice at the top per Apache 2.0 §4(b).

### Fixed
- **Stop button false "no output written" message** — clicking Stop after a search had already passed its last cancellation checkpoint let the search finish and write output normally, but the run bar still reported "Search stopped. No output files were written." The completion handler now trusts the run's actual result instead of the Stop flag alone; the "stopped, no output" message only appears when the run genuinely aborted before finishing. Live-tested and confirmed against a real full search (2026-08-24).
- **Stop button couldn't actually interrupt an in-progress search** — `Runner::run()` in the pinned Sage fork had no cancellation check anywhere inside it, so Stop could only prevent a search from *starting*, never stop one already running. Patched `neely/sage` (commit `ed5f06c`) to check a shared cancel flag before the per-spectrum scoring call, between mzML file chunks, and once more right after scoring completes but before any FDR/protein-grouping/quant/output-writing step — so a cancelled run stops quickly and writes nothing. Live-tested and confirmed (2026-08-24).
- **Modifications didn't survive an app restart** — `StaticModConfig`/`VariableModConfig` keep a `#[serde(skip)]` live map (their key type has no `Deserialize`) alongside a serializable shadow map; the `sync_from_ser()` method meant to rebuild the live map after loading a saved config was defined but never called. The saved data was always correct — it just never made it back into the map the UI reads. Fixed in `SageLauncher::new`. Live-tested and confirmed (2026-08-24). Followed by a full field-by-field persistence audit (a new test round-trips every field of the saved config through the real restore logic) confirming no other field has a gap of this kind.
- **LICENSE link in README** — corrected from `LICENSE.md` to `LICENSE`.
- **`prefilter_low_memory` documented default** — `docs/PARAMETER_REFERENCE.md` previously stated `false`; Sage actually resolves it to `true`. Corrected, and the prefiltering section rewritten to cover the file-count re-read cliff and the per-chunk decoy caveat.
- **macOS: a terminal window opened alongside the GUI** — the release archive shipped a bare Mach-O binary with no `.app` bundle, so Finder/LaunchServices ran it via Terminal.app instead of launching it directly as a windowed app. CI now packages a proper `Sage Launcher.app` (bundle structure, `Info.plist`, icon) for both macOS builds. Verified locally by hand-building the exact CI recipe and launching it with `open`: zero Terminal windows opened, and confirmed on the real GitHub Actions release build for v0.7.1.

## [0.7.0] - 2026-08-13

### Added
- **Sidebar-navigation UI** — replaced the single scrolling page with six tabs
  (Experiment, Files & Database, Search, Modifications, Quant, Run / Info) and a
  **pinned run bar** at the bottom of every tab, so Run/status/elapsed stay
  visible regardless of the active tab. Each tab has a collapsible **Advanced**
  section for rarely-touched knobs.
- **Six previously-hidden Sage parameters now configurable** — precursor charge
  range, isotope errors, scoring function (`score_type`), override precursor
  charge, write Percolator `.pin`, and annotate matches. These were frozen at
  defaults before.
- **Save / Load configuration** — export and import the full search
  configuration as JSON (Experiment tab).
- **Modifications list-picker** — the Modifications tab is now a Mascot-style
  two-box (Static / Variable) + curated "Common modifications" master list with
  transfer arrows. Multi-residue presets (e.g. Phospho S/T/Y, Deamidation N/Q,
  Acetyl K + protein N-term) insert as separate editable rows in one click; a
  "+ Custom…" escape hatch keeps the free-type residue+mass entry. Static and
  Variable are mutually exclusive (adding a key to one removes it from the other).
  Presets include Oxidation (M) and Oxidation (P) as separate entries so
  hydroxyproline can be added on top of standard M oxidation. The master list is
  alphabetical; the "+ Custom…" panel shows a Sage key-syntax cheat-sheet; and a
  footnote notes that displayed Δmasses are rounded to 4 places while the full
  monoisotopic value is stored and used. Pyro-Glu presets spell out their
  specificity, e.g. "Glu->pyro-Glu (E, peptide N-term)".
- **Inline parameter tooltips** — hover any control for a short description.

### Changed
- UI code split out of `src/main.rs` into a new `src/ui.rs` module.
- **Output Location moved to the Run / Info tab** — the output-directory control
  now lives next to the run action instead of on Files & Database.

### Removed
- **Save / Load Config** — removed from v0.7.0. The Sage `results.json` /
  `settings.json` schema differs from SageGUI's internal config struct;
  a partial bridge would silently drop fields. Feature deferred pending
  schema-alignment design work. Placeholder note left on the Experiment tab.
- **Native Bruker `.d` file picker** — SageGUI now takes `.mzML` / `.mzML.gz`
  only. Convert other formats upstream.

## [0.6.0] - 2026-07-13

### Sage Engine
- **Sage Version:** [v0.15.0-beta.2](https://github.com/lazear/sage/releases/tag/v0.15.0-beta.2)
- **Commit:** [`d74024df`](https://github.com/neely/sage/commit/d74024df774054fa411a9d5cca6013ce91d26208)
- **Fork:** [neely/sage](https://github.com/neely/sage) (synced with upstream lazear/sage)

### Added
- **Version constants** — Sage version info stored in `src/version.rs` for easy updates
- **CI/CD pipeline** — GitHub Actions workflow for building on Windows, Linux, macOS (x64 and ARM64)
- **Release automation** — Automatic binary releases when tags are pushed
- **Version badges** in README showing Sage version and build status

### Changed
- **Upgraded Sage from v0.14.7 to v0.15.0-beta.2** — Major version bump with API compatibility fixes
- Updated repository links to point to `neely/sagegui`
- Sage engine version display now uses compile-time constant instead of hardcoded string

### Fixed
- TMT 16-plex and 18-plex were incorrectly mapped to TMT 11-plex
- Fragment tolerance type switching was updating precursor tolerance instead
- API compatibility issues with Sage v0.15.0-beta.2:
  - `restrict` field type changed from `Option<char>` to `Option<String>`
  - Added `prefilter`, `prefilter_chunk_size`, `prefilter_low_memory` fields to `Builder`
  - Added `mobility_pct_tolerance` and `peptide_q_value` to `LfqOptions`
  - Replaced `bruker_spectrum_processor` with `bruker_config` in `Input`
  - Added `protein_grouping`, `protein_grouping_peptide_fdr`, `write_report` to `Input`
  - Updated `Runner::new()` signature to take `(Search, parallel)` arguments

### Removed
- `BrukerSpectrumProcessor` import (no longer needed with new API)

### Tested
- Successfully processed 60,672 PSMs from single mzML file
- LFQ quantification verified working
- Output files generated: `results.sage.tsv`, `lfq.tsv`, `results.json`

---

## [0.5.0] - Original Release (jspaezp)

Initial release by Sebastian Paez with:
- Basic GUI for Sage search configuration
- Support for mzML and Bruker .d files
- LFQ and TMT quantification options
- egui/eframe-based interface

---

## How to Update Sage Version

When a new Sage version is released:

1. **Update the fork:**
   ```bash
   cd path/to/neely/sage
   git fetch upstream
   git merge upstream/main
   git push origin main
   ```

2. **Update Cargo.toml** in sagegui:
   - Change the `rev = "..."` to the new commit hash
   - Update the comment `# Pinned to vX.X.X`

3. **Fix any API changes** (check for new/changed fields in Input, Builder, etc.)

4. **Test and release** new sagegui version

[0.6.0]: https://github.com/neely/sagegui/releases/tag/v0.6.0
[0.5.0]: https://github.com/jspaezp/sagegui/releases/tag/v0.5.0
