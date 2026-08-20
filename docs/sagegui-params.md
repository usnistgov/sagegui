# SageGUI Parameters Reference

## Experiment tab

### Experiment Type

**Custom**  Free form mode. All settings are taken from the current UI state.

**Tryptic LFQ**  Preset for fully tryptic label free workflows. Loads a tryptic enzyme, LFQ enabled, narrow tolerances, and common variable mods.[file:250]

**Wide Open**  Preset for wide precursor tolerance open search. Uses strict trypsin, disables semi enzymatic search, and sets a large Da precursor window.[file:250]

**Phospho**  Preset for phosphoproteomics. Enables a curated Phospho STY variable modification list and appropriate mass tolerances.[file:250]

**Semi Tryptic**  Preset that allows one non enzymatic terminus. Intended for more exploratory searches on the same data set.[file:250]

## Files and Database tab

### mzML picker

**Pick mzML files**  Opens a file dialog and populates `mzml_paths` with one or more mzML or mzML.gz files. The list below the button shows the current selection and supports per row removal.[file:250]

### FASTA picker

**Add FASTA**  Opens a file dialog and appends one or more FASTA files to `database.fastapaths`. All selected FASTA files are concatenated at launch time to form the search database.[file:250]

**FASTA list**  Displays the current FASTA set. Each row shows the file name and full path on hover and has a Remove button.[file:250]

### Advanced database options

**Generate Decoys**  Auto generates reversed decoy sequences from the concatenated FASTA for FDR estimation.[file:250]

**Bucket Size**  Controls the peptide index bucket size that Sage uses internally. Affects speed only and does not change results. Smaller values such as 8192 suit high resolution Orbitrap data. Larger values such as 65536 suit lower resolution instruments.[file:250]

**Decoy Tag**  String prefix applied to decoy entries. Defaults to `rev` and is written into the output so downstream tools can recognize decoys.[file:250]

## Enzyme settings

All enzyme controls live under **Enzyme Settings** on the Search tab.[file:250]

**Missed Cleavages**  Maximum number of allowed missed cuts per peptide.[file:250]

**Min Length**  Minimum peptide length in amino acids.[file:250]

**Max Length**  Maximum peptide length in amino acids.[file:250]

**Cleave At**  One or more residues where the enzyme cuts. Default is `KR` for trypsin.[file:250]

**Restrict**  Toggles proline restriction. When on, the enzyme will not cut before the `Restrict Char` residue, default `P`.[file:250]

**C Terminal**  When checked, cleavage is applied at the C terminus of the specified residues. When unchecked, cleavage is N terminal.[file:250]

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

**Lower and Upper**  Two values define the window around the experimental mass. Sage applies the window to the experimental precursor mass and searches theoretical peptides in the range `[experimental + lower, experimental + upper]`.[file:250]

- Lower is normally negative and Upper is normally positive.[file:250]
- A negative lower bound searches theoretical peptides lighter than the observed mass, which corresponds to observed peptides carrying a positive delta mass relative to the unmodified theoretical peptide.[file:250]
- Example text in the UI: `Lower -500` searches peptides 500 Da below the observed mass and therefore peptides that would carry a 500 Da modification.[file:250]

The helper warns if Lower is greater than Upper since that would create an empty search window.[file:250]

### Charge handling

**Precursor Charge Min and Max**  Range of precursor charges that Sage will consider. By default this is 2 to 4.[file:250][file:238]

**Override Precursor Charge**  When enabled, Sage ignores the charge annotation in the raw files and uses the UI range instead. This is useful for DIA and diaPASEF where precursor charge calls may be unreliable.[file:250]

### Mass ranges

**Peptide Min Mass**  Lower bound on peptide mass included in the database.[file:250]

**Peptide Max Mass**  Upper bound on peptide mass included in the database.[file:250]

### Ion kinds

**Ion Kinds**  Set of fragment ion series to score. The defaults enable b and y ions and leave a, c, x, and z disabled.[file:250]

### Isotope errors and search behavior

**Isotope Errors Min and Max**  C13 isotope error offsets Sage will consider in addition to the base precursor mass. The hover text notes that this is slower than simply widening the precursor tolerance to cover the same mass range and suggests using a wider Da window when unsure.[file:250]

**Deisotope**  Enables deisotoping of MS1 features before searching.[file:250]

