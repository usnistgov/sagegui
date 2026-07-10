/// Is this pretty? No ... but is it well tested.... also no ... was an I on a deadline
/// well ... not really. BUT I learned a lot about Rust and sage and I'm glad I did.
/// I am more than happy to take PRs and suggestions for improvements!
use eframe::egui;
use egui::include_image;
use rfd::FileDialog;
use sage_cli::{
    input::{Input, LfqOptions, QuantOptions, TmtOptions, TmtSettings},
    runner::Runner,
};
use sage_core::modification::ModificationSpecificity;
use sage_core::{
    database::{Builder, EnzymeBuilder},
    lfq::LfqSettings,
    mass::Tolerance,
    tmt::Isobaric,
};
use sage_core::{ion_series::Kind, lfq::PeakScoringStrategy};
use sage_core::{lfq::IntegrationStrategy, scoring::ScoreType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
// BrukerSpectrumProcessor removed - no longer needed, using bruker_config: None

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EnzymeConfig {
    missed_cleavages: u8,
    min_len: usize,
    max_len: usize,
    cleave_at: String,
    enable_restrict: bool,
    restrict_char: String,
    c_terminal: bool,
    semi_enzymatic: bool,
}

impl Default for EnzymeConfig {
    fn default() -> Self {
        Self {
            missed_cleavages: 2,
            min_len: 5,
            max_len: 50,
            cleave_at: "KR".to_string(),
            enable_restrict: true,
            restrict_char: "P".to_string(),
            c_terminal: true,
            semi_enzymatic: false,
        }
    }
}

#[derive(Debug)]
enum ThreadMessage {
    Progress(String),
    Completed(Result<String, String>),
}

impl EnzymeConfig {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Enzyme Settings");
        ui.add(egui::Slider::new(&mut self.missed_cleavages, 0..=5).text("Missed Cleavages"));
        ui.add(egui::Slider::new(&mut self.min_len, 1..=20).text("Min Length"));
        ui.add(egui::Slider::new(&mut self.max_len, 6..=100).text("Max Length"));
        ui.horizontal(|ui| {
            ui.label("Cleave At:");
            ui.add(egui::TextEdit::singleline(&mut self.cleave_at).desired_width(10.0));
        });
        ui.horizontal(|ui| {
            ui.label("Restrict:");
            ui.checkbox(&mut self.enable_restrict, "Enable Restrict");
            if self.enable_restrict {
                ui.label("Restrict Char:");
                ui.add(egui::TextEdit::singleline(&mut self.restrict_char).desired_width(10.0));
                // Show warning if more than 1 character is written
                if self.restrict_char.len() > 1 {
                    ui.label("Warning: Only one character is allowed! Skipping restriction.");
                }
            }
        });
        ui.checkbox(&mut self.c_terminal, "C-Terminal");
        ui.checkbox(&mut self.semi_enzymatic, "Semi-Enzymatic");
    }
}

impl From<EnzymeConfig> for EnzymeBuilder {
    fn from(val: EnzymeConfig) -> Self {
        let restrict = if val.enable_restrict && val.restrict_char.len() == 1 {
            Some(val.restrict_char.chars().next().unwrap())
        } else {
            None
        };
        EnzymeBuilder {
            missed_cleavages: Some(val.missed_cleavages),
            min_len: Some(val.min_len),
            max_len: Some(val.max_len),
            cleave_at: Some(val.cleave_at),
            restrict: restrict.map(|c| c.to_string()),
            c_terminal: Some(val.c_terminal),
            semi_enzymatic: Some(val.semi_enzymatic),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct IonKindSelection {
    ion_kinds: HashMap<Kind, bool>,
}

impl From<IonKindSelection> for Vec<Kind> {
    fn from(val: IonKindSelection) -> Self {
        val.ion_kinds
            .iter()
            .filter_map(|(k, v)| if *v { Some(*k) } else { None })
            .collect()
    }
}

impl Default for IonKindSelection {
    fn default() -> Self {
        // Set all to false ...
        let mut ion_kinds = HashMap::new();
        for kind in IonKindSelection::variants() {
            ion_kinds.insert(kind, false);
        }
        ion_kinds.insert(Kind::B, true);
        ion_kinds.insert(Kind::Y, true);

        Self { ion_kinds }
    }
}

impl IonKindSelection {
    fn variants() -> [Kind; 6] {
        [Kind::A, Kind::B, Kind::C, Kind::X, Kind::Y, Kind::Z]
    }

    fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Select Ion Kinds:");
            for kind in IonKindSelection::variants() {
                let mut enabled = *self.ion_kinds.get(&kind).unwrap_or(&false);
                ui.checkbox(&mut enabled, format!("{:?}", kind));
                self.ion_kinds.insert(kind, enabled);
            }
        });
    }
}

