// Written by Benjamin A. Neely (NIST) on 2026-09-21.
//! Run the mzIdentML and pepXML converters off the UI thread.
//!
//! The converters read a whole `results.sage.tsv`, so a run takes seconds and
//! must not block a frame. One worker thread does every format the user chose,
//! one after the other. It reports progress and its result on a channel.
//!
//! This state is separate from the search on purpose. Nothing in this module
//! can reach `SageLauncher::status_message`, so a failed conversion can never
//! change the result of a search. It has its own status line.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::export::{
    convert_to_mzid_with, convert_to_pepxml_with, is_cancelled, Control, ExportOptions, CANCELLED,
};
use crate::ui::Config;

/// What to convert, and from where.
#[derive(Debug, Clone)]
pub struct ConvertRequest {
    /// The Output Location text, as typed. It is checked when the run starts.
    pub dir: String,
    pub opts: ExportOptions,
    pub mzid: bool,
    pub pepxml: bool,
}

impl ConvertRequest {
    /// A request from the current settings, for the two buttons.
    pub fn from_config(config: &Config, mzid: bool, pepxml: bool) -> Self {
        ConvertRequest {
            dir: config.output_directory.clone(),
            opts: config.export_options.clone(),
            mzid,
            pepxml,
        }
    }

    /// The request to run after a search, or `None` when no box is ticked.
    /// It is taken when the search starts, so the folder is the one Sage used
    /// even if the text box changes during the run.
    pub fn after_search(config: &Config) -> Option<Self> {
        let request = Self::from_config(
            config,
            config.export_mzid_after_run,
            config.export_pepxml_after_run,
        );
        (request.mzid || request.pepxml).then_some(request)
    }
}

/// Start a conversion after a search? Only when the search finished with a
/// result and the request names a format.
///
/// The test is on `result`, not on the Stop button. A search that was stopped
/// in time returns `Err`. A search that finished before Stop took effect
/// returns `Ok` and wrote its files, so it is converted like any other.
pub fn should_auto_convert(
    result: &Result<String, String>,
    pending: &Option<ConvertRequest>,
) -> bool {
    result.is_ok() && pending.as_ref().is_some_and(|r| r.mzid || r.pepxml)
}

/// The folder to read from and write to.
///
/// A relative path is kept as it is. The converter runs in the process that
/// ran Sage, so it resolves against the same working folder that Sage used.
/// The check is on the text only. It does no file access, so it is safe to
/// call every frame.
pub fn conversion_dir(output_directory: &str) -> Result<PathBuf, String> {
    if output_directory.is_empty() {
        return Err("Output Location is empty. Choose a folder on Run / Info.".to_string());
    }
    if output_directory.contains("://") {
        return Err(
            "Output Location is a URL. The converter reads a local folder. Choose one on Run / Info."
                .to_string(),
        );
    }
    Ok(PathBuf::from(output_directory))
}

/// The shape of `convert_to_mzid_with` and `convert_to_pepxml_with`.
type ConvertFn = fn(&Path, &ExportOptions, &Control) -> Result<PathBuf, String>;

/// What a worker run produced. `error` can be set together with `written`,
/// when the first format worked and the second did not.
#[derive(Debug, Default)]
pub struct ConvertOutcome {
    pub written: Vec<PathBuf>,
    pub error: Option<String>,
}

/// Convert every format in `req`, mzIdentML first. Stops at the first error.
/// `on_progress` gets a fraction from 0.0 to 1.0 across all formats.
pub fn run_conversion(
    req: &ConvertRequest,
    cancel: &AtomicBool,
    on_progress: &dyn Fn(f32),
) -> ConvertOutcome {
    let mut out = ConvertOutcome::default();
    let dir: PathBuf = match conversion_dir(&req.dir) {
        Ok(dir) => dir,
        Err(e) => {
            out.error = Some(e);
            return out;
        }
    };

    let mut kinds: Vec<ConvertFn> = Vec::new();
    if req.mzid {
        kinds.push(convert_to_mzid_with);
    }
    if req.pepxml {
        kinds.push(convert_to_pepxml_with);
    }

    let total = kinds.len() as f32;
    for (i, convert) in kinds.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            out.error = Some(CANCELLED.to_string());
            break;
        }
        let done = i as f32;
        let scaled = |fraction: f32| on_progress((done + fraction) / total);
        let ctl = Control {
            progress: Some(&scaled),
            cancel: Some(cancel),
        };
        match convert(&dir, &req.opts, &ctl) {
            Ok(path) => out.written.push(path),
            Err(e) => {
                out.error = Some(e);
                break;
            }
        }
    }
    out
}

