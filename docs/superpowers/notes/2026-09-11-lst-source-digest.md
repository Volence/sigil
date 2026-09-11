# The `.lst` source digest (aeon LS-1a): spelling, read set, consumers, evidence

2026-09-11. Parcel `parcel/lst-source-digest`, base `158feb5e`. Implements the agreement
recorded in `2026-09-11-aeon-source-digest-ask.md` (and its amendment): a CONTENT
fingerprint of the build, written into the `.lst` as a new section, never into the deb2
trailer, so a stale artifact cannot read as fresh after a `touch`.

## 1. The section spelling (committed before implementation, for aeon's review)

### One real shape: `sigil build --aeon <tree> --native -o s4.bin --emit-lst s4.lst`

Rows below carry real values read from the reference tree at aeon `ec640bcf` after its
provisioning build. The file rows shown are three of the full set (a few hundred rows);
the aggregate line's value is computed over the full set and is filled in from the
landed build in section 5 of this note.

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
DIGEST-READ crc=a477aa73 size=2195 origin=external path=/<sigil checkout>/crates/sigil-harness/golden/offcanonical_sizes/s4.txt
DIGEST-READ crc=fe1503bc size=127 origin=generated path=engine/sound/generated/dac_sample_tab.bin
DIGEST-READ crc=946d49d7 size=21741 origin=source path=games/sonic4/map.toml
DIGEST-READ crc=f43b95b0 size=1909472 origin=tool path=tools/convsym
DIGEST-AGGREGATE crc=<crc32 over every DIGEST-READ line> reads=<N>
DIGEST-ROM crc=b09ccd65 size=820229
DIGEST-END
```

(The DEFINE rows shown are five of the shape's full define set, which is every row of
the merged define environment, sorted by name.)

### Grammar

The section is appended after the Phase Table, so it is always the last section of the
file. Every line after the rule and its blank line starts with `DIGEST-`, followed by one
keyword from `FORMAT ASSEMBLER SHAPE DEFINE SCAN READ AGGREGATE ROM END`. No keyword is a
prefix of another, so a consumer keying on `^DIGEST-<KEYWORD> ` (with the trailing space,
or end of line for `END`) can never match a second line kind. Fields are single-space
separated `key=value` tokens: a consumer finds a field by its key, never by position, so
adding a field later breaks no parser that follows that rule.

```text
section    = NL header NL rule NL NL format assembler shape define* scan read+ aggregate rom end
header     = "  Source Digest (the files this build read, and the ROM it wrote):"
rule       = "  " then one "-" per character of the header after its two-space indent (64)
format     = "DIGEST-FORMAT 1" NL
assembler  = "DIGEST-ASSEMBLER sigil version=" SEMVER " revision=" (HEX40 | "unknown")
             " tree=" WORD NL
shape      = "DIGEST-SHAPE target=" TARGET " game=" GAME " debug=" ("0" | "1")
             " extra-entries=" ("none" | ENTRY ("," ENTRY)*) NL
define     = "DIGEST-DEFINE " NAME "=" INT NL                  ; sorted by NAME bytes
scan       = "DIGEST-SCAN pattern=*.emp files=" DEC " crc=" HEX8 NL
read       = "DIGEST-READ crc=" HEX8 " size=" DEC " origin=" ORIGIN " path=" PATH NL
                                                                ; sorted by PATH bytes, each PATH once
aggregate  = "DIGEST-AGGREGATE crc=" HEX8 " reads=" DEC NL
rom        = "DIGEST-ROM crc=" HEX8 " size=" DEC NL
end        = "DIGEST-END" NL

TARGET     = "sonic4" | "demo" | "config-a" | "config-b" | "lean" | "stress-evict" | "stress-art"
GAME       = "sonic4" | "demo"
ORIGIN     = "source" | "generated" | "tool" | "external"
HEX8       = 8 lowercase hex digits (CRC-32, IEEE, the campaign provenance standard)
PATH       = the rest of the line; relative to the aeon root with "/" separators, except
             ORIGIN "external", which is absolute. Never contains CR or LF.
```

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
  rule to check it: recursive from the aeon root; directory symlinks are not followed;
  a subdirectory named `.worktrees`, or one containing a `.git` entry, is not entered (the
  root itself always is); every non-directory entry whose extension is exactly `emp`.
- `DIGEST-READ` rows are one per file the build read, with its CRC-32 and byte size AS
  READ (the bytes the build consumed, captured at the read, not re-read at the end).
  `origin=generated` marks a file this same build wrote before reading it (the sound
  artifacts under `engine/sound/generated/`); `origin=tool` marks an executable the build
  ran (`tools/convsym`); `origin=external` marks a file outside the aeon root, written
  with its absolute path; everything else is `origin=source`.
- `DIGEST-AGGREGATE crc` is CRC-32 over the bytes of every `DIGEST-READ` line, exactly as
  written, each including its terminating LF, concatenated in file order. `reads` is the
  number of `DIGEST-READ` lines.
- `DIGEST-ROM` is the identity of the full shipped ROM file exactly as written to `-o`,
  deb2 appendix included in the shapes that carry one. It is the same value as the
  `built: ... crc=<crc32> len=<bytes>` line.
- `DIGEST-END` closes the section. A listing whose digest lacks it was truncated.

### How a consumer checks freshness with it

An artifact pair (`.bin`, `.lst`) is fresh when all of these hold: the `.bin` file's
CRC-32 and size equal `DIGEST-ROM`; every `DIGEST-READ` row's file (under the aeon root,
or at the absolute path for `external`) exists with that CRC-32 and size; and the scan
rule above, re-run over the tree today, reproduces `DIGEST-SCAN`. A `touch` moves none of
these; a one-byte edit to any file the build read moves its row.

Sections 2 to 5 (the read-set derivation and its completeness argument, the consumer
enumeration, and the evidence) are filled in as the parcel lands.
