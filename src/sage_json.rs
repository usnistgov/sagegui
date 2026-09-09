// Written by Benjamin A. Neely (NIST) on 2026-09-08.
//! Reading Sage-schema JSON into SageGUI's [`Config`].
//!
//! One reader serves two jobs: the bundled experiment templates in
//! `assets/templates/`, and importing the parameters of a past run so a search
//! can be reproduced or tweaked. Both are the same operation — load a
//! Sage-shaped JSON document into `self.config`.
//!
//! **Two formats, one struct.** Sage writes and reads two different shapes:
//!
//! | File | Sage type | `database` field | Notes |
//! | ---- | --------- | ---------------- | ----- |
//! | `config.json` (CLI input) | `Input` | `Builder` — every field `Option` | deserialisable |
//! | `results.json` (run output) | `Search` | `Parameters` — resolved values | **serialise-only** |
//!
//! `Search` has no `Deserialize`, so we cannot reuse Sage's own types to read a
//! `results.json`. But the JSON *key names* agree between the two, and
//! `ModificationSpecificity` serialises to the same string keys that `Builder`
//! uses, so a single all-`Option` struct reads both. Unknown keys are ignored
//! (Sage sets `deny_unknown_fields` nowhere either), which is what lets a
//! `results.json`'s extra `version` / `output_paths` / `score_type` ride along
//! harmlessly, and lets our own `_sagegui` template metadata ride along in the
//! other direction — a template file stays valid input to the Sage CLI.
//!
//! **What is deliberately not imported:** `mzml_paths`, the FASTA path, and
//! `output_directory`. A template or a past run must never silently repoint
//! this session at someone else's files. The user's file selections survive
//! every import.

use serde::Deserialize;
use std::collections::HashMap;

use sage_core::ion_series::Kind;
use sage_core::mass::Tolerance;
use sage_core::modification::ModificationSpecificity;
use sage_core::tmt::Isobaric;
use std::str::FromStr;

use crate::ui::{
    Config, IsobarSelection, QuantType, SupportedQuantTypes, TmtSettingsSer, ToleranceConfig,
    ToleranceType,
};

// ─── Metadata block ──────────────────────────────────────────────────────────

/// Our own `_sagegui` block. Sage ignores it; it drives the template picker.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TemplateMeta {
    pub name: String,
    pub description: String,
    pub order: u32,
}

// ─── Document ────────────────────────────────────────────────────────────────

/// A Sage-schema JSON document — either a `config.json` or a `results.json`.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SageJson {
    #[serde(rename = "_sagegui")]
    pub meta: Option<TemplateMeta>,
    pub database: DatabaseJson,
    pub precursor_tol: Option<Tolerance>,
    pub fragment_tol: Option<Tolerance>,
    pub precursor_charge: Option<(u8, u8)>,
    pub override_precursor_charge: Option<bool>,
    pub isotope_errors: Option<(i8, i8)>,
    pub deisotope: Option<bool>,
    pub chimera: Option<bool>,
    pub wide_window: Option<bool>,
    pub min_peaks: Option<u32>,
    pub max_peaks: Option<u32>,
    pub max_fragment_charge: Option<u8>,
    pub min_matched_peaks: Option<u16>,
    pub report_psms: Option<usize>,
    pub predict_rt: Option<bool>,
    pub write_pin: Option<bool>,
    pub annotate_matches: Option<bool>,
    pub quant: Option<QuantJson>,
    // ── Read, but deliberately never applied ──────────────────────────────
    // These describe someone else's files. They are parsed only so the import
    // can tell the user what it left behind and what needs re-selecting.
    // A `results.json` writes these as percent-encoded `file://` URLs; a
    // hand-written `config.json` usually carries bare paths. Both are handled.
    pub mzml_paths: Option<Vec<String>>,
    pub output_directory: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct DatabaseJson {
    pub bucket_size: Option<usize>,
    pub enzyme: Option<EnzymeJson>,
    pub peptide_min_mass: Option<f32>,
    pub peptide_max_mass: Option<f32>,
    pub ion_kinds: Option<Vec<Kind>>,
    pub min_ion_index: Option<u32>,
    pub static_mods: Option<HashMap<String, f32>>,
    pub variable_mods: Option<HashMap<String, MassList>>,
    pub max_variable_mods: Option<u32>,
    pub decoy_tag: Option<String>,
    pub generate_decoys: Option<bool>,
    pub prefilter: Option<bool>,
    pub prefilter_chunk_size: Option<usize>,
    pub prefilter_low_memory: Option<bool>,
    /// Read but never applied — see `SageJson::mzml_paths`.
    pub fasta: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct EnzymeJson {
    pub missed_cleavages: Option<u8>,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub cleave_at: Option<String>,
    /// Absent, `null` and `""` all mean the same thing: no restriction.
    ///
    /// This mirrors Sage, which resolves the field with
    /// `en.restrict.unwrap_or_else(|| "".into())`. Sage's `Some("P")` default
    /// applies only when the whole `enzyme` key is missing, not when the key
    /// is present and this field is not. An earlier version of this reader
    /// treated absent as "leave the current value alone", which made SageGUI
    /// digest a file differently from the Sage CLI reading the same file.
    pub restrict: Option<String>,
    pub c_terminal: Option<bool>,
    pub semi_enzymatic: Option<bool>,
}

/// Sage's `variable_mods` values are `Vec<f32>`, but a hand-written config may
/// carry a bare scalar. Accept either rather than failing the whole file.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MassList {
    One(f32),
    Many(Vec<f32>),
}

