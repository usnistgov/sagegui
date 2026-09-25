# Development record

This directory is the development history of SageGUI. It is not the product, and none
of it is needed to build or run the program. The program is `src/` (with the vendored
Sage in `vendor/sage/`), and `README.md` at the repository root is the document for
users.

We publish this record because the reasoning behind a scientific tool is part of the
tool. A setting in the interface, a default, or a converter's output is only as good as
the decision that produced it, and those decisions are written down here rather than
lost.

## What is in here

| Path | What it is |
|---|---|
| `dev_AGENTS.md` | The full working protocol for development sessions. The root `AGENTS.md` is its short form |
| `PLAN.md` | The roadmap and its status block |
| `NOTES.md` | Settled decisions (marked `(locked)`), discovered facts, dead ends, entries marked "intentional, not a bug", and reference material. The main record |
| `JOURNAL.md` | Dated session debriefs, newest first: what was done, what was least certain, what to do differently |
| `feedback-2026-07-24.md` | Feature requests from a collaborator, kept verbatim |
| `old-to-new.txt` | Map from each commit SHA before the 2026-09-23 publication to its published SHA |

## Reading it honestly

These are working documents. They record mistakes as well as results, including entries
that correct an earlier conclusion. That is deliberate: a record that holds only the
conclusions we kept would hide how much those conclusions were tested.

Entries marked `(locked)` are settled and should not be reopened without new evidence.

Some entries name absolute paths on the maintainer's computers (for example the test
data under `~/Documents/proteomicsTesting/`, or a Windows path under `C:\Users\`). The
data they name is not in this repository. Those references are not broken links to fix;
they record where a check was run. Entries written before September 2026 also refer to
`github.com/neely/sagegui` and `github.com/neely/sage`, the repositories this project
lived in before it moved to `usnistgov` and before Sage was vendored.

Commit SHAs cited in these files and in code comments, when written before the
publication on 2026-09-23, refer to the history before it was prepared for
publication (author address set to NIST, AI co-author trailers removed). The contents
of every commit are unchanged. `old-to-new.txt` in this directory maps each old SHA to
its published one.
