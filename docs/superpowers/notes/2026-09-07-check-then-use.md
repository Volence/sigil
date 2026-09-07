# Check-then-use: six sites where the producer checked and the consumer cast

2026-09-07. Branch `parcel/check-then-use` off master `99aa2645`.

The class: a property verified at the PRODUCER that the CONSUMER never sees. A
value range-checked as a wide integer and then cast narrow; a window checked on
three routes and not the fourth; an evaluator that refuses at comptime and wraps
at link. The `dc.w`/`dc.l` row was already fixed (merge `81f05a29`,
`check_data_range`); this parcel lands the six remaining rows as one branch,
one commit per site, each red-first on the committed source with only the test
file added.

Every asl verdict below is from the reference build
(md5 `61e672562465725a8c102288a7da9098`) through the guarded `asl_run`, with
the exit status read on every run. A run that exits non-zero is quoted only for
its diagnostic, never for a byte column.

## 1. AS `align` (commit `40348de7`)

Reproduction on the committed release binary:

```text
    dc.b $11 / align $100000000 / dc.b $22
        thread 'main' panicked: attempt to calculate the remainder with a
        divisor of zero                                       exit 101
    dc.b $11 / align $100000001 / dc.b $22   ->  11 22       exit 0
```

`n > 0` on the i64, then `n as u32` (0 and 1 respectively) into `asl_align_pad`.

asl: `align $100000000`, `align $100000001` and `align $10000` are each
`error #1320: range overflow`, exit 2 (its count is a 16-bit word). `align
$FFFF` assembles, exit 0, next byte at `$FFFF`.

Fix: `ALIGN_COUNT_RANGE` (`1..=u32::MAX`, the domain of `asl_align_pad`) is
tested on the i64 before the cast. Diagnostic:

```text
    align_big.asm(3): error: align count 4294967296 out of range 1..=4294967295
```

`align $10000` stays accepted (the pad function is defined on every non-zero
u32); that is looser than asl in the direction that cannot refuse a program asl
builds. Zero and negative counts take the same message (was `align needs a
positive constant`; nothing pinned it).

Red (only `tests/as_align_range.rs` present): `align_two_to_the_32_...` panicked
at `align.rs:41`, `align_two_to_the_32_plus_one_...` assembled to `[11, 22]`.
Green after: 5 passed. Runner: `cargo test -p sigil-frontend-as --test
as_align_range`.

## 2. AS `ds.b/w/l` (commit `9584b7a9`)

Reproduction on the committed release binary:

```text
    dc.b $11 / ds.b $100000000 / dc.b $22    ->  11 22       exit 0
    dc.w $1111 / ds.w $80000000 / dc.b $22   ->  11 11 22    exit 0
```

In the debug profile the same inputs panic (`attempt to multiply with
overflow` at the reservation; `attempt to add with overflow` in
`IrBuilder::reserve` for `ds.b $FFFFFFFF` at offset 1).

asl: `ds.b $100000000` is `error #1320: range overflow`, exit 2. asl is itself
silent and wrong on three rows: `ds.w $80000000` exits 0 with the PC advanced
by ZERO (listing `3/ 2 : ds.w $80000000` then `4/ 2 : 22`); `ds.b -1` moves
the PC backwards to 0; `ds.b $FFFFFFFF` wraps the PC to 0. Refused here, not
matched.

Fix: three checks where the three narrowings happen. `DS_COUNT_RANGE`
(`0..=u32::MAX`) on the i64; `checked_mul(unit)` for the byte total;
`checked_add` on the section cursor. Diagnostics:

```text
    ds_big.asm(3): error: ds count 4294967296 out of range 0..=4294967295
    ds_big_w.asm(3): error: ds.w 2147483648 reserves 4294967296 bytes, more than
        the 32-bit address space holds
    (ds.b $FFFFFFFF at offset 1): ds.b 4294967295 reserves 4294967295 bytes
        past the end of the 32-bit address space
```

Controls: `ds.b 3`, `ds.w 2`, `ds.l 0` reserve exactly their bytes; `ds.b
$FFFFFFFF` at offset 0 (byte total exactly `u32::MAX`) passes every check.

Red: `[11, 22]` for 2^32; `attempt to multiply with overflow` for `ds.w 2^31`;
`attempt to add with overflow` for the cursor row. Green after: 6 passed.
Runner: `cargo test -p sigil-frontend-as --test as_ds_range`.

## 3. abs.w window, the four routes (commit `653ee405`)

Reproduction on the committed release binary, exit 0, no diagnostic:

```text
    move.w  d0,($C00004).w       31C0 0004      a store to $000004
    lea     ($C00004).w,a0       41F8 0004
    move.l  #Later,($C00004).w   21FC 00000008 0004
```

The four routes, named by the function that builds the operand:

1. `convert_one_atom_m68k`, `OperandAtom::M68kAbs` arm: the generic eager fold
   for a resolved `(addr).w`. UNCHECKED (`(v & 0xFFFF) as i16`).
2. `try_defer_long_imm`, `[Imm, M68kAbs]` arm: `move.l #Sym,(addr).w` on the
   deferral pass, destination folded eagerly. UNCHECKED, same cast.
3. `try_defer_lea_abs`: `lea (Sym).w,aN` unresolved until link, an `Abs16Be`
   fixup the linker range-checks. Checked.
4. The `jmp`/`jsr (Sym).w` deferral in `lower_m68k`: the same fixup. Checked.

So the review's "fourth path" is two paths. The window already had two
spellings: `sigil_ir::asl_width_rule` (low 24 bits) and a narrower literal in
the linker's `Abs16Be` arm that refused a 25-bit address asl accepts.

asl: `($C00004).w` and `($8000).w` on `move.w`, `lea`, `move.l #imm` and `jmp`
are `error #1340: short addressing not allowed`, exit 2. `($FFFF8000).w` and
`($FF8000).w` assemble to `8000`, `($7FFF).w` to `7FFF`, `($1007FFF).w` to
`7FFF`, exit 0: the test is on the low 24 bits.

Fix: `sigil_ir::fits_abs_w(v)`, derived from `asl_width_rule`, is the single
predicate. Routes 1 and 2 ask it through `Asm::abs_w_operand`; the linker's
`Abs16Be` arm asks it too, so routes 3 and 4 share the check. The linker now
agrees with asl on the 25-bit row. Diagnostic:

```text
    absw.asm(2): error: address $C00004 does not fit abs.w: only $0..=$7FFF and
        $FF8000..=$FFFFFF have a short spelling (asl: short addressing not
        allowed); use .l
```

Red (only `tests/as_abs_w_window.rs` present): route 1 `[31, C0, 00, 04]`,
route 2 `[21, FC, 00, 00, 00, 08, 00, 04, 00]`; routes 3 and 4 green before
and after (the witnesses of the shared arm). Green after: 5 passed, plus
`sigil-ir width::tests::fits_abs_w_agrees_with_the_width_rule_at_every_edge`
and `sigil-link tests::abs16be_window_is_the_shared_24_bit_predicate`.

Follow-up `a35e1092`: `try_defer_long_imm` folded the destination before
deciding whether to defer, so a resolved immediate fell through to the eager
path and the same refusal printed twice. The decision now runs first.
Byte-neutral (order only); AS suite 625/0.

## 4. Z80 symbolic `jp`/`call`/`ld rr` (commit `bbaf6bcd`)

Reproduction on the committed release binary, exit 0, no diagnostic:

```text
    Big equ 10000h  / jp Big        C3 00 00     (a jump to $0000)
    Big1 equ 12345h / call Big1     CD 45 23
                      ld hl,Big1    21 45 23
    Neg equ -1      / jp Neg        C3 FF FF
```

Resolved side: `Imm16(v as u16)`. Unresolved side: a `BankPtr16Le` fixup whose
linker arm is `value as u16`, where the `.emp` twin `lower_z80_abs16_sym` and
the AS `dw` deferral both emit the range-checked `Value16Le`.

asl: `jp Big`, `call Big1`, `ld hl,Big1`, `jp Neg` are each `error #1320:
range overflow`, exit 2. `jp 0FFFFh` / `call 0FFFFh` / `ld hl,0FFFFh` /
`ld hl,-32768` assemble to `C3 FF FF` / `CD FF FF` / `21 FF FF` / `21 00 80`,
exit 0. (Also seen: asl refuses a Z80 PC past `$FFFF` with `error #1925:
address overflow`; not in scope.)

Fix: the resolved arms take the literal paths' windows through `fold_imm`
(`Z80_ADDR_RANGE` `0..=$FFFF` for jp/call, `WORD_DATA_RANGE` for `ld rr,nn`);
`Z80Backend::lower_abs16` emits `Value16Le`. Diagnostics:

```text
    s_z80_1.asm(3): error: operand 65536 out of range 0..=65535
    s_z80_6.asm(3): error: operand -1 out of range 0..=65535
    (link)  [value.out-of-range] link-expr value 65536 does not fit a 16-bit cell
```

A negative link-time target is refused by `Value16Le` where the resolved
`ld rr` arm accepts `-$8000..`; link-time Z80 symbols are labels, so the
asymmetry has no population, and the `.emp` twin already has it.

