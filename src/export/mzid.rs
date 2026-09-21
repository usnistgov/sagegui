//! mzIdentML 1.1.1 writer.
//!
//! Element order is fixed by the schema. `SequenceCollection` comes before
//! the results, so the peptide, protein and evidence tables are built first
//! from the whole [`Table`].
//!
//! Every ID is a counter (`DBSeq_1`, `Pep_1`, `PE_1`, `SIR_1`, `SII_1`). The
//! protein name goes in the `accession` attribute, exactly as Sage wrote it.
//! A decoy keeps its tag, for example `rev_sp|P06727|APOA4_HUMAN`.
//!
//! CV accessions were checked against psi-ms.obo (data-version 4.2.2) and the
//! Unit Ontology on 2026-09-21. The list is in NOTES.md.
//!
//! Modifications are written as `MS:1001460` "unknown modification" with the
//! mass. A mass alone does not name a modification (57.0215 fits several
//! Unimod entries), and we do not guess.

use std::collections::HashMap;
use std::io::Write;

use sage_core::ion_series::Kind;

use super::model::{mz, parse_peptide, scan_number, ParsedPeptide, Table};
use super::params::{base_name, DeltaTol, EnzymeSpec, ModSpec, ModTarget, Params, TolUnit};
use super::xml::{num, num32, to_uri, utc_now_iso, Esc};
use super::{Control, ExportOptions, QSource, CANCELLED};

const UO_DALTON: (&str, &str) = ("UO:0000221", "dalton");
const UO_PPM: (&str, &str) = ("UO:0000169", "parts per million");
const UO_PERCENT: (&str, &str) = ("UO:0000187", "percent");
const UO_MINUTE: (&str, &str) = ("UO:0000031", "minute");

/// One `cvParam` line. `unit` is a `(UO accession, UO name)` pair.
fn cv(
    out: &mut dyn Write,
    indent: usize,
    cv_ref: &str,
    acc: &str,
    name: &str,
    value: Option<&str>,
    unit: Option<(&str, &str)>,
) -> Result<(), String> {
    let pad = " ".repeat(indent);
    wr!(
        out,
        "{pad}<cvParam cvRef=\"{cv_ref}\" accession=\"{acc}\" name=\"{}\"",
        Esc(name)
    );
    if let Some(v) = value {
        wr!(out, " value=\"{}\"", Esc(v));
    }
    if let Some((ua, un)) = unit {
        wr!(
            out,
            " unitAccession=\"{ua}\" unitName=\"{un}\" unitCvRef=\"UO\""
        );
    }
    wr!(out, "/>\n");
    Ok(())
}

/// One `userParam` line.
fn user(
    out: &mut dyn Write,
    indent: usize,
    name: &str,
    value: &str,
    unit: Option<(&str, &str)>,
) -> Result<(), String> {
    let pad = " ".repeat(indent);
    wr!(
        out,
        "{pad}<userParam name=\"{}\" value=\"{}\"",
        Esc(name),
        Esc(value)
    );
    if let Some((ua, un)) = unit {
        wr!(
            out,
            " unitAccession=\"{ua}\" unitName=\"{un}\" unitCvRef=\"UO\""
        );
    }
    wr!(out, "/>\n");
    Ok(())
}

fn ms(
    out: &mut dyn Write,
    indent: usize,
    acc: &str,
    name: &str,
    value: Option<&str>,
    unit: Option<(&str, &str)>,
) -> Result<(), String> {
    cv(out, indent, "MS", acc, name, value, unit)
}

/// The SpectrumIDFormat term for a nativeID, decided from its text.
fn id_format(native_id: &str) -> (&'static str, &'static str) {
    if native_id.starts_with("controllerType=") {
        ("MS:1000768", "Thermo nativeID format")
    } else if native_id.starts_with("index=") {
        ("MS:1000774", "multiple peak list nativeID format")
    } else if native_id.starts_with("frame=") {
        ("MS:1002818", "Bruker TDF nativeID format")
    } else if native_id.starts_with("spectrum=") {
        ("MS:1000777", "spectrum identifier nativeID format")
    } else if native_id.starts_with("scan=") {
        ("MS:1000776", "scan number only nativeID format")
    } else {
        ("MS:1000824", "no nativeID format")
    }
}

