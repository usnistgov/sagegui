# SageGUI — UI specification & redesign brief

> **Purpose of this file:** a single, self-contained block you can paste into a
> web LLM (Perplexity, Claude, ChatGPT, v0, etc.) to prototype a better UI. It
> describes *exactly* what the app does, every control it exposes, and the
> constraints any redesign must respect. Everything the model needs is here —
> you should not have to paste anything else.
>
> It also doubles as the source of truth for the planned **in-GUI parameter
> documentation** feature (tooltips/help for every control) — the "Tooltip
> draft" column below is that copy.

---

## 1. Design brief (paste this framing to the LLM)

**What it is:** SageGUI is a desktop GUI front-end for
[Sage](https://github.com/lazear/sage), a fast proteomics search engine. A
scientist selects mass-spec data files (`.mzML` / Bruker `.d`) and a protein
database (`.fasta`), configures search parameters, clicks **Run**, and Sage
matches spectra to peptides and writes result tables (`results.sage.tsv`,
`lfq.tsv`, `results.json`).

**Who uses it:** proteomics researchers. Range from novices who want to pick
files, accept sensible defaults, and hit Run — to experts who tune many
parameters. Both must be served by the same screen.

**Current state:** one long vertically-scrolling window of collapsible
sections (details below). It works but everything is flat, equally weighted, and
undocumented. A single search can run 10 min – 2 hr.

**The ask to the model:** propose a cleaner layout and interaction design for
the controls in §3, optimizing for: (a) a beginner reaching Run quickly with
good defaults, (b) an expert finding any parameter, (c) always-visible run
state, (d) room for the near-term features in §4. Show a couple of distinct
layout options (e.g. left-nav + panel, top tabs, basic/advanced split) as
labeled ASCII or image mockups, then recommend one. **See §6 for the exact
deliverable format — a written report plus a structured YAML layout spec.**

**Hard constraints — read before designing:**

- The app is **native egui/eframe (immediate-mode Rust GUI)**, and staying
  native is a locked project decision (no HTML/CSS/JS, no web stack). Design in
  terms egui can express: panels, collapsing headers, tabs, sliders, drag-values,
  radio/checkbox, text edits, scroll areas, grids. **Do not** propose CSS
  animations, custom fonts-as-images, drag-and-drop canvases, or anything that
  assumes a DOM.
- Immediate-mode means the whole UI is redrawn every frame from a single
  `Config` state struct — favor simple, stateless-looking layouts over anything
  needing hidden view state.
- Output is a *reference artifact only*. Final layout gets hand-ported into
  `src/main.rs` (soon to be `src/ui.rs`). Fidelity to egui idioms matters more
  than visual polish.

---

## 2. Current screen inventory

A single `CentralPanel` with a vertical `ScrollArea`. Top to bottom:

1. **Title** "Sage Launcher" + logo image.
2. **Run status strip** (only while running): spinner, green "Processing",
   elapsed time `Hh Mm Ss`.
3. **Collapsing: File Selection** — output dir, FASTA, mzML/.d pickers, picked-file list.
4. **Collapsing: Database Configuration** — enzyme, mods, mass ranges, ion kinds, extras.
5. **Collapsing: Tolerance Settings** — precursor & fragment tolerance.
6. **Collapsing: Quantification Options** — enable + LFQ or TMT.
7. **Collapsing: General Settings** — peak/charge/PSM knobs + toggles.
8. **Launch button** + status message (green ok / red error).
9. **Collapsing: Info/Help** — versions, author, license, citation.

All sections are collapsed/expanded independently; nothing is pinned. The Launch
button scrolls away with the rest of the page.

---

## 3. Complete control reference

Every user-facing control, grouped by its current section. `Widget` is the egui
widget in use. `Tooltip draft` is proposed help copy (also feeds the in-GUI docs
feature). Defaults/ranges are from the live code.

### File Selection

| Control | Widget | Default | Notes / Tooltip draft |
|---|---|---|---|
| Output Location | text + Browse (folder) | current working dir | Where result files are written. *Planned: default to a timestamped folder near the data.* |
| FASTA File | text + Browse (`.fasta`) | empty (required) | Protein sequence database to search against. *Planned: multi-file select + built-in cRAP contaminants toggle.* |
| mzML/.d Files | "Pick mzmls" (`.mzML/.gz`), "Pick .d files" (Bruker folders) | none (required) | The MS data to search. `.gz`-compressed mzML accepted. *Planned: auto-convert Thermo `.raw`.* |
| Picked Files list | labels | — | Read-only confirmation of selected inputs. |

### Database → Enzyme Settings

| Control | Widget | Range | Default | Tooltip draft |
|---|---|---|---|---|
| Missed Cleavages | slider | 0–5 | 2 | Max enzyme cut sites a peptide may skip. Higher = larger search space. |
| Min Length | slider | 1–20 | 5 | Shortest peptide (residues) to consider. |
| Max Length | slider | 6–100 | 50 | Longest peptide (residues) to consider. |
| Cleave At | text | — | `KR` | Residues the enzyme cuts after (trypsin = `KR`). |
| Enable Restrict + Restrict Char | checkbox + 1-char text | — | on, `P` | Block cleavage when this residue follows (trypsin "not before P"). Only 1 char honored. |
| C-Terminal | checkbox | — | true | Enzyme cuts at C-terminal side of the cleavage residues. |
| Semi-Enzymatic | checkbox | — | false | Allow one non-enzymatic terminus. Doubles+ search space. |

### Database → Modifications

Static mods apply to every matching residue; variable mods are searched
optionally/combinatorially. Sage position-prefix syntax: `^X` = peptide
N-term, `$X` = peptide C-term, `[X`/`[` = protein N-term, `]X`/`]` = protein
C-term, bare `X` = anywhere on residue X.

| Control | Widget | Default | Tooltip draft |
|---|---|---|---|
| Static: add mass + residue + Add | drag-value + text + button | C = +57.021464 (carbamidomethyl) | Fixed modification always applied. Enter residue (e.g. `C`) and mass. |
| Static: current list + Remove | labels + button | — | Each shows residue and mass; remove to delete. |
| Variable: add mass + residue + Add | drag-value + text + button | M = +15.994915 (oxidation) | Optional modification Sage tries in combination. |
| Variable: current list + Remove | labels + button | — | — |
| max_variable_mods | *(model field, exposed as slider in Database Extras — see below)* | 2 | Max variable mods co-occurring on one peptide. Raise when stacking several. |

*Planned: a preset dropdown of common mods (oxidation M/P, pyro-Glu, deamidation,
N-term acetyl) so users stop hand-typing masses.*

### Database → Mass Ranges

| Control | Widget | Range | Default | Tooltip draft |
|---|---|---|---|---|
| Peptide Min Mass | slider | 300–1000 | 500.0 | Smallest peptide precursor mass (Da) to generate. |
| Peptide Max Mass | slider | 3000–7000 | 5000.0 | Largest peptide precursor mass (Da) to generate. |

### Database → Ion Kinds

| Control | Widget | Default | Tooltip draft |
|---|---|---|---|
| A B C X Y Z checkboxes | checkboxes | B, Y on | Fragment ion series to score. b/y = standard for CID/HCD. |

### Database → Extras

| Control | Widget | Range | Default | Tooltip draft |
|---|---|---|---|---|
| Generate Decoys | checkbox | — | true | Auto-generate reversed decoys for FDR estimation. Leave on unless your FASTA already has decoys. |
| Bucket Size | slider | 8192–65536 | 32768 | **Speed only, no effect on results.** Fragment-index granularity. Use 8192 for high-res (Orbitrap), up to 65536 for low-res (ion trap). |

*(Also in the model but not all surfaced: `min_ion_index`=2, `decoy_tag`="rev_",
`max_variable_mods`=2.)*

### Tolerance Settings

| Control | Widget | Default | Tooltip draft |
|---|---|---|---|
| Precursor type | radio Ppm/Da | Ppm | Unit for precursor mass tolerance. |
| Precursor low/high | two drag-values | −10 / +10 (ppm) | Asymmetric window; Da mode default ±0.02. |
| Fragment type | radio Ppm/Da | Ppm | Unit for fragment mass tolerance. |
| Fragment low/high | two drag-values | −10 / +10 (ppm) | Asymmetric fragment window; Da mode default ±0.02. |

*(Switching Ppm↔Da resets to that unit's default window — intentional.)*

### Quantification Options

| Control | Widget | Default | Tooltip draft |
|---|---|---|---|
| Enable Quantification | checkbox | on | Turn quant on/off. |
| Type | radio LFQ / TMT | LFQ | Label-free (MS1 intensity) vs isobaric tags. **Only LFQ is validated; TMT is untested.** |
| LFQ: PPM Tolerance | drag-value | (LfqSettings default) | MS1 m/z tolerance for feature integration. |
| LFQ: Spectral Angle | slider 0–1 | (default) | Isotope-envelope match threshold. |
| LFQ: Combine Charge States | checkbox | (default) | Sum intensity across charge states of a peptide. |
| TMT: plex | radio 6/10/11/16/18 | TMT 6 | Isobaric plex size. |
| TMT: Level | slider 1–10 | (default) | MS level to read reporter ions from (usually 2 or 3). |

### General Settings

| Control | Widget | Range | Default | Tooltip draft |
|---|---|---|---|---|
| Min Peaks | slider | 5–50 | 15 | Min peaks in a spectrum to attempt a match. |
| Max Peaks | slider | 50–500 | 150 | Keep only the N most intense peaks per spectrum. |
| Min Matched Peaks | slider | 3–20 | 6 | Min matched fragments to report a PSM. |
| Max Fragment Charge | slider | 1–5 | 1 | Highest fragment-ion charge to consider. |
| Report PSMs | slider | 1–10 | 1 | Matches reported per spectrum (>1 = chimeric candidates). |
| Deisotope | checkbox | — | false | Collapse isotope peaks before matching. |
| Chimera | checkbox | — | false | Allow multiple peptides per spectrum. |
| Wide Window | checkbox | — | false | Wide-isolation / DIA-style precursor handling. |
| Predict RT | checkbox | — | true | Predict retention time to aid scoring. |

*(Model fields present but not surfaced as controls: precursor_charge (2,4),
isotope_errors (−1,3), annotate_matches, write_pin, score_type,
override_precursor_charge.)*

### Run + Info

| Control | Widget | Notes |
|---|---|---|
| Launch | button (disabled while running) | Validates FASTA + at least one data file, then runs on a background thread. |
| Status message | colored label | Green = ok, red = error. |
| Info/Help | collapsing | SageGUI version, Sage engine version, author, repo, license, citation. |

---

## 4. Near-term features the layout should accommodate

The redesign should leave room for these (planned, not yet built):

- **Async progress display** — current step (building DB → searching → scoring),
  elapsed + estimated remaining. A persistent, always-visible run area matters.
- **Results summary panel** — after a run, show PSM / peptide / protein counts at
  a chosen FDR, in-app.
- **Config presets** — default / open-search / semi-enzymatic, one click.
- **Save/load config** — JSON export/import; remember last-used settings.
- **Multi-FASTA + cRAP toggle** — select several FASTA files; one checkbox to
  append bundled contaminants.
- **Modification preset library** — dropdown of common mods instead of typing masses.
- **In-GUI parameter docs** — the tooltips drafted above, on every control.
- **Thermo `.raw` auto-conversion** — a `.raw` picker that converts before search.
- **Export formats** — MSstats / LFQ-analyst / Scaffold export buttons after a run.

---

## 5. Constraints recap (for the impatient model)

- Native **egui/eframe** only. No web tech. Express layout as egui panels/tabs/
  collapsing/sliders/etc.
- Single `Config` state struct, redrawn every frame (immediate mode).
- Serve novice-fast-path AND expert-full-control on one screen.
- Keep run state and the Run action always reachable.
- Output = ASCII/image mockups + a recommendation, not code.

---

## 6. Deliverable — what to hand back (READ THIS: it defines your output)

Produce **one markdown document** with two parts: a **written report** with
ASCII mockups (for humans to read and decide), followed by a **structured layout
spec** (for a developer to port into egui near-mechanically). Do **not** write
Rust — the structured spec is the bridge to code, not the code itself.

### Part A — Report (prose + ASCII)

1. **2–3 labeled layout options.** For each: a name, an **ASCII mockup** of the
   main window (box-drawing chars, ~40–70 cols wide, show where every §3 group
   lives and where run state / the Run button sit), then 3–6 sentences on how it
   works, what it optimizes, and its trade-offs.
2. **Recommendation.** Pick one option, say why against the §1 goals
   (beginner-fast-path, expert-full-control, always-visible run state, room for
   §4 features). Note what you'd defer.
3. **Notes for the porter** — anything egui-specific worth flagging (e.g. "use
   `SidePanel::left` for the nav", "this needs a `TopBottomPanel::bottom` so Run
   stays pinned", "Basic page is a `Grid`").

### Part B — Structured layout spec (YAML)

A single fenced ```yaml block the developer can walk top-down. Shape:

```yaml
layout: sidebar | top_tabs | basic_advanced   # the recommended one
regions:
  - id: left_nav
    egui: SidePanel::left
    items: [Files, Database, Tolerance, Quant, Advanced, Run]
  - id: run_bar
    egui: TopBottomPanel::bottom          # pinned, always visible
    contents: [run_button, status_label, progress]
  - id: main
    egui: CentralPanel
    pages:
      - id: files
        title: Files
        widgets:
          # reference §3 control names verbatim so mapping is unambiguous
          - { control: "Output Location", egui: "text_edit + folder button" }
          - { control: "FASTA File",      egui: "text_edit + file button", note: "multi-select planned" }
      - id: database
        title: Database
        groups:
          - title: Enzyme
            widgets:
              - { control: "Missed Cleavages", egui: "Slider 0..=5" }
              # ...every §3 control, by its exact name
```

Rules for Part B:
- **Cover every control in §3** — each must appear once under some page/group,
  referenced by its exact §3 name so mapping to code is unambiguous. If you
  intentionally hide one behind "Advanced", still list it (with `advanced: true`).
- Use only egui-expressible widget hints (Slider, DragValue, Checkbox,
  RadioGroup, TextEdit, ComboBox, CollapsingHeader, Grid, ScrollArea, SidePanel,
  TopBottomPanel, CentralPanel).
- Where a §4 feature needs a placeholder slot (e.g. presets dropdown, results
  panel), add it with `planned: true` so we reserve the space now.

### File to save it as

`docs/ui-proposal.md` in this repo. That keeps the human report and the
machine-readable spec versioned together, and the YAML block becomes the
checklist for the `src/ui.rs` port.
