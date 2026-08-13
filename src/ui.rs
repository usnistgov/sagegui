use eframe::egui;
use egui::include_image;
use rfd::FileDialog;
use sage_cli::input::{LfqOptions, QuantOptions, TmtOptions, TmtSettings};
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

use crate::SageLauncher;

// ─── Page enum ───────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Page {
    Experiment,
    FilesDatabase,
    Search,
    Modifications,
    Quant,
    RunInfo,
}

// ─── ExperimentType ──────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum ExperimentType {
    #[default]
    Custom,
    TrypticLfq,
    WideOpen,
    Phospho,
    SemiTryptic,
}
// ─── ToleranceType ───────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToleranceType {
    Ppm,
    Da,
}

impl ToleranceType {
    pub fn get_default_tolerance(&self) -> ToleranceConfig {
        match self {
            ToleranceType::Ppm => ToleranceConfig::Ppm(-10.0, 10.0),
            ToleranceType::Da => ToleranceConfig::Da(-0.02, 0.02),
        }
    }
}

// ─── EnzymeConfig ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnzymeConfig {
    pub missed_cleavages: u8,
    pub min_len: usize,
    pub max_len: usize,
    pub cleave_at: String,
    pub enable_restrict: bool,
    pub restrict_char: String,
    pub c_terminal: bool,
    pub semi_enzymatic: bool,
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

impl EnzymeConfig {
    pub fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Enzyme Settings");
        ui.add(egui::Slider::new(&mut self.missed_cleavages, 0..=5).text("Missed Cleavages"))
            .on_hover_text("Max enzyme cut sites a peptide may skip.");
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
                if self.restrict_char.len() > 1 {
                    ui.label("Warning: Only one character is allowed! Skipping restriction.");
                }
            }
        });
        ui.checkbox(&mut self.c_terminal, "C-Terminal");
        ui.checkbox(&mut self.semi_enzymatic, "Semi-Enzymatic")
            .on_hover_text("Allow one non-enzymatic terminus. Doubles+ search space.");
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

// ─── IonKindSelection ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IonKindSelection {
    pub ion_kinds: HashMap<Kind, bool>,
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
    pub fn variants() -> [Kind; 6] {
        [Kind::A, Kind::B, Kind::C, Kind::X, Kind::Y, Kind::Z]
    }

    pub fn update_section(&mut self, ui: &mut egui::Ui) {
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

// ─── StaticModConfig ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticModConfig {
    // Stored as String→f32 for serde; HashMap<ModificationSpecificity,f32> is
    // converted on use because ModificationSpecificity has no Deserialize.
    #[serde(default)]
    pub static_mods_ser: HashMap<String, f32>,
    #[serde(skip)]
    pub static_mods: HashMap<ModificationSpecificity, f32>,
    #[serde(default, skip)]
    pub new_mod_buffer: String,
    #[serde(default, skip)]
    pub new_mass_buffer: f32,
}

impl StaticModConfig {
    /// Re-sync the string map from the live map (call before serialising).
    fn sync_to_ser(&mut self) {
        self.static_mods_ser.clear();
        for (k, v) in &self.static_mods {
            self.static_mods_ser.insert(k.to_string(), *v);
        }
    }

    /// Re-sync the live map from the string map (call after deserialising).
    pub fn sync_from_ser(&mut self) {
        self.static_mods.clear();
        for (s, v) in &self.static_mods_ser {
            if let Ok(k) = ModificationSpecificity::from_str(s) {
                self.static_mods.insert(k, *v);
            }
        }
    }
}

impl Default for StaticModConfig {
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert(ModificationSpecificity::Residue(b'C'), 57.021464f32);
        let mut ser = HashMap::new();
        ser.insert("C".to_string(), 57.021464f32);
        Self {
            static_mods: m,
            static_mods_ser: ser,
            new_mod_buffer: "W".to_string(),
            new_mass_buffer: f32::default(),
        }
    }
}

