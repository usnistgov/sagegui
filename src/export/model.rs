//! The PSM table that both writers read.
//!
//! `results.sage.tsv` is read once, top to bottom, into a compact table. Only
//! the rows that pass the filters are kept. Long strings (peptide, protein
//! list, file name) are stored once and each row points at them by index.
//! One row is about 150 bytes, so 32,881 rows are about 5 MB.
//!
//! The TSV is read BY COLUMN NAME. Sage 0.14.6 writes 40 columns and 0.15
//! writes 43, so a position from one version is wrong in the other.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use sage_core::mass::monoisotopic;

use super::{Control, ExportOptions, QSource};

/// Mass of a proton. Neutral mass to m/z: `(M + z * PROTON) / z`.
pub const PROTON: f64 = 1.007_276_466_77;
/// Mass of the free peptide terminus groups, for `mod_nterm_mass` and
/// `mod_cterm_mass` in pepXML. N-terminus is H, C-terminus is OH.
pub const N_TERM_H: f64 = 1.007_825_032;
pub const C_TERM_OH: f64 = 17.002_739_65;

/// One PSM that passed the filters.
#[derive(Debug, Clone)]
pub struct Psm {
    /// Index into [`Table::peptides`].
    pub pep: u32,
    /// Index into [`Table::files`].
    pub file: u32,
    /// The mzML nativeID, for example `controllerType=0 controllerNumber=1 scan=30767`.
    pub native_id: Box<str>,
    pub rank: u32,
    pub charge: i32,
    /// Experimental neutral mass in Da (`expmass`).
    pub expmass: f64,
    /// Theoretical neutral mass in Da (`calcmass`).
    pub calcmass: f64,
    /// Scan start time in MINUTES, as Sage writes it. `NaN` when absent.
    pub rt_min: f64,
    pub hyperscore: f64,
    pub delta_next: f64,
    pub discriminant: f64,
    /// log10 of the posterior error probability. Not a probability.
    pub posterior_error_log10: f64,
    pub spectrum_q: f64,
    pub peptide_q: f64,
    pub protein_q: f64,
    /// Mass error in ppm after the isotope error is removed. The TSV column is
    /// `precursor_ppm`. It is Sage's `delta_mass`, verified in `scoring.rs`.
    pub delta_ppm: f64,
    /// Isotope offset in Da (`isotope_error`). 0 or a multiple of 1.00335.
    pub isotope_error_da: f64,
    pub matched_peaks: Option<u32>,
    pub missed_cleavages: Option<u32>,
}

/// One distinct (peptide, protein list, target or decoy) combination.
#[derive(Debug, Clone)]
pub struct PepEntry {
    /// Sage's inline form, for example `[+42.0]-PEPC[+57.0215]TIDE`.
    pub peptide: String,
    /// Indices into [`Table::proteins`], in the order of the TSV.
    pub proteins: Vec<u32>,
    pub decoy: bool,
}

/// What was kept from the TSV, and what was not.
#[derive(Debug, Default)]
pub struct Table {
    /// Base names of the input files, in order of first appearance.
    pub files: Vec<String>,
    pub peptides: Vec<PepEntry>,
    /// Protein names exactly as the TSV holds them. Decoys keep their tag.
    pub proteins: Vec<String>,
    pub psms: Vec<Psm>,
    pub rows_read: usize,
    pub dropped_decoy: usize,
    pub dropped_q: usize,
}

/// Column names by position, taken from the header line.
struct Cols(HashMap<String, usize>);

impl Cols {
    fn new(header: &str) -> Cols {
        Cols(
            header
                .trim_end_matches(['\r', '\n'])
                .split('\t')
                .enumerate()
                .map(|(i, name)| (name.to_string(), i))
                .collect(),
        )
    }

    fn required(&self, name: &str) -> Result<usize, String> {
        self.0.get(name).copied().ok_or_else(|| {
            format!(
                "results.sage.tsv has no `{name}` column. Found {} columns. \
                 Was this file written by Sage?",
                self.0.len()
            )
        })
    }

    fn optional(&self, name: &str) -> Option<usize> {
        self.0.get(name).copied()
    }
}

fn q_column(source: QSource) -> &'static str {
    match source {
        QSource::SpectrumQ => "spectrum_q",
        QSource::PeptideQ => "peptide_q",
        QSource::ProteinQ => "protein_q",
    }
}

fn parse_f64(field: &str, col: &str, line: usize) -> Result<f64, String> {
    field
        .parse::<f64>()
        .map_err(|_| format!("Line {line}: `{field}` in column `{col}` is not a number."))
}

