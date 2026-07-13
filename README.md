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

✅ **Working** — GUI compiles and runs searches successfully!

**Completed:**
- [x] Forked Sebastian's GUI
- [x] Fixed TMT 16/18-plex selection bug
- [x] Fixed fragment tolerance type switching bug
- [x] Set up project documentation
- [x] Forked Sage to neely/sage
- [x] Updated to Sage v0.15.0-beta.2
- [x] Fixed all API compatibility issues
- [x] Tested with real data (60,672 PSMs from single mzML)
- [x] LFQ quantification working
- [x] CI/CD pipeline with automated releases

## Features

SageGUI supports:

- **File Selection** — Browse for mzML files and FASTA databases
- **Search Parameters** — Configure tolerances, enzyme rules, modifications
- **Quantification** — TMT (6/10/11/16/18-plex), LFQ
- **Search Execution** — Run Sage with progress display
- **Results Summary** — View identification statistics

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

## Documentation

- [CHANGELOG.md](CHANGELOG.md) — Release history and changes
- [CONTEXT.md](CONTEXT.md) — Background knowledge for developers
- [PLAN.md](PLAN.md) — Development roadmap and architecture
- [NOTES.md](NOTES.md) — Progress log and decisions
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
