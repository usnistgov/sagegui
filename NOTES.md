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

## UI redesign (in progress)

Decided 2026-08-13. Moving from the single scrolling `CentralPanel` to a
**sidebar-nav + pinned run-bar** layout (inspired by MetaMorpheus's task-first
pattern). Design finalized; port planned. Full paste-in design spec and the
web-LLM proposal live in `docs/ui-spec.md`; the porter's handoff (ASCII mockups
+ YAML layout) was the source for the tab structure below.

### Agreed tab structure (6 tabs + pinned run bar)

1. **Experiment** — pick an archetype first (Custom / Tryptic LFQ / Wide-open /
   Phospho / Semi-tryptic) that writes defaults into the other tabs; Save/Load
   Config (JSON). Task-queue reserved but deferred.
2. **Files & Database** — Data (output dir, mzML picker, picked list) +
   Database sub-header (FASTA multi-file, cRAP toggle; ▸ Advanced: generate
   decoys, bucket size, min_ion_index, decoy_tag). *(No standalone Database
   tab — it dissolved into here + Search.)*
3. **Search** — precursor/fragment tolerance; Charge Handling (precursor charge
   range); Enzyme Settings; Mass Ranges; Ion Kinds; Search behavior
   (isotope_errors, deisotope, chimera, wide_window — all **visible**); Scoring
   (score_type, visible); ▸ Advanced (light): override_precursor_charge,
   spectrum filtering (min/max peaks, min matched peaks, max fragment charge),
   report_psms, predict_rt.
4. **Modifications** — own tab. This port relocates the current static/variable
   add-list-remove editors unchanged + max_variable_mods (visible, **not**
   advanced). Redesign pinned below.
5. **Quant** — standalone thin tab (enable, LFQ/TMT, per-type settings). Grows
   later (LFQ mobility tol, TMT S/N).
6. **Run / Info** — Output Options (write_pin, annotate_matches); Launch;
   status; results summary (planned); export buttons (planned); Info/Help.

Pinned **Run Bar** (`TopBottomPanel::bottom`) renders every frame regardless of
active tab — the single most important structural change (today's Launch button
scrolls away). "Advanced" now means a **per-tab collapsing sub-section**, not a
page — so there is no "Advanced" tab.

### The 6 previously-hidden Config fields — now surfaced

These were in `Config`/`Input` but had **no widget** (frozen at defaults, user
could not change them). Homes assigned:

| Field | Was | New home | Widget |
|-------|-----|----------|--------|
| `precursor_charge` (2,4) | hardcoded | Search › Charge Handling (visible) | range slider |
| `override_precursor_charge` (None, `//TODO`) | stub | Search › ▸ Advanced | checkbox "Force charge range" |
| `isotope_errors` (-1,3) | hardcoded | Search › Search behavior (visible) | range slider + trade-off tooltip |
| `score_type` (SageHyperScore) | hardcoded | Search › Scoring (visible) | ComboBox + conservative tooltip |
| `write_pin` (false) | hardcoded | Run/Info › Output Options | checkbox (Percolator .pin) |
| `annotate_matches` (false) | hardcoded | Run/Info › Output Options | checkbox |

Trade-off tooltip for `isotope_errors`: Sage docs note isotope-error search is
*slower* than simply widening `precursor_tol` to cover the same mass range, and
a wider window generally IDs more PSMs — so prefer wide Da tolerance when in
doubt. `override_precursor_charge` forces the `precursor_charge` range to be
searched instead of trusting the file's charge annotation (relevant for
DIA/diaPASEF).

### Design pins — dedicated later sessions, NOT part of the layout port

- **Modifications editor redesign.** ✅ Shipped 2026-08-13 (differs from the
  originally-pinned dual-pool transfer-list). Landed as a **Mascot-style
  list-picker**: two destination boxes (Static / Variable) on the left, a
  curated **"Common modifications"** master list on the right, and ◀ Add /
  Remove ▶ arrows that act on whichever box a **Target** toggle selects. The
  open "per-amino-acid specificity" problem was resolved by **multi-key presets
  that insert as separate editable rows** — e.g. "Phospho (S/T/Y)" adds three
  independent rows (S, T, Y), so a user can then drop one residue without the
  list exploding. A "+ Custom…" collapsing panel keeps the old free-type
  residue+mass entry for anything not in the curated set. Static/Variable are
  **mutually exclusive** (adding a key to one removes it from the other).
  Presets live hardcoded in `src/ui.rs` `MOD_PRESETS` (masses = Unimod
  monoisotopic deltas — [[deamidation-mass-doubt]] resolved: 0.984016). To
  extend the list, edit that const. **Still open:** the Experiment archetypes
  (Phospho etc.) don't yet auto-populate mods; the picker is manual.
