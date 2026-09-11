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

## 2. The read set: how it is derived, and why it is complete

### One recorder, and every read goes through it

`crates/sigil-span/src/read_set.rs` is the recorder. `read` and `read_to_string` return
the bytes AND record the file's canonical path with the CRC-32 and size of exactly those
bytes, at the read. `write_generated` writes a file and records that this process wrote
it, so a later read of it is `origin=generated` by what happened, not by where it lives.
`tool_command` records an executable's bytes before returning the `Command` that runs
it. `record_scan` records a directory walk's membership. The log is process-wide, not
thread-local, because the comptime evaluator (which performs every `embed`) runs on a
thread of its own; `sigil build` is one build per process, so the log is that build.

Every read site the native build reaches, found by searching every file-reading and
spawning spelling in `crates/*/src` and then confirmed by the kernel witness below:

| crate / file | site | what it reads |
|---|---|---|
| sigil-frontend-emp `resolve/manifest.rs` | `Manifest::scan` | every `.emp` under the root; the walk is `record_scan` |
| sigil-frontend-emp `resolve/manifest.rs` | `SourceIndex::new` | the same files again, for diagnostic locations |
| sigil-frontend-emp `eval/sandbox.rs` | `eval_embed`, `eval_import` | `embed(...)` blobs, `import(...)` JSON/TOML |
| sigil-frontend-as `lib.rs` | `assemble_root_impl` | the AS residual root (`game_root.asm`) |
| sigil-frontend-as `eval.rs` | `directive_include`, `directive_binclude` | `include` and `BINCLUDE` targets |
| sigil-harness `native.rs` | `load_frozen_table` | `golden/offcanonical_sizes/<shape>.txt` in the sigil checkout |
| sigil-harness `native.rs` | `shape_defines`, `resolve_chained`, `placement_map`, `project_memory_map` | `games/<g>/map.toml` |
| sigil-harness `native.rs` | `harvest_engine_constants`, `harvest_game_constants`, `harvest_engine_struct_offsets` | the harvested `.emp` modules |
| sigil-harness `native.rs` | `append_deb2_appendix`, `convsym_resolve` | `tools/convsym`, spawned through `tool_command` |
| sigil-harness `seam1.rs` | `parse_one`, `eval_pub_consts`; writes via `write_generated` | the resident sound sources; `engine/sound/generated/*` |
| sigil-harness `seam2.rs` | `bank_anchors`, `emit_dac_banks_at`, the seam lowering, `emit_pitchtable_doctored`, the MT bank; six writes via `write_generated` | the sound sources, the map; `engine/sound/generated/*` |
| sigil-harness `test_support.rs` | eight reads, routed | on the build path: `seam1` and `seam2` call into this module |

Sites that stay raw, each saying why on the line (`read-set: not a build input`):
`native.rs` `append_deb2_appendix` reading back convsym's output (the build's own
product; the ROM line covers it), and `test_support.rs` `derive_suite_root_from`'s git
spawn (locates the suite root for tooling; `sigil build` names its tree with `--aeon`).
Whole modules no `sigil build` path reaches say so in their module doc:
`provenance.rs`, `freeze_journal.rs`, `reference_dependence.rs`, `strict_census.rs`,
`rev_reachability.rs`, `harness_root.rs` (sigil-harness) and `asl_provenance.rs`
(sigil-isa, which has no workspace dependencies and so cannot call the recorder). Each
claim was checked by searching for callers from the build-path modules; `test_support`
was the one module that looked exemptable and was not, because `seam1` and `seam2` call
`assemble_equ_pairs`, `checkout_var_is_set` and `unnamed_default_tree`.

In `crates/sigil-cli/src/main.rs` no function reachable from `run_build` reads a file
itself; every read is inside the libraries above.

### Why it is complete: one construction, two independent nets

1. **The source gate** (`crates/sigil-span/tests/read_set_gate.rs`) refuses a raw
   `std::fs::read`, `std::fs::read_to_string`, `File::open`, an `OpenOptions` read, a
   `std::fs` read import, or a `Command::new` anywhere in `crates/*/src` (cut at the
   first inline test module, skipping out-of-line test modules and `src/bin/`), unless
   the site carries the marker. In `main.rs`, which hosts every subcommand, it scans the
   functions reachable from `run_build` through the file's own call graph. It lives
   under `tests/`, outside its own scope, so its needle strings cannot satisfy or trip
   it. Its instrument is checked against the source: every file that calls the recorder
   must have been scanned rather than exempted, `run_build_native`, `run_contract_gate`
   and `lst_source_digest` must be in the closure, and the recorder file must itself hold
   a raw read (so its exclusion is load-bearing, not the thing keeping the gate green).