fn parse_int<T: std::str::FromStr>(field: &str, col: &str, line: usize) -> Result<T, String> {
    field
        .parse::<T>()
        .map_err(|_| format!("Line {line}: `{field}` in column `{col}` is not a whole number."))
}

/// Read `results.sage.tsv`, apply the filters, and build the table.
///
/// Progress runs from `lo` to `hi` (fractions of the whole job) by bytes read.
/// A cancel request returns `Err(super::CANCELLED)`.
pub fn load_table(
    tsv: &Path,
    opts: &ExportOptions,
    ctl: &Control,
    lo: f32,
    hi: f32,
) -> Result<Table, String> {
    let file = File::open(tsv).map_err(|e| {
        format!(
            "Cannot open {}: {e}. Run a search first, or pick the folder that holds results.sage.tsv.",
            tsv.display()
        )
    })?;
    let total = file.metadata().map(|m| m.len()).unwrap_or(0).max(1);
    let mut reader = BufReader::with_capacity(1 << 20, file);

    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .map_err(|e| format!("Cannot read {}: {e}", tsv.display()))?;
    if n == 0 {
        return Err(format!("{} is empty.", tsv.display()));
    }
    let cols = Cols::new(&line);
    let c_pep = cols.required("peptide")?;
    let c_prot = cols.required("proteins")?;
    let c_file = cols.required("filename")?;
    let c_scan = cols.required("scannr")?;
    let c_rank = cols.required("rank")?;
    let c_label = cols.required("label")?;
    let c_exp = cols.required("expmass")?;
    let c_calc = cols.required("calcmass")?;
    let c_charge = cols.required("charge")?;
    let c_hyper = cols.required("hyperscore")?;
    let c_q = cols.required(q_column(opts.q_source))?;
    // Needed for the q-value scores that are written next to each PSM.
    let c_spec_q = cols.required("spectrum_q")?;
    let c_pep_q = cols.required("peptide_q")?;
    let c_prot_q = cols.required("protein_q")?;
    // Not in every Sage version, or not always filled. A missing one is left out.
    let c_id = cols.optional("psm_id");
    let c_next = cols.optional("delta_next");
    let c_rt = cols.optional("rt");
    let c_disc = cols.optional("sage_discriminant_score");
    let c_pep_err = cols.optional("posterior_error");
    let c_ppm = cols.optional("precursor_ppm");
    let c_iso = cols.optional("isotope_error");
    let c_matched = cols.optional("matched_peaks");
    let c_missed = cols.optional("missed_cleavages");
    // Every row must reach the last column that is read, so `f[c]` cannot panic.
    let needed = [
        c_pep, c_prot, c_file, c_scan, c_rank, c_label, c_exp, c_calc, c_charge, c_hyper, c_q,
        c_spec_q, c_pep_q, c_prot_q,
    ]
    .into_iter()
    .chain(
        [
            c_id, c_next, c_rt, c_disc, c_pep_err, c_ppm, c_iso, c_matched, c_missed,
        ]
        .into_iter()
        .flatten(),
    )
    .max()
    .unwrap_or(0)
        + 1;

    let mut table = Table::default();
    let mut file_ids: HashMap<String, u32> = HashMap::new();
    let mut prot_ids: HashMap<String, u32> = HashMap::new();
    let mut pep_ids: HashMap<String, u32> = HashMap::new();
    let mut bytes = n as u64;
    let mut line_no = 1usize;

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| format!("Cannot read {}: {e}", tsv.display()))?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        line_no += 1;
        let row = line.trim_end_matches(['\r', '\n']);
        if row.is_empty() {
            continue;
        }
        if line_no & 2047 == 0 {
            if ctl.cancelled() {
                return Err(super::CANCELLED.to_string());
            }
            ctl.report(lo + (hi - lo) * (bytes as f32 / total as f32).min(1.0));
        }
        // Sage writes the TSV with a CSV writer. It only adds quotes around a
        // field that holds a quote, a tab or a line break. Real data does not.
        if row.contains('"') {
            return Err(format!(
                "Line {line_no} of results.sage.tsv has a quoted field. \
                 The converter does not read quoted fields."
            ));
        }
        let f: Vec<&str> = row.split('\t').collect();
        if f.len() < needed {
            return Err(format!(
                "Line {line_no} of results.sage.tsv has {} fields. The header needs at least {needed}.",
                f.len()
            ));
        }
        table.rows_read += 1;

        let label: i32 = parse_int(f[c_label], "label", line_no)?;
        let decoy = label < 0;
        if decoy && !opts.include_decoys {
            table.dropped_decoy += 1;
            continue;
        }
        let q = parse_f64(f[c_q], q_column(opts.q_source), line_no)?;
        // A NaN q-value is not below any limit, so it is dropped too.
        if q.is_nan() || q > opts.max_q {
            table.dropped_q += 1;
            continue;
        }

        let charge: i32 = parse_int(f[c_charge], "charge", line_no)?;
        if charge <= 0 {
            return Err(format!(
                "Line {line_no}: charge {charge} is not a valid precursor charge."
            ));
        }

        // Interned strings.
        let filename = f[c_file];
        let file_idx = match file_ids.get(filename) {
            Some(&i) => i,
            None => {
                let i = table.files.len() as u32;
                table.files.push(filename.to_string());
                file_ids.insert(filename.to_string(), i);
                i
            }
        };
        if f[c_prot].is_empty() {
            return Err(format!("Line {line_no}: the `proteins` column is empty."));
        }
        let pep_key = format!("{}\t{}\t{}", f[c_pep], f[c_prot], decoy as u8);
        let pep_idx = match pep_ids.get(&pep_key) {
            Some(&i) => i,
            None => {
                let mut proteins = Vec::new();
                for name in f[c_prot].split(';').filter(|s| !s.is_empty()) {
                    let pi = match prot_ids.get(name) {
                        Some(&i) => i,
                        None => {
                            let i = table.proteins.len() as u32;
                            table.proteins.push(name.to_string());
                            prot_ids.insert(name.to_string(), i);
                            i
                        }
                    };
                    if !proteins.contains(&pi) {
                        proteins.push(pi);
                    }
                }
                let i = table.peptides.len() as u32;
                table.peptides.push(PepEntry {
                    peptide: f[c_pep].to_string(),
                    proteins,
                    decoy,
                });
                pep_ids.insert(pep_key, i);
                i
            }
        };

        let opt_f = |c: Option<usize>, name: &str| -> Result<f64, String> {
            match c {
                Some(c) if c < f.len() => parse_f64(f[c], name, line_no),
                _ => Ok(f64::NAN),
            }
        };
        // No writer uses `psm_id`. A value that is not a number is still an error.
        if let Some(c) = c_id {
            parse_int::<u64>(f[c], "psm_id", line_no)?;
        }
        table.psms.push(Psm {
            pep: pep_idx,
            file: file_idx,
            native_id: f[c_scan].into(),
            rank: parse_int(f[c_rank], "rank", line_no)?,
            charge,
            expmass: parse_f64(f[c_exp], "expmass", line_no)?,
            calcmass: parse_f64(f[c_calc], "calcmass", line_no)?,
            rt_min: opt_f(c_rt, "rt")?,
            hyperscore: parse_f64(f[c_hyper], "hyperscore", line_no)?,
            delta_next: opt_f(c_next, "delta_next")?,
            discriminant: opt_f(c_disc, "sage_discriminant_score")?,
            posterior_error_log10: opt_f(c_pep_err, "posterior_error")?,
            spectrum_q: parse_f64(f[c_spec_q], "spectrum_q", line_no)?,
            peptide_q: parse_f64(f[c_pep_q], "peptide_q", line_no)?,
            protein_q: parse_f64(f[c_prot_q], "protein_q", line_no)?,
            delta_ppm: opt_f(c_ppm, "precursor_ppm")?,
            isotope_error_da: opt_f(c_iso, "isotope_error")?,
            matched_peaks: match c_matched {
                Some(c) => Some(parse_int(f[c], "matched_peaks", line_no)?),
                None => None,
            },
            missed_cleavages: match c_missed {
                Some(c) => Some(parse_int(f[c], "missed_cleavages", line_no)?),
                None => None,
            },
        });
    }
    ctl.report(hi);
    Ok(table)
}