#[derive(Debug, Clone)]
struct DatabaseConfig {
    bucket_size: usize,
    enzyme: EnzymeConfig,
    peptide_min_mass: f32,
    peptide_max_mass: f32,
    ion_kinds: IonKindSelection,
    min_ion_index: u32,
    max_variable_mods: u32,
    decoy_tag: Option<String>,
    generate_decoys: bool,
    static_mods: StaticModConfig,
    variable_mods: VariableModConfig,
    fasta: String,
}

impl From<DatabaseConfig> for Builder {
    fn from(val: DatabaseConfig) -> Self {
        Builder {
            bucket_size: Some(val.bucket_size),
            enzyme: Some(val.enzyme.into()),
            peptide_min_mass: Some(val.peptide_min_mass),
            peptide_max_mass: Some(val.peptide_max_mass),
            ion_kinds: Some(val.ion_kinds.into()),
            min_ion_index: Some(val.min_ion_index as usize),
            max_variable_mods: Some(val.max_variable_mods as usize),
            decoy_tag: val.decoy_tag,
            generate_decoys: Some(val.generate_decoys),
            fasta: Some(val.fasta),
            static_mods: Some(val.static_mods.as_hashmap()),
            variable_mods: Some(val.variable_mods.as_hashmap()),
            prefilter: None,
            prefilter_chunk_size: None,
            prefilter_low_memory: None,
        }
    }
}

#[derive(Debug, Clone)]
struct StaticModConfig {
    static_mods: HashMap<ModificationSpecificity, f32>,
    new_mod_buffer: String,
    new_mass_buffer: f32,
}

impl Default for StaticModConfig {
    fn default() -> Self {
        Self {
            static_mods: HashMap::from([(ModificationSpecificity::Residue(b'C'), 57.021464f32)]),
            new_mod_buffer: "W".to_string(),
            new_mass_buffer: f32::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct VariableModConfig {
    // variable_mods: HashMap<ModificationSpecificity, Vec<f32>>,
    // Adding multiple variable mods later ...
    variable_mods: StaticModConfig,
}

impl Default for VariableModConfig {
    fn default() -> Self {
        let hm = HashMap::from([(ModificationSpecificity::Residue(b'M'), 15.994915f32)]);
        let def = StaticModConfig {
            static_mods: hm,
            new_mod_buffer: "M".to_string(),
            new_mass_buffer: 15.994915f32,
        };

        Self { variable_mods: def }
    }
}

impl VariableModConfig {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Variable Modifications");
        self.variable_mods._update_section(ui);
    }

    fn as_hashmap(&self) -> HashMap<String, Vec<f32>> {
        let mut hm = HashMap::new();
        for (mod_, mass) in self.variable_mods.static_mods.iter() {
            hm.insert(mod_.to_string(), vec![*mass]);
        }
        hm
    }
}

impl StaticModConfig {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Static Modifications");
        self._update_section(ui);
    }
    fn _update_section(&mut self, ui: &mut egui::Ui) {
        ui.label("Modifications are applied to all peptides.");

        // 2 parter ...
        // 1. Input boxes + button to add new mods
        // 2. List of the set mods + button to remove mods

        let ip = ui.horizontal(|ui| {
            ui.label("Add Modification:");
            ui.add(egui::DragValue::new(&mut self.new_mass_buffer).speed(0.01));

            ui.add(egui::TextEdit::singleline(&mut self.new_mod_buffer).desired_width(10.0));
            // ui.text_edit_singleline(&mut self.new_mod_buffer);

            ModificationSpecificity::from_str(&self.new_mod_buffer)
        });

        let parsed_mod = ip.inner;

        // Only allow adding valid #[cfg(test)]
        if let Ok(mod_) = parsed_mod {
            if ui.button("Add").clicked() {
                self.static_mods.insert(mod_, self.new_mass_buffer);
            }
        } else {
            ui.label("Invalid Modification ('C', ']', '$' and '^M' are all valid examples)");
        }
        ui.add_space(10.0);

        let remove_queue = self.update_deletion_queue(ui);
        for mod_ in remove_queue {
            self.static_mods.remove(&mod_);
        }
    }

    fn update_deletion_queue(&self, ui: &mut egui::Ui) -> Vec<ModificationSpecificity> {
        let mut to_remove = Vec::new();
        // In a container
        ui.group(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 10.0);
            ui.label("Current Modifications:");
            for (mod_, mass) in self.static_mods.iter() {
                ui.horizontal(|ui| {
                    ui.label(mod_.to_string());
                    ui.label(format!("{:.4}", mass));

                    if ui.button("Remove").clicked() {
                        to_remove.push(*mod_);
                    }
                });
            }
        });

