# SIGIL-AS-REPLACEMENT: where "sigil can build Sonic 1 and Sonic 2" stands, 2026-09-27

Measurement and proposal parcel (`parcel/as-replacement-state-r2`). It proposes; nothing lands
on it. Every figure below was re-measured in this run against a sigil built from this worktree,
unless a line says otherwise. A first run of the same brief died uncommitted; its claims were
treated as hypotheses and each one is marked below as reproduced or not.

## Instruments

| what | identity |
|---|---|
| sigil | built here, `CARGO_TARGET_DIR=<worktree>/target cargo build --release`, source `398e5b15`, binary md5 `dffd101f6707c2d08a6fe2bf25f5f42e` |
| reference asl (S1, S3K) | md5 `61e672562465725a8c102288a7da9098` (the copy each of `s1disasm` and `skdisasm` ships) |
| S2's own asl | md5 `0dee1f98e6480a4783d27ffd8b90896f`, the build OVERSEER-REFERENCE refuses as an oracle. It is the only asl that builds Sonic 2 (see below), so it is used ONLY as the S2 whole-ROM reference, never as a source of per-line values |
| p2bin | md5 `4f2fff99c3347bafb93b12d5be1db754` in all three repos |
| s1disasm | `f6ece657c1cf253404312137dfcb8ec15fa42318` (2026-08-01) |
| s2disasm | `e45ebf332f39987424ca3102e50c717628f71269` (2025-08-27) |
| skdisasm | `2fcd861c208f342b6d14df694c6422c74f20a4be` (2025-08-30) |

Each corpus was extracted with `git archive <sha>` into scratch inside this worktree (never its
working tree: `s1disasm` has two modified `.nem` files and `skdisasm` has untracked save files,
and neither can colour a result read from a commit). No disassembly repo was written to.
Checksums are zlib CRC-32 `crc/bytes`; identity claims are `cmp` over the whole file, each with a
planted-byte control that `cmp -l` saw as exactly 1 differing byte.

## 1. What builds, byte-identical, by which entry point (stock settings)

Measured 2026-09-27T09:19Z to 09:19:54Z, load average 6.7 to 10.0 (see section 6 for why timing
under this load is only indicative).

