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

### Custom patch carried on `neely/sage`: `Runner.progress` counter (2026-08-21)
- **What:** `neely/sage` `master` (merged PR #1, commit `cf20b75b`, on top of
  `d74024df`) carries a small hand-written patch, not from upstream
  `lazear/sage`: a `pub progress: Arc<AtomicUsize>` field on `Runner`
  ([crates/sage-cli/src/runner.rs](https://github.com/neely/sage/blob/master/crates/sage-cli/src/runner.rs)),
  incremented once per MSn spectrum scored inside `search_processed_spectra`,
  alongside the pre-existing local rate-logging counter. A caller can clone the
  `Arc` before calling `runner.run(...)` and poll `.load(Ordering::Relaxed)`
  from another thread — this is what feeds the real run-bar progress bar.
- **Why:** `sage-cli`'s public API (`lib.rs` exports `input`, `output`,
  `runner`, `telemetry`) has no progress-reporting hook at all — `Runner::run()`
  is one opaque blocking call. The atomic counter Sage already computes
  internally was private to that function. Editing `neely/sage` directly
  (rather than scraping log output) was a deliberate choice — see PLAN/NOTES
  progress-bar discussion 2026-08-21 — because Sage releases infrequently, so a
  small, isolated, additive diff is a bounded, occasional cost, not open-ended
  maintenance burden.
- **Consequence for the next Sage sync (MAINTENANCE.md Step 1):** when you
  `git merge upstream/main` into `neely/sage`, this patch must survive the
  merge. It touches: the `Runner` struct definition, both `Self { ... }`
  literals in `Runner::new` (including the `mini_runner` prefilter path), and
  one line inside `search_processed_spectra`'s `.map()` closure. If upstream
  has rewritten any of those exact spots (this file has changed before — see
  the `Runner::new` signature change and the TMT/fragment-tolerance bugs in the
  Gotchas table below), git will flag a merge conflict there — re-apply the
  4-line addition by hand (diff is in PR #1 on `neely/sage` for reference) and
  re-verify with `cargo check` before re-pinning `sagegui`. If it's clean, no
  action needed beyond noting the new commit hash.
- **How fork patches are verified (corrected 2026-08-21):** `neely/sage` itself
  has never had a GitHub Actions run. That matters less than an earlier note in
  this file implied. `sage-core`/`sage-cli`/`sage-cloudpath` are **git
  dependencies** in `Cargo.toml`, so cargo compiles the fork's source as part of
  every `sagegui` build. `sagegui`'s own `.github/workflows/build.yml` runs
  `cargo clippy -- -D warnings`, `cargo test` and `cargo build --release` on
  Windows, Linux, macOS x64 and macOS ARM64. A fork patch that does not compile
  on any of those platforms fails our CI. There is no separate fork CI to
  arrange.
  **What is still not covered:** Sage's own test suite (our `cargo test` runs
  `sagegui`'s tests, not a dependency's), and clippy, which does not lint
  dependency code. For a small additive patch — a `pub` field, an atomic load —
  compilation across four targets is close to the whole risk surface. The
  residual risk is behavioural, and the check for that is re-running the Phase 2
  baseline by hand (see Test baseline).

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
- **Multi-FASTA UX.** ✅ Shipped 2026-08-13. Files & Database tab is now an
  add/remove list. User adds target + contaminant FASTAs in order; a single-file
  search bypasses the temp copy. **cRAP design decision:** no bundled file —
  user supplies whatever cRAP/contaminant FASTA they prefer, just as another
  list entry. Dedup of identical FASTA headers: not done, just concatenate
  (documented decision).

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

1. **Save / Load Config — deferred (2026-08-13).** The Sage `results.json` /
   `settings.json` schema differs from SageGUI's `Config` struct (e.g.
   `enzyme.restrict` vs `enable_restrict`/`restrict_char`, `ion_kinds` as array
   vs HashMap, extra Sage-only fields). Rather than writing a partial bridge
   that silently drops fields, the feature was **removed from v0.7.0**. The
   Experiment tab now shows a placeholder note. Design work needed before
   re-implementing: audit every field difference between SageGUI's `Config` and
   Sage's `Input`/`database` JSON shapes, then decide whether to (a) add a
   dedicated Sage-JSON importer that maps fields explicitly, or (b) align
   SageGUI's serde output with Sage's schema end-to-end. Pairs with the
   "remember last-used settings" item.
