//! Search parameters read back from `results.json`, in the form the two
//! writers need.
//!
//! The reader reuses [`SageJson`], which already tolerates the differences
//! between Sage versions: every field is optional and unknown keys are ignored.
//! `version` and `score_type` are not part of `SageJson`, so they come from a
//! second, untyped read of the same text.
//!
//! A `results.json` from 0.14.6 has no `score_type`, and its enzyme block has
//! `c_terminal: null` and `semi_enzymatic: null`. Both cases load. Anything
//! that is missing stays missing (`None`) and the writers leave that part out.
//! We do not guess a value Sage did not record.

use std::collections::HashMap;
use std::path::Path;

use sage_core::ion_series::Kind;
use sage_core::mass::Tolerance;

use crate::sage_json::{pretty_path, MassList, SageJson};
use crate::ui::ToleranceConfig;

/// Unit of a tolerance window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TolUnit {
    Da,
    Ppm,
    Pct,
}

/// A tolerance window in DELTA-MASS terms: `observed - theoretical`.
///
/// This is NOT the pair stored in `results.json`. Sage's raw pair is the
/// negated and reversed window. See "STOP precursor tolerance sign
/// convention" in AGENTS.md. Everything written to a file goes through this
/// type, so the raw pair never reaches an output by accident.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeltaTol {
    pub unit: TolUnit,
    /// Most negative delta mass. Delta convention.
    pub low: f32,
    /// Most positive delta mass. Delta convention.
    pub high: f32,
}

impl DeltaTol {
    /// Convert Sage's raw pair (RAW convention) to a delta-mass window.
    ///
    /// The same rule holds for the fragment window. `page_search` in the Sage
    /// database centres `fragment_tol.bounds(m)` on the EXPERIMENTAL peak mass
    /// and looks up THEORETICAL fragments in it. That is the same direction as
    /// the precursor search, so the fragment pair is also negated and reversed.
    /// Fragment windows are usually symmetric, which hides the mistake.
    ///
    /// The flip itself is `ToleranceConfig::displayed_delta`, the one the GUI
    /// uses. `Pct` has no `ToleranceConfig` variant. The flip does not depend
    /// on the unit, so it goes through the `Da` variant.
    pub fn from_raw(tol: Tolerance) -> Self {
        let (unit, (low, high)) = match tol {
            Tolerance::Ppm(a, b) => (TolUnit::Ppm, ToleranceConfig::Ppm(a, b).displayed_delta()),
            Tolerance::Da(a, b) => (TolUnit::Da, ToleranceConfig::Da(a, b).displayed_delta()),
            Tolerance::Pct(a, b) => (TolUnit::Pct, ToleranceConfig::Da(a, b).displayed_delta()),
        };
        DeltaTol { unit, low, high }
    }

    /// Value for `MS:1001412` search tolerance plus. Delta convention.
    /// A window that starts above zero gives a plus value above zero too.
    pub fn plus(&self) -> f32 {
        self.high + 0.0
    }

    /// Value for `MS:1001413` search tolerance minus. Delta convention.
    /// The CV term takes a magnitude below the centre, so a window from -1.25
    /// gives 1.25. A window that starts above zero gives a negative value.
    /// `+ 0.0` turns negative zero into zero.
    pub fn minus(&self) -> f32 {
        -self.low + 0.0
    }
}

/// Where a modification applies. Mirrors the prefix syntax of Sage's mod keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModTarget {
    /// Any position of this residue.
    Residue(char),
    /// Peptide N-terminus. With a residue, only when that residue is first.
    PeptideN(Option<char>),
    PeptideC(Option<char>),
    ProteinN(Option<char>),
    ProteinC(Option<char>),
}

/// One configured modification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModSpec {
    pub target: ModTarget,
    /// Mass difference in Da, as Sage stores it (`f32`).
    pub delta: f32,
}

