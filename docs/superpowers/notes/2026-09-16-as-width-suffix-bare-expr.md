# `(expr)` is the same absolute operand as `expr`, and the 120 corners behind that refusal

2026-09-16, parcel `AS-WIDTH-SUFFIX-BARE-EXPR`, branch
`parcel/as-width-suffix-bare-expr`, base `e87dcec6` (master's tip, equal to
`origin/master` at dispatch).

sigil refused a line of the Sonic 1 disassembly that asl assembles:

```text
_incObj/DebugMode.asm(245):3: error: absolute address operand `(expr)` needs an
explicit `.w`/`.l` width suffix (width-selecting bare `(expr)` is out of scope)

		move.w	(v_limitright2),d0		; get current right level boundary
```

An over-refusal on the AS-replacement path, which is the direction that blocks
adoption. It is now accepted, and the whole of the Sonic 1 `FixBugs = 1` arm
behind it is measured for the first time.

## Headlines

1. **The fix is the routing the other two absolute spellings already had.**
   `OperandAtom::Mem` goes to `abs_ea_from_expr`, exactly as
   `OperandAtom::Value` does for a bare symbol and a bare expression.
2. **Sonic 1 `FixBugs = 1` builds byte-identically with NO source edit.** crc32
   `888defef` / 551,288 bytes, the same image the stock toolchain writes,
   planted-byte control passed, 0 bytes different in 0 runs. That is the sweep
   agent's one-line measurement reproduced without the line.
3. **120 Sonic 1 corners went from unreachable to measured, and all 120 agree
   byte for byte.** So do the 24 that were already agreeing under a different
   count. Nothing is behind that wall.
4. **`BRA-W-RANGE-UNCHECKED` is ANSWERED, and the answer is good.** At the 24
   corners asl itself cannot build, sigil refuses too, at the SAME source line,
   for the same reason. sigil does not quietly emit an out-of-range `bra.w`.
   The row closes.
5. **The atom was NOT only `(expr)`, which is why the parcel is not one line.**
   asl's register names are case-insensitive even under `-U`, sigil's operand
   classifier claims them in lower case only, and the gap lands exactly on the
   arm being changed. `(A0)` is refused rather than read as an address.
6. **The doc comment this parcel was told to update was stale in a SECOND
   clause too**, and that half was stale before this parcel: it said
   `(d8,PC,Xn)` is rejected, and sigil has been lowering it.
7. **The sweep's composition prediction had become an always-red check**, as a
   consequence of the row closing, and the runner now asks it only where there
   is a reference to ask it against. The opposite direction, which nothing
   asserted, is now a finding.
8. **A SILENT DIVERGENCE WAS FOUND NEXT DOOR AND IS NOT FIXED HERE.** Chasing
   the case question turned up `move.w A0,d0`, which asl assembles as `3008`
   (a0 direct) and sigil as `3038 1234` (an absolute load) with a symbol `A0`
   in scope. Both exit 0 and say nothing. It is on the bare-`Value` path, is
   pre-existing on master, and is booked rather than patched.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | every figure here was measured with the binary built from `132fdce1`, md5 `fc8ff6ab5f436e483afb6010c400f6f0`, `--version` `132fdce1`, `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/width-suffix/target`. The branch tip is later than that and the difference is stated rather than waved past: `a49f86b7` and `1f4ae2c5` change only `scripts/` and `docs/`, which are not compiled, `4e26f122` is rustfmt on a test file, and `a1a62575` moves a doc comment in `eval.rs` between two functions. That last one recompiles, so the tip binary has a different digest (md5 `bd78436136373827c96ff481ed2676b8`) and no different behaviour: the three whole-file asl differentials below were RE-RUN against it and came back at the same three CRC32s. |
