# The `.lst` source digest (aeon LS-1a): spelling, read set, consumers, evidence

2026-09-11. Parcel `parcel/lst-source-digest`, base `158feb5e`. Implements the agreement
recorded in `2026-09-11-aeon-source-digest-ask.md` (and its amendment): a CONTENT
fingerprint of the build, written into the `.lst` as a new section, never into the deb2
trailer, so a stale artifact cannot read as fresh after a `touch`.

## 1. The section spelling

### Revision 2 (current): aeon's four amendments, and the section moves to the top

Revision 1 (commit `33759437`) was reviewed by aeon and accepted with four amendments,
all ruled in by the controller: (a) `DIGEST-ROM` names the output file, so a mispaired
`.lst`/`.bin` fails by name before it fails by CRC; (b) on every row carrying `path=`, the
path is the last field and runs to end of line; (c) a define value never contains a
space, said in words; (d) a file outside the aeon root is written relative to a NAMED
root (`root=sigil path=crates/...`), so the aggregate is stable across sigil checkouts.

This revision also MOVES the section from the end of the listing to the start, which
revision 1 did not anticipate and aeon has not yet seen. The reason is a consumer that
revision 1 breaks: oracle's `SymbolTable::parse` (oracle `origin/main` `9c33ca05`,
`crates/oracle-core/src/symbols.rs`, the `parse` loop and its `Section::PhaseTable` arm)
enters the Phase Table state on that header and never leaves it; every later line that is
not a phase row adds to `skipped_lines`, which makes `is_intact()` false. For the shipped
shapes that is a false `NOT INTACT` caveat on every symbol load; for a ROM whose binding is
Indeterminate (no deb2 appendix, the `--lean` shape) oracle REFUSES the listing. Before the
Symbol Table header oracle is in its `Body` state, which by design treats a non-matching
line as ordinary AS source text and counts nothing. Section 3 of this note carries the full
consumer enumeration behind the placement.

### One real shape: `sigil build --aeon <tree> --native -o s4.bin --emit-lst s4.lst`

Rows carry real values read from the reference tree at aeon `ec640bcf` after its
provisioning build. Four of the few hundred READ rows are shown; the SCAN and AGGREGATE
values are computed over the full sets and are recorded from a real build in section 5.
The listing's first line is the header; after `DIGEST-END` and one blank line the listing
continues exactly as it would without the section.

```text
  Source Digest (the files this build read, and the ROM it wrote):
  ----------------------------------------------------------------

DIGEST-FORMAT 1
DIGEST-ASSEMBLER sigil version=0.1.0 revision=158feb5ec84876e5c7ea44e7019b1921ee72d593 tree=clean
DIGEST-SHAPE target=sonic4 game=sonic4 debug=0 extra-entries=none
DIGEST-DEFINE COLLECTED_WINDOW_SLOTS=9
DIGEST-DEFINE CRASH_REPORT=1
DIGEST-DEFINE DEBUG=0
DIGEST-DEFINE HAS_ACT_ART_POOL=1
DIGEST-DEFINE MAX_RING_BUFFER=128
DIGEST-SCAN pattern=*.emp files=<N> crc=<crc32 over the scanned paths>
DIGEST-READ crc=fe1503bc size=127 origin=generated path=engine/sound/generated/dac_sample_tab.bin
DIGEST-READ crc=946d49d7 size=21741 origin=source path=games/sonic4/map.toml
DIGEST-READ crc=f43b95b0 size=1909472 origin=tool path=tools/convsym
DIGEST-READ crc=a477aa73 size=2195 origin=external root=sigil path=crates/sigil-harness/golden/offcanonical_sizes/s4.txt
DIGEST-AGGREGATE crc=<crc32 over every DIGEST-READ line> reads=<N>
DIGEST-ROM crc=b09ccd65 size=820229 path=s4.bin
DIGEST-END

(0) 1/0 :        Vectors:
```

(The DEFINE rows shown are five of the shape's full define set, which is every row of the
merged define environment, sorted by name. The last line is the listing's first body row,
shown only to place the section.)

### Grammar

The section is the FIRST thing in the listing: its header is line 1, and `DIGEST-END` is
followed by exactly one blank line, after which the listing is byte-for-byte what it would
be without the section. Every line after the rule and its blank line starts with `DIGEST-`
followed by one keyword from `FORMAT ASSEMBLER SHAPE DEFINE SCAN READ AGGREGATE ROM END`. No
keyword is a prefix of another, so a consumer keying on `^DIGEST-<KEYWORD> ` (with the
trailing space, or end of line for `END`) never matches a second kind of line. Fields are
single-space separated `key=value` tokens found by key, never by position, so adding a
field later breaks no parser that follows that rule.

**`path=` is always the LAST field of any line that carries it, and its value runs to the
end of the line.** A path containing spaces therefore needs no quoting: take everything
after the first `path=` that follows the fixed fields.