| corpus | stock build script, stock tools | sigil, direct one command | sigil dropped in as `asl` | sigil behind a two-file shim |
|---|---|---|---|---|
| S1 `build.lua` | exit 0, `afe05eee/524288` | IDENTICAL | FAILS, no ROM | IDENTICAL |
| S2 `build.lua` | exit 0, `7b905383/1048576` (S2's asl) | IDENTICAL | FAILS, no ROM | IDENTICAL (28 assembler calls: 27 songs + ROM) |
| S3K `buildSK.lua` | exit 0, `0658f691/2097152` | IDENTICAL | FAILS, no ROM | IDENTICAL |
| S3 Complete `buildS3Complete.lua` | exit 0, `a651623a/3360160` | IDENTICAL | FAILS, no ROM | IDENTICAL |
| S3 alone `buildS3.lua` | exit 0, `9bc192ce/2097152` | IDENTICAL | FAILS, no ROM | IDENTICAL |

The direct commands, each run in a copy of the stock tree after the stock script had produced
its generated inputs (PCM/DPCM, S2's compressed songs):

```text
sigil sonic.asm   -o sigil.bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
sigil s2.asm      -o sigil.bin -p=0  -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
sigil sonic3k.asm -o sigil.bin -D Sonic3_Complete=0 -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
sigil sonic3k.asm -o sigil.bin -D Sonic3_Complete=1 (same -p/-z)
sigil s3.asm      -o sigil.bin -p=FF -z=0,uncompressed,Size_of_Snd_driver_guess,before -z=1300,uncompressed,Size_of_Snd_driver2_guess,before
```

The first run's `logs-direct.txt`, `logs-refs.txt`, `logs-dropin.txt` and `logs-shim.txt` all
REPRODUCE, same CRCs, on a sigil four commits newer (the difference is a queue doc).

**S2 cannot be built by the reference asl at all.** With md5 `61e67256` and its own message
catalogs in S2's tool slot, `build.lua` exits 1 on `error #1010: symbol undefined` inside the
`jsrto`/`jmpTos` macros (`s2.asm(13535)` first). This is already known to the notes ("the pinned
build cannot assemble Sonic 2", `2026-09-11-s1-driver-stage2.md`). Copying the pinned binary
alone into S2's slot fails earlier, on S2's own `.msg` catalogs ("as.msg has invalid format"),
which is what this run's first `s2-refpin` leg hit; the first run's `s2-refpin` also exited 1.
For S2, sigil's byte-identity is therefore against the refused build, which is S2's actual
toolchain; its ROM reproduced at the same CRC in both runs.

## 2. Every build option the corpora document

**S1 and S2**, by the repo's own `scripts/switch_matrix_sweep.py` (switch set derived from the
corpus source, not a list), run against this worktree's sigil and the two pinned revisions,
09:21:10Z to 09:22:52Z, load 9.6 to 10.0. `SELF_TEST PASSED`, `RECONCILE legs launched=40
reported=40`, `SWEEP PASSED`, exit 0. Both controls (a source edited after the reference was
built) reported DIFFER as they must.

| corpus | legs agreeing | not agreeing |
|---|---|---|
| S1 | shipped, `Revision` 0 and 2, `FixBugs=1`, `CheatsEnabled=1`, `AllOptimizations=1`, `EnableSRAM=1`, `improved_dac_driver_compression=true`, and the three SRAM arms measured through `EnableSRAM=1` (their single-flip legs change no stock byte, so they are reported NOT-MEASURED, not agreed) | none |
| S2 | shipped, `gameRevision` 0 and 2, `padToPowerOfTwo=0`, `allOptimizations=1`, `skipChecksumCheck=1`, `useFullWaterTables=1` | `fixBugs=1` SIGIL-DECLINED; `improved_sound_driver_compression=true` DIFFER, 2 bytes |

Both S2 exceptions are one mechanism, already adjudicated and acknowledged in the sweep's own
`ACK_DISAGREE` table, and re-derived here from bytes (`logs-s2improved.txt`): stock `p2bin`
writes the real compressed driver size into S2's share file (`s2.h` read back:
`comp_z80_size 0xF4A ...`, `#define movewZ80CompSize 0xEC04E`), and `build.lua`'s
`amend_sound_driver_size` patches the `move.w` immediate at `0xEC04E+2`. sigil writes no share
file, so the patch is skipped: at `improved=true` the image holds `3E3C 0F64` where stock holds
`3E3C 0F4A` (offset `0xEC051`) plus the header checksum that follows it (`0x18F`); at
`fixBugs=1` the stream grows to `$F88` against the declared `$F64`, and sigil refuses rather
than write a ROM whose decompressor is told `$24` bytes too few (its message names the fix). So
the first run's two S2 "candidate defects" reproduce exactly (`b8a9299a/1048576` against
`c4e244a2/1048576`, differing at `0x18F` and `0xEC051`, the first run's cmp offsets 400 and
966738 being 1-based) and are **not new**: they are the booked share-file residual. Its
hypothesis that offset 400 is the checksum following a real difference elsewhere is confirmed.

**S3K family**, which the sweep does not cover (it derives S1 and S2 only), by a 9-leg flip
script: stock build script with stock tools against the same script through the shim.
`LEGS_RUN=9 (expected 9)`, no stock image equal to its unflipped reference (no vacuous leg).

| leg | stock | sigil (shim) |
|---|---|---|
| S3K `FixBugs=1` | `76217412/2097152` | identical |
| S3K `strip_padding=1` | `94329966/2097152` | identical |
| S3K `improved_sound_driver_compression=true` (kosinski-optimised, both `-z`) | `9fe129b9/2097152` | identical |
| S3C `FixBugs=1` | `fd73f73b/3360160` | identical |
| S3C `improved_sound_driver_compression=true` | `a1d89e50/3360160` | identical |
| S3 `FixMusicAndSFXDataBugs=1` | `12266a78/2097152` | identical |
| S3K / S3C `FixMusicAndSFXDataBugs=1` | exit 1: the source's own `fatal`, `Size_of_Snd_Bank1 = $3EFC, but you have $3EFD bytes of music` | exit 1, same message, but see section 5 |
| S3 `FixBugs=1` | exit 1: `s3.asm(6446)` `symbol undefined` (`Ctrl_1_Press`) | exit 1, same line and symbol |

