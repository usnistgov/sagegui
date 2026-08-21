# Changelog

All notable changes to SageGUI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Real run-bar progress** — the progress bar now shows the actual fraction of spectra scored (live count from Sage / pre-scanned mzML total), instead of a static placeholder.
- **Tolerance Lower/Upper labels** — Da and ppm precursor/fragment tolerance fields now show "Lower" and "Upper" labels with hover text explaining the sign convention.
- **Inverted-window warning** — a non-blocking ⚠ label appears when the lower bound exceeds the upper bound (empty search range).

### Changed
- **Apache 2.0 attribution notices** — files derived from jspaezp/sagegui now carry a short notice at the top per Apache 2.0 §4(b).

### Fixed
- **LICENSE link in README** — corrected from `LICENSE.md` to `LICENSE`.

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