/// The file-format term for an input file, from its extension. `None` for a
/// format with no term we verified. The element is optional.
fn file_format(name: &str) -> Option<(&'static str, &'static str)> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".gz").unwrap_or(&lower);
    if stem.ends_with(".mzml") {
        Some(("MS:1000584", "mzML format"))
    } else if stem.ends_with(".mgf") {
        Some(("MS:1001062", "Mascot MGF format"))
    } else if stem.ends_with(".mzxml") {
        Some(("MS:1000566", "ISB mzXML format"))
    } else {
        None
    }
}

/// Regular expression for the enzyme, in the form the PSI CV uses:
/// `(?<=[KR])(?!P)` for a C-terminal enzyme, `(?=[D])` for an N-terminal one.
/// `None` when there is no enzyme or no cleavage.
///
/// Sage does not cut where the next residue is in `restrict`. That is the
/// `(?!...)` part. For an N-terminal enzyme the next residue is the cut
/// residue itself, and the look-ahead `(?![P])` at the same spot checks it.
pub fn enzyme_regexp(e: &EnzymeSpec) -> Option<String> {
    if e.cleave_at.is_empty() || e.cleave_at == "$" {
        return None;
    }
    let cut = &e.cleave_at;
    let guard = if e.restrict.is_empty() {
        String::new()
    } else {
        format!("(?![{}])", e.restrict)
    };
    Some(if e.c_terminal {
        format!("(?<=[{cut}]){guard}")
    } else {
        format!("(?=[{cut}]){guard}")
    })
}

/// The CV name for an enzyme, for the few that match exactly. Anything else
/// has only the regular expression.
fn enzyme_name(e: &EnzymeSpec) -> Option<(&'static str, &'static str)> {
    let sorted = |s: &str| {
        let mut c: Vec<char> = s.chars().collect();
        c.sort_unstable();
        c.into_iter().collect::<String>()
    };
    if e.cleave_at.is_empty() {
        return Some(("MS:1001956", "unspecific cleavage"));
    }
    if e.cleave_at == "$" {
        return Some(("MS:1001955", "no cleavage"));
    }
    match (
        sorted(&e.cleave_at).as_str(),
        sorted(&e.restrict).as_str(),
        e.c_terminal,
    ) {
        ("KR", "P", true) => Some(("MS:1001251", "Trypsin")),
        ("KR", "", true) => Some(("MS:1001313", "Trypsin/P")),
        ("K", "P", true) => Some(("MS:1001309", "Lys-C")),
        ("K", "", true) => Some(("MS:1001310", "Lys-C/P")),
        ("R", "P", true) => Some(("MS:1001303", "Arg-C")),
        ("D", "", false) => Some(("MS:1001304", "Asp-N")),
        _ => None,
    }
}

fn ion_term(k: Kind) -> (&'static str, &'static str) {
    match k {
        Kind::A => ("MS:1001108", "param: a ion"),
        Kind::B => ("MS:1001118", "param: b ion"),
        Kind::C => ("MS:1001119", "param: c ion"),
        Kind::X => ("MS:1001261", "param: x ion"),
        Kind::Y => ("MS:1001262", "param: y ion"),
        Kind::Z => ("MS:1001263", "param: z ion"),
    }
}

/// Escape the characters that are special in a regular expression.
fn regex_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub(super) fn write_tolerance(
    out: &mut dyn Write,
    tag: &str,
    tol: &DeltaTol,
) -> Result<(), String> {
    let unit = match tol.unit {
        TolUnit::Da => UO_DALTON,
        TolUnit::Ppm => UO_PPM,
        TolUnit::Pct => UO_PERCENT,
    };
    wr!(out, "      <{tag}>\n");
    // DELTA-MASS convention (observed minus theoretical). `plus()` and
    // `minus()` come from the delta window, never from the raw JSON pair.
    ms(
        out,
        8,
        "MS:1001412",
        "search tolerance plus value",
        Some(&num32(tol.plus())),
        Some(unit),
    )?;
    ms(
        out,
        8,
        "MS:1001413",
        "search tolerance minus value",
        Some(&num32(tol.minus())),
        Some(unit),
    )?;
    wr!(out, "      </{tag}>\n");
    Ok(())
}

