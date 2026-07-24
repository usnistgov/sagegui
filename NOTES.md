# SageGUI — Notes & knowledge base

Topical, not chronological. This is what you don't want to re-explain or
re-derive. Timeless reference + the reasoning behind decisions.

For chronological history, see `JOURNAL.md`. For the roadmap, see `PLAN.md`.

---

## Design decisions (locked)

### Option A — fork Sage, don't wrap it (locked)
- **What:** SageGUI embeds Sage as a Rust library dependency (via our fork `neely/sage`), rather than shelling out to `sage.exe` as a subprocess (that rejected approach is "Option C").
- **Why:** Tight integration — single-binary distribution and the ability to show real-time progress from inside the process. This was the user's preference.
- **Rejected:** Option C (subprocess wrapper generating a JSON config and calling `sage.exe`). Would decouple us from Sage's internal API, but loses single-binary distribution and in-process progress. Reconsider only if the fork-sync maintenance burden becomes too high (that's flagged as a possible Phase 8 in PLAN).
- **Consequence:** We accept the ongoing burden of keeping `neely/sage` in sync with upstream `lazear/sage`. See MAINTENANCE.md.

### egui/eframe GUI framework (locked)
- **What:** The GUI uses egui (immediate-mode) with eframe (native window wrapper).
- **Why:** Already working in Sebastian's original, and a solid choice for Rust GUIs.
- **Rejected:** Rewriting in another framework — no reason to.

### Single `main.rs` (locked, revisit-able)
- **What:** All GUI code lives in one `src/main.rs` (~1000 lines), plus `src/version.rs` for Sage version constants.
- **Why:** Keeps Sebastian's original structure; no need to refactor while it's working.
- Not deeply locked — fine to split if a feature makes the single file unwieldy.

### Pin Sage to a commit hash, not a branch (locked)
- **What:** `Cargo.toml` pins `sage-core`/`sage-cli` to `rev = "d74024df..."`, not `branch = "master"`.
- **Why:** Reproducible builds — prevents unexpected breakage when upstream changes. Update the rev deliberately per MAINTENANCE.md.

### Version sync via `src/version.rs` constants (locked)
- **What:** Sage version info lives in `src/version.rs` constants (`SAGE_VERSION`, `SAGE_COMMIT`, `SAGE_REPO`, `SAGE_UPSTREAM`), consumed at compile time.
- **Why:** Simpler than the originally-considered `build.rs` auto-detection.
- **Rejected:** `build.rs` that auto-detects the version from `Cargo.toml` — removed in favor of the plain constants. Do not re-add it.

---

## Intentional, not bugs
Things that look wrong but are correct. Do not "fix" these.

- **Default output directory is the current working directory.** Users set it explicitly in the GUI. (Smarter timestamped defaults are a *planned* Phase 5 improvement, not a bug to patch ad hoc.)
- **TMT quantification is untested.** Only LFQ has been validated with real data — TMT code paths are believed correct but need TMT-labeled data to confirm. Not a defect; a known coverage gap (see below).

---

## Known permanent / standing limitations

- **Coupled to Sage's internal API.** By design (Option A), a Sage update can break compilation. This is the accepted cost of embedding; the mitigation is MAINTENANCE.md, not a code change.
- **One search at a time.** Not a batch/queue system — that's an explicit non-goal (see PLAN). Batch processing is a possible far-future phase.
- **Not a results viewer beyond a basic summary.** Deep analysis is left to downstream tools / sagePreview.
- **macOS binaries are unsigned.** Triggers Gatekeeper "unidentified developer" warnings. Workaround documented in README (`xattr -d com.apple.quarantine`). Real fix (Apple Developer Program + code signing) is deferred — see PLAN "Future / Distribution".

---

## Dead-ends (do not re-explore)

- **Adding `lib.rs` to sage-cli ourselves** → unnecessary. Official Sage v0.15.0-beta.2 already ships `crates/sage-cli/src/lib.rs` exporting `input`, `output`, `runner`, `telemetry`. The original plan assumed we'd have to create it; we don't.
- **`build.rs` auto-version-detection** → replaced by plain `src/version.rs` constants. Don't re-add.
- **Tracking Sage by branch (`branch = "master"`)** → rolled back to a pinned `rev` for reproducibility.
- **Assuming v0.14.7 as the target** → we're on v0.15.0-beta.2 (current master at fork time). Don't downgrade expectations to v0.14.7's API.

---

## Reference

### Domain primer — what this project is

