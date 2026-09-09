// Derived from jspaezp/sagegui (Apache 2.0, J. Sebastian Paez).
// Modified by Benjamin A. Neely (NIST) on 2026-08-13: extracted from main.rs into
// this module, redesigned into 6-tab sidebar layout, rebuilt Modifications tab as
// curated list-picker, added multi-FASTA UI, surfaced hidden Sage parameters.
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
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;

use crate::SageLauncher;

// ─── Theme ───────────────────────────────────────────────────────────────────

/// Replacement for egui's stock dark palette, installed only into the
/// `Theme::Dark` slot (see `SageLauncher::new`) — light mode is left at
/// egui's own defaults, since egui already follows the OS and there's no
/// reason to touch the half of the palette that isn't the reported problem.
///
/// Two things drove this: contrast, and giving the app a look of its own
/// instead of default-egui gray. On contrast — the stock theme's body/button
/// text (gray 140/180) is already faint against its near-black panels (gray
/// 27), and disabled controls (`add_enabled(false, ..)`, e.g. Run/Stop and
/// the prefilter fields) fade *halfway towards that same near-black panel*.
/// Starting from low contrast, the faded result is close to unreadable —
/// widening the base gap fixes both the normal and disabled states. For the
/// look, everything leans on one muted sage-green accent (a nod to the Sage
/// search engine this wraps) instead of egui's default blue, and true black
/// is avoided in favor of a warm, slightly-green-tinted charcoal that's
/// easier to stare at for a long search run.
pub fn dark_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();

    let bg = egui::Color32::from_rgb(20, 22, 21); // panel / window: warm charcoal
    let surface = egui::Color32::from_rgb(38, 43, 40); // buttons, combo boxes, checkboxes
    let surface_hovered = egui::Color32::from_rgb(53, 60, 56);
    let surface_active = egui::Color32::from_rgb(31, 36, 33);
    let text = egui::Color32::from_rgb(232, 230, 224); // warm off-white, not stark white
    let border = egui::Color32::from_rgb(66, 73, 68);
    let sage = egui::Color32::from_rgb(120, 178, 148); // accent: hyperlinks, selection, focus
    let sage_bright = egui::Color32::from_rgb(160, 214, 188);

    visuals.panel_fill = bg;
    visuals.window_fill = bg;
    visuals.window_stroke.color = border;
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 13, 12); // text edit fields
    visuals.code_bg_color = egui::Color32::from_rgb(56, 60, 56);
    visuals.hyperlink_color = sage_bright;
    visuals.selection.bg_fill = egui::Color32::from_rgb(40, 66, 52);
    visuals.selection.stroke.color = sage_bright;
    visuals.warn_fg_color = egui::Color32::from_rgb(0xE0, 0x8A, 0x00); // matches ToleranceConfig's warning
    visuals.error_fg_color = egui::Color32::from_rgb(210, 90, 90); // matches the app's error label

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.weak_bg_fill = bg;
    widgets.noninteractive.bg_fill = bg;
    widgets.noninteractive.bg_stroke.color = border;
    widgets.noninteractive.fg_stroke.color = text;

    widgets.inactive.weak_bg_fill = surface;
    widgets.inactive.bg_fill = surface;
    widgets.inactive.fg_stroke.color = text;

    widgets.hovered.weak_bg_fill = surface_hovered;
    widgets.hovered.bg_fill = surface_hovered;
    widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, sage);
    widgets.hovered.fg_stroke.color = egui::Color32::WHITE;

    widgets.active.weak_bg_fill = surface_active;
    widgets.active.bg_fill = surface_active;
    widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, sage_bright);
    widgets.active.fg_stroke.color = egui::Color32::WHITE;

    widgets.open.bg_fill = bg;
    widgets.open.bg_stroke.color = border;
    widgets.open.fg_stroke.color = text;

    visuals
}