/// How the last conversion ended. Shown on Run / Info, apart from the search.
#[derive(Debug)]
pub enum ConvertStatus {
    Done(Vec<PathBuf>),
    Stopped(Vec<PathBuf>),
    Failed {
        error: String,
        written: Vec<PathBuf>,
    },
}

impl ConvertStatus {
    pub fn from_outcome(outcome: ConvertOutcome) -> Self {
        match outcome.error {
            None => ConvertStatus::Done(outcome.written),
            Some(e) if is_cancelled(&e) => ConvertStatus::Stopped(outcome.written),
            Some(error) => ConvertStatus::Failed {
                error,
                written: outcome.written,
            },
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ConvertStatus::Failed { .. })
    }

    /// The lines to show, one idea each.
    pub fn lines(&self) -> Vec<String> {
        let wrote = |paths: &[PathBuf]| {
            paths
                .iter()
                .map(|p| format!("Wrote {}", p.display()))
                .collect::<Vec<_>>()
        };
        match self {
            ConvertStatus::Done(written) => wrote(written),
            ConvertStatus::Stopped(written) => {
                let mut lines = vec!["Conversion stopped.".to_string()];
                lines.extend(wrote(written));
                lines
            }
            ConvertStatus::Failed { error, written } => {
                let mut lines = vec![format!("Conversion error: {error}")];
                lines.extend(wrote(written));
                lines
            }
        }
    }
}

enum ConvertMessage {
    Progress(f32),
    Finished(ConvertOutcome),
}

struct Job {
    receiver: Receiver<ConvertMessage>,
    handle: JoinHandle<()>,
    cancel: Arc<AtomicBool>,
    cancel_requested: bool,
}

/// The conversion the window owns: at most one running job, and the result of
/// the last one.
#[derive(Default)]
pub struct Converter {
    job: Option<Job>,
    /// The result of the last finished conversion. Cleared when a new one starts.
    pub status: Option<ConvertStatus>,
    /// 0.0 to 1.0 while a job runs.
    pub progress: f32,
}

