# `.s1probe/`: Sonic 1 corpus captures and asl probe fixtures

This directory is evidence, not code. It holds what the Sonic 1 corpus
parcels measured: sigil's raw diagnostic captures over `s1disasm` (and two
over `s2disasm` for comparison), the coverage and head-frequency probes, the
stub patch that let the corpus reach the linker, and small AS probe sources
whose expected values were read off the reference assembler. Each file is the
artifact a named commit or note cites, kept so a reader can check the number
the note quotes. Nothing in the build, the suite or `scripts/` reads this
directory.

## The reference assembler

Read `docs/OVERSEER-REFERENCE.md`, block "Selecting and citing the `asl`
oracle", and `docs/superpowers/notes/asl-reference/README.md` before quoting
any value from here. In short:

- The reference is identified by **md5 `61e672562465725a8c102288a7da9098`**,
  never by its banner. Four different `asl` builds on this machine print
  `Macro Assembler 1.42 Beta [Bld 212]` verbatim. The reference is reached at
  `s1disasm/build_tools/Linux-x86_64/asl` (and at `skdisasm`'s and
  `sonic_hack`'s copies, same digest). Measured 2026-09-11: the `s1disasm`
  path still holds md5 `61e672562465725a8c102288a7da9098`.
- `s2disasm/build_tools/Linux-x86_64/asl`, md5
  `0dee1f98e6480a4783d27ffd8b90896f`, is refused: it answers a declined
  operand from uninitialized memory.
- A stable value is not an answer. For an operand it declines to value, the
  reference build substitutes the last value it computed, with exit 0 and no
  diagnostic. A probe that exits 2 here records a REFUSAL, and any number asl
  prints on a declined operand is an artifact under either digest.

## What is here, and what produced it

75 tracked files before this README. "sigil" below means the sigil build
named in the commit or note, identified by revision and, where one was
recorded, by md5. Where a producing instrument's md5 was not recorded at the
time, this README says so rather than supplying one.

### Top level: the 2026-09-03 baseline

Note: `docs/superpowers/notes/2026-09-03-s1-corpus-baseline.md`. Corpus
`s1disasm` at `f6ece657`, run as `sigil sonic.asm` from the corpus root. The
note's provenance stanza names the oracle as `s1disasm`'s own `asl`, md5
`61e672562465725a8c102288a7da9098`, with `-xx -n -q -A -L -U -E -i .`, and
says every probe in that parcel used that binary.

| file | added by | what it is | produced by |
|---|---|---|---|
| `s1run.err.gz` | `dd9d201e` | sigil stderr, 9,747 lines: the corpus without its generated DAC includes | sigil built from `5bd06567`, md5 not recorded |
| `s1run2.err.gz` | `dd9d201e` | sigil stderr, 9,739 lines: the baseline, generated includes present | same |
| `s1cov.err.gz` | `7491c371` | sigil stderr, 10,176 lines, over the corpus with a marker planted in each of its 444 files | same |
| `asl_marks.txt` | `7491c371` | the 439 markers asl reports | `s1disasm`'s asl, md5 `61e67256...` per the note |
| `sigil_marks.txt` | `7491c371` | the 437 markers sigil reports | sigil as above |
| `s1_include_closure.txt` | `7491c371` | the 444 files in `sonic.asm`'s include closure | derived from the corpus |
| `s2run.err.gz` | `7491c371` | the same sigil over `s2disasm` `e45ebf3`, 13,109 lines | sigil as above |
| `s1_heads.txt`, `s2_heads.txt` | `b1c5e40e` | count and head token of every source line, per corpus | derived from the corpora |
| `s1_unresolved_symbols.txt` | `b1c5e40e` | the 86 symbols the S1 run leaves unresolved | derived from the captures |
| `construct_probe.sh` | `b1c5e40e` | 28 one-line constructs run through sigil alone (no asl) | see "Re-running" |
| `asl_build.log` | `f02c1379` | stdout of the asl build that produced the bit-perfect REV01 reference ROM (524,288 bytes, md5 `09dadb5071eb35050067a32462e39c5f`) | `s1disasm`'s asl per the note; the commit itself names no md5 |

### `2026-09-04/`: the path to a ROM

Note: `docs/superpowers/notes/2026-09-04-s1-path-to-rom.md`, whose provenance
table names sigil `7bef76e6` (md5 `242427190b581e5b5776a1874da990d1`) and the
oracle `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`.

| file | added by | what it is |
|---|---|---|
| `s1_318.err.gz` | `d2c32940` | sigil `7bef76e6` over `s1disasm` `f6ece657`: 318 lines |
| `s2_9432.err.gz` | `d2c32940` | the same binary over `s2disasm` `e45ebf3`: 9,432 lines |
| `s1-stub.patch.gz` | `d2c32940` | the thirteen source-side stubs (1,003 diff lines) that let the corpus reach link |
| `stub.py`, `stub_range.py` | `d2c32940` | the generators of that patch |
| `diffmap.py`, `locate.py` | `d2c32940` | the image-diff and listing-locate instruments |
| `probe/p1.asm` ... `probe/p10.asm` | `d2c32940` | ten differential probes; their asl and sigil values are quoted in the note |
| `probe/cmp.sh` | `d2c32940`, `163be7f6` | runs asl and sigil on one probe; selects asl by md5 through `asl_ref.sh` |
| `probe/f4/` (11 probes and a README) | `ce9d998c` | the F4 probes: macro default parameters, and an empty operand's span. Their own `README.md` says what each pins |

The four Python scripts hardcode paths under `/home/volence/sonic_hacks/.s1recon-*`,
and none of those exist now (checked 2026-09-11). They are a record of the
method, not something to run as committed.

### `2026-09-09/`: the census

Commit `5a57e2bf`. Its census is sigil `c6179e8a`, md5
`1b0357ce26db9ae79473244cc219aa01`, over a pristine `s1disasm` worktree at
`f6ece657`, and its oracle line names asl md5
`61e672562465725a8c102288a7da9098`.

| file | what it is |
|---|---|
| `s1-census-c6179e8a.err` | the census: 17 diagnostics, in two causes (B `charset`, E `switch` over an integer) |
| `s1-census-stubBE-c6179e8a.err` | the same run over a scratch copy with B and E stubbed: the front end is clean and the run stops in layout on one section-overlap diagnostic |
| `canary.asm` | a planted `dc.b NeverDefinedAnywhereCanary`, the positive control that the unresolved-symbol matcher can find a name |
| `switchint.asm`, `switcharm.asm` | an integer `switch` whose matching arm is second; `switcharm.asm` names the arm taken with a `message` |

### `2026-09-09-switch-int/`: the integer selector

Commits `9aed994f` (25 files) and `7ccd4dba` (3 files). The unit tests in
`crates/sigil-frontend-as/src/eval.rs` quote their expected values from these
listings.

| file | what it is | produced by |
|---|---|---|
| `p1.asm` ... `p22.asm` | 22 selector probes | hand-written |
| `asl-listings.txt` | every probe's listing, with its exit code | `capture.sh`; the file's own header records the asl md5 it ran, `61e672562465725a8c102288a7da9098`, and the invocation |
| `capture.sh` | re-captures `asl-listings.txt` | |
| `run.sh` | runs one probe and prints everything | see "Re-running" |
| `real_dac_v2.asm`, `real_dac_v2_silent.asm` | Sonic 1's own DAC `switch` block, lines 88 to 168 of `sound/_smps2asm_inc.asm`, with the matching arm second | lifted from the corpus |
| `real_dac_v2.lst` | asl's listing of `real_dac_v2.asm` | `7ccd4dba` cites the banner and no md5; identified by the check below |

**Which build wrote `real_dac_v2.lst`.** The commit that added it names only
the version banner, which identifies nothing. The listing's own symbol table
does better: it records `*ARCHITECTURE` as `"x86_64-unknown-linux"`. Measured
2026-09-11, re-assembling `real_dac_v2.asm` with each build on this machine:

| build | md5 | `*ARCHITECTURE` | differs from the committed listing in |
|---|---|---|---|
| reference, `s1disasm/.../Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | `x86_64-unknown-linux` | the page header and the `*DATE`/`*TIME` rows only |
| upstream i386, `s1disasm/.../Linux-x86/asl` | `a8cd8b80b765686b2e9266c31ffa6987` | `i386-unknown-linux` | also `*ARCHITECTURE` |
| flamewing, `s2disasm/.../Linux-x86_64/asl` | `0dee1f98e6480a4783d27ffd8b90896f` | `x86_64-Linux` | also `*ARCHITECTURE` |

So among the builds on this machine, only the reference digest reproduces the
committed listing. That is an identification made after the fact from the
artifact, not a record of what ran; the fourth digest in the asl-reference
table (`aa6de52f...`, flamewing) was not run.

## Re-running

Every runner writes its output next to the probe source it is given, so run
it on a copy, or in a copy of the directory, to keep the tracked tree clean.

- **`2026-09-09-switch-int/capture.sh`** (zsh) rewrites `asl-listings.txt` in
  its own directory. `ASL=<path>` overrides the assembler, and the file's
  header records the md5 of whatever ran, so a run under the wrong build is
  visible in the output. To check the committed file, copy the directory
  elsewhere and run the copy. Measured 2026-09-11 that way under md5
  `61e672562465725a8c102288a7da9098`: all 410 lines outside the page headers
  (which carry the run's date) match the committed file, `diff` exit 0, 22
  probes, the same exit code for each.
- **`2026-09-04/probe/cmp.sh <probe.asm>`** (bash) selects asl through
  `docs/superpowers/notes/asl-reference/asl_ref.sh`, which refuses every digest
  but the reference. Its sigil half hardcodes
  `/home/volence/sonic_hacks/.sigil-s1recon-target/release/sigil`, which no
  longer exists, so only the asl half runs as committed. Measured 2026-09-11
  on a copy of `p1.asm`: asl exit 2, three `error #1133` lines.
- **`2026-09-09-switch-int/run.sh <name>`** hardcodes the reference path
  without checking its digest, and its working directory is
  `.claude/worktrees/agent-a903dfc1b67f61ad8/.probework`, another worktree,
  not this directory. Use `capture.sh` instead.
- **`construct_probe.sh`** runs sigil only, from
  `/home/volence/sonic_hacks/.cargo-s1baseline/release/sigil`, which no longer
  exists. Set `S` in a copy to a built sigil. It writes `p.asm` into the
  current directory.
- The `.err` captures are sigil's own output. To reproduce one, build the
  sigil revision named above and run it from the corpus root at the named
  corpus commit; the numbers move with sigil, so a different count from a
  later sigil is expected, not a discrepancy.