        to_remove
    }

    fn as_hashmap(&self) -> HashMap<String, f32> {
        let mut hm = HashMap::new();
        for (mod_, mass) in self.static_mods.iter() {
            hm.insert(mod_.to_string(), *mass);
        }
        hm
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            bucket_size: 32768,
            enzyme: EnzymeConfig::default(),
            peptide_min_mass: 500.0,
            peptide_max_mass: 5000.0,
            ion_kinds: IonKindSelection::default(),
            min_ion_index: 2,
            max_variable_mods: 2,
            decoy_tag: Some("rev_".to_string()),
            generate_decoys: true,
            fasta: String::new(),
            static_mods: StaticModConfig::default(),
            variable_mods: VariableModConfig::default(),
        }
    }
}

impl DatabaseConfig {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        // Enzyme Configuration
        ui.group(|ui| {
            self.enzyme.update_section(ui);
        });

        ui.group(|ui| {
            ui.heading("Modifications");
            self.static_mods.update_section(ui);
            self.variable_mods.update_section(ui);
        });

        // Mass Ranges
        ui.group(|ui| {
            ui.heading("Mass Ranges");
            ui.add(
                egui::Slider::new(&mut self.peptide_min_mass, 300.0..=1000.0)
                    .text("Peptide Min Mass"),
            );
            ui.add(
                egui::Slider::new(&mut self.peptide_max_mass, 3000.0..=7000.0)
                    .text("Peptide Max Mass"),
            );
        });

        ui.group(|ui| {
            ui.heading("Ion Kinds");
            self.ion_kinds.update_section(ui);
        });

        ui.group(|ui| {
            ui.heading("Extras");
            ui.checkbox(&mut self.generate_decoys, "Generate Decoys");
            ui.add(egui::Slider::new(&mut self.bucket_size, 8192..=65536).text("Bucket Size"));
        });
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
enum ToleranceConfig {
    #[serde(rename = "da")]
    Da(f32, f32),
    #[serde(rename = "ppm")]
    Ppm(f32, f32),
}

impl Default for ToleranceConfig {
    fn default() -> Self {
        Self::Ppm(-10.0, 10.0)
    }
}

impl ToleranceConfig {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        match self {
            ToleranceConfig::Ppm(a, b) => {
                ui.add(egui::DragValue::new(a).speed(1));
                ui.add(egui::DragValue::new(b).speed(1));
            }
            ToleranceConfig::Da(a, b) => {
                ui.add(egui::DragValue::new(a).speed(0.01));
                ui.add(egui::DragValue::new(b).speed(0.01));
            }
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct IsobarSelection {
    selected: Isobaric,
}

impl From<IsobarSelection> for Isobaric {
    fn from(val: IsobarSelection) -> Self {
        val.selected
    }
}

impl Default for IsobarSelection {
    fn default() -> Self {
        Self {
            selected: Isobaric::Tmt6,
        }
    }
}

impl IsobarSelection {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.radio_value(&mut self.selected, Isobaric::Tmt6, "TMT 6-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt10, "TMT 10-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt11, "TMT 11-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt16, "TMT 16-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt18, "TMT 18-plex");
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Copy)]
enum SupportedQuantTypes {
    #[serde(rename = "lfq")]
    Lfq,
    #[serde(rename = "tmt")]
    Tmt,
}

#[derive(Serialize, Debug, Clone)]
enum QuantType {
    Lfq(LfqSettings),
    Tmt(IsobarSelection, TmtSettings),
}

impl QuantType {
    fn update_section(&mut self, ui: &mut egui::Ui) {
        match self {
            QuantType::Lfq(lfq) => {
                ui.group(|ui| {
                    ui.heading("LFQ Settings");
                    ui.label("PPM Tolerance");
                    ui.add(egui::DragValue::new(&mut lfq.ppm_tolerance).speed(1.0));
                    ui.add(
                        egui::Slider::new(&mut lfq.spectral_angle, 0.0..=1.0)
                            .text("Spectral Angle"),
                    );
                    ui.checkbox(&mut lfq.combine_charge_states, "Combine Charge States");
                    // TODO add selection for the scoring+integration strategy.
                });
            }
            QuantType::Tmt(isobar, tmt) => {
                ui.group(|ui| {
                    ui.heading("TMT Settings");
                    isobar.update_section(ui);
                    ui.add(egui::Slider::new(&mut tmt.level, 1..=10).text("Level"));
                });
            }
        }
    }