**SageGUI** is a graphical front-end for [Sage](https://github.com/lazear/sage), a fast Rust proteomics search engine. It lets users configure and run Sage searches without the command line: file selection (mzML + FASTA), parameter configuration, search execution with progress, and a basic results summary.

- **Original author:** Sebastian Paez (`jspaezp/sagegui`)
- **Our fork:** `neely/sagegui`
- **Sage engine fork:** `neely/sage` (from `lazear/sage`)

**What Sage does:** takes MS data (mzML) + a protein database (FASTA), matches experimental spectra to theoretical peptide fragmentation, and outputs peptide-spectrum matches (PSMs) with confidence scores. Known for being 10–100× faster than comparable tools at high sensitivity.

**Key search parameters exposed by the GUI:**

| Parameter | Description | Typical values |
|-----------|-------------|----------------|
| `precursor_tol` | Precursor-ion mass tolerance | 10–20 ppm (closed), ±500 Da (open) |
| `fragment_tol` | Fragment-ion mass tolerance | 10–20 ppm |
| `missed_cleavages` | Allowed missed enzyme cuts | 1–2 |
| `min_len` / `max_len` | Peptide length limits | 7–50 |
| `static_mods` | Fixed mods (e.g. carbamidomethyl on C) | always applied |
| `variable_mods` | Optional mods (e.g. oxidation on M) | searched combinatorially |

**Quantification:** isobaric labeling — TMT (6/10/11/16/18-plex) and iTRAQ (4/8-plex) — plus label-free (LFQ) from MS1 intensities. The GUI selects the scheme and MS level.

Term definitions live in `docs/GLOSSARY.md`.

### API changes reference (v0.14.7 → v0.15.0-beta.2)

The fixes applied when moving to v0.15.0-beta.2. Keep this as the worked example for future upgrades (MAINTENANCE.md has the full update procedure).

| Component | Change | Fix applied |
|-----------|--------|-------------|
| `EnzymeBuilder.restrict` | `Option<char>` → `Option<String>` | `.map(\|c\| c.to_string())` |
| `Builder` (database) | new fields | add `prefilter: None`, `prefilter_chunk_size: None`, `prefilter_low_memory: None` |
| `LfqOptions` | new fields | add `mobility_pct_tolerance: None`, `peptide_q_value: None` |
| `Input` | field renamed | `bruker_spectrum_processor` → `bruker_config: None` |
| `Input` | new fields | add `protein_grouping: None`, `protein_grouping_peptide_fdr: None`, `write_report: None` |
| `Runner::new` | signature change | `input.build().and_then(Runner::new)` → `let search = input.build()?; Runner::new(search, parallel.into())` |

Also removed: the `BrukerSpectrumProcessor` import (no longer needed).

### Gotchas discovered

| Gotcha | Details |
|--------|---------|
| **TMT plex bug** (fixed) | `main.rs` ~lines 421–423: TMT 16/18-plex were mapped to `Tmt11`. Fixed to `Tmt16`/`Tmt18` in commit a225481. |
| **Fragment tolerance bug** (fixed) | `main.rs` ~lines 720–726: switching tolerance type (ppm↔Da) wrote to `precursor_tol` instead of `fragment_tol`. Fixed in commit a225481. |
| **sage-cli lib target** | Official Sage *now* exposes `sage-cli` as a library (v0.15.0-beta.2+). Sebastian's older fork had to add `lib.rs`; we don't. |
| **`Kind` not hashable** | `sage_core::ion_series::Kind` doesn't implement `Hash`/`Eq` in official Sage — relevant if you touch ion-series collections. |
| **timsrust API drift** | `timsrust::readers::SpectrumReaderConfig` doesn't exist in newer versions — watch for this when touching Bruker/timsTOF paths. |

### Key files in the Sage fork

| File | Purpose |
|------|---------|
| `crates/sage-cli/src/lib.rs` | Exports `input`, `output`, `runner`, `telemetry` |
| `crates/sage-cli/src/input.rs` | `Input`, `LfqOptions`, `QuantOptions`, etc. |
| `crates/sage-cli/src/runner.rs` | `Runner::new()`, `Runner::run()` |
| `crates/sage-core/src/database.rs` | `Builder`, `EnzymeBuilder` |
| `crates/sage-core/src/lfq.rs` | LFQ options |

### Sage versions

| Version | Status | Notes |
|---------|--------|-------|
| v0.14.7 | old (Sebastian's) | what the original GUI used |
| v0.15.0-beta.2 | current | our version, commit `d74024df` |

### Test baseline (Phase 2)

The validated reference run — use to sanity-check regressions:
- **Data:** `B.naive_01steady-state.mzML.gz` + `UniProt-Human-UP000005640_canonical-2023_05.fasta` (from sagePreview testing).
- **Params:** precursor ±10 ppm, fragment ±10 ppm, trypsin (KR not P) 2 missed cleavages, static C+57.021, variable M+15.995, LFQ on.
- **Result:** 60,672 PSMs; LFQ worked; outputs `results.sage.tsv`, `lfq.tsv`, `results.json`.

### Related projects

| Project | Purpose | Location |
|---------|---------|----------|
| sagePreview | Reconnaissance tool using Sage (PTM discovery) | `C:\Users\ban\Documents\GitHub\sagePreview` · `github.com/neely/sagePreview` |
| sage (official) | The search engine | `github.com/lazear/sage` |
| sage (our fork) | Modified/pinned Sage | `github.com/neely/sage` |
| sagegui (Sebastian's) | Original GUI | `github.com/jspaezp/sagegui` |
| sagegui (ours) | This project | `github.com/neely/sagegui` |

### External reference material (from sagePreview)

Located at `C:\Users\ban\Documents\GitHub\sagePreview\reference-notes\`:

| File | Content |
|------|---------|
| `sage-online-docs.md` | Full Sage documentation (scraped) |
| `sage-config-and-gotchas.md` | Decoy handling, tolerance syntax, chimeric search |
| `unimod-decomposition.md` | Unimod matching strategy, ambiguity handling |
| `oxonium-ions.md` | Glycan diagnostic ions |
| `polymer-contaminant-ions.md` | Polymer series for contamination detection |
| `ptm-shepherd-methodology.md` | PTM-Shepherd approach reference |
| `mgf-mzml-intensity-differences.md` | Intensity handling notes |
| `MS1-intensity.md` | MS1 signal fate approaches |
| `digestion-efficiency-metrics.md` | Missed cleavages, semi-tryptic metrics |

Official Sage source also mirrored at `C:\Users\ban\Documents\GitHub\sagePreview\reference\sage\`.
