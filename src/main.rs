// Derived from jspaezp/sagegui (Apache 2.0, J. Sebastian Paez).
// Modified by Benjamin A. Neely (NIST) on 2026-07-10: upgraded to Sage v0.15.0-beta.2,
// fixed TMT plex and fragment-tolerance bugs, added structured logging and CI/CD.
// Modified by Benjamin A. Neely (NIST) on 2026-08-13: restructured into sidebar-nav +
// pinned run bar, extracted GUI code to src/ui.rs, added multi-FASTA concat.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
/// Is this pretty? No ... but is it well tested.... also no ... was an I on a deadline
/// well ... not really. BUT I learned a lot about Rust and sage and I'm glad I did.
/// I am more than happy to take PRs and suggestions for improvements!
mod ui;
mod version;

use eframe::egui;
use log::info;
use sage_cli::{input::Input, runner::Runner};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ui::*;

/// The subset of `SageLauncher` that survives between sessions. Run state
/// (thread handles, live progress, elapsed time) is intentionally excluded —
/// a saved run can't be resumed, only its parameters remembered.
#[derive(Serialize, Deserialize)]
struct PersistedState {
    config: Config,
    precursor_tolerance_type: ToleranceType,
    fragment_tolerance_type: ToleranceType,
    experiment: ExperimentType,
    active_page: Page,
}

// ─── ThreadMessage ────────────────────────────────────────────────────────────

#[derive(Debug)]
enum ThreadMessage {
    Progress(String),
    /// Sent once `Runner::new` succeeds, carrying the live spectra-scored
    /// counter so the GUI thread can poll it for a real progress percentage.
    RunnerReady(Arc<AtomicUsize>),
    /// One line captured from Sage's own `info`-level log output during a run.
    LogLine(String),
    Completed(Result<String, String>),
}

/// Longest the Sage-log scrollback is allowed to grow before old lines drop.
const MAX_LOG_LINES: usize = 500;

/// The channel to forward Sage's log lines into, while a run is active.
/// `log::set_boxed_logger` is process-global and installed once at startup,
/// long before any particular run's `mpsc::Sender` exists — this is how a
/// fresh sender gets handed to the logger each time `launch_application` runs.
static LOG_SENDER: OnceLock<Mutex<Option<Sender<ThreadMessage>>>> = OnceLock::new();

/// Wraps the normal `env_logger` logger (still prints to stderr as before)
/// and additionally forwards Sage's own log lines into the GUI, so the run
/// bar's log panel shows real output instead of nothing during the quiet
/// database-build/spectra-read phases.
struct GuiLogger {
    inner: env_logger::Logger,
}

impl log::Log for GuiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) && record.target().starts_with("sage_") {
            let sender = LOG_SENDER
                .get()
                .and_then(|mutex| mutex.lock().ok())
                .and_then(|guard| guard.clone());
            if let Some(sender) = sender {
                let _ = sender.send(ThreadMessage::LogLine(record.args().to_string()));
            }
        }
        self.inner.log(record);
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

// ─── SageLauncher ─────────────────────────────────────────────────────────────

pub struct SageLauncher {
    pub config: Config,
    pub status_message: String,
    pub precursor_tolerance_type: ToleranceType,
    pub fragment_tolerance_type: ToleranceType,
    thread_handle: Option<JoinHandle<()>>,
    message_receiver: Option<Receiver<ThreadMessage>>,
    pub start_time: Option<Instant>,
    pub elapsed_time: String,
    pub is_running: bool,
    pub active_page: Page,
    pub experiment: ExperimentType,
    /// Modifications tab: which box the master-list arrows act on.
    pub mod_target: ModTarget,
    /// Modifications tab: index of the selected preset in the master list.
    pub mod_selected_preset: Option<usize>,
    /// Modifications tab: free-type "Custom…" specificity buffer.
    pub mod_custom_key: String,
    /// Modifications tab: free-type "Custom…" mass buffer.
    pub mod_custom_mass: f32,
    /// Path of the temp-concatenated FASTA to delete after the run.
    temp_fasta_path: Option<std::path::PathBuf>,
    /// Live spectra-scored count from the running search, once `Runner` exists.
    search_progress: Option<Arc<AtomicUsize>>,
    /// Total MSn spectra across selected mzML files, pre-scanned before launch.
    /// `None` if any file's count couldn't be determined (unknown denominator).
    total_spectra: Option<usize>,
    /// Set to request cancellation; checked at phase boundaries in `run_sage`.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// True once Stop has been clicked, until the thread finishes cleaning up.
    pub stop_requested: bool,
    /// Sage's own log output captured during the current/last run. Persists
    /// after completion so the run can be reviewed; cleared on the next Run.
    pub log_lines: Vec<String>,
}

