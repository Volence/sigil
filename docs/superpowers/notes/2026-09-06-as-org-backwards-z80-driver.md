# AS-ORG-BACKWARDS-Z80-DRIVER: a backwards `org` is refused, so `$` never leaves the 68000 address space

Parcel note. Branch `parcel/as-s1-driver-size`. **BLOCKED: the fix moves emitted
bytes. Nothing in `crates/` was changed.**

## Provenance

| | |
|---|---|
| sigil | branch `parcel/as-s1-driver-size`, base and tip content-identical to master `4415832c` in `crates/`; binary built into `.target-land` in this worktree, md5 `72b3e6ab36084806538cbe00fcf8929d` |
| corpora | `s1disasm` `f6ece657`, `s2disasm` `e45ebf33`, `skdisasm` `2fcd861`, each in its OWN detached worktree under `/home/volence/sonic_hacks/.corpus-s1drv/`; the shared live checkouts were never written |
| preparation | `scripts/corpus-prepare.sh` under `Lua 5.5.1`; 8 / 74 / 100 generated files written; `corpus-baseline.sh` reports `READY (4/4)`, `(39/39)`, `(50/50)` |
| baselines reproduced | **42 / 5,162 / 2,126**, matching `2026-09-06-corpus-generated-includes.md` exactly, all three over prepared trees |
| oracle | `asl` md5 `61e672562465725a8c102288a7da9098` through `asl_ref.sh`'s `asl_run`. The `s2disasm` build (md5 `0dee1f98...`) was never invoked. Every value quoted below comes from a run reported `ASL_EXIT=0` and `ASL_DIAG=complete`, except where the text says otherwise |
| emulator | none. No runtime confirmation was attempted or is implied |

## The mechanism

`s1disasm/sound/z80.asm` opens the Z80 driver with

```
	save
	!org	0		; z80 Align, handled by the build process
	CPU Z80
```

**asl's `org 0` sets the program counter to 0. Sigil REFUSES it, and the refusal
is already in the corpus stream one row above the one this parcel was sent for:**

```
sound/z80.asm(9):   error: org target precedes the current phase base
sound/z80.asm(229): error: The driver is too big; ... It currently takes 73DFDh bytes.
```

`directive_org` (`eval.rs`) models `org` as a PHYSICAL relocation: it computes
`phys_target = target - disp` and, with a section open, refuses any target below
the section's base rather than seeking backwards in a linear image. `org 0`
inside the 68000 ROM is such a target, the refusal returns early, and the
location counter is left where it was. Every `$`, every label and every
alignment decision inside the driver is then taken in the enclosing 68000
address space.

### The discriminating measurement

`warning` probes inserted (never substituted) into the prepared tree, so the
fault is still present in the probed tree. `\{*}` rather than `\{$}` for the
sites that sit under `CPU 68000`, because both asl and sigil refuse `$` as an
interpolated PC there (see the residue section). One instrumented tree, both
assemblers, asl `ASL_EXIT=0` / `ASL_DIAG=complete`:

| probe | asl | sigil |
|---|---|---|
| before `include "sound/z80.asm"` | `72E7C` | `72240` |
| before `save` | `72E7C` | `72240` |
| **immediately after `!org 0`** | **`0`** | **`72240`** |
| `$` at the size check, z80.asm(229) | `1BC6` | `73DFD` |
| `zDAC_Kick` | `EE` | `72325` |
| `zDAC_Timpani` | `BB0` | `72DE7` |
| `DACDriver` | `72E7C` | `72240` |

The org row is the one that varies alone: everything above it agrees on the
shape of the address, and the value changes at exactly that directive. That is
the mechanism, and it is one directive wide.

### The brief's hypothesis, and what was wrong with its arithmetic

The dispatch proposed that sigil reports `$` in the enclosing 68000 space, and
offered `0x73DFD - 0x1BC6 = 0x72237` as the confirming figure "if that equals the
ROM base of the Z80 block".

**The mechanism is right. The arithmetic is not, and it would have confirmed the
right conclusion for the wrong reason.** `DACDriver` is `72E7C` under asl and
`72240` under sigil, and neither is `72237`. Two independent error terms sit
inside that subtraction:

* **`0xC3C`**, because sigil's `DACDriver` is 3,132 bytes lower than asl's. That
  is upstream emission sigil never produced (35 diagnostics precede the driver:
  18 `unexpected character`, 9 unrecognised `charset` lines, 6 `bad immediate
  expression`, one `listing` and one `page`), and it has nothing to do with
  `org`.
* **`9`**, because sigil's driver body is 9 bytes shorter than asl's. Four Z80
  operand lines fail with `trailing tokens in operand`, all four a user
  `function` call in operand position:
  `ld a,zmake68kBank(SegaPCM)&1` (2 B), `ld a,zmake68kBank(SegaPCM)>>1` (2 B),
  `ld de,zmake68kPtr(SegaPCM)` (3 B), `ld b,pcmLoopCounter(16000)` (2 B). 2+2+3+2
  is the 9.