    fn type_default(supported: SupportedQuantTypes) -> Self {
        match supported {
            SupportedQuantTypes::Lfq => Self::Lfq(LfqSettings::default()),
            SupportedQuantTypes::Tmt => {
                Self::Tmt(IsobarSelection::default(), TmtSettings::default())
            }
        }
    }
}

impl From<QuantType> for QuantOptions {
    fn from(val: QuantType) -> Self {
        match val {
            // This is in some ways dumb bc sage will immediately make it a "LFQSettings"
            // But If I make the lfq settings a struct, I cannot use the sage_cli::input::Input
            // but rather have to build directly the sage_cli::input::Search.

            // Might do that in a later version.
            QuantType::Lfq(lfq) => {
                let lfq_options = LfqOptions {
                    peak_scoring: Some(PeakScoringStrategy::Hybrid),
                    integration: Some(IntegrationStrategy::Sum),
                    spectral_angle: Some(lfq.spectral_angle),
                    ppm_tolerance: Some(lfq.ppm_tolerance),
                    mobility_pct_tolerance: None,
                    combine_charge_states: Some(lfq.combine_charge_states),
                    peptide_q_value: None,
                };
                QuantOptions {
                    tmt: None,
                    tmt_options: None,
                    lfq: Some(true),
                    lfq_options: Some(lfq_options),
                }
            }
            QuantType::Tmt(isobar, tmt) => {
                let tmt_options = TmtOptions {
                    level: Some(tmt.level),
                    sn: Some(tmt.sn),
                };

                QuantOptions {
                    tmt: Some(isobar.into()),
                    tmt_options: Some(tmt_options),
                    lfq: None,
                    lfq_options: None,
                }
            }
        }
    }
}

impl Default for QuantType {
    fn default() -> Self {
        Self::Lfq(LfqSettings::default())
    }
}

#[derive(Debug, Clone)]
struct Config {
    database: DatabaseConfig,
    precursor_tol: ToleranceConfig,
    fragment_tol: ToleranceConfig,
    precursor_charge: (u8, u8),
    isotope_errors: (i8, i8),
    deisotope: bool,

    chimera: bool,
    wide_window: bool,
    predict_rt: bool,
    min_peaks: u32,
    max_peaks: u32,
    min_matched_peaks: u16,
    max_fragment_charge: u8,
    report_psms: usize,
    mzml_paths: Vec<PathBuf>,
    dotd_paths: Vec<PathBuf>,

    quant: QuantType,
    quant_enabled: bool,
    quant_class: SupportedQuantTypes,

