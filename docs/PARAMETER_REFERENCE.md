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

### Templates

**Default:** the first template in the list, Tryptic - wide MS1 / tight MS2. Nothing changes until you click **Apply template**.

**Options:** Tryptic - wide MS1 / tight MS2, Tryptic - tight, Tryptic - open, Tryptic - biofluid, TMT 11-plex

**What it does:**
A template is a complete set of search parameters for a common experiment type. **Apply template** replaces the current search parameters with the template's values. It never changes the selected mzML files, the FASTA files or the output folder. Applying the same template again restores every setting it covers. The text under the picker describes the selected template.

**What each template sets** (tolerances are delta-mass ranges, as the Search tab shows them):
- **Tryptic - wide MS1 / tight MS2:** precursor −1.25 to +3.5 Da, fragment −10 to +10 ppm, no isotope errors. The wide MS1 window absorbs isotope errors.
- **Tryptic - tight:** precursor and fragment −20 to +20 ppm, isotope errors −1 to 3.
- **Tryptic - open:** precursor −100 to +500 Da, to find unknown modifications. Expect a long run.
- **Tryptic - biofluid:** semi-enzymatic digestion with database prefiltering on, to bound memory.
- **TMT 11-plex:** static TMT on K and the peptide N-terminus, with a 0.4 Da fragment window that suits an ion trap.

The template files are in `assets/templates/`, in Sage's own JSON format.

---

### Load settings from a Sage file

**What it does:**
**Load Sage config or results.json…** reads the search parameters from a Sage `config.json` or from the `results.json` that a past search wrote to its output folder. It never reads the file paths or the output folder in the file. After a load, the tab lists anything it could not apply, and the files you must select again.

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

Sage reads the whole FASTA, digests every protein, and builds one fragment index covering every peptide. Peak memory scales with the size of that index. The index grows with database size, missed cleavages, and variable modifications. It grows most sharply with semi-enzymatic or non-specific digestion, which yields many more peptides per protein.

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
- **Coarse progress readout.** The pass is part of the Digest proteins step. The Progress card shows which FASTA chunk is being searched, for example "prefilter chunk 3 of 9", but no percentage inside a chunk.
- **File-count cliff.** If SageGUI's thread count (half the CPU cores) is at least the number of mzML files, Sage reads all spectra once and holds them in memory for the whole pass. Otherwise it re-reads and re-processes every mzML file once per chunk, which is slow when there are many chunks. On a 16-core machine the switch happens at 9 files.

**Fields**

- `database.prefilter` (bool, default `false`)

  Enables the prefiltering pass described above.

- `database.prefilter_chunk_size` (integer, default `0` = auto)

  How many FASTA sequences are digested and scored per chunk. Larger values mean fewer chunks and less repeated work, but higher peak memory.

  `0` lets Sage choose. It estimates total peptide count from the digest, the variable-mod count and `max_variable_mods`, and targets about 8.4 million peptides per chunk. If the whole search space already fits inside one chunk, Sage skips prefiltering entirely. So auto is safe to leave on. Set a positive value for reproducible chunking or to force more aggressive splitting.

  Ignored when `prefilter = false`.

- `database.prefilter_low_memory` (bool, default `true`)

  Controls how aggressively each chunk is filtered.

  - `true`: score every preliminary hit and keep only the top `report_psms + 1` per spectrum per chunk. Fewest peptides retained, lowest memory, most CPU. This is Sage's own default.
  - `false`: keep every preliminary hit without scoring it. More peptides retained, more memory, less CPU, and FDR behaviour closer to a non-prefiltered search.

  Set this to `false` only if you have memory to spare and want results as close as possible to a non-prefiltered run.

  Ignored when `prefilter = false`.

---

### Advanced: Generate Decoys

**Default:** ✓ Checked (true)

**What it does:**
Automatically generates reversed sequence decoys from your database for FDR estimation. Sage appends these to your database during the search.[file:250]

**When to change it:**
- Leave ON for standard searches (required for FDR calculation)
- Turn OFF only if your FASTA already holds decoys whose accessions carry the Decoy Tag. With it on, Sage drops those entries and makes its own.

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
Fragment ions below this index are left out of the fragment index that Sage uses for the first-pass search. The default of 2 leaves out b1, b2, y1 and y2, which are rarely informative. Full scoring of each candidate peptide is not affected.

