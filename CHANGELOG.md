# Changelog

All notable changes to SageGUI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Real run-bar progress** — the progress bar now shows the actual fraction of spectra scored (live count from Sage / pre-scanned mzML total), instead of a static placeholder. Status text also now names the current phase ("Building peptide database…" / "Reading and searching spectra…") so the quiet build phase doesn't read as frozen.
- **Tolerance Lower/Upper labels** — Da and ppm precursor/fragment tolerance fields now show "Lower" and "Upper" labels with hover text explaining the sign convention.
- **Inverted-window warning** — a non-blocking ⚠ label appears when the lower bound exceeds the upper bound (empty search range).
- **Database prefiltering controls** — `prefilter`, `prefilter_chunk_size`, and `prefilter_low_memory` are now exposed on the Files & Database tab, with a contextual hint when semi-enzymatic digestion is on. Bounds peak memory on semi-enzymatic/non-specific searches, large databases, or heavily modified searches, at the cost of extra CPU time. Defaults match Sage's own resolved defaults (off; low-memory mode on when enabled).
- **Settings persistence** — the GUI now remembers your configuration, tolerance-type selections, experiment archetype, and active tab between sessions (via eframe's `persistence` feature), auto-saving every 30 seconds and on exit. Closing the window to start over no longer loses your parameters. **Known issue (found in live testing 2026-08-24): modifications do not persist** — needs investigation before this is reliable.
- **Stop button** — cancels a run from the run bar, but only if clicked before the search itself starts (database build/prefilter phase); a search already scoring spectra is not interrupted and runs to completion. This limitation is by design (hover text says so); see Fixed below for the false-completion-message bug found alongside it.
- **Live Sage log panel** — a "Sage Log" group on Run/Info shows Sage's own log output as it happens, auto-scrolled, capped at 500 lines. No changes to the Sage fork needed — a wrapped logger forwards matching records onto the existing run-bar channel. Confirmed working in live testing, with a caveat: it stays empty for the first couple of minutes on a large database (Sage's own build-phase logging is sparse), matching the progress bar's known gap during that phase.

### Fixed
- **Stop button false "no output written" message** — clicking Stop after a search had already passed its last cancellation checkpoint let the search finish and write output normally, but the run bar still reported "Search stopped. No output files were written." The completion handler now trusts the run's actual result instead of the Stop flag alone; the "stopped, no output" message only appears when the run genuinely aborted before finishing.

### Changed
- **Apache 2.0 attribution notices** — files derived from jspaezp/sagegui now carry a short notice at the top per Apache 2.0 §4(b).

### Fixed
- **LICENSE link in README** — corrected from `LICENSE.md` to `LICENSE`.
- **`prefilter_low_memory` documented default** — `docs/PARAMETER_REFERENCE.md` previously stated `false`; Sage actually resolves it to `true`. Corrected, and the prefiltering section rewritten to cover the file-count re-read cliff and the per-chunk decoy caveat.

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
