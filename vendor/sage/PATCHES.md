# Changes to vendored Sage

This file lists every change we (NIST) made to the vendored Sage source, relative to the
upstream base commit named in [VENDORED.md](VENDORED.md). Each entry is one commit in this
repository, so `git log -p -- vendor/sage` shows the exact lines. There are two kinds:

- **Code patches (1 to 3)** are additive. They add fields and methods that the stock Sage
  command-line tool never calls, and they change no existing behaviour.
- **Dependency patches (4 to 6)** change a version or a feature list in a `Cargo.toml`, to
  remove a package with a security advisory. Each one names the advisories it clears. At
  each update, check whether upstream has made the same change. If it has, drop the patch.

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

## 5. `reqwest` 0.11 to 0.12 in `sage-cloudpath`

- **File:** `crates/sage-cloudpath/Cargo.toml` (one version line).
- **Written:** 2026-09-24, by Benjamin A. Neely (NIST).
- **What:** `reqwest` goes from `0.11` to `0.12`. The features stay the same
  (`json`, `rustls-tls`, no default features). The one call site, `send_data` in
  `src/util.rs`, compiles unchanged.
- **Why:** `reqwest` 0.11 uses `rustls` 0.21 and `rustls-webpki` 0.101, which has three
  advisories and no patched 0.101 release (GHSA-82j2-j2ch-gfr8, GHSA-xgp8-3hg3-c2mh,
  GHSA-965h-392x-2mh5). `reqwest` 0.12 shares `rustls` 0.23 with `object_store`, so the
  old TLS stack leaves the tree.
- **At the next update:** drop this patch if upstream has moved past 0.11.

## 6. `parquet` 50 to 59 and fewer `timsrust` features in `sage-cloudpath`

- **Files:** `crates/sage-cloudpath/Cargo.toml` (two lines). SageGUI's own `Cargo.toml`
  sets the same `timsrust` features, because Cargo merges the features of both lines.
- **Written:** 2026-09-24, by Benjamin A. Neely (NIST).
- **What:** `parquet` goes from `50.0.0` to `59`. `timsrust` loses its default features
  and keeps `tdf` and `serialize`, so its `minitdf` feature is off. The Parquet writer
  code in `src/parquet.rs` compiles unchanged.
- **Why:** `thrift` 0.17 has an advisory (GHSA-2f9f-gq7v-9h6m). It came in twice: through
  `parquet` 50, and through `parquet` 53, which the `timsrust` `minitdf` feature pulls in.
  `parquet` 59 does not use the `thrift` crate.
- **Effect:** Sage can no longer read Bruker miniTDF input. SageGUI does not offer Bruker
  input (see NOTES, "Scope decision: mzML/.gz only"), so users lose nothing. Bruker `.d`
  (TDF) reading still compiles.
- **Checked:** the serum search (2026-09-22 `results.json`) with `--parquet
  --annotate-matches`, before and after. `results.sage.parquet` (32,221 rows) and
  `matched_fragments.sage.parquet` (264,036 rows) are identical in every column except
  `psm_id`. `lfq.parquet` has the same 3,249 rows; its row order changes from run to run
  even without this patch.
- **At the next update:** drop this patch if upstream uses a `parquet` without `thrift`
  0.17. Keep the `timsrust` features unless upstream has moved past `timsrust` 0.4.
