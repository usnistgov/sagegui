# Changes to vendored Sage

This file lists every change we (NIST) made to the vendored Sage source, relative to the
upstream base commit named in [VENDORED.md](VENDORED.md). Each entry is one commit in this
repository, so `git log -p -- vendor/sage` shows the exact lines. There are two kinds:

- **Code patches (1 to 3)** are additive. They add fields and methods that the stock Sage
  command-line tool never calls, and they change no existing behaviour.
- **Dependency patches (4 and 6)** change a version in a `Cargo.toml`, to remove a
  package with a security advisory. Each one names the advisories it clears. At each
  update, check whether upstream has made the same change. If it has, drop the patch.
  Patch 5 was dropped on 2026-11-12, because upstream made the same change.

The numbers are fixed. A dropped patch keeps its number, so older commits and notes
still point at the right entry.

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
- **At `4a051b2`:** upstream added its own private `spectra_searched` counter, which it
  sends with telemetry. It is not public and not live, so it does not replace this
  field. Both fields are kept. Re-applying the patch conflicted on the struct and on both
  literals; the fix keeps both lines in each place.

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
- **At `4a051b2`:** the literal also sets upstream's new `spectra_searched` field.

## 4. `env_logger` 0.8 to 0.11 in `sage-cli`

- **File:** `crates/sage-cli/Cargo.toml` (one version line).
- **Written:** 2026-09-24, by Benjamin A. Neely (NIST).
- **What:** `env_logger = "0.8.4"` becomes `env_logger = "0.11"`. The code in
  `main.rs` compiles unchanged.
- **Why:** `env_logger` 0.8 depends on `atty`, which has an advisory with no fix
  (GHSA-g98v-hv3f-hcfr). `env_logger` 0.11 does not use `atty`. SageGUI already
  uses 0.11, so the tree now has one copy.
- **At the next update:** drop this patch if upstream has moved past 0.8.

## 5. `reqwest` 0.11 to 0.12 in `sage-cloudpath` (dropped 2026-11-12)

- **Dropped:** upstream commit `f61a75c` ("migrate sage-cloudpath to reqwest 0.12") made
  the same change, so the re-vendor at `4a051b2` does not re-apply it. Upstream also
  changed the TLS feature to `rustls-tls-native-roots`. The `rustls` 0.21 and
  `rustls-webpki` 0.101 advisories stay cleared.
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

## 6. `parquet` 50 to 59 in `sage-cloudpath`

- **File:** `crates/sage-cloudpath/Cargo.toml` (one version line).
- **Written:** 2026-09-24, by Benjamin A. Neely (NIST). Narrowed on 2026-11-12.
- **What:** `parquet` goes from `50.0.0` to `59`. The Parquet writer code in
  `src/parquet.rs` compiles unchanged.
- **Why:** SageGUI offers Parquet output, so this is the `parquet` that SageGUI runs.
  `parquet` 50 uses `thrift` 0.17 (GHSA-2f9f-gq7v-9h6m, RUSTSEC-2026-0435). `parquet` 59
  does not use the `thrift` crate.
- **History:** until 2026-11-12 this patch also turned off the `timsrust` default
  features, to drop the `minitdf` reader and the `parquet` 53 it pulled in. Upstream
  `4a051b2` moved to `timsrust` 0.6.7, which is split into crates and has no such
  feature, so that half is gone. SageGUI's own `timsrust` line in `Cargo.toml` is gone too.
- **Accepted advisories (decision 2026-11-12):** `timsrust` 0.6 depends on its
  `filemanager` crate with default features. That brings `parquet` 57 with `thrift` 0.17,
  `rusqlite` with `libsqlite3-sys` 0.33 (RUSTSEC-2026-0459), `object_store` 0.11 with
  `quick-xml` 0.37 (RUSTSEC-2026-0194, -0195) and `rustls-pemfile` (unmaintained). Ben
  chose to take upstream's Bruker reader as it is, and not to patch it out. Only Bruker
  `.d` reading reaches this code, and SageGUI offers no Bruker input. Bruker becomes a
  SageGUI feature only after our own tests on timsTOF data. Recheck at each update:
  drop this note once `timsrust` trims those features.
- **Checked:** the serum search (2026-09-22 `results.json`) with `--parquet
  --annotate-matches`, before and after the 2026-09-24 change. `results.sage.parquet`
  (32,221 rows) and `matched_fragments.sage.parquet` (264,036 rows) are identical in
  every column except `psm_id`. `lfq.parquet` has the same 3,249 rows; its row order
  changes from run to run even without this patch.
- **At the next update:** drop this patch if upstream uses a `parquet` without `thrift`
  0.17.
