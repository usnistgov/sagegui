## Mass Tolerance and Charge Parameters

Reference notes for `sageGUI` parameters that are easy to misread or misconfigure. Written for both the `/docs/` folder and future hover-text.

### How Sage applies mass tolerance (read this first)

SageGUI shows each tolerance as a **delta-mass range**: "Delta mass from" and "to". Delta mass is experimental mass minus theoretical mass. A peptide that carries a +500 Da modification has a delta mass of +500 Da.

Sage's own JSON stores the window the other way. Sage searches theoretical candidate masses in

```
[experimental + lower, experimental + upper]
```

so the JSON pair is the delta-mass range **negated and swapped**:

| Delta-mass range (SageGUI) | Sage JSON |
| --- | --- |
| −1.25 to +3.5 Da | `"da": [-3.5, 1.25]` |
| −100 to +500 Da | `"da": [-500, 100]` |
| −10 to +10 ppm | `"ppm": [-10, 10]` (symmetric, so the same) |

SageGUI converts in both directions. Symmetric ppm windows hide the difference, so take care with asymmetric and open windows. In this file, a range is a delta-mass range unless it is shown as JSON.

### Search strategies (there is no single right answer)

There are a few common schools of thought on setting precursor and fragment tolerance. None of these are Sage-specific defaults, they are practical patterns worth knowing before you touch the sliders.

**Wide window / open search**

A delta-mass range of −100 to +500 Da lets you discover unexpected PTMs without listing them as variable mods. This is powerful but expands the search space a lot. Semi-enzymatic search will make this explode combinatorially, so keep the enzyme setting strict (fully tryptic, no semi-enzymatic) when running wide open, or the run time and false-match rate both get out of hand.

**Tight ppm search**

Something like −10 to +10 ppm, matched to detector accuracy, is the classic narrow search. The catch: if you go this tight without isotope errors enabled, you will miss real IDs where the instrument picked the wrong isotope peak as monoisotopic. Isotope errors of at least `-1/0/1` (sometimes out to `-1/0/1/2/3`) are worth considering any time you run a tight ppm window. `isotope_errors` in `ui.rs` defaults to `-1..3`, which already covers this, but it is worth understanding why that default exists rather than just leaving it.

**Wide MS1, tight MS2 (hybrid, detector-driven)**

A wider absolute Da window on precursor mass (MS1) combined with a tight ppm window on fragments (MS2), regardless of what generated the wide MS1 window. On an Orbitrap/Orbitrap run this might look like −1.25 to +3.5 Da on precursor and −10 to +10 ppm on fragments, which is what the Tryptic - wide MS1 / tight MS2 template sets. This gives you tolerance for small mass-calibration drift or minor unaccounted modifications on the precursor side, while keeping fragment matching strict since fragment mass accuracy is usually the more reliable signal on modern instruments.

There is no universal setting. Pick based on what you're trying to find (known peptides vs. unknown PTMs) and what your instrument actually supports.

### Combine Charge States (LFQ Settings)

`combine_charge_states` lives in `quant.lfq_settings`. It is the last LFQ setting on the Quant tab.

- **True (default):** all charge states of the same peptide are summed into one LFQ feature. Peptide ABC seen at +2, +3, and +4 collapses into a single row with a single intensity value. Because the row spans multiple charges, Sage writes `charge = -1` in `lfq.tsv` as a sentinel, not an error.
- **False:** each charge state is kept as its own LFQ feature. Peptide ABC produces three separate rows, one per charge, each with a real charge value and its own intensity.

Why this exists: peptides often elute across multiple charge states at once. Summing them into one feature is more robust and avoids fragmenting the true abundance signal across several smaller, noisier values. The tradeoff is you lose per-charge resolution in the output. If a downstream tool or analysis needs real charge values per row (e.g. matching PSM-level charge from `results.sage.tsv`, or MSstats), turn this off.

If you see `charge = -1` everywhere in `lfq.tsv` and it looks wrong, check this setting before assuming it's a bug. It is expected behavior when the flag is `true`.

### Quick reference

| Parameter | Location | Default | Effect |
|---|---|---|---|
| Precursor Tolerance | Search tab | delta −10 to +10 ppm | Delta-mass window for the precursor |
| Fragment Tolerance | Search tab | delta −10 to +10 ppm | Delta-mass window for fragments |
| Isotope Errors | Search tab | −1 to 3 | Extra C13 offsets checked, avoids missed monoisotopic picks |
| Wide Window | Search tab | off | DIA-style search mode, uses each isolation window instead of Precursor Tolerance |
| Combine Charge States | Quant tab | on | Merges LFQ features across charge states, writes `-1` charge when on |
| Semi-Enzymatic | Search tab (Enzyme) | off | Allows one non-enzymatic terminus, doubles search space, avoid combining with a wide precursor window |
