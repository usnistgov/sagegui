# Glossary — SageGUI

Terms and concepts used in this project.

---

## Proteomics Terms

### DDA (Data-Dependent Acquisition)
A mass spectrometry acquisition mode where the instrument selects the most intense precursor ions for fragmentation. Contrast with DIA.

### FASTA
A text file format for protein/nucleotide sequences. Each entry has a header line starting with `>` followed by the sequence.

### Fragment Ion
An ion produced when a peptide breaks apart during MS2 fragmentation. Common types: b-ions (N-terminal) and y-ions (C-terminal).

### mzML
An open XML format for mass spectrometry data. Contains spectra with m/z and intensity values.

### Peptide
A short chain of amino acids, typically 7-50 residues. Proteins are digested into peptides for mass spectrometry analysis.

### Precursor Ion
The intact peptide ion selected for fragmentation in MS2. Also called "parent ion."

### PSM (Peptide-Spectrum Match)
A match between an experimental MS2 spectrum and a theoretical peptide fragmentation pattern.

### PTM (Post-Translational Modification)
A chemical modification to a protein after translation. Examples: phosphorylation, oxidation, acetylation.

---

## Mass Spectrometry Terms

### Da (Dalton)
Unit of molecular mass. 1 Da ≈ mass of one hydrogen atom.

### m/z (mass-to-charge ratio)
The ratio of an ion's mass to its charge. The x-axis of a mass spectrum.

### MS1 (MS Level 1)
Survey scan showing intact precursor ions. Used for quantification.

### MS2 (MS Level 2)
Fragmentation scan showing fragment ions from a selected precursor. Used for identification.

### ppm (parts per million)
A relative mass tolerance unit. 10 ppm at m/z 1000 = ±0.01 Da.

### TIC (Total Ion Current)
The sum of all ion intensities in a spectrum or chromatogram.

---

## Sage-Specific Terms

### Chimera
When multiple co-eluting peptides are fragmented together in a single MS2 spectrum. Sage can identify multiple peptides per spectrum with `chimera: true`.

### Closed Search
A search with narrow precursor tolerance (e.g., 10-20 ppm) that only finds peptides matching expected masses.

### Decoy
A reversed or shuffled protein sequence used for FDR estimation. Sage uses `rev_` prefix by default.

### FDR (False Discovery Rate)
The expected proportion of false positives among reported identifications. Typically controlled at 1%.

### Open Search
A search with wide precursor tolerance (e.g., ±500 Da) that can discover unexpected modifications.

### q-value
The minimum FDR at which a PSM would be accepted. Lower is better.

---

## Quantification Terms

### iTRAQ (Isobaric Tags for Relative and Absolute Quantitation)
An isobaric labeling method for multiplexed quantification. Available in 4-plex and 8-plex.

### LFQ (Label-Free Quantification)
Quantification without chemical labels, using MS1 precursor intensities.

### TMT (Tandem Mass Tags)
An isobaric labeling method for multiplexed quantification. Available in 6, 10, 11, 16, and 18-plex.

---

## Software Terms

### egui
An immediate-mode GUI library for Rust. Used by SageGUI for the interface.

### eframe
A framework for running egui applications as native desktop apps.

### Cargo
Rust's package manager and build system.

### crate
A Rust package/library.

---

## Project-Specific Terms

### Option A (Fork Sage)
Our chosen approach: fork Sage, add lib exports, maintain sync with upstream.

### Option C (Wrapper)
Alternative approach (not chosen): call `sage.exe` as subprocess, generate JSON config.

### lib target
A Rust crate that can be used as a library dependency. The official `sage-cli` doesn't have one.
