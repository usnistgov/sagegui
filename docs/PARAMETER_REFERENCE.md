# SageGUI Parameter Reference

A guide to every control in the SageGUI interface. For each parameter, this document explains what it does, when to change it, and what the default means.

**Table of Contents:**
- [Experiment Tab](#experiment-tab)
- [Files & Database Tab](#files--database-tab)
- [Search Tab](#search-tab)
- [Modifications Tab](#modifications-tab)
- [Quant Tab](#quant-tab)
- [Run / Info Tab](#run--info-tab)
- [Glossary](#glossary)

---

## Experiment Tab

### Experiment Type

**Default:** Custom

**Options:** Custom, Tryptic LFQ, Wide Open, Phospho, Semi-Tryptic

**What it does:**
Selects a preset configuration archetype that populates recommended values for Search, Modifications, and Quant tabs. Currently inert (selecting an option does not yet apply the preset defaults).[file:250]

**When to change it:**
- Select **Tryptic LFQ** for standard trypsin digestion with label free quantification
- Select **Wide Open** for discovery mode searches (wide precursor tolerance, fewer restrictions)
- Select **Phospho** for phosphoproteomics experiments
- Select **Semi Tryptic** to allow peptides with one non enzymatic terminus
- Select **Custom** if you want to manually set all parameters

**Planned:** Archetype presets will automatically populate the other tabs in a future release.[file:250]

---

### Save / Load Config

**Status:** Coming in a future version.

**What it will do:**
Save your current configuration (all Search, Modifications, and Quant settings) to a JSON file for later reuse, or load a previously saved configuration.[file:250]

**Why it is not ready yet:**
The SageGUI config format differs from Sage’s native JSON schema, so a round trip import and export needs careful field mapping to avoid losing data.[file:250]

---

## Files & Database Tab

### Pick mzML Files

**What it does:**
Opens a file browser to select one or more mzML (or mzML.gz) mass spectrometry data files to search.[file:250]

**When to change it:**
Every time you want to run a new search. All selected files are searched together.

**Format notes:**
- Accepts `.mzML` or `.mzML.gz`
- Does not natively support Thermo `.raw` or Bruker `.d` – convert these to mzML first

---

### Add FASTA…

**What it does:**
Opens a file browser to add one or more protein database (FASTA) files. Selected FASTAs are concatenated automatically at search time.[file:250]

**When to change it:**
Every search. Typically add:
1. Target organism FASTA (for example human UniProt)
2. Contaminant FASTA (for example cRAP, bovine serum)
3. Any spike in proteins

**Order matters:** FASTAs are concatenated in the order you add them. Decoys are generated from the concatenated database.[file:250]

**Format notes:**
- Standard FASTA format (`.fasta`, `.fa`, `.faa`)
- No deduplication of identical headers. If you add the same protein twice, it will appear twice in the search.
- The concatenated FASTA is resolved at launch and not persisted on disk.[file:250]

---

### Database prefiltering (FASTA chunking)

Prefiltering trades CPU time for peak memory. It is off by default and most searches do not need it.

**What happens at defaults (prefiltering off)**

Sage reads the whole FASTA, digests every protein, and builds one fragment index covering every peptide. Peak memory scales with the size of that index. The index grows with database size, missed cleavages, and variable modifications — and most sharply with semi-enzymatic or non-specific digestion, which yields many more peptides per protein.

All three fields below are inert at defaults. `prefilter` is `false`, so the prefiltering pass never runs. `prefilter_chunk_size` is `0` and `prefilter_low_memory` is `true`, but neither is read while `prefilter` is `false`.

**What prefiltering changes**

With `prefilter = true`, Sage splits the FASTA into chunks of `prefilter_chunk_size` protein sequences and handles them one at a time:

1. Digest one chunk and build a fragment index for only those proteins.
2. Quick-score every MS2 spectrum against that chunk index.
3. Keep the peptides that matched a spectrum. Discard the rest and free the chunk index.
4. Repeat for the next chunk.

After the last chunk, Sage builds one final index from the kept peptides and runs the normal search against it.

Peak memory becomes roughly the larger of one chunk index or the final filtered index, instead of the full-database index.

**When to use it**

Turn it on when the search space is large enough to exhaust memory:

- Semi-enzymatic or non-specific digestion (HLA peptidomics, immunopeptidomics)
- Many variable modifications, or a raised `max_variable_mods`
- Very large FASTA files (metaproteomes, large sequence libraries)

Leave it off for normal tryptic searches at normal database sizes. The pass costs CPU time and returns nothing when the full index already fits in memory.

**Costs and caveats**

- **Extra CPU.** Every spectrum is scored once per chunk before the real search starts.
- **Slightly different results.** Decoys are generated per chunk and are not regenerated over the final filtered set, so PSM counts can differ a little from a non-prefiltered run of the same data.
- **No progress readout.** The pass runs inside database construction, where SageGUI has no progress signal. The run bar stays at 0% for its duration.
- **File-count cliff.** If SageGUI's thread count (half the CPU cores) is at least the number of mzML files, Sage reads all spectra once and holds them in memory for the whole pass. Otherwise it re-reads and re-processes every mzML file once per chunk, which is slow when there are many chunks. On a 16-core machine the switch happens at 9 files.

**Fields**

- `database.prefilter` (bool, default `false`)

  Enables the prefiltering pass described above.

- `database.prefilter_chunk_size` (integer, default `0` = auto)

  How many FASTA sequences are digested and scored per chunk. Larger values mean fewer chunks and less repeated work, but higher peak memory.

  `0` lets Sage choose. It estimates total peptide count from the digest, the variable-mod count and `max_variable_mods`, and targets about 8.4 million peptides per chunk. If the whole search space already fits inside one chunk, Sage skips prefiltering entirely — so auto is safe to leave on. Set a positive value for reproducible chunking or to force more aggressive splitting.

  Ignored when `prefilter = false`.

- `database.prefilter_low_memory` (bool, default `true`)

  Controls how aggressively each chunk is filtered.

  - `true` — score every preliminary hit and keep only the top `report_psms + 1` per spectrum per chunk. Fewest peptides retained, lowest memory, most CPU. This is Sage's own default.
  - `false` — keep every preliminary hit without scoring it. More peptides retained, more memory, less CPU, and FDR behaviour closer to a non-prefiltered search.

  Set this to `false` only if you have memory to spare and want results as close as possible to a non-prefiltered run.

  Ignored when `prefilter = false`.

---

### Advanced: Generate Decoys

**Default:** ✓ Checked (true)

**What it does:**
Automatically generates reversed sequence decoys from your database for FDR estimation. Sage appends these to your database during the search.[file:250]

**When to change it:**
- Leave ON for standard searches (required for FDR calculation)
- Turn OFF only if you are providing pre generated decoys in a separate FASTA

**Why it matters:**
FDR filtering relies on comparing target matches to decoy matches. Without decoys, you cannot reliably estimate false positives.[file:250]

---

### Advanced: Bucket Size

**Default:** 32,768

**Range:** 8,192 – 65,536

**What it does:**
Controls the peptide index bucket size that Sage uses internally. Affects speed only and does not change results.[file:250]

**When to change it:**
- Set to **8,192** for high resolution MS2 (Orbitrap, TOF)
- Set to **32,768** as a middle ground (default)
- Set to **65,536** for low resolution MS2 (ion trap)

**Why it matters:**
Smaller buckets give finer mass granularity and can be faster for high resolution instruments. Larger buckets suit lower resolution instruments and larger fragment mass errors.[file:250]

---

### Advanced: min_ion_index

**Default:** 2

**What it does:**
Minimum number of fragment ions required for a peptide to be indexed in the search database. Peptides with fewer fragments are skipped, which saves memory and speeds up the search.[file:250]

**When to change it:**
Rarely. A value of 2 is safe for most searches.

---

### Advanced: Decoy Tag

**Default:** `rev_`

**What it does:**
Prefix applied to protein names in the auto generated decoy sequences. Used to distinguish decoy matches in the results.[file:250]

**When to change it:**
Only if your protein names already start with `rev_`. Use a unique prefix like `decoy_` instead to avoid collisions.

---

## Search Tab

> **Note on search strategies:** A few common patterns are worth keeping in mind before changing tolerances.
>
> - Tight ppm precursor and fragment tolerances for known modification space on accurate instruments.
> - Wide precursor Da window with strict trypsin and standard fragment tolerance for open PTM discovery.[web:53][file:250]
> - Hybrid strategy with a wider MS1 Da window and a tight MS2 ppm window so fragment matching remains strict.

### Precursor Tolerance

**Mode:** PPM (parts per million) or Da (Daltons)

**Default:** PPM, Lower: –10, Upper: +10

**What it does:**
Defines the mass window Sage searches around the observed precursor m/z. Sage applies the window to the experimental precursor mass and looks for theoretical peptides whose mass falls within

`[observed + lower, observed + upper]`.[file:250]

**Sign convention (important):**
- Lower is normally negative and Upper is normally positive.[file:250]
- A negative lower bound searches theoretical peptides that are lighter than the observed mass. This corresponds to observed peptides that carry a positive delta mass relative to the unmodified theoretical peptide.[file:250]
- Example: `Lower = –500` searches theoretical peptides up to 500 Da below the observed mass. Those candidates would require a +500 Da modification to explain the observed m/z.[file:250]

**Examples:**
- **Closed search, tight ppm:** `PPM [–10, +10]` around the observed mass. Typical for high accuracy instruments.[file:250]
- **Open search, wide Da:** `Da [–100, +500]` searches from 100 Da below to 500 Da above the observed mass and is a common wide window for PTM discovery.[web:53]
- **Hybrid strategy:** a wider MS1 Da window with a tight MS2 ppm window. On Orbitrap MS1 and Orbitrap MS2, an example is `MS1 Da [–1.25, +3.5]` and `MS2 PPM [–20, +20]`. MS1 absorbs small mass drift or minor unmodelled modifications. MS2 remains strict because fragment mass accuracy is usually the more reliable signal.

**When to change it:**
- Use tight ppm (for example `±10 ppm`) when the detector is well calibrated and your modification space is defined.
- Use a wide Da window (for example `–100, +500`) when you want to discover unexpected PTMs. Keep the enzyme strict (fully tryptic, no semi enzymatic) or the search space grows very large.[web:53][file:250]
- Combine a wider MS1 window with a tight MS2 window when you care more about fragment evidence than about exact precursor calibration.

**Warning:** If Lower is greater than Upper, the window is inverted and Sage will compute an empty search range. The lower value should normally be the smaller number, often negative.[file:250]

---

### Fragment Tolerance

**Mode:** PPM or Da

**Default:** PPM, Lower: –10, Upper: +10

**What it does:**
Defines the mass window for fragment ion matching. Sage looks for theoretical fragments within `[observed + lower, observed + upper]` of each observed fragment.[file:250]

**When to change it:**
- High resolution MS2 (Orbitrap): around `±10 ppm`
- Low resolution MS2 (ion trap): for example `±20 ppm` or a small Da window
- Wider tolerance yields more matches but more false positives. Narrower tolerance is stricter and can be faster.

---

### Charge Handling

#### Precursor Charge Min / Max

**Default:** Min: 2, Max: 4

**What it does:**
Specifies the range of precursor charge states to search. Sage searches all charges from Min to Max inclusive for each observed precursor.[file:250][file:238]

**When to change it:**
- Leave at 2–4 for most LC MS experiments
- Change to 2–5 if you see many highly charged ions
- Change to 1–3 if your data is mostly singly or doubly charged
- The charge annotation in your mzML is used by default. This range is the fallback for unknown charges.

---

#### Override Precursor Charge (Advanced)

**Default:** ☐ Unchecked (false)

**What it does:**
When enabled, forces Sage to ignore the charge annotation in your mzML file and instead search all charges in the Precursor Charge Min and Max range for every precursor.[file:250]

**When to enable it:**
- DIA and diaPASEF experiments where charge annotations may be missing or unreliable
- Leave off for standard DDA LC MS where mzML charge states are accurate

**Warning:** Enabling this makes the search slower because more charge states are tested per spectrum.

---

### Enzyme Settings

#### Missed Cleavages

**Default:** 2

**Range:** 0–5

**What it does:**
Maximum number of allowed missed cuts per peptide.[file:250]

**When to change it:**
- Set to 1 for faster, stricter searches
- Set to 2 for more thorough searches
- Higher values enlarge the search space and slow searches

---

#### Min Length / Max Length

**Default:** Min: 5, Max: 50

**What it does:**
Restricts peptide length in amino acids. Sage only considers peptides whose length is between Min and Max inclusive.[file:250]

---

#### Cleave At

**Default:** `KR`

**What it does:**
Residues where the enzyme cuts. Default `KR` is trypsin specificity.[file:250]

---

#### Restrict (Enable + Restrict Char)

**Default:** Enabled, Restrict Char: `P`

**What it does:**
Proline restriction for trypsin. When enabled, Sage does not cut at K or R when the next residue is P.[file:250]

---

#### C Terminal

**Default:** ✓ Checked

**What it does:**
Controls whether cleavage is applied at the C terminus of the specified residues (checked) or at the N terminus (unchecked).[file:250]

---

#### Semi Enzymatic

**Default:** ☐ Unchecked

**What it does:**
Allows one non enzymatic terminus. This doubles the search space.[file:250]

**When to enable it:**
Semi tryptic and exploratory searches. Avoid enabling it together with very wide precursor windows unless you accept a large runtime increase.

---

### Mass Ranges

#### Peptide Min Mass / Max Mass

**Default:** Min: 500 Da, Max: 5,000 Da

**What it does:**
Restricts the database to peptides with monoisotopic mass in this range.[file:250]

---

### Ion Kinds

**Default:** B and Y enabled

**What it does:**
Selects which fragment ion series Sage scores. Default is b and y ions.[file:250]

---

### Search Behavior

#### Isotope Errors

**Default:** Min: –1, Max: 3

**What it does:**
C13 isotope error offsets Sage will consider in addition to the base precursor mass. The hover text notes that this is slower than simply widening the precursor tolerance to cover the same mass range and suggests using a wider Da window when unsure.[file:250]

**When to change it:**
- Leave at –1 to 3 for high resolution instruments where isotope picking matters
- Set to 0 to 0 and widen precursor tolerance instead for faster searches
- Wider ranges add more possible offsets and increase runtime

---

#### Deisotope

**Default:** ☐ Unchecked

**What it does:**
Enables deisotoping of MS1 features before searching.[file:250]

---

#### Chimera

**Default:** ☐ Unchecked

**What it does:**
Enables chimeric spectrum handling where more than one peptide may be reported for a single MS2 event.[file:250]

---

#### Wide Window

**Default:** ☐ Unchecked

**What it does:**
Switches Sage into a DIA style wide window mode where the configured precursor tolerance is ignored and all charge states in the range are considered.[file:238][file:250]

---

### Scoring

#### Score Type

**Default:** SageHyperScore

**Options:** SageHyperScore, OpenMSHyperScore

**What it does:**
Scoring function used for PSM ranking. The hover text recommends leaving this at SageHyperScore unless you are explicitly comparing scoring functions.[file:250]

---

### Advanced

#### Min Peaks / Max Peaks

**Default:** Min: 15, Max: 150

**What it does:**
Minimum and maximum number of fragment peaks retained per MS2 spectrum after preprocessing.[file:250]

---

#### Min Matched Peaks

**Default:** 6

**What it does:**
Minimum number of matched fragment ions required for a PSM to be considered.[file:250]

---

#### Max Fragment Charge

**Default:** 1

**What it does:**
Maximum fragment charge state used when generating theoretical fragments.[file:250]

---

#### Report PSMs

**Default:** 1

**What it does:**
Number of PSMs to report per spectrum. Default 1 keeps only the top scoring match per spectrum.[file:238][file:250]

---

#### Predict RT (Retention Time)

**Default:** ✓ Checked

**What it does:**
Enables retention time prediction in Sage, which can be used as an additional score feature.[file:238][file:250]

---

## Modifications Tab

### Static (Fixed) Modifications

**Default:** Carbamidomethyl on C (+57.021464 Da)

**What it does:**
Fixed modifications applied to all occurrences of specified residues. Stored as Sage specificity keys with monoisotopic delta masses.[file:250]

---

### Variable Modifications

**Default:** Oxidation on M (+15.994915 Da)

**What it does:**
Variable modifications that may occur on specified residues. Sage searches all combinations up to the Max Variable Mods limit.[file:250]

---

### Common modification presets

Curated list including Acetyl K, Carbamidomethyl C, Carbamyl K, Deamidated NQ, Glu pyro Glu E, Gln pyro Glu Q, Methyl KR, Oxidation M, Oxidation P, Phospho STY, Trimethyl KR and others. Each preset carries Sage keys and Unimod based monoisotopic deltas and a short explanatory note.[file:250]

---

### Target / Add / Remove

**Target:** Selects whether preset operations act on Static or Variable.[file:250]

**Add:** Inserts the selected preset keys into the target box.[file:250]

**Remove:** Removes the selected preset keys from the target box.[file:250]

A modification cannot be both Static and Variable at once. Moving it into one box removes it from the other.[file:250]

---

### Custom modifications

**Key:** Sage specificity key string. Supports residue, peptide terminus and protein terminus forms. The UI shows key syntax examples.[file:250]

**Mass:** Monoisotopic delta mass in Da.[file:250]

**Add to target box:** Inserts the custom key and mass into the selected Static or Variable box.[file:250]

---

### Max Variable Mods

**Default:** 2

**Range:** 1–10

**What it does:**
Maximum number of variable modifications that can co occur on a single peptide.[file:250]

---

## Quant Tab

### Enable Quantification

**Default:** ✓ Checked

**What it does:**
Global toggle for quant features. When off, Sage runs identification only. When on, it enables either LFQ or TMT based on the selection.[file:250]

---

### Quantification Type

**Options:** Label Free Quantification (LFQ), Tandem Mass Tag (TMT)

**What it does:**
Selects the quantification method. LFQ uses MS1 intensities. TMT uses isobaric reporter ions.[file:250]

---

### LFQ Settings

LFQ settings are stored in `quant.lfq_settings` and shown as a grouped panel in the Quant tab.[file:238][file:250]

#### PPM Tolerance

**Default:** 5.0 ppm

**What it does:**
Precursor tolerance used by the LFQ engine when mapping features across runs.[file:250]

---

#### Spectral Angle

**Default:** 0.7

**Range:** 0.0–1.0

**What it does:**
Similarity threshold between experimental and reference spectra used for LFQ scoring. Peaks below this threshold are filtered out.[file:250]

---

#### Combine Charge States

**Default:** ✓ Checked (true)

**What it does:**
- When on, all charge states of the same peptide are merged into one LFQ feature. Peptide ABC at charges 2, 3, and 4 produces a single row with a single intensity value. Because the row spans multiple charges, Sage records `charge = –1` in `lfq.tsv` as a sentinel value.[file:238][file:145][file:250]
- When off, each charge state is kept as its own feature. `lfq.tsv` contains separate rows per charge with real charge values.[file:145][file:250]

**Why it exists:**
Peptides often elute across multiple charge states in the same run. Summing them into one feature is more robust and avoids splitting the true abundance signal across several smaller and noisier values. This is the intended default behavior for peptide level LFQ.[file:59][file:244]

**Performance note:**
Turning this off increases LFQ runtime because Sage must trace and integrate a separate feature for every charge state of every peptide instead of one combined feature per peptide.[file:145][file:238]

**When to change it:**
- Leave on for standard peptide level quantification and most downstream pipelines, including MSstats.
- Turn off only when you need charge resolved intensities or a downstream tool expects a real charge in every LFQ row.

---

#### Peptide q value

**Default:** optional, set to 0.01 in your example config

**What it does:**
Optional per peptide q value filter in the LFQ engine. Peptides with q values above the threshold are excluded from LFQ.[file:238]

---

### TMT Settings

#### TMT Plex

**Options:** TMT 6 plex, 10 plex, 11 plex, 16 plex, 18 plex

**What it does:**
Chooses the isobaric tagging scheme.[file:250]

---

#### Level

**Default:** 3

**What it does:**
MS level used for TMT quantification. Default 3 is typical for TMT SPS MS3.[file:238][file:250]

---

#### S/N

**Default:** false

**What it does:**
Toggles a signal to noise based filter in TMT quantification.[file:238][file:250]

---

## Run / Info Tab

### Output Location

**Default:** current working directory

**What it does:**
Directory where `results.sage.tsv`, `lfq.tsv`, and other outputs are written.[file:238][file:250]

---

### Write PIN File

**Default:** ☐ Unchecked

**What it does:**
Writes a Percolator `.pin` file for downstream rescoring.[file:250]

---

### Annotate Matches

**Default:** ☐ Unchecked

**What it does:**
Writes annotated fragment ion match detail alongside results when enabled.[file:250]

---

### Info / Help

Shows SageGUI version, Sage engine version, author and maintainer, repository links and the recommended Sage citation.[file:250]

---

## Glossary

**Decoy:** Noise protein sequence generated by reversing target sequences. Used for FDR estimation.

**FDR (False Discovery Rate):** Proportion of false positives among all positive identifications. Often controlled at one percent PSM level and five percent protein level.

**Feature:** MS1 precursor ion described by mass, charge, retention time and isotopic envelope. LFQ traces features across runs.

**LFQ:** Label free quantification. Derives peptide and protein intensity from MS1 precursor peak areas.

**m/z:** Mass to charge ratio.

**PPM:** Parts per million. Relative mass accuracy measure.

**PSM:** Peptide spectrum match.

**PTM:** Post translational modification.

**Spectral Angle:** Similarity measure between peak lists on a zero to one scale where one is identical. Used in LFQ isotope profile comparison.

**TMT:** Tandem Mass Tag.

**Trypsin:** Protease that cuts after K or R except when followed by P.

---

## Related Documentation

- **Sage official docs:** https://github.com/lazear/sage[web:59]
- **SageGUI README:** `../README.md`
- **NOTES.md:** locked design decisions and known limitations
- **PLAN.md:** roadmap for future features