| asl | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`, the reference build pinned by `docs/superpowers/notes/asl-reference/asl_ref.sh`. Every probe invoked through `asl_run`; every listing quoted below came from a run reporting `ASL_EXIT=0` and `ASL_DIAG=complete`. |
| p2bin | `s1disasm/build_tools/Linux-x86_64/p2bin`, beside that asl. |
| corpora | `s1disasm f6ece657`, `s2disasm e45ebf332`, extracted read-only by `git archive` into scratch. The shared checkouts were never written to. |
| probes | `/home/volence/sonic_hacks/.scratch/width-suffix/probe/`, `p1` `p2` `p4` `p5` `p6` `p7` `p8` `p9`, each with its `.lst`, `.asl.bin` and `.sigil.bin`. |
| authoritative sweep | `--cross`, 422 legs launched and 422 reported, committed as `logs/cross-final.log` beside this note. The 422-leg run in `logs/cross-oldscript.log` is the SAME measurement taken before the runner change and is kept because it is where the 24 composition breaks are visible; it is not a second measurement. |
| workspace suite | `cargo test --release --workspace --no-fail-fast`, this branch and the base `e87dcec6`, `logs/suite-branch.log` and `logs/suite-base.log`. |
| red controls | `red-M1.log` `red-M2.log` `red-M3.log` `red-M4.log` `red-C9.log` under `.scratch/width-suffix/`, each opening with the mutation quoted back from disk and a `git diff HEAD --stat`. |

CRC32 throughout is IEEE/zlib, eight hex digits, quoted with the byte size.

## What the atom actually covered

`OperandAtom::Mem` is NOT every parenthesised operand, and this was checked in
`crates/sigil-frontend-as/src/operands.rs::classify` before anything was
changed. In order, `classify` returns BEFORE it can reach `Mem`:

| shape | atom |
|---|---|
| `#expr` | `Imm` |
| `-(An)` | `M68kPreDec` |
| `(An)+` | `M68kPostInc` |
| `(expr).w` / `(expr).l` | `M68kAbs` |
| `()` | `Value(0)`, the value zero, not an indirection |
| `(hl)` `(bc)` `(de)` `(sp)` | `IndReg` |
| `(a0)`..`(a7)` | `M68kInd` |
| `(ix+/-d)` `(iy+/-d)` | `Indexed` |
| `(d,An)` and `(d16,PC)` | `M68kDisp` |
| `(d,An,Xn)` and `(d8,PC,Xn)` | `M68kIdx` |
| `disp(An)` and `(disp)(An)` | via `build_disp_ea` |

`Mem(e)` is reached only by a whole-paren group whose inner is non-empty,
comma-free, and not one of the register spellings above. The two PC-relative
atoms then never reach `convert_atoms_m68k` at all: `lower_m68k_generic`
deflects `an == "pc"` to `lower_m68k_pcrel` and `lower_m68k_pcrel_idx` first.
`movem` reaches its memory EA through `convert_one_atom_m68k` directly, so it
gets the change without a second site.

So the hypothesis held: `Mem` is the absolute operand alone, and routing it to
`abs_ea_from_expr` is what `Value` already does.

## The asl differential

asl gives `(expr)` and bare `expr` the same encoding everywhere it was asked.
Every byte here is a listing column from a run that exited 0.

### `p1.asm`, the width rule on both spellings

