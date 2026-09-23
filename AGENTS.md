# Agent protocol

SageGUI is a desktop graphical interface for the Sage proteomics search engine. It is
the NIST-maintained fork of `jspaezp/sagegui`. See [README.md](README.md) for what it
does and how to build it.

---

## ⛔ STOP: precursor tolerance sign convention

**Sage's `precursor_tol` pair is NOT the delta-mass range. It is negated AND
reversed.** This has been got wrong repeatedly. Read this before writing,
reviewing, or describing any tolerance value.

Sage searches candidate **theoretical** masses in
`[experimental + lower, experimental + upper]` (`Tolerance::bounds`,
`vendor/sage/crates/sage/src/mass.rs`; the centre is the *experimental* mass, see
`db.query(precursor_mass, ...)` in `vendor/sage/crates/sage/src/scoring.rs`).
Delta mass, the modification a PSM carries, is `experimental - theoretical`. So:

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
are using, every time. Symmetric ppm windows (`[-10, 10]`) hide the mistake. It
only becomes visible on asymmetric and open windows, which is exactly where it
does damage.

---

## Repository layout

- `src/`: the application (Rust, egui/eframe).
- `vendor/sage/`: the Sage source, vendored at a fixed upstream commit, plus our
  small additive patches. [VENDORED.md](vendor/sage/VENDORED.md) names the base
  commit; [PATCHES.md](vendor/sage/PATCHES.md) lists every change.
- `assets/`: icons, logo and the bundled search templates (`assets/templates/`).
- `docs/`: user documentation and the parameter reference.
- `tests/`: fixtures and schemas for the converter tests.
- `_dev/`: the development record (plan, notes, journal, the full agent protocol).
  Not needed to build or run the program. See [_dev/README.md](_dev/README.md).

## Settled decisions (do not re-derive)

These are locked. [_dev/NOTES.md](_dev/NOTES.md) holds the reasoning and evidence
for each one.

- **Sage is vendored, not a Git fork or a subprocess.** It is compiled in from
  `vendor/sage`. Patches stay additive and, where possible, inside
  `crates/sage-cli/src/runner.rs`; the feature logic stays in `src/`. Record every
  patch in `vendor/sage/PATCHES.md`. Update Sage only by the re-vendor runbook in
  [MAINTENANCE.md](MAINTENANCE.md).
- **Never change the `eframe::run_native("Sage Launcher", ...)` string.** eframe
  derives the saved-settings folder from it. Changing it silently resets every
  user's settings.
- **Never rename the `_sagegui` key** in the template JSON. It would break every
  bundled and every saved template.
- **Every new `Config` or `DatabaseConfig` field needs `#[serde(default)]`.**
  Without it, settings saved by an older version fail to load and every setting
  resets.
- **`Cargo.lock` is committed.**
- **Licence:** NIST code is under `LICENSE.md`. Code derived from jspaezp/sagegui
  keeps its "Derived from" header and stays Apache-2.0. Keep
  [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) in step with the files.

## Working here

- Commit to `main` with plain messages. No branches or PRs unless asked. Commits
  are authored `Ben Neely <benjamin.neely@nist.gov>`. No commit carries an AI
  co-author trailer.
- Write in ASD-STE100: short sentences, one idea each. Never use an em dash (`—`),
  in code comments, commit messages, docs or the `_dev` record.
- Targeted edits only. Keep `README.md` and `CHANGELOG.md` in step with what they
  describe.
- For any substantial work, read [_dev/dev_AGENTS.md](_dev/dev_AGENTS.md) first. It
  holds the full protocol: reading order, session start and shutdown, and the
  debrief.

## AI use

This project uses AI coding agents under human review. See
[docs/AI_USAGE.md](docs/AI_USAGE.md).
