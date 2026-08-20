## Mass Tolerance and Charge Parameters

Reference notes for `sageGUI` parameters that are easy to misread or misconfigure. Written for both the `/docs/` folder and future hover-text.

### How Sage applies mass tolerance (read this first)

Sage tolerances are applied to the **experimental precursor mass**, not the theoretical one. The search window is:

```
theoretical candidate mass in [experimental + lower, experimental + upper]
```

The lower bound is normally negative, the upper bound normally positive. This is the part that trips people up:

- A negative lower bound searches for theoretical peptides **lighter** than the observed mass. That means the observed peptide carries a **positive** delta mass relative to the unmodified theoretical peptide.
- Example: `lower = -500` finds theoretical peptides up to 500 Da below the observed mass, i.e. peptides that would need a +500 Da modification to match what was actually observed.

`ui.rs` surfaces this directly in the tolerance hover text for the Da mode, and the same logic applies to ppm. If you only remember one thing from this doc, remember this sign convention. It is backwards from what most people expect on first read.

### Search strategies (there is no single right answer)

There are a few common schools of thought on setting precursor and fragment tolerance. None of these are Sage-specific defaults, they are practical patterns worth knowing before you touch the sliders.

**Wide window / open search**

Something like `[-100, 500]` Da lets you discover unexpected PTMs without listing them as variable mods. This is powerful but expands the search space a lot. Semi-enzymatic search will make this explode combinatorially, so keep the enzyme setting strict (fully tryptic, no semi-enzymatic) when running wide open, or the run time and false-match rate both get out of hand.

**Tight ppm search**

Something like `10/10 ppm`, matched to detector accuracy, is the classic narrow search. The catch: if you go this tight without isotope errors enabled, you will miss real IDs where the instrument picked the wrong isotope peak as monoisotopic. Isotope errors of at least `-1/0/1` (sometimes out to `-1/0/1/2/3`) are worth considering any time you run a tight ppm window. `isotope_errors` in `ui.rs` defaults to `-1..3`, which already covers this, but it is worth understanding why that default exists rather than just leaving it.

**Wide MS1, tight MS2 (hybrid, detector-driven)**

A wider absolute Da window on precursor mass (MS1) combined with a tight ppm window on fragments (MS2), regardless of what generated the wide MS1 window. On an Orbitrap/Orbitrap run this might look like `[-1.25, 3.5]` Da on precursor and `[-20, 20]` ppm on fragments. This gives you tolerance for small mass-calibration drift or minor unaccounted modifications on the precursor side, while keeping fragment matching strict since fragment mass accuracy is usually the more reliable signal on modern instruments.

There is no universal setting. Pick based on what you're trying to find (known peptides vs. unknown PTMs) and what your instrument actually supports.

### Combine Charge States (LFQ Settings)

`combine_charge_states` lives in `quant.lfq_settings`, between PPM Tolerance and Spectral Angle in the Quant tab.

- **True (default):** all charge states of the same peptide are summed into one LFQ feature. Peptide ABC seen at +2, +3, and +4 collapses into a single row with a single intensity value. Because the row spans multiple charges, Sage writes `charge = -1` in `lfq.tsv` as a sentinel, not an error.
- **False:** each charge state is kept as its own LFQ feature. Peptide ABC produces three separate rows, one per charge, each with a real charge value and its own intensity.

Why this exists: peptides often elute across multiple charge states at once. Summing them into one feature is more robust and avoids fragmenting the true abundance signal across several smaller, noisier values. The tradeoff is you lose per-charge resolution in the output. If a downstream tool or analysis needs real charge values per row (e.g. matching PSM-level charge from `results.sage.tsv`), turn this off.

If you see `charge = -1` everywhere in `lfq.tsv` and it looks wrong, check this setting before assuming it's a bug. It is expected behavior when the flag is `true`.

### Quick reference

| Parameter | Location | Default | Effect |
|---|---|---|---|
| Precursor Tolerance | Search tab | `[-10, 10]` ppm | Window around experimental precursor mass |
| Fragment Tolerance | Search tab | `[-10, 10]` ppm | Window around experimental fragment mass |
| Isotope Errors | Search tab | `[-1, 3]` | Extra C13 offsets checked, avoids missed monoisotopic picks |
| Wide Window | Search tab | off | DIA-style search mode, ignores Precursor Tolerance entirely |
| Combine Charge States | Quant tab | on | Merges LFQ features across charge states, writes `-1` charge when on |
| Semi-Enzymatic | Search tab (Enzyme) | off | Allows one non-enzymatic terminus, doubles search space, avoid combining with wide window |
