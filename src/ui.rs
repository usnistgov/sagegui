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

// ─── Modification presets (curated "Common modifications" master list) ────────
//
// A small set we maintain ourselves — Sage ships no mod dictionary. Each preset
// carries one or more (Sage-key, mass) pairs; a multi-residue preset (e.g.
// Phospho on S/T/Y) inserts several independent rows in one click. Masses are
// Unimod monoisotopic deltas. Which box (Static/Variable) an entry lands in is
// chosen by the user via the focus toggle, not fixed per preset — `typical`
// only records the conventional use for the tooltip.
pub struct ModPreset {
    /// Display name, e.g. "Phospho (S/T/Y)".
    pub label: &'static str,
    /// (Sage specificity key, monoisotopic delta) pairs applied together.
    pub keys: &'static [(&'static str, f32)],
    /// Unimod accession, for the hover tooltip.
    pub accession: u32,
    /// Short note shown on hover.
    pub note: &'static str,
}

/// Curated common-modifications list. Edit here to add/remove presets.
/// Kept in alphabetical order by `label`.
pub const MOD_PRESETS: &[ModPreset] = &[
    ModPreset {
        label: "Acetyl (K, protein N-term)",
        keys: &[("K", 42.010565), ("[", 42.010565)],
        accession: 1,
        note: "Lysine acetylation and/or protein N-term acetylation.",
    },
    ModPreset {
        label: "Carbamidomethyl (C)",
        keys: &[("C", 57.021464)],
        accession: 4,
        note: "Iodoacetamide alkylation of cysteine; standard fixed mod.",
    },
    ModPreset {
        label: "Carbamyl (K, protein N-term)",
        keys: &[("K", 43.005814), ("[", 43.005814)],
        accession: 5,
        note: "Urea/cyanate artefact; common in old/frozen samples.",
    },
    ModPreset {
        label: "Deamidated (N/Q)",
        keys: &[("N", 0.984016), ("Q", 0.984016)],
        accession: 7,
        note: "Common artefact/PTM; N more common than Q.",
    },
    ModPreset {
        label: "Glu->pyro-Glu (E, peptide N-term)",
        keys: &[("^E", -18.010565)],
        accession: 27,
        note: "Peptide N-term E; negative delta mass.",
    },
    ModPreset {
        label: "Gln->pyro-Glu (Q, peptide N-term)",
        keys: &[("^Q", -17.026549)],
        accession: 28,
        note: "Peptide N-term Q; negative delta mass.",
    },
    ModPreset {
        label: "Methyl (K/R)",
        keys: &[("K", 14.01565), ("R", 14.01565)],
        accession: 34,
        note: "Mono-methylation.",
    },
    ModPreset {
        label: "Oxidation (M)",
        keys: &[("M", 15.994915)],
        accession: 35,
        note: "Methionine oxidation; the default variable mod.",
    },
    ModPreset {
        label: "Oxidation (P)",
        keys: &[("P", 15.994915)],
        accession: 35,
        note: "Hydroxyproline; add for collagen/ECM-heavy samples (on top of Ox M).",
    },
    ModPreset {
        label: "Phospho (S/T/Y)",
        keys: &[("S", 79.96633), ("T", 79.96633), ("Y", 79.96633)],
        accession: 21,
        note: "Phosphoproteomics; typically replaces oxidation, not additive.",
    },
    ModPreset {
        label: "Trimethyl (K/R)",
        keys: &[("K", 42.04695), ("R", 42.04695)],
        accession: 37,
        note: "Distinct from Acetyl (42.010565) — do not conflate.",
    },
];

/// Which modification box the master-list arrows act on.
#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum ModTarget {
    #[default]
    Variable,
    Static,
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
        }
    }
}

impl StaticModConfig {
    pub fn as_hashmap(&self) -> HashMap<String, f32> {
        let mut hm = HashMap::new();
        for (mod_, mass) in self.static_mods.iter() {
            hm.insert(mod_.to_string(), *mass);
        }
        hm
    }

    /// Insert a mod by its Sage key string (e.g. "M", "^Q", "["). No-op on an
    /// unparseable key. Keeps the serde shadow map in sync.
    pub fn insert_key(&mut self, key: &str, mass: f32) {
        if let Ok(spec) = ModificationSpecificity::from_str(key) {
            self.static_mods.insert(spec, mass);
            self.sync_to_ser();
        }
    }

    /// Remove a mod by its Sage key string. Keeps the serde shadow map in sync.
    pub fn remove_key(&mut self, key: &str) {
        if let Ok(spec) = ModificationSpecificity::from_str(key) {
            self.static_mods.remove(&spec);
            self.sync_to_ser();
        }
    }

