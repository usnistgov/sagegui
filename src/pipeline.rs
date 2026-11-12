// SageGUI's own copy of the Sage search pipeline, with progress steps.
//
// Most functions here are copied from lazear/sage (MIT License, Michael
// Lazear), `crates/sage-cli/src/runner.rs` at commit 4a051b2 (vendored in
// vendor/sage). Each copied function says which upstream function it copies.
// Copied by Benjamin A. Neely (NIST) on 2026-11-12. The copies keep the
// upstream logic and order. The changes are the progress events, the cancel
// checks and the log targets. See THIRD_PARTY_LICENSES.md.
//
// Why a copy: Sage is a library, but `Runner::new` and `Runner::run` are two
// closed blocks. Running the steps here lets SageGUI report each one (FASTA,
// digest, fragment index, spectra, search, FDR, quant, write) and stop
// between them, without patching Sage. The writers are still Sage's own, on
// a `Runner` made with `Runner::from_parts`.
//
// Keeping in step with Sage: at each re-vendor, diff upstream `Runner::new`,
// `Runner::run` and every function named below against this file, and port
// the changes. The test `pipeline_matches_stock_runner` runs stock Sage and
// this pipeline on the same input and compares the output files.

// The copied code keeps upstream's spelling, so a diff against `runner.rs`
// stays small. Clippy prefers newer forms of two of its idioms.
#![allow(clippy::manual_is_multiple_of, clippy::manual_div_ceil)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use rayon::prelude::*;
use sage_cli::input::Search;
use sage_cli::output::SageResults;
use sage_cli::runner::Runner;
use sage_cloudpath::{FileFormat, Url};
use sage_core::database::{IndexedDatabase, Parameters};
use sage_core::fasta::Fasta;
use sage_core::mass::Tolerance;
use sage_core::peptide::Peptide;
use sage_core::scoring::{Feature, Scorer};
use sage_core::spectrum::{ProcessedSpectrum, RawSpectrum, SpectrumProcessor};

/// Log target for the copied log lines. The GUI logger forwards only
/// `sage_*` targets to the Sage Log panel, and the default filter shows
/// `sage_cli` at `info`. Using Sage's own target keeps the panel the same.
const LOG: &str = "sage_cli::runner";

/// The error text of a stopped run. `main.rs` matches on it.
pub const CANCELLED: &str = "cancelled";

/// One step of a search, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    ReadFasta,
    Digest,
    BuildIndex,
    ReadSpectra,
    Search,
    Fdr,
    Quant,
    Write,
}

impl Step {
    pub const ALL: [Step; 8] = [
        Step::ReadFasta,
        Step::Digest,
        Step::BuildIndex,
        Step::ReadSpectra,
        Step::Search,
        Step::Fdr,
        Step::Quant,
        Step::Write,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Step::ReadFasta => "Read FASTA",
            Step::Digest => "Digest proteins",
            Step::BuildIndex => "Build fragment index",
            Step::ReadSpectra => "Read spectra",
            Step::Search => "Search",
            Step::Fdr => "Score and FDR",
            Step::Quant => "Quantify",
            Step::Write => "Write results",
        }
    }
}

/// What the pipeline reports to the GUI.
#[derive(Debug, Clone)]
pub enum Event {
    Started(Step),
    /// Text for the right side of the step row, while it runs.
    Detail(Step, String),
    /// `done` of `total` units (files for ReadSpectra, spectra for Search).
    Counter(Step, usize, usize),
    /// The step is done. The text sums it up, for example "20,421 proteins".
    Finished(Step, String),
    /// The step does not run in this search. The text says why.
    Skipped(Step, String),
}

/// How the pipeline talks to its caller.
pub struct Hooks<'a> {
    /// Set by the Stop button. Checked between steps and per spectrum.
    pub cancel: &'a AtomicBool,
    /// Spectra searched so far, polled by the GUI for the Search bar.
    pub searched: Arc<AtomicUsize>,
    /// Receives each event. Called from worker threads too.
    pub emit: &'a (dyn Fn(Event) + Sync),
}

impl Hooks<'_> {
    fn emit(&self, event: Event) {
        (self.emit)(event)
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn check(&self) -> anyhow::Result<()> {
        if self.cancelled() {
            anyhow::bail!(CANCELLED);
        }
        Ok(())
    }
}

/// A number with thousands separators, for step details.
pub fn count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn secs(d: Duration) -> String {
    format!("{:.1} s", d.as_secs_f64())
}

// ─── Database build (copies `Runner::new`) ────────────────────────────────────

/// Reads the FASTA and builds the peptide database, step by step.
///
/// Copies `Runner::new`. Returns the parameters too, because the prefilter
/// path writes an automatic chunk size into them, and `results.json` records
/// it, as stock Sage does.
pub fn build_database(
    mut parameters: Search,
    parallel: usize,
    hooks: &Hooks,
) -> anyhow::Result<(Search, IndexedDatabase)> {
    let start = Instant::now();

    hooks.emit(Event::Started(Step::ReadFasta));
    let fasta_url = sage_cloudpath::to_url(&parameters.database.fasta)?;
    let fasta = sage_cloudpath::util::read_fasta(
        &fasta_url,
        &parameters.database.decoy_tag,
        parameters.database.generate_decoys,
    )
    .with_context(|| {
        format!(
            "Failed to build database from `{}`",
            parameters.database.fasta
        )
    })?;
    hooks.emit(Event::Finished(
        Step::ReadFasta,
        format!(
            "{} proteins · {}",
            count(fasta.targets.len()),
            secs(start.elapsed())
        ),
    ));
    hooks.check()?;

    let database = match parameters.database.prefilter {
        false => full_build(&parameters.database, fasta, hooks)?,
        true => {
            parameters
                .database
                .auto_calculate_prefilter_chunk_size(&fasta);
            if parameters.database.prefilter_chunk_size >= fasta.targets.len() {
                full_build(&parameters.database, fasta, hooks)?
            } else {
                let chunks = (fasta.targets.len() + parameters.database.prefilter_chunk_size - 1)
                    / parameters.database.prefilter_chunk_size;
                log::info!(
                    target: LOG,
                    "using {} db chunks of size {}",
                    chunks,
                    parameters.database.prefilter_chunk_size,
                );
                let step_start = Instant::now();
                hooks.emit(Event::Started(Step::Digest));
                let peptides = prefilter_peptides(&parameters, parallel, fasta, chunks, hooks)?;
                hooks.emit(Event::Finished(
                    Step::Digest,
                    format!(
                        "prefiltered to {} peptides · {}",
                        count(peptides.len()),
                        secs(step_start.elapsed())
                    ),
                ));
                hooks.check()?;
                index(&parameters.database, peptides, hooks)
            }
        }
    };

    log::info!(
        target: LOG,
        "generated {} fragments, {} peptides in {:#?}",
        database.fragments.len(),
        database.peptides.len(),
        (start.elapsed())
    );
    Ok((parameters, database))
}

