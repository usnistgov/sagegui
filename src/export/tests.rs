//! End-to-end tests for the converters.
//!
//! They run on a small real slice of Sage output in `tests/fixtures/export/`:
//! 185 rows taken from a 0.15.0-beta.2 serum search. The slice has targets and
//! decoys, rows above and below 1% q-value, rows with several proteins, and
//! rows with carbamidomethyl and oxidation. Its `results.json` has an
//! asymmetric Da window.
//!
//! Each test copies what it needs to a scratch folder, because the converters
//! write next to the input.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use super::model::{load_table, parse_peptide, residue_mass};
use super::params::{DeltaTol, Params};
use super::*;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export")
}

fn schema_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/schemas")
}

/// A fresh scratch folder that holds the fixture, or only its JSON.
fn scratch(name: &str, with_tsv: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sagegui-export-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture_dir().join("results.json"), dir.join("results.json")).unwrap();
    if with_tsv {
        std::fs::copy(
            fixture_dir().join("results.sage.tsv"),
            dir.join("results.sage.tsv"),
        )
        .unwrap();
    }
    dir
}

/// The header of the fixture TSV, for the hand-made rows.
fn header() -> String {
    let text = std::fs::read_to_string(fixture_dir().join("results.sage.tsv")).unwrap();
    text.lines().next().unwrap().to_string()
}

/// One row in the 43-column layout. Only the fields that matter are set.
#[allow(clippy::too_many_arguments)]
fn row(
    id: u32,
    peptide: &str,
    proteins: &str,
    scan: u32,
    rank: u32,
    label: i32,
    expmass: f64,
    calcmass: f64,
    charge: i32,
) -> String {
    let cols = header();
    cols.split('\t')
        .map(|c| match c {
            "psm_id" => id.to_string(),
            "peptide" => peptide.to_string(),
            "proteins" | "protein_groups" => proteins.to_string(),
            "num_proteins" | "num_protein_groups" => proteins.split(';').count().to_string(),
            "filename" => "run1.mzML.gz".to_string(),
            "scannr" => format!("controllerType=0 controllerNumber=1 scan={scan}"),
            "rank" => rank.to_string(),
            "label" => label.to_string(),
            "expmass" => expmass.to_string(),
            "calcmass" => calcmass.to_string(),
            "charge" => charge.to_string(),
            "hyperscore" => "50.5".to_string(),
            "delta_next" => "10.25".to_string(),
            "rt" => "12.5".to_string(),
            "spectrum_q" => "0.001".to_string(),
            "peptide_q" => "0.002".to_string(),
            "protein_q" => "0.003".to_string(),
            "precursor_ppm" => "1.5".to_string(),
            "isotope_error" => "0.0".to_string(),
            "matched_peaks" => "12".to_string(),
            "missed_cleavages" => "0".to_string(),
            _ => "0".to_string(),
        })
        .collect::<Vec<_>>()
        .join("\t")
}

fn write_tsv(dir: &Path, rows: &[String]) {
    let mut text = header();
    text.push('\n');
    for r in rows {
        text.push_str(r);
        text.push('\n');
    }
    std::fs::write(dir.join("results.sage.tsv"), text).unwrap();
}

/// (label, spectrum_q, peptide_q, protein_q) of every fixture row, read
/// without the converter's own parser.
fn fixture_rows() -> Vec<(i32, f64, f64, f64)> {
    let text = std::fs::read_to_string(fixture_dir().join("results.sage.tsv")).unwrap();
    let mut lines = text.lines();
    let names: Vec<&str> = lines.next().unwrap().split('\t').collect();
    let at = |n: &str| names.iter().position(|c| *c == n).unwrap();
    let (l, s, p, r) = (
        at("label"),
        at("spectrum_q"),
        at("peptide_q"),
        at("protein_q"),
    );
    lines
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            (
                f[l].parse().unwrap(),
                f[s].parse().unwrap(),
                f[p].parse().unwrap(),
                f[r].parse().unwrap(),
            )
        })
        .collect()
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

// ── Filters ─────────────────────────────────────────────────────────────

#[test]
fn default_filter_keeps_targets_at_or_below_one_percent_spectrum_q() {
    let rows = fixture_rows();
    assert_eq!(rows.len(), 185);
    let expected = rows.iter().filter(|r| r.0 == 1 && r.1 <= 0.01).count();
    // The fixture is built to hold rows on both sides of every filter.
    assert!(expected > 0 && expected < rows.len());

    let dir = scratch("default-filter", true);
    let path = convert_to_mzid(&dir, &ExportOptions::default()).unwrap();
    let text = read(&path);
    assert_eq!(count(&text, "<SpectrumIdentificationItem "), expected);
    assert_eq!(count(&text, "isDecoy=\"true\""), 0);
    assert_eq!(count(&text, "passThreshold=\"false\""), 0);
    // The threshold in the protocol says what was done.
    assert!(text.contains("accession=\"MS:1002260\" name=\"PSM:FDR threshold\" value=\"0.01\""));

    let pep = convert_to_pepxml(&dir, &ExportOptions::default()).unwrap();
    assert_eq!(count(&read(&pep), "<search_hit "), expected);
}