**When to change it:**
Rarely. A value of 2 is safe for most searches.

---

### Advanced: Decoy Tag

**Default:** `rev_`

**What it does:**
Prefix that marks a decoy protein. With Generate Decoys on, Sage puts it on the decoys it makes, and drops any FASTA entry whose accession already contains it. With Generate Decoys off, Sage uses the FASTA entries that contain it as the decoys.

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

**Mode:** PPM (parts per million) or Da (Daltons). Switching the unit resets the window to −10 to +10 ppm or −0.02 to +0.02 Da.

**Default:** PPM, delta mass −10 to +10

**What it does:**
Sets the precursor mass window. The Search tab shows it as a delta-mass range: **Delta mass from** and **to**. Delta mass is the experimental precursor mass minus the theoretical peptide mass. A peptide that carries a +16 Da modification has a delta mass of +16 Da. Sage matches a spectrum to every peptide whose delta mass is inside the range.

**Sage JSON convention (important):**
Sage's own `precursor_tol` pair in a JSON file is **not** the delta-mass range. It is the delta-mass range negated and swapped. Delta −1.25 to +3.5 Da is `"da": [-3.5, 1.25]` in the JSON. SageGUI converts in both directions, when it writes a search and when it loads a Sage file. A symmetric window, such as −10 to +10 ppm, looks the same in both conventions, so the difference shows only on asymmetric and open windows.

**Examples (delta mass, as entered in SageGUI):**
- **Closed search, tight ppm:** −10 to +10 ppm. Typical for high accuracy instruments.
- **Open search, wide Da:** −100 to +500 Da finds modifications from −100 to +500 Da. In Sage JSON this is `"da": [-500, 100]`. The Tryptic - open template sets it.
- **Hybrid strategy:** a wider MS1 Da window with a tight MS2 ppm window. The Tryptic - wide MS1 / tight MS2 template sets precursor −1.25 to +3.5 Da and fragment −10 to +10 ppm. MS1 absorbs isotope errors and small mass drift. MS2 remains strict because fragment mass accuracy is usually the more reliable signal.

**When to change it:**
- Use tight ppm (for example −10 to +10 ppm) when the detector is well calibrated and your modification space is defined.
- Use a wide Da window (for example −100 to +500 Da) when you want to discover unexpected PTMs. Keep the enzyme strict (fully tryptic, no semi enzymatic) or the search space grows very large.
- Combine a wider MS1 window with a tight MS2 window when you care more about fragment evidence than about exact precursor calibration.

**Warning:** If the first value is greater than the second, the window is empty and Sage finds nothing. The Search tab shows a warning when this happens.

---

### Fragment Tolerance

**Mode:** PPM or Da

**Default:** PPM, delta mass −10 to +10

**What it does:**
Sets the mass window for fragment ion matching. The Search tab shows it as a delta-mass range, in the same way as the precursor tolerance: experimental fragment mass minus theoretical fragment mass. Sage's JSON `fragment_tol` pair uses the same negated and swapped convention as `precursor_tol`.

**When to change it:**
- High resolution MS2 (Orbitrap): around `±10 ppm`
- Low resolution MS2 (ion trap): for example `±20 ppm` or a small Da window
- Wider tolerance yields more matches but more false positives. Narrower tolerance is stricter and can be faster.

---

### Charge Handling

#### Precursor Charge Min / Max

**Default:** Min: 2, Max: 4

**What it does:**
The precursor charges Sage tries when it cannot use the charge in the file. A spectrum with a charge annotation in the mzML is searched at that charge only. Sage tries every charge from Min to Max inclusive for a spectrum with no charge annotation, for every spectrum when **Override Precursor Charge** is on, and for every spectrum in **Wide Window** mode.

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

#### Enzyme (preset picker)

**Default:** trypsin

**What it does:**
Sets the cut rule from a list of 14 curated proteases. Selecting one writes
Cleave At, Restrict and Cut side. It does **not** change Missed Cleavages, Min
Length, Max Length or Semi-Enzymatic, which are tuning rather than identity.