    /// Render this box as a read-only list with a Remove button per row.
    /// Returns the key strings the user asked to remove this frame.
    pub fn show_list(&self, ui: &mut egui::Ui) -> Vec<String> {
        let mut to_remove = Vec::new();
        ui.group(|ui| {
            ui.set_min_width(200.0);
            if self.static_mods.is_empty() {
                ui.weak("(none)");
            }
            for (mod_, mass) in self.static_mods.iter() {
                ui.horizontal(|ui| {
                    ui.monospace(format!("{:<3} {:+.4}", mod_.to_string(), mass))
                        .on_hover_text(format!("Exact stored Δmass: {:+}", mass));
                    if ui.small_button("✖").on_hover_text("Remove").clicked() {
                        to_remove.push(mod_.to_string());
                    }
                });
            }
        });
        to_remove
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
        };
        Self { variable_mods: def }
    }
}

impl VariableModConfig {
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
    /// List of FASTA files to search (concatenated at launch time).
    #[serde(default)]
    pub fasta_paths: Vec<PathBuf>,
    /// Legacy single-FASTA field — migrated to fasta_paths on deserialise.
    #[serde(default, skip_serializing)]
    pub fasta: String,
    /// Resolved path of the concatenated FASTA written at launch; not persisted.
    #[serde(skip)]
    pub fasta_for_launch: String,
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
            fasta: Some(val.fasta_for_launch),
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
            fasta_paths: Vec::new(),
            fasta: String::new(),
            fasta_for_launch: String::new(),
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

