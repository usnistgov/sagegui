# SageGUI: full agent protocol

How to work in this repo. The root [AGENTS.md](../AGENTS.md) is the short
version (layout, settled decisions, the tolerance rule). This file is the full
operating protocol. Read both, every session.

This file and its siblings live in `_dev/` (see [README.md](README.md)). Paths
in them are from the repository root unless a link says otherwise.

---

## ⛔ STOP: precursor tolerance sign convention

The rule lives once, at the top of the root [AGENTS.md](../AGENTS.md). Read it
before writing, reviewing or describing any tolerance value.

---

## Reading order (cold start)

1. **AGENTS.md** (root) and **dev_AGENTS.md** (this file): how to work here.
2. **_dev/PLAN.md**: current status block, then the roadmap. Start with the status block at the top; it tells you the phase and the next concrete action.
3. **_dev/NOTES.md**: locked decisions, intentional non-bugs, dead-ends, and reference (domain primer, API quirks, gotchas). Consult before changing anything that looks wrong. It might be intentional.
4. **_dev/JOURNAL.md**: append-only session history. Read the top few entries to see what recently happened and what the last session was least sure about.

`README.md`, `CHANGELOG.md`, `MAINTENANCE.md`, and `docs/GLOSSARY.md` are **release / user-facing**, not dev context — they aren't part of the startup sequence. Keep them synced with reality, but touch them only on the trigger below.

---

## The other docs (not the startup set)

These exist alongside the dev-context files above. Know they're there and when to update them:

| File | What it's for | Touch it when… |
| ---- | ------------- | -------------- |
| `README.md` | User-facing landing page: install, quick start, feature list | user-visible behavior, install steps, or the doc list changes |
| `CHANGELOG.md` | Release history (Keep-a-Changelog format) | Add a one-liner to `[Unreleased]` for each user-visible change as it lands. Move entries to a versioned section when cutting a release. |
| `MAINTENANCE.md` | Maintainer runbook for updating the vendored Sage in `vendor/sage` | the update procedure changes, or a new Sage upgrade adds an API-change example |
| `docs/GLOSSARY.md` | Definitions of proteomics / MS / Sage terms | a new domain term enters the docs and needs defining |

Rule of thumb: **the four dev-context files (AGENTS, PLAN, NOTES, JOURNAL) are the source of truth for *how we build*; these four are for *what we ship and how to maintain it*.** Don't duplicate content across the two sets — link instead. (E.g. the API-change reference lives once in NOTES; MAINTENANCE points to it.)

---

## Editing standards

- **Targeted edits only.** Never rewrite a whole file to change a few lines.
- **Commit straight to `main`** with plain messages. No branches, no squashing, no PRs unless asked.
- **Keep docs synced with live code.** If you change behavior, update the doc that describes it in the same session.
- Match the surrounding code's style, naming, and comment density.
- **Batched atomic commits.** Group logically-related file changes (a code change + the doc update explaining it) into a single commit. One commit = one coherent decision. Hold related edits together before committing rather than committing each as it's finished.
- **Write in ASD-STE100.** Simplified Technical English. Short sentences. One idea each. Applies to commit messages and everything written in PLAN, NOTES, and JOURNAL.
- **No em dashes in UI text.** Template descriptions, tooltips, status lines and labels use full stops and short sentences. Prose in PLAN, NOTES and JOURNAL is not affected by this rule.
- **UI text is shorter than you think.** A template description names what is unique about that template. It does not explain the background or repeat what every template shares. Put the reasoning in NOTES and leave the UI bare.

---

## Respect the markers

Items tagged **(locked)**, **intentional**, or listed under **dead-ends** in NOTES.md are settled. Do not revisit or "fix" them without an explicit instruction. They exist to stop re-litigation.

---

## Session start

Before writing any code:

1. Verify PLAN's status block, checkboxes, and NOTES match the actual repo state.
2. Flag anything stale (a checkbox ticked for work that isn't there, a "current" version that no longer matches `src/version.rs`, etc.) before proceeding.

---

## Session end (shutdown routine)

1. Update PLAN's **status block** (phase, last-updated date, next action).
2. Tick completed checkboxes in PLAN.
3. Record any decisions, dead-ends, or new reference knowledge in **NOTES.md** — in the right section, not chronologically.
4. Sync **README** / **CHANGELOG** / **MAINTENANCE** if behavior or version changed.
5. Run the **debrief** (below).
6. Append one **JOURNAL.md** entry (the debrief) — newest on top, never edit past entries.
7. Commit and push. Confirm with the user before pushing.

---

## Debrief questions

Every session ends by answering these. **Q1 and Q5 are required every session**; include Q2–Q4 for substantial sessions.

1. **(required)** What are you least confident about, and what test or observation would prove it right or wrong?
2. What did you assume without stating it?
3. What's the biggest thing you might be missing?
4. What could have gone better?
5. **(required)** One concrete suggested improvement.

The JOURNAL entry is where these answers live.