fn write_search_mod(out: &mut dyn Write, m: &ModSpec, fixed: bool) -> Result<(), String> {
    // (residue text, terminal rule)
    let (residue, rule) = match m.target {
        ModTarget::Residue(c) => (c.to_string(), None),
        ModTarget::PeptideN(r) => (
            r.map_or(".".to_string(), |c| c.to_string()),
            Some(("MS:1001189", "modification specificity peptide N-term")),
        ),
        ModTarget::PeptideC(r) => (
            r.map_or(".".to_string(), |c| c.to_string()),
            Some(("MS:1001190", "modification specificity peptide C-term")),
        ),
        ModTarget::ProteinN(r) => (
            r.map_or(".".to_string(), |c| c.to_string()),
            Some(("MS:1002057", "modification specificity protein N-term")),
        ),
        ModTarget::ProteinC(r) => (
            r.map_or(".".to_string(), |c| c.to_string()),
            Some(("MS:1002058", "modification specificity protein C-term")),
        ),
    };
    wr!(
        out,
        "        <SearchModification fixedMod=\"{fixed}\" massDelta=\"{}\" residues=\"{residue}\">\n",
        num32(m.delta)
    );
    if let Some((acc, name)) = rule {
        wr!(out, "          <SpecificityRules>\n");
        ms(out, 12, acc, name, None, None)?;
        wr!(out, "          </SpecificityRules>\n");
    }
    ms(out, 10, "MS:1001460", "unknown modification", None, None)?;
    wr!(out, "        </SearchModification>\n");
    Ok(())
}

