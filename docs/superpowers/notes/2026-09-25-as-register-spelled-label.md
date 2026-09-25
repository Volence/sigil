# AS-REGISTER-SPELLED-LABEL-SILENT: asl's rule for a symbol spelled like a register

Parcel row: `docs/QUEUE.md` `### AS-REGISTER-SPELLED-LABEL-SILENT`. Origin: probe `i09`
of `2026-09-25-s3k-dollar-labels.md` (`A1:` then `move.w #$$x-A1,d0`: asl exit 3,
`#10000 internal error`; sigil assembled it with a value).

## Instrument

- Oracle: the pinned asl, md5 `61e672562465725a8c102288a7da9098`, through `asl_run` in
  `asl-reference/asl_ref.sh` (digest checked on every run), flags `-xx -n -q -A -L -U`.
- Probes: `2026-09-25-as-register-spelled-label/probes/*.asm`, written by `gen_probes.py`
  (`probes/INDEX.tsv` names each shape). Every probe is `cpu 68000` (or `cpu z80`) and
  `org $1200` (`org 1200h` under Z80, whose default integer syntax is Intel: `org $1200`
  there is asl `#1020 invalid symbol name` and sigil `trailing tokens in expression`, so
  a `$` header made every Z80 probe a both-refuse that measured nothing).
- Runner: `run_probes.sh <sigil> <out.tsv> <work-dir>`. asl bytes are read (p2bin over the
  `.p`) ONLY from a run that exited 0; any refused run's byte column is `-`. Bytes are from
  `$1200` on.
- `before.tsv`: sigil at master `1d19e60b` (release build of this branch before any
  change, `sigil --version` revision `1d19e60b`) against the pinned asl, 127 probes.
  `after.tsv`: the same 127 under the fix (`sigil --version` revision `63a678a5`,
  clean tree).

## asl's rule, measured

1. **68000: in an expression, a name spelled `d0`..`d7`, `a0`..`a7` or `sp`, in any case,
   is the register, whatever symbol of that name exists.** A label, `equ` or `set` of that
   spelling is accepted as a DEFINITION (`def_*`, `def_A1_equ`, `def_A1_set`: exit 0) and
   `ifdef A1` / `defined(A1)` see it (`lbl_ifdef`, `lbl_defined`: true), but no expression
   can read its value: `#A1`, `#A1+2`, `#Lab-A1`, `#(A1)`, `dc.w A1+2`, `A1(a0)`,
   `A1(pc)`, `bra.w A1`, `org A1+8`, `if A1=$1200`, `X set A1+2`, `ds.b A1-$11F0` are all
   refused (`#1145 ... but got register`, `#1146 expected integer or string`, or
   `#10000 internal error` for a difference of a symbol and a register). The case of the
   definition and of the use do not matter to this (`lbl_a1_*`, `case_*`), although
   symbols themselves are case-sensitive under `-U`.
2. **Only those spellings.** `USP:`, `SR:`, `CCR:`, `PC:` labels read as ordinary values
   (`lbl_USP_*`, `lbl_SR_*`, `lbl_CCR_*`, `lbl_PC_*`: same bytes in both), and so do names
   that merely contain a register spelling (`A1x`, `XA1`, `A1_2`, `A10`, `A8`, `D8`,
   `SP2`, `A1A1`, the local `A1.l`, the temp `$$A1`). Under `cpu z80` no name is a
   register in an expression: `hl:`, `a:`, `ix:`, `sp:`, `bc:`, `af:` labels read as their
   values in `dw` and `ld de,X+2` (the `2026-09-05` `r07_z80` measurement, reproduced).
3. **A register where a whole operand stands is an addressing mode, not a value.**
   `move.w A1,d0` is `3009` (register direct) with `A1:` defined; `jsr A1` is
   `#1350 addressing mode not allowed here`. And a data directive whose operand is a lone
   register SILENTLY EMITS NOTHING, exit 0: `dc.w A1`, `dc.l A1`, `dc.b A1,0` (only the
   `00`), `dc.w Lab,A1,Lab` (only the two `Lab` words), and `X equ A1` then `dc.w X`
   (asl makes `X` a register alias). That last class is the standing divergence
   `crates/sigil-frontend-as/tests/as_register_in_value_position.rs` already records and
   refuses on purpose (a `dc` that emits zero bytes is the silent-wrong-answer class).

## Verdicts, before and after (127 probes)

| verdict | before (`1d19e60b`) | after (`63a678a5`) |
|---|---|---|
| same | 58 | 58 |
| both-refuse | 11 | 58 |
| OVER-ACCEPT | 47 | 0 |
| OVER-REFUSE | 5 | 11 |
| BYTES-DIFFER | 6 | 0 |