/// `Parameters::build`, split in its two halves so each is a step.
fn full_build(params: &Parameters, fasta: Fasta, hooks: &Hooks) -> anyhow::Result<IndexedDatabase> {
    let start = Instant::now();
    hooks.emit(Event::Started(Step::Digest));
    let peptides = params.digest(&fasta);
    drop(fasta);
    hooks.emit(Event::Finished(
        Step::Digest,
        format!(
            "{} peptides · {}",
            count(peptides.len()),
            secs(start.elapsed())
        ),
    ));
    hooks.check()?;
    Ok(index(params, peptides, hooks))
}

fn index(params: &Parameters, peptides: Vec<Peptide>, hooks: &Hooks) -> IndexedDatabase {
    let start = Instant::now();
    hooks.emit(Event::Started(Step::BuildIndex));
    let database = params.clone().build_from_peptides(peptides);
    hooks.emit(Event::Finished(
        Step::BuildIndex,
        format!(
            "{} fragments · {}",
            count(database.fragments.len()),
            secs(start.elapsed())
        ),
    ));
    database
}

/// Copies `Runner::prefilter_peptides`. Adds a detail line per FASTA chunk and
/// a cancel check between chunks.
fn prefilter_peptides(
    parameters: &Search,
    parallel: usize,
    fasta: Fasta,
    chunks: usize,
    hooks: &Hooks,
) -> anyhow::Result<Vec<Peptide>> {
    let spectra: Option<Vec<ProcessedSpectrum>> = match parallel >= parameters.mzml_paths.len() {
        true => Some(read_processed_spectra(parameters, &parameters.mzml_paths, 0, 0, None).1),
        false => None,
    };

    let db_params = parameters.database.clone();
    // Upstream TODO kept as is: decoys are made per chunk.

    let mut all_peptides: Vec<Peptide> = Vec::new();
    for (chunk_id, fasta_chunk) in fasta
        .iter_chunks(parameters.database.prefilter_chunk_size)
        .enumerate()
    {
        hooks.check()?;
        hooks.emit(Event::Detail(
            Step::Digest,
            format!("prefilter chunk {} of {}", chunk_id + 1, chunks),
        ));
        let start = Instant::now();
        log::info!(target: LOG, "pre-filtering fasta chunk {}", chunk_id,);
        let mut db = db_params.clone().build(fasta_chunk);

        log::info!(
            target: LOG,
            "generated {} fragments, {} peptides in {}ms",
            db.fragments.len(),
            db.peptides.len(),
            (Instant::now() - start).as_millis()
        );

        let scorer = Scorer {
            db: &db,
            precursor_tol: parameters.precursor_tol,
            fragment_tol: parameters.fragment_tol,
            min_matched_peaks: parameters.min_matched_peaks,
            min_isotope_err: parameters.isotope_errors.0,
            max_isotope_err: parameters.isotope_errors.1,
            min_precursor_charge: parameters.precursor_charge.0,
            max_precursor_charge: parameters.precursor_charge.1,
            override_precursor_charge: parameters.override_precursor_charge,
            max_fragment_charge: parameters.max_fragment_charge,
            chimera: parameters.chimera,
            report_psms: parameters.report_psms + 1, // as upstream
            wide_window: parameters.wide_window,
            annotate_matches: parameters.annotate_matches,
            score_type: parameters.score_type,
        };

        let keep = (0..db.peptides.len())
            .map(|_| AtomicBool::new(false))
            .collect::<Vec<_>>();

        match &spectra {
            Some(spectra) => {
                peptide_filter_processed_spectra(parameters, &scorer, spectra, &keep, hooks)
            }
            None => {
                parameters
                    .mzml_paths
                    .chunks(parallel)
                    .enumerate()
                    .for_each(|(chunk_idx, chunk)| {
                        if hooks.cancelled() {
                            return;
                        }
                        let spectra_chunk =
                            read_processed_spectra(parameters, chunk, chunk_idx, parallel, None).1;
                        peptide_filter_processed_spectra(
                            parameters,
                            &scorer,
                            &spectra_chunk,
                            &keep,
                            hooks,
                        )
                    })
            }
        };
        hooks.check()?;

        let peptides = db
            .peptides
            .drain(..)
            .enumerate()
            .filter_map(|(ix, peptide)| {
                if keep[ix].load(Ordering::Relaxed) {
                    Some(peptide)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        log::info!(
            target: LOG,
            "found {} pre-filtered peptides for fasta chunk {}",
            peptides.len(),
            chunk_id,
        );
        all_peptides.extend(peptides);
    }

    Parameters::reorder_peptides(&mut all_peptides);
    Ok(all_peptides)
}

/// Copies `Runner::peptide_filter_processed_spectra`, with a cancel check.
fn peptide_filter_processed_spectra(
    parameters: &Search,
    scorer: &Scorer,
    spectra: &[ProcessedSpectrum],
    keep: &[AtomicBool],
    hooks: &Hooks,
) {
    let counter = AtomicUsize::new(0);
    let start = Instant::now();

    spectra
        .par_iter()
        .filter(|spec| spec.masses.len() >= parameters.min_peaks && spec.level == 2)
        .for_each(|spectrum| {
            let prev = counter.fetch_add(1, Ordering::Relaxed);
            if prev > 0 && prev % 10_000 == 0 {
                let duration = Instant::now().duration_since(start).as_millis() as usize;

                let rate = prev * 1000 / (duration + 1);
                log::trace!(target: LOG, "- searched {} spectra ({} spectra/s)", prev, rate);
            }
            if hooks.cancelled() {
                return;
            }
            scorer.quick_score(spectrum, parameters.database.prefilter_low_memory, keep)
        });

    let duration = Instant::now().duration_since(start).as_millis() as usize;
    let prev = counter.load(Ordering::Relaxed);
    let rate = prev * 1000 / (duration + 1);
    log::info!(
        target: LOG,
        "- prefilter search:  {:8} ms ({} spectra/s)",
        duration,
        rate
    );
}

// ─── Spectra (copies the private `Runner` helpers) ────────────────────────────

/// Copies the private `RawSpectrumAccumulator` in `runner.rs`.
#[derive(Default)]
struct RawSpectrumAccumulator {
    ms1: Vec<RawSpectrum>,
    msn: Vec<RawSpectrum>,
}

impl RawSpectrumAccumulator {
    fn fold_op(mut self, rhs: RawSpectrum) -> Self {
        if rhs.ms_level == 1 {
            self.ms1.push(rhs);
        } else {
            self.msn.push(rhs);
        }
        self
    }

    fn reduce(mut self, other: Self) -> Self {
        self.ms1.extend(other.ms1);
        self.msn.extend(other.msn);
        self
    }
}

impl FromParallelIterator<RawSpectrum> for RawSpectrumAccumulator {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = RawSpectrum>,
    {
        par_iter
            .into_par_iter()
            .fold(
                RawSpectrumAccumulator::default,
                RawSpectrumAccumulator::fold_op,
            )
            .reduce(
                RawSpectrumAccumulator::default,
                RawSpectrumAccumulator::reduce,
            )
    }
}

impl FromIterator<RawSpectrum> for RawSpectrumAccumulator {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = RawSpectrum>,
    {
        iter.into_iter().fold(
            RawSpectrumAccumulator::default(),
            RawSpectrumAccumulator::fold_op,
        )
    }
}

/// Counts files as they are read, for the Read spectra bar.
struct FileCounter<'a> {
    done: AtomicUsize,
    total: usize,
    hooks: &'a Hooks<'a>,
}

impl FileCounter<'_> {
    fn one_more(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        self.hooks
            .emit(Event::Counter(Step::ReadSpectra, done, self.total));
    }
}

/// Copies `Runner::requires_ms1`.
fn requires_ms1(parameters: &Search) -> bool {
    parameters.quant.lfq
}

/// Copies `Runner::read_processed_spectra`. Adds a per-file counter.
fn read_processed_spectra(
    parameters: &Search,
    chunk: &[Url],
    chunk_idx: usize,
    batch_size: usize,
    files: Option<&FileCounter>,
) -> (Vec<ProcessedSpectrum>, Vec<ProcessedSpectrum>) {
    log::info!(
        target: LOG,
        "processing files {} .. {} ",
        batch_size * chunk_idx,
        batch_size * chunk_idx + chunk.len()
    );
    let start = Instant::now();

    let sn = parameters
        .quant
        .tmt_settings
        .sn
        .then_some(parameters.quant.tmt_settings.level);

    let min_deisotope_mz = match &parameters.quant.tmt {
        Some(i) => match parameters.quant.tmt_settings.level {
            2 => i.reporter_masses().last().map(|x| x * (1.0 + 20E-6)),
            _ => None,
        },
        None => None,
    };

    let sp = SpectrumProcessor::new(
        parameters.max_peaks,
        parameters.deisotope,
        min_deisotope_mz.unwrap_or(0.0),
    );

    let file_serial_read = chunk
        .iter()
        .all(|path| FileFormat::from(path.as_ref()).within_file_parallel());
    log::trace!(target: LOG, "file serial read: {}", file_serial_read);
    let inner_closure = |(idx, path): (usize, &Url)| {
        let file_id = chunk_idx * batch_size + idx;
        let res = sage_cloudpath::util::read_spectra(
            path,
            file_id,
            sn,
            parameters.bruker_config,
            requires_ms1(parameters),
        );
        if let Some(files) = files {
            files.one_more();
        }

        match res {
            Ok(s) => {
                log::trace!(target: LOG, "- {}: read {} spectra", path, s.len());
                Ok(s)
            }
            Err(e) => {
                log::error!(target: LOG, "- {}: {}", path, e);
                Err(e)
            }
        }
    };

    let spectra: RawSpectrumAccumulator = if file_serial_read {
        chunk
            .iter()
            .enumerate()
            .flat_map(inner_closure)
            .flatten()
            .collect()
    } else {
        chunk
            .par_iter()
            .enumerate()
            .flat_map(inner_closure)
            .flatten()
            .collect()
    };

    let msn_spectra = spectra
        .msn
        .into_par_iter()
        .map(|s| sp.process(s))
        .collect::<Vec<_>>();

    let has_ims = spectra.ms1.iter().any(|x| x.mobility.is_some());
    let ms1_spectra = if spectra.ms1.is_empty() {
        log::trace!(target: LOG, "no MS1 spectra found");
        Vec::new()
    } else {
        if has_ims {
            log::trace!(target: LOG, "Processing MS1 spectra with IMS columns");
        } else {
            log::trace!(target: LOG, "Processing MS1 spectra without IMS");
        }
        spectra.ms1.into_par_iter().map(|s| sp.process(s)).collect()
    };

    let io_time = Instant::now() - start;
    log::info!(target: LOG, "- file IO: {:8} ms", io_time.as_millis());

    (ms1_spectra, msn_spectra)
}

/// Copies `Runner::search_processed_spectra`. The counter is the GUI's, and
/// the cancel check sits before the costly `score` call, as in patch 2.
fn search_processed_spectra(
    parameters: &Search,
    scorer: &Scorer,
    msn_spectra: &[ProcessedSpectrum],
    hooks: &Hooks,
) -> Vec<Feature> {
    let counter = AtomicUsize::new(0);
    let start = Instant::now();

    let features: Vec<_> = msn_spectra
        .par_iter()
        .filter(|spec| spec.masses.len() >= parameters.min_peaks && spec.level == 2)
        .map(|x| {
            let prev = counter.fetch_add(1, Ordering::Relaxed);
            hooks.searched.fetch_add(1, Ordering::Relaxed);
            if prev > 0 && prev % 10_000 == 0 {
                let duration = Instant::now().duration_since(start).as_millis() as usize;

                let rate = prev * 1000 / (duration + 1);
                log::trace!(target: LOG, "- searched {} spectra ({} spectra/s)", prev, rate);
            }
            x
        })
        .filter(|_| !hooks.cancelled())
        .flat_map(|spec| scorer.score(spec))
        .collect();

    let duration = Instant::now().duration_since(start).as_millis() as usize;
    let prev = counter.load(Ordering::Relaxed);
    let rate = prev * 1000 / (duration + 1);
    log::info!(target: LOG, "- search:  {:8} ms ({} spectra/s)", duration, rate);
    features
}

/// Copies `Runner::complete_features`.
fn complete_features(
    parameters: &Search,
    msn_spectra: Vec<ProcessedSpectrum>,
    ms1_spectra: Vec<ProcessedSpectrum>,
    features: Vec<Feature>,
) -> SageResults {
    let quant = parameters
        .quant
        .tmt
        .as_ref()
        .map(|isobaric| {
            let level = parameters.quant.tmt_settings.level;
            if level != 2 && level != 3 {
                log::warn!(target: LOG, "TMT quant level set at {}, is this correct?", level);
            }
            sage_core::tmt::quantify(&msn_spectra, isobaric, Tolerance::Ppm(-20.0, 20.0), level)
        })
        .unwrap_or_default();

    SageResults {
        features,
        quant,
        ms1: ms1_spectra,
    }
}

/// The MS2 spectra that `search_processed_spectra` will score: the Search
/// bar's denominator. Same filter as upstream.
fn searchable(parameters: &Search, msn_spectra: &[ProcessedSpectrum]) -> usize {
    msn_spectra
        .iter()
        .filter(|spec| spec.masses.len() >= parameters.min_peaks && spec.level == 2)
        .count()
}

/// Copies `Runner::batch_files` and `Runner::process_chunk`, with the read
/// and search steps reported per chunk.
fn batch_files(
    parameters: &Search,
    scorer: &Scorer,
    batch_size: usize,
    hooks: &Hooks,
) -> SageResults {
    let n_chunks = parameters.mzml_paths.len().div_ceil(batch_size.max(1));
    let files = FileCounter {
        done: AtomicUsize::new(0),
        total: parameters.mzml_paths.len(),
        hooks,
    };
    let mut to_search = 0usize;
    let mut ms2_read = 0usize;
    let read_start = Instant::now();
    let mut search_time = Duration::ZERO;
    hooks.emit(Event::Started(Step::ReadSpectra));
    hooks.emit(Event::Counter(Step::ReadSpectra, 0, files.total));

    let results = parameters
        .mzml_paths
        .chunks(batch_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            // Copies `process_chunk`. Cancelled between chunks: skip reading
            // and scoring this chunk.
            if hooks.cancelled() {
                return SageResults::default();
            }
            if n_chunks > 1 {
                let text = format!("file batch {} of {}", chunk_idx + 1, n_chunks);
                hooks.emit(Event::Detail(Step::ReadSpectra, text.clone()));
                hooks.emit(Event::Detail(Step::Search, text));
            }
            let spectra =
                read_processed_spectra(parameters, chunk, chunk_idx, batch_size, Some(&files));
            ms2_read += spectra.1.len();
            to_search += searchable(parameters, &spectra.1);
            if chunk_idx == 0 {
                hooks.emit(Event::Started(Step::Search));
            }
            hooks.emit(Event::Counter(
                Step::Search,
                hooks.searched.load(Ordering::Relaxed),
                to_search,
            ));
            let search_start = Instant::now();
            let features = search_processed_spectra(parameters, scorer, &spectra.1, hooks);
            search_time += search_start.elapsed();
            complete_features(parameters, spectra.1, spectra.0, features)
        })
        .collect::<SageResults>();

    let read_time = read_start.elapsed().saturating_sub(search_time);
    hooks.emit(Event::Finished(
        Step::ReadSpectra,
        format!("{} MS2 spectra · {}", count(ms2_read), secs(read_time)),
    ));
    hooks.emit(Event::Finished(
        Step::Search,
        format!(
            "{} spectra · {} PSMs · {}",
            count(hooks.searched.load(Ordering::Relaxed)),
            count(results.features.len()),
            secs(search_time)
        ),
    ));
    results
}