impl StaticModConfig {
    pub fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Static Modifications");
        self._update_section(ui);
    }
    pub fn _update_section(&mut self, ui: &mut egui::Ui) {
        ui.label("Modifications are applied to all peptides.");

        let ip = ui.horizontal(|ui| {
            ui.label("Add Modification:");
            ui.add(egui::DragValue::new(&mut self.new_mass_buffer).speed(0.01));
            ui.add(egui::TextEdit::singleline(&mut self.new_mod_buffer).desired_width(10.0));
            ModificationSpecificity::from_str(&self.new_mod_buffer)
        });

        let parsed_mod = ip.inner;

        if let Ok(mod_) = parsed_mod {
            if ui.button("Add").clicked() {
                self.static_mods.insert(mod_, self.new_mass_buffer);
                self.sync_to_ser();
            }
        } else {
            ui.label("Invalid Modification ('C', ']', '$' and '^M' are all valid examples)");
        }
        ui.add_space(10.0);

        let remove_queue = self.update_deletion_queue(ui);
        for mod_ in remove_queue {
            self.static_mods.remove(&mod_);
            self.sync_to_ser();
        }
    }

    fn update_deletion_queue(&self, ui: &mut egui::Ui) -> Vec<ModificationSpecificity> {
        let mut to_remove = Vec::new();
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

    pub fn as_hashmap(&self) -> HashMap<String, f32> {
        let mut hm = HashMap::new();
        for (mod_, mass) in self.static_mods.iter() {
            hm.insert(mod_.to_string(), *mass);
        }
        hm
    }
}

// ─── VariableModConfig ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableModConfig {
    pub variable_mods: StaticModConfig,
}

impl Default for VariableModConfig {
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert(ModificationSpecificity::Residue(b'M'), 15.994915f32);
        let mut ser = HashMap::new();
        ser.insert("M".to_string(), 15.994915f32);
        let def = StaticModConfig {
            static_mods: m,
            static_mods_ser: ser,
            new_mod_buffer: "M".to_string(),
            new_mass_buffer: 15.994915f32,
        };
        Self { variable_mods: def }
    }
}

impl VariableModConfig {
    pub fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Variable Modifications");
        self.variable_mods._update_section(ui);
    }

    pub fn as_hashmap(&self) -> HashMap<String, Vec<f32>> {
        let mut hm = HashMap::new();
        for (mod_, mass) in self.variable_mods.static_mods.iter() {
            hm.insert(mod_.to_string(), vec![*mass]);
        }
        hm
    }
}

// ─── DatabaseConfig ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub bucket_size: usize,
    pub enzyme: EnzymeConfig,
    pub peptide_min_mass: f32,
    pub peptide_max_mass: f32,
    pub ion_kinds: IonKindSelection,
    pub min_ion_index: u32,
    pub max_variable_mods: u32,
    pub decoy_tag: Option<String>,
    pub generate_decoys: bool,
    pub static_mods: StaticModConfig,
    pub variable_mods: VariableModConfig,
    pub fasta: String,
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