```text
      15/       0 : 3038 1000           	move.w	SmallW,d0          ; SmallW = $1000
      16/       4 : 3038 1000           	move.w	(SmallW),d0
      17/       8 : 3038 1000           	move.w	(SmallW).w,d0
      19/       C : 3039 00A0 0000      	move.w	BigL,d0            ; BigL = $A00000
      20/      12 : 3039 00A0 0000      	move.w	(BigL),d0
      22/      18 : 3038 7FFE           	move.w	BoundW,d0          ; $7FFE
      23/      1C : 3038 7FFE           	move.w	(BoundW),d0
      24/      20 : 3039 0000 8000      	move.w	BoundL,d0          ; $8000
      25/      26 : 3039 0000 8000      	move.w	(BoundL),d0
      26/      2C : 3038 8000           	move.w	HiW,d0             ; $FF8000
      27/      30 : 3038 8000           	move.w	(HiW),d0
      28/      34 : 3038 FFFE           	move.w	HiTop,d0           ; $FFFFFE
      29/      38 : 3038 FFFE           	move.w	(HiTop),d0
      31/      3C : 3010                	move.w	(a0),d0
      32/      3E : 3010                	move.w	(A0),d0
      33/      40 : 3018                	move.w	(a0)+,d0
      34/      42 : 3020                	move.w	-(a0),d0
      35/      44 : 3028 0004           	move.w	(4,a0),d0
      36/      48 : 3028 0004           	move.w	4(a0),d0
      38/      4C : 3038 1002           	move.w	(SmallW+2),d0
      39/      50 : 3038 1002           	move.w	SmallW+2,d0
      40/      54 : 3038 1000           	move.w	((SmallW)),d0
      42/      58 : 31C0 1000           	move.w	d0,(SmallW)
      43/      5C : 31C0 1000           	move.w	d0,SmallW
      44/      60 : 33C0 00A0 0000      	move.w	d0,(BigL)
      46/      66 : 43F8 1000           	lea	(SmallW),a1
      47/      6A : 43F8 1000           	lea	SmallW,a1
      48/      6E : 43F9 00A0 0000      	lea	(BigL),a1
      49/      74 : 4278 1000           	clr.w	(SmallW)
      50/      78 : 4A39 00A0 0000      	tst.b	(BigL)
      51/      7E : 13F8 1000 00A0 0000 	move.b	(SmallW),(BigL)
      53/      86 : 3038 0092           	move.w	(FwdLab),d0
      54/      8A : 3038 0092           	move.w	FwdLab,d0
      55/      8E : 2238 0092           	move.l	(FwdLab),d1
```

**The width rule is the same rule.** `$7FFE` short and `$8000` long; `$FF8000`
and `$FFFFFE` short again, which is asl's sign-extension window and not a
magnitude test. The parenthesised spelling never differs from the bare one.

**A forward reference behaves identically** (lines 53 to 55), so the optimistic
abs.w that `abs_ea_from_expr` writes while a symbol is unresolved converges to
asl's width for this spelling too.

### `p4.asm`, the paths with EA handling of their own

```text
       6/       4 : 323B 00FA           	move.w	(Tbl,pc,d0.w),d1
       7/       8 : 323A FFF6           	move.w	(Tbl,pc),d1
       8/       C : 4EF8 0000           	jmp	(Tbl)
       9/      10 : 4EB8 0000           	jsr	(Tbl)
      10/      14 : 4ED0                	jmp	(a0)
      11/      16 : 4EF9 0000 0000      	jmp	(Tbl).l
      12/      1C : 4878 0000           	pea	(Tbl)
      13/      20 : 48B8 0003 0000      	movem.w	d0-d1,(Tbl)
      14/      26 : 4CB8 0003 0000      	movem.w	(Tbl),d0-d1
      15/      2C : 0C78 0005 0000      	cmpi.w	#5,(Tbl)
      16/      32 : 0838 0003 0000      	btst	#3,(Tbl)
      17/      38 : 31F8 0000 0000      	move.w	(Tbl),(Tbl)
```

`jmp (Tbl)` is an absolute jump and `jmp (a0)` stays register indirect, two
lines apart in one file.

### The whole-file byte compare

Each probe assembled by both toolchains and compared as BINARIES, not as
listings (`asl -o x.p` then `p2bin`, against `sigil x.asm -o x.sigil.bin`):

| probe | bytes | crc32 | result |
|---|---|---|---|
| `p1.asm` | 148 | `7f5def7b` | byte-identical |
| `p4.asm` | 64 | `863e68e7` | byte-identical |
| `p6.asm` | 14 | `c2500b78` | byte-identical |

## The one thing this does NOT newly accept, and why

**asl's register names are case-insensitive even under `-U`, and sigil's
classifier is not.** `p2.lst`, with a symbol of that name deliberately in scope:

```text
       4/       0 : =$1234               A0:	equ	$1234
       6/       0 : 3010                	move.w	(A0),d0
       7/       2 : 3010                	move.w	(a0),d0
       8/       4 : 3008                	move.w	A0,d0
      10/       6 : 3017                	move.w	(SP),d0
```