```text
section    = header NL rule NL NL format assembler shape define* scan read+ aggregate rom end NL
header     = "  Source Digest (the files this build read, and the ROM it wrote):"
rule       = "  " then one "-" per character of the header after its two-space indent (64)
format     = "DIGEST-FORMAT 1" NL
assembler  = "DIGEST-ASSEMBLER sigil version=" SEMVER " revision=" (HEX40 | "unknown")
             " tree=" WORD NL
shape      = "DIGEST-SHAPE target=" TARGET " game=" GAME " debug=" ("0" | "1")
             " extra-entries=" ("none" | ENTRY ("," ENTRY)*) NL
define     = "DIGEST-DEFINE " NAME "=" INT NL                    ; sorted by NAME bytes
scan       = "DIGEST-SCAN pattern=*.emp files=" DEC " crc=" HEX8 NL
read       = "DIGEST-READ crc=" HEX8 " size=" DEC " origin=" ORIGIN [" root=" ROOT] " path=" PATH NL
                                     ; root= present exactly when origin=external
                                     ; sorted by (ROOT, PATH) bytes, the aeon root first; each once
aggregate  = "DIGEST-AGGREGATE crc=" HEX8 " reads=" DEC NL
rom        = "DIGEST-ROM crc=" HEX8 " size=" DEC ( " output=none" | [" root=" ROOT] " path=" PATH ) NL
end        = "DIGEST-END" NL

TARGET     = "sonic4" | "demo" | "config-a" | "config-b" | "lean" | "stress-evict" | "stress-art"
GAME       = "sonic4" | "demo"
ORIGIN     = "source" | "generated" | "tool" | "external"
ROOT       = "sigil" | "filesystem"
NAME       = an identifier; INT = a decimal integer with an optional leading "-"
ENTRY      = an --extra-entry argument as given; never contains whitespace or ","
HEX8       = 8 lowercase hex digits (CRC-32, IEEE, the campaign provenance standard)
PATH       = the rest of the line: a relative path, "/" separated, with no "." or ".."
             components and no CR or LF
```

**A `DIGEST-DEFINE` value never contains a space**: NAME is an identifier and INT is a
decimal integer, so the whole row is `DIGEST-DEFINE` and one `NAME=INT` token.

**Where a path lives.** A path is always relative. With no `root=` field it is relative to
the aeon root (the `--aeon` tree, symlinks resolved). `root=sigil` means the root of the
sigil checkout the assembler binary was compiled from (the directory `sigil --version`
names on its `source:` line). `root=filesystem` means `/`. Sigil resolves every path by
trying the aeon root, then the sigil root, then `/`, and writes the first that contains the
file.

Field meanings:

- `DIGEST-FORMAT` is this section's format version. It moves when the grammar changes.
- `DIGEST-ASSEMBLER` carries the same two facts `sigil --version` reports on its
  `revision:` and `tree:` lines, from the same build-time capture.
- `DIGEST-SHAPE` is the build configuration: the target flag, the game, the debug axis,
  and any `--extra-entry` arguments.
- `DIGEST-DEFINE` rows are the define environment the `.emp` build lowered with (the
  shape's built-in rows merged with the game's `map.toml [defines]`).
- `DIGEST-SCAN` states the module scan's membership: `files` is how many `.emp` files the
  build's directory walk found, `crc` is CRC-32 over their PATHs (rendered as in the READ
  rows), sorted by bytes, each followed by one LF. It exists because a file that APPEARS
  after the build has no READ row, yet the next build would read it (every scanned module
  is parsed, and the contract gate runs over all of them). Re-enumerate with the walk's
  rule to check it: recursive from the aeon root; directory symlinks are not followed; a
  subdirectory named `.worktrees`, or one containing a `.git` entry, is not entered (the
  root itself always is); every non-directory entry whose extension is exactly `emp`.
- `DIGEST-READ` rows are one per file the build read, with its CRC-32 and byte size AS
  READ (the bytes the build consumed, captured at the read, not re-read at the end).
  `origin=generated` marks a file this same build wrote before reading it (the sound
  artifacts under `engine/sound/generated/`); `origin=tool` marks an executable the build
  ran (`tools/convsym`); `origin=external` marks a file outside the aeon root, and only
  those rows carry `root=`; everything else is `origin=source`.
- `DIGEST-AGGREGATE crc` is CRC-32 over the bytes of every `DIGEST-READ` line, exactly as
  written, each including its terminating LF, concatenated in file order. `reads` is the
  number of `DIGEST-READ` lines.
- `DIGEST-ROM` is the identity of the full shipped ROM file exactly as written to `-o`,
  deb2 appendix included in the shapes that carry one: the same value as the
  `built: ... crc=<crc32> len=<bytes>` line. `path=` is the `-o` destination, written by
  the same root rule as a READ row (relative to the aeon root when it lies inside it).
  With no `-o` the build wrote no ROM, and the line ends `output=none` in place of a path.
- `DIGEST-END` closes the section. A digest without it was truncated.

### How a consumer checks freshness with it

An artifact pair (`.bin`, `.lst`) is fresh when all of these hold: `DIGEST-ROM`'s path
names that `.bin`, and its CRC-32 and size equal the file's; every `DIGEST-READ` row's file
exists with that CRC-32 and size; and the scan rule above, re-run over the tree today,
reproduces `DIGEST-SCAN`. A `touch` moves none of these; a one-byte edit to any file the
build read moves its row and the aggregate.

Sections 2 to 5 (the read-set derivation and its completeness argument, the consumer
enumeration, and the evidence) are filled in as the parcel lands.