#[test]
fn including_decoys_adds_them_and_marks_them() {
    let rows = fixture_rows();
    let expected = rows.iter().filter(|r| r.1 <= 0.01).count();
    let decoys = rows.iter().filter(|r| r.0 == -1 && r.1 <= 0.01).count();
    assert!(decoys > 0);

    let dir = scratch("with-decoys", true);
    let opts = ExportOptions {
        include_decoys: true,
        ..ExportOptions::default()
    };
    let text = read(&convert_to_mzid(&dir, &opts).unwrap());
    assert_eq!(count(&text, "<SpectrumIdentificationItem "), expected);
    // Every decoy PSM has a decoy tag on all of its proteins.
    let decoy_evidence = count(&text, "isDecoy=\"true\"");
    assert!(decoy_evidence >= decoys, "{decoy_evidence} < {decoys}");
    assert!(text.contains("accession=\"rev_sp|"));
}

#[test]
fn protein_q_filter_is_stricter_than_spectrum_q_on_the_fixture() {
    let rows = fixture_rows();
    let by_spectrum = rows.iter().filter(|r| r.0 == 1 && r.1 <= 0.01).count();
    let by_protein = rows.iter().filter(|r| r.0 == 1 && r.3 <= 0.01).count();
    assert!(
        by_protein < by_spectrum,
        "the fixture needs rows that differ"
    );

    let dir = scratch("protein-q", true);
    let opts = ExportOptions {
        q_source: QSource::ProteinQ,
        ..ExportOptions::default()
    };
    let text = read(&convert_to_mzid(&dir, &opts).unwrap());
    assert_eq!(count(&text, "<SpectrumIdentificationItem "), by_protein);
    assert!(
        text.contains("MS:1001447"),
        "the threshold term follows the source"
    );

    let opts = ExportOptions {
        q_source: QSource::PeptideQ,
        ..ExportOptions::default()
    };
    let by_peptide = rows.iter().filter(|r| r.0 == 1 && r.2 <= 0.01).count();
    let text = read(&convert_to_mzid(&dir, &opts).unwrap());
    assert_eq!(count(&text, "<SpectrumIdentificationItem "), by_peptide);
}

#[test]
fn a_limit_of_one_keeps_everything_and_says_no_threshold() {
    let dir = scratch("no-limit", true);
    let opts = ExportOptions {
        max_q: 1.0,
        include_decoys: true,
        ..ExportOptions::default()
    };
    let text = read(&convert_to_mzid(&dir, &opts).unwrap());
    assert_eq!(count(&text, "<SpectrumIdentificationItem "), 185);
    assert!(text.contains("MS:1001494"));
}

#[test]
fn an_empty_result_is_an_error_that_says_why() {
    let dir = scratch("nothing-left", true);
    let opts = ExportOptions {
        max_q: 0.0,
        ..ExportOptions::default()
    };
    let err = convert_to_mzid(&dir, &opts).unwrap_err();
    assert!(err.contains("No PSMs are left"), "{err}");
    assert!(!dir.join("results.sage.mzid").exists());
    assert!(!dir.join("results.sage.mzid.tmp").exists());
}

// ── Missing input ───────────────────────────────────────────────────────

#[test]
fn a_missing_results_json_is_an_error() {
    let dir = scratch("no-json", true);
    std::fs::remove_file(dir.join("results.json")).unwrap();
    for err in [
        convert_to_mzid(&dir, &ExportOptions::default()).unwrap_err(),
        convert_to_pepxml(&dir, &ExportOptions::default()).unwrap_err(),
    ] {
        assert!(err.contains("results.json"), "{err}");
    }
}

#[test]
fn a_missing_tsv_is_an_error() {
    let dir = scratch("no-tsv", false);
    for err in [
        convert_to_mzid(&dir, &ExportOptions::default()).unwrap_err(),
        convert_to_pepxml(&dir, &ExportOptions::default()).unwrap_err(),
    ] {
        assert!(err.contains("results.sage.tsv"), "{err}");
    }
}

#[test]
fn a_tsv_without_a_needed_column_names_the_column() {
    let dir = scratch("no-column", false);
    std::fs::write(
        dir.join("results.sage.tsv"),
        "psm_id\tpeptide\nrow\tPEPTIDE\n",
    )
    .unwrap();
    let err = convert_to_mzid(&dir, &ExportOptions::default()).unwrap_err();
    assert!(err.contains("`proteins`"), "{err}");
}