`(A0)` is a0 indirect and `A0` is a0 direct: the register wins over the symbol,
in both positions. sigil's `classify` matches `(a0)`..`(a7)`/`(sp)` in LOWER
CASE only, so `(A0)` and `(SP)` arrive at the very arm this parcel changed.
Routing them to absolute addressing would have turned a loud refusal into a
silently different instruction, which is worse than the refusal being removed:
the refusal announced itself, a wrong encoding would not.

So the `Mem` arm refuses a register name in any case, with a diagnostic that
names it. The set is measured, not guessed:

| inner | asl | sigil after this parcel |
|---|---|---|
| `(a0)`..`(a7)`, `(sp)` lower case | register indirect | register indirect (never reaches this arm) |
| `(A0)`, `(A7)`, `(SP)`, `(Sp)` | register indirect, `3010` / `3017` | REFUSED, naming the register |
| `(d0)`..`(d7)` any case | `error #1505: addressing mode not supported on 68000` | REFUSED, naming the register |
| `(pc)`, `(PC)` | PC-relative at zero displacement, `303A FFFE` | REFUSED, naming the register |
| `(sr)`, `(ccr)`, `(usp)` | ORDINARY SYMBOLS: with `sr: equ $2000`, `3038 2000` | absolute address, matching asl |

The last row is the one a symmetric guard would have got wrong. `p6.lst`:

```text
       4/       0 : =$2000               sr:	equ	$2000
       8/       0 : 303A FFFE           	move.w	(pc),d0
       9/       4 : 3038 2000           	move.w	(sr),d0
      10/       8 : 3038 2002           	move.w	(ccr),d0
      11/       C : 3038 2004           	move.w	(usp),d0
      13/      14 : 3017                	move.w	(A7),d0
```

Uppercase register indirect is booked as `AS-UPPERCASE-REGISTER-INDIRECT` in
the campaign gap ledger, with the reason it is a separate change: widening
`is_m68k_areg_name` widens a CPU-AGNOSTIC classifier, and `(A0)` in a Z80 file
is a legitimate memory reference through a symbol named `A0`.

**SUPERSEDED 2026-09-16 by `AS-UPPERCASE-REGISTER-INDIRECT`, on the same day.**
The two rows of the table above that read REFUSED for an ADDRESS register now
read "register indirect, matching asl": `classify` claims `(A0)`..`(A7)`/`(SP)`
in either case, and `(dN)` and `(pc)` are the only names the `Mem` arm still
refuses. The reason given here for deferring it was half right. `classify` is
not CPU-agnostic by design, it simply took a context struct with no CPU in it;
and the Z80 concern was NOT hypothetical and NOT created by the case fold -- the
`(aN)` branch already ran on every CPU in LOWER case, which was already wrong
for a Z80 `(a0)`. Threading the CPU through closed both. See
`2026-09-16-as-uppercase-registers.md`; the rest of this note stands.

## The doc comment was stale in a clause nobody was looking at

The brief named the doc comment on `convert_atoms_m68k` as a consuming surface
of the rule being changed, which it was. It also said `(d8,PC,Xn)` is a
separate open row booked against S3K and must not be closed here.

**It appears to have closed already, before this parcel.** The comment read:

> Any width-selecting bare-`(expr)` or `(d8,PC,Xn)` atom is rejected with a
> diagnostic (the latter is the only PC-relative form still unsupported).

sigil assembles `move.w (Tbl,pc,d0.w),d1` and agrees with asl at `323B 00FA`.
`lower_m68k_pcrel_idx` exists and `lower_m68k_generic` dispatches to it. The
refusal string the comment describes is still reachable, but only as the
fallback for an `M68kIdx` whose base register is not a valid `An` and is not
`pc` either, which is not the case it names. **No code was changed for this**;
the comment now says what is true, and red control M4 pins the behaviour by
breaking the deflection and watching the refusal come back.

## The corner space after the change

`scripts/switch_matrix_sweep.py --cross`, the whole documented build-option
space of both corpora: 288 Sonic 1 corners and 96 Sonic 2, 384 in all, plus the
38 one-at-a-time legs.