- **Multi-FASTA UX.** Files & Database should make it easy to add and select
  *multiple* FASTA files (target + contaminants + spike-ins), concatenated
  before Sage. Worth real design effort — not just a repeated file picker. Pairs
  with the built-in cRAP toggle.

### Scope decision: mzML/.gz only — drop native `.d` / `.raw`

Decided to **not** add native Bruker `.d` or Thermo `.raw` reading. The GUI
takes `.mzML` / `.mzML.gz` only; users convert upstream. Consequence for the
port: the `.d` picker, `dotd_paths`, and `bruker_config` plumbing become dead
code to strip. (This supersedes the earlier Phase 5 "ThermoRawFileParser
integration" item and the `.d` picker in current `main.rs`.) The Bruker/timsrust
gotchas in the reference section stay documented for maintenance history but are
no longer a feature target.

### Custom modifications — persistence (open design question, 2026-08-13)

The Modifications tab has a **"+ Custom…"** panel: raw Sage key + Δmass added
directly to the targeted box. **Current behavior:** a custom mod lives in the
config's `static_mods_ser` / `variable_mods_ser` maps, so it **round-trips
through Save/Load Config JSON** — but it does *not* join the "Common
modifications" master list and is invisible to the picker on the next launch.
So custom mods are remembered *per-config-file*, not *per-machine* and not
*in the picker*.

Collaborator flagged (2026-08-13) that a **persistent, picker-visible** custom
mod would be nicer, but wasn't sure how they'd be handled/remembered — so this
is deferred to a spec-first design session. Options captured:

1. **Session-only (status quo).** Custom = this-search-only; persists only via
   Save/Load Config. Curated list stays code-controlled (`MOD_PRESETS`).
2. **Persist to a user file.** Append user customs to e.g.
   `%APPDATA%/sagegui/custom_mods.json` (Windows) / XDG-equivalent, loaded at
   startup and merged into the picker so they reappear every launch. Needs:
   storage format, load-merge-with-`MOD_PRESETS` logic, and edit/delete UI.
3. **Add-to-list, session-only.** "Save as preset" appends to the in-memory
   master list (visible in the picker until the app closes). No file.

**Open sub-questions to resolve before building option 2/3:** dedup rule
(same key+mass? same label?); can a user *delete/rename* a persisted custom;
does it show a "(custom)" tag vs curated entries; multi-key customs (the
current panel only does single-key — a real "Phospho-like" custom needs the
multi-row model); and where the file lives cross-platform (use the `dirs`/
`directories` crate for the config dir rather than hand-rolling `%APPDATA%`).
Pairs with the still-open "remember last-used settings across sessions" item
(PLAN Phase 5) — both want the same per-machine config-dir plumbing, so build
that once.

Collaborator ran the baseline search through the new 6-tab GUI. It worked
(60,672 PSMs — see Test baseline). Follow-ups to address in the next UI session,
roughly ordered:

1. **Verify Load Config actually populates all tabs.** Does loading an
   experiment JSON truly overwrite every field across all tabs (not just the
   Experiment tab's own state)? And after loading, can you then Save the current
   (possibly-edited) config back out? Round-trip both directions and confirm.
   *(The save/load code replaces `self.config` wholesale, so it should — but this
   was never observed working end-to-end; treat as unverified.)*
2. **Move Output Location control to the Run tab.** ✅ Done 2026-08-13 — the
   Output Location group now lives on the Run / Info tab (above Output Options);
   removed from Files & Database. *(Still relates to the open "smarter output
   directory" item — default is still cwd.)*
3. **Rework the Run / Info screen.** The Info/Help block (author, repo, license,
   citation, versions) doesn't need to occupy the run screen — but don't delete
   that content, just re-home it (an About dialog/collapsing, or a footer). The
   freed space would be better as a **live console readout of what Sage is
   doing** (its log/progress stream) — pairs with the pending async-progress work.
4. **License is wrong / missing.** `Cargo.toml` says `Apache-2.0`, README links a
   `LICENSE` file that **does not exist**, and the GUI credits "Apache-2.0".
   Upstream Sage is **MIT** (© 2022 Michael Lazear). **Blocked pending
   discussion with Sebastian** (2026-08-13) — the maintainer wants to talk to the
   original author before choosing MIT vs Apache-2.0. Do not add the LICENSE file
   or change Cargo.toml/README/GUI until that conversation resolves.
5. **Run-bar progress bar.** Confirmed it renders and animates a "% change" fine,
   but it's a **placeholder** — not wired to real search progress. Real
   step/percent reporting is the pending Phase 5 async-progress item.
6. **Experiment archetypes are inert (confirmed 2026-08-13).** Collaborator
   confirmed that changing the Experiment-tab dropdown (Custom / Tryptic LFQ /
   Wide-open / Phospho / Semi-tryptic) does **nothing** to the settings on the
   other tabs. Currently `self.experiment` is only stored + shown as the combo's
   selected text ([src/ui.rs](src/ui.rs) `page_experiment`); there is **no
   apply-archetype logic** that writes defaults into `self.config`. Needs: an
   `apply_archetype(&mut self)` that, on selection change, sets the relevant
   Search/Modifications/Quant fields (e.g. Phospho → add S/T/Y phospho variable
   mods via `MOD_PRESETS`, drop Ox M; Wide-open → widen `precursor_tol`;
   Semi-tryptic → set `semi_enzymatic`). Ties into the "wire archetypes →
   MOD_PRESETS" note under the Modifications-editor pin above. **Decide:** does
   selecting an archetype *overwrite* user edits, or only seed defaults once?
7. **Theme is too low-contrast (2026-08-13, next-session request).** The default
   egui dark-grey-on-grey is too faint for readability. Collaborator wants a
   **high-contrast "Y2K"-style theme** (bright, chunky, high-contrast — think
   late-90s/early-2000s desktop UI: solid button bevels, saturated accent, black
   text on light). Scope for a dedicated session: build a custom `egui::Visuals`
   (widget fill/stroke, `override_text_color`, larger default font, stronger
   `widgets.*` contrast) applied at startup via `ctx.set_visuals` /
   `set_style`; optionally a light/Y2K toggle. Pure styling — no behavior change.
   Reference the mockup the collaborator shared (flat high-contrast panels,
   visible borders). This is a **next-session** item, not built yet.

---

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

### Divergent `v0.7.0-alpha.*` tags (not our work — deleted locally)

Local tags `v0.7.0-alpha.1`, `v0.7.0-alpha.2`, `0.7.0-alpha.1`, `0.7.0-alpha.2`
pointed at commits from **Sebastian's line** (`jspaezp`), not our `main`. They
branched from merge-base `69096ab` (2024-11-11) and were **never on our `main`**
nor pushed to our `origin`. Deleted locally 2026-08-13 to keep the tag list
reflecting only our releases (`v0.6.0` line). We do **not** use `v0.7.0` as our
next version — our next tag continues our own sequence.

Their Sage dep pointed at `jspaezp/sage` (`rev 9271e28d`, an "lfq branch"), a
different fork than ours (`neely/sage`, `d74024df`) — so their pins are not
directly reusable.

**Worth harvesting** (ideas only, verify against current code before adopting):
- **mimalloc global allocator** (`#[global_allocator]`) — reported to fix poor
  Windows performance. Cheap win; consider for a future perf pass.
- **Bruker MS1 centroiding config** — `BrukerProcessingConfig { ms1:
  BrukerMS1CentoidingConfig { mz_ppm, ims_pct }, ms2: ... }` replacing the old
  `bruker_spectrum_processor`. Relevant to the Phase 5 "Bruker configuration"
  item; confirms the newer `sage-cloudpath::tdf` API shape.
- **LFQ mobility % tolerance** surfaced in the UI (we already added the field to
  `LfqOptions` as `None`; they wired a `DragValue` for it).

### Sage parameter notes

Explanations of Sage config knobs the GUI exposes — for building inline
descriptions/tooltips (Phase 5 "parameter documentation" item) and for
answering user questions. Sourced from Sage's docs.

#### `bucket_size` — performance only, not accuracy

A pure performance-tuning knob in Sage's database config block. **It does not change identification results, only how fast the search runs.**

- **What it controls:** Sage builds a fragment-ion index by grouping theoretical fragment ions into mass-sorted "buckets." `bucket_size` = how many fragment ions per bucket. Sage rounds up to the next power of 2, so effective values are 8192, 16384, 32768, 65536, etc.
- **The tradeoff (speed):**
  - *Small buckets* (finer granularity) → fewer buckets scanned per high-res fragment match. Fast for Orbitrap / high-res MS2.
  - *Large buckets* → each bucket spans a wider mass range, suiting low-res instruments with bigger fragment mass errors (with small buckets you'd waste time checking many adjacent buckets).
- **Recommended by MS2 resolution:**

  | MS2 resolution | Suggested `bucket_size` |
  |----------------|-------------------------|
  | High (Orbitrap) | 8192 (the minimum allowed) |
  | Low (ion trap) | 65536 (starting point) |

  Start there and tune empirically — optimal value depends on fragment tolerance and dataset. (The general config page shows 32768 as a middle-ground illustrative default, but the database-config page's table above is more authoritative.)
- **Bottom line:** a speed dial. Pick by MS2 resolution; try a few values if you want to squeeze performance. Zero effect on PSM results.

#### Variable / static modification syntax

All mods live under `variable_mods` / `static_mods` in the `database` section of `config.json`. Sage uses prefix syntax for position:

| Prefix | Meaning |
|--------|---------|
| `^X` | on residue X only at **peptide N-terminus** |
| `$X` | on residue X only at **peptide C-terminus** |
| `[X` (or bare `[`) | on residue X (or any) at **protein N-terminus** |
| `]X` (or bare `]`) | on residue X (or any) at **protein C-terminus** |
| `X` (bare residue) | anywhere on residue X |

`max_variable_mods` caps how many variable mods co-occur on one peptide — **defaults to 2**, so bump it when stacking several.

Standard mod set a collaborator expects as presets (feeds the Phase 5 preset library):

| Mod | Encoding | Mass |
|-----|----------|------|
| Oxidation on M (`oxM`) | `"M": [15.9949]` | +15.9949 |
| Oxidation on P (`oxP`, skin/collagen) | `"P": [15.9949]` | +15.9949 |
| Pyro-Glu, peptide N-term Q | `"^Q": [-17.026549]` | −17.026549 |
| Pyro-Glu, peptide N-term E | `"^E": [-18.010565]` | −18.010565 |
| Deamidation N/Q | `"N": [0.98402]`, `"Q": [0.98402]` | +0.98402 |
| Acetyl, protein N-term | `"[": [42.0106]` | +42.0106 |
| **Fixed:** Carbamidomethyl on C | `static_mods` `"C": 57.0215` | +57.0215 |

Caveat: an old Sage changelog notes a historical bug where variable protein-terminal mods (e.g. N-term acetyl) could give nondeterministic results, later fixed — use a reasonably current release (we're on v0.15.0-beta.2, safe).

### Test baseline (Phase 2)

The validated reference run — use to sanity-check regressions:
- **Data:** `B.naive_01steady-state.mzML.gz` + `UniProt-Human-UP000005640_canonical-2023_05.fasta` (from sagePreview testing).
- **Params:** precursor ±10 ppm, fragment ±10 ppm, trypsin (KR not P) 2 missed cleavages, static C+57.021, variable M+15.995, LFQ on.
- **Result:** 60,672 PSMs; LFQ worked; outputs `results.sage.tsv`, `lfq.tsv`, `results.json`.
- **Post-restructure regression check (2026-08-13):** re-ran this exact baseline through the new 6-tab GUI (debug build, commit `6712bb1`) → **60,672 PSMs again** (identical). Confirms the sidebar restructure + `From<Config> for Input` remap + serde shadow-field workarounds preserved search behavior. Use this count as the known-good comparison for future UI changes. *(Output landed in `target/debug/` because the output-location default is cwd — see UI-review pin about moving that control.)*

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
