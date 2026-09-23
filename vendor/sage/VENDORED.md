# Vendored Sage source

This directory holds a copy of the source of [Sage](https://github.com/lazear/sage), the
proteomics search engine by Michael Lazear (MIT License, see [LICENSE](LICENSE)). SageGUI
compiles Sage in as a library, so a search runs in the same process as the interface. We
vendor the source, rather than depend on a Git fork of Sage, so that this repository
builds from its own contents and so that every change we make to Sage is visible in one
place.

## Source

| Field | Value |
| --- | --- |
| Upstream repository | https://github.com/lazear/sage |
| Base commit | `d74024df774054fa411a9d5cca6013ce91d26208` (`master`, 2026-05-03, "fix: track charges on deisotoped ions (#220)") |
| Relation to releases | 10 commits after the `v0.15.0-beta.2` tag (`df9219951cc9a54cf4cd55d76541af24b687bd3d`); `git describe` gives `v0.15.0-beta.2-10-gd74024d` |
| Crate version string | `0.15.0-beta.2` (upstream had not bumped it at this commit) |
| Date vendored | 2026-09-23 |

## What is copied

We copy the three crates SageGUI uses, `crates/sage` (package `sage-core`),
`crates/sage-cli` and `crates/sage-cloudpath`, together with the upstream `LICENSE`. We do
not copy the upstream workspace `Cargo.toml`, `Cargo.lock`, documentation, figures, Docker
files or the `tests/` data directory. The crates refer to each other with relative
`path` dependencies, so they build unchanged from this location, and SageGUI's own
`Cargo.lock` pins every third-party dependency. The upstream integration test in
`crates/sage-cli/tests/integration.rs` reads `../../tests/`, which is not copied, so that
test does not run from here (SageGUI's `cargo test` does not run it in any case).

## Local modifications

The first commit that adds this directory is byte-identical to upstream at the base
commit. Each later change to Sage is a separate commit and is listed, with its reason, in
[PATCHES.md](PATCHES.md). To see exactly what we changed, run
`git log -p -- vendor/sage` or compare this directory against a checkout of the base
commit.

Updating to a newer Sage is described in [MAINTENANCE.md](../../MAINTENANCE.md).
