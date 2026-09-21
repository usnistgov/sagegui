// Written by Benjamin A. Neely (NIST) on 2026-09-21.
//! Convert Sage output to mzIdentML 1.1.1 and pepXML 1.23.
//!
//! Input is one Sage output folder: `results.sage.tsv` and `results.json`.
//! Output is written next to them: `results.sage.mzid` and
//! `results.sage.pep.xml`. The decision to build these converters here, and
//! what they do and do not validate against, is in NOTES.md under
//! "Downstream tools that read Sage output".
//!
//! The functions are pure. They take a folder and options and return the path
//! of the file they wrote, or a message for the user. They never panic on bad
//! input. They know nothing about the GUI. To run one off the UI thread, pass
//! a [`Control`] to the `_with` variant:
//!
//! ```ignore
//! let cancel = Arc::new(AtomicBool::new(false));
//! let progress = |f: f32| { /* 0.0 to 1.0 */ };
//! let ctl = Control { progress: Some(&progress), cancel: Some(&cancel) };
//! let result = convert_to_mzid_with(&dir, &opts, &ctl);
//! // A cancelled run returns Err(CANCELLED) and leaves no partial file.
//! ```
//!
//! The file is written to a temporary name first and renamed on success. A
//! failed or cancelled run does not replace a good file from an earlier run.

// Nothing calls this yet. The UI wiring comes in a later change.
#![allow(dead_code)]

/// `write!` that turns an I/O error into the message the caller returns.
macro_rules! wr {
    ($out:expr, $($arg:tt)*) => {
        $out.write_fmt(format_args!($($arg)*))
            .map_err(|e| format!("Cannot write the output file: {e}"))?
    };
}

mod model;
mod mzid;
mod params;
mod pepxml;
mod xml;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

/// The message of the `Err` that a cancelled run returns. Compare against it
/// to tell a cancel from a failure.
pub const CANCELLED: &str = "Cancelled.";

/// Which q-value column filters the PSMs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QSource {
    /// PSM level. The default, and what PDV and MSstatsConvert use.
    #[default]
    SpectrumQ,
    PeptideQ,
    /// Shared peptides hold this at 1.0, so it removes them. See NOTES.
    ProteinQ,
}

/// What goes into the output.
///
/// A PSM above `max_q`, or a decoy PSM when `include_decoys` is off, is left
/// out of the file. It is not written with `passThreshold="false"`. Every PSM
/// in the file passed the filter, so every mzIdentML item has
/// `passThreshold="true"`. The rank is kept as Sage wrote it. There is no
/// rank-1-only switch: Sage writes one rank unless `report_psms` is above 1,
/// and a reader can filter on the rank.
///
/// New fields need `#[serde(default)]` in whatever struct stores this, so an
/// old settings file still loads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportOptions {
    pub q_source: QSource,
    /// Keep a PSM when its q-value is at or below this. Default 0.01.
    pub max_q: f64,
    /// Keep decoy PSMs (`label` -1). Default off.
    pub include_decoys: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            q_source: QSource::SpectrumQ,
            max_q: 0.01,
            include_decoys: false,
        }
    }
}

/// Progress and cancel hooks. Both are optional.
#[derive(Default)]
pub struct Control<'a> {
    /// Called with a fraction from 0.0 to 1.0. Called from the converting
    /// thread. It does not need to be fast, but it should not block.
    pub progress: Option<&'a dyn Fn(f32)>,
    /// Set to `true` from another thread to stop the run.
    pub cancel: Option<&'a AtomicBool>,
}

impl Control<'_> {
    pub(crate) fn report(&self, fraction: f32) {
        if let Some(f) = self.progress {
            f(fraction.clamp(0.0, 1.0));
        }
    }

    pub(crate) fn cancelled(&self) -> bool {
        self.cancel.is_some_and(|c| c.load(Ordering::Relaxed))
    }
}

/// Convert `results.sage.tsv` and `results.json` in `out_dir` to
/// `results.sage.mzid`. Returns the path of the new file.
pub fn convert_to_mzid(out_dir: &Path, opts: &ExportOptions) -> Result<PathBuf, String> {
    convert_to_mzid_with(out_dir, opts, &Control::default())
}

/// [`convert_to_mzid`] with progress and cancel.
pub fn convert_to_mzid_with(
    out_dir: &Path,
    opts: &ExportOptions,
    ctl: &Control,
) -> Result<PathBuf, String> {
    convert(out_dir, opts, ctl, "results.sage.mzid", mzid::write)
}

/// Convert `results.sage.tsv` and `results.json` in `out_dir` to
/// `results.sage.pep.xml`. Returns the path of the new file.
pub fn convert_to_pepxml(out_dir: &Path, opts: &ExportOptions) -> Result<PathBuf, String> {
    convert_to_pepxml_with(out_dir, opts, &Control::default())
}

/// [`convert_to_pepxml`] with progress and cancel.
pub fn convert_to_pepxml_with(
    out_dir: &Path,
    opts: &ExportOptions,
    ctl: &Control,
) -> Result<PathBuf, String> {
    convert(out_dir, opts, ctl, "results.sage.pep.xml", pepxml::write)
}

/// A writer: table and parameters in, XML out.
type WriteFn = fn(
    &mut dyn Write,
    &model::Table,
    &params::Params,
    &ExportOptions,
    &Control,
) -> Result<(), String>;

/// The steps both formats share: read the parameters, read the table, write
/// to a temporary file, rename.
fn convert(
    out_dir: &Path,
    opts: &ExportOptions,
    ctl: &Control,
    file_name: &str,
    write: WriteFn,
) -> Result<PathBuf, String> {
    if !opts.max_q.is_finite() || opts.max_q < 0.0 {
        return Err(format!(
            "The q-value limit must be a number from 0 up. It is {}.",
            opts.max_q
        ));
    }
    let params = params::Params::load(out_dir)?;
    let tsv = out_dir.join("results.sage.tsv");
    ctl.report(0.0);
    let table = model::load_table(&tsv, opts, ctl, 0.0, 0.5)?;
    if table.psms.is_empty() {
        return Err(format!(
            "No PSMs are left after filtering. Read {} rows. Removed {} decoy rows and {} rows \
             above the q-value limit of {}. Raise the limit or include decoys.",
            table.rows_read, table.dropped_decoy, table.dropped_q, opts.max_q
        ));
    }

    let final_path = out_dir.join(file_name);
    let tmp_path = out_dir.join(format!("{file_name}.tmp"));
    let result = (|| {
        let file = File::create(&tmp_path)
            .map_err(|e| format!("Cannot write {}: {e}", tmp_path.display()))?;
        let mut w = BufWriter::with_capacity(1 << 20, file);
        write(&mut w, &table, &params, opts, ctl)?;
        w.flush()
            .map_err(|e| format!("Cannot write {}: {e}", tmp_path.display()))?;
        drop(w);
        if ctl.cancelled() {
            return Err(CANCELLED.to_string());
        }
        std::fs::rename(&tmp_path, &final_path).map_err(|e| {
            format!(
                "Cannot replace {}: {e}. Is the file open in another program?",
                final_path.display()
            )
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result?;
    ctl.report(1.0);
    Ok(final_path)
}

/// `true` when `err` came from a cancel and not from a failure.
pub fn is_cancelled(err: &str) -> bool {
    err == CANCELLED
}

#[cfg(test)]
mod tests;