/// Write the whole document. See the module docs for the order.
pub fn write(
    out: &mut dyn Write,
    t: &Table,
    p: &Params,
    opts: &ExportOptions,
    ctl: &Control,
) -> Result<(), String> {
    // ── Tables that SequenceCollection needs ──────────────────────────────
    let parsed: Vec<ParsedPeptide> = t
        .peptides
        .iter()
        .map(|e| parse_peptide(&e.peptide))
        .collect::<Result<_, _>>()?;

    // One <Peptide> per distinct inline string. A target and a decoy that
    // read the same share it.
    let mut pep_of_entry: Vec<usize> = Vec::with_capacity(t.peptides.len());
    let mut pep_first_entry: Vec<usize> = Vec::new();
    let mut by_text: HashMap<&str, usize> = HashMap::new();
    for (i, e) in t.peptides.iter().enumerate() {
        let next = pep_first_entry.len();
        let idx = *by_text.entry(e.peptide.as_str()).or_insert(next);
        if idx == next {
            pep_first_entry.push(i);
        }
        pep_of_entry.push(idx);
    }

    // One <PeptideEvidence> per (peptide, protein, decoy flag).
    let mut pe_ids: HashMap<(usize, u32, bool), usize> = HashMap::new();
    let mut pe_list: Vec<(usize, u32, bool)> = Vec::new();
    let mut evidence_of_entry: Vec<Vec<usize>> = Vec::with_capacity(t.peptides.len());
    for (i, e) in t.peptides.iter().enumerate() {
        let mut ids = Vec::with_capacity(e.proteins.len());
        for &prot in &e.proteins {
            let key = (pep_of_entry[i], prot, e.decoy);
            let next = pe_list.len();
            let id = *pe_ids.entry(key).or_insert(next);
            if id == next {
                pe_list.push(key);
            }
            ids.push(id);
        }
        evidence_of_entry.push(ids);
    }

    // Spectra data: the first nativeID of each file decides its ID format.
    let mut first_id: Vec<Option<&str>> = vec![None; t.files.len()];
    for psm in &t.psms {
        let slot = &mut first_id[psm.file as usize];
        if slot.is_none() {
            *slot = Some(&psm.native_id);
        }
    }

    let now = utc_now_iso();
    let sage_version = p.sage_version.as_deref();

    // ── Header, CVs, software ─────────────────────────────────────────────
    wr!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    wr!(
        out,
        "<MzIdentML xmlns=\"http://psidev.info/psi/pi/mzIdentML/1.1.1\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://psidev.info/psi/pi/mzIdentML/1.1.1 \
         http://psidev.info/files/mzIdentML1.1.1.xsd\" \
         id=\"SageGUI_export\" version=\"1.1.1\" creationDate=\"{now}\">\n"
    );
    wr!(out, "  <cvList>\n");
    wr!(
        out,
        "    <cv id=\"MS\" fullName=\"PSI-MS\" version=\"4.2.2\" \
         uri=\"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo\"/>\n"
    );
    wr!(
        out,
        "    <cv id=\"UO\" fullName=\"UNIT-ONTOLOGY\" \
         uri=\"https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/uo.obo\"/>\n"
    );
    wr!(out, "  </cvList>\n");

    wr!(out, "  <AnalysisSoftwareList>\n");
    wr!(out, "    <AnalysisSoftware id=\"AS_Sage\" name=\"Sage\"");
    if let Some(v) = sage_version {
        wr!(out, " version=\"{}\"", Esc(v));
    }
    wr!(out, " uri=\"https://github.com/lazear/sage\">\n");
    wr!(out, "      <SoftwareName>\n");
    ms(out, 8, "MS:1004007", "Sage", None, None)?;
    wr!(out, "      </SoftwareName>\n");
    wr!(out, "    </AnalysisSoftware>\n");
    wr!(
        out,
        "    <AnalysisSoftware id=\"AS_SageGUI\" name=\"SageGUI\" version=\"{}\">\n",
        env!("CARGO_PKG_VERSION")
    );
    wr!(out, "      <SoftwareName>\n");
    user(out, 8, "SageGUI mzIdentML converter", "", None)?;
    wr!(out, "      </SoftwareName>\n");
    wr!(out, "    </AnalysisSoftware>\n");
    wr!(out, "  </AnalysisSoftwareList>\n");

    // ── SequenceCollection ────────────────────────────────────────────────
    wr!(out, "  <SequenceCollection>\n");
    for (i, name) in t.proteins.iter().enumerate() {
        wr!(
            out,
            "    <DBSequence id=\"DBSeq_{}\" accession=\"{}\" searchDatabase_ref=\"SDB_1\"/>\n",
            i + 1,
            Esc(name)
        );
    }
    for (i, &entry) in pep_first_entry.iter().enumerate() {
        let pp = &parsed[entry];
        wr!(out, "    <Peptide id=\"Pep_{}\">\n", i + 1);
        wr!(
            out,
            "      <PeptideSequence>{}</PeptideSequence>\n",
            Esc(&pp.sequence)
        );
        for m in &pp.mods {
            wr!(out, "      <Modification location=\"{}\"", m.position);
            if let Some(r) = m.residue {
                wr!(out, " residues=\"{r}\"");
            }
            wr!(out, " monoisotopicMassDelta=\"{}\">\n", num(m.delta));
            ms(out, 8, "MS:1001460", "unknown modification", None, None)?;
            wr!(out, "      </Modification>\n");
        }
        wr!(out, "    </Peptide>\n");
    }
    for (i, &(pep, prot, decoy)) in pe_list.iter().enumerate() {
        wr!(
            out,
            "    <PeptideEvidence id=\"PE_{}\" peptide_ref=\"Pep_{}\" dBSequence_ref=\"DBSeq_{}\" isDecoy=\"{decoy}\"/>\n",
            i + 1,
            pep + 1,
            prot + 1
        );
    }
    wr!(out, "  </SequenceCollection>\n");

    // ── AnalysisCollection ────────────────────────────────────────────────
    wr!(out, "  <AnalysisCollection>\n");
    wr!(
        out,
        "    <SpectrumIdentification id=\"SI_1\" spectrumIdentificationProtocol_ref=\"SIP_1\" \
         spectrumIdentificationList_ref=\"SIL_1\" activityDate=\"{now}\">\n"
    );
    for i in 0..t.files.len() {
        wr!(
            out,
            "      <InputSpectra spectraData_ref=\"SD_{}\"/>\n",
            i + 1
        );
    }
    wr!(
        out,
        "      <SearchDatabaseRef searchDatabase_ref=\"SDB_1\"/>\n"
    );
    wr!(out, "    </SpectrumIdentification>\n");
    wr!(out, "  </AnalysisCollection>\n");

    // ── AnalysisProtocolCollection ────────────────────────────────────────
    wr!(out, "  <AnalysisProtocolCollection>\n");
    wr!(
        out,
        "    <SpectrumIdentificationProtocol id=\"SIP_1\" analysisSoftware_ref=\"AS_Sage\">\n"
    );
    wr!(out, "      <SearchType>\n");
    ms(out, 8, "MS:1001083", "ms-ms search", None, None)?;
    wr!(out, "      </SearchType>\n");

    wr!(out, "      <AdditionalSearchParams>\n");
    ms(out, 8, "MS:1001211", "parent mass type mono", None, None)?;
    ms(out, 8, "MS:1001256", "fragment mass type mono", None, None)?;
    for &k in &p.ion_kinds {
        let (acc, name) = ion_term(k);
        ms(out, 8, acc, name, None, None)?;
    }
    if let Some(n) = p.max_variable_mods {
        user(out, 8, "max_variable_mods", &n.to_string(), None)?;
    }
    if let Some((lo, hi)) = p.isotope_errors {
        user(out, 8, "isotope_errors", &format!("{lo},{hi}"), None)?;
    }
    if let Some((lo, hi)) = p.precursor_charge {
        user(out, 8, "precursor_charge", &format!("{lo},{hi}"), None)?;
    }
    if let Some(e) = &p.enzyme {
        if let Some(n) = e.min_len {
            user(out, 8, "peptide_min_length", &n.to_string(), None)?;
        }
        if let Some(n) = e.max_len {
            user(out, 8, "peptide_max_length", &n.to_string(), None)?;
        }
    }
    if let Some(n) = p.report_psms {
        user(out, 8, "report_psms", &n.to_string(), None)?;
    }
    if let Some(s) = &p.score_type {
        user(out, 8, "score_type", s, None)?;
    }
    if let Some(s) = &p.decoy_tag {
        user(out, 8, "decoy_tag", s, None)?;
    }
    user(
        out,
        8,
        "tolerance convention",
        "Tolerances are delta-mass windows (observed minus theoretical). \
         results.json stores the negated and reversed pair.",
        None,
    )?;
    user(out, 8, "SageGUI export filter", &filter_text(opts), None)?;
    wr!(out, "      </AdditionalSearchParams>\n");

    if !p.static_mods.is_empty() || !p.variable_mods.is_empty() {
        wr!(out, "      <ModificationParams>\n");
        for m in &p.static_mods {
            write_search_mod(out, m, true)?;
        }
        for m in &p.variable_mods {
            write_search_mod(out, m, false)?;
        }
        wr!(out, "      </ModificationParams>\n");
    }

    if let Some(e) = &p.enzyme {
        wr!(out, "      <Enzymes independent=\"false\">\n");
        wr!(out, "        <Enzyme id=\"ENZ_1\"");
        if let Some(n) = e.missed_cleavages {
            wr!(out, " missedCleavages=\"{n}\"");
        }
        wr!(out, " semiSpecific=\"{}\">\n", e.semi_enzymatic);
        if let Some(re) = enzyme_regexp(e) {
            wr!(out, "          <SiteRegexp>{}</SiteRegexp>\n", Esc(&re));
        }
        if let Some((acc, name)) = enzyme_name(e) {
            wr!(out, "          <EnzymeName>\n");
            ms(out, 12, acc, name, None, None)?;
            wr!(out, "          </EnzymeName>\n");
        }
        wr!(out, "        </Enzyme>\n");
        wr!(out, "      </Enzymes>\n");
    }

    if let Some(tol) = &p.fragment_tol {
        write_tolerance(out, "FragmentTolerance", tol)?;
    }
    if let Some(tol) = &p.precursor_tol {
        write_tolerance(out, "ParentTolerance", tol)?;
    }

    wr!(out, "      <Threshold>\n");
    if opts.max_q >= 1.0 {
        ms(out, 8, "MS:1001494", "no threshold", None, None)?;
    } else {
        let (acc, name) = match opts.q_source {
            QSource::SpectrumQ => ("MS:1002260", "PSM:FDR threshold"),
            QSource::PeptideQ => ("MS:1001448", "pep:FDR threshold"),
            QSource::ProteinQ => ("MS:1001447", "prot:FDR threshold"),
        };
        ms(out, 8, acc, name, Some(&num(opts.max_q)), None)?;
    }
    wr!(out, "      </Threshold>\n");
    wr!(out, "    </SpectrumIdentificationProtocol>\n");
    wr!(out, "  </AnalysisProtocolCollection>\n");

    // ── DataCollection: Inputs ────────────────────────────────────────────
    wr!(out, "  <DataCollection>\n");
    wr!(out, "    <Inputs>\n");
    let fasta = p.fasta.as_deref();
    wr!(
        out,
        "      <SearchDatabase id=\"SDB_1\" location=\"{}\"",
        Esc(&fasta.map_or("unknown".to_string(), to_uri))
    );
    if let Some(f) = fasta {
        wr!(out, " name=\"{}\"", Esc(base_name(f)));
    }
    wr!(out, ">\n");
    wr!(out, "        <FileFormat>\n");
    ms(out, 10, "MS:1001348", "FASTA format", None, None)?;
    wr!(out, "        </FileFormat>\n");
    wr!(out, "        <DatabaseName>\n");
    user(out, 10, fasta.map_or("unknown", base_name), "", None)?;
    wr!(out, "        </DatabaseName>\n");
    if p.generate_decoys == Some(true) {
        ms(
            out,
            8,
            "MS:1001197",
            "DB composition target+decoy",
            None,
            None,
        )?;
        ms(out, 8, "MS:1001195", "decoy DB type reverse", None, None)?;
        if let Some(tag) = p.decoy_tag.as_deref().filter(|s| !s.is_empty()) {
            ms(
                out,
                8,
                "MS:1001283",
                "decoy DB accession regexp",
                Some(&format!("^{}", regex_escape(tag))),
                None,
            )?;
        }
    }
    wr!(out, "      </SearchDatabase>\n");

    for (i, file) in t.files.iter().enumerate() {
        let full = p.path_of(file).unwrap_or(file);
        wr!(
            out,
            "      <SpectraData id=\"SD_{}\" name=\"{}\" location=\"{}\">\n",
            i + 1,
            Esc(file),
            Esc(&to_uri(full))
        );
        if let Some((acc, name)) = file_format(file) {
            wr!(out, "        <FileFormat>\n");
            ms(out, 10, acc, name, None, None)?;
            wr!(out, "        </FileFormat>\n");
        }
        let (acc, name) = id_format(first_id[i].unwrap_or(""));
        wr!(out, "        <SpectrumIDFormat>\n");
        ms(out, 10, acc, name, None, None)?;
        wr!(out, "        </SpectrumIDFormat>\n");
        wr!(out, "      </SpectraData>\n");
    }
    wr!(out, "    </Inputs>\n");

    // ── DataCollection: AnalysisData ──────────────────────────────────────
    wr!(out, "    <AnalysisData>\n");
    wr!(out, "      <SpectrumIdentificationList id=\"SIL_1\">\n");

    // Group PSMs of one spectrum into one result. Rows are in score order,
    // not spectrum order, so sort a list of indices. The sort key is the file,
    // then the scan number, then the ID text, then the rank.
    let mut order: Vec<u32> = (0..t.psms.len() as u32).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (&t.psms[a as usize], &t.psms[b as usize]);
        (a.file, scan_number(&a.native_id), &a.native_id, a.rank).cmp(&(
            b.file,
            scan_number(&b.native_id),
            &b.native_id,
            b.rank,
        ))
    });

    let mut sir_n = 0usize;
    let mut sii_n = 0usize;
    let mut i = 0usize;
    while i < order.len() {
        if sir_n & 2047 == 0 {
            if ctl.cancelled() {
                return Err(CANCELLED.to_string());
            }
            ctl.report(0.5 + 0.5 * (i as f32 / order.len() as f32));
        }
        let head = &t.psms[order[i] as usize];
        let mut j = i;
        while j < order.len() {
            let x = &t.psms[order[j] as usize];
            if x.file != head.file || x.native_id != head.native_id {
                break;
            }
            j += 1;
        }
        sir_n += 1;
        wr!(
            out,
            "        <SpectrumIdentificationResult id=\"SIR_{sir_n}\" spectrumID=\"{}\" spectraData_ref=\"SD_{}\">\n",
            Esc(&head.native_id),
            head.file + 1
        );
        for &k in &order[i..j] {
            let psm = &t.psms[k as usize];
            sii_n += 1;
            let entry = psm.pep as usize;
            wr!(
                out,
                "          <SpectrumIdentificationItem id=\"SII_{sii_n}\" chargeState=\"{}\" \
                 experimentalMassToCharge=\"{}\" calculatedMassToCharge=\"{}\" peptide_ref=\"Pep_{}\" \
                 rank=\"{}\" passThreshold=\"true\">\n",
                psm.charge,
                num(mz(psm.expmass, psm.charge)),
                num(mz(psm.calcmass, psm.charge)),
                pep_of_entry[entry] + 1,
                psm.rank
            );
            for &pe in &evidence_of_entry[entry] {
                wr!(
                    out,
                    "            <PeptideEvidenceRef peptideEvidence_ref=\"PE_{}\"/>\n",
                    pe + 1
                );
            }
            let f = |v: f64| v.is_finite().then(|| num(v));
            ms(
                out,
                12,
                "MS:1001331",
                "X!Tandem:hyperscore",
                f(psm.hyperscore).as_deref(),
                None,
            )?;
            for (acc, name, v) in [
                ("MS:1002354", "PSM-level q-value", psm.spectrum_q),
                (
                    "MS:1001868",
                    "distinct peptide-level q-value",
                    psm.peptide_q,
                ),
            ] {
                if let Some(v) = f(v) {
                    ms(out, 12, acc, name, Some(&v), None)?;
                }
            }
            if let Some(n) = psm.matched_peaks {
                ms(
                    out,
                    12,
                    "MS:1001121",
                    "number of matched peaks",
                    Some(&n.to_string()),
                    None,
                )?;
            }
            ms(
                out,
                12,
                "MS:1001117",
                "theoretical neutral mass",
                Some(&num(psm.calcmass)),
                Some(UO_DALTON),
            )?;
            if let Some(v) = f(psm.protein_q) {
                user(out, 12, "Sage:protein_q", &v, None)?;
            }
            if let Some(v) = f(psm.delta_next) {
                user(out, 12, "Sage:delta_next", &v, None)?;
            }
            if let Some(v) = f(psm.discriminant) {
                user(out, 12, "Sage:sage_discriminant_score", &v, None)?;
            }
            if let Some(v) = f(psm.posterior_error_log10) {
                user(out, 12, "Sage:posterior_error_log10", &v, None)?;
            }
            if let Some(v) = f(psm.delta_ppm) {
                // Sage's `precursor_ppm` column. It is the mass error in ppm
                // after the isotope error is removed.
                user(out, 12, "Sage:precursor_ppm", &v, Some(UO_PPM))?;
            }
            if let Some(v) = f(psm.isotope_error_da) {
                user(out, 12, "Sage:isotope_error", &v, Some(UO_DALTON))?;
            }
            if let Some(n) = psm.missed_cleavages {
                user(out, 12, "Sage:missed_cleavages", &n.to_string(), None)?;
            }
            wr!(out, "          </SpectrumIdentificationItem>\n");
        }
        // Scan start time is a property of the spectrum. Sage writes minutes.
        if head.rt_min.is_finite() {
            ms(
                out,
                10,
                "MS:1000016",
                "scan start time",
                Some(&num(head.rt_min)),
                Some(UO_MINUTE),
            )?;
        }
        wr!(out, "        </SpectrumIdentificationResult>\n");
        i = j;
    }
    wr!(out, "      </SpectrumIdentificationList>\n");
    wr!(out, "    </AnalysisData>\n");
    wr!(out, "  </DataCollection>\n");
    wr!(out, "</MzIdentML>\n");
    Ok(())
}

