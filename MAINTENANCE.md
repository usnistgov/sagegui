# Maintaining SageGUI

This guide explains how to keep SageGUI up-to-date with new Sage releases.

## Overview

SageGUI compiles Sage in from a vendored copy of its source in `vendor/sage/`: upstream
[lazear/sage](https://github.com/lazear/sage) at a fixed commit, plus a small set of
additive NIST patches. `vendor/sage/VENDORED.md` names the base commit, and
`vendor/sage/PATCHES.md` lists every patch. When upstream Sage moves forward, we update
the copy by hand, following the steps below. We do not track upstream automatically:
moving to a newer Sage is a deliberate, tested and recorded change.

**Estimated effort:** 1 to 2 hours per Sage update, assuming no major API changes.

---

## Updating to a New Sage Version

### Step 1: Copy the new upstream crates as one pristine commit

```bash
# Anywhere outside this repository
git clone https://github.com/lazear/sage.git
cd sage
git checkout NEW_COMMIT          # a release tag, or a master commit
git describe --tags NEW_COMMIT   # for example v0.15.0-beta.2-10-gd74024d

# In this repository: replace the three crates, keep our docs
rm -rf vendor/sage/crates
cp -R /path/to/sage/crates vendor/sage/crates
cp /path/to/sage/LICENSE vendor/sage/LICENSE
```

Update the table in `vendor/sage/VENDORED.md` (base commit, relation to releases, crate
version string, date vendored). Commit this alone, before any patch. At this point the
build usually fails, because our patches are gone. That is expected: this commit exists
so that `git log -p -- vendor/sage` separates upstream code from ours.

### Step 2: Re-apply each patch in PATCHES.md as its own commit

Take the patches in order. For each one, find the commit that last applied it
(`git log --oneline -- vendor/sage`) and replay its diff:

```bash
git show PATCH_COMMIT -- vendor/sage/crates | git apply --3way
```

If upstream has rewritten the same lines, `git apply --3way` stops with a conflict.
Resolve it by hand: keep the upstream change and re-add ours. All patches so far touch
only `crates/sage-cli/src/runner.rs`: the `Runner` struct, both `Self { ... }` literals in
`Runner::new` (including the prefilter `mini_runner`), `search_processed_spectra`,
`process_chunk` and `run()`. Keep the NIST header at the top of `runner.rs`. Commit each
patch separately with a message that names it, and update its PATCHES.md entry if the
code moved.

If upstream has adopted one of our patches, drop it and delete its PATCHES.md entry.

### Step 3: Update SageGUI's record of the Sage version

Edit `src/version.rs`:

```rust
pub const SAGE_VERSION: &str = "vX.Y.Z";              // nearest release tag (README badge)
pub const SAGE_DESCRIBE: &str = "vX.Y.Z-N-gABCDEF0";  // git describe of the base commit
pub const SAGE_COMMIT: &str = "FULL_BASE_COMMIT_SHA";
pub const SAGE_COMMIT_SHORT: &str = "ABCDEF01";
// and SAGE_RELEASE_URL, SAGE_COMMIT_URL
```

`Cargo.toml` needs no change: the `path` dependencies point at `vendor/sage/crates/`.
Run `cargo build` so `Cargo.lock` picks up any new version string or new dependency of
Sage, and review that diff.

### Step 4: Fix API Changes

Run `cargo check` to see if there are any compilation errors:

```bash
cargo check
```

Common API changes to watch for:
- **Input struct**: New fields added (set to `None` or sensible defaults)
- **Builder struct**: New database configuration options
- **Runner signature**: Constructor parameter changes
- **Quantification options**: New LFQ/TMT settings

See [API Changes Fixed](#api-changes-reference) below for examples.

### Step 5: Test

```bash
# Run automated tests
cargo test

# Build release
cargo build --release

# Manual test with real data
./target/release/sagegui
# 1. Load an mzML file
# 2. Load a FASTA database
# 3. Run a search
# 4. Verify output files are created
```

**Regression check against a past run.** The stock Sage command-line tool builds from
the vendored source, and it takes a SageGUI `results.json` as its config. Re-running a
past search with it isolates the engine from the interface. Always pass the telemetry
flag, or Sage sends a usage report to its author's server:

```bash
cargo build --release -p sage-cli --bin sage
./target/release/sage --disable-telemetry-i-dont-want-to-improve-sage \
    -o /tmp/sage-check /path/to/past/run/results.json
```

Compare `results.sage.tsv` with the past run, row by row on `filename`, `scannr`,
`peptide` and `charge`. With no upstream change, every column except `psm_id` should
match (`psm_id` follows parallel scheduling order). After a real upstream update,
differences are expected. Explain them in the CHANGELOG before release.

### Step 6: Update Documentation

1. Update `CHANGELOG.md` with the new version
2. Update README badges if needed (automated via `update-badges.yml`)
3. **Bump `version:` and `date-released:` in `CITATION.cff`.** Nothing checks
   these against `Cargo.toml`, and a stale value makes the citation point at a
   release nobody ran. Do not automate this in `update-badges.yml`: that
   workflow triggers on `src/version.rs`, which carries the *Sage* version, not
   the SageGUI version.
4. Commit all changes

### Step 7: Release

```bash
# Commit changes
git add -A
git commit -m "Update to Sage vX.Y.Z"
git push origin main

# Create release tag
git tag -a vX.Y.Z -m "Release vX.Y.Z - Updated to Sage vX.Y.Z"
git push origin vX.Y.Z
```

The GitHub Actions workflow will automatically build binaries and create the release.

---

## API Changes Reference

### v0.14.7 → v0.15.0-beta.2 Changes

| Component | Change | Fix |
|-----------|--------|-----|
| `EnzymeBuilder.restrict` | `Option<char>` → `Option<String>` | `.map(\|c\| c.to_string())` |
| `Builder` | New fields | Add `prefilter: None`, `prefilter_chunk_size: None`, `prefilter_low_memory: None` |
| `LfqOptions` | New fields | Add `mobility_pct_tolerance: None`, `peptide_q_value: None` |
| `Input` | Field renamed | `bruker_spectrum_processor` → `bruker_config` |
| `Input` | New fields | Add `protein_grouping: None`, `protein_grouping_peptide_fdr: None`, `write_report: None` |
| `Runner::new` | Signature change | `Runner::new(search)` → `Runner::new(search, parallel)` |

### Finding API Changes

When updating, compare the Sage source code:

```bash
# In a clone of lazear/sage (OLD_COMMIT is the base in vendor/sage/VENDORED.md)
git diff OLD_COMMIT..NEW_COMMIT -- crates/sage-cli/src/input.rs
git diff OLD_COMMIT..NEW_COMMIT -- crates/sage/src/database.rs
```

Key files to check:
- `crates/sage-cli/src/input.rs`: Input struct definition
- `crates/sage/src/database.rs`: Builder struct
- `crates/sage/src/lfq.rs`: LFQ options
- `crates/sage-cli/src/runner.rs`: Runner implementation

---

## Testing Checklist

Before releasing a new version:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `cargo build --release` succeeds
- [ ] Manual test: Load mzML files
- [ ] Manual test: Load FASTA database
- [ ] Manual test: Configure search parameters
- [ ] Manual test: Run search successfully
- [ ] Manual test: Output files created (results.sage.tsv, etc.)
- [ ] Manual test: LFQ quantification (if enabled)
- [ ] Manual test: TMT quantification (if TMT data available)

---

## Troubleshooting

### "unresolved import" errors

Sage may have reorganized modules. Check the new module structure:

```bash
# In sage repo
find . -name "*.rs" | xargs grep "pub struct YourStruct"
```

### "missing field" errors

New required fields were added. Check the struct definition in Sage and add the new fields with appropriate defaults (usually `None` for optional fields).

### "wrong number of arguments" errors

Function signatures changed. Check the function definition in Sage for the new signature.

### Build fails on specific platform

Check the GitHub Actions logs for the failing platform. Common issues:
- Missing system dependencies on Linux (libxcb, etc.)
- macOS SDK version issues
- Windows path length limits

---

## Project Structure

```
sagegui/
├── src/
│   ├── main.rs          # All GUI code (single file)
│   └── version.rs       # Sage version constants
├── assets/
│   └── sagegui_logo-removebg.png
├── .github/
│   ├── workflows/
│   │   ├── build.yml    # CI/CD pipeline
│   │   └── update-badges.yml
│   └── dependabot.yml   # Auto-update dependencies
├── Cargo.toml           # Dependencies (Sage commit hash here)
├── AGENTS.md            # Agent/contributor working protocol
├── CHANGELOG.md         # Release history
├── MAINTENANCE.md       # This file
├── PLAN.md              # Development roadmap
├── NOTES.md             # Locked decisions, gotchas, reference
├── JOURNAL.md           # Append-only session history
└── README.md            # User-facing documentation
```

---

## Memory Allocation

**Sage does not expose a maximum RAM setting.** Allocation is automatic and managed by the Rust runtime. Memory usage depends on:

1. **Database size**: Larger FASTA files → larger fragment index in RAM
2. **Batching**: Spectra are processed in batches; tune batch size if needed (see "Performance tuning")
3. **Quantification**: LFQ keeps identified peptides in memory; lower `peptide_q_value` threshold reduces this
4. **Prefiltering**: Enable `prefilter_low_memory` for chunk-based FASTA processing (slower but lower peak memory)

### If you're running out of memory:

- **Reduce `max_peaks`** (default 150): fewer peaks per spectrum = less processing overhead
- **Enable `prefilter_low_memory`**: process FASTA in smaller chunks during database build
- **Lower `peptide_q_value`** threshold if using LFQ: only keep high-confidence peptides in RAM
- **Use OS-level limits** (Linux: `ulimit -v`, Windows: Job Objects) to constrain the process
- **Split large datasets**: run smaller files separately rather than concatenating them

This is by design: Sage trades "set-it-and-forget-it" configuration for speed. The automagic memory management works well for typical proteomics datasets.

---

## Contact

- **Repository:** https://github.com/neely/sagegui
- **Sage upstream:** https://github.com/lazear/sage
- **Original sagegui:** https://github.com/jspaezp/sagegui