`0x72E7C - 0xC3C - 9 = 0x72237`, so the two terms compose into a plausible base
that is not anybody's base. Had `72237` been checked against `DACDriver` from
sigil alone it would have missed by 9; against asl alone, by `0xC45`. The probe
table above avoids the subtraction entirely by asking each assembler for each
value.

## The blast radius

### What reads `$`

`Asm::here()` / `here_i64()` in `crates/sigil-frontend-as/src/eval.rs`, complete
list of readers:

| site | what it does | emits? |
|---|---|---|
| `eval.rs:5021` `define_label` | **the value of every label** | yes, through every reference |
| `eval.rs:1968` / `1969` | `$` and `*` as an expression operand | yes, anywhere the expression is `dc`/`dw`/an immediate |
| `eval.rs:1588` | `$` in symbol position | yes |
| `eval.rs:7402` | `$` folded into a deferred `Expr` | yes |
| `eval.rs:6173` `asl_align_pad` | how many bytes `align n` inserts | **yes, directly** |
| `eval.rs:5137` `word_pad_due` | whether a 68000 word pad byte is emitted | **yes, directly** |
| `eval.rs:6878` | the PC-relative displacement word | **yes, directly** |
| `eval.rs:4181` | `struct` member base addresses | yes |
| `eval.rs:4544` | whether a value is a PLACED label the linker may relocate | yes, through placement |

Nine readers, seven of which reach emitted bytes. `$` is not a diagnostic
input in this frontend; it is the location counter.

### The bytes this specific site moves, measured

`sound/z80.asm`'s own `zPCM_Table` emits Z80-space label addresses as data. From
the clean asl listing:

```
(2)  220/      E6 : B0 0B          dw      zDAC_Timpani     ; Start
```

`B0 0B` is `0BB0h` little-endian, which is `zDAC_Timpani` in the driver's own
address space. Sigil's `zDAC_Timpani` is `72DE7h`. Correcting `$` at this site
changes that word, and the two beside it, and every Z80 `jp`/`call` target in
the 5,000 lines the fatal currently prevents sigil from reaching.

`ensure1byteoffset` in the same file conditions an `align 100h` on `$`
(`offsetover1byte function from,maxsize, ((from&0FFh)>(100h-maxsize))`), so the
PADDING COUNT is a direct function of the value under discussion.

### And the current behaviour is arrangement-dependent, which is the worse half

The same `!org 0` does something DIFFERENT in Sonic 2, and it does it silently.
`directive_org` has two paths, chosen by whether a section happens to be open:
with one open it refuses, with none open it sets `phys_base` unconditionally.
`s2disasm/s2.sounddriver.asm(248)` reaches it with no section open. Probed the
same way, sigil alone:

```
s2.sounddriver.asm(248): PROBE-S2 before-org $=EA53Eh
s2.sounddriver.asm(250): PROBE-S2 after-org  $=0h
```

**No diagnostic at all**, and the Z80 driver is then placed at physical ROM
offset 0, on top of the vector table. s2disasm's own source comment at that site
says why `phase` is not the answer:

> In what I believe is an unfortunate design choice in AS, both the phased and
> unphased PCs must be within the target processor's range, which means phase is
> useless here despite being designed to fix this problem.

So the corpus author has already ruled out the cheap repair, in writing, at the
site. asl's `org 0` really does reset the counter and really does put the driver
in its own output chunk at address 0; the build then compresses that chunk and
pastes it into the hole (`p2bin -z=0,kosinski,Size_of_DAC_driver_guess,after`).
Reproducing that is a placement change, not an expression change.

## Scope verdict: BYTE-MOVING, STOPPED

Every candidate repair changes emitted bytes:

1. **Let the backwards `org` through as a physical relocation.** Emits the driver
   at ROM offset 0. This is what already happens in Sonic 2, silently.
2. **Model it as a phase.** Changes every label in the block from 68000-space to
   Z80-space, which changes the `dw` words above and the `align 100h` padding.
   Also refused by the corpus author's own comment for the general case.
3. **Give `org` its own output chunk at the target.** The correct emulation of
   asl, and by construction a placement change.
4. **Touch only the diagnostic string.** There is no diagnostic-only fix here:
   the wrong number is `$`, the location counter, and the two `fatal`s are the
   corpus correctly reporting the value sigil gave it. Suppressing them would
   hide a real defect rather than repair one.

Per the dispatch, this stops here. Aeon needs a paired verification before any
of the above lands.