**Chimera**  Enables chimeric spectrum handling where more than one peptide may be reported for a single MS2 event.[file:250]

**Wide Window**  Switches Sage into a DIA style wide window mode where the precursor tolerance setting is ignored and all charge states in the configured range are considered.[file:238][file:250]

### Scoring

**Score Type**  Choice of SageHyperScore or OpenMSHyperScore. The hover text suggests keeping SageHyperScore unless you are explicitly comparing scoring functions.[file:250]

## Quant tab

### Quantification enable and type

**Enable Quantification**  Global toggle for quant features. When off, Sage runs identification only. When on, it enables either LFQ or TMT based on the type selection.[file:250]

**Label Free Quantification LFQ**  Selects LFQ as the quantification class. Exposes LFQ specific settings.[file:250]

**Tandem Mass Tag TMT**  Selects TMT as the quantification class. Exposes isobaric and TMT specific settings.[file:250]

### LFQ settings

LFQ settings are stored in `quant.lfq_settings` and surfaced as a grouped panel.[file:238][file:250]

**PPM Tolerance**  Precursor tolerance used specifically by the LFQ engine when mapping features across runs.[file:250]

**Spectral Angle**  Similarity threshold between experimental and reference spectra used for LFQ scoring. Range is 0.0 to 1.0.[file:250]

**Combine Charge States**  Boolean that controls whether Sage sums LFQ features across charge states.[file:238][file:250]

- When true all charge states of the same peptide are merged into one LFQ feature. Peptide ABC at charges 2, 3, and 4 produces a single row with a single intensity value. Because the row spans multiple charges Sage writes `charge = -1` in `lfq.tsv` as a sentinel value.[file:145][file:250]
- When false each charge state is kept as its own feature and `lfq.tsv` contains separate rows with real charge values.[file:145][file:250]

**Peptide q value**  Optional per peptide q value filter in the LFQ engine. In your example config this is set to 0.01.[file:238]

### TMT settings

**Isobar Selection**  Radio buttons for TMT 6 plex, 10 plex, 11 plex, 16 plex, and 18 plex. Chooses the isobaric tagging scheme.[file:250]

**Level**  MS level used for TMT quantification. Default is level 3.[file:238][file:250]

**S/N**  Boolean that toggles a signal to noise based filter in TMT quantification.[file:238][file:250]

## Run Info tab

### Output location and options

**Output Location**  Directory where `results.sage.tsv`, `lfq.tsv`, and any additional outputs are written.[file:238][file:250]

**Write PIN file**  Writes a Percolator `.pin` file for downstream rescoring.[file:250]

**Annotate Matches**  Writes fragment ion match annotations alongside the main results.[file:250]

## Advanced scoring and limits

These options live under the Advanced collapsible section on the Search tab.[file:250]

**Min Peaks**  Minimum number of fragment peaks retained per MS2 spectrum after preprocessing.[file:250]

**Max Peaks**  Maximum number of fragment peaks retained per MS2 spectrum.[file:250]

**Min Matched Peaks**  Minimum number of fragment matches required for a PSM to be considered.[file:250]

**Max Fragment Charge**  Maximum fragment charge state to consider when generating theoretical fragments.[file:250]

**Report PSMs**  Number of PSMs to report per spectrum. Default is 1 which keeps only the top scoring match per spectrum.[file:238][file:250]

**Predict RT**  Enables retention time prediction in Sage, which can be used as an additional score feature.[file:238][file:250]

## Defaults (summary)

The default `Config` struct in `ui.rs` uses the following key values:[file:250]

- Precursor tolerance: `[-10, 10]` ppm for both precursor and fragment.[file:250]
- Precursor charge range: 2 to 4.[file:250][file:238]
- Isotope errors: `[-1, 3]`.[file:250][file:238]
- Min peaks: 15, Max peaks: 150, Min matched peaks: 6, Max fragment charge: 1.[file:250]
- Report PSMs: 1.[file:238][file:250]
- LFQ enabled, TMT disabled by default. LFQ uses Hybrid peak scoring and Sum integration with spectral angle 0.7 and PPM tolerance 5.0.[file:238][file:250]
- Combine Charge States: true.[file:238]
- Generate Decoys: true with `rev` decoy tag.[file:250]
- Static mod: Carbamidomethyl C at 57.021464. Variable mod: Oxidation M at 15.994915.[file:250]
