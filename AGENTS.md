# SageGUI — Agent protocol

How to work in this repo. Read this first, every session.

---

## ⛔ STOP — precursor tolerance sign convention

**Sage's `precursor_tol` pair is NOT the delta-mass range. It is negated AND
reversed.** This has been got wrong repeatedly. Read this before writing,
reviewing, or describing any tolerance value.

Sage searches candidate **theoretical** masses in
`[experimental + lower, experimental + upper]` (`Tolerance::bounds`,
`crates/sage/src/mass.rs`; the centre is the *experimental* mass — see
`db.query(precursor_mass, ...)` in `crates/sage/src/scoring.rs`). Delta mass,
the modification a PSM carries, is `experimental - theoretical`. So:

```
delta_range = (-upper_raw,  -lower_raw)    # negate AND swap
raw_json    = (-delta_high, -delta_low)    # negate AND swap back
```

| What a person means | What goes in the JSON |
| ------------------- | --------------------- |
| delta −1.25 to +3.5 Da | `"da": [-3.5, 1.25]` |
| delta −100 to +500 Da | `"da": [-500, 100]` |
| "find a +500 Da modification" | lower bound `-500` |

**Never present a raw JSON pair as if it were the delta-mass range, and never
present a delta-mass range as if it were raw JSON.** State which convention you
are using, every time. Symmetric ppm windows (`[-10, 10]`) hide the mistake —
it only becomes visible on asymmetric and open windows, which is exactly where
it does damage.

---

## Reading order (cold start)

1. **AGENTS.md** (this file) — how to work here.
2. **PLAN.md** — current status block, then the roadmap. Start with the status block at the top; it tells you the phase and the next concrete action.
3. **NOTES.md** — locked decisions, intentional non-bugs, dead-ends, and reference (domain primer, API quirks, gotchas). Consult before changing anything that looks wrong — it might be intentional.
4. **JOURNAL.md** — append-only session history. Read the top few entries to see what recently happened and what the last session was least sure about.

`README.md`, `CHANGELOG.md`, `MAINTENANCE.md`, and `docs/GLOSSARY.md` are **release / user-facing**, not dev context — they aren't part of the startup sequence. Keep them synced with reality, but touch them only on the trigger below.

---

## STOP — the precursor window is written backwards

Read this before you write, quote, or explain any precursor tolerance value.

Sage applies the window to the **experimental** mass and searches for
**theoretical** peptide masses inside it:

```
theoretical mass in [experimental + lower, experimental + upper]
```

Delta mass is `experimental - theoretical`. So the raw JSON pair and the
delta-mass range are **negated and swapped**:

```
raw JSON [lower, upper]   <->   delta mass [-upper, -lower]
```

Worked examples. Learn these two:

| Raw JSON (what Sage reads) | Delta mass (what a person means) |
| -------------------------- | -------------------------------- |
| `"da": [-3.5, 1.25]`       | -1.25 to +3.5 Da                 |
| `"da": [-500, 100]`        | -100 to +500 Da                  |

A **+500 Da modification** is found by a **-500 lower bound**. The large
number always goes on the left in the JSON, with a minus sign.

This has been explained more than once and got repeated wrong anyway. It is
easy to state the rule correctly and then still describe a specific pair
backwards in prose. So: every time you write a precursor window in a
sentence, a table, a doc, a commit message, or a template description, do the
negate-and-swap on paper first. Say which convention you are using. Never
print a raw pair and call it a delta mass.

Source of truth, if you doubt it: `Tolerance::bounds()` in
`crates/sage/src/mass.rs` returns `(center + lo, center + hi)`, and
`Database::query()` in `crates/sage/src/database.rs` calls it on the
experimental precursor mass, then binary-searches peptides by their
theoretical `monoisotopic` mass.

See NOTES.md "Precursor/fragment tolerance window" for the UI history.

---

## The other docs (not the startup set)

These exist alongside the dev-context files above. Know they're there and when to update them:

| File | What it's for | Touch it when… |
| ---- | ------------- | -------------- |
| `README.md` | User-facing landing page: install, quick start, feature list | user-visible behavior, install steps, or the doc list changes |
| `CHANGELOG.md` | Release history (Keep-a-Changelog format) | Add a one-liner to `[Unreleased]` for each user-visible change as it lands. Move entries to a versioned section when cutting a release. |
| `MAINTENANCE.md` | Maintainer runbook for syncing the Sage fork to a new version | the update procedure changes, or a new Sage upgrade adds an API-change example |
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
