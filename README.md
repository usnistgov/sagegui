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
- TMT (6/10/11/16/18-plex) and LFQ quantification
- Builds for Windows, macOS (Intel + Apple Silicon), and Linux

## To be added

- Experimental templates and loading in results.json for settings
- Export options: pepXML/mzIdentML, Perseus-format (for Perseus/[ProteoPlotter](https://github.com/JGM-Lab-UoG/ProteoPlotter)), MSstats (add feature to [MSstatsConvert](https://github.com/Vitek-Lab/MSstatsConvert), [feature requested](https://github.com/Vitek-Lab/MSstatsConvert/issues/143)), [DIAgui](https://github.com/mgerault/DIAgui), [LFQ-Analyst](https://github.com/MonashBioinformaticsPlatform/LFQ-Analyst)/FragPipe-Analyst/[*-Analyst](https://analyst-suites.org/), add import to PDV for viewing ([feature requested](https://github.com/wenbostar/PDV/issues/110#issue-5145322431)), Scaffold (?)
- iBAQ and other LFQ options

## Download

Get the latest release from the [Releases page](https://github.com/neely/sagegui/releases/latest):

| Platform | Download |
|----------|----------|
| Windows (x64) | [sage-launcher-windows-x64.exe.zip](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-windows-x64.exe.zip) |
| Linux (x64) | [sage-launcher-linux-x64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-linux-x64.tar.gz) |
| macOS (Intel) | [sage-launcher-macos-x64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-macos-x64.tar.gz) |
| macOS (Apple Silicon) | [sage-launcher-macos-arm64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-macos-arm64.tar.gz) |

> **macOS:** If you see an "unidentified developer" warning, right-click and select Open, or run `xattr -d com.apple.quarantine sagegui`

## Quick start

1. Download and extract the archive for your platform
2. Run `sagegui.exe` (Windows) or `sagegui` (macOS/Linux)
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
- [sagePreview](https://github.com/neely/sagePreview) — PTM discovery and reconnaissance using Sage

## Credits

- Michael Lazear — Sage
- Sebastian Paez — original sagegui
- egui/eframe — GUI framework

## License

[Apache-2.0](LICENSE)