#[test]
fn a_bad_number_names_the_line_and_the_column() {
    let dir = scratch("bad-number", false);
    let mut bad = row(1, "PEPTIDE", "sp|A|A", 5, 1, 1, 800.4, 800.4, 2);
    bad = bad.replace("\t0.001\t", "\tnotanumber\t");
    write_tsv(&dir, &[bad]);
    let err = convert_to_mzid(&dir, &ExportOptions::default()).unwrap_err();
    assert!(
        err.contains("Line 2") && err.contains("notanumber"),
        "{err}"
    );
}

// ── Parameters ──────────────────────────────────────────────────────────

#[test]
fn asymmetric_tolerance_windows_are_written_in_delta_mass_terms() {
    // Da. RAW convention in the JSON: [-500, 100]. Delta window: -100 to +500.
    let mut buf = Vec::new();
    mzid::write_tolerance(
        &mut buf,
        "ParentTolerance",
        &DeltaTol::from_raw(sage_core::mass::Tolerance::Da(-500.0, 100.0)),
    )
    .unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(
        text.contains(
            "accession=\"MS:1001412\" name=\"search tolerance plus value\" value=\"500\" \
             unitAccession=\"UO:0000221\" unitName=\"dalton\""
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "accession=\"MS:1001413\" name=\"search tolerance minus value\" value=\"100\" \
             unitAccession=\"UO:0000221\""
        ),
        "{text}"
    );

    // ppm. RAW [-20, 10]. Delta window: -10 to +20.
    let mut buf = Vec::new();
    mzid::write_tolerance(
        &mut buf,
        "FragmentTolerance",
        &DeltaTol::from_raw(sage_core::mass::Tolerance::Ppm(-20.0, 10.0)),
    )
    .unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(
        text.contains(
            "name=\"search tolerance plus value\" value=\"20\" unitAccession=\"UO:0000169\""
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "name=\"search tolerance minus value\" value=\"10\" unitAccession=\"UO:0000169\""
        ),
        "{text}"
    );
}

#[test]
fn the_fixture_json_gives_plus_3_5_and_minus_1_25_da() {
    // The fixture `results.json` holds RAW Da [-3.5, 1.25]. Delta is -1.25 to +3.5.
    let dir = scratch("tol-fixture", true);
    let text = read(&convert_to_mzid(&dir, &ExportOptions::default()).unwrap());
    let parent = text
        .split("<ParentTolerance>")
        .nth(1)
        .and_then(|s| s.split("</ParentTolerance>").next())
        .expect("ParentTolerance block");
    assert!(parent.contains("plus value\" value=\"3.5\""), "{parent}");
    assert!(parent.contains("minus value\" value=\"1.25\""), "{parent}");

    let pep = read(&convert_to_pepxml(&dir, &ExportOptions::default()).unwrap());
    assert!(pep.contains("name=\"precursor_delta_mass_low\" value=\"-1.25\""));
    assert!(pep.contains("name=\"precursor_delta_mass_high\" value=\"3.5\""));
}

#[test]
fn search_parameters_reach_the_mzid_protocol() {
    let dir = scratch("protocol", true);
    let text = read(&convert_to_mzid(&dir, &ExportOptions::default()).unwrap());
    // Fixed carbamidomethyl and variable oxidation, as unknown modifications.
    assert!(text
        .contains("<SearchModification fixedMod=\"true\" massDelta=\"57.0215\" residues=\"C\">"));
    assert!(text
        .contains("<SearchModification fixedMod=\"false\" massDelta=\"15.9949\" residues=\"M\">"));
    assert!(text.contains("accession=\"MS:1001460\" name=\"unknown modification\""));
    // KR with no restrict is Trypsin/P. Two missed cleavages.
    assert!(text.contains("<SiteRegexp>(?&lt;=[KR])</SiteRegexp>"));
    assert!(text.contains("accession=\"MS:1001313\" name=\"Trypsin/P\""));
    assert!(text.contains("missedCleavages=\"2\""));
    // Software and search type.
    assert!(text.contains("accession=\"MS:1004007\" name=\"Sage\""));
    assert!(text.contains("version=\"0.15.0-beta.2\""));
    assert!(text.contains("accession=\"MS:1001083\" name=\"ms-ms search\""));
    // Input files and the nativeID form.
    assert!(text.contains("accession=\"MS:1000768\" name=\"Thermo nativeID format\""));
    assert!(text.contains("accession=\"MS:1000584\" name=\"mzML format\""));
    assert!(text.contains("accession=\"MS:1001348\" name=\"FASTA format\""));
    assert!(text.contains("<DBSequence id=\"DBSeq_1\" accession=\"sp|"));
}

// ── PSM content ─────────────────────────────────────────────────────────

