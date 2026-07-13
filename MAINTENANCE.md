# Maintaining SageGUI

This guide explains how to keep SageGUI up-to-date with new Sage releases.

## Overview

SageGUI depends on a fork of Sage at [neely/sage](https://github.com/neely/sage). When the official [lazear/sage](https://github.com/lazear/sage) releases a new version, follow this guide to update SageGUI.

**Estimated effort:** 1-2 hours per Sage release (assuming no major API changes).

---

## Updating to a New Sage Version

### Step 1: Sync the Sage Fork

```bash
# Clone your fork if you haven't already
git clone https://github.com/neely/sage.git
cd sage

# Add upstream remote (only needed once)
git remote add upstream https://github.com/lazear/sage.git

# Fetch and merge upstream changes
git fetch upstream
git checkout main
git merge upstream/main

# Push to your fork
git push origin main
```

### Step 2: Get the New Commit Hash

```bash
# Get the commit hash of the version you want
git log --oneline -1
# Example output: d74024df Add new feature...
```

### Step 3: Update SageGUI

Edit `src/version.rs` with the new version info:

```rust
pub const SAGE_VERSION: &str = "v0.15.0-beta.2";  // Update this
pub const SAGE_COMMIT: &str = "d74024df";          // Update this
pub const SAGE_REPO: &str = "https://github.com/neely/sage";
pub const SAGE_UPSTREAM: &str = "https://github.com/lazear/sage";
```

Edit `Cargo.toml` to point to the new commit:

```toml
[dependencies]
sage-core = { git = "https://github.com/neely/sage.git", rev = "NEW_COMMIT_HASH" }
sage-cli = { git = "https://github.com/neely/sage.git", rev = "NEW_COMMIT_HASH" }
```

### Step 4: Fix API Changes

Run `cargo check` to see if there are any compilation errors:

```bash
cargo check
```

Common API changes to watch for:
- **Input struct** — New fields added (set to `None` or sensible defaults)
- **Builder struct** — New database configuration options
- **Runner signature** — Constructor parameter changes
- **Quantification options** — New LFQ/TMT settings

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

### Step 6: Update Documentation

1. Update `CHANGELOG.md` with the new version
2. Update README badges if needed (automated via `update-badges.yml`)
3. Commit all changes

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
# In your sage fork
git diff OLD_COMMIT..NEW_COMMIT -- crates/sage-cli/src/input.rs
git diff OLD_COMMIT..NEW_COMMIT -- crates/sage-core/src/database.rs
```

Key files to check:
- `crates/sage-cli/src/input.rs` — Input struct definition
- `crates/sage-core/src/database.rs` — Builder struct
- `crates/sage-core/src/lfq.rs` — LFQ options
- `crates/sage-cli/src/runner.rs` — Runner implementation

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
├── CHANGELOG.md         # Release history
├── MAINTENANCE.md       # This file
├── PLAN.md              # Development roadmap
├── CONTEXT.md           # Background knowledge
└── README.md            # User-facing documentation
```

---

## Contact

- **Repository:** https://github.com/neely/sagegui
- **Sage upstream:** https://github.com/lazear/sage
- **Original sagegui:** https://github.com/jspaezp/sagegui