/// Digestion settings, with Sage's defaults filled in where the file has
/// `null`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnzymeSpec {
    /// Residues that are cut at. Empty means no enzyme. `$` means no cleavage.
    pub cleave_at: String,
    /// No cut when the next residue is one of these. Empty means no rule.
    pub restrict: String,
    pub c_terminal: bool,
    pub semi_enzymatic: bool,
    pub missed_cleavages: Option<u32>,
    pub min_len: Option<u32>,
    pub max_len: Option<u32>,
}

/// Everything the writers take from `results.json`.
#[derive(Debug, Clone, Default)]
pub struct Params {
    /// Sage version as recorded, for example `0.15.0-beta.2`.
    pub sage_version: Option<String>,
    pub score_type: Option<String>,
    pub enzyme: Option<EnzymeSpec>,
    pub static_mods: Vec<ModSpec>,
    pub variable_mods: Vec<ModSpec>,
    pub max_variable_mods: Option<u32>,
    pub decoy_tag: Option<String>,
    pub generate_decoys: Option<bool>,
    /// Path or URL of the FASTA as recorded. Decoded from `file://`.
    pub fasta: Option<String>,
    /// Delta-mass window. See [`DeltaTol`].
    pub precursor_tol: Option<DeltaTol>,
    /// Delta-mass window. See [`DeltaTol`].
    pub fragment_tol: Option<DeltaTol>,
    pub precursor_charge: Option<(u8, u8)>,
    pub isotope_errors: Option<(i8, i8)>,
    pub ion_kinds: Vec<Kind>,
    pub report_psms: Option<usize>,
    /// The input files as recorded, decoded from `file://`.
    pub mzml_paths: Vec<String>,
}

impl Params {
    /// Read `results.json` from a Sage output folder.
    pub fn load(out_dir: &Path) -> Result<Params, String> {
        let path = out_dir.join("results.json");
        let text = std::fs::read_to_string(&path).map_err(|e| {
            format!(
                "Cannot read {}: {e}. The converter needs results.json next to results.sage.tsv.",
                path.display()
            )
        })?;
        Self::from_json(&text).map_err(|e| format!("Cannot read {}: {e}", path.display()))
    }

    /// Parse the text of a `results.json`. Missing fields stay `None`.
    pub fn from_json(text: &str) -> Result<Params, String> {
        let doc = SageJson::from_str(text).map_err(|e| format!("not a Sage results.json ({e})"))?;
        // `version` and `score_type` are not in `SageJson`. Read them by name.
        let raw: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("not valid JSON ({e})"))?;
        let text_of = |key: &str| {
            raw.get(key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .filter(|s| !s.is_empty())
        };

        let db = &doc.database;
        let enzyme = db.enzyme.as_ref().map(|e| EnzymeSpec {
            cleave_at: e.cleave_at.clone().unwrap_or_default(),
            restrict: e.restrict.clone().unwrap_or_default(),
            // `null` in a 0.14.x file. Sage's own default is true.
            c_terminal: e.c_terminal.unwrap_or(true),
            // `null` in a 0.14.x file. Sage's own default is false.
            semi_enzymatic: e.semi_enzymatic.unwrap_or(false),
            missed_cleavages: e.missed_cleavages.map(u32::from),
            min_len: e.min_len.map(|n| n as u32),
            max_len: e.max_len.map(|n| n as u32),
        });

        let mut static_mods = Vec::new();
        for (key, mass) in db.static_mods.iter().flatten() {
            if let Some(target) = parse_mod_key(key) {
                static_mods.push(ModSpec {
                    target,
                    delta: *mass,
                });
            }
        }
        let mut variable_mods = Vec::new();
        for (key, masses) in db.variable_mods.iter().flatten() {
            let Some(target) = parse_mod_key(key) else {
                continue;
            };
            let list: Vec<f32> = match masses {
                MassList::One(m) => vec![*m],
                MassList::Many(v) => v.clone(),
            };
            for delta in list {
                variable_mods.push(ModSpec { target, delta });
            }
        }
        // A JSON object has no order. Sort so two runs write the same file.
        let by_key = |a: &ModSpec, b: &ModSpec| {
            format!("{:?}", a.target)
                .cmp(&format!("{:?}", b.target))
                .then(a.delta.total_cmp(&b.delta))
        };
        static_mods.sort_by(by_key);
        variable_mods.sort_by(by_key);

        Ok(Params {
            sage_version: text_of("version"),
            score_type: text_of("score_type"),
            enzyme,
            static_mods,
            variable_mods,
            max_variable_mods: db.max_variable_mods,
            decoy_tag: db.decoy_tag.clone(),
            generate_decoys: db.generate_decoys,
            fasta: db
                .fasta
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(pretty_path),
            precursor_tol: doc.precursor_tol.map(DeltaTol::from_raw),
            fragment_tol: doc.fragment_tol.map(DeltaTol::from_raw),
            precursor_charge: doc.precursor_charge,
            isotope_errors: doc.isotope_errors,
            ion_kinds: db.ion_kinds.clone().unwrap_or_default(),
            report_psms: doc.report_psms,
            mzml_paths: doc
                .mzml_paths
                .iter()
                .flatten()
                .map(|p| pretty_path(p))
                .collect(),
        })
    }