#[test]
fn mzid_item_holds_the_right_mz_scores_and_evidence() {
    let dir = scratch("mzid-item", false);
    // Charge 3, neutral masses 2256.162 and 2255.1548.
    write_tsv(
        &dir,
        &[row(
            7,
            "AGAFC[+57.0215]LSEDAGLGISSTASLR",
            "sp|P01023|A2MG_HUMAN;sp|P20742|PZP_HUMAN",
            42,
            1,
            1,
            2256.162,
            2255.1548,
            3,
        )],
    );
    let text = read(&convert_to_mzid(&dir, &ExportOptions::default()).unwrap());

    let attr = |name: &str| -> f64 {
        let key = format!("{name}=\"");
        let i = text.find(&key).unwrap() + key.len();
        text[i..].split('"').next().unwrap().parse().unwrap()
    };
    // (M + z * 1.00727646677) / z
    assert!((attr("experimentalMassToCharge") - 753.06127646677).abs() < 1e-9);
    assert!(
        (attr("calculatedMassToCharge") - (2255.1548 + 3.0 * 1.00727646677) / 3.0).abs() < 1e-9
    );
    assert!(text.contains("chargeState=\"3\""));
    assert!(text.contains("rank=\"1\" passThreshold=\"true\""));
    // The spectrum ID is the full nativeID. The scores follow.
    assert!(text.contains("spectrumID=\"controllerType=0 controllerNumber=1 scan=42\""));
    assert!(text.contains("accession=\"MS:1001331\" name=\"X!Tandem:hyperscore\" value=\"50.5\""));
    assert!(text.contains("accession=\"MS:1002354\" name=\"PSM-level q-value\" value=\"0.001\""));
    assert!(text.contains(
        "accession=\"MS:1001868\" name=\"distinct peptide-level q-value\" value=\"0.002\""
    ));
    assert!(text.contains("name=\"Sage:protein_q\" value=\"0.003\""));
    // The 12.5 min retention time keeps its unit.
    assert!(text.contains("accession=\"MS:1000016\" name=\"scan start time\" value=\"12.5\" unitAccession=\"UO:0000031\" unitName=\"minute\""));
    // The `precursor_ppm` column is a ppm value, and it is labelled that way.
    assert!(text.contains("name=\"Sage:precursor_ppm\" value=\"1.5\" unitAccession=\"UO:0000169\""));
    // One peptide, its modification, and two protein evidences.
    assert!(text.contains("<PeptideSequence>AGAFCLSEDAGLGISSTASLR</PeptideSequence>"));
    assert!(text.contains(
        "<Modification location=\"5\" residues=\"C\" monoisotopicMassDelta=\"57.0215\">"
    ));
    assert_eq!(count(&text, "<PeptideEvidenceRef "), 2);
    assert_eq!(count(&text, "<DBSequence "), 2);
    assert_eq!(count(&text, "<PeptideEvidence "), 2);
}

#[test]
fn terminal_and_negative_mods_are_placed_at_the_ends_in_mzid() {
    let dir = scratch("mzid-terminal", false);
    write_tsv(
        &dir,
        &[row(
            1,
            "[+42.0106]-Q[-17.026548]PEPTIDEK-[+0.984]",
            "sp|X|X",
            9,
            1,
            1,
            1000.5,
            1000.5,
            2,
        )],
    );
    let text = read(&convert_to_mzid(&dir, &ExportOptions::default()).unwrap());
    assert!(text.contains("<PeptideSequence>QPEPTIDEK</PeptideSequence>"));
    // N-terminus is location 0, and has no residue.
    assert!(text.contains("<Modification location=\"0\" monoisotopicMassDelta=\"42.0106\">"));
    // A negative delta on the first residue.
    assert!(text.contains(
        "<Modification location=\"1\" residues=\"Q\" monoisotopicMassDelta=\"-17.026548\">"
    ));
    // C-terminus is length + 1.
    assert!(text.contains("<Modification location=\"10\" monoisotopicMassDelta=\"0.984\">"));
}

#[test]
fn ranks_of_one_spectrum_share_one_result() {
    let dir = scratch("ranks", false);
    write_tsv(
        &dir,
        &[
            row(2, "PEPTIDEK", "sp|B|B", 100, 2, 1, 900.45, 900.44, 2),
            row(1, "PEPTIDER", "sp|A|A", 100, 1, 1, 900.45, 900.45, 2),
            row(3, "OTHERPEPK", "sp|C|C", 101, 1, 1, 950.5, 950.5, 2),
        ],
    );
    let opts = ExportOptions::default();
    let text = read(&convert_to_mzid(&dir, &opts).unwrap());
    assert_eq!(count(&text, "<SpectrumIdentificationResult "), 2);
    assert_eq!(count(&text, "<SpectrumIdentificationItem "), 3);
    let first = text.find("rank=\"1\"").unwrap();
    let second = text.find("rank=\"2\"").unwrap();
    assert!(first < second, "the better hit comes first");

    let pep = read(&convert_to_pepxml(&dir, &opts).unwrap());
    assert_eq!(count(&pep, "<spectrum_query "), 2);
    assert_eq!(count(&pep, "<search_hit "), 3);
    assert!(pep.contains("hit_rank=\"2\""));
}

