/// Is this pretty? No ... but is it well tested.... also no ... was an I on a deadline
/// well ... not really. BUT I learned a lot about Rust and sage and I'm glad I did.
/// I am more than happy to take PRs and suggestions for improvements!
mod ui;
mod version;

use eframe::egui;
use log::info;
use sage_cli::{input::Input, runner::Runner};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ui::*;

// ─── ThreadMessage ────────────────────────────────────────────────────────────

#[derive(Debug)]
enum ThreadMessage {
    Progress(String),
    Completed(Result<String, String>),
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
        }
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

                if self.is_running {
                    ui.spinner();
                    ui.colored_label(egui::Color32::GREEN, "Processing");
                    if let Some(start_time) = self.start_time {
                        self.elapsed_time = format_duration(start_time.elapsed());
                        ui.label(format!("({})", self.elapsed_time));
                    }
                }

                ui.add(
                    egui::ProgressBar::new(0.0)
                        .desired_width(120.0)
                        .show_percentage(),
                );
            });

            if !self.status_message.is_empty() {
                ui.colored_label(
                    if self.status_message.starts_with("Error") {
                        egui::Color32::RED
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
}

// ─── SageLauncher impl ────────────────────────────────────────────────────────

impl SageLauncher {
    fn check_thread_status(&mut self) {
        if let Some(receiver) = &self.message_receiver {
            match receiver.try_recv() {
                Ok(ThreadMessage::Progress(msg)) => {
                    self.status_message = msg;
                }
                Ok(ThreadMessage::Completed(result)) => {
                    match result {
                        Ok(msg) => self.status_message = msg,
                        Err(err) => self.status_message = format!("Error: {}", err),
                    }
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
            self.config.database.fasta_paths[0].to_string_lossy().to_string()
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
                let content = std::fs::read(src)
                    .map_err(|e| format!("{}: {}", src.display(), e))?;
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

        let (sender, receiver) = mpsc::channel();
        self.message_receiver = Some(receiver);

        let thread_handle = thread::spawn(move || {
            let _ = sender.send(ThreadMessage::Progress("Starting analysis...".to_string()));

            let result = match run_sage(sage_input, parallel, parquet) {
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

fn run_sage(input: Input, parallel: u16, parquet: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running analysis... Building");
    let search = input.build()?;
    let runner = Runner::new(search, parallel.into())?;
    println!("Running analysis... Executing");
    let _tel = runner.run(parallel.into(), parquet)?;
    Ok(())
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

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    eframe::run_native(
        "Sage Launcher",
        options,
        Box::new(|_cc| {
            egui_extras::install_image_loaders(&_cc.egui_ctx);
            Ok(Box::new(SageLauncher::default()))
        }),
    )
}
