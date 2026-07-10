# Changelog

All notable changes to SageGUI will be documented in this file.

## [Unreleased]

### Changed
- **Upgraded Sage from v0.14.7 to v0.15.0-beta.2** — Major version bump with API compatibility fixes
- Updated repository links to point to `neely/sagegui`
- Added Sage engine version display in Info/Help section
- Pinned Sage dependency to specific commit hash for reproducibility

### Fixed
- TMT 16-plex and 18-plex were incorrectly mapped to TMT 11-plex
- Fragment tolerance type switching was updating precursor tolerance instead
- API compatibility issues with Sage v0.15.0-beta.2:
  - `restrict` field type changed from `Option<char>` to `Option<String>`
  - Added `prefilter`, `prefilter_chunk_size`, `prefilter_low_memory` fields to `Builder`
  - Added `mobility_pct_tolerance` and `peptide_q_value` to `LfqOptions`
  - Replaced `bruker_spectrum_processor` with `bruker_config` in `Input`
  - Updated `Runner::new()` signature to take `(Search, parallel)` arguments

### Removed
- `BrukerSpectrumProcessor` import (no longer needed with new API)

## [0.5.0] - Original Release (jspaezp)

Initial release by Sebastian Paez with:
- Basic GUI for Sage search configuration
- Support for mzML and Bruker .d files
- LFQ and TMT quantification options
- egui/eframe-based interface