The picker shows the enzyme you currently have, whether you chose it here, set
it by hand, or loaded it with a template. It reads `Custom` when the rule is
not one of the presets.

| Enzyme | Cleaves at | Do not cut before | Cut side |
|--------|-----------|-------------------|----------|
| trypsin | K, R | P | after |
| trypsin/p | K, R | | after |
| arg-c | R | P | after |
| asp-n | D | | **before** |
| asp-n/ambic | D, E | | **before** |
| chymotrypsin | F, Y, W, L | P | after |
| cnbr | M | | after |
| lys-c | K | P | after |
| lys-c/p | K | | after |
| lys-n | K | | **before** |
| pepsin-a | F, L | | after |
| trypchymo | F, Y, W, L, K, R | P | after |
| glu-c | E | P | after |
| glu-c/de | D, E | P | after |

Ported from [sageRecon](https://github.com/usnistgov/sageRecon), whose source
is Mascot's published enzyme list. Ambiguity codes (B, Z) are dropped, because
Sage rejects them. Enzymes needing more than one cleavage rule are not
included, because Sage holds only one.

Two buffer-dependent proteases appear twice on purpose. Use `glu-c` in
phosphate buffer and `glu-c/de` in ammonium bicarbonate; the same distinction
separates `asp-n` from `asp-n/ambic`.

---

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
Residues where the enzyme cuts, in capitals. Default `KR` is trypsin
specificity.[file:250]

Only residues Sage accepts are allowed: `ACDEFGHIKLMNPQRSTVWYUO`. Anything
else, including lower case and the ambiguity codes B, Z, J and X, is reported
before the search starts. Sage rejects them in a way the interface cannot
recover from once a run has begun.

Two special values:

- **Empty** means a non-specific search. Sage cuts everywhere, which is very
  large. Turn on database prefiltering if you use it.
- **`$`** means no digestion at all.

---

#### Restrict (Enable + Restrict Char)

**Default:** Enabled, Restrict Char: `P`

**What it does:**
Proline restriction for trypsin. When enabled, Sage does not cut at K or R when the next residue is P.[file:250]

---

#### Cut side

**Default:** After the residue

**What it does:**
Whether the enzyme cuts after the Cleave At residue or before it.[file:250]

Most proteases cut after. Three of the presets cut before: `asp-n`,
`asp-n/ambic` and `lys-n`. Getting this wrong produces a digest that shares no
peptides with the intended one, so it is shown as two labelled choices rather
than a single checkbox.

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
Selects which fragment ion series Sage scores. Default is b and y ions, which suit CID and HCD.[file:250] For ETD or ECD, select c and z, or c and z• (the radical z ion).

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

**Default:** ☐ Unchecked in SageGUI. The Sage command line defaults to on.

**What it does:**
Deisotopes MS2 spectra and deconvolutes fragment charge states before scoring.

---

#### Chimera

**Default:** ☐ Unchecked

**What it does:**
Searches for co-fragmenting peptides in each MS2 spectrum. Sage reports up to **Report PSMs** of them per spectrum, so set Report PSMs above 1 when Chimera is on.

---

#### Wide Window

**Default:** ☐ Unchecked

**What it does:**
For DIA or wide-window data. Sage ignores the precursor tolerance and uses each spectrum's isolation window instead. It tries every precursor charge in the Precursor Charge range.

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
**Min Peaks:** Sage does not search a spectrum with fewer peaks than this. **Max Peaks:** Sage keeps only this many of the most intense peaks in each spectrum.

---

#### Min Matched Peaks

**Default:** 6

**What it does:**
Fewest matched fragment ions for a PSM to be reported. Every bundled template uses 4.

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
Aligns retention times across files and predicts each peptide's retention time. Sage uses the prediction as a feature for FDR. **LFQ needs it:** with Predict RT off, LFQ does not run, and the Quant step on the Progress card says so.

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
Selects the quantification method. LFQ uses MS1 intensities. TMT uses isobaric reporter ions.[file:250] LFQ needs **Predict RT** on the Search tab.

---

### LFQ Settings

LFQ settings are stored in `quant.lfq_settings` and shown as a grouped panel in the Quant tab.[file:238][file:250]

#### How Sage LFQ works

Sage quantifies peptides by MS1 ion current extraction, in the same way as FlashLFQ and IonStar. The steps are as follows:

1. Sage selects the peptides that pass the **Peptide q-value** limit. It also makes a decoy for each peptide, to control the false MS1 integration rate.
2. For each peptide, Sage collects MS1 intensity within the **PPM Tolerance** and a fixed retention time window. It does this for the first three isotopes in every file.
3. Sage puts the intensities on a grid of 100 retention time points. It aligns the files to each other on this grid.
4. At each grid point, Sage calculates the spectral angle between the observed and the expected isotope envelope.
5. Sage uses **Peak Scoring** to find the best grid point (the apex). The apex must pass the **Spectral Angle** limit.
6. Sage extends the peak to the left and to the right of the apex. It stops at a point that scores below half of the apex score, at a point below the **Spectral Angle** limit, or at one fifth of the grid on each side.
7. Sage uses **Integration** to calculate one intensity for each file.

LFQ needs retention time prediction (`predict_rt`). With it off, the Quant step on the Progress card says that LFQ did not run. For the upstream description, see the [Sage quantification docs](https://sage-docs.vercel.app/docs/configuration/quantification) and [how Sage works](https://sage-docs.vercel.app/docs/how_it_works).

#### Integration

**Default:** Sum (area)

**Options:** Sum (area), Apex. JSON values: `"Sum"`, `"Apex"`.

**What it does:**
Selects how Sage calculates one intensity per file from the MS1 trace of the picked peak.

- **Sum (area):** the sum of the trace across the full peak, from the left edge to the right edge (steps 6 and 7 above). This is the peak area.
- **Apex:** the trace height at the apex point only.

**When to change it:**
Sum uses more of the signal, so it is usually less noisy for peaks that are well defined. Apex is not affected by where Sage puts the peak edges. Thus, Apex can be more stable when a peak is narrow or when a nearby ion widens the peak. We have not compared the two options on a benchmark data set. Thus, we cannot recommend one over the other for all data.

This option was added in answer to a feature request from MJanech ([usnistgov/sagegui#1](https://github.com/usnistgov/sagegui/issues/1)).

---

#### Peak Scoring

**Default:** Hybrid

**Options:** Hybrid, Retention time, Spectral angle, Intensity. JSON values: `"Hybrid"`, `"RetentionTime"`, `"SpectralAngle"`, `"Intensity"`.

**What it does:**
Selects the score that Sage uses to find the apex on the retention time grid (step 5 above).

- **Hybrid:** combines the three scores below. It is the spectral angle cubed, times the retention time score, times the intensity score.
- **Retention time:** prefers the point nearest to the expected retention time.
- **Spectral angle:** prefers the point with the best isotope envelope match.
- **Intensity:** prefers the most intense point.

**When to change it:**
The Sage documentation recommends that you do not change this setting. The upstream documentation lists Hybrid, RetentionTime and SpectralAngle. The Sage code also accepts Intensity.

---

#### PPM Tolerance

**Default:** 5.0 ppm

**What it does:**
The MS1 m/z tolerance, in ppm, that Sage uses to collect the precursor isotope intensities for each peptide (step 2 above).

---

#### Spectral Angle

**Default:** 0.7

**Range:** 0.0–1.0

**What it does:**
Minimum spectral angle between the observed and the expected isotope envelope. A retention time point below it cannot be the apex or part of the peak (steps 5 and 6 above).

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
- Leave on for standard peptide level quantification.
- Turn off if you plan to use MSstats. With it on, Sage writes a charge of -1 and `PrecursorCharge` has no meaning downstream (see the README, "Downstream tools").
- Turn off only when you need charge resolved intensities or a downstream tool expects a real charge in every LFQ row.

---

#### Peptide q-value

**Default:** 0.01

**Range:** 0.0 to 1.0

**What it does:**
Sage quantifies only target peptides with a peptide q-value at or below this limit. Peptides above the limit get no LFQ intensity.

**When to change it:**
A higher limit quantifies more peptides, but uses more time and memory. A limit below 0.01 leaves some identified peptides without an LFQ intensity.

---

#### Mobility tolerance

**Default:** 1.0 percent (Sage code default)

**What it does:**
The ion mobility tolerance for MS1 tracing. Sage uses it only for Bruker timsTOF data. SageGUI reads mzML only, so the Quant tab does not show this setting. If you load a Sage JSON that sets `mobility_pct_tolerance`, SageGUI keeps the value and sends it to Sage. The upstream documentation gives a default of 3.0. The vendored Sage code uses 1.0.

---

### TMT Settings

#### TMT Plex

**Default:** TMT 6 plex

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
Toggles a signal to noise based filter in TMT quantification.[file:238][file:250] The Quant tab does not show this setting. A value in a loaded Sage JSON is kept and sent to Sage.

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

**Verified (2026-09-22):** on a real 32,221-PSM run, `results.sage.pin` matched `results.sage.tsv` exactly on target/decoy counts and peptide sequences, and [mokapot](https://mokapot.readthedocs.io/) loaded it and completed a full rescoring run.

---

### Annotate Matches

**Default:** ☐ Unchecked

**What it does:**
Writes annotated fragment ion match detail alongside results when enabled.[file:250]

**Verified (2026-09-22):** on the same run, every row in `matched_fragments.sage.tsv` traced back to a real PSM, and `results.sage.tsv`'s match-quality columns (`matched_peaks`, `longest_b`, `longest_y`) reproduced correctly from the raw fragment data.

---

### Write HTML Report

**Default:** ☐ Unchecked

**What it does:**
Writes `results.sage.report.html`, a QC report from Sage. It shows a per-file table at 1% q-value, score and q-value histograms, retention time prediction plots and the search settings. Sage makes it only during a search. The page needs internet access to display, because it loads its plotting scripts from the web.

---

### Results Location

**Default:** the same folder as Output Location

**What it does:**
The folder that the Convert buttons read from and write to. It follows Output Location: after a search ends without error, it points to the folder that search used. Browse to pick the folder of an earlier run. Use Output Location makes it follow again. A status line under the box lists which of `results.sage.tsv` and `results.json` are in the folder.

---

### Convert to mzIdentML and Convert to pepXML

**What it does:**
Writes `results.sage.mzid` (mzIdentML 1.1.1) or `results.sage.pep.xml` (pepXML 1.23) from `results.sage.tsv` and `results.json` in the Results Location. It needs no new search. The q-value source (spectrum, peptide or protein), the q-value limit (default 0.01) and Include decoys set which rows are kept. Two checkboxes convert automatically after a search that ends without error. The pepXML file does not pass strict schema validation, because the schema does not list Sage as a search engine.

**Verified (2026-09-22):** on a real 9,392-entry conversion, [pyteomics](https://pyteomics.readthedocs.io/) (an independent Python reader) parsed every entry in both files with zero errors, correctly resolved all CV terms, and confirmed pepXML's modification masses. No specific target tool (Scaffold, Skyline, PeptideShaker, TPP) has opened either file yet.

---

### Info / Help

Shows SageGUI version, Sage engine version, author and maintainer, repository links and the recommended Sage citation.[file:250]

---

## Glossary

**Decoy:** Noise protein sequence generated by reversing target sequences. Used for FDR estimation.

**FDR (False Discovery Rate):** Proportion of false positives among all positive identifications. Often controlled at one percent PSM level and five percent protein level.

**Feature:** MS1 precursor ion described by mass, charge, retention time and isotopic envelope. LFQ traces features across runs.

**LFQ:** Label free quantification. Derives peptide intensity from the MS1 precursor signal: the peak area (Sum) or the apex intensity (Apex).

**m/z:** Mass to charge ratio.

**PPM:** Parts per million. Relative mass accuracy measure.

**PSM:** Peptide spectrum match.

**PTM:** Post translational modification.

**Spectral Angle:** Similarity measure between two intensity patterns, on a zero to one scale where one is identical. In LFQ, Sage compares the observed isotope envelope with the expected one.

**TMT:** Tandem Mass Tag.

**Trypsin:** Protease that cuts after K or R except when followed by P.

---

## Related Documentation

- **Sage official docs:** https://github.com/lazear/sage[web:59]
- **SageGUI README:** `../README.md`
- **_dev/NOTES.md:** locked design decisions and known limitations
- **_dev/PLAN.md:** roadmap for future features