impl Default for SageLauncher {
    fn default() -> Self {
        Self {
            config: Config::default(),
            status_message: String::new(),
            precursor_tolerance_type: ToleranceType::Ppm,
            fragment_tolerance_type: ToleranceType::Ppm,
            thread_handle: None,
            message_receiver: None,
            start_time: None,
            is_running: false,
            elapsed_time: String::new(),
            active_page: Page::Experiment,
            experiment: ExperimentType::Custom,
            mod_target: ModTarget::Variable,
            mod_selected_preset: None,
            mod_custom_key: String::new(),
            mod_custom_mass: 0.0,
            temp_fasta_path: None,
            search_progress: None,
            total_spectra: None,
            cancel_flag: None,
            stop_requested: false,
            log_lines: Vec::new(),
        }
    }
}

impl SageLauncher {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx
            .set_visuals_of(egui::Theme::Dark, ui::dark_visuals());
        cc.egui_ctx
            .all_styles_mut(|style| style.text_styles = ui::readable_text_styles());

        let mut app = Self::default();
        if let Some(storage) = cc.storage {
            if let Some(persisted) = eframe::get_value::<PersistedState>(storage, eframe::APP_KEY) {
                app.config = persisted.config;
                // `static_mods`/`variable_mods` carry a `#[serde(skip)]` live
                // map alongside a serialized shadow map (`ModificationSpecificity`
                // has no Deserialize) — the shadow round-trips correctly, but
                // the live map the UI actually reads from comes back empty
                // unless it's rebuilt here.
                app.config.database.static_mods.sync_from_ser();
                app.config
                    .database
                    .variable_mods
                    .variable_mods
                    .sync_from_ser();
                app.precursor_tolerance_type = persisted.precursor_tolerance_type;
                app.fragment_tolerance_type = persisted.fragment_tolerance_type;
                app.experiment = persisted.experiment;
                app.active_page = persisted.active_page;
            }
        }
        app
    }
}

// ─── eframe::App ─────────────────────────────────────────────────────────────