2. **Move Output Location control to the Run tab.** ✅ Done 2026-08-13 — the
   Output Location group now lives on the Run / Info tab (above Output Options);
   removed from Files & Database. *(Still relates to the open "smarter output
   directory" item — default is still cwd.)*
3. **Rework the Run / Info screen — log panel done, Info/Help re-home still
   open.** The Info/Help block (author, repo, license, citation, versions)
   still occupies the run screen and hasn't been re-homed — that half is
   unchanged. **The live console readout is done (2026-08-21):** a "Sage Log"
   group above it shows Sage's own `info`-level log output live, in a
   `stick_to_bottom` scroll area, capped at 500 lines (`MAX_LOG_LINES`), no
   fork changes. Mechanism: `GuiLogger` ([src/main.rs](src/main.rs)) wraps the
   normal `env_logger::Logger` — still prints to stderr exactly as before —
   and additionally forwards any record whose target starts with `sage_` onto
   the same `mpsc` channel already used for run-bar messages, as a new
   `ThreadMessage::LogLine`. The sender is registered into a process-global
   `LOG_SENDER` slot at the start of each run (a fresh `Sender` each time,
   since `log::set_boxed_logger` is a one-time global install at startup, long
   before any run exists) and cleared on completion; the line buffer itself
   persists after a run so it can be reviewed, and clears on the next Run.
   **Gotcha found building this:** Sage's own crates' actual lib names are
   `sage_cli`/`sage_core`/`sage_cloudpath` (confirmed via `sage-cli`'s
   `[lib] name = "sage_cli"` and Cargo's default hyphen→underscore rule for
   the other two, which have no `[lib]` override). `env_logger`/`env_filter`
   directive matching requires an exact target match or a `::` boundary, so
   stock Sage CLI's own default filter — literally `"sage=info"`, see
   `sage-cli/src/main.rs` — **does not match any of them** and is effectively
   a no-op; Sage's official CLI likely shows only `error`-level output by
   default, less than it appears to intend. `sagegui`'s logger avoids the same
   bug by naming all three crates explicitly:
   `"error,sage_cli=info,sage_core=info,sage_cloudpath=info"` (still
   overridable via `RUST_LOG`).
   **Confirmed working, with an explained gap (live test 2026-08-24):** the
   panel stays empty for the first ~2 minutes on a large database (human
   FASTA), then fills quickly once `"generated N fragments, M peptides in T"`
   fires — matching the progress bar's own known gap during the same phase,
   for the same reason: `digest()`/`build_from_peptides()` only log at
   `trace!` level internally (see `crates/sage/src/database.rs`), so nothing
   crosses the `info` threshold until that one summary line at the very end.
   **With prefiltering on, the log starts instantly instead** — the chunked
   path logs `"using N db chunks of size M"` and `"pre-filtering fasta chunk
   N"` at `info!` per chunk, much earlier than the monolithic path's single
   end-of-phase summary. Not something to fix by itself; worth knowing if a
   future session wants less silence during a non-prefiltered build.
4. **License file — RESOLVED 2026-08-16.** Confirmed Apache-2.0 directly with
   Sebastian Paez (original GUI author). Added a `LICENSE` file at repo root
   with the standard Apache-2.0 text, crediting both Sebastian Paez (original
   sagegui) and Benjamin Neely (this fork). README's License section now links
   to it. Upstream Sage remains **MIT** (© 2022 Michael Lazear) — that's a
   separate third-party-notice question for distributed binaries, not
   addressed by this change.
5. **Run-bar progress bar — ✅ real progress, 2026-08-21.** No longer a
   placeholder. `total_mzml_spectra()` ([src/main.rs](src/main.rs)) pre-scans
   each selected mzML/mzML.gz file's `<spectrumList count="N">` tag before
   launch (plain-text scan, not a full XML parse; `None` if any file's count
   can't be found, rather than a misleadingly-low partial sum) for the
   denominator. The numerator comes from `Runner.progress` (see the fork-patch
   note above) via `ThreadMessage::RunnerReady`, sent once `Runner::new()`
   succeeds. Percent = live count / pre-scanned total. Status text names the
   current phase ("Building peptide database…" / "Reading and searching
   spectra…") since only the search phase has a real percentage — database
   build has no per-item signal available without changing public function
   signatures in both Sage crates (see fork-patch note above); this is an
   intentional limit, not a bug. See JOURNAL 2026-08-21 for the session
   narrative and the deferred real-DB-progress option.
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
   MOD_PRESETS" note under the Modifications-editor pin above. **Decision
   (2026-08-13):** do **not** hardcode `apply_archetype`. Instead ship
   **JSON-file templates** — a template *is* a saved config JSON, reusing the
   existing Save/Load Config plumbing. Bundle example templates in
   `assets/templates/` (tryptic-lfq / phospho / wide-open, built from real lab
   settings), replace the inert dropdown with a Templates picker that loads a
   bundled JSON into `self.config`, and add "Save current as template." `Custom`
   = no template applied. See PLAN "Experiment templates (JSON-file approach)".
   Secondary priority — if not quick, leave the dropdown in-progress and ship
   v0.7.0 on multi-FASTA alone.
7. **Theme is too low-contrast (2026-08-13, next-session request).** The default
   egui dark-grey-on-grey is too faint for readability. Collaborator shared a
   **reference screenshot** (a color/font mock, *not* the live UI) showing the
   target look: a **flat high-contrast light theme** — solid **black text on
   white / very-light-grey panels**, a crisp **light-blue selection highlight**
   on the active sidebar item, **thin flat borders** around inputs/buttons, and
   a **larger, legible font**. (Read as "clean high-contrast light," closer to a
   modern flat desktop UI than heavy Y2K bevels — earlier note said "chunky Y2K";
   the actual reference is flatter.) Scope for a dedicated session: build a
   custom `egui::Visuals` (light base, `override_text_color` → near-black, white
   `panel_fill` / `window_fill`, light-blue `selection.bg_fill`, visible
   `widgets.*` bg_stroke, bump default font sizes via `Style.text_styles`)
   applied at startup via `ctx.set_visuals` / `ctx.set_style`; optionally a
   theme toggle. Pure styling — no behavior change. This is a **next-session**
   item, not built yet.

---

## Settings persistence (landed 2026-08-21)

**What:** `SageLauncher` now saves `config`, `precursor_tolerance_type`,
`fragment_tolerance_type`, `experiment`, and `active_page` through eframe's
`persistence` feature (`Cargo.toml` — not a default eframe feature, enabled
explicitly). Restored in `SageLauncher::new(cc)` from `cc.storage` at startup;
saved via `App::save` on exit and on eframe's 30-second auto-save interval.
Implementation: a small `PersistedState` struct in `src/main.rs` mirrors just
those fields — `SageLauncher` itself is not `Serialize`, deliberately, so run
state (thread handles, live progress, elapsed time) can never accidentally leak
into the saved blob.

**Why:** The maintainer's actual workflow for "start over" was closing the
window — which already cancels a running search correctly (the process exits,
the thread dies with it) — but that also discarded every parameter. This closes
that gap without touching the deferred Save/Load Config work, which is a
different problem (matching Sage's `results.json` schema for
import/export — see "1. Save / Load Config" under UI-review feedback above).
Persisting our own `Config` to our own file needs no schema alignment.

**Consequence for future `Config` fields:** every field added to `Config` (or
anything it contains) must carry `#[serde(default)]` — or the deserialize of an
old saved blob fails outright (`eframe::get_value` returns `None` on failure,
which silently falls back to `SageLauncher::default()`, i.e. **all** settings
reset, not just the new field). The prefilter fields added in the same session
already follow this rule; see `src/ui.rs` `tests::old_config_json_without_prefilter_fields_still_loads`
for the regression pattern — extend it, don't just remember it, for the next
field addition.

**Storage location:** platform-native, chosen by eframe/`NativeOptions::persistence_path`
default (not inspected this session — if the file ever needs to be found or
cleared by hand, check eframe's `storage_dir` docs for the current version
rather than assuming a path).

**Side effect fixed in the same change:** closing the window mid-run used to
leak the temp concatenated multi-FASTA file (`cleanup_thread`, which deletes
it, never ran on that path). Now deleted in `App::on_exit` too.

**Confirmed gap (live test 2026-08-24):** mzML/FASTA file paths, output
directory, and other core fields persist correctly across a restart —
**modifications (static/variable) do not.** Not yet root-caused; check whether
the Modifications tab's selections actually live inside `Config` (which
`PersistedState` saves whole) or in separate `SageLauncher` fields
(`mod_target`, `mod_selected_preset`, `mod_custom_key`, `mod_custom_mass` are
known to be picker-UI-only state, correctly excluded — but if the chosen
mods themselves live somewhere similarly excluded rather than in
`Config.database`, that would explain it). Also unconfirmed which *other*
fields besides mods might have the same problem — do a full field-by-field
audit next session, not just a mods fix.

## Stop button (landed 2026-08-21; false-message bug fixed 2026-08-24)

**What works:** clicking Stop *before* the search phase starts (during
database build or the prefilter pass) — the `Arc<AtomicBool>` cancel flag is
checked in `run_sage` (`src/main.rs`) at phase boundaries before
`input.build()`, after it, and after `Runner::new()`; the run genuinely
aborts and writes nothing, matching the "No output files were written" message.

**Bug found in live testing 2026-08-24, fixed same day:** clicking Stop
*after* the search phase had started did not stop anything —
`Runner::run()` has no cancellation check inside it (always known, and
separately documented below as by-design) — so the search ran to completion
normally, **writing its output files as usual**. But `check_thread_status`'s
`Completed` handler looked only at `self.stop_requested` (still `true` from
the click) to decide what message to show, so it reported "Search stopped. No
output files were written." even though the run had actually finished and the
files existed. The earlier "verified" note in this section had only checked
the pre-`run()` cancellation path; it never covered a Stop clicked during
`run()` followed by natural completion, where the message was simply false.

**Fix (`src/main.rs` `check_thread_status`, `Completed` arm):** the handler now
matches on the actual `result` from `run_sage` instead of branching on
`stop_requested` first. `run_sage`'s cancellation checks return the specific
error string `"cancelled"`; the "Search stopped. No output files were written."
message now only appears when the result is `Err("cancelled")` *and*
`stop_requested` is true. Any other `Ok` result shows the real success message
regardless of whether Stop was clicked, and any other `Err` shows the real
error. Covered by three unit tests in `src/main.rs` `mod tests`
(`stop_clicked_but_search_finished_reports_real_success`,
`stop_clicked_before_completion_reports_no_output_written`,
`genuine_failure_without_stop_reports_error`) that drive
`check_thread_status` directly through a real `mpsc` channel — no live GUI
run needed to verify this specific logic bug, since it lived entirely in
message handling, not in Sage interaction. **Not yet live-tested in the real
GUI** (same tooling gap as before — no accessibility/computer-use tool for a
native macOS app); the maintainer should re-run the click-Stop-mid-search
reproduction from the 2026-08-24 test to confirm the fix end-to-end.

**"Run bar left in a confusing state" symptom — likely explained, not
separately fixed.** The reporter also described the run bar as looking stuck
("turns yellow and doesn't stop, and then run is not open") after a Stop
click during an active search. Reading `cleanup_thread`: it unconditionally
resets `is_running`, `thread_handle`, `stop_requested`, etc. as soon as the
`Completed` message arrives, so the Run button does re-enable once the search
actually finishes — there is no separate deadlock in the code. The most
likely explanation is that `is_running` (and the yellow "Stopping" spinner)
correctly stays true for the *entire remaining duration of the real search*,
since nothing past the last cancellation checkpoint can shorten it — on a long
search this can look frozen for minutes, and the false completion message
made it look like something had gone permanently wrong on top of that. If the
maintainer's live re-test still sees a stuck state *after* the search
actually completes (not just during the long wait), that would be a second,
distinct bug — reopen this section if so.

**What doesn't work by design, not by bug:** a search already inside
`Runner::run()` — i.e. already scoring spectra — is **not interrupted**. It
runs to completion regardless of Stop. The button's hover text says this
plainly ("Stops at the end of the current step. A search already scoring
spectra runs to completion.") — this part is working as intended; only the
resulting status message and run-bar state are wrong.

**To make Stop interrupt an in-progress search:** needs a cooperative-cancellation
flag threaded into `neely/sage`'s `Runner`, in the same additive shape as the
`Runner.progress` patch documented above — a `pub cancel: Arc<AtomicBool>` field
checked inside `search_processed_spectra`'s map closure
(`sage-cli/src/runner.rs:308-330`) and a couple of other loop sites. Sketched but
**not implemented** — deliberately deferred, since settings persistence (above)
already removes most of the original pain (relaunching after a bad parameter
choice is now fast and lossless). Revisit only if mid-search cancellation turns
out to still matter once persistence has been used for a while. If it's built,
extend the "Custom patch carried on `neely/sage`" section above with the new
field — that section is the merge-conflict guide for the next Sage sync, and an
unlisted patch is silently lost on the next `git merge upstream/main`.

---

Things that look wrong but are correct. Do not "fix" these.

- **Default output directory is the current working directory.** Users set it explicitly in the GUI. (Smarter timestamped defaults are a *planned* Phase 5 improvement, not a bug to patch ad hoc.)
- **TMT quantification is untested.** Only LFQ has been validated with real data — TMT code paths are believed correct but need TMT-labeled data to confirm. Not a defect; a known coverage gap (see below).
- **Stop doesn't interrupt a search already in progress.** Intentional, current limitation — see "Stop button" above. Not a bug to "fix" without first reading why it was deferred. **This is separate from the false "No output files were written" message** after a Stop-then-natural-completion — that was a real bug, fixed 2026-08-24 (same section).

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

#### Database prefiltering — `prefilter`, `prefilter_chunk_size`, `prefilter_low_memory`

Derived from the pinned Sage source on 2026-08-21. **Sage's own `DOCS.md` does
not document these three parameters at all** — a grep of every `.md` in the Sage
repo finds "prefilter" only in one CHANGELOG line. The code is the only source of
truth. Do not re-derive this; the user-facing write-up is in
`docs/PARAMETER_REFERENCE.md`.

Resolved defaults, from `Builder::make_parameters`
(`crates/sage/src/database.rs:111-113`):

| Field | Resolves to when unset |
| ----- | ---------------------- |
| `prefilter` | `false` |
| `prefilter_chunk_size` | `0` (auto) |
| `prefilter_low_memory` | **`true`** |

`prefilter_low_memory` is read at exactly one site
(`sage-cli/src/runner.rs:271`). That site is reached only when `prefilter` is
`true` **and** `prefilter_chunk_size < fasta.targets.len()`
(`runner.rs:114-129`). So at defaults it resolves to `true` but never runs.

**Mechanism** (`runner.rs:149-245`, `crates/sage/src/fasta.rs:81`):
`iter_chunks(n)` splits `fasta.targets` — protein entries — into groups of `n`.
Per chunk, Sage builds a full index for those proteins, quick-scores every MS2
spectrum, keeps the peptides that matched, and frees the chunk index. After the
last chunk, `build_from_peptides` builds one final index from the survivors.
Peak memory becomes about the larger of one chunk index or the final index.

**`quick_score`** (`crates/sage/src/scoring.rs:256-299`): with `low_memory =
true`, Sage scores every preliminary hit and keeps the top `report_psms + 1` per
spectrum per chunk. The `+1` comes from `prefilter_peptides` building its
`Scorer` with `report_psms + 1`. With `false`, Sage keeps every preliminary hit
unscored — more peptides, more memory, less CPU.

**Auto chunk size** (`database.rs:142-159`): `0` makes Sage digest the whole
FASTA to estimate peptide count, scale it by
`(variable_mods.len() + 1) * 2^max_variable_mods`, and divide by `2^23`
(8,388,608) peptides per chunk. If that gives zero chunks, chunk size becomes
`fasta.targets.len()`, which trips the `>=` guard and skips prefiltering. So
auto self-disables on small search spaces. The wasted work is the extra digest.

**Two gotchas worth remembering:**
- **File-count cliff** (`runner.rs:150-157`). If `parallel >= mzml_paths.len()`,
  Sage reads all spectra once and holds them for the whole pass. Otherwise it
  re-reads and re-processes every mzML file once per chunk. `sagegui` sets
  `parallel = num_cpus::get() / 2` (`src/main.rs:266`), so a 16-core machine
  switches at 9 files.
- **Per-chunk decoys.** `prefilter_peptides` generates decoys per chunk and does
  not regenerate them over the final filtered set. The disabling code is
  commented out with an upstream TODO (`runner.rs:163-170`). Expect PSM counts
  to differ slightly from a non-prefiltered run.

**Progress-bar consequence.** `peptide_filter_processed_spectra` never touches
`self.progress`, and the `mini_runner` holds its own separate `Arc`. The whole
prefilter pass therefore shows 0% in the run bar. This was recorded as an
unreachable path while `prefilter` was hardcoded off; it becomes reachable the
moment the GUI exposes the checkbox.

#### Precursor/fragment tolerance window — sign & delta-mass convention

The Da tolerance window is **relative to the experimental precursor mass** and is
**not symmetric-by-assumption** — the two boxes are an independent (lower, upper)
pair. This trips people up, so the exact semantics:

From Sage's `Tolerance::bounds` ([`crates/sage-core/src/mass.rs`], `Tolerance::Da`):

```rust
Tolerance::Da(lower, upper) => (center + lower, center + upper)
```

where `center` = the **experimental** monoisotopic precursor mass. Sage then
searches for theoretical peptides whose mass lands in
`[center + lower, center + upper]`.

- **Lower is normally negative, upper positive.** `-500, 100` searches theoretical
  peptides from **500 Da below to 100 Da above** the observed precursor.
- **The sign flips when you think in delta-mass / modification space.** A candidate
  peptide that is 500 Da *lighter* than the observed precursor means the observed
  species carries a **+500 Da modification** relative to that peptide. So a `-500`
  *lower bound* corresponds to finding IDs with a **+500 Da delta mass**. The two
  framings (theoretical-peptide space vs. delta-mass space) are consistent —
  they just reference opposite endpoints. This is the "-500,100 = -100 to 500"
  confusion Michael flagged: it is the delta-mass reading of the same window.
- **Inverted window = empty search.** If `lower > upper`, Sage computes an empty
  range and finds nothing. Enter the smaller (usually negative) value in the
  **Lower** box.

**GUI behavior:** SageGUI passes the two DragValues straight through, verbatim —
first box → `lower`, second box → `upper`, no sign manipulation or reordering
([src/ui.rs](src/ui.rs) `ToleranceConfig::update_section`, and `From<ToleranceConfig>
for Tolerance`). So an asymmetric window is preserved exactly as typed. The boxes
are labelled **Lower / Upper**, carry hover text explaining the convention, and
show a non-blocking ⚠ warning if `lower > upper`.

**Deferred behavior change:** an optional "show as delta mass" framing (type
`+500` for a +500 Da mod, GUI flips the sign) is captured in PLAN Phase 5 with
caveats — *not* built, because it would diverge from every Sage config file/CLI.
The raw `(lower, upper)` convention above is what's stored and shipped.

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
- **Local copy (2026-08-21):** the baseline files live at
  `~/Documents/proteomicsTesting/B.naive_01steady-state.mzML.gz` and
  `~/Documents/proteomicsTesting/UniProt-Human-UP000005640_canonical-2023_05.fasta`
  on the maintainer's Mac. Two other mzML.gz files sit alongside them
  (`2019-4-9_909c_0311.mzML.gz`, `b1906_293T_proteinID_01A_QE3_122212.mzML.gz`) —
  not the validated baseline, but available for spot-checks.
- **Prefilter controls, settings persistence, and Stop button (2026-08-21) —
  not yet re-run against this baseline.** These landed with `cargo
  build`/`clippy`/`fmt`/`test` all clean and three new unit tests covering the
  prefilter serde-migration path (`src/ui.rs` `mod tests`), but **no live GUI
  run has confirmed the baseline PSM count is unaffected**, since driving the
  native window isn't something the agent tooling in this session could do
  (no accessibility/computer-use tool for a native macOS app was available —
  only browser and iOS Simulator automation). Next session (or the maintainer,
  interactively): (1) re-run the exact baseline above with prefiltering off,
  confirm still 60,672 PSMs; (2) re-run with prefiltering on
  (`prefilter_chunk_size` on auto, `low_memory` on), confirm a plausible but
  *different* count — see "Database prefiltering" above for why it should
  differ; (3) close-and-reopen the app after setting unusual parameters,
  confirm they're restored; (4) click Stop during a run and confirm the
  documented "stops at the end of the current step" behavior.

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