#[test]
fn pepxml_query_has_seconds_massdiff_and_total_residue_mass() {
    let dir = scratch("pepxml-query", false);
    write_tsv(
        &dir,
        &[row(
            5,
            "[+42.0106]-AGAFC[+57.0215]LSEDAM[+15.9949]K-[+0.984]",
            "sp|P01023|A2MG_HUMAN;sp|P20742|PZP_HUMAN",
            31593,
            1,
            1,
            1200.5,
            1200.25,
            2,
        )],
    );
    let text = read(&convert_to_pepxml(&dir, &ExportOptions::default()).unwrap());

    // Base name, scan and charge in the spectrum name.
    assert!(text.contains("spectrum=\"run1.31593.31593.2\""), "{text}");
    assert!(text.contains("start_scan=\"31593\" end_scan=\"31593\""));
    assert!(text.contains("precursor_neutral_mass=\"1200.5\" assumed_charge=\"2\" index=\"1\""));
    // 12.5 minutes is 750 seconds.
    assert!(text.contains("retention_time_sec=\"750\""));
    // massdiff is expmass - calcmass, in Da, signed.
    assert!(text.contains("calc_neutral_pep_mass=\"1200.25\" massdiff=\"0.25\""));
    // Residue mods are the TOTAL residue mass, not the delta.
    // C: 103.00919 + 57.0215 = 160.03069. M: 131.0405 + 15.9949 = 147.0354.
    assert!(
        text.contains("<mod_aminoacid_mass position=\"5\" mass=\"160.03069\"/>"),
        "{text}"
    );
    assert!(
        text.contains("<mod_aminoacid_mass position=\"11\" mass=\"147.0354\"/>"),
        "{text}"
    );
    // Termini count the free terminus. H + 42.0106 and OH + 0.984.
    assert!(text.contains("mod_nterm_mass=\"43.018425"), "{text}");
    assert!(text.contains("mod_cterm_mass=\"17.98674"), "{text}");
    // The bare sequence, with every protein.
    assert!(text.contains("peptide=\"AGAFCLSEDAMK\""));
    assert!(text.contains("protein=\"sp|P01023|A2MG_HUMAN\" num_tot_proteins=\"2\""));
    assert!(text.contains("<alternative_protein protein=\"sp|P20742|PZP_HUMAN\"/>"));
    // Scores. `nextscore` is hyperscore minus delta_next: 50.5 - 10.25.
    assert!(text.contains("<search_score name=\"hyperscore\" value=\"50.5\"/>"));
    assert!(text.contains("<search_score name=\"nextscore\" value=\"40.25\"/>"));
    assert!(text.contains("<search_score name=\"delta_next\" value=\"10.25\"/>"));
    assert!(text.contains("<search_score name=\"spectrum_q\" value=\"0.001\"/>"));
    // The run and the engine.
    assert!(
        text.contains(
            "<msms_run_summary base_name=\"run1\" raw_data_type=\"raw\" raw_data=\".mzML.gz\">"
        ),
        "{text}"
    );
    assert!(text.contains("search_engine=\"Sage\""));
    assert!(text.contains("search_engine_version=\"0.15.0-beta.2\""));
    // Search modifications: static C, variable M, both with the total mass.
    assert!(text.contains("aminoacid=\"C\" massdiff=\"57.0215\" mass=\"160.03069\" variable=\"N\""));
    assert!(text.contains("aminoacid=\"M\" massdiff=\"15.9949\" mass=\"147.0354\" variable=\"Y\""));
    // Enzyme.
    assert!(text.contains("<sample_enzyme name=\"trypsin_p\" fidelity=\"specific\">"));
    assert!(text.contains("<specificity sense=\"C\" cut=\"KR\"/>"));
}

#[test]
fn recomputed_peptide_mass_matches_calcmass_for_every_fixture_row() {
    // This checks the peptide parser and the residue table against Sage's own
    // theoretical mass. If a delta or a residue mass were wrong, a row would
    // miss by more than the `f32` rounding of `calcmass`.
    let dir = scratch("mass-check", true);
    let opts = ExportOptions {
        max_q: 1.0,
        include_decoys: true,
        ..ExportOptions::default()
    };
    let table = load_table(
        &dir.join("results.sage.tsv"),
        &opts,
        &Control::default(),
        0.0,
        1.0,
    )
    .unwrap();
    assert_eq!(table.psms.len(), 185);
    let h2o = 18.010565;
    for psm in &table.psms {
        let entry = &table.peptides[psm.pep as usize];
        let pp = parse_peptide(&entry.peptide).unwrap();
        let residues: f64 = pp.sequence.chars().map(|c| residue_mass(c).unwrap()).sum();
        let mods: f64 = pp.mods.iter().map(|m| m.delta).sum();
        let total = residues + h2o + mods;
        assert!(
            (total - psm.calcmass).abs() < 0.005,
            "{}: {total} vs {}",
            entry.peptide,
            psm.calcmass
        );
    }
}

