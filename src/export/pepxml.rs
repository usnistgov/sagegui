//! pepXML 1.23 writer.
//!
//! One `msms_run_summary` per input file, one `spectrum_query` per spectrum
//! and precursor, one `search_hit` per PSM.
//!
//! Three rules here are easy to get wrong:
//!
//! - `mod_aminoacid_mass` is the TOTAL mass of the modified residue: the
//!   residue mass plus the delta. `C[+57.0215]` becomes `160.03069`, not
//!   `57.0215`. Terminal masses count the free terminus too: `mod_nterm_mass`
//!   is H plus the delta, `mod_cterm_mass` is OH plus the delta.
//! - `retention_time_sec` is in SECONDS. Sage writes minutes.
//! - `massdiff` is `expmass - calcmass` in Da. It includes the isotope offset
//!   when Sage picked a +1 or +2 peak. The TSV column `precursor_ppm` is a ppm
//!   value and is not used here.
//!
//! `search_engine` is written as `Sage`. The 1.23 schema lists a fixed set of
//! engines and Sage is not in it. See _dev/NOTES.md, "Downstream tools that read
//! Sage output", for what that means for strict validation.

use std::io::Write;

use super::model::{
    decimal, parse_peptide, residue_mass, round_to, scan_number, Psm, Table, C_TERM_OH, N_TERM_H,
};
use super::mzid::filter_text;
use super::params::{base_name, DeltaTol, EnzymeSpec, ModSpec, ModTarget, Params, TolUnit};
use super::xml::{num, num32, utc_now_iso, Esc};
use super::{Control, ExportOptions, CANCELLED};

/// Name written in `search_engine`. Not in the 1.23 enumeration.
pub const SEARCH_ENGINE: &str = "Sage";