Red: `[C3, 00, 00]`, `[21, 45, 23]`, `[C3, FF, FF]`, and the linker accepting
`$10000`. Green after: 5 passed (`as_z80_abs16_range`), backend test
`abs16_emits_value16le_fixup_at_last_two_bytes`.

## 5. `.emp` `bank:` (commit `7223d871`)

Reproduction on the committed release binary:

```text
    section s (bank: $100000000) { data X: u8 = 0 }
    e_bank_2p32.emp:3:5: section `s` (0x1 bytes) cannot fit a 0x0 bank, over
        by 1 bytes                                              exit 1
```

Loud, but from the placer, about a `0x0 bank` nobody wrote; the lowering test
shows the producer side: `diags [], Section.bank Some(0)`.

Fix: a power of two above `u32::MAX` is refused at the attribute:

```text
    e_bank_2p32.emp:2:18: section `s` `bank:` $100000000 exceeds the 32-bit
        address space (largest bank: $80000000)
```

`$80000000` is accepted and arrives intact. Red: `diags [], section bank
Some(Some(0))` for `$100000000`, `$200000000`, `1 << 40`. Green after:
`banks.rs` 28 passed. No asl row (no AS surface).

## 6. Comptime vs link arithmetic (commit `0aaf0012`)

`sigil_ir::Expr::fold` is the one evaluator behind the AS front end at assembly
time and the linker at link time. Reproduction on the committed release binary:

```text
    move.w  #5/0,d0               303C 0000    exit 0
    dc.l    (1<<64)+5             00000006     exit 0
    dc.l    ($100<<62)>>62        00000000     exit 0
    dc.l    $7FFFFFFFFFFFFFFF*2   FFFFFFFE     exit 0
    dc.l    5/0                   "unresolved long expression", exit 1
```

The first is a silent zero: `Asm::fold_imm` reads a nameless `Poison` as a 0
placeholder and records nothing. At link, `(L << 62) >> 62` for `L = $100`
wrote `00000000`, and `L / 0` said `unresolved target expression`.

asl: `dc.l 5/0` is `error #1310: division by 0`, exit 2. On the overflow rows
asl is silently wrong the same way: `00000006`, `00000000`, `FFFFFFFE`, exit 0,
which is C's undefined signed overflow observed on x86-64, not a semantics. Not
matched.

The link-time wrapping site set (`grep wrapping_` in sigil-ir and sigil-link):

| site | wrapping | call |
|---|---|---|
| `sigil-ir/src/expr.rs` `Expr::fold`: Neg, Add, Sub, Mul, Div, Mod, Shl, Shr | silent overflow; shift amounts were masked to 6 bits (`1 << 64` was 1) | CHANGED |
| `sigil-ir/src/align.rs` `asl_align_pad`: add/sub/rem on the PC | two's-complement address arithmetic measured against asl over 30 rows; the RAM-side overshoot depends on it | unchanged |
| `sigil-link/src/lib.rs` checksum: add on u16 | the Sega header checksum is the 16-bit sum mod 2^16 by definition | unchanged |

Fix: `Fold::Fault(ArithFault)` carries the operator and operand values; every
consumer names the arithmetic. Rules are the comptime evaluator's on i64:
checked add/sub/mul/div/rem/neg; shift amount in `0..=63`; a left shift must
round-trip, so `1 << 63` is an overflow rather than `i64::MIN`; `>>` stays
arithmetic. A fault outranks a poison on the other side of an operator.
Diagnostics:

```text
    s_arith8.asm(2): error: division by zero: 5 / 0
    s_arith1.asm(2): error: shift amount 64 out of range 0..=63 in 1 << 64
    s_arith1.asm(3): error: arithmetic overflow: 256 << 62 does not fit a 64-bit value
    s_arith1.asm(4): error: arithmetic overflow: 9223372036854775807 * 2 does not fit a 64-bit value
    (link)  [value.fault] arithmetic overflow: 256 << 62 does not fit a 64-bit
            value for fixup in section s at offset 0
```

Consumers given a Fault arm: linker `apply_fixup`, both equ folds, the link
assertion condition and message parts, the three relax target folds; AS
`fold_imm`, `eval_all`, `dc.b`/`dw`/`dc.w`/`dc.l` (fault reported, width-sized
zero placeholder keeps the pass shape), the jmp/jsr width fold,
`abs_ea_from_expr`, the Z80 symbolic arms, `fold_const` (refuses as `None` by
design), `partial_fold` (the tree travels to the linker).