#[test]
fn every_kept_psm_lies_inside_the_emitted_precursor_window() {
    // The data decides the tolerance convention, not our own arithmetic.
    // Sage removes the isotope offset first. What is left is `observed -
    // theoretical`, and it must sit inside the delta window that the file
    // states. The fixture holds RAW Da [-3.5, 1.25]. The delta window is
    // -1.25 to +3.5. If the pair were read as delta directly (-3.5 to +1.25),
    // rows above +1.25 would break this test.
    let dir = scratch("window-check", true);
    let opts = ExportOptions {
        max_q: 1.0,
        include_decoys: true,
        ..ExportOptions::default()
    };
    let table = load_table(
        &dir.join("results.sage.tsv"),
        &opts,
        &Control::default(),
        0.0,
        1.0,
    )
    .unwrap();
    let tol = Params::load(&dir).unwrap().precursor_tol.unwrap();
    let (plus, minus) = (tol.plus() as f64, tol.minus() as f64);
    assert_eq!((plus, minus), (3.5, 1.25));
    let deltas: Vec<f64> = table
        .psms
        .iter()
        .map(|p| p.expmass - p.calcmass - p.isotope_error_da)
        .collect();
    let (lo, hi) = deltas
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), &d| (l.min(d), h.max(d)));
    eprintln!("delta after isotope removal: {lo} to {hi}; window -{minus} to +{plus}");
    // 1e-3 Da covers the f32 rounding of the masses in the TSV.
    assert!(lo >= -minus - 1e-3 && hi <= plus + 1e-3, "{lo} to {hi}");
    // The check must be able to fail: some row must lie above the flipped
    // window's upper edge (+1.25) and some below zero.
    assert!(
        hi > 1.25 && lo < 0.0,
        "the fixture does not separate the two readings: {lo} to {hi}"
    );
}

#[test]
fn xml_special_characters_in_names_are_escaped() {
    let dir = scratch("escape", false);
    write_tsv(
        &dir,
        &[row(
            1,
            "PEPTIDEK",
            "sp|A&B|<x>'q'",
            5,
            1,
            1,
            900.5,
            900.5,
            2,
        )],
    );
    let mzid = read(&convert_to_mzid(&dir, &ExportOptions::default()).unwrap());
    assert!(mzid.contains("accession=\"sp|A&amp;B|&lt;x&gt;&apos;q&apos;\""));
    let pep = read(&convert_to_pepxml(&dir, &ExportOptions::default()).unwrap());
    assert!(pep.contains("protein=\"sp|A&amp;B|&lt;x&gt;&apos;q&apos;\""));
}

// ── Cancel, progress, atomic write ──────────────────────────────────────

#[test]
fn a_cancelled_run_leaves_no_file_and_keeps_an_older_one() {
    let dir = scratch("cancel", true);
    let good = convert_to_mzid(&dir, &ExportOptions::default()).unwrap();
    let before = std::fs::read(&good).unwrap();

    let cancel = AtomicBool::new(true);
    let ctl = Control {
        progress: None,
        cancel: Some(&cancel),
    };
    // The flag is set from the start. The writer checks it before the first result.
    let err = convert_to_mzid_with(&dir, &ExportOptions::default(), &ctl).unwrap_err();
    assert!(is_cancelled(&err), "{err}");
    assert_eq!(
        std::fs::read(&good).unwrap(),
        before,
        "the old file is untouched"
    );
    assert!(!dir.join("results.sage.mzid.tmp").exists());
}

#[test]
fn progress_runs_from_zero_to_one_and_never_backwards() {
    let dir = scratch("progress", true);
    let seen = std::cell::RefCell::new(Vec::new());
    let cb = |f: f32| seen.borrow_mut().push(f);
    let ctl = Control {
        progress: Some(&cb),
        cancel: None,
    };
    convert_to_pepxml_with(&dir, &ExportOptions::default(), &ctl).unwrap();
    let seen = seen.into_inner();
    assert_eq!(seen.first(), Some(&0.0));
    assert_eq!(seen.last(), Some(&1.0));
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
}

#[test]
fn the_default_options_are_one_percent_spectrum_q_no_decoys() {
    let o = ExportOptions::default();
    assert_eq!(
        (o.q_source, o.max_q, o.include_decoys),
        (QSource::SpectrumQ, 0.01, false)
    );
    // A stored settings file from before a field existed must still load.
    let back: ExportOptions = serde_json::from_str("{\"max_q\": 0.05}").unwrap();
    assert_eq!(back.max_q, 0.05);
    assert_eq!(back.q_source, QSource::SpectrumQ);
    assert!(!back.include_decoys);
}