    // bruker_spectrum_processor removed - using bruker_config: None in Input
    annotate_matches: bool,
    write_pin: bool,
    score_type: ScoreType,
    output_directory: String,
}

impl From<Config> for Input {
    fn from(val: Config) -> Self {
        let quant = if val.quant_enabled {
            Some(val.quant.into())
        } else {
            None
        };

        let mut mzml_path_strings: Vec<String> = val
            .mzml_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let dotd_path_strings: Vec<String> = val
            .dotd_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        mzml_path_strings.extend(dotd_path_strings);

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
            override_precursor_charge: None, // TODO
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

impl Default for Config {
    fn default() -> Self {
        let cwd_str: Option<String> = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string());
        Self {
            database: DatabaseConfig::default(),
            precursor_tol: ToleranceConfig::default(),
            fragment_tol: ToleranceConfig::default(),
            precursor_charge: (2, 4),
            isotope_errors: (-1, 3),
            deisotope: false,
            chimera: false,
            wide_window: false,
            predict_rt: true,
            min_peaks: 15,
            max_peaks: 150,
            min_matched_peaks: 6,
            max_fragment_charge: 1,
            report_psms: 1,
            mzml_paths: Vec::new(),
            dotd_paths: Vec::new(),
            quant_enabled: true,
            quant: QuantType::default(),
            quant_class: SupportedQuantTypes::Lfq,
            annotate_matches: false,
            write_pin: false,
            score_type: ScoreType::SageHyperScore,
            output_directory: cwd_str.unwrap_or_else(|| "output".to_string()),
        }
    }
}

struct SageLauncher {
    config: Config,
    status_message: String,
    precursor_tolerance_type: ToleranceType,
    fragment_tolerance_type: ToleranceType,
    thread_handle: Option<JoinHandle<()>>,
    message_receiver: Option<Receiver<ThreadMessage>>,
    start_time: Option<Instant>,
    elapsed_time: String,
    is_running: bool,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ToleranceType {
    Ppm,
    Da,
}

impl ToleranceType {
    fn get_default_tolerance(&self) -> ToleranceConfig {
        match self {
            ToleranceType::Ppm => ToleranceConfig::Ppm(-10.0, 10.0),
            ToleranceType::Da => ToleranceConfig::Da(-0.02, 0.02),
        }
    }
}

impl From<ToleranceConfig> for Tolerance {
    fn from(val: ToleranceConfig) -> Self {
        match val {
            ToleranceConfig::Ppm(a, b) => Tolerance::Ppm(a, b),
            ToleranceConfig::Da(a, b) => Tolerance::Da(a, b),
        }
    }
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
        }
    }
}

impl SageLauncher {
    fn update_tolerances(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Precursor Tolerance");
            ui.radio_value(
                &mut self.precursor_tolerance_type,
                ToleranceType::Ppm,
                "PPM",
            );
            ui.radio_value(&mut self.precursor_tolerance_type, ToleranceType::Da, "Da");

            match (self.precursor_tolerance_type, self.config.precursor_tol) {
                (ToleranceType::Ppm, ToleranceConfig::Da(..)) => {
                    self.config.precursor_tol =
                        self.precursor_tolerance_type.get_default_tolerance()
                }
                (ToleranceType::Da, ToleranceConfig::Ppm(..)) => {
                    self.config.precursor_tol =
                        self.precursor_tolerance_type.get_default_tolerance()
                }
                _ => {}
            }
            self.config.precursor_tol.update_section(ui);
        });

        ui.group(|ui| {
            ui.heading("Fragment Tolerance");
            ui.radio_value(&mut self.fragment_tolerance_type, ToleranceType::Ppm, "PPM");
            ui.radio_value(&mut self.fragment_tolerance_type, ToleranceType::Da, "Da");

            match (self.fragment_tolerance_type, self.config.fragment_tol) {
                (ToleranceType::Ppm, ToleranceConfig::Da(..)) => {
                    self.config.fragment_tol = self.fragment_tolerance_type.get_default_tolerance()
                }
                (ToleranceType::Da, ToleranceConfig::Ppm(..)) => {
                    self.config.fragment_tol = self.fragment_tolerance_type.get_default_tolerance()
                }
                _ => {}
            }
            self.config.fragment_tol.update_section(ui);
        });
    }