/// Split a file name into (name without extension, extension with dot).
/// `run.mzML.gz` gives (`run`, `.mzML.gz`).
fn split_extension(name: &str) -> (&str, &str) {
    let lower = name.to_ascii_lowercase();
    let gz = if lower.ends_with(".gz") { 3 } else { 0 };
    let core = &name[..name.len() - gz];
    match core.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Characters the schema allows in `specificity/@cut`: `[A,C-I,K-N,P-T,VWY]`.
fn cut_ok(c: char) -> bool {
    matches!(c, 'A' | 'C'..='I' | 'K'..='N' | 'P'..='T' | 'V' | 'W' | 'Y')
}

/// The `sample_enzyme` name, from the enzyme rules.
fn enzyme_label(e: &EnzymeSpec) -> String {
    match (e.cleave_at.as_str(), e.restrict.as_str(), e.c_terminal) {
        ("", _, _) => "nonspecific".to_string(),
        ("$", _, _) => "no_cleavage".to_string(),
        ("KR" | "RK", "P", true) => "trypsin".to_string(),
        ("KR" | "RK", "", true) => "trypsin_p".to_string(),
        ("K", "P", true) => "lys-c".to_string(),
        ("K", "", true) => "lys-c_p".to_string(),
        ("R", "P", true) => "arg-c".to_string(),
        ("D", "", false) => "asp-n".to_string(),
        (cut, _, _) => format!("sage_{cut}"),
    }
}

fn write_sample_enzyme(out: &mut dyn Write, e: &EnzymeSpec) -> Result<(), String> {
    let cut: String = e.cleave_at.chars().filter(|&c| cut_ok(c)).collect();
    let none = e.cleave_at.is_empty() || e.cleave_at == "$" || cut.is_empty();
    let fidelity = if none {
        "nonspecific"
    } else if e.semi_enzymatic {
        "semispecific"
    } else {
        "specific"
    };
    wr!(
        out,
        "    <sample_enzyme name=\"{}\" fidelity=\"{fidelity}\"",
        Esc(&enzyme_label(e))
    );
    if none {
        wr!(out, "/>\n");
        return Ok(());
    }
    wr!(out, ">\n");
    let no_cut: String = e.restrict.chars().filter(|&c| cut_ok(c)).collect();
    wr!(
        out,
        "      <specificity sense=\"{}\" cut=\"{cut}\"",
        if e.c_terminal { "C" } else { "N" }
    );
    if !no_cut.is_empty() {
        wr!(out, " no_cut=\"{no_cut}\"");
    }
    wr!(out, "/>\n");
    wr!(out, "    </sample_enzyme>\n");
    Ok(())
}

/// A modification row of `search_summary`.
fn write_search_mod(out: &mut dyn Write, m: &ModSpec, variable: bool) -> Result<(), String> {
    let var = if variable { "Y" } else { "N" };
    let delta = num32(m.delta);
    // Terminus a mod targets, if any: (peptide or protein, n or c).
    let (residue, pep_term, prot_term, term_only) = match m.target {
        ModTarget::Residue(c) => (Some(c), None, None, false),
        ModTarget::PeptideN(r) => (r, Some("n"), None, r.is_none()),
        ModTarget::PeptideC(r) => (r, Some("c"), None, r.is_none()),
        ModTarget::ProteinN(r) => (r, None, Some("n"), r.is_none()),
        ModTarget::ProteinC(r) => (r, None, Some("c"), r.is_none()),
    };
    if term_only {
        // No residue: a terminal modification. `mass` counts the free terminus.
        let (terminus, group) = match (pep_term, prot_term) {
            (Some(t), _) | (None, Some(t)) => (t, if t == "n" { N_TERM_H } else { C_TERM_OH }),
            (None, None) => return Ok(()),
        };
        let protein = if prot_term.is_some() { "Y" } else { "N" };
        wr!(
            out,
            "      <terminal_modification terminus=\"{terminus}\" massdiff=\"{delta}\" mass=\"{}\" variable=\"{var}\" protein_terminus=\"{protein}\"/>\n",
            num(round_to(group + decimal(m.delta), 6))
        );
        return Ok(());
    }
    let aa = residue.unwrap_or('X');
    let total = residue_mass(aa).unwrap_or(0.0) + decimal(m.delta);
    wr!(
        out,
        "      <aminoacid_modification aminoacid=\"{aa}\" massdiff=\"{delta}\" mass=\"{}\" variable=\"{var}\"",
        num(round_to(total, 6))
    );
    if let Some(t) = pep_term {
        wr!(out, " peptide_terminus=\"{t}\"");
    }
    if let Some(t) = prot_term {
        wr!(out, " protein_terminus=\"{t}\"");
    }
    wr!(out, "/>\n");
    Ok(())
}

fn parameter(out: &mut dyn Write, name: &str, value: &str) -> Result<(), String> {
    wr!(
        out,
        "      <parameter name=\"{}\" value=\"{}\"/>\n",
        Esc(name),
        Esc(value)
    );
    Ok(())
}

/// Tolerance as three parameters. DELTA-MASS convention (observed minus
/// theoretical), from [`DeltaTol`], never the raw JSON pair.
fn tolerance_parameters(out: &mut dyn Write, what: &str, tol: &DeltaTol) -> Result<(), String> {
    let unit = match tol.unit {
        TolUnit::Da => "Da",
        TolUnit::Ppm => "ppm",
        TolUnit::Pct => "percent",
    };
    parameter(
        out,
        &format!("{what}_delta_mass_low"),
        &num32(tol.low + 0.0),
    )?;
    parameter(
        out,
        &format!("{what}_delta_mass_high"),
        &num32(tol.high + 0.0),
    )?;
    parameter(out, &format!("{what}_delta_mass_unit"), unit)
}

/// Write the whole document.
pub fn write(
    out: &mut dyn Write,
    t: &Table,
    p: &Params,
    opts: &ExportOptions,
    ctl: &Control,
) -> Result<(), String> {
    let parsed: Vec<_> = t
        .peptides
        .iter()
        .map(|e| parse_peptide(&e.peptide))
        .collect::<Result<_, _>>()?;

    // Order: file, then spectrum, then precursor, then rank. Rows arrive in
    // score order, so sort a list of indices.
    let mut order: Vec<u32> = (0..t.psms.len() as u32).collect();
    let key = |x: &Psm| {
        (
            x.file,
            scan_number(&x.native_id),
            x.native_id.clone(),
            x.charge,
            x.expmass.to_bits(),
            x.rank,
        )
    };
    order.sort_by_key(|&i| key(&t.psms[i as usize]));

    wr!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    wr!(
        out,
        "<msms_pipeline_analysis xmlns=\"http://regis-web.systemsbiology.net/pepXML\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://regis-web.systemsbiology.net/pepXML \
         https://svn.code.sf.net/p/sashimi/code/trunk/trans_proteomic_pipeline/schema/pepXML_v123.xsd\" \
         date=\"{}\" summary_xml=\"results.sage.pep.xml\">\n",
        utc_now_iso()
    );

    let mut i = 0usize;
    while i < order.len() {
        let file = t.psms[order[i] as usize].file;
        let file_name = &t.files[file as usize];
        let full = p.path_of(file_name).unwrap_or(file_name);
        let (stem_path, ext) = split_extension(full);
        let stem = base_name(stem_path);

        wr!(
            out,
            "  <msms_run_summary base_name=\"{}\" raw_data_type=\"raw\" raw_data=\"{}\">\n",
            Esc(stem_path),
            Esc(ext)
        );
        if let Some(e) = &p.enzyme {
            write_sample_enzyme(out, e)?;
        }
        wr!(
            out,
            "    <search_summary base_name=\"{}\" search_engine=\"{SEARCH_ENGINE}\"",
            Esc(stem_path)
        );
        if let Some(v) = &p.sage_version {
            wr!(out, " search_engine_version=\"{}\"", Esc(v));
        }
        wr!(
            out,
            " precursor_mass_type=\"monoisotopic\" fragment_mass_type=\"monoisotopic\" search_id=\"1\">\n"
        );
        if let Some(f) = &p.fasta {
            wr!(
                out,
                "      <search_database local_path=\"{}\" database_name=\"{}\" type=\"AA\"/>\n",
                Esc(f),
                Esc(base_name(f))
            );
        }
        if let Some(e) = &p.enzyme {
            if let Some(n) = e.missed_cleavages {
                let termini = if e.cleave_at.is_empty() || e.cleave_at == "$" {
                    0
                } else if e.semi_enzymatic {
                    1
                } else {
                    2
                };
                wr!(
                    out,
                    "      <enzymatic_search_constraint enzyme=\"{}\" max_num_internal_cleavages=\"{n}\" min_number_termini=\"{termini}\"/>\n",
                    Esc(&enzyme_label(e))
                );
            }
        }
        for m in &p.static_mods {
            write_search_mod(out, m, false)?;
        }
        for m in &p.variable_mods {
            write_search_mod(out, m, true)?;
        }
        parameter(
            out,
            "search_engine_note",
            "Sage is not in the search_engine list of pepXML 1.23. The name is written as is.",
        )?;
        if let Some(s) = &p.score_type {
            parameter(out, "score_type", s)?;
        }
        if let Some(n) = p.max_variable_mods {
            parameter(out, "max_variable_mods", &n.to_string())?;
        }
        if let Some(s) = &p.decoy_tag {
            parameter(out, "decoy_prefix", s)?;
        }
        if let Some(tol) = &p.precursor_tol {
            tolerance_parameters(out, "precursor", tol)?;
        }
        if let Some(tol) = &p.fragment_tol {
            tolerance_parameters(out, "fragment", tol)?;
        }
        parameter(
            out,
            "tolerance_convention",
            "Windows are delta mass (observed minus theoretical). results.json stores the negated and reversed pair.",
        )?;
        parameter(out, "sagegui_export_filter", &filter_text(opts))?;
        wr!(out, "    </search_summary>\n");

        // The spectra of this file.
        let mut index = 0usize;
        while i < order.len() && t.psms[order[i] as usize].file == file {
            if index & 2047 == 0 {
                if ctl.cancelled() {
                    return Err(CANCELLED.to_string());
                }
                ctl.report(0.5 + 0.5 * (i as f32 / order.len() as f32));
            }
            let head = &t.psms[order[i] as usize];
            let mut j = i;
            while j < order.len() {
                let x = &t.psms[order[j] as usize];
                if x.file != file
                    || x.native_id != head.native_id
                    || x.charge != head.charge
                    || x.expmass.to_bits() != head.expmass.to_bits()
                {
                    break;
                }
                j += 1;
            }
            index += 1;
            let scan = scan_number(&head.native_id).unwrap_or(0);
            wr!(
                out,
                "    <spectrum_query spectrum=\"{stem}.{scan:05}.{scan:05}.{}\" spectrumNativeID=\"{}\" \
                 start_scan=\"{scan}\" end_scan=\"{scan}\" precursor_neutral_mass=\"{}\" \
                 assumed_charge=\"{}\" index=\"{index}\"",
                head.charge,
                Esc(&head.native_id),
                num(head.expmass),
                head.charge
            );
            if head.rt_min.is_finite() {
                // Sage writes minutes. pepXML wants seconds.
                wr!(
                    out,
                    " retention_time_sec=\"{}\"",
                    num(round_to(head.rt_min * 60.0, 4))
                );
            }
            wr!(out, ">\n");
            wr!(out, "      <search_result>\n");
            for &k in &order[i..j] {
                let psm = &t.psms[k as usize];
                let entry = &t.peptides[psm.pep as usize];
                let pp = &parsed[psm.pep as usize];
                wr!(
                    out,
                    "        <search_hit hit_rank=\"{}\" peptide=\"{}\" protein=\"{}\" num_tot_proteins=\"{}\" \
                     calc_neutral_pep_mass=\"{}\" massdiff=\"{}\"",
                    psm.rank,
                    Esc(&pp.sequence),
                    Esc(&t.proteins[entry.proteins[0] as usize]),
                    entry.proteins.len(),
                    num(psm.calcmass),
                    num(round_to(psm.expmass - psm.calcmass, 6))
                );
                if let Some(n) = psm.matched_peaks {
                    wr!(out, " num_matched_ions=\"{n}\"");
                }
                if let Some(n) = psm.missed_cleavages {
                    wr!(out, " num_missed_cleavages=\"{n}\"");
                }
                wr!(out, ">\n");
                for &alt in &entry.proteins[1..] {
                    wr!(
                        out,
                        "          <alternative_protein protein=\"{}\"/>\n",
                        Esc(&t.proteins[alt as usize])
                    );
                }
                if !pp.mods.is_empty() {
                    let mut nterm = None;
                    let mut cterm = None;
                    let mut residues = Vec::new();
                    for m in &pp.mods {
                        match (m.position, m.residue) {
                            (0, None) => nterm = Some(N_TERM_H + m.delta),
                            (_, None) => cterm = Some(C_TERM_OH + m.delta),
                            (pos, Some(r)) => {
                                residues.push((pos, residue_mass(r).unwrap_or(0.0) + m.delta))
                            }
                        }
                    }
                    wr!(out, "          <modification_info");
                    if let Some(m) = nterm {
                        wr!(out, " mod_nterm_mass=\"{}\"", num(round_to(m, 6)));
                    }
                    if let Some(m) = cterm {
                        wr!(out, " mod_cterm_mass=\"{}\"", num(round_to(m, 6)));
                    }
                    if residues.is_empty() {
                        wr!(out, "/>\n");
                    } else {
                        wr!(out, ">\n");
                        for (pos, total) in residues {
                            wr!(
                                out,
                                "            <mod_aminoacid_mass position=\"{pos}\" mass=\"{}\"/>\n",
                                num(round_to(total, 6))
                            );
                        }
                        wr!(out, "          </modification_info>\n");
                    }
                }
                let mut score = |name: &str, v: f64| -> Result<(), String> {
                    if v.is_finite() {
                        wr!(
                            out,
                            "          <search_score name=\"{name}\" value=\"{}\"/>\n",
                            num(v)
                        );
                    }
                    Ok(())
                };
                score("hyperscore", psm.hyperscore)?;
                // Sage's `delta_next` is the hyperscore minus the next best
                // hyperscore (0 when there is no other candidate). Adding it
                // back gives the X!Tandem `nextscore`. Both are written.
                score("nextscore", psm.hyperscore - psm.delta_next)?;
                score("delta_next", psm.delta_next)?;
                score("spectrum_q", psm.spectrum_q)?;
                score("peptide_q", psm.peptide_q)?;
                score("protein_q", psm.protein_q)?;
                score("sage_discriminant_score", psm.discriminant)?;
                // log10 of the posterior error probability, as Sage writes it.
                score("posterior_error_log10", psm.posterior_error_log10)?;
                wr!(out, "        </search_hit>\n");
            }
            wr!(out, "      </search_result>\n");
            wr!(out, "    </spectrum_query>\n");
            i = j;
        }
        wr!(out, "  </msms_run_summary>\n");
    }
    wr!(out, "</msms_pipeline_analysis>\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_split_into_stem_and_extension() {
        assert_eq!(split_extension("run.mzML.gz"), ("run", ".mzML.gz"));
        assert_eq!(split_extension("c:/a/run.mzML"), ("c:/a/run", ".mzML"));
        assert_eq!(split_extension("a.b.mzML"), ("a.b", ".mzML"));
        assert_eq!(split_extension("noext"), ("noext", ""));
    }

    #[test]
    fn a_carbamidomethyl_search_row_carries_the_total_mass() {
        let mut buf = Vec::new();
        let m = ModSpec {
            target: ModTarget::Residue('C'),
            delta: 57.0215,
        };
        write_search_mod(&mut buf, &m, false).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("aminoacid=\"C\""), "{text}");
        assert!(text.contains("massdiff=\"57.0215\""), "{text}");
        assert!(text.contains("mass=\"160.03069\""), "{text}");
        assert!(text.contains("variable=\"N\""), "{text}");
    }

    #[test]
    fn a_terminal_search_row_counts_the_free_terminus() {
        let mut buf = Vec::new();
        let m = ModSpec {
            target: ModTarget::ProteinN(None),
            delta: 42.0106,
        };
        write_search_mod(&mut buf, &m, true).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("<terminal_modification terminus=\"n\""),
            "{text}"
        );
        assert!(text.contains("mass=\"43.018425\""), "{text}");
        assert!(text.contains("variable=\"Y\""), "{text}");
        assert!(text.contains("protein_terminus=\"Y\""), "{text}");
    }

    #[test]
    fn cut_residues_follow_the_schema_pattern() {
        assert!(cut_ok('K') && cut_ok('R') && cut_ok('D'));
        assert!(!cut_ok('B') && !cut_ok('U') && !cut_ok('X') && !cut_ok('$'));
    }
}
