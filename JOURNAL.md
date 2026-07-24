# SageGUI — Journal

Append-only. Newest entry on top. Never edit past entries — this is history,
not current state. One entry per session: the shutdown debrief.

> Entries below dated 2026-07-10 through 2026-07-13 were reconstructed from the
> old NOTES.md session log during the migration to the agent-context template.
> They are summaries, not contemporaneous debriefs.

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