#[test]
fn a_0_14_6_tsv_with_40_columns_converts() {
    // The 0.14.6 layout has no protein_groups, num_protein_groups or
    // protein_group_q. Reading by position would shift every later field.
    let dir = scratch("old-layout", false);
    std::fs::copy(
        fixture_dir().join("results_0.14.6.json"),
        dir.join("results.json"),
    )
    .unwrap();
    let cols = "psm_id\tpeptide\tproteins\tnum_proteins\tfilename\tscannr\trank\tlabel\texpmass\tcalcmass\tcharge\t\
                peptide_len\tmissed_cleavages\tsemi_enzymatic\tisotope_error\tprecursor_ppm\tfragment_ppm\thyperscore\t\
                delta_next\tdelta_best\trt\taligned_rt\tpredicted_rt\tdelta_rt_model\tion_mobility\tpredicted_mobility\t\
                delta_mobility\tmatched_peaks\tlongest_b\tlongest_y\tlongest_y_pct\tmatched_intensity_pct\tscored_candidates\t\
                poisson\tsage_discriminant_score\tposterior_error\tspectrum_q\tpeptide_q\tprotein_q\tms2_intensity";
    assert_eq!(cols.split('\t').count(), 40);
    let line = "8411\tNQGGYGGSSSSSSYGSGR\tsp|A0A2R8Y4L2|RA1L3_HUMAN;sp|P09651|ROA1_HUMAN\t2\tB.naive_01steady-state.mzML.gz\t\
                controllerType=0 controllerNumber=1 scan=9681\t1\t1\t1694.6768\t1693.6927\t2\t18\t0\t0\t0.0\t580.8155\t2.1242871\t\
                46.042985924331035\t19.28917056680129\t0.0\t24.71024\t24.71024\t0.0\t0.999\t0.0\t0.0\t0.999\t16\t0\t16\t0.8888889\t\
                59.161163\t64459\t-13.7429724741967\t1.0929191\t-123.43977\t0.000029399658\t0.000048981092\t0.000221\t1.0";
    std::fs::write(dir.join("results.sage.tsv"), format!("{cols}\n{line}\n")).unwrap();

    let mzid = read(&convert_to_mzid(&dir, &ExportOptions::default()).unwrap());
    assert!(mzid.contains(
        "accession=\"MS:1001331\" name=\"X!Tandem:hyperscore\" value=\"46.042985924331035\""
    ));
    assert!(mzid.contains("name=\"Sage:precursor_ppm\" value=\"580.8155\""));
    // RAW Da [-500, 100] is delta -100 to +500.
    let parent = mzid
        .split("<ParentTolerance>")
        .nth(1)
        .and_then(|s| s.split("</ParentTolerance>").next())
        .unwrap();
    assert!(
        parent.contains("plus value\" value=\"500\"")
            && parent.contains("minus value\" value=\"100\"")
    );
    // `restrict` P with `c_terminal` null. Sage's default is C-terminal.
    assert!(mzid.contains("<SiteRegexp>(?&lt;=[KR])(?![P])</SiteRegexp>"));
    assert!(mzid.contains("name=\"Trypsin\""));
    // No score_type in 0.14.6, so none is written.
    assert!(!mzid.contains("score_type"));
    let pep = read(&convert_to_pepxml(&dir, &ExportOptions::default()).unwrap());
    assert!(pep.contains("spectrum=\"B.naive_01steady-state.09681.09681.2\""));
    assert!(pep.contains("search_engine_version=\"0.14.6\""));
    if xmllint().is_some() {
        for line in validate_outputs(&dir) {
            eprintln!("{line}");
        }
    }
}

// ── Validation against the published schemas ────────────────────────────

/// The path of `xmllint` if it runs here. `None` skips the test. It is not on
/// the Windows CI image.
fn xmllint() -> Option<&'static str> {
    Command::new("xmllint")
        .arg("--version")
        .output()
        .ok()
        .map(|_| "xmllint")
}