impl eframe::App for SageLauncher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_thread_status();

        // ── Left sidebar navigation ───────────────────────────────────────────
        egui::SidePanel::left("nav")
            .resizable(false)
            .default_width(150.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("Navigation");
                ui.add_space(8.0);
                ui.selectable_value(&mut self.active_page, Page::Experiment, "Experiment");
                ui.selectable_value(
                    &mut self.active_page,
                    Page::FilesDatabase,
                    "Files & Database",
                );
                ui.selectable_value(&mut self.active_page, Page::Search, "Search");
                ui.selectable_value(&mut self.active_page, Page::Modifications, "Modifications");
                ui.selectable_value(&mut self.active_page, Page::Quant, "Quant");
                ui.selectable_value(&mut self.active_page, Page::RunInfo, "Run / Info");
            });

        // ── Bottom run bar ────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("run_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let run_btn = ui.add_enabled(!self.is_running, egui::Button::new("Run"));
                if run_btn.clicked() {
                    match self.launch_application() {
                        Ok(_) => self.status_message = "Analysis started".to_string(),
                        Err(e) => self.status_message = format!("Error: {}", e),
                    }
                }

                let stop_btn = ui
                    .add_enabled(
                        self.is_running && !self.stop_requested,
                        egui::Button::new("Stop"),
                    )
                    .on_hover_text(
                        "Stops at the end of the current step. A search already scoring \
                         spectra runs to completion.",
                    );
                if stop_btn.clicked() {
                    if let Some(flag) = &self.cancel_flag {
                        flag.store(true, Ordering::Relaxed);
                    }
                    self.stop_requested = true;
                    self.status_message = "Stopping — finishing the current step…".to_string();
                }

                if self.is_running {
                    ui.spinner();
                    if self.stop_requested {
                        ui.colored_label(egui::Color32::YELLOW, "Stopping");
                    } else {
                        ui.label("Processing");
                    }
                    if let Some(start_time) = self.start_time {
                        self.elapsed_time = format_duration(start_time.elapsed());
                        ui.label(format!("({})", self.elapsed_time));
                    }
                }

                let fraction = match (&self.search_progress, self.total_spectra) {
                    (Some(scored), Some(total)) if total > 0 => {
                        (scored.load(Ordering::Relaxed) as f32 / total as f32).min(1.0)
                    }
                    _ => 0.0,
                };
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .desired_width(120.0)
                        .show_percentage(),
                );
            });

            if !self.status_message.is_empty() {
                ui.colored_label(
                    if self.status_message.starts_with("Error") {
                        egui::Color32::RED
                    } else if self.status_message.starts_with("Search stopped") {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GREEN
                    },
                    &self.status_message,
                );
            }
            ui.add_space(4.0);
        });

        // ── Central content area ──────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match self.active_page {
                    Page::Experiment => self.page_experiment(ui),
                    Page::FilesDatabase => self.page_files_database(ui),
                    Page::Search => self.page_search(ui),
                    Page::Modifications => self.page_modifications(ui),
                    Page::Quant => self.page_quant(ui),
                    Page::RunInfo => self.page_run_info(ui),
                });
        });

        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let persisted = PersistedState {
            config: self.config.clone(),
            precursor_tolerance_type: self.precursor_tolerance_type,
            fragment_tolerance_type: self.fragment_tolerance_type,
            experiment: self.experiment,
            active_page: self.active_page,
        };
        eframe::set_value(storage, eframe::APP_KEY, &persisted);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Closing the window while a search is running skips `cleanup_thread`,
        // which would otherwise delete the temp concatenated FASTA.
        if let Some(p) = self.temp_fasta_path.take() {
            let _ = std::fs::remove_file(&p);
        }
    }
}

// ─── SageLauncher impl ────────────────────────────────────────────────────────

impl SageLauncher {
    fn check_thread_status(&mut self) {
        if let Some(receiver) = &self.message_receiver {
            match receiver.try_recv() {
                Ok(ThreadMessage::Progress(msg)) => {
                    self.status_message = msg;
                }
                Ok(ThreadMessage::RunnerReady(progress)) => {
                    self.search_progress = Some(progress);
                }
                Ok(ThreadMessage::LogLine(line)) => {
                    self.log_lines.push(line);
                    if self.log_lines.len() > MAX_LOG_LINES {
                        let excess = self.log_lines.len() - MAX_LOG_LINES;
                        self.log_lines.drain(0..excess);
                    }
                }
                Ok(ThreadMessage::Completed(result)) => {
                    // `stop_requested` only means Stop was clicked, not that the
                    // run actually stopped — `run_sage` may have already passed
                    // its last cancellation checkpoint and completed normally
                    // (writing real output) before this message arrives. Trust
                    // `result`, not the flag: only the "cancelled" error text
                    // (from `run_sage`'s own cancellation checks) means the run
                    // genuinely aborted and wrote nothing.
                    self.status_message = match result {
                        Ok(msg) => msg,
                        Err(err) if self.stop_requested && err == "cancelled" => {
                            "Search stopped. No output files were written.".to_string()
                        }
                        Err(err) => format!("Error: {}", err),
                    };
                    self.cleanup_thread();
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.cleanup_thread();
                }
            }
        }
    }