// ─── ToleranceConfig ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ToleranceConfig {
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
    pub fn update_section(&mut self, ui: &mut egui::Ui) {
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

impl From<ToleranceConfig> for Tolerance {
    fn from(val: ToleranceConfig) -> Self {
        match val {
            ToleranceConfig::Ppm(a, b) => Tolerance::Ppm(a, b),
            ToleranceConfig::Da(a, b) => Tolerance::Da(a, b),
        }
    }
}

// ─── IsobarSelection ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IsobarSelection {
    pub selected: Isobaric,
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
    pub fn update_section(&mut self, ui: &mut egui::Ui) {
        ui.radio_value(&mut self.selected, Isobaric::Tmt6, "TMT 6-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt10, "TMT 10-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt11, "TMT 11-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt16, "TMT 16-plex");
        ui.radio_value(&mut self.selected, Isobaric::Tmt18, "TMT 18-plex");
    }
}

// ─── SupportedQuantTypes / QuantType ─────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Copy)]
pub enum SupportedQuantTypes {
    #[serde(rename = "lfq")]
    Lfq,
    #[serde(rename = "tmt")]
    Tmt,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QuantType {
    Lfq(LfqSettings),
    // TmtSettings has no Deserialize in the dependency; store the isobar and
    // the two scalar fields that TmtSettings wraps so we can round-trip them.
    Tmt(IsobarSelection, TmtSettingsSer),
}

/// Serialisable mirror of TmtSettings (which only derives Serialize in the dep).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmtSettingsSer {
    pub level: u8,
    pub sn: bool,
}

impl Default for TmtSettingsSer {
    fn default() -> Self {
        let d = TmtSettings::default();
        Self {
            level: d.level,
            sn: d.sn,
        }
    }
}

impl From<TmtSettingsSer> for TmtSettings {
    fn from(v: TmtSettingsSer) -> Self {
        TmtSettings {
            level: v.level,
            sn: v.sn,
        }
    }
}

impl QuantType {
    pub fn update_section(&mut self, ui: &mut egui::Ui) {
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

    pub fn type_default(supported: SupportedQuantTypes) -> Self {
        match supported {
            SupportedQuantTypes::Lfq => Self::Lfq(LfqSettings::default()),
            SupportedQuantTypes::Tmt => {
                Self::Tmt(IsobarSelection::default(), TmtSettingsSer::default())
            }
        }
    }
}

impl From<QuantType> for QuantOptions {
    fn from(val: QuantType) -> Self {
        match val {
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
            QuantType::Tmt(isobar, tmt_ser) => {
                let tmt: TmtSettings = tmt_ser.into();
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

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub precursor_tol: ToleranceConfig,
    pub fragment_tol: ToleranceConfig,
    pub precursor_charge: (u8, u8),
    pub isotope_errors: (i8, i8),
    pub deisotope: bool,
    pub chimera: bool,
    pub wide_window: bool,
    pub predict_rt: bool,
    pub min_peaks: u32,
    pub max_peaks: u32,
    pub min_matched_peaks: u16,
    pub max_fragment_charge: u8,
    pub report_psms: usize,
    pub mzml_paths: Vec<PathBuf>,
    pub quant: QuantType,
    pub quant_enabled: bool,
    pub quant_class: SupportedQuantTypes,
    pub annotate_matches: bool,
    pub write_pin: bool,
    pub score_type: ScoreType,
    pub output_directory: String,
    pub override_precursor_charge: bool,
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
            quant_enabled: true,
            quant: QuantType::default(),
            quant_class: SupportedQuantTypes::Lfq,
            annotate_matches: false,
            write_pin: false,
            score_type: ScoreType::SageHyperScore,
            output_directory: cwd_str.unwrap_or_else(|| "output".to_string()),
            override_precursor_charge: false,
        }
    }
}

// ─── Per-page render methods on SageLauncher ─────────────────────────────────

impl SageLauncher {
    pub fn page_experiment(&mut self, ui: &mut egui::Ui) {
        ui.heading("Experiment");
        ui.add_space(10.0);
        ui.add(
            egui::Image::new(include_image!("../assets/sagegui_logo-removebg.png"))
                .max_width(400.0),
        );
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Experiment Type:");
            egui::ComboBox::from_id_salt("experiment_type")
                .selected_text(format!("{:?}", self.experiment))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.experiment, ExperimentType::Custom, "Custom");
                    ui.selectable_value(
                        &mut self.experiment,
                        ExperimentType::TrypticLfq,
                        "Tryptic LFQ",
                    );
                    ui.selectable_value(
                        &mut self.experiment,
                        ExperimentType::WideOpen,
                        "Wide Open",
                    );
                    ui.selectable_value(&mut self.experiment, ExperimentType::Phospho, "Phospho");
                    ui.selectable_value(
                        &mut self.experiment,
                        ExperimentType::SemiTryptic,
                        "Semi-Tryptic",
                    );
                });
        });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Save Config…").clicked() {
                if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).save_file() {
                    let mut cfg = self.config.clone();
                    // sync string maps before serialising
                    cfg.database.static_mods.sync_to_ser();
                    cfg.database.variable_mods.variable_mods.sync_to_ser();
                    match serde_json::to_string_pretty(&cfg) {
                        Ok(s) => {
                            if let Err(e) = std::fs::write(&path, s) {
                                self.status_message = format!("Error saving: {}", e);
                            } else {
                                self.status_message = "Config saved.".to_string();
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Error serialising: {}", e);
                        }
                    }
                }
            }

            if ui.button("Load Config…").clicked() {
                if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        match serde_json::from_str::<Config>(&s) {
                            Ok(mut c) => {
                                // re-hydrate live mod maps from the string maps
                                c.database.static_mods.sync_from_ser();
                                c.database.variable_mods.variable_mods.sync_from_ser();
                                self.config = c;
                                self.status_message = "Config loaded.".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Error loading: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn page_files_database(&mut self, ui: &mut egui::Ui) {
        ui.heading("Files & Database");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Data");

            // Output location
            ui.horizontal(|ui| {
                ui.label("Output Location:");
                ui.text_edit_singleline(&mut self.config.output_directory);
                if ui.button("Browse").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.config.output_directory = path.display().to_string();
                    }
                }
            });

            // mzML file picker
            ui.horizontal(|ui| {
                if ui.button("Pick mzML files").clicked() {
                    if let Some(paths) = FileDialog::new()
                        .add_filter("mzML", &["mzML", "gz", "mzml"])
                        .pick_files()
                    {
                        self.config.mzml_paths = paths;
                    } else {
                        self.config.mzml_paths = Vec::new();
                    }
                }
            });

            // Picked files list
            ui.label("Picked Files:");
            ui.separator();
            for path in self.config.mzml_paths.iter() {
                ui.label(path.to_string_lossy());
            }
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Database");

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

            // cRAP placeholder
            let mut _crap = false;
            ui.add_enabled(
                false,
                egui::Checkbox::new(&mut _crap, "Include cRAP contaminants (coming soon)"),
            );

            egui::CollapsingHeader::new("Advanced")
                .default_open(false)
                .show(ui, |ui| {
                    ui.checkbox(&mut self.config.database.generate_decoys, "Generate Decoys")
                        .on_hover_text("Auto-generate reversed decoys for FDR estimation.");

                    ui.add(
                        egui::Slider::new(&mut self.config.database.bucket_size, 8192..=65536)
                            .text("Bucket Size"),
                    )
                    .on_hover_text(
                        "Speed only, no effect on results. 8192 for high-res (Orbitrap), \
                         up to 65536 for low-res (ion trap).",
                    );

                    ui.add(
                        egui::DragValue::new(&mut self.config.database.min_ion_index)
                            .prefix("min_ion_index: "),
                    );

                    // decoy_tag (Option<String>)
                    let mut tag = self.config.database.decoy_tag.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("Decoy Tag:");
                        if ui
                            .add(egui::TextEdit::singleline(&mut tag).desired_width(80.0))
                            .changed()
                        {
                            self.config.database.decoy_tag =
                                if tag.is_empty() { None } else { Some(tag) };
                        }
                    });
                });
        });
    }

    pub fn page_search(&mut self, ui: &mut egui::Ui) {
        ui.heading("Search");
        ui.add_space(10.0);

        // Tolerances
        self.update_tolerances(ui);

        ui.add_space(10.0);

        // Charge Handling
        ui.group(|ui| {
            ui.heading("Charge Handling");
            ui.horizontal(|ui| {
                ui.label("Precursor Charge Min:");
                ui.add(egui::DragValue::new(&mut self.config.precursor_charge.0).range(1..=10));
                ui.label("Max:");
                ui.add(egui::DragValue::new(&mut self.config.precursor_charge.1).range(1..=10));
            });
        });

        ui.add_space(10.0);

        // Enzyme Settings
        ui.group(|ui| {
            self.config.database.enzyme.update_section(ui);
        });

        ui.add_space(10.0);

        // Mass Ranges
        ui.group(|ui| {
            ui.heading("Mass Ranges");
            ui.add(
                egui::Slider::new(&mut self.config.database.peptide_min_mass, 300.0..=1000.0)
                    .text("Peptide Min Mass"),
            );
            ui.add(
                egui::Slider::new(&mut self.config.database.peptide_max_mass, 3000.0..=7000.0)
                    .text("Peptide Max Mass"),
            );
        });

        ui.add_space(10.0);

        // Ion Kinds
        ui.group(|ui| {
            ui.heading("Ion Kinds");
            self.config.database.ion_kinds.update_section(ui);
        });

        ui.add_space(10.0);

        // Search Behavior
        ui.group(|ui| {
            ui.heading("Search Behavior");
            ui.horizontal(|ui| {
                ui.label("Isotope Errors Min:")
                    .on_hover_text("C13 isotope-error offsets. Slower than simply widening precursor tolerance to cover the same mass range — prefer a wider Da window when unsure.");
                ui.add(egui::DragValue::new(&mut self.config.isotope_errors.0).range(-5..=0));
                ui.label("Max:");
                ui.add(egui::DragValue::new(&mut self.config.isotope_errors.1).range(0..=10));
            });
            ui.checkbox(&mut self.config.deisotope, "Deisotope");
            ui.checkbox(&mut self.config.chimera, "Chimera");
            ui.checkbox(&mut self.config.wide_window, "Wide Window");
        });

        ui.add_space(10.0);

        // Scoring
        ui.group(|ui| {
            ui.heading("Scoring");
            ui.horizontal(|ui| {
                ui.label("Score Type:").on_hover_text(
                    "Scoring function. Leave at SageHyperScore unless comparing scoring functions.",
                );
                egui::ComboBox::from_id_salt("score_type")
                    .selected_text(format!("{:?}", self.config.score_type))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                matches!(self.config.score_type, ScoreType::SageHyperScore),
                                "SageHyperScore",
                            )
                            .clicked()
                        {
                            self.config.score_type = ScoreType::SageHyperScore;
                        }
                        if ui
                            .selectable_label(
                                matches!(self.config.score_type, ScoreType::OpenMSHyperScore),
                                "OpenMSHyperScore",
                            )
                            .clicked()
                        {
                            self.config.score_type = ScoreType::OpenMSHyperScore;
                        }
                    });
            });
        });

        ui.add_space(10.0);

        // Advanced
        egui::CollapsingHeader::new("Advanced")
            .default_open(false)
            .show(ui, |ui| {
                ui.checkbox(
                    &mut self.config.override_precursor_charge,
                    "Override Precursor Charge",
                )
                .on_hover_text("Force the charge range to be searched instead of trusting the file's charge annotation (useful for DIA/diaPASEF).");
                ui.add(
                    egui::Slider::new(&mut self.config.min_peaks, 5..=50).text("Min Peaks"),
                );
                ui.add(
                    egui::Slider::new(&mut self.config.max_peaks, 50..=500).text("Max Peaks"),
                );
                ui.add(
                    egui::Slider::new(&mut self.config.min_matched_peaks, 3..=20)
                        .text("Min Matched Peaks"),
                );
                ui.add(
                    egui::Slider::new(&mut self.config.max_fragment_charge, 1..=5)
                        .text("Max Fragment Charge"),
                );
                ui.add(
                    egui::Slider::new(&mut self.config.report_psms, 1..=10).text("Report PSMs"),
                );
                ui.checkbox(&mut self.config.predict_rt, "Predict RT");
            });
    }

    pub fn page_modifications(&mut self, ui: &mut egui::Ui) {
        ui.heading("Modifications");
        ui.add_space(10.0);
        self.config.database.static_mods.update_section(ui);
        ui.add_space(10.0);
        self.config.database.variable_mods.update_section(ui);
        ui.add_space(10.0);
        ui.add(
            egui::Slider::new(&mut self.config.database.max_variable_mods, 1..=10)
                .text("Max Variable Mods"),
        );
    }

    pub fn page_quant(&mut self, ui: &mut egui::Ui) {
        ui.heading("Quantification");
        ui.add_space(10.0);
        self.update_quant_options(ui);
    }

    pub fn page_run_info(&mut self, ui: &mut egui::Ui) {
        ui.heading("Run / Info");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Output Options");
            ui.checkbox(&mut self.config.write_pin, "Write PIN file")
                .on_hover_text("Write a Percolator .pin file for downstream rescoring.");
            ui.checkbox(&mut self.config.annotate_matches, "Annotate Matches")
                .on_hover_text("Write annotated fragment-ion match detail alongside results.");
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Info / Help");
            ui.label("Sage GUI Version:");
            ui.label(env!("CARGO_PKG_VERSION"));
            ui.label(format!(
                "Sage Engine Version: {}",
                crate::version::SAGE_VERSION
            ));
            ui.add_space(10.0);
            ui.label("Original Author: J.Sebastian Paez");
            ui.label("Current Maintainer: neely");
            ui.label("Repository: https://github.com/neely/sagegui");
            ui.label("License: Apache-2.0");
            ui.add_space(20.0);
            ui.label("Search engine repository: https://github.com/lazear/sage");
            ui.label(
                "If you use Sage in a scientific publication, please cite the following paper: \
                 'Sage: An Open-Source Tool for Fast Proteomics Searching and Quantification at \
                 Scale' https://doi.org/10.1021/acs.jproteome.3c00486",
            );
        });
    }

    // ── Shared helpers ──────────────────────────────────────────────────────

    pub fn update_tolerances(&mut self, ui: &mut egui::Ui) {
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

    pub fn update_quant_options(&mut self, ui: &mut egui::Ui) {
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
