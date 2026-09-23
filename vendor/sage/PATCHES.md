# Changes to vendored Sage

This file lists every change we (NIST) made to the vendored Sage source, relative to the
upstream base commit named in [VENDORED.md](VENDORED.md). Each entry is one commit in this
repository, so `git log -p -- vendor/sage` shows the exact lines. All changes so far are
additive: they add fields and methods that the stock Sage command-line tool never calls,
and they change no existing behaviour.

We keep patches small and confined to `crates/sage-cli/src/runner.rs` where possible, and
we keep feature logic in SageGUI itself, because every edited line in Sage has to be
re-applied by hand at the next update (see [MAINTENANCE.md](../../MAINTENANCE.md)).

## 1. Progress counter on `Runner`

- **File:** `crates/sage-cli/src/runner.rs` (8 added lines).
- **Written:** 2026-08-21, by Benjamin A. Neely (NIST). First carried as `neely/sage`
  commit `b51e9ac`.
- **What:** a public `progress: Arc<AtomicUsize>` field on `Runner`. It is incremented
  once per MS2 spectrum in `search_processed_spectra`, next to the existing local
  counter, and it is initialised in both `Runner` literals in `Runner::new` (the normal
  path and the prefilter `mini_runner`).
- **Why:** SageGUI clones the `Arc` before calling `run()` and polls it from the
  interface thread, so the progress bar shows real search progress. Stock Sage reports
  progress only as log lines.