The three failing legs fail with BOTH assemblers for a corpus reason (a bank the option
overflows, a symbol `s3.asm` does not define), so they are not sigil gaps. The two
`improved_*` legs are new coverage relative to the first run.

## 3. What a user swapping `asl` for sigil in the disassembly's own pipeline hits

All three corpora share one `build_tools/lua/common.lua` contract (`assemble_file`, lines
773 to 790 in `s1disasm`), and sigil meets none of it:

| the pipeline does | sigil today |
|---|---|
| `asl -xx -n -q -A -L -U -E -i . [-c] [-D N=V] X.asm` | refuses at the first flag: `error: unexpected argument '-xx'`, exit 2. Measured on all five scripts (`logs-phase1.txt`, DROPIN rows): no ROM, no artifact at the tree root. The only asl flags sigil takes are `-D` and none of the others |
| success is "`X.p` exists" | writes no `.p` (its only output is the finished ROM, `-o`) |
| failure is "`X.log` exists" (`-E`), printed by the script; neither file means "The assembler crashed" | writes diagnostics to stderr only, so any sigil failure would be reported by the script as a crash (derived from `common.lua` 777 to 783, not run) |
| `p2bin <-p/-z> X.p X.bin [X.h]` does placement and Z80 compression | sigil does the same work itself, from `-p`/`-z` given to IT, so the script's split into two calls has no seam to hand them over |
| `-c`: `shared` symbols to `X.h`; `p2bin` appends `comp_z80_size`; S2's `build.lua` patches the ROM from it | no share file; `shared` warns and is ignored (every S2 leg) |
| `-L`: `X.lst`, left beside the ROM for the user | no AS listing on this route |
| `fix_header` in Lua | sigil folds the same header fix into its own output (idempotent, so the script's pass is harmless) |

**The shim** (`scratch-r2/shim/`, copied from the first run; a measurement device, not a
deliverable) is the concrete description of the gap. Its `asl` records the argument vector into
`X.p` as JSON and ignores `-xx -n -q -A -L -U -E -i`; its `p2bin` runs one `sigil X.asm -o
X.bin -D ... -p ... -z ...` and touches an EMPTY `X.h`. With it all five scripts produce the
stock ROM (section 1). What it cannot do is the share-file value, which is exactly the S2
residual in section 2.

**A real `.p` is cheap and the stock `p2bin` accepts one, measured.** `prewrite.py` re-encodes
asl's own `.p` with a minimal writer (magic `89 14`, `0x81` records of CPU `0x01` 68000 or
`0x51` Z80, segment 1, granularity 1, `u32` start, `u16` length, creator record) and the stock
`p2bin` (md5 `4f2fff99`) is run on both:

| writer shape | S1 | S2 (with `X.h`) | S3K |
|---|---|---|---|
| records sorted by CPU and coalesced | DIFFER (5,707 B, wrong size) | DIFFER (3,821 B) | DIFFER (8,610 B) |
| emission order kept, every contiguous same-CPU run coalesced | identical | identical, and `p2bin` wrote `comp_z80_size 0xF64` into the `.h` | DIFFER (7,039 B) |
| emission order kept, and the run immediately before each other-CPU record kept separate | identical | identical | identical |