Controls asl folds without wrapping keep their bytes: `1<<31`, `$FFFF<<16`,
`-1<<31`, `$7FFFFFFF+1`, `(-1)>>1`, `($80000000<<1)>>1`, `6*7`, `-5#3`.

Red: `[30, 3C, 00, 00]`, `[00, 00, 00, 06]`, `[00, 00, 00, 00]`,
`[FF, FF, FF, FE]`; linker accepted the overflowing expression. Green after:
`as_arith_fault` 6 passed, `sigil-link tests::value_fixup_refuses_link_time_arithmetic_faults`,
sigil-ir expr tests for every fault kind, each edge's inside value, and
fault-over-poison.

## aeon exposure

- `bank:` in `.emp` (`git grep 'bank:' origin/master -- '*.emp'`): four
  sections, all `bank: $8000`. Cannot fire.
- The three `.asm` files (`engine/debug/debugger.asm`, `games/demo/game_root.asm`,
  `games/sonic4/game_root.asm`): `!align 2` only (debugger.asm, ten sites); no
  `ds.`; two `).w` forms, both in debugger.asm (`(MemFlag).w` in a comment,
  `(.__dval).w` in a macro body on a RAM-window address); no Z80 code; no shift
  by 32 or more; no division by a zero constant. None of the new refusals has a
  population there. The controller proves the four ROM shapes at the landing
  gate.

## Totals (CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-ctu, `--no-fail-fast`)

| crate | passed | failed | ignored | failing names |
|---|---|---|---|---|
| sigil-ir | 52 | 0 | 0 | |
| sigil-link | 136 | 0 | 0 | |
| sigil-backend-z80 | 6 | 0 | 0 | |
| sigil-backend-m68k | 19 | 0 | 0 | |
| sigil-frontend-as | 625 | 0 | 0 | |
| sigil-frontend-emp (SIGIL_ALLOW_PARTIAL=1) | 2643 | 0 | 0 | |
| sigil-cli (SIGIL_ALLOW_PARTIAL=1) | 695 | 2 | 1 | `image_bounds::org_minus_one_is_refused_not_a_four_gib_allocation`, `image_bounds::org_minus_one_with_hex_is_refused_before_rendering` |

Without `SIGIL_ALLOW_PARTIAL=1` two sigil-frontend-emp rows
(`cfg_blind_spots::corpus_computed_dispatch_census_is_six_sites_five_procs`,
`no_out_declaring_proc_carries_a_targets_dispatch`) refuse for want of a named
reference tree, the expected shape here; reference-dependent rows were left
unmeasured under the partial declaration.

The two sigil-cli failures are PRE-EXISTING: with the baseline `99aa2645`
crates checked out into this worktree's working tree they fail identically
(`relax.rs:193: attempt to add with overflow`, the debug binary's `base +
advance` for `org -1` before the image-bounds refusal the test expects). They
landed today in `f28aaa33` from another lane. Same class (a wrapped add in
release lets the later check run; in debug it panics), left to that lane since
the test pins the later check's message.

Clippy `--all-targets -- -D warnings`: exit 0 on sigil-ir, sigil-link,
sigil-frontend-as, sigil-frontend-emp, sigil-backend-z80.

## Findings beyond the brief

- `move.w #5/0,d0` assembled to `303C 0000` with exit 0 (a nameless `Poison`
  read as a placeholder). Fixed under site 6; it was the worst shape in the set.
- The abs.w "fourth path" is two paths (routes 1 and 2 above).
- The linker's `Abs16Be` window was narrower than asl on 25-bit addresses;
  unified on the 24-bit predicate.
- asl is silently wrong on `ds.w $80000000`, `ds.b -1`, `ds.b $FFFFFFFF`,
  `align -256` (assembles as `align $FF00`), and every i64 overflow row.

## Left open

- `.emp` `pub equ E = 1 << 64` lowers through `Expr::Int(n as i64)`
  (`lower/mod.rs:823`, also `:830` and `:1024`): a comptime i128 that does not
  fit i64 reaches the link symbol table truncated. Not fixed here: outside the
  six rows and the brief scoped `.emp` edits to the `bank:` path. Same class.
- The `.emp` `CodeOperand::AbsInt` arm (`lower/code.rs:1443`) spells the abs.w
  window itself rather than through `fits_abs_w`; correct today, a second
  spelling. Same scoping reason.
- `BankPtr16Le`/`BankPtr16Be` keep `value as u16`: the masking address kind for
  a windowed bank pointer (`winptr`), where the mask is the semantics.
- A link-time `>> n` with `n` in `64..128` is refused here where the comptime
  i128 evaluator answers it (sign fill). A refusal, not a value; no population.
