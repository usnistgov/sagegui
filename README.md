# SageGUI

> A community-maintained graphical interface for [**Sage**](https://github.com/lazear/sage), 
> the proteomics search engine created by [Michael Lazear](https://github.com/lazear).
> 
> *This is an unofficial GUI — not affiliated with or endorsed by the Sage project.*

Based on the original [sagegui](https://github.com/jspaezp/sagegui) by [Sebastian Paez](https://github.com/jspaezp).

[![Sage Version](https://img.shields.io/badge/Sage-v0.15.0--beta.2-blue)](https://github.com/lazear/sage/releases/tag/v0.15.0-beta.2)
[![Build Status](https://github.com/neely/sagegui/actions/workflows/build.yml/badge.svg)](https://github.com/neely/sagegui/actions)
[![Release](https://img.shields.io/github/v/release/neely/sagegui)](https://github.com/neely/sagegui/releases/latest)

<p align="center">
  <img src="assets/sagegui_logo-removebg.png" alt="SageGUI Logo" width="400">
</p>

## Origin Story

Sebastian's original sagegui embedded Sage as a Rust library dependency, which gave tight integration but meant the GUI broke whenever Sage's internal types changed. His GUI was pinned to a custom fork of Sage that hadn't been updated.

We loved the concept — a simple, cross-platform way to run Sage without touching the command line — so we forked it and updated it to work with the latest Sage version.

## Goals

1. **Stay current with Sage** — Works with Sage v0.15.0-beta.2 (latest)
2. **Cross-platform** — Build for Windows, macOS, and Linux via GitHub Actions
3. **User-friendly** — Make Sage accessible to users who prefer GUIs over command lines
4. **Maintainable** — Document the update process so future Sage versions can be supported with minimal effort

## Status

✅ **v0.7.0** — Multi-FASTA + on-the-fly concatenation. Tested: 60,672 PSMs from single mzML, LFQ working.

## Features

- **Multi-FASTA support** — Add multiple FASTA files (target database, cRAP contaminants, spike-ins); concatenated automatically at search time
- **Six-tab navigation** — Experiment, Files & Database, Search, Modifications, Quant, Run / Info; pinned Run bar visible on every tab
- **Search parameters** — Tolerances, enzyme rules, charge handling, isotope errors, scoring function, ion kinds
- **Modifications picker** — Mascot-style two-box (Static / Variable) list with a curated "Common modifications" master list and a custom escape hatch
- **Quantification** — TMT (6/10/11/16/18-plex), LFQ
- **Cross-platform** — Windows, macOS (Intel + Apple Silicon), Linux

## Installation

### Download Pre-built Binaries

Download the latest release for your platform from the [Releases page](https://github.com/neely/sagegui/releases/latest):

| Platform | Download |
|----------|----------|
| **Windows (x64)** | [sage-launcher-windows-x64.exe.zip](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-windows-x64.exe.zip) |
| **Linux (x64)** | [sage-launcher-linux-x64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-linux-x64.tar.gz) |
| **macOS (Intel)** | [sage-launcher-macos-x64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-macos-x64.tar.gz) |
| **macOS (Apple Silicon)** | [sage-launcher-macos-arm64.tar.gz](https://github.com/neely/sagegui/releases/latest/download/sage-launcher-macos-arm64.tar.gz) |

### Building from Source

Requires Rust toolchain (1.70+):

```bash
git clone https://github.com/neely/sagegui.git
cd sagegui
cargo build --release
```

The binary will be at `target/release/sagegui` (or `sagegui.exe` on Windows).

## Quick Start

1. **Download** the binary for your platform from the [Releases page](https://github.com/neely/sagegui/releases/latest)
2. **Extract** the archive (unzip on Windows, `tar -xzf` on macOS/Linux)
3. **Run** the executable (`sagegui.exe` on Windows, `sagegui` on macOS/Linux)
4. **Files & Database tab:**
   - Click "Add FASTA…" to add your target database and any contaminant FASTAs (e.g. cRAP). Files are concatenated automatically at search time.
   - Click "Pick mzML files" to select your mzML files
5. **Configure** search parameters across the Search, Modifications, and Quant tabs (or use defaults)
6. **Click "Run"** in the bar at the bottom of any tab to start the search

Results are saved to the output directory configured on the Run / Info tab (defaults to current working directory).

> **macOS users:** If you see "unidentified developer" warning, right-click the app and select "Open", or run: `xattr -d com.apple.quarantine sagegui`

## Documentation

- [CHANGELOG.md](CHANGELOG.md) — Release history and changes
- [MAINTENANCE.md](MAINTENANCE.md) — How to update Sage version (for maintainers)
- [AGENTS.md](AGENTS.md) — Agent/contributor working protocol
- [PLAN.md](PLAN.md) — Development roadmap and architecture
- [NOTES.md](NOTES.md) — Locked decisions, gotchas, and reference knowledge
- [JOURNAL.md](JOURNAL.md) — Append-only session history
- [docs/GLOSSARY.md](docs/GLOSSARY.md) — Term definitions

## Related Projects

- [Sage](https://github.com/lazear/sage) — The search engine this GUI wraps
- [sagePreview](https://github.com/neely/sagePreview) — A reconnaissance tool using Sage for PTM discovery

## Credits

- **Michael Lazear** — Creator of Sage
- **Sebastian Paez** — Original sagegui author
- **egui/eframe** — The Rust GUI framework used

## License

Apache-2.0 License — see [LICENSE](LICENSE) for details.