So a `.p` writer has two constraints and both are measured: **file order is meaning** (a `-z`
blob is placed relative to the code around it in the stream), and **the record just before
each Z80 record is meaning** (`before` stores the compressed blob over that run; the S3K
variants use `before`). sigil's `sigil-link/src/blob.rs` already models exactly this "run before
the blob" to reproduce p2bin, so the information exists on sigil's side; this is a hypothesis
about the implementation, not a measurement of it. The rest of the records (440 to 866 per
corpus in asl's output) can be merged freely.

## 4. Beyond the corpora: what a hack author writing new code hits

The corpora can only measure the forms they contain. A disassembly user writes new code, so
this run re-ran the first run's instruction-form sweep (`probes/isa_sweep.py`, 143 forms)
extended by 24 common author spellings, each form alone in a file through the reference asl
(`asl_run`, md5 `61e67256`, exit status checked, bytes read only from exit-0 runs) and sigil.

**167 forms: 134 agree byte for byte, 31 sigil refuses where asl assembles, 1 sigil accepts where
asl refuses, 1 both refuse.** The 31:

| class | forms (asl bytes) |
|---|---|
| unsized branch (asl sizes it) | `bra`, `bsr`, `beq` with no suffix (`6002`, `6102`, `6702`); sigil: "branch needs an explicit size suffix (.s or .w), Aeon pins branch width, no relaxation" |
| `.b`/`.l` branch suffix | `beq.b`, `bra.b` (`6702`, `6002`), `bra.l` (`6000 0002`) |
| 68000 mnemonics sigil does not know | `chk.w`, `illegal`, `link`, `unlk`, `reset`, `rtr`, `stop`, `trapv` |
| unsized operation, asl defaults to `.w` | `move #1,d0`, `add #1,d0`, `addq #1,d0`, `cmp #1,d0`, `clr (a0)`, `tst (a0)`, `ext d0` |
| `ccr`/`sr` immediates with no suffix | `andi`, `ori`, `eori` to `ccr` and `sr` (6 forms) |
| asl's operand-driven aliases | `add.w d0,a1` / `sub.w d0,a1` (asl emits `adda`/`suba`), `eor.w #1,d0` (asl emits `eori`) |
| absolute with a `.w` suffix on a literal | `jmp $1234.w` ("trailing tokens in operand") |

Plus two outside the sweep, from the probe set: bare `ds 2` (asl reserves 4 bytes, `ds.w`;
sigil "not a recognized 68000 mnemonic") and unquoted `binclude blob.bin` (sigil "BINCLUDE needs
a quoted path"). The one over-acceptance is `swap.w d0` (asl refuses, sigil emits `4840`).
Bare absolute operands (`jmp Label`, `jsr`, `lea`, `move.w Label,d0`) AGREE (`4EF8 0010` etc.).

**Amended 2026-09-27 by AS-AUTHOR-FORMS-EXACT's edge probes** (126 probes, same `asl_run`, bytes from exit-0
runs only, `2026-09-27-as-author-forms-exact-probes/`). The picture above was narrower than the truth in
five places. (1) The `ccr`/`sr` row was not only unsuffixed: sigil also refused the SUFFIXED `eori.b #1,ccr`
and `andi.w`/`ori.w`/`eori.w #,sr` (no ISA row), and it ACCEPTED `andi.w #$FE,ccr`, which asl refuses.
(2) `swap.w` is one of three over-acceptances: asl refuses `swap.b`, `swap.w` and `swap.s` and accepts
`swap.l`. (3) `binclude` with an offset and length emitted the whole file with no diagnostic
(`binclude "f",1,2` gave all four bytes of a 4-byte file where asl gives two): silent wrong bytes, quoted or
not. (4) asl folds the width suffix's case, so `jmp ($1234).W` was refused too. (5) asl assembles `chk.l`
(`4300`, the 68020 form) under `cpu 68000`, and turns a `.s` branch to the next instruction into `4E71`
(a NOP); sigil refuses both.

**None of these appear in the three corpora, so no corpus measurement can see them.** A census
of every `.asm`/`.inc` line (`formcount.py`, textual, comment-stripped, dead arms included):
S1 459 files / 74,403 lines, S2 371 / 130,111, S3K 959 / 410,221: **0** unsized branches, **0**
`.b`/`.l` branch suffixes, **0** of the eight missing mnemonics, **0** bare `ds`. Positive
control: the same program on a copy of `sonic.asm` with three planted lines counts 2 branches
and 1 `ds`. The first run's `unsized_count.py` also printed zeros, but **its zero was vacuous**:
it tracks `cpu` lines linearly, and `sonic.asm:323` has a `CPU Z80` inside the driver include,
after which it skips every line of the file; the same three planted lines scored 0 there.

**The community's trust complaints, probed** (empyrean `docs/2026-09-02-as-community-feedback.md`):

| probe | reference asl | sigil |
|---|---|---|
| Selbi's `addq.b #2,obRoutine(a0` | exit 0, emits nothing for the line, no diagnostic | `paren.asm(3):12: error: trailing tokens in operand` |
| vladikcomper's `StartOffset > EndOffset` repro, as written | exit 3, the `fatal` fires | refused earlier: its `bra` is unsized and its `ds` bare (section 4 table) |
| the same repro with `bra.w` and `ds.b` | exit 0, `6000 0108 / 6000 0104` | identical bytes |
| a `StartOffset > EndOffset` guard with NO `MOMPASS` guard, plus an out-of-range `bra.s` | exit 3: `error #1820: expression must be evaluatable in first pass` and then the `fatal`, whose claim is FALSE (StartOffset precedes EndOffset); the real defect, the `bra.s` reach, is never reported | `smps3.asm(3):2: error: bra.s/Bcc.s displacement out of range (258)`, the real defect and nothing else |
| EQU trampling a local label (`.thing`) | `error #1010: symbol undefined` | `unresolved symbol Top.thing`, which names the scope it looked in |

## 5. Diagnostics when a build fails

On the S3K `FixMusicAndSFXDataBugs=1` leg both toolchains refuse for the same reason, the
source's own `fatal` at `sonic3k.asm(201045)`. The stock build prints that one line and stops.
sigil prints **563 error lines, 562 of them `unresolved ...` cascades** (467 `unresolved long
expression` from line 78 on, the rest unresolved symbols), and the one line that names the
cause is the **468th**. The small form of the same shape is `probes/fatal2.asm`: after the
`fatal` sigil adds `unresolved symbol Later`, which is false (it is defined on the last line).
Hypothesis: the front end stops mid-pass at the `fatal`, so every forward reference it had not
reached is reported as unresolved. A user reading from the top would conclude sigil cannot
handle the source.

## 6. Performance

Assembler step only (`timing.sh`): `asl` with the pipeline's flags plus `p2bin`, against
sigil's single call, 5 interleaved runs each, in trees holding the generated inputs. CPU
figures are the child's `getrusage`; every sigil image equalled the asl+p2bin image.
16 cores, load average 9.3 to 11.3 throughout (a busy machine, so absolute figures are
indicative; the ratios are interleaved run by run).

| corpus | asl + p2bin wall | sigil wall | ratio | peak RSS asl / sigil |
|---|---|---|---|---|
| S1 (05:23:47 to 05:23:56 local) | 0.51 to 0.58 + 0.01 s | 1.03 to 1.15 s | 2.0x | 14 MB / 70 to 73 MB |
| S2 (05:24:25 to 05:24:46) | 1.05 to 1.22 + 0.00 s | 2.75 to 3.00 s | 2.5x | 14 MB / 112 MB |
| S3K (05:24:02 to 05:24:18) | 0.87 to 1.00 + 0.01 s | 2.18 to 2.39 s | 2.5x | 14 MB / 250 MB |

User CPU is within 0.1 s of wall for both, so both are single-threaded. Whole-script wall
through the shim is worse for S2 (4.33 s against 1.53 s stock) because the 27 compressed songs
are 27 more full sigil runs. The first run's "about 2x" reproduces (it measured 1.9x, 2.4x,
2.3x). Where sigil's time goes is **not measured**: no profiler (`perf`, `valgrind`, `samply`)
is installed and the AS route has no phase timing. Build time is the complaint the community
repeats most (four respondents), so sigil is currently slower on the one axis users named
first.

