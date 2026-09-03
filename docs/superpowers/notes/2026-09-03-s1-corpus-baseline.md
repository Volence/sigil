# The Sonic 1 corpus baseline — 9,739 diagnostics over a complete walk

Measurement parcel. No sigil source is changed by it.

## Provenance

| | |
|---|---|
| corpus | `s1disasm` at `f6ece657`, in a detached worktree — the live tree is never written |
| entry | `sonic.asm` (not `s1.asm`; the S2 naming does not carry) |
| command | `sigil sonic.asm`, run from the corpus root, no flags |
| sigil | built from master `5bd06567` into a target dir outside every shared checkout |
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, `-xx -n -q -A -L -U -E -i .` |

**The two `asl` binaries are not the same build.** S1 ships its own copy and it is
upstream AS — `Macro Assembler 1.42 Beta [Bld 212] (x86_64-unknown-linux)`, md5
`61e672562465725a8c102288a7da9098`. S2's is the flamewing fork — same version string,
`(x86_64-Linux)`, `(C) 2022-2024 flamewing`, md5 `0dee1f98e6480a4783d27ffd8b90896f`.
Every probe here used **S1's own binary**, and `-U` is on every invocation.

The freshness witness for the sigil binary is not its build date: the same binary run
on `s2disasm` `e45ebf3` reports **13,109**, reproducing today's S2 figure independently.

## The baseline

**9,739 diagnostics, every one severity `error`.** Zero warnings, zero notes. Every
line of output matches `file(line): error: …`; there is no summary line, no unclassified
remainder, and no output on stdout.

### Coverage — the whole corpus, not a prefix

The front end returns `Err`, `main.rs` renders and exits 1, and `sigil_link::link` is
never called. **But it does not abort early**, and a count read as a prefix measurement
would be wrong.

The witness is a marker probe rather than an argument. A unique bogus head
(`zqpmark0000`…`zqpmark0443`) was planted at the end of each of the **444** files in
`sonic.asm`'s include closure, and both assemblers were asked which ones they report.

| | markers reported |
|---|---|
| `asl` | 439 |
| `sigil` | 437 |
| both | 436 |

The gap is three markers, and it is **not** a coverage gap:

- The three are `sound/dac/{dpcm}/generated/{kick,snare,timpani}.inc`, reached under
  `CPU Z80`, where an unrecognized bare head is silently bound as a label rather than
  diagnosed (below). sigil does include those files; it eats the marker.
- The one marker `sigil` reports and `asl` does not is `zqpmark0000`, planted after
  `sonic.asm`'s `END`. **`asl` honours `END` and stops; sigil does not.**

So sigil's walk covers every file AS's does. Diagnostics land in 44 distinct files
spanning `MacroSetup.asm(7)` — the first include — to `s1.sounddriver.asm(2616)`, and
`s1.sounddriver.asm` is the last include in `sonic.asm` (line 5229 of 5237).

### One caveat on the number

`sound/dac/*/generated/*.inc` are build-generated and gitignored. A bare checkout lacks
them and reads **9,747** (4 `cannot include` plus 4 downstream `int(): could not
evaluate float expression`). 9,739 is the run with them regenerated, i.e. the corpus in
the state the real build assembles. Both captures are kept under `.s1probe/`.

## The decomposition — sums to 9,739 with no remainder

`counts` is diagnostics emitted; `sites` is distinct `file(line)`. The ratio is the
macro/`rept` multiplication, and it is what makes the ranking counter-intuitive.

| counts | sites | class |
|---|---|---|
| 8,939 | 161 | `X` is not a recognized 68000 mnemonic |
| 497 | 463 | unresolved symbol in operand |
| 161 | 1 | operand out of range -128..=255 |
| 36 | 1 | bad operand expression |
| 25 | 21 | unresolved long expression |
| 18 | 1 | unexpected character |
| 18 | 1 | instruction needs an explicit size suffix |
| 14 | 2 | bad word expression |
| 8 | 1 | unresolved rept count |
| 6 | 6 | case needs a string literal |
| 6 | 6 | bad immediate expression |
| 4 | 4 | trailing tokens in operand |
| 2 | 2 | unsupported form: ccr is not a general EA |
| 2 | 2 | switch needs a string expression |
| 1 | 1 | the corpus's own `error` text (`_Variables.asm(430)`) |
| 1 | 1 | unknown directive or mnemonic |
| 1 | 1 | org target precedes the current phase base |
| **9,739** | **675** | |

The 8,939 unrecognized heads are **15 names**:

| head | counts | sites |
|---|---|---|
| `eval` | 8,211 | 59 |
| `irpc` | 615 | 1 |
| `bchg` | 49 | 49 |
| `nextenum` | 20 | 20 |
| `irp` | 14 | 2 |
| `charset` | 9 | 9 |
| `enum` | 6 | 6 |
| `exg` | 4 | 4 |
| `endm` | 3 | 3 |
| `roxl` | 2 | 2 |
| `enumconf` | 2 | 2 |
| `warning` | 1 | 1 |
| `SMPS_RAM` | 1 | 1 |
| `page` | 1 | 1 |
| `listing` | 1 | 1 |

The 497 unresolved symbols are **86 distinct names**, and 496 of them are struct
members — `SMPS_Track.*` (280), `SMPS_RAM.*` (190), `v_snddriver_ram.*` (6) and the
`SMPS_*_TRACK_COUNT` equates derived from them. The single outlier is `usp`, the 68000
user stack pointer, which sigil does not know as a register.

## Ranked causes

Ordered by counts, with the source construct each one actually is. Every row was
confirmed by reading the cited line and reproducing it standalone against **both**
assemblers.

**1 — `eval`, 8,211 counts on 59 lines (84.3% of the baseline).**
`sound/_smps2asm_inc.asm`. AS's `EVAL` is a synonym for `SET`, and the corpus writes
both forms: `vcFeedback eval val` (`:792`) and `eval vcD1R1Unk,d1r1<<5` (`:803`).
sigil implements the two-operand `set NAME,value` form but not `eval` under either
spelling. One directive is five-sixths of the S1 baseline.

**2 — `irpc`, 615 counts on ONE line (6.3%).**
`Macros.asm(317)`: `irpc btn,"buttons"` inside the `demoinput` macro, invoked 615 times
across the demo scripts. The `switch`/`case` block inside it cascades into the 6
`case needs a string literal` and 2 `switch needs a string expression` rows, and
`Macros.asm(339)`'s `endm` reaches `dispatch` because the block never opened. With
`irp` (14) and those cascades this family is 632 counts on six lines.

**3 — AS `STRUCT` and struct-member addressing, ~499 counts.**
`s1.sounddriver.ram.asm:33` declares `SMPS_RAM struct DOTS`; `_Variables.asm:114`
instantiates it with `v_snddriver_ram: SMPS_RAM`. sigil does not recognize the
instantiation head (the lone `SMPS_RAM` mnemonic error) and does not bind the members,
so all 496 member references go unresolved. It also shifts every RAM variable declared
after line 114, which is why the corpus's **own** self-check fires at
`_Variables.asm(430)` — sigil emits the disassembly's `error "…"` text. That row is a
sigil arithmetic divergence reported by the corpus, not an independent defect.

**4 — `dc.b ALLARGS`, 161 counts on ONE line.**
`sound/_smps2asm_inc.asm(901)`, inside `smpsDcb`. Reported values are 34–35 million,
i.e. addresses reaching a byte directive. Downstream of rows 1 and 3; **re-measure
after they land rather than working it directly.**

**5 — real 68000 encodings absent from the table, 58 counts.**
`bchg` (49), `exg` (4), `roxl` (2), `move.w d6,ccr` (2 —
`_inc/Decompression/Kosinski Decompression.asm(32,50)`), `usp` (1). Not AS
compatibility at all; instruction coverage.

**6 — macro default argument values, 54 counts on two lines.**
`Macros.asm(12)`: `locVRAM: macro loc,controlport=(vdp_control_port).l`. When the
caller omits the parameter sigil substitutes nothing instead of the default. Proven
standalone: `m macro a,q=7` / `m 1` is exit 0 under `asl` and `bad byte expression`
under sigil. This produces the 18 `instruction needs an explicit size suffix` and the
36 `bad operand expression`.

**Its span is also wrong.** All 36 `bad operand expression` are reported at
`sonic.asm(1)` — line 1 of the root file, which has nothing to do with them. The
standalone repro reproduces the misattribution: the diagnostic points at line 1 while
the macro call is at line 6.

**7 — `enum` / `nextenum` / `enumconf` / `charset`, 37 counts.**
AS enumeration and character-set directives, mostly `sound/_smps2asm_inc.asm`.

**8 — `[count]value` repetition in a `dc` operand, 18 counts on ONE line. S1-only.**
`MacroSetup.asm(98)`: `dc.ATTRIBUTE [count]value`, the body of the corpus's `dcb`
macro. sigil says `unexpected character`. Verified against `asl` standalone:
`dc.b [4]$FF` is exit 0 under AS and an error under sigil. **`s2disasm` has zero `dcb.b`
call sites; S1 has 20.**

