# SageGUI

A graphical interface for [Sage](https://github.com/lazear/sage), the proteomics search engine by [Michael Lazear](https://github.com/lazear). Unofficial; not affiliated with the Sage project.

This repository is the NIST-maintained fork of [jspaezp/sagegui](https://github.com/jspaezp/sagegui), the original graphical interface by [J. Sebastian Paez](https://github.com/jspaezp). It preserves the upstream project's history and incorporates substantial NIST-developed modifications. The upstream project remains available for general collaboration, while this fork provides NIST-controlled, versioned releases.

[![Sage Version](https://img.shields.io/badge/Sage-v0.15.0--beta.2-blue)](https://github.com/lazear/sage/releases/tag/v0.15.0-beta.2)
[![Build Status](https://github.com/usnistgov/sagegui/actions/workflows/build.yml/badge.svg)](https://github.com/usnistgov/sagegui/actions)
[![Release](https://img.shields.io/github/v/release/usnistgov/sagegui)](https://github.com/usnistgov/sagegui/releases/latest)

<p align="center">
  <img src="assets/sagegui_logo-removebg.png" alt="SageGUI Logo" width="400">
</p>

## What it does

SageGUI lets you configure and run Sage searches without the command line. Pick your mzML files and FASTA databases, set search parameters, and hit Run. Results land wherever you point the output directory.

This fork compiles Sage from source vendored in [`vendor/sage`](vendor/sage/VENDORED.md): upstream `lazear/sage` at commit `d74024d` (10 commits after the v0.15.0-beta.2 release), plus three small additive patches (search progress, the Stop button and the database cache), listed in [`PATCHES.md`](vendor/sage/PATCHES.md).

## Features

- Multiple FASTA files (target + contaminants + spike-ins), concatenated automatically at search time
- Six tabs: Experiment, Files & Database, Search, Modifications, Quant, Run/Info
- Run bar pinned at the bottom so you can launch from any tab
- Modifications picker with a curated preset list (Static and Variable boxes, Mascot-style)
- Enzyme presets: 14 curated proteases, including the Asp-N and Lys-N cases that cut before their residue
- TMT (6/10/11/16/18-plex) and LFQ quantification
- Database prefiltering (chunked FASTA processing) to bound peak memory on semi-enzymatic and non-specific searches
- Optional database cache: save the built peptide database and reuse it on a later run with the same FASTA and database settings (off by default; see below)
- Settings remembered between sessions (parameters, file picks, active tab, and modifications)
- Starting templates on the Experiment tab (tryptic wide MS1/tight MS2, tight, open, biofluid, TMT 11-plex)
- Load search parameters from any Sage `config.json` or a past run's `results.json`
- Tolerance windows entered as a delta-mass range, so a +500 Da modification is set as +500
- Stop button — cancels a run, including one already scoring spectra
- Search output: write a Percolator `.pin` file, an HTML QC report, or per-fragment match detail alongside the results
- Convert results to mzIdentML 1.1.1 and pepXML 1.23 from the Run / Info tab, for any earlier run or automatically after a search. Choose the q-value (spectrum, peptide or protein), the limit and whether to keep decoys
- Builds for Windows, macOS (Intel + Apple Silicon), and Linux

### Database cache

On Files & Database, "Cache prepared database" saves the peptide database that Sage builds from the FASTA. A later run with the same FASTA content and the same database settings (enzyme, modifications, mass range, decoys) loads it instead of building it again. Changing only tolerances, charges, quantification or the spectrum files still reuses it. It is off by default, because one entry for a human proteome is about 3 GB, and it is not available with prefiltering.

On our test Mac the gain is small: the human proteome database took 9 s to build and 6 s to load from the cache. It may help more on slower computers. It does not help semi-enzymatic or non-specific searches, whose databases are too large to cache (the limit is 12 GiB per entry).

The cache lives in `~/Library/Caches/gov.nist.sagegui/index-cache` (macOS), `%LOCALAPPDATA%\SageGUI\index-cache` (Windows) or `~/.cache/sagegui/index-cache` (Linux). Run / Info shows its size and has a Clear button. It holds at most 20 GiB and deletes the least recently used entries first.

## To be added

- Export options: Perseus-format (for Perseus/[ProteoPlotter](https://github.com/JGM-Lab-UoG/ProteoPlotter)), [DIAgui](https://github.com/mgerault/DIAgui), [LFQ-Analyst](https://github.com/MonashBioinformaticsPlatform/LFQ-Analyst)/FragPipe-Analyst/[*-Analyst](https://analyst-suites.org/), Scaffold (?). PDV and MSstats are covered under Downstream tools instead: both read Sage output directly, so no exporter is needed here.
- iBAQ and other LFQ options

## Download

NIST maintains an independent release series from the upstream [jspaezp/sagegui](https://github.com/jspaezp/sagegui) project. NIST releases use Git tags of the form `nist-vX.Y.Z`. Version numbers in the two series do not correspond: upstream's `v0.7.0`, for example, is a different release from our `nist-v0.7.0`. This fork branched from upstream at commit [`e6ccd69`](https://github.com/jspaezp/sagegui/commit/e6ccd69ce52ddebf837edc91d6eb8e194a415229) (2025-08-22), two commits after upstream's `v0.5.0`. Our releases 0.6.0 to 0.9.0 were first published on `neely/sagegui` and are available here as `nist-v0.6.0` to `nist-v0.9.0`, with the same binaries.

Get the latest release from the [Releases page](https://github.com/usnistgov/sagegui/releases/latest):

| Platform | Download |
|----------|----------|
| Windows (x64) | [sage-launcher-windows-x64.exe.zip](https://github.com/usnistgov/sagegui/releases/latest/download/sage-launcher-windows-x64.exe.zip) |
| Linux (x64) | [sage-launcher-linux-x64.tar.gz](https://github.com/usnistgov/sagegui/releases/latest/download/sage-launcher-linux-x64.tar.gz) |
| macOS (Intel) | [sage-launcher-macos-x64.zip](https://github.com/usnistgov/sagegui/releases/latest/download/sage-launcher-macos-x64.zip) |
| macOS (Apple Silicon) | [sage-launcher-macos-arm64.zip](https://github.com/usnistgov/sagegui/releases/latest/download/sage-launcher-macos-arm64.zip) |

> **macOS:** the archive contains **Sage Launcher.app**. The app is not notarized by Apple, so after you download it macOS refuses to open it with **"Sage Launcher.app is damaged and can't be opened."** The app is not actually damaged. Unzip it, then run this once in Terminal from the folder containing the app:
>
> ```bash
> xattr -dr com.apple.quarantine "Sage Launcher.app"
> ```
>
> Then open it normally. Right-clicking and choosing Open does **not** work for this message. If you clicked **Move to Trash**, open the Trash in Finder, right-click the app and choose **Put Back**, or download it again.

## Quick start

1. Download and extract the archive for your platform
2. Run `sagegui.exe` (Windows), double-click **Sage Launcher.app** (macOS), or run `./sagegui` (Linux)
3. On the **Files & Database** tab, click "Add FASTA..." and add your database(s), then "Pick mzML files"
4. Adjust search parameters on the Search, Modifications, and Quant tabs if needed
5. Click **Run** at the bottom of any tab

Output goes to the directory set on the Run/Info tab (defaults to the working directory).

## Building from source

Requires Rust 1.70+:

```bash
git clone https://github.com/usnistgov/sagegui.git
cd sagegui
cargo build --release
```

Binary is at `target/release/sagegui` (or `sagegui.exe` on Windows).

## Related

- [Sage](https://github.com/lazear/sage) — the search engine
- [sageRecon](https://github.com/usnistgov/sageRecon) — reconnaissance for unfamiliar data: detects modifications and recommends mass tolerances before a production search

## Downstream tools

Tools that read what SageGUI produces. This list grows as support lands.

- [PDV](https://github.com/wenbostar/PDV) — spectrum and PSM viewer. Sage support arrived in
  [v2.7.0](https://github.com/wenbostar/PDV/releases/tag/v2.7.0): open `results.sage.tsv` in the
  Database Searching dialog together with the mzML or mgf files the search used. Gzipped spectrum
  files work, and decoys and hits above 1% q-value can be filtered on import.
- [MSstats](https://github.com/Vitek-Lab/MSstatsConvert) — statistical analysis. A Sage converter is
  [in development](https://github.com/Vitek-Lab/MSstatsConvert/issues/143) and will read `lfq.tsv`,
  which carries the MS1 areas MSstats wants. **If you plan to use it, turn off Combine Charge States
  on the Quant tab.** With it on, which is the default, Sage writes a charge of -1 and
  `PrecursorCharge` is meaningless downstream.
- Tools that read mzIdentML or pepXML — use the **Results** group on the Run / Info tab. It writes
  `results.sage.mzid` (mzIdentML 1.1.1) and `results.sage.pep.xml` (pepXML 1.23) next to the Sage
  output. The mzIdentML file passes strict schema validation. The pepXML file does not, because the
  pepXML 1.23 schema does not list Sage as a search engine and the file names it as `Sage`. Both
  files were parsed with zero errors by [pyteomics](https://pyteomics.readthedocs.io/), an
  independent Python reader, on a real 9,392-entry run. No specific target tool (Scaffold, Skyline,
  PeptideShaker, TPP) has opened either file yet.
- [Percolator](https://github.com/percolator/percolator) or [mokapot](https://mokapot.readthedocs.io/)
  — PSM rescoring. Tick **Write PIN file** under Search output on the Run / Info tab to write
  `results.sage.pin`. Verified: mokapot loads it with the exact target and decoy counts from the
  search and completes a full semi-supervised rescoring run.
- Spectral-library builders and manual QC tools that read per-fragment match detail — tick
  **Annotate Matches** under Search output to write `matched_fragments.sage.tsv` (ion type, ordinal,
  charge, calculated and observed m/z, intensity). Verified against a real run: every fragment row
  traces to a real PSM, and `results.sage.tsv`'s match-quality columns reproduce correctly from it.

## Citation

If you use SageGUI in published work, please cite the software:

> Neely, B.A. (2026). *SageGUI: a graphical interface for the Sage proteomics
> search engine* (Version 0.9.0) [Computer software]. National Institute of
> Standards and Technology. https://github.com/usnistgov/sagegui

Cite the version you ran, not the repository in general. The pinned Sage
engine version and the bundled templates both move between releases, so two
versions of SageGUI do not necessarily run the same search. The Run / Info tab
prints the SageGUI version and the Sage engine version it was built against.

A `CITATION.cff` file is included, so GitHub's "Cite this repository" control
produces the same reference in BibTeX or APA.

Please also cite Sage, which performs the searches:

> Lazear, M.R. "Sage: An Open-Source Tool for Fast Proteomics Searching and
> Quantification at Scale." *Journal of Proteome Research* 2023, 22(11),
> 3652–3659. doi:10.1021/acs.jproteome.3c00486

## Third-Party Software

SageGUI incorporates code from two projects:

- **sagegui** (J. Sebastian Paez), Apache License 2.0, https://github.com/jspaezp/sagegui. This repository is a fork of it. The files that derive from it carry a notice that says so, as Apache License 2.0 Section 4(b) requires, and `THIRD_PARTY_LICENSES.md` lists them.
- **Sage** (Michael Lazear), MIT License, https://github.com/lazear/sage. Its source is vendored in [`vendor/sage`](vendor/sage/VENDORED.md) and compiled into the program. We made three small additive changes to one file, listed in [`vendor/sage/PATCHES.md`](vendor/sage/PATCHES.md). The SageGUI logo, made at NIST, and the app icons derive from Sage's logo.

Full licence texts, the list of derived files and the modification notices are in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Credits

- Michael Lazear: Sage
- J. Sebastian Paez: the original sagegui
- egui/eframe: GUI framework

## License

This software was developed by employees of the National Institute of Standards and Technology. See [LICENSE.md](LICENSE.md) for the NIST Software Licensing Statement. Third-party components retain their original licences: code derived from sagegui remains under the Apache License 2.0, and Sage remains under the MIT License. See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

Project FAIR and governance practices are described in
[fair-software.md](fair-software.md).

## Contact

Benjamin A. Neely, PI  
Data Science and AI Group, Material Data Division, Material Measurement Laboratory, National Institute of Standards and Technology  
benjamin.neely@nist.gov