### Sonic 1, before and after

| | before (`SWITCH-MATRIX-SWEEP`, base `4f2b0cf6`) | after (this parcel) |
|---|---|---|
| corners | 288 | 288 |
| both toolchains built | 144 | 264 |
| ... agreed byte for byte | 144 | **264** |
| sigil declined for the width row | 120 | **0** |
| stock toolchain could not build | 24 | 24 |
| ... of those, sigil also declined | not measurable | **24** |
| ... of those, sigil built an unverifiable image | not measurable | **0** |

**All 120 corners that were behind the wall now build and all 120 agree.** No
new silent ROM anywhere in the option space.

The authoritative run's own lines, `logs/cross-final.log`, **SWEEP PASSED**,
exit 0, 422 legs launched and 422 reported:

```text
== CROSS s1disasm: 288 corners
   composition prediction on the sigil side: 264 held, 0 broke, 24 not asked
     (the stock toolchain declined the corner, so there is no reference and a
     sigil refusal there agrees with it)
   both toolchains built: 264 corners, 264 agreed byte for byte, 0 did not
   stock toolchain declined: 24 corners, 24 covered by an acknowledged rule
   ... of those, sigil ALSO declined 24 and BUILT 0. A corner only sigil builds
     has no reference image, so it is a finding and not a pass.
     sigil's refusal at s1disasm-X-0100000: _incObj/87, 88, 89 Ending Sequence
     Sonic, Emeralds, Logo.asm(270):3: error: (d16,PC)/bra.w displacement out of
   rule {'Revision': 0, 'FixBugs': 1, 'AllOptimizations': 0} covered 24 corner(s)
   distinct stock images across the corners: 126

== CROSS s2disasm: 96 corners
   both toolchains built: 48 corners, 48 agreed byte for byte, 0 did not

RECONCILE legs launched=422 reported=422
RECONCILE outcomes: 1 non-agreeing, 1 acknowledged, 0 unacknowledged,
                    0 stale acknowledgements, 0 class mismatches
SWEEP PASSED
```

The one non-agreeing leg is Sonic 2 `fixBugs = 1`, the driver-size row, which is
the only entry left in `ACK_DISAGREE`. Zero stale acknowledgements is the
positive half of removing the Sonic 1 entry: had the row not actually closed,
that line would read 1.

Sonic 2 is unchanged at 48 agreeing and 48 declined, for the unrelated
driver-size reason (`s2.sounddriver.asm:248`, `Size_of_Snd_driver_guess`),
which is a different open row and was not touched.

### And the row that could only be answered from behind the wall

`BRA-W-RANGE-UNCHECKED` asked whether sigil would quietly assemble the
out-of-range `bra.w` that asl refuses at the 24 Sonic 1 corners with
`Revision = 0` + `FixBugs = 1` + `AllOptimizations = 0`. Before this parcel
sigil stopped on the width suffix long before reaching that instruction, so the
question was unmeasurable and both toolchains showed as declining for reasons
that had nothing to do with each other.

Now they reach it, and they agree:

```text
asl    87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270):
       error #1370: jump distance too big
         bra.w DisplaySprite    ; display sprite

sigil  _incObj/87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270):3:
       error: (d16,PC)/bra.w displacement out of range (32790) in section sec752
```