**9 — `switch`/`case` on integers, 8 counts.**
`sound/_smps2asm_inc.asm(63)`: `switch SonicDriverVer` / `case 1`. sigil's `switch`
handles string expressions only.

**10 — float literals in `irp`, 14 counts on two lines.**
`s1.sounddriver.asm(1796)`: `irp op, 15.39, 16.35, …` feeding `MakeFMFrequency(op)`.

**11 — multi-character literals as immediates, 6 counts.**
`_incObj/82, 83 SBZ Eggman Cutscene and Crumbling Floor.asm`: `move.w #"SW",…`.

**12 — the Z80 section, 6 counts.** `sound/z80.asm(9)` `!org 0` gives
`org target precedes the current phase base`; `(11)` `listing purecode` gives
`unknown directive or mnemonic purecode`; four `trailing tokens in operand` at
`(51,55,188,197)`. Small, but see the silent class below — this file is under-measured.

**13 — `warning` / `page` / `listing`, 3 counts.** `warning` is unimplemented while
`error` is.

**14 — `unresolved long expression`, 25 counts on 21 sites.** Downstream. Five are at
`_inc/Special Stage Mappings & VRAM Pointers.asm(10)`, whose
`__LABEL__: label (*-SS_MapIndex)/(4+2)+1` composite **works standalone** — the failures
are the operands, not the construct.

## The silent half

What was searched, not only what was found.

1. **Every output line was classified.** 0 of 9,739 lines fail to match
   `file(line): severity: …`. Nothing is being summarised away or suppressed.
2. **The marker probe** (444 files, both assemblers) — this is what found rows a and b
   below.
3. **A 28-construct probe** comparing sigil's exit code and emitted bytes against
   `asl`'s on the same input, covering every directive the corpus census shows S1 using.
4. **Link is never reached**, so nothing can be silently emitted wrong at this stage;
   the exit is loud.

Found:

**a. Under `CPU Z80`, an unrecognized bare head is silently bound as a label.**
The worst finding here. Standalone:

```
	cpu 68000
	dc.b $AA
	CPU Z80
	zqp_bogus_head        ← no diagnostic, no bytes, exit 0
	dc.b 2
	CPU 68000
	dc.b (zqp_bogus_head)&$FF   ← emits 01: it was bound to the PC
```

The same head on the 68000 path is a loud
`` `zqp_bogus_head` is not a recognized 68000 mnemonic ``. With operands it errors, but
misleadingly — naming the first *operand* rather than the head. This is a
silent-wrong-answer class, not a missing-feature class: an unimplemented Z80 mnemonic
becomes a label, emits nothing, and the ROM is quietly short. **It also means the S1
baseline under-counts `sound/z80.asm`** — six diagnostics there is a floor, not a
measurement.

**b. `END` is not honoured.** Text after `END` is still assembled.

**c. `message` is a silent no-op** and `\{…}` interpolation is not performed. The
`error` directive does print, but with the interpolation raw — visible verbatim in the
`_Variables.asm(430)` row.

Probed and found **correct**, i.e. not silent no-ops: `save`/`restore` (the CPU really
is restored — `4E 71` after `restore`), `phase`/`dephase`, `align`, `org`, `while`,
`padding`, `supmode`, `switch`/`elsecase` on strings, `shift`, `MOMCPU` (`00 06 80 00`
for the 68000), `ALLARGS`.

## Which of today's landed features S1 exercises