/// One line that says what the filter was. Written as provenance.
pub fn filter_text(opts: &ExportOptions) -> String {
    let col = match opts.q_source {
        QSource::SpectrumQ => "spectrum_q",
        QSource::PeptideQ => "peptide_q",
        QSource::ProteinQ => "protein_q",
    };
    format!(
        "{col} <= {}; decoys {}",
        num(opts.max_q),
        if opts.include_decoys {
            "included"
        } else {
            "excluded"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(cleave: &str, restrict: &str, c_terminal: bool) -> EnzymeSpec {
        EnzymeSpec {
            cleave_at: cleave.into(),
            restrict: restrict.into(),
            c_terminal,
            semi_enzymatic: false,
            missed_cleavages: Some(2),
            min_len: None,
            max_len: None,
        }
    }

    #[test]
    fn enzyme_regexp_matches_the_psi_cv_forms() {
        assert_eq!(
            enzyme_regexp(&spec("KR", "P", true)).as_deref(),
            Some("(?<=[KR])(?![P])")
        );
        assert_eq!(
            enzyme_regexp(&spec("KR", "", true)).as_deref(),
            Some("(?<=[KR])")
        );
        assert_eq!(
            enzyme_regexp(&spec("D", "", false)).as_deref(),
            Some("(?=[D])")
        );
        assert_eq!(enzyme_regexp(&spec("$", "", true)), None);
        assert_eq!(enzyme_regexp(&spec("", "", true)), None);
    }

    #[test]
    fn enzyme_names_are_only_given_for_exact_matches() {
        assert_eq!(enzyme_name(&spec("KR", "P", true)).unwrap().1, "Trypsin");
        assert_eq!(enzyme_name(&spec("RK", "", true)).unwrap().1, "Trypsin/P");
        assert_eq!(enzyme_name(&spec("K", "P", true)).unwrap().1, "Lys-C");
        assert_eq!(enzyme_name(&spec("D", "", false)).unwrap().1, "Asp-N");
        assert_eq!(enzyme_name(&spec("$", "", true)).unwrap().1, "no cleavage");
        assert_eq!(
            enzyme_name(&spec("", "", true)).unwrap().1,
            "unspecific cleavage"
        );
        assert_eq!(enzyme_name(&spec("FYWL", "P", true)), None);
    }

    #[test]
    fn native_id_forms_pick_the_right_format_term() {
        assert_eq!(
            id_format("controllerType=0 controllerNumber=1 scan=30767").0,
            "MS:1000768"
        );
        assert_eq!(id_format("index=12").0, "MS:1000774");
        assert_eq!(id_format("scan=5").0, "MS:1000776");
        assert_eq!(id_format("frame=3 scan=9").0, "MS:1002818");
        assert_eq!(id_format("whatever").0, "MS:1000824");
    }

    #[test]
    fn file_format_is_found_from_the_extension() {
        assert_eq!(file_format("a.mzML.gz").unwrap().0, "MS:1000584");
        assert_eq!(file_format("a.MZML").unwrap().0, "MS:1000584");
        assert_eq!(file_format("a.mgf").unwrap().0, "MS:1001062");
        assert_eq!(file_format("a.d"), None);
    }

    #[test]
    fn a_decoy_tag_is_escaped_for_the_regexp() {
        assert_eq!(regex_escape("rev_"), "rev_");
        assert_eq!(regex_escape("DECOY."), "DECOY\\.");
    }
}