**What the aeon check will need to establish, and what this parcel could see
without one.** No `.asm` file in the aeon tree contains an `org`, `save`,
`restore`, `phase` or `dephase` directive: a case-insensitive sweep of all 378
`*.asm` files returns nothing in directive position (the only `save`/`restore`
hits are prose in `engine/debug/debugger.asm`'s comments). Aeon's Z80 sound
driver is `engine/sound/z80_sound_driver.emp`, on the `.emp` frontend, which does
not route through `sigil-frontend-as`'s `directive_org`. That is a source-side
observation and NOT a build: `AEON_DIR` was deliberately not set, because
`sigil build --aeon` writes into the owner's live checkout. **A ROM-byte
comparison was not run and is not claimed.**

## The class is larger than the row it was reported as

Sized over the three prepared corpora at the binary above, all three baselines
reproduced.

| corpus | site | sigil says | asl says |
|---|---|---|---|
| s1disasm | `sound/z80.asm(9)` `!org 0` | `org target precedes the current phase base` | PC becomes `0` |
| s1disasm | `sound/z80.asm(229)` driver size | `73DFDh` (fatal) | `1BC6h` (message, and the driver fits) |
| skdisasm | `Sound/Z80 Sound Driver.asm(226)` `!org 0` | `org target precedes the current phase base` | PC becomes `0` |
| skdisasm | `Sound/Z80 Sound Driver.asm(345)` `rsttarget` | `Function GetPointerTable is at 0F0908h, but must be at a multiple of 8 bytes <= 38h` (fatal) | `GetPointerTable label $` lists `=8H`, and `rst GetPointerTable` assembles to `CF`, which IS `rst 08h` |
| s2disasm | `s2.sounddriver.asm(248)` `!org 0` | **nothing**; `$` becomes `0` and the driver is placed at ROM offset 0 | PC becomes `0` |

Four rows in the corpus streams, plus one site that produces no row at all. The
s2 site is the reason a diagnostic count cannot price this class: the arm with
the worst consequence is the one that says nothing.

### What the two fatals cost the rest of the run

Both `fatal`s set `aborted`, which stops the pass at that source line.

* s1disasm: the fatal is at `sound/z80.asm(229)` of a 237-line file, so the last
  8 lines of the driver and everything `s1.sounddriver.asm` includes after it are
  never reached.
* skdisasm: the fatal is at `Sound/Z80 Sound Driver.asm(345)` of a **5,315-line**
  file. **The whole stream carries exactly 3 rows naming that file**, all three
  at or above line 345. Whatever sigil would say about the other 4,970 lines of
  Z80 source is not in the 2,126 total and has never been counted.

So fixing this can ADD rows in two corpora by letting the assembler reach source
it currently abandons. **The post-fix totals must be measured, never predicted by
subtraction.**

## Residue found on the way, not fixed here

**`\{$}` interpolates to nothing under `CPU 68000`, where asl raises an error.**
The first probe round wrote `\{$}` at three sites under the 68000 CPU. asl
refused all three:

```
s1.sounddriver.asm(2632):38: error #1020: invalid symbol name
$
```

Sigil emitted the warning with the text `$=\{$}h` LEFT UNINTERPOLATED and raised
nothing. Under `CPU Z80` (where `$` is unambiguous, hex taking an `h` suffix)
both assemblers interpolate it. That is a separate silent-acceptance row, not
this parcel's, and it is in the gap ledger. It is also the reason the probe table
above uses `\{*}`: `*` is the PC on both CPUs in both assemblers, so the
comparison is not confounded by it.

**A Z80 operand cannot be a user `function` call.** The four `trailing tokens in
operand` rows in `sound/z80.asm` are all `ld r,userfunc(arg)`, and they are the
whole of the 9-byte body difference above. In the gap ledger.

## What a reader should not take from this note

The 9-byte and `0xC3C` figures are properties of TODAY's binary
(`72b3e6ab36084806538cbe00fcf8929d`) over these corpus revisions. They are given
to show that the brief's subtraction concealed two error terms, not as constants.
Re-derive them, do not carry them.

## What could not be run, named

* **`scripts/landing-run.sh` was not run.** It resolves a reference aeon tree,
  and step 3 of its resolution order derives the owner's live checkout from this
  worktree's `git-common-dir`. The dispatch forbids pointing this lane at that
  tree. No file under `crates/` was changed by this parcel, so the suite result
  at this tip is the suite result at master `4415832c`; that is an argument, not
  a measurement, and it is stated as one.
* **No emulator.** Nothing here wants runtime confirmation.
* **No sigil run in this parcel produced a binary.** Every sigil invocation ended
  at a `fatal`, so the placement claims ("the driver is emitted at `72240h`", "at
  physical ROM offset 0") are read off the FRONTEND's location counter and label
  values, which is where `directive_org` acts. What the link stage would then do
  with those sections is unmeasured, and no ROM-byte diff exists for any of this.
* **The s2disasm asl comparison exited 2** (pre-existing `#1010 symbol undefined`
  at `jsrto` sites under the plain invocation, which is not the flag set
  `build.lua` uses). Its probe values are therefore NOT quoted as oracle values.
  The load-bearing s2 finding, that sigil sets `$` to 0 there and says nothing,
  is a measurement of sigil alone and needs no oracle.