/// A modification on a parsed peptide.
#[derive(Debug, Clone, PartialEq)]
pub struct ModSite {
    /// 0 is the peptide N-terminus. 1 to `len` is a residue. `len + 1` is the
    /// peptide C-terminus. This is the numbering of mzIdentML `location`.
    pub position: usize,
    /// The residue, or `None` for a terminus.
    pub residue: Option<char>,
    /// Signed mass difference in Da. Read from the text, so `+57.0215` is
    /// exactly 57.0215.
    pub delta: f64,
}

/// A Sage peptide split into its bare sequence and its modifications.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPeptide {
    pub sequence: String,
    pub mods: Vec<ModSite>,
}

/// Parse Sage's inline peptide form.
///
/// Grammar, from `Peptide::fmt` in the Sage source:
/// - N-terminal mass first: `[+42.0106]-PEPTIDE`
/// - a residue mass right after its letter: `PEPC[+57.0215]TIDE`
/// - C-terminal mass last: `PEPTIDE-[+0.9840]`
///
/// The number always has a sign and may be negative.
pub fn parse_peptide(text: &str) -> Result<ParsedPeptide, String> {
    let bad = |why: &str| format!("Cannot read peptide `{text}`: {why}.");
    let b = text.as_bytes();
    let mut i = 0usize;
    let mut sequence = String::new();
    let mut mods = Vec::new();

    // A mass in brackets, starting at the `[`. Returns the value and the index
    // after the `]`.
    let bracket = |start: usize| -> Result<(f64, usize), String> {
        let end = text[start..]
            .find(']')
            .map(|e| start + e)
            .ok_or_else(|| bad("a `[` has no closing `]`"))?;
        let value = text[start + 1..end]
            .parse::<f64>()
            .map_err(|_| bad("a bracket does not hold a number"))?;
        Ok((value, end + 1))
    };

    if b.first() == Some(&b'[') {
        let (delta, next) = bracket(0)?;
        if b.get(next) != Some(&b'-') {
            return Err(bad("an N-terminal mass must be followed by `-`"));
        }
        mods.push(ModSite {
            position: 0,
            residue: None,
            delta,
        });
        i = next + 1;
    }
    let mut cterm: Option<f64> = None;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_uppercase() {
            sequence.push(c as char);
            i += 1;
            if b.get(i) == Some(&b'[') {
                let (delta, next) = bracket(i)?;
                mods.push(ModSite {
                    position: sequence.len(),
                    residue: Some(c as char),
                    delta,
                });
                i = next;
            }
        } else if c == b'-' && b.get(i + 1) == Some(&b'[') {
            let (delta, next) = bracket(i + 1)?;
            if next != b.len() {
                return Err(bad("text follows the C-terminal mass"));
            }
            cterm = Some(delta);
            i = next;
        } else {
            return Err(bad(&format!("unexpected character `{}`", c as char)));
        }
    }
    if sequence.is_empty() {
        return Err(bad("there are no residues"));
    }
    if let Some(delta) = cterm {
        mods.push(ModSite {
            position: sequence.len() + 1,
            residue: None,
            delta,
        });
    }
    Ok(ParsedPeptide { sequence, mods })
}

