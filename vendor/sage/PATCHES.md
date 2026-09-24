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

## 2. Cooperative cancellation on `Runner`

- **File:** `crates/sage-cli/src/runner.rs` (37 added lines).
- **Written:** 2026-08-24, by Benjamin A. Neely (NIST). First carried as `neely/sage`
  commit `ed5f06c`.
- **What:** a public `cancel: Arc<AtomicBool>` field on `Runner`, off by default, and a
  `with_cancel(flag)` builder method that replaces it with a flag the caller owns.
  `run()` checks the flag in three places: before each per-spectrum `score()` call in
  `search_processed_spectra`, between file chunks in `process_chunk`, and once more after
  scoring (before FDR, protein grouping, quantification or any output), where it returns
  the error `"cancelled"`.
- **Why:** SageGUI's Stop button sets the flag. Without it, a started search could not be
  interrupted. The last check guarantees that a cancelled run writes no file that looks
  like a completed search. The stock command-line tool never calls `with_cancel`, so its
  behaviour is unchanged.

## 3. `Runner::from_parts`

- **File:** `crates/sage-cli/src/runner.rs` (one new method, about 15 lines with its
  documentation).
- **Written:** 2026-09-23, by Benjamin A. Neely (NIST). This is the first patch made
  directly in the vendored copy.
- **What:** a public constructor that takes a `Search` and an `IndexedDatabase` that was
  already built, and returns a `Runner` without reading or digesting the FASTA. It does
  not change the `Runner` struct or `Runner::new`. It does not check that the database
  matches the parameters.
- **Why:** SageGUI can cache the built peptide database on disk (the "Cache prepared
  database" option on Files & Database). On a later run with the same FASTA content and
  the same database settings, SageGUI loads the cached database and passes it here,
  which skips the database build (about two minutes for a human proteome). SageGUI's
  cache key covers every parameter that affects the build; see `src/index_cache.rs`.

## 4. `env_logger` 0.8 to 0.11 in `sage-cli`

- **File:** `crates/sage-cli/Cargo.toml` (one version line).
- **Written:** 2026-09-24, by Benjamin A. Neely (NIST).
- **What:** `env_logger = "0.8.4"` becomes `env_logger = "0.11"`. The code in
  `main.rs` compiles unchanged.
- **Why:** `env_logger` 0.8 depends on `atty`, which has an advisory with no fix
  (GHSA-g98v-hv3f-hcfr). `env_logger` 0.11 does not use `atty`. SageGUI already
  uses 0.11, so the tree now has one copy.
- **At the next update:** drop this patch if upstream has moved past 0.8.