impl Converter {
    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    pub fn cancel_requested(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.cancel_requested)
    }

    /// Start a job on its own thread. Does nothing, and returns `false`, when
    /// one is already running.
    pub fn start(&mut self, req: ConvertRequest) -> bool {
        if self.job.is_some() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let thread_cancel = cancel.clone();
        let handle = thread::spawn(move || {
            // The converter can call back many times a second. Send only when
            // the whole percent changes.
            let last = Cell::new(-1);
            let progress_sender = sender.clone();
            let on_progress = |fraction: f32| {
                let percent = (fraction * 100.0) as i32;
                if percent != last.get() {
                    last.set(percent);
                    let _ = progress_sender.send(ConvertMessage::Progress(fraction));
                }
            };
            let outcome = run_conversion(&req, &thread_cancel, &on_progress);
            let _ = sender.send(ConvertMessage::Finished(outcome));
        });
        self.status = None;
        self.progress = 0.0;
        self.job = Some(Job {
            receiver,
            handle,
            cancel,
            cancel_requested: false,
        });
        true
    }

    pub fn request_cancel(&mut self) {
        if let Some(job) = &mut self.job {
            job.cancel.store(true, Ordering::Relaxed);
            job.cancel_requested = true;
        }
    }

    /// Read every message the worker sent. Call once per frame. It reads a
    /// channel and does no file access. It never touches the search state.
    pub fn poll(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        let outcome = loop {
            match job.receiver.try_recv() {
                Ok(ConvertMessage::Progress(fraction)) => self.progress = fraction,
                Ok(ConvertMessage::Finished(o)) => break o,
                Err(TryRecvError::Empty) => return,
                // The thread ended without a result. That means it panicked.
                Err(TryRecvError::Disconnected) => {
                    break ConvertOutcome {
                        written: Vec::new(),
                        error: Some("The conversion stopped unexpectedly.".to_string()),
                    };
                }
            }
        };
        if let Some(job) = self.job.take() {
            let _ = job.handle.join();
        }
        self.progress = 1.0;
        self.status = Some(ConvertStatus::from_outcome(outcome));
    }

    /// Ask a running job to stop and wait for it, so that closing the window
    /// does not leave a `.tmp` file.
    pub fn stop_and_join(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
            let _ = job.handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export")
    }

    /// A scratch folder that holds the fixture. The converters write next to
    /// their input.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sagegui-convert-job-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        for file in ["results.json", "results.sage.tsv"] {
            std::fs::copy(fixture_dir().join(file), dir.join(file)).expect("copy fixture");
        }
        dir
    }

    fn request(dir: &Path, mzid: bool, pepxml: bool) -> ConvertRequest {
        ConvertRequest {
            dir: dir.to_string_lossy().to_string(),
            opts: ExportOptions::default(),
            mzid,
            pepxml,
        }
    }

    /// Wait for a job to finish, for at most a while.
    fn wait_for(converter: &mut Converter) {
        for _ in 0..600 {
            converter.poll();
            if !converter.is_running() {
                return;
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("the conversion did not finish");
    }

    #[test]
    fn the_worker_writes_both_files_on_the_fixture() {
        let dir = scratch("both");
        let seen = Cell::new(0.0f32);
        let out = run_conversion(&request(&dir, true, true), &AtomicBool::new(false), &|f| {
            assert!(f >= seen.get() - 1e-6, "progress went back");
            seen.set(f);
        });
        assert_eq!(out.error, None);
        assert_eq!(out.written.len(), 2);
        assert!(dir.join("results.sage.mzid").is_file());
        assert!(dir.join("results.sage.pep.xml").is_file());
        assert!(seen.get() > 0.99, "progress must reach the end");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_worker_writes_only_the_format_that_was_asked_for() {
        let dir = scratch("one");
        let out = run_conversion(
            &request(&dir, false, true),
            &AtomicBool::new(false),
            &|_| {},
        );
        assert_eq!(out.error, None);
        assert!(!dir.join("results.sage.mzid").exists());
        assert!(dir.join("results.sage.pep.xml").is_file());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_cancelled_worker_returns_the_cancel_message_and_writes_nothing() {
        let dir = scratch("cancel");
        let cancel = AtomicBool::new(true);
        let out = run_conversion(&request(&dir, true, true), &cancel, &|_| {});
        let error = out.error.expect("a cancelled run is an error");
        assert!(is_cancelled(&error), "got: {error}");
        assert!(out.written.is_empty());
        assert!(!dir.join("results.sage.mzid").exists());
        assert!(!dir.join("results.sage.pep.xml").exists());
        assert!(matches!(
            ConvertStatus::from_outcome(ConvertOutcome {
                written: Vec::new(),
                error: Some(error),
            }),
            ConvertStatus::Stopped(_)
        ));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_cancel_during_the_first_format_leaves_no_file() {
        // The flag is set from the progress callback, so the converter sees it
        // while it is running and not only before it starts.
        let dir = scratch("cancel-mid");
        let cancel = AtomicBool::new(false);
        let out = run_conversion(&request(&dir, true, true), &cancel, &|f| {
            if f > 0.0 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        let error = out.error.expect("a cancelled run is an error");
        assert!(is_cancelled(&error), "got: {error}");
        assert!(!dir.join("results.sage.mzid").exists());
        assert!(!dir.join("results.sage.mzid.tmp").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_folder_or_file_gives_a_message_and_not_a_panic() {
        let empty = std::env::temp_dir().join("sagegui-convert-job-empty-folder");
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        let out = run_conversion(
            &request(&empty, true, false),
            &AtomicBool::new(false),
            &|_| {},
        );
        let error = out.error.expect("no results files, so an error");
        assert!(error.contains("results.json"), "got: {error}");
        assert!(matches!(
            ConvertStatus::from_outcome(ConvertOutcome {
                written: Vec::new(),
                error: Some(error),
            }),
            ConvertStatus::Failed { .. }
        ));
        let _ = std::fs::remove_dir_all(empty);
    }

    #[test]
    fn conversion_dir_refuses_an_empty_path_and_a_url() {
        assert!(conversion_dir("").unwrap_err().contains("empty"));
        assert!(conversion_dir("s3://bucket/out")
            .unwrap_err()
            .contains("URL"));
        assert_eq!(conversion_dir("out").unwrap(), PathBuf::from("out"));
        let out = run_conversion(
            &ConvertRequest {
                dir: String::new(),
                ..request(Path::new("x"), true, true)
            },
            &AtomicBool::new(false),
            &|_| {},
        );
        assert!(out.error.unwrap().contains("empty"));
    }

    #[test]
    fn auto_convert_needs_a_finished_search_and_a_ticked_box() {
        let ok: Result<String, String> = Ok("Analysis completed successfully".to_string());
        let failed: Result<String, String> = Err("out of memory".to_string());
        let cancelled: Result<String, String> = Err("cancelled".to_string());
        let dir = Path::new("out");
        let both = Some(request(dir, true, true));
        let mzid = Some(request(dir, true, false));
        let pepxml = Some(request(dir, false, true));
        let neither = Some(request(dir, false, false));

        assert!(should_auto_convert(&ok, &both));
        assert!(should_auto_convert(&ok, &mzid));
        assert!(should_auto_convert(&ok, &pepxml));
        assert!(!should_auto_convert(&ok, &neither), "no box ticked");
        assert!(!should_auto_convert(&ok, &None), "nothing pending");
        assert!(!should_auto_convert(&failed, &both), "search failed");
        assert!(!should_auto_convert(&cancelled, &both), "search cancelled");
    }

    #[test]
    fn after_search_follows_the_two_checkboxes() {
        let mut config = Config::default();
        assert!(ConvertRequest::after_search(&config).is_none());

        config.export_pepxml_after_run = true;
        config.output_directory = "the/folder".to_string();
        let req = ConvertRequest::after_search(&config).expect("one box is ticked");
        assert!(!req.mzid && req.pepxml);
        assert_eq!(req.dir, "the/folder");

        config.export_mzid_after_run = true;
        let req = ConvertRequest::after_search(&config).unwrap();
        assert!(req.mzid && req.pepxml);
    }

    #[test]
    fn the_converter_runs_a_job_and_keeps_one_at_a_time() {
        let dir = scratch("converter");
        let mut converter = Converter::default();
        assert!(converter.start(request(&dir, true, false)));
        assert!(converter.is_running());
        assert!(
            !converter.start(request(&dir, false, true)),
            "a second job must not start while one runs"
        );
        wait_for(&mut converter);
        match converter.status.as_ref().expect("a status after the job") {
            ConvertStatus::Done(paths) => assert_eq!(paths.len(), 1),
            other => panic!("expected Done, got {other:?}"),
        }
        assert!(dir.join("results.sage.mzid").is_file());
        assert!(!dir.join("results.sage.pep.xml").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_converter_reports_a_failure_as_a_status() {
        let empty = std::env::temp_dir().join("sagegui-convert-job-converter-fail");
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        let mut converter = Converter::default();
        assert!(converter.start(request(&empty, true, true)));
        wait_for(&mut converter);
        let status = converter.status.as_ref().expect("a status after the job");
        assert!(status.is_error());
        assert!(status.lines()[0].starts_with("Conversion error: "));
        // A new job clears the old status.
        let dir = scratch("converter-after-fail");
        assert!(converter.start(request(&dir, true, false)));
        assert!(converter.status.is_none());
        wait_for(&mut converter);
        assert!(!converter.status.as_ref().unwrap().is_error());
        let _ = std::fs::remove_dir_all(empty);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_dead_worker_is_reported_and_does_not_hang_the_state() {
        let (sender, receiver) = mpsc::channel::<ConvertMessage>();
        drop(sender);
        let mut converter = Converter {
            job: Some(Job {
                receiver,
                handle: thread::spawn(|| {}),
                cancel: Arc::new(AtomicBool::new(false)),
                cancel_requested: false,
            }),
            ..Converter::default()
        };
        converter.poll();
        assert!(!converter.is_running());
        assert!(converter.status.as_ref().unwrap().is_error());
    }
}
