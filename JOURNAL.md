# SageGUI — Journal

Append-only. Newest entry on top. Never edit past entries — this is history,
not current state. One entry per session: the shutdown debrief.

> Entries below dated 2026-07-10 through 2026-07-13 were reconstructed from the
> old NOTES.md session log during the migration to the agent-context template.
> They are summaries, not contemporaneous debriefs.

---

## 2026-08-13 — UI-review follow-up: move Output Location to Run tab; Modifications list-picker

**Refinement addendum (same sitting):** After the collaborator ran the new
Modifications tab in the debug build (works), several tweaks: (1) sorted
`MOD_PRESETS` alphabetically by label + comment to keep it sorted; (2) replaced
the combined "Oxidation (M/P)" preset with a standalone "Oxidation (P)" so it
adds on top of Ox M rather than duplicating M; (3) the "+ Custom…" panel now
shows a Sage key-syntax cheat-sheet grid (`X`, `^X`, `$X`, `[`/`[X`, `]`/`]X`)
+ a `^Q` hint; (4) added a tab footnote that displayed Δmasses round to 4 places
while the full monoisotopic value is stored/used, plus a per-row hover showing
the exact stored mass; (5) spelled out the pyro-Glu preset labels
("Glu->pyro-Glu (E, peptide N-term)" etc.) to match the Acetyl/Carbamyl style.
Two next-session items were **captured, not built**: the Experiment archetypes
are **inert** (changing the dropdown does nothing to other tabs — confirmed;
needs `apply_archetype`), and the collaborator wants a **high-contrast "Y2K"
theme** because the default grey-on-grey is too faint. Both recorded in NOTES
(UI-review feedback #6, #7) and PLAN. Custom-mod *persistence* also remains an
open design question (NOTES "Custom modifications — persistence"). Build +
clippy clean.

**Did (part 1 — Output Location):** Moved the Output Location control from Files &
Database to the Run / Info tab, per UI-review follow-up #2. It now sits in its own
group above Output Options on the Run tab (browse-folder + text field unchanged).
Committed `84c927a`.

**Did (part 2 — Modifications list-picker):** Rebuilt the Modifications tab from the
inherited add/list/remove form into a **Mascot-style list-picker** per the
collaborator's spec: two destination boxes (Static / Variable) on the left, a
curated **"Common modifications"** master list on the right (hardcoded
`MOD_PRESETS` in `src/ui.rs`, 11 entries, Unimod monoisotopic deltas), and
◀ Add / Remove ▶ arrows acting on whichever box a **Target** toggle selects.
Decisions confirmed with the user first: (a) list hardcoded in Rust; (b)
multi-residue presets insert as **separate editable rows** (Phospho → S, T, Y as
three rows) — resolves the pinned "per-AA specificity" open problem; (c) build now
(full redesign). Kept a "+ Custom…" collapsing free-type escape hatch. Enforced
**Static/Variable mutual exclusion** on every add. Deleted the now-dead
`_update_section` / `update_deletion_queue` / `new_mod_buffer` / `new_mass_buffer`
machinery and added `insert_key`/`remove_key`/`show_list` helpers on
`StaticModConfig`. `cargo build` + `cargo clippy` both clean. Synced CHANGELOG,
PLAN (ticked preset-library item), NOTES (rewrote the "Modifications editor
redesign" pin as shipped — [[deamidation-mass-doubt]] resolved: 0.984016).

**License (follow-up #4) deferred by the maintainer** — wants to talk to Sebastian
(original author) before choosing MIT vs Apache-2.0; recorded as blocked in NOTES,
so no LICENSE file was added and Cargo.toml/README/GUI were left untouched.

**Least confident about (Q1):** That the multi-key remove path is intuitive — Add
inserts three rows for Phospho, but Remove ▶ (preset still selected) removes only
the exact keys in that preset from the *targeted* box; if a user manually edited
rows first, the mismatch is a silent no-op (correct, but unobserved). Proven
right/wrong by launching the GUI and exercising add → manual-remove-one →
preset-remove; the per-row ✖ buttons are the reliable path regardless.

**Unstated assumptions (Q2):** Assumed the serde round-trip still works after
dropping the transient `new_mod_buffer`/`new_mass_buffer` fields — they were
`#[serde(skip)]` so JSON shape is unchanged, but I did not re-run a Save/Load
round-trip (still the untested UI-review #1 item).

**Biggest thing being missed (Q3):** The Experiment archetypes (Phospho,
Semi-tryptic, etc.) don't touch the other tabs at all — **confirmed inert** by
the collaborator this session, not just the mod boxes. Wiring `apply_archetype`
(archetypes → Search/Mods/Quant defaults, reusing `MOD_PRESETS`) is the natural
next step; the open decision is overwrite-user-edits vs seed-once. Also queued:
a **flat high-contrast light theme** (the collaborator's screenshot was a
color/font reference, not the live UI — black-on-white panels, light-blue
active-item highlight, larger font). Both in NOTES UI-review #6/#7 + PLAN.

**Suggested improvement (Q5):** Add unit tests over `MOD_PRESETS` — assert every
`keys` entry parses via `ModificationSpecificity::from_str` and no mass is zero —
so a future typo in the hardcoded table fails CI instead of silently producing a
dead preset.

---

**Post-session addendum (same sitting):** Collaborator ran the Phase 2 baseline
through the new GUI → **60,672 PSMs, identical** to the pre-restructure run. My
Q1 doubt below is resolved: the layout port preserved search behavior. Recorded
as a regression checkpoint in NOTES. Also captured five UI-review follow-ups in
NOTES ("UI-review feedback") for next session: verify Load Config populates all
tabs + re-save; move Output Location to the Run tab; rework Run/Info (re-home
Info/Help, use the space for a live Sage console readout); **license is
wrong/missing** — Cargo.toml/README/GUI say Apache-2.0 but the LICENSE file
doesn't exist and upstream Sage is MIT (pinned as an open decision); run-bar
progress bar animates but is still a placeholder.

**Did:** Cleaned up four divergent `v0.7.0-alpha.*` tags (Sebastian's line, never on our main/origin) — documented provenance + harvest ideas (mimalloc, Bruker centroiding config, LFQ mobility tol) in NOTES, deleted the local tags. Wrote `docs/ui-spec.md` (paste-in design spec + web-LLM deliverable format: report + YAML layout). Collaborator ran it through a web LLM (MetaMorpheus-inspired result) and we iterated the tab structure together. Then ported: single scrolling page → **6-tab sidebar layout** (Experiment, Files & Database, Search, Modifications, Quant, Run/Info) with a **pinned bottom run bar** on every tab; extracted all UI into new `src/ui.rs` (main.rs 1074→299, ui.rs 1090). Surfaced the **6 previously-hidden Sage params** (precursor_charge, override_precursor_charge, isotope_errors, score_type, write_pin, annotate_matches) with real controls + tooltips. Added **Save/Load Config JSON** and `on_hover_text` tooltips throughout. **Dropped native Bruker `.d`** support (mzML/.gz only). A Sonnet subagent did the bulk port; I verified build/clippy/launch independently. Synced CHANGELOG ([Unreleased]), PLAN (status→Phase 5 in progress, ticked save/load + param-docs), NOTES (UI-redesign section + design pins). `.claude/` gitignored. Committed `6712bb1` and pushed.

**Least confident about (Q1):** ~~That a real search still completes end-to-end through the new UI.~~ **RESOLVED (see addendum): baseline re-ran at 60,672 PSMs.** Remaining sub-doubt: the Save→Load round-trip was *not* observed end-to-end — pinned as UI-review follow-up #1.

**Unstated assumptions (Q2):** Assumed the subagent's serde workaround for `ModificationSpecificity` (shadow `HashMap<String,f32>` synced via `sync_to_ser`/`sync_from_ser`) round-trips losslessly for all valid mod syntaxes — I read the code but didn't test odd cases (protein-terminal `[`/`]`, peptide-terminal `^`/`$`). Also assumed `score_type`'s two variants are the complete set (verified against the Sage source mirror: `{SageHyperScore, OpenMSHyperScore}`).

**Biggest thing being missed (Q3):** The modifications editor is now on its own tab but is still the *old* add/list/remove form — the real UX win (dual-pool transfer-list, per-amino-acid handling, presets) is only pinned in NOTES, not built. This is the highest-value remaining UI work and the hardest; the current tab may under-deliver on the "substantially improve" goal until that lands.

**Could have gone better (Q4):** I burned a Write call and two ExitPlanMode attempts fighting a harness hiccup (server-unavailable classifier + plan-mode state confusion). Should have recognized the ExitPlanMode rejection as a harness issue faster and just presented the plan in-chat, which is what worked.

**Suggested improvement (Q5):** Before the next UI session, run the Phase 2 baseline search through the new GUI once and record the PSM count in NOTES as a post-restructure regression check — turns the Q1 doubt into a green checkmark and gives every future UI change a known-good comparison point.

---

## 2026-07-24 — Preserve feedback + document parameters (post-shutdown addendum)

**Did:** Saved the collaborator's original feature list verbatim to `docs/feedback-2026-07-24.md` (was only in chat before). Added live-use follow-up: `bucket_size` dial confused a user, and a standard-mods list. Recorded both in NOTES.md under a new "Sage parameter notes" subsection (bucket_size explanation + variable/static mod syntax table with encodings). Added a Phase 5 item for **in-GUI parameter documentation** (tooltips/doc links for every control), generalized from the bucket_size confusion.

**Least confident about (Q1):** The deamidation mass I recorded (`+0.98402` for N/Q). Standard value is ~0.984016 — should double-check against Unimod before it ships in a preset. Proven right/wrong by cross-referencing Unimod accession 7 (Deamidated).

**Suggested improvement (Q5):** When the mod preset library is built, source masses from Unimod directly rather than hand-transcribing — avoids exactly the doubt above.

---

## 2026-07-24 — Session shutdown: template migration + Phase 5/6 planning

**Did:** Full session covering two things: (1) migrated project docs to the agent-context template (AGENTS.md, JOURNAL.md, NOTES.md topical reformat, PLAN.md status + handoff, CONTEXT.md folded/deleted); (2) scoped Phase 5 and Phase 6 from user feedback — async execution prioritized as #1, added multi-FASTA + cRAP input, ThermoRawFileParser .raw conversion with license-check gate, FDR-filtered rollup export, and format spoofing for MSstats / LFQ-analyst / Scaffold. Renumbered old "Phase 6–8" stubs to 7–9 to make room. Phases 5 & 6 are fully written in PLAN; nothing is implemented yet.

**Least confident about (Q1):** ThermoRawFileParser cross-platform bundling — it's Apache-2.0 (license clear) but it's a .NET binary; on Linux/macOS users may not have the .NET runtime. Proven right/wrong by a 30-min spike: grab the self-contained release build, call it from `std::process::Command`, run on all three CI platforms.

**Unstated assumptions (Q2):** Assumed the rollup scripts are Python. If they're R, the "call as subprocess" strategy is the same but the dependency story for end users is different. Verify when locating the scripts.

**Biggest thing being missed (Q3):** No priority ordering on the Phase 6 export formats (MSstats vs. LFQ-analyst vs. Scaffold). They vary enormously in difficulty — Scaffold requires pepXML spoofing; MSstats may just need column renames. Knowing which one the user actually needs first would sharpen the phase.

**Could have gone better (Q4):** Two separate JOURNAL entries for the same sitting (template migration and feature planning). Should have been one. Minor, but violates the "one entry per session" protocol.

**Suggested improvement (Q5):** Before any Phase 6 implementation, do a 30-min format survey: find a sample input file for each target tool, identify which columns Sage already produces vs. what would need to be synthesized, and record the gap analysis in NOTES under a new "Output format reference" subsection. That shapes the entire phase's scope and will surface Scaffold's pepXML complexity early.

---

## 2026-07-24 — Incorporate user feedback; restructure phases 5 & 6

**Did:** Refined Phase 5 and added new Phase 6 based on user feedback from GUI testing. Phase 5 now has priority ordering (async execution first) and two new input sections: multi-FASTA concatenation with built-in cRAP toggle, and ThermoRawFileParser integration for `.raw` conversion. Phase 6 is new — FDR-filtered peptide/protein rollup export plus format spoofing for MSstats, FragPipe Analyst/LFQ-analyst, and Scaffold. Resolved the old Phase 5 discussion points (rollup scripts exist in a separate project — action item to locate/read them; format export is "spoof where we have the data"). Also fixed markdown lint warnings (blank lines around headings/lists).

**Least confident about (Q1):** Whether ThermoRawFileParser can actually be bundled cleanly cross-platform (it's .NET — requires runtime on Linux/macOS). Would be proven right/wrong by a quick spike: download the binary, call it from a Rust `std::process::Command`, test on each CI platform.

**Suggested improvement (Q5):** Before Phase 6 gets implemented, do a 30-min spike to confirm MSstats input format requirements — it has had breaking column-name changes across versions, and discovering that mid-implementation would be expensive.

---

## 2026-07-24 — Migrate to agent-context project template

**Did:** Adopted the [agent-context-project-template](https://github.com/neely/agent-context-project-template). Added AGENTS.md (agent protocol) and JOURNAL.md (this file). Reformatted NOTES.md from a chronological progress log into a topical knowledge base (locked decisions, intentional non-bugs, dead-ends, reference). Added a status block and handoff section to PLAN.md. Folded CONTEXT.md's durable content (domain primer, gotchas, reference index) into NOTES.md and deleted CONTEXT.md; updated README links.

**Least confident about (Q1):** Whether all the chronological detail from the old NOTES was correctly re-homed into topical NOTES vs. journal without losing anything — would be proven right/wrong by diffing the old NOTES.md against the new NOTES.md + JOURNAL.md and confirming every fact landed somewhere.

**Suggested improvement (Q5):** Do a real working session against the new file layout to see whether the reading order and section boundaries actually hold up, then adjust AGENTS.md if the split feels wrong in practice.

---

## 2026-07-13 — Phase 3 & 4: CI/CD, release, documentation

**Did:** Completed Phase 3 (CI/CD & Release) and Phase 4 (Documentation & Handoff). Set up GitHub Actions building on Windows, Linux, macOS (x64 + ARM64) with automated releases on tag push. Added `cargo fmt`/`clippy`/`test`/`build --release` to CI. Cut release `v0.6.0`. Simplified version sync to `src/version.rs` constants (removed `build.rs`). Configured Dependabot, auto-generated release notes, a badge-update workflow, and structured logging via the `log` crate. Wrote MAINTENANCE.md and updated README with Quick Start + macOS Gatekeeper bypass.

**Did:** Marked project ready for handoff — Phases 0–4 complete.

---

## 2026-07-12 — Phase 2 debrief; Phase 5 planning

**Did:** Ran a full search on real data (60,672 PSMs), verified LFQ quantification. Added a version badge to README. Expanded PLAN with Phase 5 GUI improvements.

**Clarified:** Test output directory (`test/`) was manually set in the GUI. sagePreview LFQ/rollup scripts are separate tools for later discussion. TMT testing deferred (LFQ sufficient for now).

**Improvements identified:** automated testing in CI; version auto-sync (was hardcoded); better progress display; results summary panel; config persistence; smarter output directory; "Analyze with sagePreview" link — folded into Phases 3 and 5.

---

## 2026-07-10 — Phase 1 & 2: Fork Sage, update to v0.15.0-beta.2, test

**Did:** Forked `lazear/sage` to `neely/sage`. Discovered `lib.rs` already exists in v0.15.0-beta.2 — no modifications needed (the plan had predicted we'd add it). Updated sagegui `Cargo.toml` to use the fork, pinned to commit `d74024df`. Fixed 6 API compatibility issues (see NOTES reference). Added Sage version display in the GUI, created CHANGELOG.md. Confirmed the GUI launches and ran a successful search.

**Key learnings:** lib.rs already existed; used v0.15.0-beta.2 (current master) rather than the v0.14.7 the plan assumed; actual API changes differed from the plan's predictions; pinning to a commit hash beats tracking a branch for reproducibility.

---

## 2026-07-10 — Phase 0: Bug fixes & documentation setup

**Did:** Cloned and analyzed Sebastian Paez's original sagegui. Found and fixed two bugs — TMT 16/18-plex mis-mapped to `Tmt11`, and fragment-tolerance type switching writing to `precursor_tol` instead of `fragment_tol` (both in commit a225481). Pushed to `neely/sagegui`. Decided on Option A (fork Sage) over Option C (subprocess wrapper). Created the initial documentation set (CONTEXT.md, PLAN.md, NOTES.md, GLOSSARY.md).

---