## 7. The gap list

"Does sigil build Sonic 1 and Sonic 2" is **yes** for anyone who runs sigil's own one-line
command: every stock entry point of all three disassemblies, and every documented build option
the sweep derives, byte-identical, except S2's two share-file legs. The gap is everywhere else a
user touches:

| # | gap | who hits it | loud or silent | where |
|---|---|---|---|---|
| G1 | sigil cannot stand in for `asl` in the corpora's own build scripts: no asl flags, no `.p`, no `.log`, no `-c` share file | every user who swaps the binary, all five scripts | loud (script fails) | sections 1, 3 |
| G2 | S2 compressed-driver size is not handed back (`fixBugs=1` refused, `improved_sound_driver_compression` 2 bytes wrong) | S2 users flipping either option | `fixBugs` loud; `improved` **silent**, a ROM whose decompressor reads the wrong byte count | section 2 |
| G3 | 31 common 68000 author spellings refused, 1 accepted that asl refuses | anyone writing new code | loud (refused), `swap.w` silent | section 4 |
| G4 | a source `fatal` is buried under hundreds of false `unresolved` cascades | anyone whose build fails on a guard | loud but misleading | section 5 |
| G5 | 2.0x to 2.5x slower than asl+p2bin, 5x to 18x the memory | everyone, on the complaint the community ranks first | n/a | section 6 |
| G6 | no AS listing (`-L`), which the pipeline leaves beside the ROM for debugging | users who read `sonic.lst` | loud (absent) | section 3 |
| G7 | `sigil <input.asm> --help` lists `-z` formats `uncompressed, kosinski or saxman-bugged`; it also takes `kosinski-optimised`, `saxman-optimised`, `saxman` (`p2bin_codec.rs`), which the sweep exercised | users reading the help | stale text | this run |
| G8 | no Windows build (the community notes most of the scene is on Windows) | Windows users | n/a | not measured, section 9 |