Same file, same line, same cause, and sigil names the actual displacement
(32790, which is 23 past `bra.w`'s 32767). **sigil range-checks `bra.w` and
does not write a silent wrong ROM.** The row closes.

That answer is now a standing gate rather than an observation:
`reconcile_stock_declines` in the sweep splits stock-declined corners by what
sigil did, and a corner only sigil builds must be named in
`ACK_STOCK_DECLINE_SIGIL_BUILT` or the run fails. The table is empty and the
emptiness is asserted in both directions.

### And a check that had become always-red

Closing the width row turned the sweep's composition prediction red on correct
behaviour. The prediction reads sigil's refusing settings off the same run's
phase 1; with no Sonic 1 setting refusing, it predicts BUILT at all 288
corners, and the 24 asl cannot build would score as 24 broken predictions
forever. A check that fires on correct code trains people to weaken it. The
prediction is now asked only where the stock toolchain built, the skipped count
is reported, and the direction that actually carries risk (sigil building where
asl could not) became the assertion above.

## The red controls

Every gate here has been shown firing on a mutation printed back from disk,
with `git diff HEAD --stat` beside it, and restored from a COMMITTED baseline.

| control | mutation | result |
|---|---|---|
| M1 | the `Mem` arm reverted to the pre-parcel unconditional refusal | 9 of 13 tests red, including every acceptance test and both refusal-text tests |
| M2 | `m68k_reg_name_any_case` returns `false`, so the register guard is gone | 3 red: the uppercase, data-register and `(pc)` refusals |
| M3 | `is_m68k_areg_name` returns `false`, so `(a0)` falls out of the classifier into the absolute arm | 3 red: the register-indirect family, the symbol-named-like-a-register case, and the control-flow encodings. The guard caught `(a0)` loudly rather than encoding an address, which is the guard doing its job |
| M4 | the `(d8,PC,Xn)` deflection in `lower_m68k_generic` matches a name no atom carries | 1 red: `pc_relative_is_still_pc_relative`, with the fallback refusal in the message |
| C9 | `reconcile_stock_declines` never reports the unverifiable half | the sweep self-test's C9 fails and `self_test` returns `False`; green before and after |

M1's green-after was the commit itself; C9's log carries an explicit green,
red, restore, green so the control is known to have a possible green.

## The workspace suite, and why its number needs a baseline beside it

| tree | passed | failed | ignored |
|---|---|---|---|
| this branch | **5,142** | **392** | 2 |
| base `e87dcec6` | **5,129** | **392** | 2 |

The failing sets are IDENTICAL: 375 distinct test names on both sides, zero on
one and not the other. The branch adds exactly 13 passes, which is exactly the
13 tests this parcel adds. **Nothing moved.**

The 392 are not a parcel result and must not be read as one. **389 of them are a
single environment gap**: `crates/sigil-harness/src/test_support.rs` refuses to
name a reference tree because this lane has no `AEON_DIR` provisioned, and says
so in those words. The other 3 are `PoisonError` cascading from those panics
inside the same test binary. Booked in the campaign gap ledger; the landing gate
the controller runs on the merged tree is what answers the aeon-paired half.

## Reproducing

```sh
# the asl differential (asl_ref.sh refuses any build but the reference one)
cd /home/volence/sonic_hacks/.scratch/width-suffix/probe
. /home/volence/sonic_hacks/sigil/docs/superpowers/notes/asl-reference/asl_ref.sh || exit $?
./diff.sh p1.asm          # BYTE-IDENTICAL, 148 bytes, crc32 7f5def7b

# the sharp check: unpatched Sonic 1, FixBugs = 1, no source edit
python3 scripts/switch_matrix_sweep.py --sigil <sigil> --scratch <dir> \
    --corpus s1disasm=/home/volence/sonic_hacks/s1disasm --only s1disasm-FixBugs-1

# the whole corner space
python3 scripts/switch_matrix_sweep.py --sigil <sigil> --scratch <dir> --cross
```

## What is NOT closed

* **Uppercase register indirect** (`(A0)`, `(SP)`), refused rather than
  assembled. Ledger row `AS-UPPERCASE-REGISTER-INDIRECT`.
* **Bare uppercase register operands, and this one is a SILENT divergence
  that exists on master today.** `move.w A0,d0` with `A0: equ $1234` in scope:
  asl writes `3008` (a0 direct, the register winning over the symbol), sigil
  writes `3038 1234` (an absolute load from $1234). Both exit 0, neither says
  anything, and the instructions are different. It is on the `Value` path, not
  the arm this parcel changed, and it was NOT touched here. It is the reason
  the uppercase question is booked as a row of its own rather than patched at
  this arm: the arm is only half of it.
* **Sonic 2 `fixBugs = 1`**, 48 corners, the driver-size row.
* **Runtime.** No ROM built here was run. No emulator was touched. TAGGED for
  the controller.
