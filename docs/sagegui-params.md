# SageGUI Parameters Reference

## Experiment tab

### Templates

**Templates**  A picker with five complete starting configurations: Tryptic - wide MS1 / tight MS2, Tryptic - tight, Tryptic - open, Tryptic - biofluid and TMT 11-plex. **Apply template** replaces the search parameters. It never changes the selected files or the output folder. See `PARAMETER_REFERENCE.md`, "Templates", for what each one sets.

**Load Sage config or results.json**  Reads the search parameters from a Sage `config.json` or a past run's `results.json`. File paths and the output folder in the file are ignored.

## Files and Database tab

### mzML picker

**Pick mzML files**  Opens a file dialog and populates `mzml_paths` with one or more mzML or mzML.gz files. The list below the button shows the current selection and supports per row removal.[file:250]

### FASTA picker

**Add FASTA**  Opens a file dialog and appends one or more FASTA files to `database.fastapaths`. All selected FASTA files are concatenated at launch time to form the search database.[file:250]

**FASTA list**  Displays the current FASTA set. Each row shows the file name and full path on hover and has a Remove button.[file:250]

### Advanced database options

**Generate Decoys**  Auto generates reversed decoy sequences from the concatenated FASTA for FDR estimation.[file:250]

**Bucket Size**  Controls the peptide index bucket size that Sage uses internally. Affects speed only and does not change results. Smaller values such as 8192 suit high resolution Orbitrap data. Larger values such as 65536 suit lower resolution instruments.[file:250]

**Decoy Tag**  Prefix that marks a decoy protein. Default `rev_`. With Generate Decoys on, Sage puts it on the decoys it makes and drops FASTA entries that already carry it. With Generate Decoys off, Sage uses those entries as the decoys.

## Enzyme settings

All enzyme controls live under **Enzyme Settings** on the Search tab.[file:250]

**Missed Cleavages**  Maximum number of allowed missed cuts per peptide.[file:250]

**Min Length**  Minimum peptide length in amino acids.[file:250]

**Max Length**  Maximum peptide length in amino acids.[file:250]

**Cleave At**  One or more residues where the enzyme cuts. Default is `KR` for trypsin.[file:250]

**Restrict**  Toggles proline restriction. When on, the enzyme will not cut before the `Restrict Char` residue, default `P`.[file:250]

**Cut side**  Two choices: after the residue (the default) or before it. Asp-N and Lys-N cut before their residue.

**Semi Enzymatic**  Allows one non enzymatic terminus. This doubles the search space and should be combined carefully with wide precursor windows.[file:250]

## Modifications tab

### Static and variable modifications

**Static box**  Holds fixed modifications such as Carbamidomethyl C. Each entry is a Sage specificity key and a monoisotopic delta mass.[file:250]

**Variable box**  Holds variable modifications such as Oxidation M. Entries are keyed and stored the same way as static mods but are treated as optional during the search.[file:250]

**Max Variable Mods**  Caps how many variable modifications can co occur on a single peptide. Sage default is 2.[file:250]

### Common modification presets

The curated list provides presets such as Acetyl K, Carbamidomethyl C, Oxidation M, Oxidation P, Phospho STY, Methyl KR, and Trimethyl KR. Each preset inserts one or more Sage keys with Unimod based monoisotopic deltas and carries a short note in the hover text.[file:250]

**Target**  Chooses whether the arrow buttons act on the Static or Variable box.[file:250]

**Add**  Copies the currently selected preset into the target box.[file:250]

**Remove**  Removes the currently selected preset keys from the target box if present.[file:250]

### Custom modifications

**Key**  Sage specificity key string. Supports residue only, peptide terminus, and protein terminus forms. The UI shows examples such as `M`, `C`, and terminus variants in a grid.[file:250]

**Mass**  Monoisotopic delta mass in Da for the custom modification.[file:250]

**Add to target box**  Inserts the custom key and mass into the Static or Variable box selected in Target.[file:250]

## Search tab

### Precursor and fragment tolerance

Precursor and fragment tolerance share the same logic, exposed as separate groups for MS1 and MS2.[file:250]

**Unit selector**  PPM or Da radio buttons control whether the tolerance is interpreted in ppm or absolute Da.[file:250]

**Delta mass from and to**  The window is shown as a delta-mass range: experimental mass minus theoretical mass. A peptide that carries a +500 Da modification has a delta mass of +500 Da, so a range of −100 to +500 Da finds it.

- Sage's own JSON pair is the delta-mass range negated and swapped. Delta −100 to +500 Da is `"da": [-500, 100]` in Sage JSON. SageGUI converts in both directions.
- The Search tab warns if the first value is greater than the second, because that window is empty.

### Charge handling

**Precursor Charge Min and Max**  The precursor charges Sage tries when a spectrum has no charge in the file, when Override Precursor Charge is on, or with Wide Window. A spectrum with a charge in the file is searched at that charge only. By default this is 2 to 4.

**Override Precursor Charge**  When enabled, Sage ignores the charge annotation in the raw files and uses the UI range instead. This is useful for DIA and diaPASEF where precursor charge calls may be unreliable.[file:250]

### Mass ranges

**Peptide Min Mass**  Lower bound on peptide mass included in the database, in Da. Default 500.

**Peptide Max Mass**  Upper bound on peptide mass included in the database, in Da. Default 5000.

### Ion kinds

**Ion Kinds**  Set of fragment ion series to score. The defaults enable b and y ions and leave a, c, x, and z disabled.[file:250]

### Isotope errors and search behavior