    /// Full path of an input file, found by the base name that the TSV holds.
    /// `None` when `mzml_paths` does not list it.
    pub fn path_of(&self, filename: &str) -> Option<&str> {
        let map: HashMap<&str, &str> = self
            .mzml_paths
            .iter()
            .map(|p| (base_name(p), p.as_str()))
            .collect();
        map.get(filename).copied()
    }
}

/// The last path component. Splits on both separators, because a Windows path
/// can arrive on a Mac and the other way round.
pub fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Parse a key of `static_mods` or `variable_mods`. Same grammar as Sage's
/// `ModificationSpecificity::from_str`: an optional prefix (`^ $ [ ]`) and an
/// optional residue. An unknown key returns `None` and is left out.
pub fn parse_mod_key(key: &str) -> Option<ModTarget> {
    let mut chars = key.chars();
    let first = chars.next()?;
    let residue = chars.next();
    if chars.next().is_some() {
        return None;
    }
    let res = |c: Option<char>| c.filter(char::is_ascii_uppercase);
    match first {
        '^' => Some(ModTarget::PeptideN(res(residue))),
        '$' => Some(ModTarget::PeptideC(res(residue))),
        '[' => Some(ModTarget::ProteinN(res(residue))),
        ']' => Some(ModTarget::ProteinC(res(residue))),
        c if c.is_ascii_uppercase() && residue.is_none() => Some(ModTarget::Residue(c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precursor_window_is_written_in_delta_mass_terms_da() {
        // RAW convention in the JSON: [-500, 100]. That is delta -100 to +500.
        let t = DeltaTol::from_raw(Tolerance::Da(-500.0, 100.0));
        assert_eq!(t.unit, TolUnit::Da);
        assert_eq!((t.low, t.high), (-100.0, 500.0));
        // What the file gets, in delta terms.
        assert_eq!(t.plus(), 500.0);
        assert_eq!(t.minus(), 100.0);
    }

    #[test]
    fn precursor_window_is_written_in_delta_mass_terms_ppm() {
        // RAW [-20, 10] ppm is delta -10 to +20 ppm.
        let t = DeltaTol::from_raw(Tolerance::Ppm(-20.0, 10.0));
        assert_eq!(t.unit, TolUnit::Ppm);
        assert_eq!((t.low, t.high), (-10.0, 20.0));
        assert_eq!(t.plus(), 20.0);
        assert_eq!(t.minus(), 10.0);
    }

    #[test]
    fn a_symmetric_window_gives_equal_plus_and_minus_and_no_negative_zero() {
        let t = DeltaTol::from_raw(Tolerance::Ppm(-10.0, 10.0));
        assert_eq!((t.plus(), t.minus()), (10.0, 10.0));
        let zero = DeltaTol::from_raw(Tolerance::Da(0.0, 5.0));
        // Delta window is -5 to 0. Plus is 0, and it must not print as -0.
        assert_eq!(zero.plus().to_string(), "0");
        assert_eq!(zero.minus(), 5.0);
    }

    #[test]
    fn a_window_that_starts_above_zero_gives_a_negative_minus() {
        // RAW [-5, -1] is delta +1 to +5. The lower edge is above the centre.
        let t = DeltaTol::from_raw(Tolerance::Da(-5.0, -1.0));
        assert_eq!((t.low, t.high), (1.0, 5.0));
        assert_eq!((t.plus(), t.minus()), (5.0, -1.0));
    }

    #[test]
    fn mod_keys_follow_the_sage_prefix_grammar() {
        assert_eq!(parse_mod_key("M"), Some(ModTarget::Residue('M')));
        assert_eq!(parse_mod_key("^Q"), Some(ModTarget::PeptideN(Some('Q'))));
        assert_eq!(parse_mod_key("$"), Some(ModTarget::PeptideC(None)));
        assert_eq!(parse_mod_key("["), Some(ModTarget::ProteinN(None)));
        assert_eq!(parse_mod_key("]K"), Some(ModTarget::ProteinC(Some('K'))));
        assert_eq!(parse_mod_key("m"), None);
        assert_eq!(parse_mod_key("KR"), None);
        assert_eq!(parse_mod_key(""), None);
    }

    #[test]
    fn a_0_14_6_results_json_loads_with_null_enzyme_flags() {
        let p = Params::from_json(include_str!(
            "../../tests/fixtures/export/results_0.14.6.json"
        ))
        .expect("0.14.6 JSON must load");
        assert_eq!(p.sage_version.as_deref(), Some("0.14.6"));
        assert_eq!(p.score_type, None, "0.14.6 has no score_type");
        let e = p.enzyme.clone().expect("enzyme block");
        assert_eq!(e.cleave_at, "KR");
        assert_eq!(e.restrict, "P");
        // `null` in the file, so Sage's own defaults apply.
        assert!(e.c_terminal);
        assert!(!e.semi_enzymatic);
        // The real 0.14.6 run was an open search. RAW [-500, 100] Da.
        let t = p.precursor_tol.expect("precursor tolerance");
        assert_eq!((t.unit, t.low, t.high), (TolUnit::Da, -100.0, 500.0));
        assert_eq!(p.mzml_paths.len(), 1);
        assert_eq!(
            p.path_of("B.naive_01steady-state.mzML.gz"),
            Some("c:/path/to/inputs/B.naive_01steady-state.mzML.gz")
        );
    }

    #[test]
    fn a_0_15_results_json_loads_with_mods_and_asymmetric_tolerance() {
        let p = Params::from_json(include_str!("../../tests/fixtures/export/results.json"))
            .expect("0.15 JSON must load");
        assert_eq!(p.sage_version.as_deref(), Some("0.15.0-beta.2"));
        assert_eq!(p.score_type.as_deref(), Some("SageHyperScore"));
        assert_eq!(p.static_mods.len(), 1);
        assert_eq!(p.static_mods[0].target, ModTarget::Residue('C'));
        assert_eq!(p.variable_mods.len(), 1);
        assert_eq!(p.max_variable_mods, Some(3));
        // RAW Da [-3.5, 1.25] is delta -1.25 to +3.5.
        let t = p.precursor_tol.expect("precursor tolerance");
        assert_eq!((t.low, t.high), (-1.25, 3.5));
    }

    #[test]
    fn missing_or_broken_results_json_is_an_error_not_a_panic() {
        let dir = std::env::temp_dir().join("sagegui-export-params-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let err = Params::load(&dir).unwrap_err();
        assert!(err.contains("results.json"), "{err}");

        std::fs::write(dir.join("results.json"), "{ not json").unwrap();
        let err = Params::load(&dir).unwrap_err();
        assert!(err.contains("results.json"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_object_loads_and_leaves_everything_unset() {
        let p = Params::from_json("{}").expect("empty object");
        assert!(p.enzyme.is_none() && p.precursor_tol.is_none() && p.fasta.is_none());
    }

    #[test]
    fn base_name_splits_on_both_separators() {
        assert_eq!(base_name("c:/a/b/run.mzML.gz"), "run.mzML.gz");
        assert_eq!(base_name("c:\\a\\b\\run.mzML"), "run.mzML");
        assert_eq!(base_name("run.mzML"), "run.mzML");
    }
}