    fn update_general_settings(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.config.min_peaks, 5..=50).text("Min Peaks"));
        ui.add(egui::Slider::new(&mut self.config.max_peaks, 50..=500).text("Max Peaks"));
        ui.add(
            egui::Slider::new(&mut self.config.min_matched_peaks, 3..=20).text("Min Matched Peaks"),
        );
        ui.add(
            egui::Slider::new(&mut self.config.max_fragment_charge, 1..=5)
                .text("Max Fragment Charge"),
        );
        ui.add(egui::Slider::new(&mut self.config.report_psms, 1..=10).text("Report PSMs"));

        ui.checkbox(&mut self.config.deisotope, "Deisotope");
        ui.checkbox(&mut self.config.chimera, "Chimera");
        ui.checkbox(&mut self.config.wide_window, "Wide Window");
        ui.checkbox(&mut self.config.predict_rt, "Predict RT");
    }

    fn update_quant_options(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.config.quant_enabled, "Enable Quantification");

        if self.config.quant_enabled {
            ui.label("Quantification Type");
            ui.radio_value(
                &mut self.config.quant_class,
                SupportedQuantTypes::Lfq,
                "Label-Free Quantification (LFQ)",
            );
            ui.radio_value(
                &mut self.config.quant_class,
                SupportedQuantTypes::Tmt,
                "Tandem Mass Tag (TMT)",
            );

            // TODO: remove this clone
            match (self.config.quant_class, self.config.quant.clone()) {
                (SupportedQuantTypes::Lfq, QuantType::Lfq(..)) => {}
                (SupportedQuantTypes::Tmt, QuantType::Tmt(..)) => {}
                _ => {
                    self.config.quant = QuantType::type_default(self.config.quant_class);
                }
            }

            self.config.quant.update_section(ui);
        }
    }
}

impl eframe::App for SageLauncher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update process status and elapsed time
        self.check_thread_status();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Sage Launcher");

                // Thread Status Section
                if self.is_running {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.spinner(); // Show spinning animation
                        ui.colored_label(egui::Color32::GREEN, "Processing");
                        if let Some(start_time) = self.start_time {
                            self.elapsed_time = format_duration(start_time.elapsed());
                            ui.label(format!("({})", self.elapsed_time));
                        }
                    });

                    ui.add_space(10.0);
                }

                ui.add(egui::Image::new(include_image!("../assets/logo.png")).max_width(400.0));
                ui.add_space(20.0);

                // File Selection Section
                ui.collapsing("File Selection", |ui| {
                    // Output loc picker
                    ui.horizontal(|ui| {
                        ui.label("Output Location:");
                        ui.text_edit_singleline(&mut self.config.output_directory);
                        if ui.button("Browse").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.config.output_directory = path.display().to_string();
                            }
                        }
                    });

                    // FASTA file picker
                    ui.horizontal(|ui| {
                        ui.label("FASTA File:");
                        ui.text_edit_singleline(&mut self.config.database.fasta);
                        if ui.button("Browse").clicked() {
                            if let Some(path) = FileDialog::new()
                                .add_filter("FASTA", &["fasta"])
                                .pick_file()
                            {
                                self.config.database.fasta = path.display().to_string();
                            }
                        }
                    });

                    // mzML file picker
                    ui.horizontal(|ui| {
                        ui.label("mzML/.d Files:");
                        if ui.button("Pick mzmls").clicked() {
                            if let Some(paths) = FileDialog::new()
                                .add_filter("mzml", &["mzML", "gz", "mzml"])
                                .pick_files()
                            {
                                self.config.mzml_paths = paths
                            } else {
                                self.config.mzml_paths = Vec::new();
                            }
                        }
                        if ui.button("Pick .d files").clicked() {
                            if let Some(paths) = FileDialog::new()
                                .add_filter("Bruker Raw Data", &["d"])
                                .pick_folders()
                            {
                                self.config.dotd_paths = paths
                            } else {
                                self.config.dotd_paths = Vec::new();
                            }
                        }
                    });

                    // Show the picked files in a scrollable list
                    ui.label("Picked Files:");
                    ui.spacing();
                    ui.separator();
                    ui.spacing();
                    for path in self
                        .config
                        .mzml_paths
                        .iter()
                        .chain(self.config.dotd_paths.iter())
                    {
                        ui.label(path.to_string_lossy());
                    }
                });

                // Database Configuration Section
                ui.collapsing("Database Configuration", |ui| {
                    self.config.database.update_section(ui);
                });

                // Tolerance Configuration Section
                ui.collapsing("Tolerance Settings", |ui| {
                    self.update_tolerances(ui);
                });

                ui.collapsing("Quantification Options", |ui| {
                    self.update_quant_options(ui);
                });

                // General Settings Section
                ui.collapsing("General Settings", |ui| {
                    self.update_general_settings(ui);
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    let launch_button = ui.add_enabled(
                        !self.is_running, // Disable when process is running
                        egui::Button::new("Launch"),
                    );

                    if launch_button.clicked() {
                        match self.launch_application() {
                            Ok(_) => self.status_message = "Analysis started".to_string(),
                            Err(e) => self.status_message = format!("Error: {}", e),
                        }
                    }
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

                ui.add_space(20.0);
                ui.collapsing("Info/Help", |ui| {
                    ui.label("Sage GUI Version:");
                    ui.label(env!("CARGO_PKG_VERSION"));
                    ui.label("Sage Engine Version: v0.15.0-beta.2");
                    ui.add_space(10.0);
                    ui.label("Original Author: J.Sebastian Paez");
                    ui.label("Current Maintainer: neely");
                    ui.label("Repository: https://github.com/neely/sagegui");
                    ui.label("License: Apache-2.0");
                    ui.add_space(20.0);
                    ui.label("Search engine repository: https://github.com/lazear/sage");
                    ui.label("If you use Sage in a scientific publication, please cite the following paper: 'Sage: An Open-Source Tool for Fast Proteomics Searching and Quantification at Scale' https://doi.org/10.1021/acs.jproteome.3c00486");
                });
                ui.add_space(50.0);
            });
        });

        // Request continuous repaint while process is running
        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

