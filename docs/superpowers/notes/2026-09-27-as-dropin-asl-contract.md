# AS-DROPIN-ASL-CONTRACT: sigil as `asl` in the disassemblies' own build scripts, 2026-09-27

Implementation parcel `parcel/as-dropin-asl-contract`, project SIGIL-AS-REPLACEMENT. It closes
gap G1 of `2026-09-27-as-replacement-state.md` and, for anyone building through the scripts,
G2. Every figure below was measured in this parcel's worktree against a sigil built there.

## What a user does now

Copy (or link) the `sigil` executable over `build_tools/<platform>/asl` in `s1disasm`,
`s2disasm` or `skdisasm` and run the disassembly's own, unmodified `build.lua`,
`buildSK.lua`, `buildS3.lua` or `buildS3Complete.lua`. Under the program name `asl` (or
`asl.exe`, extension in any case) sigil takes asl's command line, as `common.lua`'s
`assemble_file` passes it:

```text
asl -xx -n -q -A -L -U -E -i . [-c] [-D NAME=VALUE] X.asm
```

and writes what asl writes: `X.p` for the stock `p2bin`, `X.h` under `-c`, and its
diagnostics to `X.log` under `-E`. It does not place, compress or fix the header; the
script's own `p2bin` and `fix_header` do that, as they do after asl. Under any other name
the executable is `sigil`, unchanged. `sigil --help` says so in four lines.

