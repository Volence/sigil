# asl's register names fold case, and they are per-CPU: closing `AS-UPPERCASE-REGISTER-INDIRECT`

2026-09-16, parcel `AS-UPPERCASE-REGISTER-INDIRECT`, branch
`parcel/as-uppercase-registers`, base `8befab99` (master's tip at dispatch).

The booking had two halves, one loud and one silent, and a warning that the
change needed a CPU the classifier did not have. All three held up. The
measurement that decided the design was not in the booking.

## Headlines

1. **The SILENT half was on master and is measured here end to end.** With
   `A0: equ $1234` in scope, `move.w A0,d0` is `3008` to asl and was
   `3038 1234` to sigil. Two different instructions, both toolchains exit 0,
   neither says a word. Probe `probes/silent.asm`: asl writes 6 bytes, sigil
   wrote 14.
2. **asl's register table is per-CPU, and that is a measurement, not an
   argument.** Under `cpu z80`, with `A0: equ 05678h` in scope, `ld a,(A0)`
   assembles to `3A 78 56` -- an ABSOLUTE load through the symbol. Under
   `cpu 68000` the same parenthesised name is `3010`, a0 indirect. A case fold
   that did not know the CPU would be right in one corpus and wrong in the
   other.
3. **So `ExprCtx` now carries the CPU, and `classify` asks it.** That was the
   real obstacle behind the booking's framing that `classify` is CPU-agnostic
   by design: it is not a design, it is a struct with two fields.
4. **The `(a0)` branch was already wrong for the Z80 before the fold, in LOWER
   case.** It ran on every CPU with a comment arguing that `a`+digit is
   unambiguously 68k because the Z80 has no such register. That is an argument
   about the NAME; asl settles it by CPU. Gating the branch on `M68000` fixes
   that alongside the case fold, which is the payoff of threading the CPU
   rather than folding structurally.
5. **The LOUD half is now simply correct rather than loud.** `(A0)`/`(SP)`
   assemble as registers, matching asl. What stays refused is `(dN)` and
   `(pc)`: registers to asl, and sigil has no operand for either.

## The reference

asl, md5 `61e672562465725a8c102288a7da9098` (`s1disasm/build_tools/Linux-x86_64`),
through `asl_run` with `-xx -n -q -A -L -U`. Every listing quoted below came
from a run reporting `ASL_EXIT=0` and `ASL_DIAG=complete`, except where the
probe exists to observe a refusal and says so. Probes and listings are in
`2026-09-16-as-uppercase-registers/probes/`.

## What asl does, measured

`probes/p9.asm`, `cpu 68000`, no `org`, with `A0`, `SP`, `D0`, `SR`, `CCR` and
`USP` all defined as equates. asl marks every one of them unused (`*`) in the
symbol table, which is asl saying in its own output that it never consulted
them:

```text
       8/       0 : 3010                	move.w	(A0),d0
       9/       2 : 3008                	move.w	A0,d0
      10/       4 : 3017                	move.w	(SP),d0
      11/       6 : 300F                	move.w	SP,d0
      12/       8 : 300F                	move.w	Sp,d0
      13/       A : 3200                	move.w	D0,d1
      14/       C : 3018                	move.w	(A0)+,d0
      15/       E : 3020                	move.w	-(A0),d0
      16/      10 : 3028 0004           	move.w	(4,A0),d0
      17/      14 : 3028 0004           	move.w	4(A0),d0
      18/      18 : 3030 1004           	move.w	(4,A0,D1.W),d0
      19/      1C : 3030 1000           	move.w	(A0,D1),d0
      20/      20 : 3030 1800           	move.w	(A0,D1.L),d0
      21/      24 : 48A7 8080           	movem.w	D0/A0,-(sp)
      22/      28 : 43D0                	lea	(A0),A1
      23/      2A : 40C0                	move.w	SR,d0
      24/      2C : 44C0                	move.b	d0,CCR
      25/      2E : 4E60                	move.l	a0,USP
```

`probes/p10.asm`, `cpu z80`, no `org`, with `HL: equ 01234h` and
`A0: equ 05678h`:

```text
       4/       0 : 7E                  	ld	a,(HL)
       5/       1 : 7E                  	ld	A,(HL)
       6/       2 : 41                  	ld	B,C
       7/       3 : DD 7E 03            	ld	a,(IX+3)
       8/       6 : 3A 78 56            	ld	a,(A0)
       9/       9 : 01 34 12            	ld	BC,01234h
      10/       C : 20 FE               	jr	NZ,$
      11/       E : E3                  	ex	(SP),HL
```

Line 8 is the whole design question in one row. `HL` is a register on this CPU
and its equate is ignored; `A0` is not, and its equate is loaded from.

The refusals, each from a run that reports the error rather than a byte:

* `probes/p4.asm`: `move.w (D0),d1` is `error #1505: addressing mode not
  supported on 68000`.
* `probes/p5.asm`: `move.w (A0).w,d0` is `error #1146: expected integer or
  string` -- a register in parens is not an address expression, so a width
  suffix cannot apply to it.
* `probes/p6.asm` line 11: Z80 `ld a,(SP)` is `error #1350: addressing mode not
  allowed here`; `(sp)` is a register indirect only for `ex (sp),hl`.
* `probes/p3.asm`: `move.w (PC),d0` asl ACCEPTS, as PC-relative at zero
  displacement (`303A EFFE` at `$1000`, that is `0 - ($1002)`). sigil has no
  operand for it and refuses. A documented gap, not an agreement.

## What changed

`crates/sigil-frontend-as/src/expr.rs`
: `ExprCtx` gains `cpu`. The test-only `plain` constructor takes it as an
  argument rather than defaulting, for the same reason the struct bundles its
  fields: a default would let a caller ask a CPU-keyed question without saying
  which CPU it meant.

`crates/sigil-frontend-as/src/operands.rs`
: `classify`'s single-ident-in-parens region is now CPU-keyed. `(hl)`/`(bc)`/
  `(de)` fold only off the 68000; `(sp)` folds on both, because `(SP)` is a
  register on both (`3017` on the 68000, the `ex (sp),hl` operand on the Z80);
  and the `(aN)` branch runs on the 68000 ONLY, in either case. The bare
  register/condition word folds off the 68000. Every exact-lower-case arm is
  kept unconditional, so the lower-case reading does not move anywhere.
  `is_m68k_areg_name`, `is_m68k_dreg_name`, `index_reg`, `is_bare_register_token`,
  `split_index_reg_size` and `disp_base_reg` fold case.

`crates/sigil-frontend-as/src/eval.rs`
: `m68k_addr_reg` and `m68k_data_reg` fold case. They can do so with no CPU
  argument because every caller is on a 68000 lowering path -- a Z80 statement
  never reaches one -- so the per-CPU half of asl's rule is carried by which
  lowering ran. Same for the Z80 `reg8`/`reg16`/`cond_word` and the `IndReg`
  arm, which only a Z80 statement reaches. `sr`/`ccr`/`usp` in the `Value(Sym)`
  arm fold. `m68k_reg_name_any_case` is now
  `m68k_paren_reg_without_operand` and covers `pc` and `dN` only: address
  registers were in it because `classify` could not claim an uppercase `(A0)`,
  and that path no longer exists.

## Four more sites, found by MEASURING a claim instead of asserting it

The first commit on this branch amended `2026-09-11-z80-half-registers.md` to
say its four upper-case probes were now closed. That amendment was written from
the shape of the change, not from a run, and it was WRONG. Measured on
`probes/p15.asm`, `cpu z80undoc`:

```text
       3/       0 : DD 7D               	ld	A,ixl      sigil: REFUSED
       4/       2 : DD 60               	ld	IXU,B      sigil: REFUSED
       5/       4 : DD 7D               	ld	a,IXL      sigil: agreed
       6/       6 : FD 67               	ld	IYU,A      sigil: REFUSED
```

`index_half` already folded, so the HALF's own spelling was fine; the plain
register beside it went through a `reg_word` closure comparing `w.as_str()`
against a lower-case list. Widening the search from there found three sites of
the same shape, all LOUD rather than silent, all now folded and pinned:

* `lower_m68k_generic`'s PC-relative scan matched `an == "pc"` exactly, so
  `move.w (Tbl,PC),d1` missed the PC-relative lowering and died on
  `m68k_addr_reg("PC")`. asl: `323A FFFA` (`probes/p16.asm` 4).
* `m68k_special_reg_size` read the implicit size off the operand NAME with an
  exact match, so `move D6,CCR` was refused for want of a size suffix asl never
  needed. asl: `44C6` / `46FC 2700` / `4E66` (`probes/p16.asm` 6 to 8).
* the half-register `reg_word` above.
* and one more found by sweeping the crate for the shape rather than waiting to
  trip over it: `classify` compared `w == "af'"` exactly, so `ex AF,AF'` was
  refused where asl writes `08` (`probes/p17.asm`).

**The lesson is the one in the memory note about names and behaviour: a fold in
one function is not a fold in the paths that read its output.** The amendment
has been rewritten to say what a run says. The whole crate was then swept for
the shape (a string comparison against a register-name literal outside a
case-folded scrutinee) and the four above are all of them.

## What is NOT in this parcel, and is booked instead

asl reads a register name as a register in EXPRESSION position too, not only in
an operand, and sigil does not. Measured on `probes/p2.asm`, `probes/p11.asm`,
`probes/p12.asm` and `probes/p13.asm`, all `cpu 68000`:

| source | equate in scope | asl | sigil |
|---|---|---|---|
| `dc.w a0` | none | exit 0, EMITS NOTHING, no diagnostic | `error: \`a0\` is a register, not a value` |
| `dc.w a0` | `a0: equ $1234` | exit 0, EMITS NOTHING, no diagnostic | emits `1234` |
| `dc.w A0` | `A0: equ $1234` | exit 0, EMITS NOTHING, no diagnostic | emits `1234` |
| `dc.w A0+1` | `A0: equ $1234` | `error #1145: expected integer, floating point number or string but got register` | emits `1235` |
| `move.w #A0+1,d0` | `A0: equ $1234` | same `#1145` | emits `303C 1235` |

Two separate divergences live there and neither is case-specific.

1. A BARE register in a data directive is asl's silent-decline regime: it
   values nothing, advances the address by nothing, and says nothing. sigil
   either refuses it loudly (no symbol) or emits the shadowing symbol's value.
   The refusal is the better behaviour of the two and is what
   `is_expr_register_name` / `register_in_value_position` were built for; the
   shadowed case slips past because those fire only on an UNRESOLVED name.
2. A register inside an EXPRESSION is a hard `#1145` to asl and is silently
   valued by sigil whenever a symbol of that name exists.

Both are the same root: sigil consults the symbol table for a name asl has
already claimed for its register table. It is orthogonal to case -- lower-case
`dc.w a0` with an equate in scope diverges identically -- so it is booked as its
own row rather than folded into this parcel. Closing it makes any symbol named
`a0`..`a7`, `d0`..`d7` or `sp` unusable in a 68000 expression, which is a wider
blast radius than an operand-position fold and wants its own gate.