| feature | S1 | evidence |
|---|---|---|
| `shift` | **not at all** | zero head-position `shift` lines in the whole closure; `s2disasm` has 4 |
| `{INTLABEL}` / `__LABEL__` / `label` | **heavily** | `{INTLABEL}` in 14 files, `__LABEL__` in 27; the `specialStageData` composite assembles correctly standalone |
| macro-local scoping by written form | yes | `.rep`/`.val`/`.start`/`.end` locals throughout `Macros.asm`, `MacroSetup.asm` |
| single-pass argument substitution | yes | and it matches AS on the hard case: `m macro a,b` with `dc.b a,b` produces `dc.2` under **both** tools (AS error #1107) |
| `!` forced-builtin, macro-beats-builtin | **heavily** | 32 lines across `MacroSetup.asm`, `_Variables.asm`, `sound/z80.asm` — `!org`, `!oper`, `!dc.b`, `!ds.ATTRIBUTE` |
| `MOMCPU` / `TRUE` / `FALSE` | yes | `MOMCPU` in 2 files, `TRUE` 20 lines, `FALSE` 30; `MOMPASS` in 6 files |
| case folding | yes | `CPU Z80` capitalised at `sound/z80.asm(10)`, `EQU` in `sound/_smps2asm_inc.asm` |
| source locations | yes, and it is load-bearing | 675 distinct sites across 44 files; also **exposes a bug** — 36 diagnostics misattributed to `sonic.asm(1)` |
| processor spellings | partly | S1 uses only `68000` and `Z80`; it has no `z80undoc`, which is what forced the S2 parcel |

## What S1 uses that S2 never did

The naive guess is wrong and worth stating: **`eval` is not it.** Both corpora share
`sound/_smps2asm_inc.asm` and both contain exactly 71 `eval` lines. S2 reports 3,417
`eval` diagnostics of its own. The difference is depth of reach, not vocabulary.

Genuinely S1-only, from a head-token census of both closures:

- **`dcb.b` — 20 sites in S1, 0 in S2.** Routes through `MacroSetup.asm`'s `dcb` macro
  into `dc.ATTRIBUTE [count]value`. The AS feature it needs, `[count]value`, is
  cause 8 and appears nowhere in S2.
- **`equ` at 1,104 sites vs S2's 40.** S1 declares constants with `EQU`; S2 uses `=`.
- **`include` at 443 sites vs S2's 339, over a 444-file closure.** S1 is far more
  fragmented, which is why its diagnostics land in 44 files and S2's in 6.
- **A Z80 sub-assembly with `save` / `!org 0` / `CPU Z80` / `listing purecode`**
  (`sound/z80.asm`). S2's Z80 driver reaches sigil still on the 68000 because of
  `cpu z80undoc`; S1's really does switch, which is how the silent Z80 label class above
  became reachable at all. **This is the single most important structural difference and
  it is the one that hides defects rather than reporting them.**
- **The corpus's own `error`/`warning` self-checks on RAM layout** (`_Variables.asm`
  430, 486) — a free correctness oracle S2 does not offer.
- **Float literals in `irp`** (`s1.sounddriver.asm(1796)`).
- **`move.w Dn,ccr`** in the Kosinski decompressor.

And what S2 has that S1 does not, because it reorders the queue:

- **The nameless `+`/`-` local labels are S2's single largest class at 2,307 sites
  (d-22). S1 does not use them anywhere.** The largest known S1-independent gap in the
  project is irrelevant to Sonic 1.
- `shift` (4), `pushv`/`popv` (1 each), `shared` (1), `cpu z80undoc`, the 114
  `absolute address needs an explicit width` sites, `int()` on floats — all zero in S1.

## Whether it reaches link

**No.** `assemble_root_located` returns `Err`; the CLI renders and exits 1 before
`sigil_link::link` is called. Nothing is emitted.

The one place link *was* reached during this parcel is telling: an isolated
`struct … DOTS` reproduction fails with `error: unresolved symbol S.b for fixup in
section sec0 at offset 0` — a **link** diagnostic with no `file(line)`, a different
shape from the front-end row the corpus produces. Struct support has a second failure
mode waiting behind the first.

## The byte reference — it exists, and it is strong

Not merely comparable: the corpus's own gate is a retail-ROM identity check.

`build_tools/chkbitperfect.lua` hashes the built ROM against three hardcoded MD5s for
REV00, REV01 and REVXB. Run in this parcel's worktree at the default `Revision = 1`:

```
ROM is bit-perfect with REV01.
```

`s1built.bin` — md5 `09dadb5071eb35050067a32462e39c5f`, crc32 `afe05eee`, 524,288 bytes.
The reference is the retail cartridge, not a previous build, so it cannot drift.

`asl` assembles the corpus with **exit 0 and no log file at all** — zero errors, zero
warnings. There is no ambiguity about what the target output is.

Two further gates ship with the corpus: `Revision = 0` and `Revision = 2` select REV00
and REVXB, each with its own hash, so three independent byte targets exist from one
source tree. None were built here.

## Booked, not done

- The Z80 silent-label class (silent half, row a) — this is a soundness defect and it
  is not measured by the 9,739.
- `END` not honoured.
- The `sonic.asm(1)` span misattribution.
- Re-measure cause 4 (161 counts) only after causes 1 and 3 land.
- `_inc/Special Stage Mappings & VRAM Pointers.asm(10)`'s five `unresolved long
  expression` — the construct works standalone, so the operands need their own look.