/// Body/button text at egui's 12.5pt default is small on a high-DPI monitor.
/// Bumps every text style up while preserving their relative proportions.
/// Theme-agnostic (applied to both light and dark `Style`s), since this is a
/// legibility fix, not a look-and-feel one.
pub fn readable_text_styles() -> BTreeMap<egui::TextStyle, egui::FontId> {
    use egui::FontFamily::{Monospace, Proportional};
    use egui::{FontId, TextStyle};

    [
        (TextStyle::Small, FontId::new(11.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Heading, FontId::new(20.0, Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, Monospace)),
    ]
    .into()
}

// ─── Page enum ───────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
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
    /// The restriction Sage will actually apply.
    ///
    /// `enable_restrict` and `restrict_char` disagree routinely. Importing a
    /// Sage config with `"restrict": null` clears the flag but leaves the
    /// character behind (see `sage_json`), so reading the raw field would
    /// report a restriction that is not in force. Everything that needs to
    /// know the real rule goes through here.
    pub fn effective_restrict(&self) -> &str {
        if self.enable_restrict && self.restrict_char.chars().count() == 1 {
            &self.restrict_char
        } else {
            ""
        }
    }

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
        let restrict = val.effective_restrict().to_string();
        EnzymeBuilder {
            missed_cleavages: Some(val.missed_cleavages),
            min_len: Some(val.min_len),
            max_len: Some(val.max_len),
            cleave_at: Some(val.cleave_at),
            restrict: if restrict.is_empty() {
                None
            } else {
                Some(restrict)
            },
            c_terminal: Some(val.c_terminal),
            semi_enzymatic: Some(val.semi_enzymatic),
        }
    }
}

/// Residues Sage will accept in an enzyme rule. Read from `VALID_AA` in the
/// pinned Sage source (`crates/sage/src/enzyme.rs`): the 20 standard residues
/// plus U and O. Not B, Z, J or X.
const SAGE_VALID_AA: &str = "ACDEFGHIKLMNPQRSTVWYUO";

