# Aeon's ask: a digest of the linked source set in the listing (aeon LS-1a, lens CTRL-6)

2026-09-11. A cross-lane COMMITMENT made to the aeon lane, banked here with the send
(protocol bar 20, sending side) so it survives a rotation of this lane.

## The ask, read at their revision rather than taken from the message

aeon `cd075f2d` (verified an ancestor of their `origin/master`), `docs/DEFERRED_WORK.md`
row LS-1a: fourteen build-time consumers decide an artifact is fresh by `mtime >= ${SIGIL_T0}`
(eleven `--built-after` gate invocations in `build.sh`, the `--artifacts-built-after` lane,
`tools/landing_build.sh`, `tools/nightly_effects_gates.sh`), and they disagree on the verdict for
a stale artifact (exit 2, exit 1, or exit 0 without `--gate`). Their fix is one shared provenance
primitive; they asked sigil for a content digest emitted BY THE BUILD, since a sidecar aeon writes
is forgeable by the same `touch`.

## The answer: yes, with a counter-shape

**Where: the `.lst`, never the deb2 trailer.** The deb2 appendix is ROM bytes in every shape
that carries the fault-handler island, debug and release both, so a digest there would change the
ROM on every source edit and move the four-shape pins for no gain. The `.lst` is ROM-neutral:
`append_deb2_appendix` is computed from the in-memory `listing`, not from the `.lst` text
(`crates/sigil-cli/src/main.rs`, the native-build tail where `emit_listing` output is written
and the appendix is built from the same `(rom, listing)` pair). This is to be PROVEN at landing
by the four-shape byte gates, not assumed from that reading.

**Shape: a SECTION naming the set, not one aggregate line.** A lone digest cannot be checked:
a consumer that wants to re-verify freshness must recompute over the same file set, so the set
has to be written down. Proposed content, spelling to be sent to aeon for review before it lands:

- a format version for the section;
- the assembler's own revision (the `--version` banner's revision);
- the build configuration: game, debug, target shape, and the define environment;
- one row per file the build READ (every `.emp` module, every include, every embedded data file,
  `map.toml`, and the generated inputs under `engine/sound/generated/`, marked as generated since
  sigil writes them itself), as `crc32 size path`, path relative to the aeon root, sorted by path
  bytes;
- an aggregate: CRC32 over the rows' canonical text.

The section must match none of the listing's existing consumer grammars (Oracle's line-header
parse, `s4budget`'s symbol table and trailer regexes, the equate rows), by the same rule
`crates/sigil-link/src/listing.rs` states for the Phase Table.

**Hash: CRC32 plus size per file**, the campaign's provenance standard. The threat is accidental
staleness (a `touch`, an unrelated rewrite, a stale artifact), not an adversary.

**The enumeration of the read set is the hard part and the risk.** A file the build reads that
the section omits makes a stale artifact read as fresh, which is the exact failure the ask exists
to close. So the parcel must derive the set from what the build actually opens, with a control that
plants an unlisted read and shows the gate catching it, never from a list of known file kinds.

## When

Queued as `AEON-LS1A-SOURCE-DIGEST`, size M, after the three parcels in flight on 2026-09-11 land.
Not urgent for aeon by their own word: they proceed on the shared primitive and plug the digest in
when it exists. On landing, this lane sends aeon the landed SHA and the exact section spelling.