// ─── Search, FDR, quant and output (copies `Runner::run`) ─────────────────────

/// Copies `Runner::spectrum_fdr`.
fn spectrum_fdr(parameters: &Search, features: &mut [Feature]) -> usize {
    if sage_core::ml::linear_discriminant::score_psms(features, parameters.precursor_tol).is_none()
    {
        log::warn!(
            target: LOG,
            "linear model fitting failed, falling back to heuristic discriminant score"
        );
        features.par_iter_mut().for_each(|feat| {
            feat.discriminant_score = (-feat.poisson as f32).ln_1p() + feat.longest_y_pct / 3.0
        });
    }
    features.par_sort_unstable_by(|a, b| b.discriminant_score.total_cmp(&a.discriminant_score));
    sage_core::ml::qvalue::spectrum_q_value(features)
}

/// Copies `Runner::make_path`.
fn make_path(parameters: &Search, file_name: &str) -> Url {
    parameters
        .output_directory
        .join(file_name)
        .expect("valid path segment")
}

/// Searches, scores and writes the output. Copies `Runner::run`.
///
/// Stock `run` prints `results.json` to standard output and returns
/// telemetry. SageGUI has no console and sends no telemetry, so both are
/// left out. Every file and every `info` line is the same.
pub fn run_search(
    mut parameters: Search,
    database: IndexedDatabase,
    parallel: usize,
    parquet: bool,
    hooks: &Hooks,
) -> anyhow::Result<()> {
    let start = Instant::now();
    hooks.check()?;
    let scorer = Scorer {
        db: &database,
        precursor_tol: parameters.precursor_tol,
        fragment_tol: parameters.fragment_tol,
        min_matched_peaks: parameters.min_matched_peaks,
        min_isotope_err: parameters.isotope_errors.0,
        max_isotope_err: parameters.isotope_errors.1,
        min_precursor_charge: parameters.precursor_charge.0,
        max_precursor_charge: parameters.precursor_charge.1,
        override_precursor_charge: parameters.override_precursor_charge,
        max_fragment_charge: parameters.max_fragment_charge,
        chimera: parameters.chimera,
        report_psms: parameters.report_psms,
        wide_window: parameters.wide_window,
        annotate_matches: parameters.annotate_matches,
        score_type: parameters.score_type,
    };

    let mut outputs = batch_files(&parameters, &scorer, parallel, hooks);

    // Stop before FDR or any output: a stopped run writes nothing.
    hooks.check()?;

    let fdr_start = Instant::now();
    hooks.emit(Event::Started(Step::Fdr));
    let alignments = if parameters.predict_rt {
        hooks.emit(Event::Detail(Step::Fdr, "retention time model".into()));
        outputs
            .features
            .par_sort_unstable_by(|a, b| a.poisson.total_cmp(&b.poisson));
        sage_core::ml::qvalue::spectrum_q_value(&mut outputs.features);

        let alignments = sage_core::ml::retention_alignment::global_alignment(
            &mut outputs.features,
            parameters.mzml_paths.len(),
        );
        let _ = sage_core::ml::retention_model::predict(&database, &mut outputs.features);
        let _ = sage_core::ml::mobility_model::predict(&database, &mut outputs.features);
        Some(alignments)
    } else {
        None
    };

    hooks.emit(Event::Detail(
        Step::Fdr,
        "spectrum, peptide and protein FDR".into(),
    ));
    let q_spectrum = spectrum_fdr(&parameters, &mut outputs.features);
    let q_peptide = sage_core::fdr::picked_peptide(&database, &mut outputs.features);
    let q_protein = sage_core::fdr::picked_protein(&database, &mut outputs.features);
    hooks.emit(Event::Detail(Step::Fdr, "protein groups".into()));
    sage_core::protein_grouping::generate_protein_groups(
        &database,
        &mut outputs.features,
        parameters.protein_grouping,
        Some(parameters.protein_grouping_peptide_fdr),
    );
    let q_protein_group = sage_core::fdr::picked_protein_group(&database, &mut outputs.features);
    hooks.emit(Event::Finished(
        Step::Fdr,
        format!(
            "{} PSMs · {} peptides at 1% FDR · {}",
            count(q_spectrum),
            count(q_peptide),
            secs(fdr_start.elapsed())
        ),
    ));
    hooks.check()?;

    let filenames = parameters
        .mzml_paths
        .iter()
        .map(|url| {
            sage_cloudpath::filename(url)
                .unwrap_or_else(|| url.as_str())
                .to_string()
        })
        .collect::<Vec<_>>();

    // Upstream runs LFQ only when there are alignments, that is when
    // `predict_rt` is on. The Quant step says so instead of passing quietly.
    let quant_start = Instant::now();
    let areas = match (&alignments, parameters.quant.lfq) {
        (Some(alignments), true) => {
            hooks.emit(Event::Started(Step::Quant));
            log::trace!(target: LOG, "performing LFQ");
            let mut areas = sage_core::lfq::build_feature_map(
                parameters.quant.lfq_settings,
                parameters.precursor_charge,
                &outputs.features,
            )
            .quantify(&database, &outputs.ms1, alignments);

            let q_precursor = sage_core::fdr::picked_precursor(&mut areas);

            log::info!(
                target: LOG,
                "discovered {} target MS1 peaks at 5% FDR",
                q_precursor
            );
            hooks.emit(Event::Finished(
                Step::Quant,
                format!(
                    "LFQ · {} MS1 peaks at 5% FDR · {}",
                    count(q_precursor),
                    secs(quant_start.elapsed())
                ),
            ));
            Some(areas)
        }
        (None, true) => {
            hooks.emit(Event::Skipped(
                Step::Quant,
                "LFQ needs retention time prediction".into(),
            ));
            None
        }
        (_, false) if !outputs.quant.is_empty() => {
            hooks.emit(Event::Finished(
                Step::Quant,
                format!("TMT · {} spectra", count(outputs.quant.len())),
            ));
            None
        }
        (_, false) => {
            hooks.emit(Event::Skipped(Step::Quant, "off".into()));
            None
        }
    };

    log::info!(
        target: LOG,
        "discovered {} target peptide-spectrum matches at 1% FDR",
        q_spectrum
    );
    log::info!(target: LOG, "discovered {} target peptides at 1% FDR", q_peptide);
    log::info!(
        target: LOG,
        "discovered {} target proteins (supported by proteotypic peptides only) at 1% FDR",
        q_protein
    );
    log::info!(
        target: LOG,
        "discovered {} target protein groups (supported by proteotypic peptides only) at 1% FDR",
        q_protein_group
    );
    log::trace!(target: LOG, "writing outputs");

    // The last stop point. From here on, files are written.
    hooks.check()?;
    hooks.emit(Event::Started(Step::Write));
    let write_start = Instant::now();

    let mut output_paths = Vec::new();
    // The writers are Sage's own, on a `Runner` that holds the database.
    let runner = Runner::from_parts(parameters.clone(), database);

    if parquet {
        log::warn!(
            target: LOG,
            "parquet output format is currently unstable! There may be failures or schema changes!"
        );

        let bytes = sage_cloudpath::parquet::serialize_features(
            &outputs.features,
            &outputs.quant,
            &filenames,
            &runner.database,
        )?;

        let path = make_path(&parameters, "results.sage.parquet");
        sage_cloudpath::write_bytes_sync(&path, bytes)?;
        output_paths.push(path);

        if parameters.annotate_matches {
            let bytes = sage_cloudpath::parquet::serialize_matched_fragments(&outputs.features)?;
            let path = make_path(&parameters, "matched_fragments.sage.parquet");
            sage_cloudpath::write_bytes_sync(&path, bytes)?;
            output_paths.push(path);
        }

        if let Some(areas) = &areas {
            let bytes =
                sage_cloudpath::parquet::serialize_lfq(areas, &filenames, &runner.database)?;

            let path = make_path(&parameters, "lfq.parquet");
            sage_cloudpath::write_bytes_sync(&path, bytes)?;
            output_paths.push(path);
        }
    } else {
        output_paths.push(runner.write_features(&outputs.features, &filenames)?);

        if parameters.annotate_matches {
            output_paths.push(runner.write_fragments(&outputs.features)?);
        }

        if !outputs.quant.is_empty() {
            output_paths.push(runner.write_tmt(&outputs.quant, &filenames)?);
        }
        if let Some(areas) = areas.clone() {
            output_paths.push(runner.write_lfq(areas, &filenames)?);
        }
    }

    if parameters.write_pin {
        output_paths.push(runner.write_pin(&outputs.features, &filenames)?);
    }

    if parameters.write_report {
        hooks.emit(Event::Detail(Step::Write, "HTML report".into()));
        output_paths.push(runner.write_report(&outputs.features, areas, &filenames)?);
    }

    parameters.output_paths.extend(output_paths);
    let path = make_path(&parameters, "results.json");
    let bytes = serde_json::to_vec_pretty(&parameters)?;
    sage_cloudpath::write_bytes_sync(&path, bytes)?;
    parameters.output_paths.push(path);

    hooks.emit(Event::Finished(
        Step::Write,
        format!(
            "{} files · {}",
            parameters.output_paths.len(),
            secs(write_start.elapsed())
        ),
    ));

    let run_time = start.elapsed().as_secs();
    log::info!(target: LOG, "finished in {}s", run_time);
    log::info!(target: LOG, "cite: \"Sage: An Open-Source Tool for Fast Proteomics Searching and Quantification at Scale\" https://doi.org/10.1021/acs.jproteome.3c00486");
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The guard on the copy: stock Sage (`Runner::new` then `Runner::run`)
    //! and this pipeline run on the same input, into two folders, and every
    //! output file must match. Fails when an upstream change to `runner.rs`
    //! was not ported here.
    use super::*;
    use sage_cli::input::Input;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Upstream Sage's own test data (`tests/` in lazear/sage at 4a051b2):
    /// one spectrum of LQSRPAAPPAPGPGQLTLR and the protein it comes from.
    const MZML: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sage/LQSRPAAPPAPGPGQLTLR.mzML"
    );
    const FASTA: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sage/Q99536.fasta"
    );

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "sagegui-pipeline-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn no_events(_: Event) {}

    /// A fixed pseudo-random sequence, so every run makes the same data.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) as usize
        }
        fn unit(&mut self) -> f32 {
            (self.next() % 1_000_000) as f32 / 1_000_000.0
        }
    }

    /// Test data made in the test, so the repository holds no large fixture:
    /// the real protein plus 400 made-up ones, and an MGF of 300 spectra. Of
    /// those, 220 are b and y ladders of tryptic peptides from the FASTA
    /// (true matches) and 80 are noise. That gives targets, decoys, a working
    /// LDA and several ranks per spectrum, so a change to ranking, FDR or
    /// grouping changes the output. One real spectrum alone did not.
    fn test_data(root: &Path) -> (PathBuf, PathBuf) {
        use sage_core::mass::{monoisotopic, H2O, PROTON};
        let mut rng = Lcg(0x5eed_5a9e);
        let mut text = fs::read_to_string(FASTA).unwrap();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        const AA: &[u8] = b"ACDEFGHIKLMNPQRSTVWY";
        let mut proteins: Vec<String> = vec![text.lines().skip(1).collect()];
        for i in 0..400 {
            let len = 150 + rng.next() % 300;
            let mut seq: String = (0..len)
                .map(|_| AA[rng.next() % AA.len()] as char)
                .collect();
            // Every fourth protein repeats a stretch of the one before, so
            // some peptides are shared and protein grouping has work to do.
            if i % 4 == 3 {
                let prev = proteins.last().unwrap();
                seq.push_str(&prev[..120.min(prev.len())]);
            }
            text.push_str(&format!(">sp|T{i:04}|TEST{i}_MADEUP\n{seq}\n"));
            proteins.push(seq);
        }
        let fasta = root.join("test.fasta");
        fs::write(&fasta, text).unwrap();

        // Tryptic peptides of 7 to 20 residues: cut after K or R, not before P.
        let mut peptides = Vec::new();
        for p in &proteins {
            let b = p.as_bytes();
            let mut start = 0;
            for i in 0..b.len() {
                let cut = (b[i] == b'K' || b[i] == b'R') && b.get(i + 1) != Some(&b'P');
                if cut || i + 1 == b.len() {
                    if (7..=20).contains(&(i + 1 - start)) {
                        peptides.push(p[start..=i].to_string());
                    }
                    start = i + 1;
                }
            }
        }
        let residue = |c: u8| monoisotopic(c) + if c == b'C' { 57.0216 } else { 0.0 };

        let mut mgf = String::new();
        let noise = |rng: &mut Lcg, out: &mut String, n: usize| {
            for _ in 0..n {
                let mz = 150.0 + rng.unit() * 1350.0;
                out.push_str(&format!("{mz:.5} {:.1}\n", 50.0 + rng.unit() * 500.0));
            }
        };
        for scan in 0..300 {
            let rt = 60.0 + scan as f32 * 12.0;
            mgf.push_str(&format!(
                "BEGIN IONS\nTITLE=index={scan}\nRTINSECONDS={rt:.2}\n"
            ));
            if scan % 15 < 11 {
                let pep = peptides[rng.next() % peptides.len()].as_bytes();
                let mass: f32 = pep.iter().map(|c| residue(*c)).sum::<f32>() + H2O;
                mgf.push_str(&format!(
                    "PEPMASS={:.5}\nCHARGE=2+\n",
                    (mass + 2.0 * PROTON) / 2.0
                ));
                let mut b = 0.0;
                for c in &pep[..pep.len() - 1] {
                    b += residue(*c);
                    let y = mass - b;
                    if rng.unit() < 0.8 {
                        let i = 1000.0 + rng.unit() * 9000.0;
                        mgf.push_str(&format!("{:.5} {i:.1}\n", b + PROTON));
                    }
                    if rng.unit() < 0.9 {
                        let i = 1000.0 + rng.unit() * 9000.0;
                        mgf.push_str(&format!("{:.5} {i:.1}\n", y + PROTON));
                    }
                }
                noise(&mut rng, &mut mgf, 30);
            } else {
                let mz = 400.0 + rng.unit() * 800.0;
                mgf.push_str(&format!("PEPMASS={mz:.5}\nCHARGE=2+\n"));
                noise(&mut rng, &mut mgf, 60);
            }
            mgf.push_str("END IONS\n");
        }
        let spectra = root.join("test.mgf");
        fs::write(&spectra, mgf).unwrap();
        (fasta, spectra)
    }

    /// Runs stock Sage and the pipeline on `input`, into `root/stock` and
    /// `root/pipeline`, and returns the two folders.
    fn run_both(
        input: &dyn Fn() -> Input,
        root: &Path,
        parallel: usize,
        parquet: bool,
    ) -> (PathBuf, PathBuf) {
        let build = |name: &str| {
            let mut input = input();
            let out = root.join(name);
            input.output_directory = Some(out.to_string_lossy().to_string());
            (input.build().expect("config must build"), out)
        };

        let (search, stock) = build("stock");
        Runner::new(search, parallel)
            .unwrap()
            .run(parallel, parquet)
            .unwrap();

        let (search, ours) = build("pipeline");
        let cancel = AtomicBool::new(false);
        let hooks = Hooks {
            cancel: &cancel,
            searched: Arc::new(AtomicUsize::new(0)),
            emit: &no_events,
        };
        let (search, db) = build_database(search, parallel, &hooks).unwrap();
        run_search(search, db, parallel, parquet, &hooks).unwrap();
        (stock, ours)
    }

    /// The rows of a tab-separated file as a sorted list, without the columns
    /// that differ from run to run (`psm_id` and the PIN `SpecId`, which is
    /// the same number). Row order is not compared: LFQ order changes between
    /// runs of stock Sage too.
    fn rows(path: &Path) -> Vec<String> {
        let text = fs::read_to_string(path).unwrap();
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
        let skip: Vec<usize> = header
            .iter()
            .enumerate()
            .filter(|(_, h)| **h == "psm_id" || **h == "SpecId")
            .map(|(i, _)| i)
            .collect();
        let keep = |line: &str| {
            line.split('\t')
                .enumerate()
                .filter(|(i, _)| !skip.contains(i))
                .map(|(_, c)| c)
                .collect::<Vec<_>>()
                .join("\t")
        };
        let mut out: Vec<String> = lines.map(keep).collect();
        out.sort();
        out.insert(0, keep(&header.join("\t")));
        out
    }

    /// `results.json` with the output folder written as `<out>`.
    fn results_json(dir: &Path) -> serde_json::Value {
        let text = fs::read_to_string(dir.join("results.json")).unwrap();
        let canon = fs::canonicalize(dir).unwrap();
        let mut text = text.replace(&canon.to_string_lossy().to_string(), "<out>");
        text = text.replace(&dir.to_string_lossy().to_string(), "<out>");
        serde_json::from_str(&text).unwrap()
    }

    fn files(dir: &Path) -> BTreeMap<String, PathBuf> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (e.file_name().to_string_lossy().to_string(), e.path())
            })
            .collect()
    }

    fn assert_same_outputs(stock: &Path, ours: &Path) {
        let a = files(stock);
        let b = files(ours);
        assert_eq!(
            a.keys().collect::<Vec<_>>(),
            b.keys().collect::<Vec<_>>(),
            "the same files must be written"
        );
        assert_eq!(results_json(stock), results_json(ours), "results.json");
        for (name, path) in &a {
            if name.ends_with(".tsv") || name.ends_with(".pin") {
                let (x, y) = (rows(path), rows(&b[name]));
                assert!(x.len() > 1, "{name} has no rows; the fixture tests nothing");
                assert_eq!(x.len(), y.len(), "{name}: row count");
                for (i, (r, s)) in x.iter().zip(&y).enumerate() {
                    assert_eq!(r, s, "{name}: sorted row {i} differs");
                }
            } else if name.ends_with(".html") || name.ends_with(".parquet") {
                // The report and Parquet files embed nothing that we can
                // compare row by row here. They must be written and not empty.
                assert!(fs::metadata(path).unwrap().len() > 0, "{name} is empty");
                assert!(fs::metadata(&b[name]).unwrap().len() > 0, "{name} is empty");
            }
        }
    }

    fn input(data: &(PathBuf, PathBuf), extra: serde_json::Value) -> Input {
        let (fasta, mgf) = data;
        let mut v = serde_json::json!({
            "database": {
                "bucket_size": 16384,
                "enzyme": { "missed_cleavages": 1, "cleave_at": "KR", "restrict": "P" },
                "static_mods": { "C": 57.0216 },
                "variable_mods": { "M": [15.9949] },
                "decoy_tag": "rev_",
                "generate_decoys": true,
                "fasta": fasta.to_string_lossy()
            },
            "deisotope": true,
            "max_fragment_charge": 1,
            "report_psms": 3,
            "min_matched_peaks": 2,
            "precursor_tol": { "da": [-25, 25] },
            "fragment_tol": { "ppm": [-10, 10] },
            "isotope_errors": [-1, 3],
            "mzml_paths": [mgf.to_string_lossy(), MZML],
            "write_pin": true,
            "annotate_matches": true,
            "write_report": true
        });
        merge(&mut v, extra);
        serde_json::from_value(v).expect("test Input JSON")
    }

    fn merge(a: &mut serde_json::Value, b: serde_json::Value) {
        match (a, b) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                for (k, v) in b {
                    merge(a.entry(k).or_insert(serde_json::Value::Null), v);
                }
            }
            (a, b) => *a = b,
        }
    }

    #[test]
    fn pipeline_matches_stock_runner() {
        let root = temp_dir("basic");
        let data = test_data(&root);
        let (stock, ours) = run_both(&|| input(&data, serde_json::json!({})), &root, 2, false);
        assert_same_outputs(&stock, &ours);
        let _ = fs::remove_dir_all(&root);
    }

    /// More files than `parallel`: Sage reads and searches in batches.
    #[test]
    fn pipeline_matches_stock_runner_in_file_batches() {
        let root = temp_dir("batches");
        let data = test_data(&root);
        let mgf = data.1.to_string_lossy().to_string();
        let i = || input(&data, serde_json::json!({ "mzml_paths": [mgf, MZML, mgf] }));
        let (stock, ours) = run_both(&i, &root, 1, false);
        assert_same_outputs(&stock, &ours);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pipeline_matches_stock_runner_with_parquet() {
        let root = temp_dir("parquet");
        let data = test_data(&root);
        let (stock, ours) = run_both(&|| input(&data, serde_json::json!({})), &root, 2, true);
        assert_same_outputs(&stock, &ours);
        let _ = fs::remove_dir_all(&root);
    }

    /// The prefilter path, with the FASTA in nine chunks.
    #[test]
    fn pipeline_matches_stock_runner_with_prefilter() {
        let root = temp_dir("prefilter");
        let data = test_data(&root);
        let i = || {
            input(
                &data,
                serde_json::json!({
                    "database": {
                        "prefilter": true,
                        "prefilter_chunk_size": 50,
                        "prefilter_low_memory": false
                    }
                }),
            )
        };
        let (stock, ours) = run_both(&i, &root, 2, false);
        assert_same_outputs(&stock, &ours);
        let _ = fs::remove_dir_all(&root);
    }

    /// The Stop button: a run cancelled before the search writes nothing.
    #[test]
    fn a_cancelled_run_writes_nothing() {
        let root = temp_dir("cancel");
        let mut i = input(&test_data(&root), serde_json::json!({}));
        i.output_directory = Some(root.join("out").to_string_lossy().to_string());
        let search = i.build().unwrap();
        let cancel = AtomicBool::new(false);
        let hooks = Hooks {
            cancel: &cancel,
            searched: Arc::new(AtomicUsize::new(0)),
            emit: &no_events,
        };
        let (search, db) = build_database(search, 1, &hooks).unwrap();
        cancel.store(true, Ordering::Relaxed);
        let err = run_search(search, db, 1, false, &hooks).unwrap_err();
        assert_eq!(err.to_string(), CANCELLED);
        assert_eq!(files(&root.join("out")).len(), 0, "no file may be written");
        let _ = fs::remove_dir_all(&root);
    }

    /// The steps are reported in order, and each started step finishes.
    #[test]
    fn every_step_is_reported() {
        let root = temp_dir("events");
        let mut i = input(&test_data(&root), serde_json::json!({}));
        i.output_directory = Some(root.join("out").to_string_lossy().to_string());
        let search = i.build().unwrap();
        let events = std::sync::Mutex::new(Vec::new());
        let record = |e: Event| events.lock().unwrap().push(e);
        let cancel = AtomicBool::new(false);
        let hooks = Hooks {
            cancel: &cancel,
            searched: Arc::new(AtomicUsize::new(0)),
            emit: &record,
        };
        let (search, db) = build_database(search, 1, &hooks).unwrap();
        run_search(search, db, 1, false, &hooks).unwrap();
        let events = events.lock().unwrap().clone();
        let ended: Vec<Step> = events
            .iter()
            .filter_map(|e| match e {
                Event::Finished(s, _) | Event::Skipped(s, _) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(ended, Step::ALL.to_vec());
        // 300 MGF spectra and the one mzML spectrum.
        assert_eq!(hooks.searched.load(Ordering::Relaxed), 301);
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Counter(Step::Search, _, 301))));
        let _ = fs::remove_dir_all(&root);
    }

    /// Stock against pipeline on real data. Needs a Sage `results.json` (or
    /// `config.json`) whose FASTA and spectra are on this machine, so it is
    /// ignored by default. Run it with:
    ///
    /// `SAGEGUI_PIPELINE_E2E=/path/to/results.json cargo test --release \
    ///  pipeline_matches_stock_runner_on_real_data -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn pipeline_matches_stock_runner_on_real_data() {
        let Ok(config) = std::env::var("SAGEGUI_PIPELINE_E2E") else {
            eprintln!("SAGEGUI_PIPELINE_E2E is not set; nothing to do");
            return;
        };
        let i = || {
            let mut i = Input::load(&config).expect("config must load");
            if std::env::var("SAGEGUI_PIPELINE_E2E_REPORT").is_err() {
                // The report adds minutes and nothing comparable.
                i.write_report = Some(false);
            }
            i
        };
        let root = temp_dir("real");
        let parallel = (num_cpus::get() / 2).max(1);
        let t = Instant::now();
        let (stock, ours) = run_both(&i, &root, parallel, false);
        eprintln!("both runs took {:.1?}", t.elapsed());
        assert_same_outputs(&stock, &ours);
        eprintln!(
            "outputs identical except psm_id; kept in {}",
            root.display()
        );
    }
}