    fn cleanup_thread(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        self.thread_handle = None;
        self.message_receiver = None;
        self.start_time = None;
        self.is_running = false;
        self.search_progress = None;
        self.total_spectra = None;
        self.cancel_flag = None;
        self.stop_requested = false;
        if let Some(mutex) = LOG_SENDER.get() {
            if let Ok(mut guard) = mutex.lock() {
                *guard = None;
            }
        }
        if let Some(p) = self.temp_fasta_path.take() {
            let _ = std::fs::remove_file(&p);
        }
    }

    fn launch_application(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.database.fasta_paths.is_empty() {
            return Err("No FASTA files selected".into());
        }
        if self.config.mzml_paths.is_empty() {
            return Err("mzML file is not selected".into());
        }

        // Concatenate all selected FASTAs into a single temp file.
        let fasta_path = if self.config.database.fasta_paths.len() == 1 {
            // Single file — no temp copy needed.
            self.temp_fasta_path = None;
            self.config.database.fasta_paths[0]
                .to_string_lossy()
                .to_string()
        } else {
            let tmp = std::env::temp_dir().join(format!(
                "sagegui_concat_{}.fasta",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ));
            let mut out = std::fs::File::create(&tmp)?;
            for src in &self.config.database.fasta_paths {
                let content =
                    std::fs::read(src).map_err(|e| format!("{}: {}", src.display(), e))?;
                std::io::Write::write_all(&mut out, &content)?;
                // Ensure each file ends with a newline before the next header.
                if !content.ends_with(b"\n") {
                    std::io::Write::write_all(&mut out, b"\n")?;
                }
            }
            self.temp_fasta_path = Some(tmp.clone());
            tmp.to_string_lossy().to_string()
        };

        self.config.database.fasta_for_launch = fasta_path;

        let parallel = num_cpus::get() as u16 / 2;
        info!("Starting search with {} parallel threads", parallel);
        let parquet = false;
        let sage_input: Input = self.config.clone().into();

        self.search_progress = None;
        self.total_spectra = total_mzml_spectra(&self.config.mzml_paths);

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_flag = Some(cancel.clone());
        self.stop_requested = false;

        let (sender, receiver) = mpsc::channel();
        self.message_receiver = Some(receiver);
        self.log_lines.clear();
        LOG_SENDER
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .replace(sender.clone());

        let thread_handle = thread::spawn(move || {
            let _ = sender.send(ThreadMessage::Progress("Starting analysis...".to_string()));

            let result = match run_sage(sage_input, parallel, parquet, &sender, &cancel) {
                Ok(_) => Ok("Analysis completed successfully".to_string()),
                Err(e) => Err(e.to_string()),
            };

            let _ = sender.send(ThreadMessage::Completed(result));
        });

        self.thread_handle = Some(thread_handle);
        self.start_time = Some(Instant::now());
        self.is_running = true;

        Ok(())
    }
}

// ─── From<Config> for Input ───────────────────────────────────────────────────

