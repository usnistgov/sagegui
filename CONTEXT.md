# Project Context — SageGUI

**Purpose:** This document provides the background knowledge needed to work on this project effectively. Read this at the start of each session.

---

## What is SageGUI?

A **graphical user interface** for [Sage](https://github.com/lazear/sage), a fast proteomics search engine written in Rust. SageGUI allows users to configure and run Sage searches without using the command line.

**Original author:** Sebastian Paez (jspaezp)  
**Current fork:** neely/sagegui  
**Upstream:** jspaezp/sagegui

---

## Domain Primer

### What is Sage?

Sage is a proteomics search engine that:
- Takes mass spectrometry data (mzML files) and a protein database (FASTA file)
- Identifies peptides by matching experimental spectra to theoretical fragmentation patterns
- Outputs peptide-spectrum matches (PSMs) with confidence scores

Sage is known for being **extremely fast** (often 10-100× faster than other tools) while maintaining high sensitivity.

### What does the GUI do?

The GUI provides:
1. **File selection** — Browse for mzML files and FASTA databases
2. **Parameter configuration** — Set tolerances, modifications, enzyme rules, etc.
3. **Search execution** — Run Sage and display progress
4. **Results viewing** — Show identified peptides and proteins

### Key Sage Parameters

| Parameter | Description | Typical Values |
|-----------|-------------|----------------|
| `precursor_tol` | Mass tolerance for precursor ions | 10-20 ppm (closed), ±500 Da (open) |
| `fragment_tol` | Mass tolerance for fragment ions | 10-20 ppm |
| `missed_cleavages` | Allowed missed enzyme cuts | 1-2 |
| `min_len` / `max_len` | Peptide length limits | 7-50 |
| `static_mods` | Fixed modifications (e.g., Carbamidomethyl on C) | Always applied |
| `variable_mods` | Optional modifications (e.g., Oxidation on M) | Searched combinatorially |

### TMT/iTRAQ Quantification

Sage supports isobaric labeling quantification:
- **TMT** (Tandem Mass Tags): 6-plex, 10-plex, 11-plex, 16-plex, 18-plex
- **iTRAQ**: 4-plex, 8-plex

The GUI allows selecting the labeling scheme and MS level for quantification.

---

## Architecture

### Current State (Sebastian's Fork)

```
sagegui/
├── Cargo.toml          # Dependencies including sage-core, sage-cli, sage-cloudpath
├── src/
│   └── main.rs         # Single-file GUI using egui/eframe
├── assets/
│   └── logo.png        # Application icon
└── .github/
    └── workflows/
        └── build.yml   # CI/CD for multi-platform builds
```

**Key architectural decision:** The GUI embeds Sage as a Rust library dependency, not as a subprocess. This means:
- ✅ Single binary distribution
- ✅ Tight integration (can show real-time progress)
- ❌ Coupled to Sage's internal API (breaks when Sage updates)

### The API Coupling Problem

Sebastian's GUI depends on a **custom fork of Sage** (`jspaezp/sage`) because:
1. The official `sage-cli` crate doesn't expose a library target
2. Internal types like `Kind` (ion series) don't implement `Hash`/`Eq`
3. The `timsrust` crate API changed between versions

This is why the GUI is stuck on an old Sage version.

---

## Key Assumptions (Decided, Don't Relitigate)

1. **Option A chosen** — We will fork Sage ourselves and maintain lib exports, rather than using a wrapper approach (Option C) that calls `sage.exe` as a subprocess.

2. **Maintenance commitment** — This means we accept the ongoing maintenance burden of keeping our Sage fork in sync with upstream releases.

3. **egui/eframe framework** — The GUI uses egui (immediate mode GUI) with eframe (native window wrapper). This is a good choice for Rust GUIs.

---

## Gotchas Discovered

| Gotcha | Details |
|--------|---------|
| **TMT plex bug** | Lines 421-423: TMT 16/18-plex were incorrectly mapped to `Tmt11`. Fixed in commit a225481. |
| **Fragment tolerance bug** | Lines 720-726: Switching tolerance type updated `precursor_tol` instead of `fragment_tol`. Fixed in commit a225481. |
| **sage-cli no lib target** | Official Sage doesn't expose `sage-cli` as a library. Sebastian added `lib.rs` to his fork. |
| **Kind not hashable** | `sage_core::ion_series::Kind` doesn't implement `Hash`/`Eq` in official Sage. |
| **timsrust API change** | `timsrust::readers::SpectrumReaderConfig` doesn't exist in newer versions. |

---

## Reference Material Index

| File | When to consult |
|------|-----------------|
| `C:\Users\ban\Documents\GitHub\sagePreview\reference\sage\` | Official Sage source code |
| `C:\Users\ban\Documents\GitHub\sagePreview\reference-notes\sage-online-docs.md` | Sage documentation |
| `jspaezp/sage` fork | Sebastian's modified Sage with lib exports |

---

## Project Structure

```
sagegui/
├── CONTEXT.md           # This file — background knowledge
├── PLAN.md              # Architecture, phases, requirements
├── NOTES.md             # Progress log, decisions
├── README.md            # User-facing documentation
├── docs/
│   └── GLOSSARY.md      # Term definitions
├── Cargo.toml           # Rust dependencies
├── src/
│   └── main.rs          # GUI implementation (~1000 lines)
├── assets/
│   └── logo.png         # Application icon
└── .github/
    └── workflows/
        └── build.yml    # CI/CD for releases
```

---

## Workflow Notes

- **Plan mode for kickoffs** — Use Plan mode for architecture decisions
- **Act mode for implementation** — Use Act mode for coding
- **Commit at checkpoints** — Every phase ends with a commit
- **Update BOTH PLAN.md AND NOTES.md** — Keep both files in sync

---

## Current State

See `NOTES.md` for current phase status and recent decisions.

**Last updated:** 2026-07-10

**Completed phases:** 0 (Bug fixes pushed)  
**Current phase:** 1 (Get organized)  
**Next phase:** 2 (Fork Sage, add lib exports)

---

## Related Projects

| Project | Purpose | Location |
|---------|---------|----------|
| **sagePreview** | Reconnaissance tool using Sage | `C:\Users\ban\Documents\GitHub\sagePreview` |
| **sage** (official) | Proteomics search engine | `github.com/lazear/sage` |
| **sage** (Sebastian's fork) | Modified Sage with lib exports | `github.com/jspaezp/sage` |
| **sagegui** (Sebastian's) | Original GUI | `github.com/jspaezp/sagegui` |
| **sagegui** (our fork) | This project | `github.com/neely/sagegui` |