## 8. Ranked next-parcel candidates

Ranked by what a disassembly user gains per unit of risk. **Anything marked OWNER changes a
user-visible spelling or behaviour and goes to the owner before landing.** None of them adds
`.emp` language surface.

**1. AS-DROPIN-ASL-CONTRACT (closes G1, and should close G2 for build-script users). Size M.**
Accept the asl flag set `common.lua` passes (`-xx -n -q -A -L -U -E -i <dir> -c -D`, the
cosmetic ones accepted and ignored, as the shim does), and on that route write `X.p` for the
stock `p2bin`, `X.h` for `shared` under `-c` (asl's three-line `#define` layout, which `p2bin`
then extends with `comp_z80_size`), and diagnostics to `X.log` under `-E`. Then one replaced
file makes all five scripts work, and the stock `p2bin` and `build.lua` do the S2 size patch
themselves, so both S2 exceptions should become agreements (hypothesis: verify by running the
switch sweep through the drop-in, which is the acceptance bar, not the five shipped ROMs).
*Sized off*: the shim (two files, about 60 lines) already reproduces all five stock ROMs through
the unmodified scripts; the `.p` feasibility probe shows a writer of about 30 lines that the
stock `p2bin` accepts and that reproduces S1, S2 (with the `.h` round trip) and S3K, once it
keeps emission order and the run before each Z80 record. *Risk*: record boundaries carry
meaning to `p2bin` and two such rules are now measured; option legs may show more, which is why
the sweep is the bar. On this route sigil's own `-z` compressors and header fold are bypassed,
so identity rests on the stock `p2bin`, which is the point of a drop-in. *Needs from the
owner* (OWNER): how sigil knows it is being asked to be `asl` (an `argv[0]` of `asl`/`asw`, or
any asl-only flag, or an explicit `--asl`); that is a user-visible CLI spelling.

