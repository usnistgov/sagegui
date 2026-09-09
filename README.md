# SageGUI

A graphical interface for [Sage](https://github.com/lazear/sage), the proteomics search engine by [Michael Lazear](https://github.com/lazear). Unofficial; not affiliated with the Sage project.

Based on the original [sagegui](https://github.com/jspaezp/sagegui) by [Sebastian Paez](https://github.com/jspaezp).

[![Sage Version](https://img.shields.io/badge/Sage-v0.15.0--beta.2-blue)](https://github.com/lazear/sage/releases/tag/v0.15.0-beta.2)
[![Build Status](https://github.com/neely/sagegui/actions/workflows/build.yml/badge.svg)](https://github.com/neely/sagegui/actions)
[![Release](https://img.shields.io/github/v/release/neely/sagegui)](https://github.com/neely/sagegui/releases/latest)

<p align="center">
  <img src="assets/sagegui_logo-removebg.png" alt="SageGUI Logo" width="400">
</p>

## What it does

SageGUI lets you configure and run Sage searches without the command line. Pick your mzML files and FASTA databases, set search parameters, and hit Run. Results land wherever you point the output directory.

Sebastian's original GUI was pinned to a stale Sage fork. This fork updates it to Sage v0.15.0-beta.2 and keeps it current.

## Features

- Multiple FASTA files (target + contaminants + spike-ins), concatenated automatically at search time
- Six tabs: Experiment, Files & Database, Search, Modifications, Quant, Run/Info
- Run bar pinned at the bottom so you can launch from any tab
- Modifications picker with a curated preset list (Static and Variable boxes, Mascot-style)
- Enzyme presets: 14 curated proteases, including the Asp-N and Lys-N cases that cut before their residue
- TMT (6/10/11/16/18-plex) and LFQ quantification
- Database prefiltering (chunked FASTA processing) to bound peak memory on semi-enzymatic and non-specific searches
- Settings remembered between sessions (parameters, file picks, active tab, and modifications)
- Starting templates on the Experiment tab (tryptic wide MS1/tight MS2, tight, open, biofluid, TMT 11-plex)
- Load search parameters from any Sage `config.json` or a past run's `results.json`
- Tolerance windows entered as a delta-mass range, so a +500 Da modification is set as +500
- Stop button — cancels a run, including one already scoring spectra
- Builds for Windows, macOS (Intel + Apple Silicon), and Linux

## To be added

- Export options: pepXML/mzIdentML, Perseus-format (for Perseus/[ProteoPlotter](https://github.com/JGM-Lab-UoG/ProteoPlotter)), [DIAgui](https://github.com/mgerault/DIAgui), [LFQ-Analyst](https://github.com/MonashBioinformaticsPlatform/LFQ-Analyst)/FragPipe-Analyst/[*-Analyst](https://analyst-suites.org/), Scaffold (?). PDV and MSstats are covered under Downstream tools instead: both read Sage output directly, so no exporter is needed here.
- iBAQ and other LFQ options

## Download

Get the latest release from the [Releases page](https://github.com/neely/sagegui/releases/latest):

| Platform | Download |
|----------|----------|
| Windows (x64) | [sage-launcher-windows-x64.exe.zip](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-windows-x64.exe.zip) |
| Linux (x64) | [sage-launcher-linux-x64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-linux-x64.tar.gz) |
| macOS (Intel) | [sage-launcher-macos-x64.zip](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-macos-x64.zip) |
| macOS (Apple Silicon) | [sage-launcher-macos-arm64.zip](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-macos-arm64.zip) |

> **macOS:** the archive contains **Sage Launcher.app** — double-click to run. If you see an "unidentified developer" warning, right-click the app and select Open, or run `xattr -dr com.apple.quarantine "Sage Launcher.app"`

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
git clone https://github.com/neely/sagegui.git
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

## Citation

If you use SageGUI in published work, please cite the software:

> Neely, B.A. (2026). *SageGUI: a graphical interface for the Sage proteomics
> search engine* (Version 0.8.1) [Computer software]. National Institute of
> Standards and Technology. https://github.com/neely/sagegui

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

## Attribution

This project is an officially supported NIST fork of Sebastian Paez’s
sagegui, built on Sage:

- **Sage** (Michael Lazear), MIT License — https://github.com/lazear/sage
- **sagegui** (J. Sebastian Paez), Apache License 2.0 — https://github.com/jspaezp/sagegui

Full upstream license texts are provided in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Credits

- Michael Lazear — Sage
- Sebastian Paez — original sagegui
- egui/eframe — GUI framework

## License

NIST-authored portions of this project are distributed under the NIST Software
Licensing Statement — see [LICENSE](LICENSE). Third-party components
(Sage and the original sagegui) retain their original licenses; see
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) for details.

Project FAIR and governance practices are described in
[fair-software.md](fair-software.md).

## Contact

Benjamin A. Neely — PI  
Data Science and AI Group, Material Data Division, Material Measurement Laboratory, National Institute of Standards and Technology  
benjamin.neely@nist.gov