impl MassList {
    fn as_slice(&self) -> Vec<f32> {
        match self {
            MassList::One(m) => vec![*m],
            MassList::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct QuantJson {
    pub tmt: Option<Isobaric>,
    pub tmt_settings: Option<TmtJson>,
    pub lfq: Option<bool>,
    pub lfq_settings: Option<LfqJson>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TmtJson {
    pub level: Option<u8>,
    pub sn: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LfqJson {
    pub ppm_tolerance: Option<f32>,
    pub spectral_angle: Option<f64>,
    pub combine_charge_states: Option<bool>,
}

// ─── Import report ───────────────────────────────────────────────────────────

/// What an import actually did. Anything skipped or lossy is reported rather
/// than silently dropped — a search that quietly differs from the file the user
/// pointed at is worse than one that refuses to load.
#[derive(Debug, Default)]
pub struct ImportReport {
    /// Human-readable name of what was loaded.
    pub source: String,
    /// Non-fatal problems: unsupported values, keys we could not map.
    pub warnings: Vec<String>,
    /// File selections the imported document carried that were deliberately
    /// not applied, and which the user therefore has to pick again.
    pub needs_reselect: Vec<String>,
}

impl ImportReport {
    /// One-line summary for the status bar.
    pub fn summary(&self) -> String {
        let mut summary = if self.warnings.is_empty() {
            format!("Loaded {}.", self.source)
        } else {
            format!(
                "Loaded {} with {} warning{}.",
                self.source,
                self.warnings.len(),
                if self.warnings.len() == 1 { "" } else { "s" }
            )
        };
        if !self.needs_reselect.is_empty() {
            summary.push_str(" Search parameters only — file selections need picking again.");
        }
        summary
    }
}

// ─── Parsing ─────────────────────────────────────────────────────────────────

impl SageJson {
    pub fn from_str(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Display name for this document — the template's own name if it carries
    /// one, otherwise the caller's fallback (usually a file name).
    pub fn display_name(&self, fallback: &str) -> String {
        self.meta
            .as_ref()
            .map(|m| m.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Write this document's parameters onto `config`.
    ///
    /// File selections (`mzml_paths`, FASTA paths, `output_directory`) are
    /// never touched — see the module docs.
    pub fn apply(
        &self,
        config: &mut Config,
        precursor_type: &mut ToleranceType,
        fragment_type: &mut ToleranceType,
        source: &str,
    ) -> ImportReport {
        let mut report = ImportReport {
            source: self.display_name(source),
            warnings: Vec::new(),
            needs_reselect: Vec::new(),
        };

        self.apply_database(config, &mut report);
        self.apply_tolerances(config, precursor_type, fragment_type, &mut report);
        self.apply_scalars(config);
        self.apply_quant(config, &mut report);
        self.note_file_selections(&mut report);

        report
    }

    /// Record the file selections the document carried but that we refuse to
    /// apply. Importing a past run's `results.json` is the main case: the
    /// mzML and FASTA paths in it are from that run, on that machine, and may
    /// not exist here — so the user is told what was in the file, whether it
    /// is still on disk, and where to re-pick it.
    fn note_file_selections(&self, report: &mut ImportReport) {
        if let Some(paths) = &self.mzml_paths {
            if !paths.is_empty() {
                let pretty: Vec<String> = paths.iter().map(|p| pretty_path(p)).collect();
                let found = pretty.iter().filter(|p| path_exists(p)).count();
                let detail = if found == pretty.len() {
                    "all still on disk at those paths".to_string()
                } else if found == 0 {
                    "none found on this machine".to_string()
                } else {
                    format!("{found} of {} still on disk", pretty.len())
                };
                report.needs_reselect.push(format!(
                    "{} spectrum file{} listed ({detail}) — re-select them on Files & Database. \
                     First: {}",
                    pretty.len(),
                    if pretty.len() == 1 { "" } else { "s" },
                    pretty[0]
                ));
            }
        }

        if let Some(fasta) = &self.database.fasta {
            if !fasta.is_empty() {
                let pretty = pretty_path(fasta);
                let state = if path_exists(&pretty) {
                    "still on disk"
                } else {
                    "not found on this machine"
                };
                report.needs_reselect.push(format!(
                    "FASTA listed ({state}) — re-select it on Files & Database: {pretty}"
                ));
            }
        }

        if let Some(dir) = &self.output_directory {
            if !dir.is_empty() {
                report.needs_reselect.push(format!(
                    "Output folder listed — set it on Run / Info if you want it: {}",
                    pretty_path(dir)
                ));
            }
        }
    }

    fn apply_database(&self, config: &mut Config, report: &mut ImportReport) {
        let db = &self.database;
        let target = &mut config.database;

        if let Some(v) = db.bucket_size {
            target.bucket_size = v;
        }
        if let Some(v) = db.peptide_min_mass {
            target.peptide_min_mass = v;
        }
        if let Some(v) = db.peptide_max_mass {
            target.peptide_max_mass = v;
        }
        if let Some(v) = db.min_ion_index {
            target.min_ion_index = v;
        }
        if let Some(v) = db.max_variable_mods {
            target.max_variable_mods = v;
        }
        if let Some(v) = &db.decoy_tag {
            target.decoy_tag = Some(v.clone());
        }
        if let Some(v) = db.generate_decoys {
            target.generate_decoys = v;
        }
        if let Some(v) = db.prefilter {
            target.prefilter = v;
        }
        if let Some(v) = db.prefilter_chunk_size {
            target.prefilter_chunk_size = v;
        }
        if let Some(v) = db.prefilter_low_memory {
            target.prefilter_low_memory = v;
        }

        // Ion kinds arrive as a list; our UI holds a checkbox per kind, so an
        // explicit list replaces the whole selection rather than merging.
        if let Some(kinds) = &db.ion_kinds {
            for kind in crate::ui::IonKindSelection::variants() {
                target.ion_kinds.ion_kinds.insert(kind, false);
            }
            for kind in kinds {
                target.ion_kinds.ion_kinds.insert(*kind, true);
            }
        }

        if let Some(enzyme) = &db.enzyme {
            let e = &mut target.enzyme;
            if let Some(v) = enzyme.missed_cleavages {
                e.missed_cleavages = v;
            }
            if let Some(v) = enzyme.min_len {
                e.min_len = v;
            }
            if let Some(v) = enzyme.max_len {
                e.max_len = v;
            }
            if let Some(v) = &enzyme.cleave_at {
                e.cleave_at = v.clone();
            }
            if let Some(v) = enzyme.c_terminal {
                e.c_terminal = v;
            }
            if let Some(v) = enzyme.semi_enzymatic {
                e.semi_enzymatic = v;
            }
            // Resolved exactly as Sage does: absent, null and empty all mean
            // no restriction. Only a real value restricts.
            match enzyme.restrict.as_deref().unwrap_or("") {
                "" => e.enable_restrict = false,
                c => {
                    e.enable_restrict = true;
                    e.restrict_char = c.to_string();
                }
            }
        }

        // Modifications replace rather than merge. A template must be able to
        // clear a stale mod carried over from an unrelated prior search — that
        // is the whole point of having a reset-to-a-known-state control.
        if let Some(mods) = &db.static_mods {
            target.static_mods.static_mods.clear();
            target.static_mods.static_mods_ser.clear();
            for (key, mass) in mods {
                if ModificationSpecificity::from_str(key).is_ok() {
                    target.static_mods.insert_key(key, *mass);
                } else {
                    report
                        .warnings
                        .push(format!("Skipped unrecognised static mod key {key:?}."));
                }
            }
        }

        if let Some(mods) = &db.variable_mods {
            target.variable_mods.variable_mods.static_mods.clear();
            target.variable_mods.variable_mods.static_mods_ser.clear();
            for (key, masses) in mods {
                let masses = masses.as_slice();
                if ModificationSpecificity::from_str(key).is_err() {
                    report
                        .warnings
                        .push(format!("Skipped unrecognised variable mod key {key:?}."));
                    continue;
                }
                match masses.split_first() {
                    None => report
                        .warnings
                        .push(format!("Variable mod {key:?} had no masses; skipped.")),
                    Some((first, rest)) => {
                        target.variable_mods.variable_mods.insert_key(key, *first);
                        if !rest.is_empty() {
                            // Sage allows several masses per residue; this GUI
                            // holds one. Say so rather than drop them quietly.
                            report.warnings.push(format!(
                                "Variable mod {key:?} listed {} masses; kept {first} and dropped {}. \
                                 SageGUI holds one mass per residue.",
                                masses.len(),
                                rest.iter()
                                    .map(|m| m.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    }
                }
            }
        }
    }

    fn apply_tolerances(
        &self,
        config: &mut Config,
        precursor_type: &mut ToleranceType,
        fragment_type: &mut ToleranceType,
        report: &mut ImportReport,
    ) {
        if let Some(tol) = self.precursor_tol {
            match convert_tolerance(tol) {
                Some((cfg, kind)) => {
                    config.precursor_tol = cfg;
                    *precursor_type = kind;
                }
                None => report.warnings.push(
                    "Precursor tolerance uses Sage's `pct` unit, which this GUI has no control for; \
                     left unchanged."
                        .to_string(),
                ),
            }
        }
        if let Some(tol) = self.fragment_tol {
            match convert_tolerance(tol) {
                Some((cfg, kind)) => {
                    config.fragment_tol = cfg;
                    *fragment_type = kind;
                }
                None => report.warnings.push(
                    "Fragment tolerance uses Sage's `pct` unit, which this GUI has no control for; \
                     left unchanged."
                        .to_string(),
                ),
            }
        }
    }

    fn apply_scalars(&self, config: &mut Config) {
        if let Some(v) = self.precursor_charge {
            config.precursor_charge = v;
        }
        if let Some(v) = self.override_precursor_charge {
            config.override_precursor_charge = v;
        }
        if let Some(v) = self.isotope_errors {
            config.isotope_errors = v;
        }
        if let Some(v) = self.deisotope {
            config.deisotope = v;
        }
        if let Some(v) = self.chimera {
            config.chimera = v;
        }
        if let Some(v) = self.wide_window {
            config.wide_window = v;
        }
        if let Some(v) = self.min_peaks {
            config.min_peaks = v;
        }
        if let Some(v) = self.max_peaks {
            config.max_peaks = v;
        }
        if let Some(v) = self.max_fragment_charge {
            config.max_fragment_charge = v;
        }
        if let Some(v) = self.min_matched_peaks {
            config.min_matched_peaks = v;
        }
        if let Some(v) = self.report_psms {
            config.report_psms = v;
        }
        if let Some(v) = self.predict_rt {
            config.predict_rt = v;
        }
        if let Some(v) = self.write_pin {
            config.write_pin = v;
        }
        if let Some(v) = self.annotate_matches {
            config.annotate_matches = v;
        }
    }

    fn apply_quant(&self, config: &mut Config, report: &mut ImportReport) {
        let Some(quant) = &self.quant else {
            return;
        };

        if let Some(isobar) = &quant.tmt {
            match IsobarSelection::from_isobaric(isobar) {
                Some(selection) => {
                    let mut settings = TmtSettingsSer::default();
                    if let Some(t) = &quant.tmt_settings {
                        if let Some(level) = t.level {
                            settings.level = level;
                        }
                        if let Some(sn) = t.sn {
                            settings.sn = sn;
                        }
                    }
                    config.quant_class = SupportedQuantTypes::Tmt;
                    config.quant = QuantType::Tmt(selection, settings);
                    config.quant_enabled = true;
                }
                None => report.warnings.push(
                    "TMT uses a custom reporter-ion list (`User`), which this GUI cannot represent; \
                     quantification left unchanged."
                        .to_string(),
                ),
            }
            return;
        }

        if quant.lfq == Some(true) || quant.lfq_settings.is_some() {
            let mut settings = match &config.quant {
                QuantType::Lfq(existing) => *existing,
                _ => sage_core::lfq::LfqSettings::default(),
            };
            if let Some(l) = &quant.lfq_settings {
                if let Some(v) = l.ppm_tolerance {
                    settings.ppm_tolerance = v;
                }
                if let Some(v) = l.spectral_angle {
                    settings.spectral_angle = v;
                }
                if let Some(v) = l.combine_charge_states {
                    settings.combine_charge_states = v;
                }
            }
            config.quant_class = SupportedQuantTypes::Lfq;
            config.quant = QuantType::Lfq(settings);
            config.quant_enabled = true;
        } else if quant.lfq == Some(false) && quant.tmt.is_none() {
            config.quant_enabled = false;
        }
    }
}

/// Turn a stored path into something worth showing a person.
///
/// A `results.json` records spectrum files as `file://` URLs with the path
/// percent-encoded, so a file called `B.naive 01.mzML.gz` comes back as
/// `file:///Users/…/B.naive%2001.mzML.gz`. A hand-written `config.json` just
/// has the bare path. Handle both; leave genuine cloud URLs (`s3://`, `gs://`,
/// `az://`) alone, since for those the URL *is* the readable form.
fn pretty_path(raw: &str) -> String {
    let Some(rest) = raw.strip_prefix("file://") else {
        return raw.to_string();
    };
    percent_decode(rest)
}

/// Minimal percent-decoder, enough for the `file://` URLs Sage writes.
/// Invalid escapes are passed through untouched rather than dropped.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether a recorded path is present on this machine. Cloud URLs always read
/// as absent — we are not going to reach out to object storage to find out.
fn path_exists(path: &str) -> bool {
    !path.contains("://") && std::path::Path::new(path).exists()
}

/// Map Sage's `Tolerance` onto the GUI's narrower `ToleranceConfig`.
/// Returns `None` for `Pct`, which the GUI has no widget for.
fn convert_tolerance(tol: Tolerance) -> Option<(ToleranceConfig, ToleranceType)> {
    match tol {
        Tolerance::Ppm(lo, hi) => Some((ToleranceConfig::Ppm(lo, hi), ToleranceType::Ppm)),
        Tolerance::Da(lo, hi) => Some((ToleranceConfig::Da(lo, hi), ToleranceType::Da)),
        Tolerance::Pct(_, _) => None,
    }
}

// ─── Bundled templates ───────────────────────────────────────────────────────

/// The starting configurations shipped with the app.
///
/// Embedded with `include_str!` rather than read from `assets/templates/` at
/// runtime: the released artifact is a single binary (and, on macOS, an `.app`
/// bundle), so a directory beside the executable is not something we can rely
/// on. Adding a template therefore needs a rebuild — acceptable, because a
/// user's own templates come in through "Load Sage config…" instead.
pub const BUNDLED_TEMPLATES: &[(&str, &str)] = &[
    (
        "tryptic-wide-ms1.json",
        include_str!("../assets/templates/tryptic-wide-ms1.json"),
    ),
    (
        "tryptic-tight.json",
        include_str!("../assets/templates/tryptic-tight.json"),
    ),
    (
        "tryptic-open.json",
        include_str!("../assets/templates/tryptic-open.json"),
    ),
    (
        "tryptic-biofluid.json",
        include_str!("../assets/templates/tryptic-biofluid.json"),
    ),
    ("tmt11.json", include_str!("../assets/templates/tmt11.json")),
];

/// A bundled template, parsed and ready to show in the picker.
pub struct Template {
    pub file: &'static str,
    pub meta: TemplateMeta,
    pub doc: SageJson,
}

/// Parse every bundled template, sorted by the `order` in its metadata.
/// A malformed bundled template is a build-time mistake, not a user error, so
/// it is logged and skipped rather than crashing the app.
pub fn bundled_templates() -> Vec<Template> {
    let mut out: Vec<Template> = BUNDLED_TEMPLATES
        .iter()
        .filter_map(|(file, text)| match SageJson::from_str(text) {
            Ok(doc) => {
                let meta = doc.meta.clone().unwrap_or_default();
                Some(Template { file, meta, doc })
            }
            Err(e) => {
                log::error!("bundled template {file} failed to parse: {e}");
                None
            }
        })
        .collect();
    out.sort_by_key(|t| t.meta.order);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ui::Config;

    fn apply(text: &str) -> (Config, ToleranceType, ToleranceType, ImportReport) {
        let doc = SageJson::from_str(text).expect("must parse");
        let mut config = Config::default();
        let mut p = ToleranceType::Ppm;
        let mut f = ToleranceType::Ppm;
        let report = doc.apply(&mut config, &mut p, &mut f, "test");
        (config, p, f, report)
    }

    /// Every bundled template must parse and apply cleanly. This is the test
    /// that fails if a template file is edited into an invalid shape.
    #[test]
    fn bundled_templates_all_load() {
        let templates = bundled_templates();
        assert_eq!(
            templates.len(),
            BUNDLED_TEMPLATES.len(),
            "a bundled template failed to parse"
        );
        for t in &templates {
            assert!(!t.meta.name.is_empty(), "{} has no name", t.file);
            assert!(
                !t.meta.description.is_empty(),
                "{} has no description",
                t.file
            );
            let mut config = Config::default();
            let mut p = ToleranceType::Ppm;
            let mut f = ToleranceType::Ppm;
            let report = t.doc.apply(&mut config, &mut p, &mut f, t.file);
            assert!(
                report.warnings.is_empty(),
                "{} applied with warnings: {:?}",
                t.file,
                report.warnings
            );
        }
    }

    /// What Sage itself does with `restrict`, asserted against Sage's own
    /// types rather than against our reading of its source.
    ///
    /// This is the reference the importer has to match. Absent, `null` and
    /// `""` are all "no restriction"; only a real value restricts. Sage's
    /// `Some("P")` default applies only when the entire `enzyme` key is
    /// missing, which is a different case.
    ///
    /// If a Sage upgrade changes this, this test fails first and the importer
    /// follows it. Do not adjust the importer without re-running this.
    #[test]
    fn sage_treats_absent_null_and_empty_restrict_the_same() {
        use sage_core::database::EnzymeBuilder;
        use sage_core::enzyme::EnzymeParameters;
        let p_index = (b'P' - b'A') as usize;

        let skips_before_p = |json: &str| {
            let builder: EnzymeBuilder = serde_json::from_str(json).expect("valid enzyme block");
            let params: EnzymeParameters = builder.into();
            params.enzyme.expect("an enzyme").skip_suffix[p_index]
        };

        assert!(!skips_before_p(r#"{"cleave_at":"KR"}"#), "absent");
        assert!(
            !skips_before_p(r#"{"cleave_at":"KR","restrict":null}"#),
            "null"
        );
        assert!(
            !skips_before_p(r#"{"cleave_at":"KR","restrict":""}"#),
            "empty"
        );
        assert!(
            skips_before_p(r#"{"cleave_at":"KR","restrict":"P"}"#),
            "explicit P"
        );
    }

    /// The importer must agree with the reference above. Getting this wrong
    /// makes SageGUI digest a file differently from the Sage CLI reading the
    /// same file, which is a wrong result with nothing on screen to show it.
    #[test]
    fn the_importer_matches_sage_on_restrict() {
        for (label, json) in [
            ("absent", r#"{"database":{"enzyme":{"cleave_at":"KR"}}}"#),
            ("null", r#"{"database":{"enzyme":{"restrict":null}}}"#),
            ("empty", r#"{"database":{"enzyme":{"restrict":""}}}"#),
        ] {
            let (config, ..) = apply(json);
            assert!(
                !config.database.enzyme.enable_restrict,
                "{label} restrict should leave no restriction in force"
            );
            assert_eq!(config.database.enzyme.effective_restrict(), "", "{label}");
        }

        let (config, ..) = apply(r#"{"database":{"enzyme":{"restrict":"P"}}}"#);
        assert!(config.database.enzyme.enable_restrict);
        assert_eq!(config.database.enzyme.effective_restrict(), "P");
    }

    /// The wide-MS1 template's window is a delta mass of -1.25 to +3.5 Da,
    /// which Sage stores as the negated-and-swapped raw pair [-3.5, 1.25].
    /// Asserting the raw pair here pins the convention: see AGENTS.md.
    #[test]
    fn wide_ms1_template_keeps_raw_window_orientation() {
        let t = bundled_templates();
        let highres = t
            .iter()
            .find(|t| t.file == "tryptic-wide-ms1.json")
            .expect("template present");
        let mut config = Config::default();
        let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
        highres.doc.apply(&mut config, &mut p, &mut f, "test");

        match config.precursor_tol {
            ToleranceConfig::Da(lo, hi) => {
                assert_eq!((lo, hi), (-3.5, 1.25), "raw Sage pair, not the delta range");
                assert!(lo < hi, "lower bound must be the more negative number");
            }
            other => panic!("expected a Da window, got {other:?}"),
        }
        assert!(matches!(p, ToleranceType::Da));
        assert!(matches!(f, ToleranceType::Ppm));
    }

    /// The open template's window must stay oriented the same way: the large
    /// negative number on the left. Raw [-500, 100] means delta -100 to +500.
    #[test]
    fn open_template_window_is_not_flipped() {
        let t = bundled_templates();
        let open = t
            .iter()
            .find(|t| t.file == "tryptic-open.json")
            .expect("template present");
        let mut config = Config::default();
        let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
        open.doc.apply(&mut config, &mut p, &mut f, "test");

        match config.precursor_tol {
            ToleranceConfig::Da(lo, hi) => assert_eq!((lo, hi), (-500.0, 100.0)),
            other => panic!("expected a Da window, got {other:?}"),
        }
    }

    /// Importing must replace modifications, not merge them — otherwise a stale
    /// mod from a prior search survives a template load, which is the exact
    /// problem templates exist to solve.
    #[test]
    fn static_mods_replace_rather_than_merge() {
        let (config, ..) = apply(r#"{"database":{"static_mods":{"K":229.16293}}}"#);
        let keys: Vec<String> = config
            .database
            .static_mods
            .static_mods
            .keys()
            .map(|k| k.to_string())
            .collect();
        assert_eq!(keys, vec!["K".to_string()], "default C mod must be cleared");
    }

    /// A residue carrying several masses is lossy here (the GUI holds one).
    /// It must warn rather than drop silently.
    #[test]
    fn multi_mass_variable_mod_warns() {
        let (_, _, _, report) = apply(r#"{"database":{"variable_mods":{"M":[15.9949,31.9898]}}}"#);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("dropped"));
    }

    /// File selections must never be touched by an import.
    #[test]
    fn import_leaves_file_selections_alone() {
        let doc = SageJson::from_str(
            r#"{"mzml_paths":["/somewhere/else.mzML"],
                "output_directory":"/not/my/folder",
                "database":{"fasta":"/not/my/db.fasta"}}"#,
        )
        .expect("must parse");
        let mut config = Config::default();
        config.mzml_paths = vec!["/my/real.mzML".into()];
        config.database.fasta_paths = vec!["/my/real.fasta".into()];
        config.output_directory = "/my/output".to_string();

        let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
        let report = doc.apply(&mut config, &mut p, &mut f, "test");

        assert_eq!(
            config.mzml_paths,
            vec![std::path::PathBuf::from("/my/real.mzML")]
        );
        assert_eq!(
            config.database.fasta_paths,
            vec![std::path::PathBuf::from("/my/real.fasta")]
        );
        assert_eq!(config.output_directory, "/my/output");

        // Skipping them silently would be its own bug: the user needs telling
        // that the file carried selections they have to make themselves.
        assert_eq!(
            report.needs_reselect.len(),
            3,
            "expected notes for spectra, FASTA and output folder: {:?}",
            report.needs_reselect
        );
    }

    /// A `results.json` records spectrum files as percent-encoded `file://`
    /// URLs. The re-select note has to show a readable path, not the raw URL.
    #[test]
    fn results_json_file_urls_are_shown_readably() {
        let doc = SageJson::from_str(
            r#"{"mzml_paths":["file:///Users/ben/data/B.naive%2001steady-state.mzML.gz"],
                "database":{"fasta":"file:///Users/ben/data/human%20canonical.fasta"}}"#,
        )
        .expect("must parse");
        let mut config = Config::default();
        let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
        let report = doc.apply(&mut config, &mut p, &mut f, "results.json");

        let joined = report.needs_reselect.join("\n");
        assert!(
            joined.contains("/Users/ben/data/B.naive 01steady-state.mzML.gz"),
            "percent-encoding should be decoded: {joined}"
        );
        assert!(
            joined.contains("/Users/ben/data/human canonical.fasta"),
            "FASTA path should be decoded: {joined}"
        );
        assert!(!joined.contains("file://"), "scheme should be stripped");
        assert!(!joined.contains("%20"), "escapes should be decoded");
    }

    /// Bundled templates carry no file paths, so they must produce no
    /// re-select notes — otherwise every template load nags the user.
    #[test]
    fn templates_produce_no_reselect_notes() {
        for t in bundled_templates() {
            let mut config = Config::default();
            let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
            let report = t.doc.apply(&mut config, &mut p, &mut f, t.file);
            assert!(
                report.needs_reselect.is_empty(),
                "{} should not ask for re-selection: {:?}",
                t.file,
                report.needs_reselect
            );
        }
    }

    /// The load-bearing claim of this module is that one struct reads both
    /// Sage formats. This proves it against Sage's **own** serialization
    /// rather than a hand-typed approximation: build the real `Search` type
    /// that `Runner` writes to `results.json`, serialize it with serde exactly
    /// as Sage does, and read the result back through the importer.
    ///
    /// Deriving the fixture this way is deliberate — a hand-authored one gets
    /// the shape subtly wrong (see NOTES: the prefilter tests had exactly that
    /// problem with `ion_kinds`/`static_mods`), and would keep passing after a
    /// Sage upgrade changed the real layout. This test fails on that upgrade,
    /// which is the point.
    #[test]
    fn real_sage_results_json_round_trips() {
        use sage_cli::input::TmtSettings;
        use sage_cli::input::{QuantSettings, Search};
        use sage_cloudpath::Url;
        use sage_core::database::{EnzymeBuilder, Parameters};
        use sage_core::scoring::ScoreType;

        let mut static_mods = HashMap::new();
        static_mods.insert(ModificationSpecificity::from_str("C").unwrap(), 57.0215f32);
        let mut variable_mods = HashMap::new();
        variable_mods.insert(
            ModificationSpecificity::from_str("M").unwrap(),
            vec![15.9949f32],
        );

        let search = Search {
            version: "0.15.0-beta.2".into(),
            database: Parameters {
                bucket_size: 8192,
                enzyme: EnzymeBuilder {
                    missed_cleavages: Some(2),
                    min_len: Some(7),
                    max_len: Some(50),
                    cleave_at: Some("KR".into()),
                    // The case that matters: Michael's go-to config turns the
                    // proline restriction off, and Sage records that as null.
                    restrict: None,
                    c_terminal: Some(true),
                    semi_enzymatic: Some(false),
                },
                peptide_min_mass: 500.0,
                peptide_max_mass: 5000.0,
                ion_kinds: vec![Kind::B, Kind::Y],
                min_ion_index: 2,
                static_mods,
                variable_mods,
                max_variable_mods: 3,
                decoy_tag: "rev_".into(),
                generate_decoys: true,
                fasta: "/some/other/machine/human.fasta".into(),
                prefilter_chunk_size: 0,
                prefilter: false,
                prefilter_low_memory: true,
            },
            quant: QuantSettings {
                tmt: None,
                tmt_settings: TmtSettings::default(),
                lfq: true,
                lfq_settings: sage_core::lfq::LfqSettings::default(),
            },
            precursor_tol: Tolerance::Da(-3.5, 1.25),
            fragment_tol: Tolerance::Ppm(-10.0, 10.0),
            precursor_charge: (2, 4),
            override_precursor_charge: false,
            isotope_errors: (0, 0),
            deisotope: true,
            chimera: false,
            wide_window: false,
            min_peaks: 15,
            max_peaks: 150,
            max_fragment_charge: Some(1),
            min_matched_peaks: 4,
            report_psms: 1,
            predict_rt: true,
            mzml_paths: vec![Url::parse("file:///some/other/machine/run%2001.mzML.gz").unwrap()],
            output_paths: vec![Url::parse("file:///some/other/machine/results.sage.tsv").unwrap()],
            bruker_config: Default::default(),
            protein_grouping: true,
            protein_grouping_peptide_fdr: 0.01,
            output_directory: Url::parse("file:///some/other/machine/out").unwrap(),
            write_pin: false,
            write_report: false,
            annotate_matches: false,
            score_type: ScoreType::SageHyperScore,
        };

        let json = serde_json::to_string_pretty(&search).expect("Search must serialize");
        let doc = SageJson::from_str(&json).expect("our importer must read Sage's own output");

        let mut config = Config::default();
        let (mut p, mut f) = (ToleranceType::Ppm, ToleranceType::Ppm);
        let report = doc.apply(&mut config, &mut p, &mut f, "results.json");

        // Search parameters came through.
        assert_eq!(config.database.bucket_size, 8192);
        assert_eq!(config.database.enzyme.min_len, 7);
        assert_eq!(config.database.max_variable_mods, 3);
        assert_eq!(config.min_matched_peaks, 4);
        assert!(config.deisotope);
        assert!(matches!(config.precursor_tol, ToleranceConfig::Da(lo, hi)
            if (lo, hi) == (-3.5, 1.25)));
        assert!(matches!(p, ToleranceType::Da));

        // The null restrict survived Sage's own round trip.
        assert!(
            !config.database.enzyme.enable_restrict,
            "Sage writes a disabled restriction as null; it must stay disabled"
        );

        // Mods came back through the string-keyed shadow map.
        assert_eq!(config.database.static_mods.static_mods.len(), 1);
        assert_eq!(
            config
                .database
                .variable_mods
                .variable_mods
                .static_mods
                .len(),
            1
        );

        // Someone else's files were not adopted, but were reported.
        assert!(config.mzml_paths.is_empty());
        assert!(config.database.fasta_paths.is_empty());
        assert!(
            report
                .needs_reselect
                .iter()
                .any(|n| n.contains("run 01.mzML.gz")),
            "the spectrum file should be named, decoded: {:?}",
            report.needs_reselect
        );
        assert!(
            report
                .needs_reselect
                .iter()
                .any(|n| n.contains("human.fasta")),
            "the FASTA should be named: {:?}",
            report.needs_reselect
        );
        // `output_directory` is `skip_serializing` on `Search`, so a real
        // results.json has none — nothing to re-select for it.
        assert!(
            !json.contains("output_directory"),
            "Sage does not write output_directory into results.json"
        );

        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    /// A `results.json` carries keys a `config.json` never has (`version`,
    /// `output_paths`, `score_type`, a resolved `quant` block). They must be
    /// ignored, and the parameters still applied.
    #[test]
    fn results_json_extra_keys_are_ignored() {
        let (config, ..) = apply(
            r#"{"version":"0.15.0-beta.2",
                "output_paths":["/tmp/results.sage.tsv"],
                "score_type":"SageHyperScore",
                "bruker_config":{"ms1":{},"ms2":{}},
                "protein_grouping":true,
                "database":{"bucket_size":8192},
                "report_psms":3}"#,
        );
        assert_eq!(config.database.bucket_size, 8192);
        assert_eq!(config.report_psms, 3);
    }
}