**2. AS-AUTHOR-FORMS-EXACT (closes most of G3). Size S.** The encodings asl gives without any
policy question: the eight missing mnemonics (`chk illegal link unlk reset rtr stop trapv`),
`.b` as a branch-size synonym for `.s`, `andi/ori/eori` to `ccr`/`sr` with the size implied,
`add/sub <ea>,An` emitted as `adda/suba` and `eor #imm` as `eori` (asl's own aliasing), a `.w`
suffix on a literal absolute (`$1234.w`), unquoted `binclude`, and refusing `swap.w`. *Sized
off*: 20 of the 31 over-refusals plus two probe forms, each with its asl bytes already in
`logs-isa.txt` from exit-0 runs, so every test has an oracle value and none needs a new rule.
*Risk*: low; all are new acceptances of shapes sigil refuses today, so no existing byte moves
(the corpora contain none of them, measured). Not OWNER, unless the `adda`/`eori` aliasing is
judged a policy question (the frontend today refuses them with a message telling the user to
write `adda`, which reads as deliberate: if it is a ruling, this item goes OWNER).

**3. AS-UNSIZED-DEFAULTS (the rest of G3). Size M. OWNER.** asl's defaults where the source
names no size: `.w` for sized operations (`move add addq cmp clr tst ext` in the sweep) and for
bare `ds`, and relaxation of unsized `Bcc`/`bra`/`bsr` (and `bra.l`, which asl emits as the
word form). The refusal text today says "Aeon pins branch width, no relaxation", which is
Aeon's policy spoken by the AS frontend, so lifting it is a behaviour change for the owner.
*Sized off*: the machinery exists (`Fragment::RelaxLadder`, used by `.emp`'s `jbra`, lowered by
`resolve_layout`, which the AS route already runs), so the frontend work is wiring; the cost is
parity. *Risk*: asl chooses widths by its pass loop and sigil by a monotonic fixpoint, and the
two can disagree; each accepted shape needs asl-probe byte checks, including forward and
backward targets at the `.s` reach boundary. No aeon byte can move (aeon's AS files cannot
contain a refused form).

**4. AS-DIAG-FATAL-FIRST (G4). Size S.** When a `fatal` or `error` directive fires, lead with it
and do not report forward references the stopped pass never reached as unresolved. *Sized off*:
563 lines against 1 on the S3K leg, and the two-line `fatal2.asm` reproducer. Diagnostic
ordering and wording, which the autonomy directive's just-do-it clause names. *Risk*: must not
hide a real unresolved symbol; the discriminator is whether the pass that raised it completed.

**5. AS-PERF-MEASURE (G5). Size S to measure; the fix is unsized until measured.** Phase timing
on the AS route (front-end passes, layout/relaxation, link, flatten and compression), reported
per corpus, then a sized follow-up. *Sized off*: 2.0x to 2.5x wall and 5x to 18x RSS against
asl, with no phase split available (no profiler installed here). This ranks fifth only because
it cannot be sized yet; for adoption it is the first thing people will measure.

**6. Later, not yet sized**: an AS listing for `-L` (G6; `sigil-link/src/listing.rs` already
writes the AS symbol-table section for aeon, the line listing does not exist); a Windows build
and timing (G8); the stale `-z` help text (G7, trivial, can ride with item 1).

**On G2 without item 1**: if the drop-in is not taken, the direct route still needs its own
answer to the S2 share file, and the three shapes the 2026-09-16 census listed remain (sigil
patches the immediate itself, sigil writes the share file, or sigil refuses when the hardcoded
value disagrees). Item 1 makes that question moot for build-script users; the direct route can
keep refusing `fixBugs` but still writes the `improved=true` ROM silently wrong.

## 9. What this run could not measure, and why

- **Where sigil's time goes.** No `perf`, `valgrind` or `samply` on the machine, no phase timing
  in the CLI, and adding either is a code change this parcel may not make.
- **A Windows build.** No `rustup`, no Windows target installed; the three `-sys` crates build C
  and would need a cross C toolchain.
- **Whether the ROMs run.** Byte identity to the stock ROM is the only bar used; nothing was run
  in an emulator (standing rule 1). None needed here: every claim is a byte compare against a
  stock build.
- **Other community disassemblies and hacks** (for example ones with their own macros or
  `-L`-consuming tools): only the three pinned corpora exist on this machine.
- **The drop-in's behaviour on errors and warnings.** Derived from `common.lua`, not run, since
  sigil cannot be dropped in.

## 10. Reproduction

Scratch lives in the uncommitted `scratch-r2/` of this parcel's worktree: `phase1.sh` (refs,
direct, drop-in, shim), `refpin2.sh`, `sweep.sh`, `skflips.sh`, `s2improved.sh`, `pfeas.sh` with
`prewrite.py` / `prewrite2.py`, `probes/run.sh` and `probes/isa_sweep.py`, `formcount.py`,
`timing.sh` with `tm.py`; each writes a `logs-*.txt` with an end marker. The first run's
scratch (`agent-a83b3b56407567d30/scratch/`) was read and copied, never written.

## 11. What in the brief turned out wrong or needs amending

- **"Cite every asl result by md5 `61e67256`" cannot hold for Sonic 2's whole ROM.** That build
  cannot assemble `s2.asm` (section 1). S2's reference ROM here, like the census before it, comes
  from S2's own `0dee1f98` build; every per-line value in this note (sections 4 and 5) comes from
  `61e67256` through `asl_run`.
- **The two S2 "candidate defects" are not candidates.** Both are acknowledged, adjudicated rows
  in `switch_matrix_sweep.py`'s `ACK_DISAGREE` (the share-file residual), and the nightly sweep
  reports them every night.
- **The first run's `unsized_count.py` zero was vacuous** (a `CPU Z80` line inside `sonic.asm`
  switches it off for the rest of the file); the corrected census agrees on zero but only now
  with a control that could have seen otherwise.
- **The brief frames `-xx -n -q -A -L -U -i .` as the flags to check.** The scripts also pass
  `-E` (the `.log` their failure path reads) and, for S2, `-c` (the share file behind G2); those
  two are the load-bearing ones, the rest are cosmetic to a drop-in.
- **"Stock source vs every build option" as a hypothesised gap is closed for S1/S2** by the
  nightly sweep, and this run found the S3K family in the same state; the gaps are the pipeline
  contract, author-written code, diagnostics and speed.
