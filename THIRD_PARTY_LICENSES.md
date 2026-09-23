# Third-Party Licenses

This project incorporates and derives from the following third-party
open-source software. Each is used in compliance with its original
license, summarized or reproduced below.

---

## Sage

Source: https://github.com/lazear/sage  
License: MIT  
Vendored: `vendor/sage/`, base commit `d74024df774054fa411a9d5cca6013ce91d26208`
(`master`, 10 commits after the `v0.15.0-beta.2` tag), vendored 2026-09-23.

**Redistribution notice:** this repository contains the source of three Sage crates
(`crates/sage`, published as `sage-core`; `crates/sage-cli`; `crates/sage-cloudpath`),
and SageGUI compiles them into its binary. The copy is **MODIFIED**. One file differs
from upstream:

| File | State |
|---|---|
| `crates/sage/**` | unmodified |
| `crates/sage-cloudpath/**` | unmodified |
| `crates/sage-cli/src/runner.rs` | **MODIFIED, see below** |
| every other file in `crates/sage-cli/` | unmodified |

**Modifications to `runner.rs`,** made by Benjamin A. Neely (NIST) and marked in a
header at the top of the file:

- 2026-08-21: a public `progress` counter on `Runner`, so SageGUI can show search
  progress.
- 2026-08-24: a public `cancel` flag, a `with_cancel` method, and three cancellation
  checks in `Runner::run`, so SageGUI's Stop button can interrupt a search.
- 2026-09-23: a `Runner::from_parts` constructor, so SageGUI can run a search on a
  peptide database loaded from its on-disk cache.

All three changes are additive. The stock Sage command-line tool never uses them, so its
behaviour is unchanged. `vendor/sage/PATCHES.md` gives the full list and the reasons,
and `git log -p -- vendor/sage` shows every changed line. The upstream MIT license text
is kept at `vendor/sage/LICENSE` and is reproduced below; it applies to the original
files.

MIT License

Copyright (c) 2022 Michael Lazear

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## sagegui (J. Sebastian Paez)

Source: https://github.com/jspaezp/sagegui  
License: Apache License, Version 2.0

License confirmed directly with the author (J. Sebastian Paez, August 2026)
and referenced inside the GUI; no LICENSE file is present in the upstream
repository at time of forking.

Copyright 2023 J. Sebastian Paez

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

Note: this project is a heavily modified fork of sagegui. Substantial
portions have been rewritten, including GUI redesign, executable packaging,
and updated Sage version integration. Modified files carry a notice indicating
they have been changed from the original, per Apache License 2.0 Section 4(b).

---

## Test schemas (not shipped in the app)

Two XML Schema files in `tests/schemas/` are used only by the tests, to check
the output of the mzIdentML and pepXML converters. They are not built into the
program. Both were fetched on 2026-09-21 and are not modified.

**mzIdentML 1.1.1** (`tests/schemas/mzIdentML1.1.1.xsd`)  
Source: https://github.com/HUPO-PSI/mzIdentML  
License: Creative Commons Attribution 2.0 (CC BY 2.0). The file header says:
"Distributed under the Creative Commons license
http://creativecommons.org/licenses/by/2.0/". Attribution: HUPO Proteomics
Standards Initiative (PSI).

**pepXML 1.23** (`tests/schemas/pepXML_v123.xsd`)  
Source: Trans-Proteomic Pipeline,
https://svn.code.sf.net/p/sashimi/code/trunk/trans_proteomic_pipeline/schema/pepXML_v123.xsd  
License: the file has no licence text. Its header says "Developed by ISB
proteome center". The licence of the source tree was not checked. The file is
kept for validation in the tests only.

