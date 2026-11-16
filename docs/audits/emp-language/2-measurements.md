---
kind: audit-stage
system: emp-language
stage: "2 Measurement"
agent: Opus
status: done
description: "16,046 instruction lines (12,960 68k + 3,086 Z80) carry 48,841 comment lines (3.0 per instruction); only 2.2% of body lines use any construct above an instruction; 931 prose register headers restate 605 checked contracts; 349 hand-counted cycle comments against 1 @budget. GCC 13.3 -O2 m68k (MEASURED codegen) costs 1.3x to 1.8x the emp cycles (ESTIMATED by hand count) on all three test routines; a functional tier with register contracts and in-place slot mutation is estimated at 1.0x to 1.25x."
---
# EMP language: measurement

> [!verdict] Result
> - **The language is used as an assembler with a contract checker bolted on.** Of the 16,411 proc and template body lines that do work, 16,046 are raw instructions and **365 (2.2%) use any higher construct** (template calls, `with`/`invoke`, `assert`/`raise_*`). Game logic (Sonic 4 player and objects) is the same: 2.3% (MEASURED, §2).
> - **Comments do the work the language does not.** 48,841 comment lines against 16,046 instructions, **3.0 comment lines per instruction**. 931 comment lines restate register contracts in prose (`In:`/`Out:`/`Clobbers:`) beside 605 machine-checked proc headers. 349 comment lines hold hand-counted cycle sums, against **1** `@budget` and 28 `cycles()` calls (MEASURED, §4).
> - **The functional style already exists, at build time.** 4,499 comptime-fn code lines, 48% of them in two DSL files (`raster_dsl.emp`, `scene_dsl.emp`). The palette transform exists twice, as a 5-line comptime model and a 222-byte asm proc, joined only by an external Python gate (§5.3).
> - **Codegen experiment (§5).** A real m68k GCC (13.3, Ubuntu cross package, run in a throwaway container) compiled C versions of three aeon routines. GCC's output (MEASURED bytes, ESTIMATED cycles) costs **1.46x to 1.83x** the emp cycles on the object routines, **1.73x** on the sprite piece loop (1.77x for C written in the asm's own style), and **1.29x** on the palette table builder. It is also **1.4x to 2.5x** larger. The causes are specific: stack-passed arguments, callee saves, load-modify-store instead of memory-operand updates, 32-bit int promotion, and no `(An)+` streaming.
> - **A hypothetical functional tier** that keeps the existing register contracts as its calling convention and guarantees in-place slot mutation is ESTIMATED at **1.0x to 1.1x** on object logic and **1.07x to 1.25x** on the hot loop. A naive one, with stack calls and load-modify-store, lands at GCC's numbers.
> - **Shipped constructs a run-and-gun boss or shmup script would reach for are unused.** `script` 0, `dispatch` 0, runtime `enum` 0, `bitfield` 0. State machines are spelled as 26 `code_addr` pointer writes in `games/` (MEASURED).

## 0. Provenance

| Item | Value |
|---|---|
| Trees | aeon `936439fa` (2026-09-28), sigil `7630ba0d`. Same as stage 0 |
| Corpus | 204 `.emp` under `aeon/engine` + `aeon/games`: 190 hand-written (86,080 lines), 14 under `generated/` (1,029 lines). My line totals reconcile exactly with stage 0 (48,841 comment-only lines, 86,080 hand lines) |
| ROMs for sizes and disassembly | `aeon/s4.bin` + `s4.lst` (release, `DIGEST-SHAPE debug=0`, sigil `7630ba0d`, tree `clean-sources`), and `aeon/s4.debug.bin` + `s4.debug.lst` (`debug=1`, same sigil). Copied read-only to the scratchpad and confirmed identical with `cmp` |
| Disassembler / compiler | `m68k-linux-gnu-objdump` and `m68k-linux-gnu-gcc (Ubuntu 13.3.0-6ubuntu2~24.04) 13.3.0`, installed with apt inside a throwaway `ubuntu:24.04` container (`--rm`-style; removed at the end). No m68k compiler is installed on the host: `m68k-elf-gcc`, `m68k-linux-gnu-gcc` and `vbcc` are absent, and the host clang 23.1.4 has no M68k target ("No available targets are compatible with triple m68k-unknown-unknown-elf") |
| Scripts | `measure.py`, `themes.py` (appendix), under `<scratch>/audit-builds/emp/scripts/`; outputs under `<scratch>/audit-builds/emp/out/`. C sources and GCC output under `<scratch>/audit-builds/emp/gcc/` |
| Machine | AMD Ryzen 7 9800X3D. The scripts run in under 2 s wall clock, so there are no host-timing figures in this report |
| Cycle method | 68000 cycle counts are **ESTIMATED by hand count** over the MEASURED disassembly. Each instruction is charged its time from the PUBLISHED M68000 User's Manual timing tables (Motorola M68000UM §8). Wait states and VDP/DMA stalls are not included. Sigil's own 68000 cost model could not be used: `@budget` refuses loops and calls (`cycle_budget.rs:28-40`), and every routine measured here has one or the other. `cycles(.a,.b)` is Z80-only (`eval/call.rs:84`) |

