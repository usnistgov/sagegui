# Changes to vendored Sage

This file lists every change we (NIST) made to the vendored Sage source, relative to the
upstream base commit named in [VENDORED.md](VENDORED.md). Each entry is one commit in this
repository, so `git log -p -- vendor/sage` shows the exact lines. All changes so far are
additive: they add fields and methods that the stock Sage command-line tool never calls,
and they change no existing behaviour.

We keep patches small and confined to `crates/sage-cli/src/runner.rs` where possible, and
we keep feature logic in SageGUI itself, because every edited line in Sage has to be
re-applied by hand at the next update (see [MAINTENANCE.md](../../MAINTENANCE.md)).

None yet. The vendored source is identical to upstream.
