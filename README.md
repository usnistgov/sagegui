# SageGUI

A graphical user interface for [Sage](https://github.com/lazear/sage), the blazingly fast proteomics search engine.

![SageGUI Screenshot](assets/logo.png)

## Origin Story

This project is a fork of [jspaezp/sagegui](https://github.com/jspaezp/sagegui), a GUI created by Sebastian Paez approximately 2 years ago. We loved the concept — a simple, cross-platform way to run Sage without touching the command line — but found it was stuck on an old Sage version due to tight coupling with Sage's internal API.

Sebastian's approach embedded Sage as a Rust library dependency, which gave tight integration but meant the GUI broke whenever Sage's internal types changed. His GUI was pinned to a custom fork of Sage that hasn't been updated.

## Goals

1. **Stay current with Sage** — Update to work with official Sage releases (currently targeting v0.14.7, with v0.15.0-beta.2 on the horizon)
2. **Cross-platform** — Build for Windows, macOS, and Linux via GitHub Actions
3. **User-friendly** — Make Sage accessible to users who prefer GUIs over command lines
4. **Maintainable** — Document the update process so future Sage versions can be supported with minimal effort

## Status

🚧 **Work in Progress**

**Completed:**
- [x] Forked Sebastian's GUI
- [x] Fixed TMT 16/18-plex selection bug
- [x] Fixed fragment tolerance type switching bug
- [x] Set up project documentation

**In Progress:**
- [ ] Fork official Sage and add library exports
- [ ] Update GUI to work with Sage v0.14.7+
- [ ] Test and validate all features
- [ ] Create first release

## Features

When complete, SageGUI will support:

- **File Selection** — Browse for mzML files and FASTA databases
- **Search Parameters** — Configure tolerances, enzyme rules, modifications
- **Quantification** — TMT (6/10/11/16/18-plex), iTRAQ (4/8-plex), LFQ
- **Search Execution** — Run Sage with progress display
- **Results Summary** — View identification statistics

## Installation

*Coming soon — releases will be available on the [Releases page](https://github.com/neely/sagegui/releases).*

## Building from Source

Requires Rust toolchain (1.70+):

```bash
git clone https://github.com/neely/sagegui.git
cd sagegui
cargo build --release
```

The binary will be at `target/release/sagegui` (or `sagegui.exe` on Windows).

## Documentation

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

MIT License — see [LICENSE](LICENSE) for details.