2. **The kernel witness** (`every_file_a_build_opens_is_a_digest_row_and_every_row_was_opened`
   in `crates/sigil-cli/tests/lst_source_digest.rs`) builds all four shipped shapes
   under an `LD_PRELOAD` interposer of `open`, `open64`, `openat`, `openat64`,
   `posix_spawn`, `posix_spawnp`, `execve` and `execvp`, and requires the set of
   regular files opened read-only plus the executables spawned (outside `/proc`,
   `/sys`, `/dev`, `/etc`, `/usr`, `/lib*`, `/run`, and excluding files deleted by the
   end of the run, which are the appendix pass's temporaries) to EQUAL the digest's rows.
   The direction rows-to-opens is the control that the interposer is not blind; the
   direction opens-to-rows is the omission check. It needs no knowledge of file kinds.

Two checks inside the digest builder (`crates/sigil-harness/src/source_digest.rs`)
refuse a snapshot that cannot be a digest: a file read with two different contents
(`Conflict`, the build consumed two versions), and a module the scan found with no
read of it. With no module scan recorded at all, the build fails.

What the nets do not cover, stated so no one reads them as total: a read by raw
syscall or `io_uring`, or by C code in a `-sys` crate calling `fopen` (the compressor
crates take their input from memory, not files); a build shape other than the four
the witness runs; and a file whose ABSENCE matters. The only absence-sensitive input is
the module walk, which is why `DIGEST-SCAN` exists; every other read of a missing file
is a build error, so there is no artifact to go stale.

### The half-fix question, answered by tests

A section that lists the `.emp` modules and misses another kind looks complete and is
wrong. Each kind has a test that goes red on its omission (red-first runs in section 5):

| omitted | what goes red |
|---|---|
| `embed` reads | the gate; the witness (the blobs are opened and are not rows); the content test (no `embed()` target of a read `.emp` is a row) |
| AS `include` reads | the gate; `read_set_as_reads`; the witness; the content test (no included `.asm` row) |
| AS `BINCLUDE` reads | the gate; `read_set_as_reads` (the corpus at aeon `ec640bcf` BINCLUDEs nothing, so the end-to-end test prints `0 target(s)` and the synthetic test carries this kind) |
| the generated marking | the content test (the generated pitch table is not `origin=generated`) |
| the convsym spawn | the gate; the witness; the content test (no tool row) |
| the module walk | the build itself (no module scan recorded), so the witness and the content test |
| the manifest's module read | the gate only: `SourceIndex::new` re-reads every scanned module through the recorder, so the rows survive and the end-to-end tests stay green (measured, M7 below). The digest is still complete; what is lost is the per-read record of the first read |
| `map.toml` | the gate per site; the witness and the content test if every site is missed |

The content test (`the_digest_moves_with_content_and_never_with_time`) runs on a private
copy of the reference tree: two builds give identical sections; touching every file the
build read leaves the section identical; one edit to each kind (an `.emp` module, the AS
root, an included `.asm`, `map.toml`, the pitch-table source whose generated `.bin` the
build reads back, `tools/convsym`, an `embed`ed blob) moves exactly that file's row (and
the generated file's, for the sound source) and the aggregate; a new module file moves
`DIGEST-SCAN` and adds exactly its own row; undoing everything restores the section byte
for byte.

### The external row: the controller's three questions, from the recorder

**(1) Which call opens it.** A backtrace hook in the recorder (a temporary patch in a
COPY of the checkout, built with debuginfo; not committed) printed six reads of
`crates/sigil-harness/golden/offcanonical_sizes/s4.txt` in one `sigil build --native`,
every one at `sigil_harness::native::load_frozen_table` (`native.rs:236`), called from
`sonic4_profile` (`native.rs:701`), in turn from:

- `BuildTarget::label_and_profile` (`main.rs:2179`) via `corpus_closure_or_exit`
  (`main.rs:1644`) via `run_contract_gate` (`main.rs:1706`) via `run_build_native`;
- the same via `run_contract_gate` (`main.rs:1844`);
- `label_and_profile` via `run_build_native` (`main.rs:2520`, the label);
- `build_native_rom_with_listing` (`native.rs:4071`) via `run_build_native` (the build);
- `label_and_profile` via `run_build_native` (`main.rs:2607`, the island question);
- `label_and_profile` via `lst_source_digest` (`main.rs:2667`, the digest's own profile).

The path comes from `env!("CARGO_MANIFEST_DIR")`, fixed at COMPILE time, so it is the
checkout the binary was built from. The row is a real read, not a recorder artifact:
the kernel witness sees the same file opened.

**(2) What its contents can change.** Probed with a binary built from a copy of the
checkout, editing the copy's table (`scratch probe-frozen.sh`, `probe-frozen2.sh`), shape
sonic4 plain unless named:

| edit | build | ROM |
|---|---|---|
| none | ok | `b09ccd65/820229` |
| a comment byte | ok | `b09ccd65/820229` |
| first row `+2` | ok | `b09ccd65/820229` |
| `EndOfRom` `+2` | ok | `b09ccd65/820229` |
| a middle row deleted | ok | `b09ccd65/820229` |
| a middle row `+0x10000` | ok | `b09ccd65/820229` |
| two rows' addresses swapped | ok | `b09ccd65/820229` |
| every row zeroed | FAILS: `packed layout overlaps at its real bases, a run grew into a declared anchor` | none |
| demo, none / first row `+2` | ok | `0ad17404/96863` both |

No edit that still built moved a ROM byte or the warning tally. The table can change
the build's OUTCOME (it seeds the provisional bases that classify islands, per
`GameProfile::frozen_sizes`'s own doc), so it is an input and its row stays. The digest
row moved with every edit (CRC `a477aa73` to `22c61cfd`, `e27d202f`, `452e69de`), which
is the property the digest needs.

**(3) Which shapes read it.** Each shape reads exactly its own table, one row per
shape: s4 `s4.txt`, s4 debug `s4_debug.txt`, demo `demo.txt`, demo debug
`demo_debug.txt` (and by the same code config-a, config-b and lean read theirs).

## 3. The `.lst` consumers, and why each ignores the section

### Sigil (this tree)

- `sigil_harness::test_support::listing_symbol_addr` (reads `s4.lst`/`s4.debug.lst`
  for the port gates: `parallax_port`, `rings_port`, `test_p1_player_port` and others):
  a line must start with ` NAME : `. No digest line starts with a space followed by a
  name and ` : ` (the header starts `  Source`, the rule `  ---`, every other line
  `DIGEST-`).
- `test_support::listing_symbols_with_prefix`: a line starting with a space and holding
  ` : `. The header and rule hold no ` : `; the rest start with `D`.
- `repin` names `s4.lst` only as a provenance label; its pins come from the in-memory
  listing (`sigil_native_symbol_listing`).
- `append_deb2_appendix` and `convsym_resolve` write their own temporary listing with
  `emit_listing`, which carries no digest; convsym never sees one. `m1b_gate` and
  `listing_phase_marker` test `emit_listing` text, likewise digest-free.
- `game_defines.rs` does not read a listing: it WRITES the define rows into one
  (`define_listing_rows`); its only text parse is `map.toml`'s `[defines]`.

The collision control `source_digest_lines_never_parse_as_a_consumer_row`
(`crates/sigil-link/src/listing.rs`) transcribes each of these grammars plus the aeon
and oracle ones below, runs every digest line through them (with a hostile path shaped
like a symbol row), and has a positive control per transcription on a real row.

### Aeon, at `origin/master` `826159e7`

The enumeration (every `*.py`/`*.sh` naming `.lst`, 131 files) maps each consumer to
one of these grammars; none matches a digest line:

- G1 `raster_cost_probe.parse_lst` and its ~40 importers, plus local copies: `startswith("(0) ")`.
- G2 the body-row regexes `^\(0\)\s+\d+/HEX\s+:\s+NAME:` and variants (`scene_spans`
  `LST_HEAD_RE`, `bganim_room`, `plane_base_swap_gate`, `dplc_straddle`, `row_remap_gate`,
  `s4budget` above its header, and others).
- G3 the symbol-table row `^ NAME : HEX [A-Z] \|` and looser variants
  (`^\s*(\S+)\s*:\s*HEX\s+[A-Z]\s*\|`, `^\s*(\w+)\s*:\s*HEX`).
- G4 `^EQU ...` equate readers.
- G5 `s4budget.parse_listing`: from the `Symbol Table` header to end of file, and it
  fails loudly on a count mismatch; the digest precedes the header, so it is not in range.
- G6 `deb2_probe.read_symtab`, `test_deb2_appendix`: header to the `symbols` trailer.
- G7 `fixtures/make_listing_excerpt.py`: header to end of file, same shape as G5.
- G8 `state_ram.load_symbols`: `split(":")` with a `/` before it and a `:`-terminated
  name.
- G9, G10 unanchored searches for `$cap_*_begin|end` and `Dbg_PageIn_Preempts`.
- G11 everything handed to oracle (below).

No aeon tool takes the first or last line of a listing, counts lines, slices by line
number, or hashes a live listing; the checks for freshness use mtime only today.

**Measured, not only read.** Aeon's own parsers, extracted from `826159e7` with
`git archive`, were run over the four listings built without the digest (base binary)
and with it, each pair read from one path so an error message naming its input could
not differ for that reason: 91 function-and-shape calls returned identical results,
0 differed, 0 raised on one side only; 61 raised identically on both (a plain shape
missing a debug-only symbol, or a function taking a parsed object), and 21 modules need
an environment this probe does not provide. The four most-used readers were also called
correctly by hand, identical on all four shapes: `s4budget.rom_labels` and `ram_labels`
over `parse_listing` (2177 / 310 entries on s4), `scene_budget_report.read_equates`
(780), `state_ram.load_symbols` (2487).

### Oracle, at `origin/main` `9c33ca05`

`crates/oracle-core/src/symbols.rs` `SymbolTable::parse` (shared by oracle-aether, the
frontend, the player and oracle-replay, and so by aurora and every aeon tool that hands
oracle a listing) is the one consumer whose state machine reacts to position: after an
`Equate Table` or `Phase Table` header, every unrecognised line is counted in
`skipped_lines`, which makes `is_intact()` false, adds a NOT INTACT caveat, and makes
the load policy refuse the listing for an Indeterminate binding. Placed FIRST, every
digest line is met in the `Body` state, where `parse_body_line` (four tokens, `(..)`,
`N/HEX`, `:`, `Name:`) matches none of them and a non-match counts nothing by design.
The collision control walks oracle's header transitions over section plus listing and
asserts every digest line is met before the first header.

Oracle's `aeon_pin` and `aeon_dimensions` tests hash its frozen `fixtures/aeon/*.lst`,
so a re-pin of those fixtures from a digest-bearing build changes their recorded hashes
and sizes: an expected re-pin, not a parse change.

### Aurora, seraph, empyrean

No parser of a digest-bearing listing: aurora loads listings through oracle, and its two
scratchpad scripts use the G3 row shape; seraph and empyrean mention `.lst` in documents
only.

## 4. The hunks in `main.rs` and `sigil-link`

- `crates/sigil-cli/src/main.rs`, `run_build_native`: the early `.lst` write (before the
  appendix) is removed; after `full` is final, `lst_source_digest` renders the section
  (a failure exits 1 before either artifact is written), the ROM is written to `-o`,
  then the `.lst` is written as the section followed by `emit_listing`'s text.
- `crates/sigil-cli/src/main.rs`, new `lst_source_digest` (the define environment, the
  recorder snapshot, `source_digest::source_digest`, `emit_source_digest`) and new
  `digest_target` (an exhaustive `BuildTarget` to `target=`/`game=` map), both placed
  directly after `run_build_native`.
- `crates/sigil-link/src/lib.rs`: re-exports the digest types and functions.
- `crates/sigil-link/src/listing.rs`: new, after `emit_listing`: the digest types, the
  renderer `emit_source_digest`, the strict parser `parse_source_digest`,
  `digest_scan_identity`; and seven tests at the end of its test module. `emit_listing`
  itself is unchanged.

## 5. Evidence

All runs against the private reference tree `/home/volence/sonic_hacks/.aeon-sigil-ls1a`
(aeon `ec640bcf`, the provenance tip's `aeon_rev`), provisioned by
`scripts/provision-aeon-ref.sh` and witnessed by `repin --check` printing
`pins.rs unchanged`. "Before" is the base binary (`sigil 0.1.0 (158feb5e)`, clean);
"after" is the implementation commit `96fab5e6` rebuilt from a clean `crates/`
(`sigil 0.1.0 (96fab5e6)`).

### The real section, sonic4 plain (`-o` into a scratch directory)

```text
  Source Digest (the files this build read, and the ROM it wrote):
  ----------------------------------------------------------------

DIGEST-FORMAT 1
DIGEST-ASSEMBLER sigil version=0.1.0 revision=96fab5e630e4e55361919f57c0ea4cd02c64e923 tree=clean-sources
DIGEST-SHAPE target=sonic4 game=sonic4 debug=0 extra-entries=none
DIGEST-DEFINE COLLECTED_WINDOW_SLOTS=9
  (ten DEFINE rows in all, through)
DIGEST-DEFINE VRAM_RING_PLACEHOLDER=1000
DIGEST-SCAN pattern=*.emp files=202 crc=98cbecb8
DIGEST-READ crc=fe1503bc size=127 origin=generated path=engine/sound/generated/dac_sample_tab.bin
DIGEST-READ crc=946d49d7 size=21741 origin=source path=games/sonic4/map.toml
DIGEST-READ crc=f43b95b0 size=1909472 origin=tool path=tools/convsym
DIGEST-READ crc=a477aa73 size=2195 origin=external root=sigil path=crates/sigil-harness/golden/offcanonical_sizes/s4.txt
  (326 READ rows in all)
DIGEST-AGGREGATE crc=fd3535ae reads=326
DIGEST-ROM crc=b09ccd65 size=820229 root=filesystem path=home/volence/sonic_hacks/.scratch/lst-source-digest/final1/s4.bin
DIGEST-END
```

(Under aeon's `build.sh`, `-o s4.bin` inside the tree writes `path=s4.bin` with no
`root=`. The four READ rows shown are the ones section 1 predicted, with the same values.)

### ROM bytes do not move

| shape | provenance tip | before | after (twice) |
|---|---|---|---|
| s4 | `b09ccd65/820229` | `b09ccd65/820229` | `b09ccd65/820229` |
| s4 debug | `1b7fe316/846529` | `1b7fe316/846529` | `1b7fe316/846529` |
| demo | `0ad17404/96863` | `0ad17404/96863` | `0ad17404/96863` |
| demo debug | `2565ece2/103185` | `2565ece2/103185` | `2565ece2/103185` |

The deb2 appendix is in all four (it follows the fault-handler island, debug and
release alike), so moving the `.lst` write after the appendix is proven byte-neutral in
every shape that carries one.

### The `.lst` changes only by the section

For each shape, the after listing with its section removed (everything through
`DIGEST-END` and its blank line) is byte-identical to the before listing. Sections:
s4 31801 bytes / 326 rows, s4 debug 32486 / 333, demo 20220 / 209, demo debug
20681 / 214.

### Determinism, and freshness

Two builds of each shape give identical sections (the second run wrote to another `-o`
directory, the only difference, in `DIGEST-ROM`'s path). The content test, on a private
copy of the tree with a fixed `-o`, measures the rest: identical sections across two
builds; identical after touching every aeon-root file the build read (325 on s4 plain:
the source, generated and tool rows) forward one hour; each one-content edit moves exactly its row (for the pitch-table source, its row
and the generated `.bin`'s) and the aggregate; restoring everything restores the
section byte for byte. Its run log:

```text
BINCLUDE: 0 target(s) named by the .asm rows, 0 of them editable source rows
an .emp module: engine/compression/s4lz.emp moved {"engine/compression/s4lz.emp"}
the AS residual root: games/sonic4/game_root.asm moved {"games/sonic4/game_root.asm"}
an included .asm: engine/debug/debugger.asm moved {"engine/debug/debugger.asm"}
map.toml: games/sonic4/map.toml moved {"games/sonic4/map.toml"}
a sound source whose generated file the build reads back: games/sonic4/data/sound/movingtrucks_pitchtable.emp moved {"engine/sound/generated/movingtrucks_pitchtable.bin", "games/sonic4/data/sound/movingtrucks_pitchtable.emp"}
the convsym tool: tools/convsym moved {"tools/convsym"}
an embed()ed blob: art/optimized/characters/knuckles.bin moved {"art/optimized/characters/knuckles.bin"}
```

### Completeness, witnessed by the kernel

```text
s4: 326 rows, the kernel saw the same 326 files
s4.debug: 333 rows, the kernel saw the same 333 files
demo: 209 rows, the kernel saw the same 209 files
demo.debug: 214 rows, the kernel saw the same 214 files
```

### Red-first, every test this parcel adds

Each mutation is one exact replacement (refused unless its needle occurs once), quoted
back from disk, run, then restored with `git show HEAD:<path>` and checked identical to
HEAD. Logs: scratch `redfirst.log`, `redfirst2.log`.

| # | mutation (file:line as mutated) | red |
|---|---|---|
| M1 | `sandbox.rs:261` `let bytes = match std::fs::read(&resolved) {` (embed) | gate; witness; content test |
| M2 | `seam2.rs:1075` `std::fs::write(&p, &bytes)` (pitch table) | content test only (gate green: a write is not a read) |
| M3 | `native.rs:4688` `let out = std::process::Command::new(&convsym)` | gate; witness; content test |
| M4 | `eval.rs:3783` `match std::fs::read_to_string(&path) {` (include) | gate; `read_set_as_reads`; witness; content test |
| M5 | `eval.rs:3855` `match std::fs::read(&path) {` (BINCLUDE) | gate; `read_set_as_reads` (end-to-end green: the corpus BINCLUDEs nothing) |
| M6 | `manifest.rs` the `record_scan` line deleted | witness; content test (the build fails: no module scan); gate green |
| M7 | `manifest.rs:71` `let src = match std::fs::read_to_string(path) {` | gate only (see the half-fix table) |
| M8 | `listing.rs:488` `" DIGEST-READ crc=..."` | collision control (`listing_symbol_addr would read a digest line`), grammar, round trip, parser, keywords |
| M9 | `listing.rs:214` header `"Phase Table digest (...)"` | collision control (oracle header), grammar |
| M10 | `listing.rs:665` the aggregate check guarded by `if false &&` | `source_digest_parser_refuses_a_doctored_section` |
| M11 | `read_set.rs:220` `if !seen.is_empty() {` | `a_file_read_with_two_contents_is_a_conflict_not_a_row` |
| M12 | `read_set.rs:81` the CRC XORed with the file's mtime | content test (the touch assertion) |
| M13 | `listing.rs` `digest_path(&r.file)?;` deleted from the row loop | `source_digest_refuses_what_its_grammar_cannot_carry` |
| M14 | `listing.rs:333` `let sorted` with the sort deleted | scan identity test; grammar test |
| M15 | `read_set.rs:81` `bytes.len() as u64 + 1` | `a_read_records_the_bytes_it_returned` |
| M16 | `read_set.rs:75` `canonical` without `canonicalize` | `two_spellings_of_one_file_are_one_record` |
| M17 | `read_set.rs:109` the write not remembered | `a_generated_write_and_a_tool_mark_their_reads` |
| M18 | `read_set.rs:133` scan members kept absolute | `a_scan_records_its_membership_and_a_changed_walk_conflicts` |
| M19 | `read_set_gate.rs:63` `Command::neww(` | `the_matcher_flags_raw_reads_and_passes_the_recorder` |
| M20 | `read_set_gate.rs:81` the shipping cut disabled | `the_shipping_cut_...`; `no_build_reachable_read_bypasses_the_recorder` |
| M21 | `read_set_gate.rs:107` out-of-line test modules not skipped | `the_shipping_cut_...` (the main gate stays green: `tree_class.rs` holds no raw read) |

`crc32_matches_the_standard_check_value` and `source_digest_scan_identity` are pinned
to external values (the CRC-32 check value `0xCBF43926`; the CRC of the sorted text)
rather than to this code's own output.

### Suites and clippy (on `96fab5e6`)

`SIGIL_STRICT_GATE=1 AEON_DIR=<private tree> cargo test --release -p <crate> --no-fail-fast`:

| crate | passed | failed | ignored |
|---|---|---|---|
| sigil-span | 15 | 0 | 0 |
| sigil-link | 145 | 0 | 0 |
| sigil-isa | 146 | 0 | 0 |
| sigil-frontend-as | 762 | 0 | 0 |
| sigil-frontend-emp | 2668 | 0 | 0 |
| sigil-harness | 469 | 1 | 1 |
| sigil-cli | 780 | 0 | 1 |

The one failure is `m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`, which
stops because no `ORACLE_DIR` was named (strict mode refuses the derived legacy
`oracle-old` checkout). It tests `emit_listing`, which this parcel does not change;
re-run with `ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`, `m1b_gate` is 5 passed,
0 failed.

`cargo clippy --release -p sigil-span -p sigil-link -p sigil-frontend-as
-p sigil-frontend-emp -p sigil-harness -p sigil-cli -p sigil-isa --all-targets --
-D warnings`: exit 0.