impl From<Config> for Input {
    fn from(val: Config) -> Self {
        let quant = if val.quant_enabled {
            Some(val.quant.into())
        } else {
            None
        };

        let mzml_path_strings: Vec<String> = val
            .mzml_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        Input {
            database: val.database.into(),
            precursor_tol: val.precursor_tol.into(),
            fragment_tol: val.fragment_tol.into(),
            report_psms: Some(val.report_psms),
            chimera: Some(val.chimera),
            wide_window: Some(val.wide_window),
            min_peaks: Some(val.min_peaks as usize),
            max_peaks: Some(val.max_peaks as usize),
            max_fragment_charge: Some(val.max_fragment_charge),
            min_matched_peaks: Some(val.min_matched_peaks),
            precursor_charge: Some(val.precursor_charge),
            override_precursor_charge: Some(val.override_precursor_charge),
            isotope_errors: Some(val.isotope_errors),
            deisotope: Some(val.deisotope),
            quant,
            predict_rt: Some(val.predict_rt),
            output_directory: Some(val.output_directory),
            mzml_paths: Some(mzml_path_strings),
            bruker_config: None,
            protein_grouping: None,
            protein_grouping_peptide_fdr: None,
            annotate_matches: Some(val.annotate_matches),
            write_pin: Some(val.write_pin),
            write_report: None,
            score_type: Some(val.score_type),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Returns an error (any error — the caller only checks `stop_requested`, not
/// this text, except for the specific "cancelled" text `run_sage` itself
/// produces or forwards from `Runner::run`) as soon as `cancel` is observed
/// set. Also injected into the `Runner` via `with_cancel` before `run()`, so
/// a click during the scoring phase interrupts it too — see NOTES.md "Stop
/// button" for the fork-patch details (`neely/sage` commit `ed5f06c`).
fn run_sage(
    input: Input,
    parallel: u16,
    parquet: bool,
    sender: &Sender<ThreadMessage>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    println!("Running analysis... Building");
    let prefiltering = input.database.prefilter.unwrap_or(false);
    let build_msg = if prefiltering {
        "Prefiltering database in chunks… (no progress % for this phase)"
    } else {
        "Building peptide database…"
    };
    let _ = sender.send(ThreadMessage::Progress(build_msg.to_string()));
    let search = input.build()?;

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    let runner = Runner::new(search, parallel.into())?.with_cancel(cancel.clone());
    let _ = sender.send(ThreadMessage::RunnerReady(runner.progress.clone()));

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    let _ = sender.send(ThreadMessage::Progress(
        "Reading and searching spectra…".to_string(),
    ));
    println!("Running analysis... Executing");
    let _tel = runner.run(parallel.into(), parquet)?;
    Ok(())
}

/// Best-effort total MSn spectrum count across all selected mzML files, read
/// from each file's `<spectrumList count="N">` opening tag (a plain-text scan,
/// not a full XML parse — just enough to give the run-bar progress bar a
/// denominator). Returns `None` if any file's count can't be determined,
/// rather than reporting a partial (and misleadingly low) total.
fn total_mzml_spectra(paths: &[std::path::PathBuf]) -> Option<usize> {
    let mut total = 0usize;
    for path in paths {
        total += count_spectra_in_mzml(path)?;
    }
    Some(total)
}

fn count_spectra_in_mzml(path: &std::path::Path) -> Option<usize> {
    // Cap how much of the file we scan — the spectrumList tag is near the top,
    // well before any actual spectrum data, so this is normally a tiny read.
    const MAX_SCAN_BYTES: usize = 8 * 1024 * 1024;

    let file = std::fs::File::open(path).ok()?;
    let is_gz = path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);

    let mut reader: Box<dyn Read> = if is_gz {
        Box::new(flate2::read::MultiGzDecoder::new(file))
    } else {
        Box::new(file)
    };

    let mut buf = Vec::new();
    let mut chunk = [0u8; 65536];
    loop {
        let n = reader.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(count) = extract_spectrum_list_count(&buf) {
            return Some(count);
        }
        if buf.len() >= MAX_SCAN_BYTES {
            return None;
        }
    }
}

fn extract_spectrum_list_count(buf: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(buf);
    let tag_start = text.find("spectrumList")?;
    let after_tag = &text[tag_start..];
    let count_start = after_tag.find("count=")?;
    let after_count = &after_tag[count_start + "count=".len()..];
    let quote = after_count.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after_count[quote.len_utf8()..];
    let end = rest.find(quote)?;
    rest[..end].parse::<usize>().ok()
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// The window/taskbar icon (a crab wizard, cropped from `assets/sagegui_logo-removebg.png`
/// — see `assets/icon-master.png` for the full-size source). Without this,
/// eframe falls back to its own default: a white "e" on a black background.
fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon-256.png");
    let image = image::load_from_memory(bytes)
        .expect("bundled icon-256.png must decode")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> Result<(), eframe::Error> {
    // Default filter elevates Sage's own crates to `info` (their actual lib
    // names are sage_cli/sage_core/sage_cloudpath — stock Sage's own default
    // filter, "sage=info", is a no-op against those, since env_logger directive
    // matching requires an exact name or a "::" boundary). RUST_LOG still
    // overrides this if set.
    let env_logger_inner = env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("error,sage_cli=info,sage_core=info,sage_cloudpath=info"),
    )
    .build();
    log::set_max_level(env_logger_inner.filter());
    log::set_boxed_logger(Box::new(GuiLogger {
        inner: env_logger_inner,
    }))
    .expect("logger not already initialized");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Sage Launcher",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(SageLauncher::new(cc)))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wires a fresh channel into a default `SageLauncher` and marks it
    /// "running" the way `launch_application` would, without spawning Sage.
    fn running_launcher(stop_requested: bool) -> (SageLauncher, Sender<ThreadMessage>) {
        let mut app = SageLauncher::default();
        let (sender, receiver) = mpsc::channel();
        app.message_receiver = Some(receiver);
        app.thread_handle = Some(thread::spawn(|| {}));
        app.is_running = true;
        app.stop_requested = stop_requested;
        (app, sender)
    }

    #[test]
    fn stop_clicked_but_search_finished_reports_real_success() {
        // Reproduces the 2026-08-24 live-test bug: Stop was clicked after
        // `run_sage` passed its last cancellation checkpoint, so the search
        // completed and wrote output — `result` is `Ok`, not "cancelled".
        let (mut app, sender) = running_launcher(true);
        sender
            .send(ThreadMessage::Completed(Ok(
                "Analysis completed successfully".to_string(),
            )))
            .unwrap();

        app.check_thread_status();

        assert_eq!(app.status_message, "Analysis completed successfully");
        assert!(!app.is_running);
    }

    #[test]
    fn stop_clicked_before_completion_reports_no_output_written() {
        let (mut app, sender) = running_launcher(true);
        sender
            .send(ThreadMessage::Completed(Err("cancelled".to_string())))
            .unwrap();

        app.check_thread_status();

        assert_eq!(
            app.status_message,
            "Search stopped. No output files were written."
        );
        assert!(!app.is_running);
    }

    #[test]
    fn genuine_failure_without_stop_reports_error() {
        let (mut app, sender) = running_launcher(false);
        sender
            .send(ThreadMessage::Completed(Err(
                "database file not found".to_string()
            )))
            .unwrap();

        app.check_thread_status();

        assert_eq!(app.status_message, "Error: database file not found");
        assert!(!app.is_running);
    }

    /// Field-by-field persistence audit (requested 2026-08-24, after the
    /// modifications-shadow-map bug). Builds a `PersistedState` with every
    /// field set to a deliberately non-default, distinguishable value —
    /// including every nested field of `Config`/`DatabaseConfig` — round-trips
    /// it through serde exactly as `eframe::set_value`/`get_value` do, applies
    /// the same `sync_from_ser()` calls `SageLauncher::new` applies after a
    /// real restore, and asserts every single field survived unchanged. This
    /// is the mechanism-level proof that the mods bug was the only gap of its
    /// kind — a plain code review (grepping for `#[serde(skip)]`) already
    /// suggested that; this test actually exercises the round trip instead of
    /// trusting the review.
    #[test]
    fn every_config_field_survives_a_persistence_round_trip() {
        use sage_core::ion_series::Kind;
        use sage_core::scoring::ScoreType;
        use sage_core::tmt::Isobaric;
        use std::path::PathBuf;

        let mut ion_kinds = std::collections::HashMap::new();
        for kind in IonKindSelection::variants() {
            ion_kinds.insert(kind, false);
        }
        // Opposite of `IonKindSelection::default()` (which sets B, Y true).
        ion_kinds.insert(Kind::A, true);
        ion_kinds.insert(Kind::C, true);
        ion_kinds.insert(Kind::X, true);
        ion_kinds.insert(Kind::Z, true);

        let mut static_mods = StaticModConfig {
            static_mods: std::collections::HashMap::new(),
            static_mods_ser: std::collections::HashMap::new(),
        };
        static_mods.insert_key("C", 57.021464);
        static_mods.insert_key("^Q", -17.026549);

        let mut variable_mods = VariableModConfig {
            variable_mods: StaticModConfig {
                static_mods: std::collections::HashMap::new(),
                static_mods_ser: std::collections::HashMap::new(),
            },
        };
        variable_mods.variable_mods.insert_key("M", 15.994915);
        variable_mods.variable_mods.insert_key("N", 0.98402);

        let database = DatabaseConfig {
            bucket_size: 65536,
            enzyme: EnzymeConfig {
                missed_cleavages: 3,
                min_len: 8,
                max_len: 40,
                cleave_at: "RK".to_string(),
                enable_restrict: false,
                restrict_char: "D".to_string(),
                c_terminal: false,
                semi_enzymatic: true,
            },
            peptide_min_mass: 300.0,
            peptide_max_mass: 6000.0,
            ion_kinds: IonKindSelection { ion_kinds },
            min_ion_index: 5,
            max_variable_mods: 4,
            decoy_tag: Some("DECOY_".to_string()),
            generate_decoys: false,
            static_mods,
            variable_mods,
            prefilter: true,
            prefilter_chunk_size: 12345,
            prefilter_low_memory: false,
            fasta_paths: vec![
                PathBuf::from("/tmp/db1.fasta"),
                PathBuf::from("/tmp/db2.fasta"),
            ],
            fasta: String::new(),
            fasta_for_launch: "/tmp/should-not-persist.fasta".to_string(),
        };

        let original = PersistedState {
            config: Config {
                database,
                precursor_tol: ToleranceConfig::Da(-500.0, 123.4),
                fragment_tol: ToleranceConfig::Ppm(-33.0, 44.0),
                precursor_charge: (3, 6),
                isotope_errors: (-2, 5),
                deisotope: true,
                chimera: true,
                wide_window: true,
                predict_rt: false,
                min_peaks: 22,
                max_peaks: 200,
                min_matched_peaks: 9,
                max_fragment_charge: 2,
                report_psms: 3,
                mzml_paths: vec![
                    PathBuf::from("/tmp/a.mzML"),
                    PathBuf::from("/tmp/b.mzML.gz"),
                ],
                quant: QuantType::Tmt(
                    IsobarSelection {
                        selected: Isobaric::Tmt16,
                    },
                    TmtSettingsSer { level: 3, sn: true },
                ),
                quant_enabled: false,
                quant_class: SupportedQuantTypes::Tmt,
                annotate_matches: true,
                write_pin: true,
                score_type: ScoreType::OpenMSHyperScore,
                output_directory: "/custom/output/dir".to_string(),
                override_precursor_charge: true,
            },
            precursor_tolerance_type: ToleranceType::Da,
            fragment_tolerance_type: ToleranceType::Da,
            experiment: ExperimentType::Phospho,
            active_page: Page::Quant,
        };

        let json = serde_json::to_string(&original).expect("PersistedState must serialize");
        let mut restored: PersistedState =
            serde_json::from_str(&json).expect("a saved PersistedState must deserialize");
        // The exact fix from the mods-persistence bug: SageLauncher::new must
        // do this after every restore, or the live mod maps come back empty.
        restored.config.database.static_mods.sync_from_ser();
        restored
            .config
            .database
            .variable_mods
            .variable_mods
            .sync_from_ser();

        let od = &original.config.database;
        let rd = &restored.config.database;
        assert_eq!(rd.bucket_size, od.bucket_size);
        assert_eq!(rd.enzyme.missed_cleavages, od.enzyme.missed_cleavages);
        assert_eq!(rd.enzyme.min_len, od.enzyme.min_len);
        assert_eq!(rd.enzyme.max_len, od.enzyme.max_len);
        assert_eq!(rd.enzyme.cleave_at, od.enzyme.cleave_at);
        assert_eq!(rd.enzyme.enable_restrict, od.enzyme.enable_restrict);
        assert_eq!(rd.enzyme.restrict_char, od.enzyme.restrict_char);
        assert_eq!(rd.enzyme.c_terminal, od.enzyme.c_terminal);
        assert_eq!(rd.enzyme.semi_enzymatic, od.enzyme.semi_enzymatic);
        assert_eq!(rd.peptide_min_mass, od.peptide_min_mass);
        assert_eq!(rd.peptide_max_mass, od.peptide_max_mass);
        assert_eq!(rd.ion_kinds.ion_kinds, od.ion_kinds.ion_kinds);
        assert_eq!(rd.min_ion_index, od.min_ion_index);
        assert_eq!(rd.max_variable_mods, od.max_variable_mods);
        assert_eq!(rd.decoy_tag, od.decoy_tag);
        assert_eq!(rd.generate_decoys, od.generate_decoys);
        assert_eq!(rd.static_mods.static_mods, od.static_mods.static_mods);
        assert_eq!(
            rd.variable_mods.variable_mods.static_mods,
            od.variable_mods.variable_mods.static_mods
        );
        assert_eq!(rd.prefilter, od.prefilter);
        assert_eq!(rd.prefilter_chunk_size, od.prefilter_chunk_size);
        assert_eq!(rd.prefilter_low_memory, od.prefilter_low_memory);
        assert_eq!(rd.fasta_paths, od.fasta_paths);
        assert_eq!(
            rd.fasta_for_launch, "",
            "fasta_for_launch is `#[serde(skip)]` on purpose (recomputed at \
             launch from fasta_paths) — it must NOT come back from storage"
        );

        let oc = &original.config;
        let rc = &restored.config;
        assert!(matches!(rc.precursor_tol, ToleranceConfig::Da(l, u) if l == -500.0 && u == 123.4));
        assert!(matches!(rc.fragment_tol, ToleranceConfig::Ppm(l, u) if l == -33.0 && u == 44.0));
        assert_eq!(rc.precursor_charge, oc.precursor_charge);
        assert_eq!(rc.isotope_errors, oc.isotope_errors);
        assert_eq!(rc.deisotope, oc.deisotope);
        assert_eq!(rc.chimera, oc.chimera);
        assert_eq!(rc.wide_window, oc.wide_window);
        assert_eq!(rc.predict_rt, oc.predict_rt);
        assert_eq!(rc.min_peaks, oc.min_peaks);
        assert_eq!(rc.max_peaks, oc.max_peaks);
        assert_eq!(rc.min_matched_peaks, oc.min_matched_peaks);
        assert_eq!(rc.max_fragment_charge, oc.max_fragment_charge);
        assert_eq!(rc.report_psms, oc.report_psms);
        assert_eq!(rc.mzml_paths, oc.mzml_paths);
        match &rc.quant {
            QuantType::Tmt(isobar, tmt) => {
                assert!(matches!(isobar.selected, Isobaric::Tmt16));
                assert_eq!(tmt.level, 3);
                assert!(tmt.sn);
            }
            QuantType::Lfq(_) => panic!("expected QuantType::Tmt to survive the round trip"),
        }
        assert_eq!(rc.quant_enabled, oc.quant_enabled);
        assert_eq!(rc.quant_class, oc.quant_class);
        assert_eq!(rc.annotate_matches, oc.annotate_matches);
        assert_eq!(rc.write_pin, oc.write_pin);
        assert!(matches!(rc.score_type, ScoreType::OpenMSHyperScore));
        assert_eq!(rc.output_directory, oc.output_directory);
        assert_eq!(rc.override_precursor_charge, oc.override_precursor_charge);

        assert_eq!(
            restored.precursor_tolerance_type,
            original.precursor_tolerance_type
        );
        assert_eq!(
            restored.fragment_tolerance_type,
            original.fragment_tolerance_type
        );
        assert_eq!(restored.experiment, original.experiment);
        assert_eq!(restored.active_page, original.active_page);
    }
}