`<scratch>` = `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad`.

## 1. Lines by category, per system

Command: `python3 -I scripts/measure.py /home/volence/sonic_hacks/aeon out`. Each line is classed once.
- **blank** and **comment** lines: nothing outside comments. Strings are stripped first.
- Inside a `proc`, `script` or `section` body, or an `asm {}` template:
  - **instr**: a 68000 or Z80 mnemonic (by the module's `cpu:`), or `jbra`, `jbsr`, `mul_const`, `mul_bounded`;
  - **label**: a label alone on its line;
  - **tmpl**: a call to a comptime template, e.g. `abs_w(d1)`;
  - **with/inv**: a `with` block or an `invoke`;
  - **diag**: `assert`, `raise_error` or `raise_exception`;
  - **struct**: braces, comptime `if`/`for`/`let`.
- **hdr**: proc and script headers and attributes.
- **guard**: `ensure` statements. **g-cont**: their continuation lines.
- **ct**: comptime fn bodies outside `asm {}`.
- **decl**: const, equ, struct, vars and the like. **data**: `data`/`table`/`offsets` items and `dc.*`.
- **imp**: `use`/`module`. **cdecl**: context, interface, implement and type declarations.

| System | Lines | blank | comment | instr | label | tmpl | with/inv | diag | struct | hdr | cdecl | guard | g-cont | ct | decl | data | imp |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| engine/level | 19,573 | 732 | 10,590 | 5,067 | 739 | 40 | 7 | 43 | 305 | 80 | 0 | 234 | 123 | 1,189 | 319 | 3 | 95 |
| engine/sound | 10,202 | 463 | 4,597 | 3,211 | 321 | 5 | 10 | 5 | 265 | 213 | 0 | 168 | 67 | 11 | 612 | 196 | 35 |
| games/sonic4/test | 9,235 | 436 | 5,762 | 865 | 108 | 16 | 2 | 7 | 79 | 17 | 0 | 193 | 86 | 105 | 1,107 | 297 | 155 |
| games/sonic4/data | 8,807 | 377 | 6,147 | 32 | 4 | 0 | 0 | 0 | 13 | 5 | 0 | 341 | 152 | 496 | 671 | 441 | 128 |
| games/sonic4/player | 8,446 | 367 | 4,587 | 2,178 | 295 | 46 | 0 | 3 | 145 | 101 | 4 | 95 | 82 | 106 | 159 | 146 | 132 |
| engine/effects | 7,805 | 242 | 5,034 | 737 | 111 | 15 | 1 | 0 | 37 | 24 | 0 | 222 | 67 | 1,103 | 189 | 2 | 21 |
| engine/objects | 6,786 | 533 | 3,049 | 2,112 | 272 | 47 | 6 | 32 | 174 | 87 | 3 | 42 | 12 | 227 | 105 | 6 | 79 |
| engine/system | 5,425 | 304 | 3,168 | 873 | 120 | 29 | 12 | 4 | 123 | 45 | 15 | 68 | 45 | 22 | 432 | 114 | 51 |
| engine/(root files) | 3,233 | 160 | 2,253 | 7 | 1 | 0 | 0 | 0 | 6 | 0 | 26 | 24 | 9 | 95 | 641 | 0 | 11 |
| games/sonic4/objects | 3,114 | 245 | 1,681 | 549 | 61 | 13 | 0 | 2 | 42 | 35 | 0 | 38 | 38 | 49 | 177 | 91 | 93 |
| games/sonic4/config | 1,417 | 74 | 1,003 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 14 | 46 | 10 | 19 | 218 | 18 | 15 |
| engine/debug | 890 | 50 | 386 | 148 | 33 | 0 | 1 | 17 | 22 | 30 | 0 | 4 | 4 | 0 | 53 | 133 | 9 |
| engine/compression | 569 | 30 | 291 | 185 | 38 | 0 | 0 | 2 | 14 | 5 | 0 | 0 | 0 | 0 | 1 | 0 | 3 |
| games/demo (all) | 385 | 41 | 194 | 25 | 1 | 0 | 0 | 0 | 3 | 3 | 6 | 11 | 0 | 0 | 36 | 41 | 24 |
| games/sonic4/debug | 193 | 7 | 99 | 57 | 7 | 0 | 0 | 0 | 2 | 2 | 0 | 0 | 0 | 0 | 1 | 10 | 8 |
| **Hand total** | **86,080** | 4,061 | **48,841** | **16,046** | 2,111 | 211 | 39 | 115 | 1,230 | 647 | 68 | **1,486** | 695 | 3,422 | 4,721 | 1,498 | 859 |
| generated/ (separate) | 1,029 | 77 | 523 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 15 | 6 | 55 | 227 | 97 | 29 |

There are also 30 unclassified body lines (MEASURED; all in `engine/level` 7 and `engine/sound` 23, mostly multi-line operands). The categories sum to the total exactly.

> [!finding] Instructions are 18.6% of the hand-written lines; comments are 56.7%
> Instruction lines are 16,046 of 86,080 (18.6%, MEASURED). Comment-only lines are 48,841 (56.7%). Stage 0's "60% comment share" uses comment ÷ (comment + code), the same data on a different denominator.
>
> Data is small in the source (1,498 lines). Level, art and sound data enter as binary through `embed()` (107 uses, stage 0), not as `.emp` data.

## 2. Instruction mix and high-level feature use

### 2a. Instruction mix (68000, 12,960 lines including pseudo-ops)

| Class | Lines | Share | Notes |
|---|---|---|---|
| `move`/`movea`/`moveq` | 4,604 | 35.5% | `move` 3,561, `moveq` 719, `movea` 324 |
| conditional branch `b<cc>` | 1,734 | 13.4% | `beq` 663, `bne` 369 |
| add/sub/neg family | 1,464 | 11.3% | |
| `cmp`/`tst` family | 1,080 | 8.3% | |
| `jbra` + `jbsr` (relaxing pseudo-ops) | 979 | 7.6% | 404 + 575 |
| `lea`/`pea` | 613 | 4.7% | |
| shift/rotate/`swap` | 483 | 3.7% | |
| `rts`/`rte`/`rtr` | 447 | 3.4% | |
| logic (`and`/`or`/`eor`/`not`, immediates) | 422 | 3.3% | |
| `clr` | 308 | 2.4% | |
| bit ops | 278 | 2.1% | |
| `db<cc>` | 214 | 1.7% | `dbf` 211 |
| `movem` | 100 | 0.8% | |
| `bra` (raw) | 53 | 0.4% | |
| `ext`/`exg` | 53 | 0.4% | |
| `mul_const`/`mul_bounded` (compiler-chosen sequence) | 32 | 0.2% | |
| `s<cc>` | 30 | 0.2% | |
| raw `bsr`/`jsr` + `jmp` | 29 | 0.2% | |
| `nop` 14, `movep` 13, `mul*`/`div*` 10 | 37 | 0.3% | |

Size suffixes on 68000 instructions (MEASURED): `.w` 4,829, `.b` 1,612, `.l` 1,337, `.s` 13, none 4,158 (unsized branches, `moveq`, `lea` and similar).

**Z80, 3,086 lines (MEASURED):** `ld` 1,305 (42%), `jr` 301, `call` 231, `ret` 155, `add` 141, `cp` 117, `inc` 114, `or` 111, `pop` 99, `jp` 97, `push` 89, `bit` 67; 34 distinct mnemonics. All of them are in `engine/sound` plus `z80_init.emp`.

**Where the 68000 instructions are (MEASURED):**

| Location | 68k lines |
|---|---|
| engine/level | 4,650 |
| games/sonic4/player | 1,842 |
| engine/objects | 1,782 |
| engine/system | 758 |
| engine/effects | 679 |
| games/sonic4/test | 642 |
| inside `asm {}` templates | 589 |
| games/sonic4/objects | 481 |
| engine/compression | 174 |
| engine/debug | 125 |
| engine/sound (68k side) | 122 |

### 2b. How often the higher constructs are used

Rates are per 1,000 instruction lines (16,046). Counts are stage 0's `features.tsv`, re-run unchanged, or this stage's `measure.py` where marked.

| Construct | Uses | Per 1k instr | Note |
|---|---|---|---|
| `clobbers(...)` | 611 | 38 | on 596 of 605 proc headers |
| `preserves(...)` | 183 | 11 | |
| `out(...)` (incl. conditional and carry) | 76 | 4.7 | |
| `falls_into` | 42 | 2.6 | |
| `requires`/`grants` | 10 / 1 | 0.7 | |
| `ensure`/`ensure_fatal` | 1,486 | 93 | 1,191 at item position, 310 inside fn or proc bodies (MEASURED, `measure.py`; the 1,501 includes 15 generated) |
| `assert.<w>` / `raise_*` | 66 / 49 | 7.2 | |
| template calls in proc bodies | 211 (60 distinct templates) | 13 | `measure.py`; top: `pixels_to_coord` 26, `abs_w` 21, `vdp_comm_reg` 9, `dma_send_entry` 8 |
| `asm {}` templates defined | 103 | 6.4 | |
| `with` / `invoke` | 31 / 8 | 2.4 | |
| `struct` / `vars` / `region` | 64 / 14 / 8 | | |
| `comptime fn` | 384 (4,499 code lines) | | 2,138 lines (48%) in `raster_dsl.emp` + `scene_dsl.emp` (MEASURED) |
| `match` / `comptime enum` | 65 / 13 | | 4 files |
| lambdas / `\|>` | 15 / 7 | | |
| `cycles()` / `@budget` / `@cycles_exact` | 28 / **1** / **0** | | |
| `script` / `dispatch` / runtime `enum` / `bitfield` / `table` | **0 / 0 / 0 / 0 / 1** | | |

**Higher-construct share inside bodies, per system** (tmpl + with/inv + diag) ÷ (those + instr), MEASURED:

| System | instr | higher constructs | share | guards per 100 instr | comment lines per instr |
|---|---|---|---|---|---|
| engine/level | 5,067 | 90 | 1.7% | 4.6 | 2.09 |
| engine/sound | 3,211 | 20 | 0.6% | 5.2 | 1.43 |
| games/sonic4/player | 2,178 | 49 | 2.2% | 4.4 | 2.11 |
| engine/objects | 2,112 | 85 | 3.9% | 2.0 | 1.44 |
| engine/system | 873 | 45 | 4.9% | 7.8 | 3.63 |
| games/sonic4/test | 865 | 25 | 2.8% | 22.3 | 6.66 |
| engine/effects | 737 | 16 | 2.1% | 30.1 | 6.83 |
| games/sonic4/objects | 549 | 15 | 2.7% | 6.9 | 3.06 |
| engine/compression | 185 | 2 | 1.1% | 0 | 1.57 |
| **All hand-written** | **16,046** | **365** | **2.2%** | **9.3** | **3.04** |

### 2c. Contract and guard lines against instructions (MEASURED)

| Ratio | Value |
|---|---|
| Proc-header lines (contracts + attributes) ÷ instruction lines | 647 / 16,046 = **4.0%** |
| Contract clauses ÷ proc headers | 919 clauses (`clobbers` 605, `preserves` 183, `out` 76, `falls_into` 42, `requires` 10, `inout` 6, `grants` 1; header lines only) ÷ 605 = **1.5 per proc** |
| `ensure` statements ÷ instruction lines | 1,486 / 16,046 = **1 per 10.8 instructions**; with continuation lines, 2,181 guard lines = 13.6% of instruction lines |
| Guards in data and test files | `games/sonic4/data` 341 guards against 32 instructions; `games/sonic4/test` 193 against 865 (stage 0: 352 of all guards pin a name to a literal) |
| Mean instructions per proc | 16,046 / 605 = 26.5 (template bodies included) |

## 3. Game-logic shape (for the run-and-gun and shmup workloads)

| Measure | Value (MEASURED) |
|---|---|
| State transitions spelled as `move.w #X - ObjCodeBase, code_addr(aN)` in `games/` | 26 |
| `routine`-byte state machines | 0 |
| PC-relative computed jump tables (`jmp …(pc,dN)`) | 5 (`s4lz.emp` 2, `player_sensors.emp` 2, `animate.emp` 1) |
| `script` (coroutine with `yield`/`wait_frames`) | 0 uses (shipped, `lower/script.rs`) |
| `dispatch` | 0 uses (shipped) |
| Higher constructs in Sonic 4 player + object bodies | 64 of 2,791 body work lines = 2.3% |

> [!note] What this means for the named workloads
> A Gunstar or Alien Soldier style multi-part boss is a parent, N parts, and a phase state machine with many projectile spawns. A Thunder Force IV enemy script is a timed sequence of waits and moves. Today both would be written the way `TestParent_Main` and `TestChildPart_Main` are (§5.1): hand-held timers in `vars` overlays, state as a code pointer, phases as branches. The two constructs that target these shapes, `script` and `dispatch`, have no consumer. Nothing in the corpus shows whether they work at scale. Stage 3 and 4 reviewers should port one on paper.

## 4. Workaround and comment-borne patterns

### 4a. The searches the brief named

Command: `measure.py`, regexes over comment text only (hand-written files):

| Pattern (case-insensitive) | Hits | What the hits actually are (read) |
|---|---|---|
| `workaround` | 7 | 5 say "not a workaround"; 2 are history |
| `can't` | 49 | Almost all are domain invariants ("so the Z80 can't read …"). **4 are language gaps:** link-time immediate plus relaxable destination cannot share one instruction (`core.emp:95`, `boot.emp:313`, `:325`, `ojz_scroll_test.emp:979`), and 1 is the absolute-symbol-plus-index lowering gap (`replay.emp:240`) |
| `cannot express` / `not expressible` / `no way to` | 21 | Mostly limits of aeon's own data formats. Language ones: unions cannot be byte-sharing (`player_common.emp:133`, `:207`); bus state not expressible (`z80_bus.emp:141`); `cycles()` cannot express a span (`sound_fm.emp:201`) |
| `cannot` (any) | 615 | Too broad to read in full; sampled through the theme pass below |
| `hand-*` | 165 | `hand-authored` 23, `hand-spelled` 20, `hand-derived` 17, `hand-typed` 13, `hand-built` 8, `hand-rolled` 6, `hand-maintained` 5, `hand-unrolled` 4 |
| `pinned` / `pin` | 519 | Mostly "this ensure pins X". 20 are `widths pinned` notes (stage 0) |
| `until sigil` / `TODO sigil` | **0** | |
| `BLOCKED` | 1 | `scene_dsl.emp:2624` (an aeon task, not a language gap) |
| `TODO`/`FIXME`/`XXX` | 21 | 16 are `(xxx).W` addressing notation; **5 are demo-template TODOs** ("your game starts here") |
| `limitation` / "sigil gap" / "sigil cannot" | 12 | `page_cache.emp:591`, `:709` "abs+indexed in one op is a sigil gap"; `ram.emp:1510` "sigil cannot yet emit movem to a symbolic absolute" |

### 4b. Theme pass

Command: `python3 -I scripts/themes.py /home/volence/sonic_hacks/aeon out/themes`. Each row counts comment lines matching a theme regex (appendix). The hit lists, with `file:line`, are in `out/themes/T*.txt`.

| # | Theme | Comment lines | Files | Top files |
|---|---|---|---|---|
| T9 | **Register contract restated in prose** (`// In:`, `Out:`, `Clobbers:`, `Preserves:`) | **931** | 69 | `entity_window.emp` 77, `player_common.emp` 59, `ojz_scroll_test.emp` 48 |
| T10 | Dated history | 1,033 | 108 | `parallax.emp` 113 |
| T5 | AS-port residue words (`AS`, `.asm`, byte-identical, ported) | 437 | 96 | `player_common.emp` 31, `test_solid.emp` 28 |
| T8 | **Hand-counted cycles** ("12 cycles", "8 + 4 =") | **349** | 46 | `raster_dsl.emp` 104, `raster.emp` 55, `parallax.emp` 44 |
| T6 | **Protocol held in a comment** (MUST, NEVER, ordering) | 241 | 67 | `raster.emp`, `parallax.emp`, `scene_dsl.emp` 14 each |
| T3 | **Value kept in sync by hand** (hand-spelled, must match, mirrors) | 145 | 64 | `raster_dsl.emp` 10, `tile_cache.emp` 9, `z80_bus.emp` 8 |
| T7 | Union / overlay / byte sharing | 135 | 44 | `player_common.emp` 17, `scene_dsl.emp` 14 |
| T4 | **The language or checker cannot see or say it** | 61 | 37 | `scene_dsl.emp` 6, `ram.emp` 6 |
| T1 | Width or size pinned by hand (explicit `.w`, imm-link) | 39 | 13 | `vblank.emp` 9, `boot.emp` 8 |
| T2 | Addressing or lowering gap named | 9 | 4 | `core.emp` 4, `page_cache.emp` 2 |

**The T4 lines, read and grouped** (61 lines; about 30 are language or toolchain gaps and the rest are domain format limits):

| Gap | Examples |
|---|---|
| Import graph forces literal mirrors (`ram.emp` cannot import a code module's constants) | `ram.emp:53`, `:898`; `constants.emp:344`; `games/sonic4/config/constants.emp:63`; `bg_anim.emp:82` |
| Unions cannot share bytes | `player_common.emp:133`, `:207` |
| Checker models with holes | `falls_into` not a closure edge (`s4lz.emp:108`); no model of `rte` (`irq.emp:42`); `ix` lost through computed dispatch (`sound_sequencer.emp:1194`); Z80 modules skipped by the clobber closure (`z80_init.emp:98`); bus state (`z80_bus.emp:141`); two contracts disagreed with no check (`collision.emp:134`, `entity_window.emp:1241`); advisory text lint only (`buffers.emp:238`) |
| Encoding and lowering | abs-symbol + index (`replay.emp:240`, `page_cache.emp:591`, `:709`); `movem` to a symbolic absolute (`ram.emp:1510`); link immediate + relaxable destination (4 sites, §4a) |
| Language surface | no line continuation (`scene_dsl.emp:3108`); no `as` alias on import (`scene_equiv_proof.emp:141`); no array of computed length (`poison_scene_capacity.emp:35`); no dead-symbol elimination (`ojz_effects.emp:1542`) |
| Comptime cannot see the asm | `palette.emp:828`, `palette_dsl.emp:53` (the model and the code are two programs, see §5.3) |

> [!finding] The top workaround patterns, ranked by volume
> 1. **Prose contracts duplicating checked ones.** 931 lines, against 605 machine-checked headers.
> 2. **Hand cycle arithmetic in comments.** 349 lines, against 1 `@budget`.
>
> Both exist because the checked forms do not reach far enough. `@budget` refuses any loop or call. The contract surface does not carry an argument's meaning ("d0.l = x_pos 16.16").
>
> 3. **Rules held only in comments.** 241 MUST/NEVER/ordering lines.
> 4. **Values kept equal by hand.** 145 lines, about 5 of them forced by the import graph.
> 5. **Unions/overlays explained in prose.** 135 lines.
>
> Encoding gaps (T1 + T2, 48 lines) are small and fixable inside the current design.

## 5. Codegen experiment

Three real aeon routines, one per kind. The emp sizes are label differences in the listings, confirmed by disassembling the ROM bytes. GCC's code comes from C written to the same data layout. GCC sizes are MEASURED with `nm -S` on `-O2 -m68000 -fomit-frame-pointer` objects (`-Os` given too). All cycle counts are ESTIMATED by hand from the PUBLISHED M68000UM tables, on the common path, excluding callees.

### 5.0 Summary

| Routine | Kind | emp bytes (MEASURED) | emp cycles (ESTIMATED) | GCC -O2 bytes (MEASURED) | GCC cycles (ESTIMATED) | GCC ÷ emp, cycles | Functional tier, ESTIMATED |
|---|---|---|---|---|---|---|---|
| `TestParent_Main` (DEBUG ROM) | object state machine (timer, 2 states, delete) | 72 | 118 | 106 (`-Os` 98) | 172, plus about 40 at the caller for the stack argument | **1.46x** (1.80x with the caller) | 118–130 (1.0–1.1x) good; ~170 naive |
| `TestChildPart_Main` (DEBUG ROM) | multi-part boss child (orbit around parent) | 52 | 156 (+ `GetSineCosine`) | 98 (`-Os` 98) | 286 (+ callee) | **1.83x** | 156–170 (1.0–1.1x) |
| `Emit_ObjectPieces`, unflipped loop (release) | hot inner loop, per sprite piece | 38-byte loop (260 for all 4 flip variants) | **112 per piece** | 104 (`-Os` 98); "expert C" 122 | 194 per piece; expert C 198 | **1.73x** (1.77x) | 120–140 per piece (1.07–1.25x) |
| `Palette_DeriveVariant` (release) | table builder (3 × 8-entry LUTs) + 48-word apply | 222 | ~9,050 (3 lines, shift 0) | 352 (`-Os` 290) | ~11,650 | **1.29x** | ~9,100–9,600 (1.0–1.06x), from **one** source |

GCC's per-call overhead also includes the m68k SysV convention. Arguments arrive on the stack and callee-saved registers go through `movem`. The piece routine pays about 210 cycles per call in its prologue and epilogue: `movem.l` of 5 registers is 48, six stack loads about 90, the restore 52, and `rts` 16. emp passes register arguments under a `clobbers` contract and pays nothing for this.

### 5.1 Object / state-machine routine: `TestParent_Main` and `TestChildPart_Main`

Source `games/sonic4/objects/test_parent.emp:101-214`. These exist only in the DEBUG ROM (`s4.debug.lst:2775-2783`): `TestChildPart_Main` at `$1323E` (52 bytes), `TestParent_Main` at `$132D6` (72 bytes).

The emp code updates memory in place (`subq.w #1, life_timer(a0)` = 16 cycles) and tail-jumps (`jmp Draw_Sprite.w`, 10).

GCC 13.3's code, MEASURED (`obj.O2.s`):
- **Argument from the stack:** `move.l 4(%sp),%a0` (16).
- **Load-modify-store for every counter:** `move.w 48(%a0),%d0; subq.w #1,%d0; move.w %d0,48(%a0)` = 28 against 16.
- **Re-stores the argument before each tail call:** `move.l %a0,4(%sp)` (16).
- **In the child:** saves and restores `a2`/`a3` (48), zero-extends the angle to a long (`and.l #255`, 16), pushes it (12), pops it (8), and splits the `SinCos` struct return with `swap`/`ext.l`.

> [!note] The hypothetical functional tier on this routine
> The sketch in `SIGIL_IDEAS_FUNCTIONAL_TIER.md:103-113` (match → routine dispatch, arms → direct calls, **in-place slot mutation as a hard rule**) plus two more assumptions:
> 1. the existing `clobbers`/`out` contracts are the calling convention (register arguments, no saves);
> 2. a peephole that folds `x = x - 1` on a slot field into `subq.w #1, d16(a0)`.
>
> With both, the instruction stream is essentially what is hand-written today: 118–130 cycles. Without (2), the counters cost +12 each, about 150. Without (1) as well, it is GCC.
>
> The state type (`x_dir`, `life_timer`, `swing_phase`) becomes a sum type, and the 26 hand-written `code_addr` transitions in `games/` become constructor changes.

### 5.2 Hot inner loop: `Emit_ObjectPieces`, the unflipped variant

Source `engine/objects/sprites.emp:646-868` (the `emit_piece_loop` template with `y_term`/`size_link`/`tile_term`/`x_term` splices). Release ROM `$33FA-$3424` (`s4.lst:649-652`).

Per piece, 16 instructions, MEASURED disassembly. Cycles ESTIMATED:

```
move.w (a3)+,d0 8 | add.w d3,d0 4 | move.w d0,(a4)+ 8          ; Y
move.w (a3)+,d1 8 | addq.w #1,d5 4 | move.b d5,d1 4 | move.w d1,(a4)+ 8   ; size|link (pad-byte trick)
move.w (a3)+,d0 8 | add.w d6,d0 4 | move.w d0,(a4)+ 8          ; tile
move.w (a3)+,d0 8 | add.w d2,d0 4 | bne.s 10 | move.w d0,(a4)+ 8   ; X (0 -> 1 guard)
cmpi.b #80,d5 8 | dbeq d4 10                                    ; cap test fused with the count
= 112 cycles per piece
```

GCC 13.3, MEASURED (`pieces.O2.s`; `pieces2.O2.s` is the "expert C" version with the same pad-byte trick and pointer post-increment):
- **No `(An)+` addressing.** It uses `d16(An)` and then `addq.l #8,An` twice, +16 per piece and +4 per displacement access.
- **Separate tests for count and cap.** `cmp.w`/`jeq` at the top plus `cmp.b`/`jne` at the bottom, instead of one `dbeq`.
- **The natural C rebuilds the size|link word with `lsl.w #8` (22).** The expert C instead masks with `and.w #-256`/`and.w #255`/`or.w` (20), **and emits a redundant store** (`move.w %d1,-2(%a0)` twice).
- Result: **194 and 198 cycles per piece.** Writing the C closer to the asm did not help GCC.

> [!note] Functional tier on the hot loop
> A `map`/`for` over a counted stream lowers well if the tier:
> - recognises the sequential read and write as `(An)+`;
> - keeps the counter and the cap in registers;
> - fuses "count or cap" into `dbeq`, a pattern-specific rule.
>
> Missing only the `dbeq` fusion costs +4 to +8 per piece. Missing the pad-byte layout trick costs +12 to +20, and that trick is a data-layout fact a compiler can only use if the type says the pad byte is the link slot.
>
> **Estimate: 120–140 cycles per piece (1.07–1.25x).** At the 80-sprite cap that is +640 to +2,240 cycles per frame against 8,960 today (ESTIMATED). An allocating functional lowering (intermediate lists) is not viable at all on 64 KB of RAM. That is a hard rule of the same kind as the in-place-slot rule.

### 5.3 Table / data builder: `Palette_DeriveVariant`

Source `engine/effects/palette.emp:212-241` (the `derive_table` template) and `:806-869`. Release ROM `$7C82-$7D60` = 222 bytes (`s4.lst:1667-1679`).

- **Table phase:** 3 channels × 8 entries. A typical entry is 72 + `lsl` cycles: R 80, G 88, B 96. About 2,110 cycles plus about 100 of setup.
- **Apply phase:** 136 cycles per colour word: 3 masked indexed lookups OR-ed together, `lsr.w #8` costing 22. 48 words = 6,528.
- Line overhead about 90 per line, prologue and epilogue about 52.
- **Total about 9,050 cycles** for 3 covered lines at shift 0 (ESTIMATED). The file's own measured "before" figure for the old per-word derive is 18,946 (`palette.emp:822`).

GCC 13.3, MEASURED (`derive.O2.s`):
- **The table phase is about equal.** GCC fused the three channel loops into one, about 2,064 cycles.
- **The apply loop is 189 cycles per word, 1.39x emp.**
  - C's integer promotion turns the masks into `and.l` with `moveq #7` and `add.l` doubling.
  - It shifts by 9 with `lsr.w %d4` (24).
  - It does not find the asm's layout trick: `& $000E` is already a word offset, so no doubling is needed.
- **Prologue:** a 9-register `movem` (80 + 84 on exit).
- **Total about 11,650 cycles, 1.29x; 352 bytes against 222.**

> [!finding] This routine is already written twice
> `palette_dsl.emp:61-76` holds a 5-line functional model (`clamp07`, `variant_channel`, `variant_word`) with three `ensure` vectors. `palette.emp:806-869` holds the 222-byte asm. The header says plainly that the ensures "cannot see this asm — sigil does not execute 68000" (`palette.emp:828`). The two are tied together only by `tools/palette_variant_gate.py`, run under the emulator.
>
> A compiled tier that lowers `variant_word` per word would cost about 200 cycles per word, about 9,600 in total (1.06x). Choosing the LUT strategy, which is still the programmer's call, gives about 9,100. **Either way, one source replaces two, and the Python gate becomes unnecessary.** This is the clearest measured case for a compiled tier in the corpus. It is also an example of the T3 theme (values kept in sync by hand) at routine scale.

### 5.4 What was not run

| Item | Status | Why |
|---|---|---|
| `m68k-elf-gcc` on the host | **BLOCKED** | Not installed; host clang has no M68k backend. Substituted with `m68k-linux-gnu-gcc` 13.3 in a throwaway container. For `-m68000 -O2` code generation the ELF and Linux targets differ only in ABI details such as `a0`/`d0` pointer returns. Not verified further |
| Older GCC as shipped with SGDK, and vbcc | **DID NOT RUN** | Not available. m68k codegen quality differs across GCC versions; these ratios are for 13.3 only |
| Z80 compiler comparison (SDCC) for the sound driver | **DID NOT RUN** | No Z80 C compiler installed. The 3,086-line Z80 driver is the code the owner's "functional everywhere" question is most exposed on; it needs its own experiment |
| Sigil-model cycle counts | **BLOCKED** | `@budget` refuses loops and calls; `cycles()` is Z80-only. Hand counts used instead, labelled ESTIMATED |
| Emulator-measured cycles for these routines | **DID NOT RUN** | Out of scope for a language measurement, and `mcp__oracle__*` is forbidden. A headless probe could confirm the 112-per-piece figure |

> [!note] For the owner's "functional everywhere" question (evidence, not a verdict)
> - Measured GCC overheads are 1.3x to 1.8x on ordinary routines, and come from the calling convention, promotion and addressing. A purpose-built tier that reuses the contracts removes most of that (ESTIMATED 1.0x to 1.25x).
> - The code where even 1.07x matters is concentrated:
>   - 349 hand-cycle comment lines sit in `raster*.emp`/`parallax.emp`;
>   - stage 0's cycle-critical class is 14% of code lines, 81% of it the Z80 driver;
>   - `@cycles_exact` has no use, but the driver's balance proofs are hand-written in comments.
> - No measurement here shows a compiler matching cycle-exact or bus-timed code. That case is untested, not disproven.

## Appendix: scripts (abridged; full files under `<scratch>/audit-builds/emp/scripts/`)

> [!note]- measure.py (classification core)
> ```python
> # per line: split_line() strips strings and separates comment text (same as stage 0's inventory.py)
> # top-level item tracked by brace depth; asm{} depth tracked separately
> if DATA_DIR.match(s) or item in ('data','table','offsets'): cat='data'
> elif re.match(r'^\s*(ensure|ensure_fatal)\s*\(', s): cat='guard'
> elif depth==0 and item in ('ensure','ensure_fatal'): cat='guard-cont'
> elif depth==0 and item in ('proc','script','attr'): cat='contract/header'
> elif body or in_template:
>     label-only -> 'label'; mnemonic in PSEUDO -> 'instr'; in DIAG -> 'diag'
>     mnemonic in (Z80 if module cpu:z80 else M68K) -> 'instr' (mix counted)
>     with/invoke -> 'with/invoke'; name(...) / name! -> 'template-call' (if/for/while/match excluded)
>     braces / else / if / for / let / comptime -> 'structure/comptime'; else 'body-other'
> else: comptime fn -> 'comptime'; const/equ/struct/vars/region/newtype/enum/bitfield/align -> 'decl';
>       use/module -> 'import'; context/interface/implement/type -> 'contract-decl'
> ```

> [!note]- themes.py regexes (over comment text)
> ```
> T1 explicit\s*`?\.w|kept[- ]width|widths?[ -](?:pinned|kept)|width-PINNED|imm-link|link-imm|\.w kept|sized (?:branch|on purpose)
> T2 abs-sym\s*\+\s*indexed|sigil gap|lowering gap|cannot yet emit|can'?t lower|sigil (?:cannot|can'?t|does not|doesn'?t) (?:emit|lower|encode|accept|parse)
> T3 hand-(?:spell|spelled|spelling|spells|typed|derived|computed|copied|synced|maintained|kept|word)|keep (?:it |them |these )?in sync|in lockstep|must (?:match|equal|agree with)|mirrors? (?:the|of)|duplicated? (?:here|in)
> T4 (?:language|sigil|\.emp|emp|checker|closure|contract model|gate|the model)\b[^.;]{0,40}\b(?:cannot|can'?t|does not|doesn'?t|has no|lacks|won'?t|no way to|is blind|not (?:expressible|modelled|modeled|checked|see))|cannot express|not expressible|no way to (?:say|express)|blind spot
> T5 (case-sensitive) \bAS\b|\basl\b|\.asm\b|byte-identical|\bported\b|the port\b
> T6 (case-sensitive) \bMUST\b|\bNEVER\b|caller must|must be called|only after|before calling|do not (?:reorder|move|call|touch|split)|order matters|ordering
> T7 \bunion\b|\boverlay|byte-shar|aliases? the same
> T8 \b\d+\s*(?:cycles?|cyc)\b|\bcycles?\s*[:=]\s*\d|\b\d+\s*\+\s*\d+\s*(?:=|cycles)
> T9 ^\s*//\s*(?:In|Out|Inputs?|Outputs?|Clobbers|Preserves|Trashes|Destroys|Uses)\s*:
> T10 \b20\d\d-[01]\d-[0-3]\d\b
> ```

> [!note]- Compiler commands
> ```
> docker run -d --name emp-audit-gcc -v <scratch>/audit-builds/emp/gcc:/work ubuntu:24.04 sleep 7200
> docker exec emp-audit-gcc apt-get install -y gcc-m68k-linux-gnu binutils-m68k-linux-gnu
> m68k-linux-gnu-gcc -O2|-Os -m68000 -fomit-frame-pointer -S|-c {obj,pieces,pieces2,derive}.c
> m68k-linux-gnu-nm -S --size-sort *.o
> m68k-linux-gnu-objdump -D -b binary -m m68k:68000 --start-address=… --stop-address=… s4.bin / s4.debug.bin
> docker rm -f emp-audit-gcc   # done at the end
> ```