/// Monoisotopic residue mass in Da, taken from Sage's own table so it matches
/// the masses Sage used. `None` for a letter Sage has no mass for (B, J, X, Z).
///
/// The value is rounded to 5 decimals to drop `f32` noise.
pub fn residue_mass(aa: char) -> Option<f64> {
    if !aa.is_ascii_uppercase() {
        return None;
    }
    let m = monoisotopic(aa as u8);
    if m == 0.0 {
        None
    } else {
        Some(round_to(m as f64, 5))
    }
}

/// An `f32` as the decimal number a person would read from it: `57.0215f32`
/// gives `57.0215`, not `57.02149963378906`. Sage stores mod masses as `f32`.
pub fn decimal(v: f32) -> f64 {
    format!("{v}").parse().unwrap_or(v as f64)
}

/// Round to `places` decimals. Used so `103.00919 + 57.0215` prints as
/// `160.03069` and not `160.03069000000002`.
pub fn round_to(v: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (v * k).round() / k
}

/// The scan number in a nativeID, for the pepXML `start_scan` and for sorting.
///
/// Order of rules: `scan=N` (Thermo and Bruker TDF forms), then `index=N`
/// plus one (an mzML index counts from 0 and a scan number from 1), then the
/// last run of digits. `None` when the text has no digits.
pub fn scan_number(native_id: &str) -> Option<u64> {
    let after = |key: &str| {
        native_id.rfind(key).and_then(|i| {
            let digits: String = native_id[i + key.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse::<u64>().ok()
        })
    };
    if let Some(n) = after("scan=") {
        return Some(n);
    }
    if let Some(n) = after("index=") {
        return Some(n + 1);
    }
    let tail: String = native_id
        .chars()
        .rev()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    tail.parse::<u64>().ok()
}

/// Neutral mass of experimental or theoretical precursor to m/z.
pub fn mz(neutral_mass: f64, charge: i32) -> f64 {
    (neutral_mass + charge as f64 * PROTON) / charge as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_peptide_has_no_mods() {
        let p = parse_peptide("SLAELGGHLDQQVEEFR").unwrap();
        assert_eq!(p.sequence, "SLAELGGHLDQQVEEFR");
        assert!(p.mods.is_empty());
    }

    #[test]
    fn residue_mods_get_one_based_positions() {
        let p = parse_peptide("AGAFC[+57.0215]LSEDAGM[+15.9949]LGISSTASLR").unwrap();
        assert_eq!(p.sequence, "AGAFCLSEDAGMLGISSTASLR");
        assert_eq!(
            p.mods,
            vec![
                ModSite {
                    position: 5,
                    residue: Some('C'),
                    delta: 57.0215
                },
                ModSite {
                    position: 12,
                    residue: Some('M'),
                    delta: 15.9949
                },
            ]
        );
    }

    #[test]
    fn n_term_and_c_term_mods_use_positions_0_and_len_plus_1() {
        let p = parse_peptide("[+42.0106]-PEPTIDE-[-0.984]").unwrap();
        assert_eq!(p.sequence, "PEPTIDE");
        assert_eq!(
            p.mods,
            vec![
                ModSite {
                    position: 0,
                    residue: None,
                    delta: 42.0106
                },
                ModSite {
                    position: 8,
                    residue: None,
                    delta: -0.984
                },
            ]
        );
    }

    #[test]
    fn a_negative_residue_delta_and_several_mods_together() {
        // Pyro-Glu on the first Q, carbamidomethyl on C, oxidation on M.
        let p = parse_peptide("Q[-17.026548]C[+57.0215]M[+15.9949]K").unwrap();
        assert_eq!(p.sequence, "QCMK");
        let deltas: Vec<f64> = p.mods.iter().map(|m| m.delta).collect();
        assert_eq!(deltas, vec![-17.026548, 57.0215, 15.9949]);
        let positions: Vec<usize> = p.mods.iter().map(|m| m.position).collect();
        assert_eq!(positions, vec![1, 2, 3]);
    }

    #[test]
    fn bad_peptide_text_is_an_error() {
        assert!(parse_peptide("PEP[+1.0").is_err(), "no closing bracket");
        assert!(
            parse_peptide("[+1.0]PEPTIDE").is_err(),
            "no dash after N-term mass"
        );
        assert!(parse_peptide("PEPtide").is_err(), "lower case");
        assert!(
            parse_peptide("PEPTIDE-[+1.0]K").is_err(),
            "text after C-term mass"
        );
        assert!(parse_peptide("PEP[abc]").is_err(), "not a number");
        assert!(parse_peptide("").is_err());
    }

    #[test]
    fn residue_masses_match_the_known_monoisotopic_values() {
        // From the Unimod / IUPAC residue table. Sage stores these as f32.
        let known = [
            ('G', 57.02146),
            ('A', 71.03711),
            ('S', 87.03203),
            ('P', 97.05276),
            ('V', 99.06841),
            ('T', 101.04768),
            ('C', 103.00919),
            ('L', 113.08406),
            ('I', 113.08406),
            ('N', 114.04293),
            ('D', 115.02694),
            ('Q', 128.05858),
            ('K', 128.09496),
            ('E', 129.04259),
            ('M', 131.0405),
            ('H', 137.05891),
            ('F', 147.0684),
            ('R', 156.1011),
            ('Y', 163.06332),
            ('W', 186.07932),
        ];
        for (aa, mass) in known {
            let got = residue_mass(aa).expect("residue");
            assert!((got - mass).abs() < 1e-4, "{aa}: {got} vs {mass}");
        }
        assert_eq!(residue_mass('X'), None);
        assert_eq!(residue_mass('c'), None);
    }

    #[test]
    fn total_mass_of_a_modified_residue_is_residue_plus_delta() {
        // pepXML wants 160.03069 for a carbamidomethyl cysteine, not 57.0215.
        let total = round_to(residue_mass('C').unwrap() + 57.0215, 6);
        assert_eq!(total, 160.03069);
        let total = round_to(residue_mass('M').unwrap() + 15.9949, 6);
        assert_eq!(total, 147.0354);
    }

    #[test]
    fn scan_numbers_come_from_the_native_id() {
        assert_eq!(
            scan_number("controllerType=0 controllerNumber=1 scan=30767"),
            Some(30767)
        );
        assert_eq!(scan_number("index=0"), Some(1));
        assert_eq!(scan_number("frame=12 scan=345"), Some(345));
        assert_eq!(scan_number("run7_spectrum_42"), Some(42));
        assert_eq!(scan_number("no digits"), None);
    }

    #[test]
    fn mz_uses_the_proton_mass() {
        // (1926.9476 + 2 * 1.00727646677) / 2
        let m = mz(1926.9476, 2);
        assert!((m - 964.48107646677).abs() < 1e-9, "{m}");
        let m3 = mz(2256.162, 3);
        assert!((m3 - 753.06127646677).abs() < 1e-8, "{m3}");
    }
}