| option | sigil as asl |
|---|---|
| `-xx -n -q -A` | accepted, change nothing (asl's message shape and symbol storage); no banner is printed either way |
| `-U` | **required**; see the case finding below |
| `-L` | accepted; writes no listing (gap G6 stands) and removes a stale `X.lst` |
| `-E [file]` | diagnostics to `file`, default `X.log`; asl's grammar, so the next argument is the file when it does not start with `-` |
| `-i <dir>` | accepted when `<dir>` is the source's own directory, refused by name otherwise |
| `-c` | share file `X.h`, asl's layout |
| `-D` | asl's command-line define, as on sigil's direct route |
| anything else | refused by name, exit 4, as asl's `Invalid option` |

`X` is the source argument up to its FIRST `.` anywhere in the path, which is how asl names
its outputs (measured: `d.x/f.asm` gives `d.p`, `./sub/in.asm` gives `.p`) and how
`common.lua` finds them; a file name with no `.` is read as `<name>.asm`.

## The files the script reads, derived from the reference asl

All from the reference build, md5 `61e672562465725a8c102288a7da9098`, exit status checked,
on probes in this parcel's scratch; each is also a test in
`crates/sigil-cli/tests/asl_dropin.rs` that runs that asl beside sigil at test time.

- **Object file.** Magic `89 14`; records `81 cpu seg gran u32-start u16-len bytes` (CPU
  `01` 68000, `51` Z80, segment 1, granularity 1); `00` and the creator text. The records
  are the runs `sigil-link/src/blob.rs` already models as p2bin's records, in program
  order, which the note hypothesised: on a probe with two `before` blobs over filler that
  starts at an `!org` rewind (skdisasm's shape) and one `after` blob (s1disasm's), sigil's
  record list equals asl's, CPU, address and bytes, including the filler record that is
  contiguous with the code before it and stays separate. The stock p2bin makes the same
  image of both, and so does sigil's direct route.
- **Share file.** `/* <source>-Include File for C Program */`, one `#define NAME 0xVALUE`
  per name in the order written (upper-case hex, a negative value as its 64-bit two's
  complement), `/* Ende Include File for C Program */`. The value is the one the name has
  AT the `shared` line on the final pass: a `set` symbol rebound below keeps the earlier
  value, a label defined below gets its address, a duplicate is written twice, a bare
  `shared` writes nothing, an undefined name is `error #1010`. s2disasm's own asl
  (`0dee1f98`) writes `s2.h` in the same layout (`#define movewZ80CompSize 0xEC04E`).
- **Log.** Written on errors (and the object file removed) and on warnings beside a written
  object file; a clean run removes a stale log. `common.lua` reads "`.p` and `.log`" as
  warnings, "`.log` only" as errors, and "neither" as a crash. The text in sigil's log is
  sigil's own diagnostics.

## Proof 1: the five scripts, unmodified

Binary: sigil `e6d2e221`, md5 `5c79cbd4c679532de4c41222c12318db` (and again on the final `ecbaff2a`, below), copied to
`build_tools/Linux-x86_64/asl` in a fresh extraction of each disassembly (`git archive` of
the revision below). Stock: the same extraction with its shipped tools. `cmp` over the whole
file, each with a planted-byte control that saw exactly 1 difference. No script printed a
warning, error or crash line; no `.p`, `.h`, `.lst` or `.log` was left at a tree root.

| script | revision | stock (asl md5, p2bin `4f2fff99`) | sigil as asl |
|---|---|---|---|
| s1disasm `build.lua` | `f6ece657c1cf253404312137dfcb8ec15fa42318` | `afe05eee/524288` (`61e67256`) | IDENTICAL |
| s2disasm `build.lua` (28 asl calls: 27 songs, the ROM with `-c`) | `e45ebf332f39987424ca3102e50c717628f71269` | `7b905383/1048576` (`0dee1f98`, S2's own) | IDENTICAL |
| skdisasm `buildSK.lua` | `2fcd861c208f342b6d14df694c6422c74f20a4be` | `0658f691/2097152` (`61e67256`) | IDENTICAL |
| skdisasm `buildS3Complete.lua` | same | `a651623a/3360160` (`61e67256`) | IDENTICAL |
| skdisasm `buildS3.lua` | same | `9bc192ce/2097152` (`61e67256`) | IDENTICAL |

The reference figures of this morning's note all reproduce.

## Proof 2: the option sweep through the drop-in

`scripts/switch_matrix_sweep.py --route dropin`, new in this parcel: sigil copied over the
leg tree's own `asl` and the leg's own `build.lua` run a second time, the substitution
proven by the file being byte-identical to sigil afterwards and by no asl-format `> > >`
line in the output. S1 and S2, 40 legs, binary md5 `ba80534a` (the build that became `4d0ab783`), repeated with the same outcome on the final `ecbaff2a` (md5 `1bc9606b`):
`SELF_TEST PASSED`, `RECONCILE legs launched=40 reported=40`, both C7 controls PASSED
(DIFFER), outcomes **0 non-agreeing**, warning-parity keys 0, the `asl#180 = [as.odd-address]`
pairing seen, `SWEEP PASSED`. The two S2 legs this morning's note called the share-file
residual now agree:

| leg | direct route (unchanged) | drop-in route |
|---|---|---|
| `s2disasm-fixBugs-1` | SIGIL-DECLINED (stream $F88 over the declared $F64) | AGREE `9d758b6f/2097152` |
| `s2disasm-build.lua:improved_sound_driver_compression-1` | DIFFER, 2 bytes at `0x18F 0xEC051` | AGREE `c4e244a2/1048576` |

**The two `ACK_DISAGREE` entries were scoped, not removed.** The nightly runs the default
direct route (`scripts/nightly_switch_sweep.sh`), where both outcomes are still true and
still acknowledged: removing the entries would make the nightly red on a true finding.
`DIRECT_ROUTE_ONLY` names them, with the `shared` warning-gap entry and the
improved-compression cross rule, and a `--route dropin` run leaves those three out of its
tables, which are then asserted in both directions as before. Proven red: with the scoping
disabled, a clean-binary drop-in run fails on exactly those three as stale. The direct
route re-run with this parcel's sweep: 40/40, 2 non-agreeing, 2 acknowledged, 0 stale,
`SWEEP PASSED`.

The first drop-in sweep failed one leg, and that is how the record-splitting rule below was
found: `s1disasm-AllOptimizations-1`, where the run before the DAC driver is 0x71E9A bytes.

**The cross product through the drop-in** (`--route dropin --cross`, the nightly's shape;
binary md5 `ba80534acad048433584e20c519c8dec`, the build of `4d0ab783`'s code before its
commit, whose later changes are tests, `shared A1`, the `-E` guard and help text):
`SELF_TEST PASSED`, `RECONCILE legs launched=808 reported=808`, both C7 controls PASSED,
S1 576 corners (48 declined by the stock toolchain and by sigil alike, all 48 covered by
the existing `ACK_STOCK_DECLINE` rule), S2 192 corners, 0 non-agreeing, 0 acknowledged,
0 stale, `SWEEP PASSED`, exit 0. So the 48 S2 corners the direct route's
improved-compression cross rule covers (2 or 3 bytes at `0x18E`/`0x18F` and `0xEC051`)
agree on the drop-in route too.

## Proof 3: an error through the script

A copy of s1disasm with `move.w #NoSuchSymbolAnywhere,d0` planted at `sonic.asm` line
5237 (read back from disk before the run), sigil as asl: `sonic.log` present, `sonic.p`
absent, no ROM, and `build.lua` prints the log and `There were build errors. See sonic.log
for more details.`, not "The assembler crashed". The stock asl on the same tree takes the
same path with its own wording.

## Records longer than one record

asl starts a new record when the next statement's bytes would take the current one past
`0xFFFF` (s2disasm and skdisasm show records of `0xFF00`, `0xFFFC`, `0xFFFE`, `0xFFFF`
bytes, each continued by the next); sigil's runs are sections and can be far longer. The
writer splits a long run into `0xFFFF`-byte records from its start. That changes no
address and nothing an `after` blob reads (where the record before it ends), so Sonic 1
with `AllOptimizations = 1` agrees, and a test reproduces the shape against the reference
asl and p2bin. **Gap:** for a `before` blob after a run longer than one record, asl's last
record starts at a statement boundary sigil does not track, so p2bin would store the blob
at a different offset. Such a blob overwrites code rather than filler under either tool;
no corpus has the shape.

## The `-U` and case finding

The AS front end folds the case of directives, mnemonics and registers and never of
symbols (the standing ruling). asl's `-U` makes symbols case-sensitive; without it asl folds
them. Measured with the reference asl on four probes:

| probe | asl `-U` | asl without `-U` | sigil as asl `-U` |
|---|---|---|---|
| `Foo = 1`, `foo = 2`, `dc.w Foo,foo` | `0001 0002` | `error #1000: symbol double defined` | `0001 0002` |
| `Bar = 5`, `dc.w BAR` | `error #1010: symbol undefined` | `0005` | refused, unresolved `BAR` |
| `x set 1`, `X set 2`, `dc.w x` | `0001` | **`0002`, no diagnostic** | `0001` |
| `CPU 68000`, `DC.W 7`, `Move.W #1,D0` | `0007 303C 0001` | the same | the same |

So with `-U` the two tools agree on every probe, and without it they can differ silently
(row three). sigil as asl therefore REQUIRES `-U` and refuses its absence by name; every
disassembly script passes it. This goes one step past the brief's "accept it" and is
argued in the refusal's own text.

## `-L`

Accepted, no listing written. No script consumes the listing: across all three
disassemblies' `.lua` files the only `.lst` reference is s2disasm's `build.lua` removing
`song.lst` after each song. A stale `X.lst` from an earlier asl run is removed, so none
stands beside a fresh object file. The AS-style line listing stays gap G6.

## Gaps left, and why

- **G6, the `-L` listing**, above. Its own parcel (`sigil-link/src/listing.rs` has the
  symbol-table half, not the line half).
- **`before` after a run longer than one record**, above.
- **`-i`** names only the source's own directory; sigil searches no other, so any other is
  refused. Nested includes resolve against the root's directory (the front end's standing
  behaviour), where asl looks in the including file's directory first.
- **One source per call.** asl assembles several; sigil as asl refuses a second by name.
- **Exit status.** 4 for a refused command line (asl's), 2 for any assembly failure (asl
  uses 2 for errors and 3 for `fatal`); the scripts read files, not the status.
- **`-E` naming the source.** asl reads `-E x.asm` as the log's name and, with no source
  and nothing to report, removes the file: the reference build deleted `sub/in2.asm` this
  way here. sigil reads the line the same way and refuses without touching the file.
- **The direct route keeps the S2 residual** (`sigil s2.asm ...` with
  `improved_sound_driver_compression = true` is still 2 bytes wrong, silently; `fixBugs`
  still refused). Closed for build-script users only.
- **The nightly still sweeps the direct route only.** Adding a `--route dropin` nightly
  pass touches the nightly job and is the controller's call.
- **Windows.** `asl.exe` is recognised by name; there is no Windows build (G8).
- **Speed.** Not measured on this route (G5 stands).

## Tests

`crates/sigil-cli/tests/asl_dropin.rs` (9): personality by name, including a symlink and
the names that must NOT select it; refusal by name with the log under `-E` and a stale
`.p` removed; `-U` required; `-i`; `-E` naming the source; and, against the reference asl
and stock p2bin read by `git archive` of s1disasm `f6ece657`'s `build_tools/Linux-x86_64`
with md5 checks (the pattern of `as_driver_placement_corpus.rs`, with `suite_root_absent`
when there is no suite root): record list and order, the long-run split, the share file
byte for byte, and the `-E` file states. `asl_mode.rs` unit tests (7), `code_file.rs` unit
tests (4), `as_shared.rs` (2 new). Each was proven red by a mutation quoted from disk, run,
and restored from HEAD; the list is in commit `e6d2e221`'s message.

## Landing run

`scripts/landing-run.sh --baseline 5745 --aeon /home/volence/sonic_hacks/.aeon-sigil-ref`
(the baseline is the most recent landing log found on this machine, `land-modreg`,
2026-09-26, 5745 passed), aeon `ec640bcf`.

- Run 1, HEAD `d937961b`: 5766 passed, **1 failed**, 2 ignored, 0 skip lines, clippy 0,
  ledger 0. The failure was `stdout_writer_population`: `asl_mode.rs` printed `message`
  lines with its own `println!`, which that gate counts as a library file writing to
  stdout outside the broken-pipe rule. Fixed in `ecbaff2a` by printing through main.rs's
  `render_as_messages`, the direct route's own function.
- Run 2, HEAD `ecbaff2a` (tracked files clean; the stamp's DIRTY is the untracked scratch
  directory): **5767 passed, 0 failed, 2 ignored, 0 skip lines**, CARGO_EXIT 0,
  CLIPPY_EXIT 0, LEDGER_EXIT 0, reconciles 5745 + 22 new, **RESULT GREEN**.
  `asl_dropin.rs` ran in it. This worktree's `repin --check` prints `pins.rs unchanged`.

On `ecbaff2a` (sigil md5 `1bc9606b37c12b0af54367e1b042c27f`) the five scripts were run
again (all IDENTICAL, same CRCs as the table above), and so were the error proof (the same
output) and the drop-in sweep (40/40, 0 non-agreeing, SWEEP PASSED).