impl SageLauncher {
    fn check_thread_status(&mut self) {
        if let Some(receiver) = &self.message_receiver {
            // Check for messages from the thread
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
                Err(mpsc::TryRecvError::Empty) => {
                    // No message available, continue running
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Thread has finished or been disconnected
                    self.cleanup_thread();
                }
            }
        }
    }

    fn cleanup_thread(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            // Wait for the thread to finish
            let _ = handle.join();
        }
        self.thread_handle = None;
        self.message_receiver = None;
        self.start_time = None;
        self.is_running = false;
    }

    fn launch_application(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.database.fasta.is_empty() {
            return Err("FASTA file is not selected".into());
        }
        if self.config.mzml_paths.is_empty() && self.config.dotd_paths.is_empty() {
            return Err("mzML file is not selected".into());
        }

        let parallel = num_cpus::get() as u16 / 2;
        println!("Parallel: {}", parallel);
        let parquet = false;
        let sage_input: Input = self.config.clone().into();

        // Create channel for thread communication
        let (sender, receiver) = mpsc::channel();
        self.message_receiver = Some(receiver);

        // Spawn the analysis thread
        let thread_handle = thread::spawn(move || {
            // Send initial progress
            let _ = sender.send(ThreadMessage::Progress("Starting analysis...".to_string()));

            // TODO: I could build the input and on update and provide real time feedback.

            // Run the analysis
            let result = match run_sage(sage_input, parallel, parquet) {
                Ok(_) => Ok("Analysis completed successfully".to_string()),
                Err(e) => Err(e.to_string()),
            };

            // Send completion message
            let _ = sender.send(ThreadMessage::Completed(result));
        });

        self.thread_handle = Some(thread_handle);
        self.start_time = Some(Instant::now());
        self.is_running = true;

        Ok(())
    }
}

fn run_sage(input: Input, parallel: u16, parquet: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running analysis... Building");
    let search = input.build()?;
    let runner = Runner::new(search, parallel.into())?;
    println!("Running analysis... Executing");
    let _tel = runner.run(parallel.into(), parquet)?;
    Ok(())
}

// Helper function to format duration
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
    // Setup env logger
    env_logger::init();

    let options = eframe::NativeOptions {
        // initial_window_size: Some(egui::vec2(600.0, 800.0)),
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