**Isotope Errors Min and Max**  C13 isotope error offsets Sage will consider in addition to the base precursor mass. The hover text notes that this is slower than simply widening the precursor tolerance to cover the same mass range and suggests using a wider Da window when unsure.[file:250]

**Deisotope**  Deisotopes MS2 spectra and deconvolutes fragment charge states before scoring. Off by default in SageGUI. The Sage command line defaults to on.

**Chimera**  Searches for co-fragmenting peptides in each MS2 spectrum. Sage reports up to Report PSMs of them, so set Report PSMs above 1.

**Wide Window**  For DIA or wide-window data. Sage ignores the precursor tolerance, uses each spectrum's isolation window instead, and tries every precursor charge in the range.

### Scoring

**Score Type**  Choice of SageHyperScore or OpenMSHyperScore. The hover text suggests keeping SageHyperScore unless you are explicitly comparing scoring functions.[file:250]

## Quant tab

### Quantification enable and type

**Enable Quantification**  Global toggle for quant features. When off, Sage runs identification only. When on, it enables either LFQ or TMT based on the type selection.[file:250]

**Label Free Quantification LFQ**  Selects LFQ as the quantification class. Exposes LFQ specific settings.[file:250]

**Tandem Mass Tag TMT**  Selects TMT as the quantification class. Exposes isobaric and TMT specific settings.[file:250]

### LFQ settings

LFQ settings are stored in `quant.lfq_settings` and surfaced as a grouped panel.[file:238][file:250]

**Integration**  How Sage calculates one intensity per file from the MS1 trace. Sum (area, the default) adds the trace across the full peak. Apex takes the trace height at the apex point only. Requested in [usnistgov/sagegui#1](https://github.com/usnistgov/sagegui/issues/1).

**Peak Scoring**  The score that Sage uses to find the peak apex. Hybrid (the default and the upstream recommendation), Retention time, Spectral angle or Intensity.

**PPM Tolerance**  MS1 m/z tolerance, in ppm, that Sage uses to collect the precursor isotope intensities for each peptide. Default 5.

**Spectral Angle**  Minimum match between the observed and the expected isotope envelope. A retention time point below it is not part of the peak. Range 0.0 to 1.0, default 0.70.

**Combine Charge States**  Boolean that controls whether Sage sums LFQ features across charge states.[file:238][file:250]

- When true all charge states of the same peptide are merged into one LFQ feature. Peptide ABC at charges 2, 3, and 4 produces a single row with a single intensity value. Because the row spans multiple charges Sage writes `charge = -1` in `lfq.tsv` as a sentinel value.[file:145][file:250]
- When false each charge state is kept as its own feature and `lfq.tsv` contains separate rows with real charge values.[file:145][file:250]

**Peptide q-value**  Sage quantifies only peptides at or below this q-value. Default 0.01.

**Mobility tolerance**  Bruker timsTOF only. Not shown, because SageGUI reads mzML only. A value from an imported Sage JSON is kept.

### TMT settings

**Isobar Selection**  Radio buttons for TMT 6 plex, 10 plex, 11 plex, 16 plex, and 18 plex. Chooses the isobaric tagging scheme.[file:250]

**Level**  MS level used for TMT quantification. Default is level 3.[file:238][file:250]

**S/N**  Boolean that toggles a signal to noise based filter in TMT quantification. Not shown on the Quant tab. A value in a loaded Sage JSON is kept.

## Run Info tab

### Output location and options

**Output Location**  Directory where `results.sage.tsv`, `lfq.tsv`, and any additional outputs are written.[file:238][file:250]

**Write PIN file**  Writes a Percolator `.pin` file for downstream rescoring.[file:250]

**Annotate Matches**  Writes fragment ion match annotations alongside the main results.[file:250]

**Write HTML report**  Writes `results.sage.report.html`, a QC report from Sage. Made only during a search. Needs internet to display.

**Results group**  Converts the results of a search to mzIdentML or pepXML. See `PARAMETER_REFERENCE.md`, "Results Location" and "Convert to mzIdentML and Convert to pepXML".

## Advanced scoring and limits

These options live under the Advanced collapsible section on the Search tab.[file:250]

**Min Peaks**  Sage does not search a spectrum with fewer peaks than this. Default 15.

**Max Peaks**  Sage keeps only this many of the most intense peaks in each spectrum. Default 150.

**Min Matched Peaks**  Fewest matched fragments for a PSM to be reported. Default 6. Every bundled template uses 4.

**Max Fragment Charge**  Maximum fragment charge state to consider when generating theoretical fragments.[file:250]

**Report PSMs**  Number of PSMs to report per spectrum. Default is 1 which keeps only the top scoring match per spectrum.[file:238][file:250]

**Predict RT**  Aligns retention times across files and predicts each peptide's retention time, as a feature for FDR. LFQ needs it: with it off, LFQ does not run. Default on.

## Defaults (summary)

The default `Config` struct in `ui.rs` uses the following key values:[file:250]

- Precursor and fragment tolerance: delta mass −10 to +10 ppm.
- Precursor charge range: 2 to 4.[file:250][file:238]
- Isotope errors: `[-1, 3]`.[file:250][file:238]
- Min peaks: 15, Max peaks: 150, Min matched peaks: 6, Max fragment charge: 1.[file:250]
- Report PSMs: 1.[file:238][file:250]
- LFQ enabled, TMT disabled by default. LFQ uses Hybrid peak scoring and Sum integration with spectral angle 0.7, PPM tolerance 5.0 and peptide q-value 0.01.
- Combine Charge States: true.[file:238]
- Generate Decoys: true with `rev_` decoy tag.
- Static mod: Carbamidomethyl C at 57.021464. Variable mod: Oxidation M at 15.994915.[file:250]