/// Reject what Sage's `Enzyme::new` would `assert!` on.
///
/// This is a hang guard, not a style check. Sage is compiled in, so the assert
/// fires on the run thread inside `input.build()`. The GUI cannot see that as a
/// channel disconnect (see the `catch_unwind` note in `src/main.rs`), and
/// before that guard existed a single stray character left the run bar spinning
/// forever with no message.
///
/// Two inputs must pass, or valid configurations break: an empty `cleave_at`
/// means non-specific digestion, and `"$"` means no digestion. Sage allows both
/// explicitly.
pub fn validate_enzyme_residues(enzyme: &EnzymeConfig) -> Result<(), String> {
    let check = |field: &str, value: &str| -> Result<(), String> {
        match value.chars().find(|c| !SAGE_VALID_AA.contains(*c)) {
            None => Ok(()),
            Some(bad) => Err(format!(
                "Enzyme {field} contains '{bad}', which is not an amino acid Sage accepts. \
                 Use only {SAGE_VALID_AA}, in capitals."
            )),
        }
    };

    if enzyme.cleave_at != "$" {
        check("Cleave At", &enzyme.cleave_at)?;
    }
    check("Restrict", enzyme.effective_restrict())
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
        note: "Distinct from Acetyl (42.010565). Do not conflate.",
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
    /// Enable Sage's chunked database prefiltering pass (lower peak memory).
    #[serde(default)]
    pub prefilter: bool,
    /// FASTA sequences per prefilter chunk. 0 = let Sage auto-calculate.
    #[serde(default)]
    pub prefilter_chunk_size: usize,
    /// Aggressive prefilter mode — keeps only report_psms + 1 hits per chunk.
    #[serde(default = "default_prefilter_low_memory")]
    pub prefilter_low_memory: bool,
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

fn default_prefilter_low_memory() -> bool {
    true
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
            prefilter: Some(val.prefilter),
            prefilter_chunk_size: Some(val.prefilter_chunk_size),
            prefilter_low_memory: Some(val.prefilter_low_memory),
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
            prefilter: false,
            prefilter_chunk_size: 0,
            prefilter_low_memory: true,
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
    /// Convert the stored Sage pair into the delta-mass pair shown to the user.
    ///
    /// Sage applies the window to the *experimental* mass and looks for
    /// *theoretical* peptide masses inside it:
    ///   `theoretical in [experimental + lower, experimental + upper]`
    /// Delta mass is `experimental - theoretical`, so the two orientations are
    /// the negation of each other with the bounds swapped. See AGENTS.md.
    fn to_delta(raw_lower: f32, raw_upper: f32) -> (f32, f32) {
        (-raw_upper, -raw_lower)
    }

    /// Inverse of [`to_delta`]. Same operation, since negate-and-swap is its
    /// own inverse.
    fn from_delta(delta_lower: f32, delta_upper: f32) -> (f32, f32) {
        (-delta_upper, -delta_lower)
    }

    /// The window as the user sees it on screen: a delta-mass range.
    ///
    /// The widget renders exactly this, and the tests assert against exactly
    /// this. Keep it that way. If a test computed the flip itself, it would be
    /// checking its own arithmetic rather than what the GUI shows.
    pub fn displayed_delta(&self) -> (f32, f32) {
        let (lower, upper) = match *self {
            ToleranceConfig::Ppm(a, b) | ToleranceConfig::Da(a, b) => (a, b),
        };
        Self::to_delta(lower, upper)
    }

    pub fn update_section(&mut self, ui: &mut egui::Ui) {
        // The widget works in DELTA MASS, which is how a person states the
        // window: "find IDs carrying up to +500 Da". Sage's own config files
        // store the opposite orientation. Only the display is flipped. What is
        // stored in `Config`, and what is written to Sage, stays raw.
        let (unit, speed) = match *self {
            ToleranceConfig::Ppm(..) => ("ppm", 1.0),
            ToleranceConfig::Da(..) => ("Da", 0.01),
        };
        let (mut delta_lower, mut delta_upper) = self.displayed_delta();

        ui.horizontal(|ui| {
            ui.label("Delta mass from:");
            ui.add(egui::DragValue::new(&mut delta_lower).speed(speed))
                .on_hover_text(format!(
                    "Most negative delta mass to search, in {unit}. A modification that \
                     removes mass gives a negative delta."
                ));
            ui.label("to:");
            ui.add(egui::DragValue::new(&mut delta_upper).speed(speed))
                .on_hover_text(format!(
                    "Most positive delta mass to search, in {unit}. Set 500 to find IDs \
                     carrying a modification of up to +500 {unit}."
                ));
        });

        let (new_lower, new_upper) = Self::from_delta(delta_lower, delta_upper);
        match self {
            ToleranceConfig::Ppm(a, b) | ToleranceConfig::Da(a, b) => {
                *a = new_lower;
                *b = new_upper;
            }
        }

        Self::warn_if_inverted(ui, delta_lower, delta_upper);
    }

    /// Non-blocking hint if the window is inverted, which would make Sage
    /// compute an empty search range. The test is the same in either
    /// orientation, because negate-and-swap preserves the ordering.
    fn warn_if_inverted(ui: &mut egui::Ui, lower: f32, upper: f32) {
        if lower > upper {
            ui.colored_label(
                egui::Color32::from_rgb(0xE0, 0x8A, 0x00),
                "⚠ The first value is greater than the second. This is an empty window.",
            );
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
    /// Build a selection from a Sage `Isobaric` value read out of a config or
    /// results JSON. Returns `None` for `Isobaric::User(..)` — a custom
    /// reporter-ion list has no radio button here, and silently collapsing it
    /// to a named plex would change the quantification.
    pub fn from_isobaric(isobar: &Isobaric) -> Option<Self> {
        match isobar {
            Isobaric::User(_) => None,
            other => Some(Self {
                selected: other.clone(),
            }),
        }
    }

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

        self.templates_section(ui);
        ui.add_space(16.0);
        self.import_section(ui);
        ui.add_space(16.0);
        self.import_result_section(ui);
    }

    /// Bundled starting configurations. Replaces the old archetype dropdown,
    /// which only ever stored its own selection and changed nothing else.
    fn templates_section(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Templates");
            ui.label(
                "Starting configurations for common experiment types. Applying one replaces \
                 your search parameters.",
            );
            ui.weak("Your selected files and output folder are never changed by a template.");
            ui.add_space(6.0);

            if self.templates.is_empty() {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    "No bundled templates loaded. This is a build problem, not a settings one.",
                );
                return;
            }

            let selected = self.selected_template.min(self.templates.len() - 1);
            self.selected_template = selected;

            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("template_picker")
                    .width(260.0)
                    .selected_text(self.templates[selected].meta.name.clone())
                    .show_ui(ui, |ui| {
                        for (i, template) in self.templates.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_template,
                                i,
                                &template.meta.name,
                            );
                        }
                    });

                if ui
                    .button("Apply template")
                    .on_hover_text(
                        "Overwrite the current search parameters with this template's values.",
                    )
                    .clicked()
                {
                    let index = self.selected_template;
                    let report = self.templates[index].doc.apply(
                        &mut self.config,
                        &mut self.precursor_tolerance_type,
                        &mut self.fragment_tolerance_type,
                        self.templates[index].file,
                    );
                    self.last_import = Some(Ok(report));
                }
            });

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&self.templates[self.selected_template].meta.description)
                    .weak(),
            );
        });
    }

    /// Load parameters out of a Sage `config.json` or a past run's
    /// `results.json`. Import-only by design — see NOTES "UI-review feedback #1".
    fn import_section(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Load settings from a Sage file");
            ui.label(
                "Reuse the parameters of an earlier search: point at a config.json you gave the \
                 Sage command line, or the results.json written into a past run's output folder.",
            );
            ui.add_space(6.0);

            if ui
                .button("Load Sage config or results.json…")
                .on_hover_text(
                    "Reads search parameters only. File paths and the output folder in the \
                     file are ignored.",
                )
                .clicked()
            {
                if let Some(path) = FileDialog::new()
                    .add_filter("Sage JSON", &["json"])
                    .pick_file()
                {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    self.last_import = Some(match std::fs::read_to_string(&path) {
                        Err(e) => Err(format!("Could not read {name}: {e}")),
                        Ok(text) => match crate::sage_json::SageJson::from_str(&text) {
                            Err(e) => Err(format!("{name} is not a valid Sage JSON file: {e}")),
                            Ok(doc) => Ok(doc.apply(
                                &mut self.config,
                                &mut self.precursor_tolerance_type,
                                &mut self.fragment_tolerance_type,
                                &name,
                            )),
                        },
                    });
                }
            }
        });
    }

    /// Outcome of the last apply/import. Warnings are shown in full rather than
    /// counted — a parameter that silently failed to load is the failure mode
    /// worth being loud about.
    fn import_result_section(&mut self, ui: &mut egui::Ui) {
        match &self.last_import {
            None => {}
            Some(Err(message)) => {
                ui.colored_label(ui.visuals().error_fg_color, message);
            }
            Some(Ok(report)) => {
                ui.label(report.summary());
                if !report.warnings.is_empty() {
                    ui.add_space(4.0);
                    ui.group(|ui| {
                        for warning in &report.warnings {
                            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {warning}"));
                        }
                    });
                }
                if !report.needs_reselect.is_empty() {
                    ui.add_space(4.0);
                    ui.group(|ui| {
                        ui.strong("Not imported. Pick these again yourself");
                        ui.weak(
                            "The file records where its own data lived. Those paths are not \
                             applied, because they may be from another machine.",
                        );
                        ui.add_space(4.0);
                        for note in &report.needs_reselect {
                            ui.label(format!("• {note}"));
                        }
                    });
                }
            }
        }
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

            ui.separator();
            ui.strong("Database prefiltering (memory)");

            ui.checkbox(&mut self.config.database.prefilter, "Enable prefiltering")
                .on_hover_text(
                    "Digest the FASTA in chunks and keep only peptides that matched a \
                     spectrum, then search that reduced database. Cuts peak memory for \
                     semi-enzymatic or non-specific digests, many variable mods, or very \
                     large databases. Costs extra CPU. Off by default; with chunk size on \
                     auto, Sage skips the pass when the search space is small enough not \
                     to need it.",
                );

            if self.config.database.enzyme.semi_enzymatic && !self.config.database.prefilter {
                ui.label("ℹ Semi-enzymatic digestion is on. Prefiltering limits peak memory.");
            }

            let prefiltering_enabled = self.config.database.prefilter;
            ui.add_enabled_ui(prefiltering_enabled, |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.config.database.prefilter_chunk_size)
                        .prefix("Chunk size: ")
                        .speed(100.0)
                        .range(0..=10_000_000)
                        .custom_formatter(|n, _| {
                            if n <= 0.0 {
                                "auto".to_owned()
                            } else {
                                format!("{}", n as usize)
                            }
                        }),
                )
                .on_hover_text(
                    "FASTA sequences digested and scored per chunk. 0 = auto: Sage targets \
                     about 8.4 million peptides per chunk, and skips prefiltering entirely \
                     if the whole search space already fits. Smaller values use less memory \
                     but add chunks.",
                );

                ui.checkbox(
                    &mut self.config.database.prefilter_low_memory,
                    "Low-memory mode",
                )
                .on_hover_text(
                    "On (Sage's default): score every preliminary hit and keep only the \
                     best few per spectrum per chunk. Lowest memory, most CPU. Off: keep \
                     every preliminary hit unscored. More memory, less CPU, and closer \
                     to a non-prefiltered search's FDR behaviour.",
                );
            });

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
                    .on_hover_text("C13 isotope-error offsets. Slower than simply widening precursor tolerance to cover the same mass range. Prefer a wider Da window when unsure.");
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
            ui.heading("Sage Log");
            ui.label("Live output from Sage's own search engine, captured during a run.");
            egui::ScrollArea::vertical()
                .id_salt("sage_log_scroll")
                .max_height(220.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if self.log_lines.is_empty() {
                        ui.weak("No output yet. Run a search to see live log output here.");
                    } else {
                        for line in &self.log_lines {
                            ui.monospace(line);
                        }
                    }
                });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Sage `assert!`s on non-residue characters, on the run thread. Before
    /// the guard, one stray keystroke in Cleave At hung the app with no
    /// message. Reject exactly what Sage rejects, and nothing more.
    #[test]
    fn the_validator_rejects_what_sage_asserts_on() {
        let mut e = EnzymeConfig::default();

        for bad in ["B", "Z", "J", "X", "kr", "K1", "K R"] {
            e.cleave_at = bad.to_string();
            assert!(
                validate_enzyme_residues(&e).is_err(),
                "{bad:?} should be rejected before it reaches Sage"
            );
        }

        for good in ["KR", "FYWL", "U", "O", "ACDEFGHIKLMNPQRSTVWYUO"] {
            e.cleave_at = good.to_string();
            assert!(
                validate_enzyme_residues(&e).is_ok(),
                "{good:?} is valid for Sage and must pass"
            );
        }
    }

    /// Two special forms Sage allows explicitly. Rejecting them would break
    /// valid configurations: empty means non-specific, "$" means no digestion.
    #[test]
    fn the_validator_allows_sages_two_special_cases() {
        let mut e = EnzymeConfig::default();
        e.cleave_at = String::new();
        assert!(validate_enzyme_residues(&e).is_ok(), "empty = non-specific");
        e.cleave_at = "$".to_string();
        assert!(validate_enzyme_residues(&e).is_ok(), "$ = no digestion");
    }

    /// A bad restrict character is just as fatal as a bad cleave residue.
    /// It is only checked when the restriction is actually in force.
    #[test]
    fn the_validator_checks_restrict_only_when_it_applies() {
        let mut e = EnzymeConfig::default();
        e.restrict_char = "B".to_string();

        e.enable_restrict = true;
        assert!(
            validate_enzyme_residues(&e).is_err(),
            "in force, so checked"
        );

        e.enable_restrict = false;
        assert!(
            validate_enzyme_residues(&e).is_ok(),
            "not in force, so it never reaches Sage"
        );
    }

    /// `effective_restrict` is the single source of truth for whether a
    /// restriction applies. Importing a Sage config with `"restrict": null`
    /// leaves a stale character behind with the flag cleared, and reading the
    /// raw field would report a restriction that is not in force.
    #[test]
    fn effective_restrict_ignores_a_stale_character() {
        let mut e = EnzymeConfig::default();
        assert_eq!(e.effective_restrict(), "P");

        e.enable_restrict = false;
        assert_eq!(e.effective_restrict(), "", "cleared flag wins");

        e.enable_restrict = true;
        e.restrict_char = "KR".to_string();
        assert_eq!(e.effective_restrict(), "", "Sage takes one character only");
    }

    /// The conversion Sage actually receives must agree with
    /// `effective_restrict`, not repeat the rule with its own copy.
    #[test]
    fn builder_restrict_agrees_with_effective_restrict() {
        for (enable, ch, want) in [
            (true, "P", Some("P".to_string())),
            (false, "P", None),
            (true, "", None),
            (true, "KR", None),
        ] {
            let mut e = EnzymeConfig::default();
            e.enable_restrict = enable;
            e.restrict_char = ch.to_string();
            let builder: EnzymeBuilder = e.clone().into();
            assert_eq!(
                builder.restrict, want,
                "enable={enable} char={ch:?} disagreed with effective_restrict"
            );
        }
    }

    /// The GUI shows the precursor window as a delta mass, which is how a
    /// person states it. Sage stores the opposite orientation. Getting this
    /// backwards has been the single most repeated mistake in this project,
    /// so both directions are pinned here. See AGENTS.md.
    #[test]
    fn delta_display_negates_and_swaps_the_stored_pair() {
        // Michael Lazear's wide-MS1 window: stored [-3.5, 1.25], and what the
        // user means by it is a delta mass of -1.25 to +3.5.
        assert_eq!(ToleranceConfig::to_delta(-3.5, 1.25), (-1.25, 3.5));
        // The open search: stored [-500, 100] means delta -100 to +500. A
        // +500 Da modification really is found by a -500 lower bound.
        assert_eq!(ToleranceConfig::to_delta(-500.0, 100.0), (-100.0, 500.0));
    }

    /// Editing a value in the widget must not drift it. The widget converts
    /// out and back on every frame, so a non-identity round trip would walk
    /// the user's numbers away from what they typed.
    #[test]
    fn delta_round_trip_is_the_identity() {
        for (lo, hi) in [
            (-3.5f32, 1.25f32),
            (-500.0, 100.0),
            (-10.0, 10.0),
            (-20.0, 20.0),
            (0.0, 0.0),
        ] {
            let (d_lo, d_hi) = ToleranceConfig::to_delta(lo, hi);
            assert_eq!(
                ToleranceConfig::from_delta(d_lo, d_hi),
                (lo, hi),
                "round trip drifted for [{lo}, {hi}]"
            );
        }
    }

    /// A symmetric window looks the same in either orientation, which is
    /// exactly why this bug hides: it only shows up on asymmetric windows.
    #[test]
    fn symmetric_windows_look_identical_in_both_orientations() {
        assert_eq!(ToleranceConfig::to_delta(-20.0, 20.0), (-20.0, 20.0));
    }

    /// The guard that matters. For every bundled template, run the real
    /// import path and assert the delta-mass window the widget will render,
    /// written here the way a person says it out loud.
    ///
    /// This is deliberately end to end. The unit tests above prove the
    /// arithmetic; this proves the whole chain, so it fails if anyone edits a
    /// template file's numbers, changes the importer, or "fixes" the flip
    /// direction. CI runs `cargo test`, so no build ships past a break here.
    ///
    /// If this test fails, do NOT flip the expected values to make it pass.
    /// Read the STOP section at the top of AGENTS.md first.
    #[test]
    fn every_bundled_template_shows_the_intended_delta_window() {
        // (file, precursor shown as delta, fragment shown as delta)
        let expected: &[(&str, (f32, f32), (f32, f32))] = &[
            // Wide MS1, tight MS2. Stored [-3.5, 1.25]. The +3.5 delta side is
            // what absorbs monoisotopic peak misassignment.
            ("tryptic-wide-ms1.json", (-1.25, 3.5), (-10.0, 10.0)),
            ("tryptic-tight.json", (-20.0, 20.0), (-20.0, 20.0)),
            // Open search. Stored [-500, 100]. A +500 Da modification is found
            // by the -500 lower bound, and must READ as +500 to the user.
            ("tryptic-open.json", (-100.0, 500.0), (-20.0, 20.0)),
            ("tryptic-biofluid.json", (-20.0, 20.0), (-20.0, 20.0)),
            ("tmt11.json", (-20.0, 20.0), (-0.4, 0.4)),
        ];

        let templates = crate::sage_json::bundled_templates();
        assert_eq!(
            templates.len(),
            expected.len(),
            "a template was added or removed without updating this test"
        );

        for (file, want_precursor, want_fragment) in expected {
            let template = templates
                .iter()
                .find(|t| t.file == *file)
                .unwrap_or_else(|| panic!("{file} is not in the bundled set"));

            let mut config = Config::default();
            let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
            template.doc.apply(&mut config, &mut p, &mut f, file);

            assert_eq!(
                config.precursor_tol.displayed_delta(),
                *want_precursor,
                "{file}: precursor window reads wrong. Stored {:?}",
                config.precursor_tol
            );
            assert_eq!(
                config.fragment_tol.displayed_delta(),
                *want_fragment,
                "{file}: fragment window reads wrong. Stored {:?}",
                config.fragment_tol
            );
        }
    }

    /// What the user types is what gets stored, flipped exactly once. Catches
    /// a double flip, which would look right on symmetric windows and be
    /// silently wrong on every asymmetric one.
    #[test]
    fn typing_a_delta_window_stores_the_matching_sage_pair() {
        // "I want to find modifications from -100 to +500 Da."
        let (stored_lower, stored_upper) = ToleranceConfig::from_delta(-100.0, 500.0);
        assert_eq!((stored_lower, stored_upper), (-500.0, 100.0));

        let tol = ToleranceConfig::Da(stored_lower, stored_upper);
        assert_eq!(
            tol.displayed_delta(),
            (-100.0, 500.0),
            "what Sage stores must read back as what the user asked for"
        );

        // And that stored pair is what Sage itself receives.
        match Tolerance::from(tol) {
            Tolerance::Da(lo, hi) => assert_eq!((lo, hi), (-500.0, 100.0)),
            other => panic!("expected a Da tolerance, got {other:?}"),
        }
    }

    /// An inverted window stays inverted in delta space, so the existing
    /// empty-window warning keeps working after the flip.
    #[test]
    fn inversion_is_preserved_by_the_flip() {
        let (lo, hi) = (5.0f32, -5.0f32); // stored inverted
        assert!(lo > hi);
        let (d_lo, d_hi) = ToleranceConfig::to_delta(lo, hi);
        assert!(d_lo > d_hi, "an empty window must still read as empty");
    }

    /// Simulates a config JSON saved by v0.7.0, before the prefilter fields
    /// existed: serializes the real default shape, then strips the three new
    /// keys out of the JSON `Value` (rather than hand-typing a fixture that
    /// could drift from the actual serde shape of nested types like
    /// `IonKindSelection` or `StaticModConfig`). Must still deserialize once
    /// `prefilter`/`prefilter_chunk_size`/`prefilter_low_memory` exist, falling
    /// back to Sage's own resolved defaults (see NOTES "Database prefiltering").
    #[test]
    fn old_config_json_without_prefilter_fields_still_loads() {
        let mut value = serde_json::to_value(DatabaseConfig::default())
            .expect("DatabaseConfig::default() must serialize");
        let obj = value
            .as_object_mut()
            .expect("DatabaseConfig serializes as an object");
        for key in ["prefilter", "prefilter_chunk_size", "prefilter_low_memory"] {
            assert!(
                obj.remove(key).is_some(),
                "expected `{key}` in the serialized default — did the field get renamed?"
            );
        }

        let db: DatabaseConfig = serde_json::from_value(value)
            .expect("a pre-prefilter config JSON must still deserialize");
        assert!(
            !db.prefilter,
            "prefilter must default to false (Sage's own default)"
        );
        assert_eq!(
            db.prefilter_chunk_size, 0,
            "chunk size must default to 0 (auto)"
        );
        assert!(
            db.prefilter_low_memory,
            "low_memory must default to true, matching Sage's own resolved default \
             (Builder::make_parameters, crates/sage/src/database.rs) — NOT false"
        );
    }

    #[test]
    fn default_database_config_maps_to_sage_defaults_in_builder() {
        let builder: Builder = DatabaseConfig::default().into();
        assert_eq!(builder.prefilter, Some(false));
        assert_eq!(builder.prefilter_chunk_size, Some(0));
        assert_eq!(builder.prefilter_low_memory, Some(true));
    }

    #[test]
    fn enabling_prefilter_round_trips_through_builder() {
        let db = DatabaseConfig {
            prefilter: true,
            prefilter_chunk_size: 5000,
            prefilter_low_memory: false,
            ..DatabaseConfig::default()
        };
        let builder: Builder = db.into();
        assert_eq!(builder.prefilter, Some(true));
        assert_eq!(builder.prefilter_chunk_size, Some(5000));
        assert_eq!(builder.prefilter_low_memory, Some(false));
    }

    #[test]
    fn deserialized_mods_are_invisible_until_synced_from_ser() {
        // Reproduces the 2026-08-24 live-test bug: `static_mods`/`variable_mods`
        // are `#[serde(skip)]` (their key type has no Deserialize), so a plain
        // `serde_json::from_str` — exactly what `eframe::get_value` does —
        // restores the serializable shadow map but leaves the live map, which
        // the UI actually reads from, empty.
        let mut db = DatabaseConfig::default();
        db.static_mods.insert_key("Y", 79.9663); // phospho on Y, not a default
        db.variable_mods.variable_mods.insert_key("N", 0.98402); // deamidation, not a default

        let json = serde_json::to_string(&db).expect("DatabaseConfig must serialize");
        let mut restored: DatabaseConfig =
            serde_json::from_str(&json).expect("a saved config must deserialize");

        assert!(
            restored.static_mods.static_mods.is_empty(),
            "documents the bug: the live map comes back empty from plain deserialize"
        );
        assert_eq!(
            restored.static_mods.static_mods_ser.len(),
            2,
            "the serializable shadow map, unlike the live one, did round-trip"
        );

        restored.static_mods.sync_from_ser();
        restored.variable_mods.variable_mods.sync_from_ser();

        assert_eq!(
            restored.static_mods.static_mods, db.static_mods.static_mods,
            "after syncing, the live static mods must match what was saved"
        );
        assert_eq!(
            restored.variable_mods.variable_mods.static_mods,
            db.variable_mods.variable_mods.static_mods,
            "after syncing, the live variable mods must match what was saved"
        );
    }
}