        ui.label("Save / Load Config: coming in a future version.");
    }

    pub fn page_files_database(&mut self, ui: &mut egui::Ui) {
        ui.heading("Files & Database");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Data");

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

            // Multi-FASTA list
            ui.horizontal(|ui| {
                if ui
                    .button("Add FASTA…")
                    .on_hover_text("Add one or more FASTA files (target, contaminants, spike-ins).")
                    .clicked()
                {
                    if let Some(paths) = FileDialog::new()
                        .add_filter("FASTA", &["fasta", "fa", "faa"])
                        .pick_files()
                    {
                        for p in paths {
                            if !self.config.database.fasta_paths.contains(&p) {
                                self.config.database.fasta_paths.push(p);
                            }
                        }
                    }
                }
            });

            // FASTA list with per-row remove
            let mut remove_idx: Option<usize> = None;
            if self.config.database.fasta_paths.is_empty() {
                ui.weak("(no FASTA files selected)");
            } else {
                for (i, path) in self.config.database.fasta_paths.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string()),
                        )
                        .on_hover_text(path.to_string_lossy());
                        if ui.small_button("✖").on_hover_text("Remove").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
            }
            if let Some(i) = remove_idx {
                self.config.database.fasta_paths.remove(i);
            }

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
        ui.add_space(6.0);
        ui.label(
            "Pick a target box, then use ◀ / ▶ to move a common modification in or out. \
             A modification cannot be both Static and Variable at once.",
        );
        ui.add_space(10.0);

        // Which box do the arrows act on?
        ui.horizontal(|ui| {
            ui.label("Target:");
            ui.selectable_value(&mut self.mod_target, ModTarget::Variable, "Variable")
                .on_hover_text("Arrows add/remove modifications in the Variable box.");
            ui.selectable_value(&mut self.mod_target, ModTarget::Static, "Static (fixed)")
                .on_hover_text("Arrows add/remove modifications in the Static box.");
        });
        ui.add_space(8.0);

        // Deferred mutations: collect during the immediate-mode pass, apply after.
        let mut add_keys: Vec<(&'static str, f32)> = Vec::new();
        let mut remove_keys: Vec<String> = Vec::new();

        ui.horizontal_top(|ui| {
            // ── Left: the two destination boxes ──────────────────────────────
            ui.vertical(|ui| {
                ui.strong("Static (fixed)");
                let stat_rm = self.config.database.static_mods.show_list(ui);
                remove_keys.extend(stat_rm.into_iter().map(|k| format!("S\u{1}{}", k)));

                ui.add_space(10.0);

                ui.strong("Variable");
                let var_rm = self
                    .config
                    .database
                    .variable_mods
                    .variable_mods
                    .show_list(ui);
                remove_keys.extend(var_rm.into_iter().map(|k| format!("V\u{1}{}", k)));
            });

            ui.add_space(12.0);

            // ── Middle: transfer arrows ──────────────────────────────────────
            ui.vertical(|ui| {
                ui.add_space(24.0);
                let has_sel = self.mod_selected_preset.is_some();
                if ui
                    .add_enabled(has_sel, egui::Button::new("◀ Add"))
                    .on_hover_text("Add the selected common modification to the target box.")
                    .clicked()
                {
                    if let Some(i) = self.mod_selected_preset {
                        for (key, mass) in MOD_PRESETS[i].keys {
                            add_keys.push((key, *mass));
                        }
                    }
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Remove ▶"))
                    .on_hover_text("Remove the selected common modification from the target box.")
                    .clicked()
                {
                    if let Some(i) = self.mod_selected_preset {
                        for (key, _) in MOD_PRESETS[i].keys {
                            remove_keys.push(match self.mod_target {
                                ModTarget::Static => format!("S\u{1}{}", key),
                                ModTarget::Variable => format!("V\u{1}{}", key),
                            });
                        }
                    }
                }
            });

            ui.add_space(12.0);

            // ── Right: the curated master list ───────────────────────────────
            ui.vertical(|ui| {
                ui.strong("Common modifications");
                ui.group(|ui| {
                    ui.set_min_width(230.0);
                    for (i, preset) in MOD_PRESETS.iter().enumerate() {
                        let selected = self.mod_selected_preset == Some(i);
                        if ui
                            .selectable_label(selected, preset.label)
                            .on_hover_text(format!(
                                "{}\nUnimod accession {}",
                                preset.note, preset.accession
                            ))
                            .clicked()
                        {
                            self.mod_selected_preset = Some(i);
                        }
                    }
                });

                ui.add_space(6.0);

                // ── Custom escape hatch ──────────────────────────────────────
                ui.collapsing("+ Custom…", |ui| {
                    ui.label("Enter a Sage specificity key and a delta mass (Da).");
                    ui.label("Sage key syntax:");
                    egui::Grid::new("custom_mod_key_hints")
                        .num_columns(2)
                        .spacing([12.0, 2.0])
                        .show(ui, |ui| {
                            ui.monospace("X");
                            ui.label("anywhere on residue X (e.g. M, C)");
                            ui.end_row();
                            ui.monospace("^X");
                            ui.label("residue X at peptide N-terminus (e.g. ^Q)");
                            ui.end_row();
                            ui.monospace("$X");
                            ui.label("residue X at peptide C-terminus");
                            ui.end_row();
                            ui.monospace("[  /  [X");
                            ui.label("protein N-terminus (any / residue X)");
                            ui.end_row();
                            ui.monospace("]  /  ]X");
                            ui.label("protein C-terminus (any / residue X)");
                            ui.end_row();
                        });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Key:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mod_custom_key)
                                .desired_width(40.0)
                                .hint_text("^Q"),
                        )
                        .on_hover_text("Sage specificity, e.g. C, M, ^Q, $K, [, ].");
                        ui.label("Δmass:");
                        ui.add(egui::DragValue::new(&mut self.mod_custom_mass).speed(0.001));
                    });
                    let parsed =
                        ModificationSpecificity::from_str(self.mod_custom_key.trim()).is_ok();
                    if !self.mod_custom_key.trim().is_empty() && !parsed {
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 80, 80),
                            "Invalid key (valid: C, ], $, ^M …).",
                        );
                    }
                    if ui
                        .add_enabled(parsed, egui::Button::new("Add to target box"))
                        .clicked()
                    {
                        // Leak-free: custom keys go through the owned-string path.
                        let key = self.mod_custom_key.trim().to_string();
                        let mass = self.mod_custom_mass;
                        match self.mod_target {
                            ModTarget::Static => {
                                self.config
                                    .database
                                    .variable_mods
                                    .variable_mods
                                    .remove_key(&key);
                                self.config.database.static_mods.insert_key(&key, mass);
                            }
                            ModTarget::Variable => {
                                self.config.database.static_mods.remove_key(&key);
                                self.config
                                    .database
                                    .variable_mods
                                    .variable_mods
                                    .insert_key(&key, mass);
                            }
                        }
                    }
                });
            });
        });

        // ── Apply deferred removals ───────────────────────────────────────────
        for tagged in &remove_keys {
            if let Some(key) = tagged.strip_prefix("S\u{1}") {
                self.config.database.static_mods.remove_key(key);
            } else if let Some(key) = tagged.strip_prefix("V\u{1}") {
                self.config
                    .database
                    .variable_mods
                    .variable_mods
                    .remove_key(key);
            }
        }

        // ── Apply deferred adds with mutual exclusion ─────────────────────────
        for (key, mass) in add_keys {
            match self.mod_target {
                ModTarget::Static => {
                    self.config
                        .database
                        .variable_mods
                        .variable_mods
                        .remove_key(key);
                    self.config.database.static_mods.insert_key(key, mass);
                }
                ModTarget::Variable => {
                    self.config.database.static_mods.remove_key(key);
                    self.config
                        .database
                        .variable_mods
                        .variable_mods
                        .insert_key(key, mass);
                }
            }
        }

        ui.add_space(14.0);
        ui.add(
            egui::Slider::new(&mut self.config.database.max_variable_mods, 1..=10)
                .text("Max Variable Mods"),
        )
        .on_hover_text("Caps how many variable mods can co-occur on one peptide (Sage default 2).");

        ui.add_space(10.0);
        ui.separator();
        ui.weak(
            "Note: delta masses are displayed rounded to 4 decimal places, but the \
             full 5–6 decimal monoisotopic value is stored and used in the search.",
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
            ui.heading("Output Location");
            ui.horizontal(|ui| {
                ui.label("Output Location:");
                ui.text_edit_singleline(&mut self.config.output_directory);
                if ui.button("Browse").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.config.output_directory = path.display().to_string();
                    }
                }
            });
        });

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