/// Run `xmllint --noout --schema` and return (success, stderr).
fn validate(xsd: &str, file: &Path) -> (bool, String) {
    let out = Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(schema_dir().join(xsd))
        .arg(file)
        .output()
        .expect("xmllint runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Validate one folder's output. mzIdentML must pass. pepXML must fail only on
/// the `search_engine` value, and must pass once that value is swapped for one
/// the schema lists. Returns the report lines.
fn validate_outputs(dir: &Path) -> Vec<String> {
    let opts = ExportOptions {
        include_decoys: true,
        ..ExportOptions::default()
    };
    let mzid = convert_to_mzid(dir, &opts).unwrap();
    let pep = convert_to_pepxml(dir, &opts).unwrap();
    let mut report = Vec::new();

    let (ok, err) = validate("mzIdentML1.1.1.xsd", &mzid);
    assert!(ok, "mzIdentML does not validate:\n{err}");
    report.push(format!("mzIdentML 1.1.1: {}", err.trim()));

    // Strict: `Sage` is not in the search_engine list, so this must fail, and
    // only there.
    let (ok, err) = validate("pepXML_v123.xsd", &pep);
    assert!(!ok, "expected the search_engine enumeration to fail");
    let errors: Vec<&str> = err.lines().filter(|l| l.contains("error")).collect();
    assert!(!errors.is_empty());
    assert!(
        errors.iter().all(|l| l.contains("search_engine")),
        "pepXML fails somewhere other than search_engine:\n{err}"
    );
    report.push(format!(
        "pepXML 1.23 strict: {} error lines, all about search_engine",
        errors.len()
    ));

    // With an engine the schema lists, the file must be fully valid.
    let swapped = dir.join("results.sage.swapped.pep.xml");
    std::fs::write(
        &swapped,
        read(&pep).replace("search_engine=\"Sage\"", "search_engine=\"X! Tandem\""),
    )
    .unwrap();
    let (ok, err) = validate("pepXML_v123.xsd", &swapped);
    assert!(ok, "pepXML does not validate with a listed engine:\n{err}");
    report.push(format!("pepXML 1.23 with a listed engine: {}", err.trim()));
    report
}

#[test]
fn the_fixture_output_validates_against_the_schemas() {
    if xmllint().is_none() {
        eprintln!("xmllint not found. Skipping schema validation.");
        return;
    }
    let dir = scratch("xmllint-fixture", true);
    for line in validate_outputs(&dir) {
        eprintln!("{line}");
    }
}

#[test]
fn hand_made_edge_cases_validate_against_the_schemas() {
    if xmllint().is_none() {
        eprintln!("xmllint not found. Skipping schema validation.");
        return;
    }
    let dir = scratch("xmllint-edge", false);
    write_tsv(
        &dir,
        &[
            row(
                1,
                "[+42.0106]-Q[-17.026548]PEPTIDEK-[+0.984]",
                "sp|A&B|<x>",
                9,
                1,
                1,
                1000.5,
                1000.5,
                2,
            ),
            row(
                2,
                "PEPTIDER",
                "rev_sp|A|A;rev_sp|B|B",
                9,
                2,
                -1,
                1000.5,
                1000.4,
                2,
            ),
            row(
                3,
                "AGAFC[+57.0215]LSEDAGLGISSTASLR",
                "sp|C|C",
                12,
                1,
                1,
                2256.162,
                2255.1548,
                3,
            ),
        ],
    );
    for line in validate_outputs(&dir) {
        eprintln!("{line}");
    }
}

/// Run on real outputs: `SAGEGUI_EXPORT_DIRS=dirA:dirB cargo test --offline
/// export_real_outputs -- --ignored --nocapture`. Each folder needs
/// `results.sage.tsv` and `results.json`. The files are copied to a scratch
/// folder first, so the originals get no new files. The scratch path is printed
/// so `xmllint` can be run by hand too.
#[test]
#[ignore]
fn export_real_outputs() {
    let dirs = std::env::var("SAGEGUI_EXPORT_DIRS").expect("set SAGEGUI_EXPORT_DIRS");
    for (n, src) in dirs.split(':').filter(|s| !s.is_empty()).enumerate() {
        let dir = std::env::temp_dir().join(format!("sagegui-export-real-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["results.sage.tsv", "results.json"] {
            std::fs::copy(Path::new(src).join(name), dir.join(name)).unwrap();
        }
        let opts = ExportOptions {
            include_decoys: true,
            max_q: 1.0,
            ..ExportOptions::default()
        };
        let started = std::time::Instant::now();
        let mzid = convert_to_mzid(&dir, &opts).unwrap();
        let pep = convert_to_pepxml(&dir, &opts).unwrap();
        eprintln!(
            "{src}\n  mzid: {} ({} bytes)\n  pepxml: {} ({} bytes)\n  converted in {:.1?}",
            mzid.display(),
            std::fs::metadata(&mzid).unwrap().len(),
            pep.display(),
            std::fs::metadata(&pep).unwrap().len(),
            started.elapsed()
        );
        // The strict schema check for pepXML fails on `search_engine` only.
        // Write a copy with a listed engine so both files can be checked fully.
        let swapped = dir.join("results.sage.swapped.pep.xml");
        std::fs::write(
            &swapped,
            read(&pep).replace("search_engine=\"Sage\"", "search_engine=\"X! Tandem\""),
        )
        .unwrap();
        eprintln!(
            "  swapped-engine copy for validation: {}",
            swapped.display()
        );
    }
}

#[test]
fn params_load_reads_the_fixture_from_disk() {
    let dir = scratch("params-disk", false);
    let p = Params::load(&dir).unwrap();
    assert_eq!(p.max_variable_mods, Some(3));
}
