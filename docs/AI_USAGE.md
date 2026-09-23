# AI usage

This project was developed with AI coding agents (Claude Code, running Anthropic Claude
models) working alongside a human author, under continuous human review.

- [`AGENTS.md`](../AGENTS.md) and [`_dev/dev_AGENTS.md`](../_dev/dev_AGENTS.md) are the
  operating protocols those agents follow: how to verify a claim before stating it, when
  to stop and flag a contradiction, and which parts of the project are settled and should
  not be reopened.
- Every commit is authored, reviewed and pushed by Benjamin A. Neely, who is accountable
  for the code, the searches it configures, and the claims made about it. No commit
  carries an AI co-author trailer. Some commits made before September 2026 carried one;
  it was removed from those commit messages when the history was prepared for
  publication under `usnistgov`, and this file records that instead.
- AI assistance covered design discussion, code, tests and documentation (including this
  file). It did not replace checking against real data: search behaviour is checked by
  running real searches and comparing their output (see the test baseline and the
  regression checks in `_dev/NOTES.md`).

This is a plain factual record, not a policy statement. NIST and GitHub do not yet have
a standard disclosure format for AI-assisted development, so this file states what was
actually done rather than following a template.

The code in `src/` that derives from J. Sebastian Paez's `jspaezp/sagegui`, and the
vendored Sage source by Michael Lazear, were written by those authors. See
[`THIRD_PARTY_LICENSES.md`](../THIRD_PARTY_LICENSES.md).