Transitions: 47 OVER-ACCEPT to both-refuse (every one refused with the new
shadowed-register sentence as its first error), 6 BYTES-DIFFER to OVER-REFUSE, and the
58 same, 11 both-refuse and 5 OVER-REFUSE rows unchanged. `same` compares sigil's bytes
with asl's, so the 58 accepted shapes still produce asl's bytes.

**Sizing.** All 47 over-accepts are rule 1 and nothing else: every one is a register
spelling that names a defined symbol, read in value position, and asl's diagnostic is one
of the three register-value classes (30 `#1145`, 9 `#1146`, 7 `#10000`) plus `jsr A1`'s
`#1350` (rule 3's bare-operand reading, which falls out of rule 1 because sigil reads the
bare name as an absolute address only when it resolves). Sigil already applies rule 1 to
a register spelling with NO symbol behind it (`ctl_nodef_imm`, `ctl_nodef_imm_plus`, `ctl_nodef_dcw_plus` both refuse with sigil's
`is a register, not a value`); the defect is only that a defined symbol of that spelling
wins over the register. One rule, so this parcel fixes it whole.

The 5 OVER-REFUSE and 6 BYTES-DIFFER rows are two other classes. Eight are rule 3's
silent-empty `dc`; three are asl's register ALIAS (`X equ d3`, `X reg d3`, and `X equ A1`
with `A1:` defined: asl makes `X` a register, `move.w X,d0` is `3003` / `3009`), which
sigil has no symbol kind for. The `dc` OVER-REFUSE rows (`ctl_nodef_dcl`, `ctl_nodef_dcw_multi`, `dc_b`) are the
standing deliberate refusal; `alias_equ_d3` and `alias_reg_d3` are sigil refusing the
alias by name (`is a register, not a value`; `reg` is not a recognized mnemonic). The BYTES-DIFFER rows (`lbl_dcw`, `lbl_dcl`, `equ_A1_dcw`,
`lbl_equ_rhs`, `dc_multi`) are the `dc` shape with a symbol of the register's spelling in
scope: sigil emits the symbol's value where asl emits nothing. They are not a separate
rule for this parcel to adopt: once rule 1 holds, they join the standing refusal, and
they do (after table). This is a REFUSAL of a shape asl accepts, reported rather than
silent: asl's answer for these lines is zero bytes, which the standing ruling declines
to reproduce, and the alternative (keep writing the symbol's value) is a byte
disagreement with asl at exit 0. The sixth BYTES-DIFFER row, `alias_equ_A1_label`, was
worse than a byte count: sigil wrote `move.w X,d0` as `3038 1200` (absolute, the label's
address) where asl writes `3009` (register direct), and it now refuses the `equ` line by
name like the other two alias rows. Register aliases are booked in `docs/QUEUE.md` as
their own row (`AS-REGISTER-ALIAS-SYMBOL`) rather than grown into this parcel.

The Z80 rows all agree (`same` or both-refuse), so no Z80 change.

## The fix

`crates/sigil-frontend-as/src/eval.rs`: `reads_as_register` (the 68000 set of
`is_expr_register_name`) is asked first by the three expression readers, `fold`,
`unresolved_names` and `parse_num_atom`, so a register spelling folds to Poison whatever
the symbol table holds and the existing register-in-value-position reporting fires at
every consumer that already had it. The one consumer that had not: `fixup_target`, which
hands a Poison branch or pc-relative target to the linker symbolically, and the linker
resolved `A1` as the label (`bra.w A1`, `A1(pc)`, `A1(pc,d0.w)`, `movem.l A1(pc),...`).
It now reports the register with the line's span and emits nothing. With a symbol of the
spelling in scope the message says so: "`A1` is a register, not a value: expected an
integer, floating point number or string. A symbol named `A1` is defined, but in an
expression that spelling is the 68000 register, as it is to asl, so the symbol's value
cannot be read here: rename the symbol". The definition side, `ifdef`, `defined()`,
`pushv`/`popv` and the symbol-name tests do not ask.

## Tests

- `tests/as_register_spelled_label.rs`: every refusal consumer by exact sentence and
  line (18 rows plus six spellings, an `equ`, a macro-body label and a read above the
  definition), the booked `i09`, and 14 accept-side shapes with asl's bytes, each from a
  probe asl exited 0 on.
- `tests/over_acceptance/`: 27 `reg_*` probes, verdicts re-minted (the 88 existing rows
  unchanged), `reg_label_dcw_lone` ledgered as over-refusal under the standing ruling;
  floors 115 probes, 44 classes, 69 agreed refusals, 34 agreed acceptances.
- Mutation 1, `reads_as_register` made to return `false` (the fail-open form): the four
  refusal tests of `as_register_spelled_label.rs` red (`i09: asl refuses this, sigil
  assembled 4616 bytes`), plus `no_unledgered_over_acceptance`,
  `feed_control_the_ledger_is_not_an_escape_hatch` and `as_dollar_labels`' G10 row;
  the accept test stays green. Restored with `git show HEAD:... >`, all green.
- Mutation 2, the `fixup_target` register check removed with the fold gate kept:
  `every_consumer_of_a_value_reads_the_register` reds on `pc-relative: asl refuses this,
  sigil assembled 4616 bytes`. Restored, green.

## The fix

`crates/sigil-frontend-as/src/eval.rs`: `reads_as_register` (the 68000 set of
`is_expr_register_name`) is asked first by the three expression readers, `fold`,
`unresolved_names` and `parse_num_atom`, so a register spelling folds to Poison whatever
the symbol table holds and the existing register-in-value-position reporting fires at
every consumer that already had it. The one consumer that had not: `fixup_target`, which
hands a Poison branch or pc-relative target to the linker symbolically, and the linker
resolved `A1` as the label (`bra.w A1`, `A1(pc)`, `A1(pc,d0.w)`, `movem.l A1(pc),...`).
It now reports the register with the line's span and emits nothing. With a symbol of the
spelling in scope the message says so: "`A1` is a register, not a value: expected an
integer, floating point number or string. A symbol named `A1` is defined, but in an
expression that spelling is the 68000 register, as it is to asl, so the symbol's value
cannot be read here: rename the symbol". The definition side, `ifdef`, `defined()`,
`pushv`/`popv` and the symbol-name tests do not ask.

## Tests

- `tests/as_register_spelled_label.rs`: every refusal consumer by exact sentence and
  line (18 rows plus six spellings, an `equ`, a macro-body label and a read above the
  definition), the booked `i09`, and 14 accept-side shapes with asl's bytes, each from a
  probe asl exited 0 on.
- `tests/over_acceptance/`: 27 `reg_*` probes, verdicts re-minted (the 88 existing rows
  unchanged), `reg_label_dcw_lone` ledgered as over-refusal under the standing ruling;
  floors 115 probes, 44 classes, 69 agreed refusals, 34 agreed acceptances.
- Mutation 1, `reads_as_register` made to return `false` (the fail-open form): the four
  refusal tests of `as_register_spelled_label.rs` red (`i09: asl refuses this, sigil
  assembled 4616 bytes`), plus `no_unledgered_over_acceptance`,
  `feed_control_the_ledger_is_not_an_escape_hatch` and `as_dollar_labels`' G10 row;
  the accept test stays green. Restored with `git show HEAD:... >`, all green.
- Mutation 2, the `fixup_target` register check removed with the fold gate kept:
  `every_consumer_of_a_value_reads_the_register` reds on `pc-relative: asl refuses this,
  sigil assembled 4616 bytes`. Restored, green.

## Probe table, before (`1d19e60b`)

| probe | shape | asl | sigil | verdict |
|---|---|---|---|---|
| `alias_equ_A1_label` | `A1:` label, `X equ A1`, `move.w X,d0` | exit 0, `4e713009` | exit 0, `4e7130381200` | BYTES-DIFFER |
| `alias_equ_d3` | `X equ d3` then `move.w X,d0` (a register alias) | exit 0, `3003` | exit 1, `` `d3` is a register, not a value: expected an integer, floating point number or string `` | OVER-REFUSE |
| `alias_reg_d3` | `X reg d3` then `move.w X,d0` | exit 0, `3003` | exit 1, `` `reg` is not a recognized 68000 mnemonic `` | OVER-REFUSE |
| `case_a1_read_A1` | `a1:` label, `#A1+2` (other case) | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `case_A1_read_a1` | `A1:` label, `#a1+2` (other case) | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `a1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `cont_A10` | `A10:` label, `#A10+2` and `dc.w A10` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A1_2` | `A1_2:` label, `#A1_2+2` and `dc.w A1_2` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A1A1` | `A1A1:` label, `#A1A1+2` and `dc.w A1A1` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A1_dotw` | `A1:` label, `#A1.w` | exit 2, `` #1010 symbol undefined `` | exit 1, `` unresolved symbol `A1.w` in operand `` | both-refuse |
| `cont_A1_local` | `A1:` then `.l:`, read `A1.l` | exit 0, `4e714e71303c12041202` | exit 0, `4e714e71303c12041202` | same |
| `cont_A1x` | `A1x:` label, `#A1x+2` and `dc.w A1x` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A8` | `A8:` label, `#A8+2` and `dc.w A8` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_D8` | `D8:` label, `#D8+2` and `dc.w D8` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_dollar_A1` | `$$A1:` temp label, read | exit 0, `4e714e71303c12041202` | exit 0, `4e714e71303c12041202` | same |
| `cont_SP2` | `SP2:` label, `#SP2+2` and `dc.w SP2` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_XA1` | `XA1:` label, `#XA1+2` and `dc.w XA1` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `ctl_nodef_dcl` | control: no `A1` defined, `dc.l A1` | exit 0, `4e71` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | OVER-REFUSE |
| `ctl_nodef_dcw_multi` | control: no label, `dc.w Lab,d3,Lab` | exit 0, `4e7112001200` | exit 1, `` `d3` is a register, not a value: expected an integer, floating point number or string `` | OVER-REFUSE |
| `ctl_nodef_dcw_plus` | control: no `A1` defined, `dc.w A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `ctl_nodef_imm` | control: no label, `#A1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `ctl_nodef_imm_plus` | control: no `A1` defined, `#A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `dc_b` | `A1:` label, `dc.b A1` | exit 0, `4e7100` | exit 1, `` operand 4608 out of range -128..=255 `` | OVER-REFUSE |
| `dc_multi` | `A1:` label, `dc.w Lab,A1,Lab` | exit 0, `4e714e7112021202` | exit 0, `4e714e71120212001202` | BYTES-DIFFER |
| `def_a1` | `a1:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_A1` | `A1:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_A1_equ` | `A1 equ 5`, never read | exit 0, `4e71` | exit 0, `4e71` | same |
| `def_A1_set` | `A1 set 5`, never read | exit 0, `4e71` | exit 0, `4e71` | same |
| `def_A7` | `A7:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_CCR` | `CCR:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_d0` | `d0:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_D0` | `D0:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_PC` | `PC:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_sp` | `sp:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_SP` | `SP:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_SR` | `SR:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_USP` | `USP:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `ds_count` | `A1:` label, `ds.b A1-$11F0` | exit 3, `` #10000 internal error `` | exit 0, `4e71` | OVER-ACCEPT |
| `equ_A1_dcw` | `A1 equ 5`, `dc.w A1` | exit 0, `(empty)` | exit 0, `0005` | BYTES-DIFFER |
| `equ_A1_dcw_plus` | `A1 equ 5`, `dc.w A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `0007` | OVER-ACCEPT |
| `equ_A1_if` | `A1 equ 5`, `if A1=5` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71` | OVER-ACCEPT |
| `equ_A1_imm` | `A1 equ 5`, `#A1` | exit 2, `` #1146 expected integer or string `` | exit 0, `303c0005` | OVER-ACCEPT |
| `equ_A1_imm_plus` | `A1 equ 5`, `#A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `303c0007` | OVER-ACCEPT |
| `i09` | booked i09 shape: `#$$x-A1` | exit 3, `` #10000 internal error `` | exit 0, `4e714e71303c0002` | OVER-ACCEPT |
| `i09_nodollar` | i09 without `$$`: `#B-A1` | exit 3, `` #10000 internal error `` | exit 0, `4e714e71303c0002` | OVER-ACCEPT |
| `lbl_a1_dcw_plus` | `a1:` label, `dc.w a1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e711202` | OVER-ACCEPT |
| `lbl_a1_imm_plus` | `a1:` label, `#a1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `lbl_A7_dcw_plus` | `A7:` label, `dc.w A7+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e711202` | OVER-ACCEPT |
| `lbl_A7_imm_plus` | `A7:` label, `#A7+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `lbl_absl` | `A1:` label, `move.w (A1).l,d0` | exit 2, `` #1146 expected integer or string `` | exit 1, `` trailing tokens in operand `` | both-refuse |
| `lbl_abs_plus` | `A1:` label, `move.w A1+2,d0` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e7130381202` | OVER-ACCEPT |
| `lbl_absw` | `A1:` label, `move.w (A1).w,d0` | exit 2, `` #1146 expected integer or string `` | exit 1, `` trailing tokens in operand `` | both-refuse |
| `lbl_bare_jsr` | `A1:` label, `jsr A1` | exit 2, `` #1350 addressing mode not allowed here `` | exit 0, `4e714e714eb81200` | OVER-ACCEPT |
| `lbl_bare_lea` | `A1:` label, `lea A1,a0` | exit 2, `` #1350 addressing mode not allowed here `` | exit 1, `` unsupported form: An is not a legal addressing mode for this operand position `` | both-refuse |
| `lbl_bare_move` | `A1:` label, `move.w A1,d0` (bare: register direct?) | exit 0, `4e714e713009` | exit 0, `4e714e713009` | same |
| `lbl_bra` | `A1:` label, `bra.w A1` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e716000fffa` | OVER-ACCEPT |
| `lbl_CCR_dcw_plus` | `CCR:` label, `dc.w CCR+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_CCR_imm_plus` | `CCR:` label, `#CCR+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `lbl_d0_dcw_plus` | `d0:` label, `dc.w d0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e711202` | OVER-ACCEPT |
| `lbl_D0_dcw_plus` | `D0:` label, `dc.w D0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e711202` | OVER-ACCEPT |
| `lbl_d0_imm_plus` | `d0:` label, `#d0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `lbl_D0_imm_plus` | `D0:` label, `#D0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `lbl_dbf` | `A1:` label, `dbf d0,A1` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e7151c8fffa` | OVER-ACCEPT |
| `lbl_dcb_plus` | `A1:` label, `dc.b A1-Lab+2` | exit 3, `` #10000 internal error `` | exit 0, `4e714e7100` | OVER-ACCEPT |
| `lbl_dcl` | `A1:` label, `dc.l A1` | exit 0, `4e714e71` | exit 0, `4e714e7100001200` | BYTES-DIFFER |
| `lbl_dcl_fwd` | `A1:` label, `dc.l A1+Fwd`, `Fwd` defined after | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e71000024084e71` | OVER-ACCEPT |
| `lbl_dcl_sub` | `A1:` label, `dc.l Lab-A1` | exit 3, `` #10000 internal error `` | exit 0, `4e714e7100000002` | OVER-ACCEPT |
| `lbl_dcw` | `A1:` label, `dc.w A1` | exit 0, `4e714e71` | exit 0, `4e714e711200` | BYTES-DIFFER |
| `lbl_dcw_plus` | `A1:` label, `dc.w A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e711202` | OVER-ACCEPT |
| `lbl_defined` | `A1:` label, `if defined(A1)` | exit 0, `4e714e714e71` | exit 0, `4e714e714e71` | same |
| `lbl_disp` | `A1:` label, `move.w A1(a0),d0` (displacement) | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e7130281200` | OVER-ACCEPT |
| `lbl_disp_plus` | `A1:` label, `move.w A1+2(a0),d0` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e7130281202` | OVER-ACCEPT |
| `lbl_equ_rhs` | `A1:` label, `X equ A1` then `dc.w X` | exit 0, `4e714e71` | exit 0, `4e714e711200` | BYTES-DIFFER |
| `lbl_if_cmp` | `A1:` label, `if A1=$1200` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e714e71` | OVER-ACCEPT |
| `lbl_ifdef` | `A1:` label, `ifdef A1` | exit 0, `4e714e714e71` | exit 0, `4e714e714e71` | same |
| `lbl_imm` | `A1:` label, `#A1` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e71303c1200` | OVER-ACCEPT |
| `lbl_imm_before_def` | `#A1+2` read ABOVE the `A1:` definition | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `303c12064e71` | OVER-ACCEPT |
| `lbl_imm_lab` | `A1:` label, `#A1+Lab` (z01's shape) | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e71323c2402` | OVER-ACCEPT |
| `lbl_imm_paren` | `A1:` label, `#(A1)` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e71303c1200` | OVER-ACCEPT |
| `lbl_imm_plus` | `A1:` label, `#A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e71303c1202` | OVER-ACCEPT |
| `lbl_imm_sub` | `A1:` label, `#Lab-A1` | exit 3, `` #10000 internal error `` | exit 0, `4e714e71303c0002` | OVER-ACCEPT |
| `lbl_jmp_absl` | `A1:` label, `jmp (A1+2).l` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e714ef900001202` | OVER-ACCEPT |
| `lbl_jsr_plus` | `A1:` label, `jsr A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e714eb81202` | OVER-ACCEPT |
| `lbl_lea_plus` | `A1:` label, `lea A1+2,a0` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e7141f81202` | OVER-ACCEPT |
| `lbl_movem_pc` | `A1:` label, `movem.l A1(pc),d0-d1` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e714cfa0003fff8` | OVER-ACCEPT |
| `lbl_org` | `A1:` label, `org A1+8` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e71000000004e71` | OVER-ACCEPT |
| `lbl_PC_dcw_plus` | `PC:` label, `dc.w PC+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_pcidx` | `A1:` label, `lea A1(pc,d0.w),a0` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e7141fb00fa` | OVER-ACCEPT |
| `lbl_PC_imm_plus` | `PC:` label, `#PC+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `lbl_pcrel` | `A1:` label, `lea A1(pc),a0` | exit 2, `` #1146 expected integer or string `` | exit 0, `4e714e7141fafffa` | OVER-ACCEPT |
| `lbl_rept` | `A1:` label, `rept A1-$11FF` | exit 3, `` #10000 internal error `` | exit 0, `4e714e714e71` | OVER-ACCEPT |
| `lbl_set_rhs` | `A1:` label, `X set A1+2` then `dc.w X` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e714e711202` | OVER-ACCEPT |
| `lbl_sp_dcw_plus` | `sp:` label, `dc.w sp+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e711202` | OVER-ACCEPT |
| `lbl_SP_dcw_plus` | `SP:` label, `dc.w SP+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e711202` | OVER-ACCEPT |
| `lbl_sp_imm_plus` | `sp:` label, `#sp+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `lbl_SP_imm_plus` | `SP:` label, `#SP+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `lbl_SR_dcw_plus` | `SR:` label, `dc.w SR+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_SR_imm_plus` | `SR:` label, `#SR+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `lbl_USP_dcw_plus` | `USP:` label, `dc.w USP+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_USP_imm_plus` | `USP:` label, `#USP+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `macro_body_label` | `A1:` defined inside a macro body, `#A1+2` in the same body | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `4e71303c1202` | OVER-ACCEPT |
| `set_a0_dcl` | `a0 set 5`, `dc.l a0+1` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 0, `00000006` | OVER-ACCEPT |
| `z80_ctl_dw_hl` | z80 control: no `hl` defined, `dw hl+2` | exit 2, `` #1010 symbol undefined `` | exit 1, `` unresolved target expression (dangling symbol(s) `hl`) for fixup in section sec4608 at off `` | both-refuse |
| `z80_def_a` | z80 `a:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_af` | z80 `af:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_bc` | z80 `bc:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_hl` | z80 `hl:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_ix` | z80 `ix:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_sp` | z80 `sp:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_dw_a` | z80 `a:` label, `dw a` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_af` | z80 `af:` label, `dw af` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_bc` | z80 `bc:` label, `dw bc` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_hl` | z80 `hl:` label, `dw hl` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_ix` | z80 `ix:` label, `dw ix` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dwplus_a` | z80 `a:` label, `dw a+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_af` | z80 `af:` label, `dw af+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_bc` | z80 `bc:` label, `dw bc+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_hl` | z80 `hl:` label, `dw hl+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_ix` | z80 `ix:` label, `dw ix+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_sp` | z80 `sp:` label, `dw sp+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dw_sp` | z80 `sp:` label, `dw sp` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_ld_hl_bare` | z80 `hl:` label, `ld hl,hl` | exit 2, `` #1500 instruction not supported on Z80 `` | exit 1, `` unsupported form: Instruction { mnemonic: Ld, ops: [Pair(Hl), Pair(Hl)] } `` | both-refuse |
| `z80_ldimm_a` | z80 `a:` label, `ld de,a+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_af` | z80 `af:` label, `ld de,af+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_bc` | z80 `bc:` label, `ld de,bc+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_hl` | z80 `hl:` label, `ld de,hl+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_ix` | z80 `ix:` label, `ld de,ix+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_sp` | z80 `sp:` label, `ld de,sp+2` | exit 0, `00110212` | exit 0, `00110212` | same |

## Probe table, after (`63a678a5`)

| probe | shape | asl | sigil | verdict |
|---|---|---|---|---|
| `alias_equ_A1_label` | `A1:` label, `X equ A1`, `move.w X,d0` | exit 0, `4e713009` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `alias_equ_d3` | `X equ d3` then `move.w X,d0` (a register alias) | exit 0, `3003` | exit 1, `` `d3` is a register, not a value: expected an integer, floating point number or string `` | OVER-REFUSE |
| `alias_reg_d3` | `X reg d3` then `move.w X,d0` | exit 0, `3003` | exit 1, `` `reg` is not a recognized 68000 mnemonic `` | OVER-REFUSE |
| `case_a1_read_A1` | `a1:` label, `#A1+2` (other case) | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `case_A1_read_a1` | `A1:` label, `#a1+2` (other case) | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `a1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `cont_A10` | `A10:` label, `#A10+2` and `dc.w A10` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A1_2` | `A1_2:` label, `#A1_2+2` and `dc.w A1_2` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A1A1` | `A1A1:` label, `#A1A1+2` and `dc.w A1A1` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A1_dotw` | `A1:` label, `#A1.w` | exit 2, `` #1010 symbol undefined `` | exit 1, `` unresolved symbol `A1.w` in operand `` | both-refuse |
| `cont_A1_local` | `A1:` then `.l:`, read `A1.l` | exit 0, `4e714e71303c12041202` | exit 0, `4e714e71303c12041202` | same |
| `cont_A1x` | `A1x:` label, `#A1x+2` and `dc.w A1x` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_A8` | `A8:` label, `#A8+2` and `dc.w A8` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_D8` | `D8:` label, `#D8+2` and `dc.w D8` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_dollar_A1` | `$$A1:` temp label, read | exit 0, `4e714e71303c12041202` | exit 0, `4e714e71303c12041202` | same |
| `cont_SP2` | `SP2:` label, `#SP2+2` and `dc.w SP2` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `cont_XA1` | `XA1:` label, `#XA1+2` and `dc.w XA1` | exit 0, `4e71303c12021200` | exit 0, `4e71303c12021200` | same |
| `ctl_nodef_dcl` | control: no `A1` defined, `dc.l A1` | exit 0, `4e71` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | OVER-REFUSE |
| `ctl_nodef_dcw_multi` | control: no label, `dc.w Lab,d3,Lab` | exit 0, `4e7112001200` | exit 1, `` `d3` is a register, not a value: expected an integer, floating point number or string `` | OVER-REFUSE |
| `ctl_nodef_dcw_plus` | control: no `A1` defined, `dc.w A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `ctl_nodef_imm` | control: no label, `#A1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `ctl_nodef_imm_plus` | control: no `A1` defined, `#A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| `dc_b` | `A1:` label, `dc.b A1` | exit 0, `4e7100` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `dc_multi` | `A1:` label, `dc.w Lab,A1,Lab` | exit 0, `4e714e7112021202` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `def_a1` | `a1:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_A1` | `A1:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_A1_equ` | `A1 equ 5`, never read | exit 0, `4e71` | exit 0, `4e71` | same |
| `def_A1_set` | `A1 set 5`, never read | exit 0, `4e71` | exit 0, `4e71` | same |
| `def_A7` | `A7:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_CCR` | `CCR:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_d0` | `d0:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_D0` | `D0:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_PC` | `PC:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_sp` | `sp:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_SP` | `SP:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_SR` | `SR:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `def_USP` | `USP:` defined, never read | exit 0, `4e714e71` | exit 0, `4e714e71` | same |
| `ds_count` | `A1:` label, `ds.b A1-$11F0` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `equ_A1_dcw` | `A1 equ 5`, `dc.w A1` | exit 0, `(empty)` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `equ_A1_dcw_plus` | `A1 equ 5`, `dc.w A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `equ_A1_if` | `A1 equ 5`, `if A1=5` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `equ_A1_imm` | `A1 equ 5`, `#A1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `equ_A1_imm_plus` | `A1 equ 5`, `#A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `i09` | booked i09 shape: `#$$x-A1` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `i09_nodollar` | i09 without `$$`: `#B-A1` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_a1_dcw_plus` | `a1:` label, `dc.w a1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `a1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_a1_imm_plus` | `a1:` label, `#a1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `a1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_A7_dcw_plus` | `A7:` label, `dc.w A7+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A7` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_A7_imm_plus` | `A7:` label, `#A7+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A7` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_absl` | `A1:` label, `move.w (A1).l,d0` | exit 2, `` #1146 expected integer or string `` | exit 1, `` trailing tokens in operand `` | both-refuse |
| `lbl_abs_plus` | `A1:` label, `move.w A1+2,d0` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_absw` | `A1:` label, `move.w (A1).w,d0` | exit 2, `` #1146 expected integer or string `` | exit 1, `` trailing tokens in operand `` | both-refuse |
| `lbl_bare_jsr` | `A1:` label, `jsr A1` | exit 2, `` #1350 addressing mode not allowed here `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_bare_lea` | `A1:` label, `lea A1,a0` | exit 2, `` #1350 addressing mode not allowed here `` | exit 1, `` unsupported form: An is not a legal addressing mode for this operand position `` | both-refuse |
| `lbl_bare_move` | `A1:` label, `move.w A1,d0` (bare: register direct?) | exit 0, `4e714e713009` | exit 0, `4e714e713009` | same |
| `lbl_bra` | `A1:` label, `bra.w A1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_CCR_dcw_plus` | `CCR:` label, `dc.w CCR+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_CCR_imm_plus` | `CCR:` label, `#CCR+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `lbl_d0_dcw_plus` | `d0:` label, `dc.w d0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `d0` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_D0_dcw_plus` | `D0:` label, `dc.w D0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `D0` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_d0_imm_plus` | `d0:` label, `#d0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `d0` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_D0_imm_plus` | `D0:` label, `#D0+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `D0` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_dbf` | `A1:` label, `dbf d0,A1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_dcb_plus` | `A1:` label, `dc.b A1-Lab+2` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_dcl` | `A1:` label, `dc.l A1` | exit 0, `4e714e71` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `lbl_dcl_fwd` | `A1:` label, `dc.l A1+Fwd`, `Fwd` defined after | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_dcl_sub` | `A1:` label, `dc.l Lab-A1` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_dcw` | `A1:` label, `dc.w A1` | exit 0, `4e714e71` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `lbl_dcw_plus` | `A1:` label, `dc.w A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_defined` | `A1:` label, `if defined(A1)` | exit 0, `4e714e714e71` | exit 0, `4e714e714e71` | same |
| `lbl_disp` | `A1:` label, `move.w A1(a0),d0` (displacement) | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_disp_plus` | `A1:` label, `move.w A1+2(a0),d0` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_equ_rhs` | `A1:` label, `X equ A1` then `dc.w X` | exit 0, `4e714e71` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | OVER-REFUSE |
| `lbl_if_cmp` | `A1:` label, `if A1=$1200` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_ifdef` | `A1:` label, `ifdef A1` | exit 0, `4e714e714e71` | exit 0, `4e714e714e71` | same |
| `lbl_imm` | `A1:` label, `#A1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_imm_before_def` | `#A1+2` read ABOVE the `A1:` definition | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_imm_lab` | `A1:` label, `#A1+Lab` (z01's shape) | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_imm_paren` | `A1:` label, `#(A1)` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_imm_plus` | `A1:` label, `#A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_imm_sub` | `A1:` label, `#Lab-A1` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_jmp_absl` | `A1:` label, `jmp (A1+2).l` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_jsr_plus` | `A1:` label, `jsr A1+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_lea_plus` | `A1:` label, `lea A1+2,a0` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_movem_pc` | `A1:` label, `movem.l A1(pc),d0-d1` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_org` | `A1:` label, `org A1+8` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_PC_dcw_plus` | `PC:` label, `dc.w PC+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_pcidx` | `A1:` label, `lea A1(pc,d0.w),a0` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_PC_imm_plus` | `PC:` label, `#PC+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `lbl_pcrel` | `A1:` label, `lea A1(pc),a0` | exit 2, `` #1146 expected integer or string `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_rept` | `A1:` label, `rept A1-$11FF` | exit 3, `` #10000 internal error `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_set_rhs` | `A1:` label, `X set A1+2` then `dc.w X` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_sp_dcw_plus` | `sp:` label, `dc.w sp+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `sp` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_SP_dcw_plus` | `SP:` label, `dc.w SP+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `SP` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_sp_imm_plus` | `sp:` label, `#sp+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `sp` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_SP_imm_plus` | `SP:` label, `#SP+2` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `SP` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `lbl_SR_dcw_plus` | `SR:` label, `dc.w SR+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_SR_imm_plus` | `SR:` label, `#SR+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `lbl_USP_dcw_plus` | `USP:` label, `dc.w USP+2` | exit 0, `4e711202` | exit 0, `4e711202` | same |
| `lbl_USP_imm_plus` | `USP:` label, `#USP+2` | exit 0, `4e71303c1202` | exit 0, `4e71303c1202` | same |
| `macro_body_label` | `A1:` defined inside a macro body, `#A1+2` in the same body | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `A1` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `set_a0_dcl` | `a0 set 5`, `dc.l a0+1` | exit 2, `` #1145 expected integer, floating point number or string but got register `` | exit 1, `` `a0` is a register, not a value: expected an integer, floating point number or string. A s `` | both-refuse |
| `z80_ctl_dw_hl` | z80 control: no `hl` defined, `dw hl+2` | exit 2, `` #1010 symbol undefined `` | exit 1, `` unresolved target expression (dangling symbol(s) `hl`) for fixup in section sec4608 at off `` | both-refuse |
| `z80_def_a` | z80 `a:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_af` | z80 `af:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_bc` | z80 `bc:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_hl` | z80 `hl:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_ix` | z80 `ix:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_def_sp` | z80 `sp:` defined, never read | exit 0, `0000` | exit 0, `0000` | same |
| `z80_dw_a` | z80 `a:` label, `dw a` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_af` | z80 `af:` label, `dw af` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_bc` | z80 `bc:` label, `dw bc` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_hl` | z80 `hl:` label, `dw hl` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dw_ix` | z80 `ix:` label, `dw ix` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_dwplus_a` | z80 `a:` label, `dw a+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_af` | z80 `af:` label, `dw af+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_bc` | z80 `bc:` label, `dw bc+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_hl` | z80 `hl:` label, `dw hl+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_ix` | z80 `ix:` label, `dw ix+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dwplus_sp` | z80 `sp:` label, `dw sp+2` | exit 0, `000212` | exit 0, `000212` | same |
| `z80_dw_sp` | z80 `sp:` label, `dw sp` | exit 0, `000012` | exit 0, `000012` | same |
| `z80_ld_hl_bare` | z80 `hl:` label, `ld hl,hl` | exit 2, `` #1500 instruction not supported on Z80 `` | exit 1, `` unsupported form: Instruction { mnemonic: Ld, ops: [Pair(Hl), Pair(Hl)] } `` | both-refuse |
| `z80_ldimm_a` | z80 `a:` label, `ld de,a+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_af` | z80 `af:` label, `ld de,af+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_bc` | z80 `bc:` label, `ld de,bc+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_hl` | z80 `hl:` label, `ld de,hl+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_ix` | z80 `ix:` label, `ld de,ix+2` | exit 0, `00110212` | exit 0, `00110212` | same |
| `z80_ldimm_sp` | z80 `sp:` label, `ld de,sp+2` | exit 0, `00110212` | exit 0, `00110212` | same |
