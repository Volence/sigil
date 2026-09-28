//! A symbol followed by an absolute-size suffix (`jmp Foo.w`), and the other
//! places asl reads a trailing `.w`/`.l`/`.b`/`.s` off a 68000 address or
//! displacement, on the AS route.
//!
//! sigil's lexer delivers `Foo.w` as one identifier, because `.` is an
//! identifier character and `Parent.local` is how a local label is spelled.
//! asl instead peels the width off the end of an effective-address operand
//! first and looks up what is left, so:
//!
//! 1. `jmp Foo.w` is `Foo` at word width, `jsr Foo.l` at long width, in any
//!    effective-address operand and either case, before or after `Foo` is
//!    defined. A value that does not fit a short address is refused.
//! 2. That reading wins over a name spelled with the dot: with `Foo = $1234`
//!    and `Foo.w = $5678` (or a local label `.w` under `Foo`), `jmp Foo.w` is
//!    `Foo`. sigil read the dotted name there, silently, before this change.
//!    Where asl reads a dotted name whole (`dc.w`, `#imm`, `(Foo.w).w`, a
//!    name followed by more expression) sigil does too.
//! 3. The same peel applies to `(Foo.w)`, and to a displacement: `.w` before
//!    `(An)`/`(pc)` or inside `(d,An)`, `.b` before `(An,Xn)`/`(pc,Xn)`. Every
//!    other width there is refused.
//! 4. A branch or `dbcc` target is a label, not an address, and asl reads the
//!    name whole there (`bra.s T.w` branches to the local label `T.w`).
//!
//! ## Provenance
//!
//! Every expected byte string is the hex of asl's own image, never sigil's:
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run as
//!
//! ```text
//! . docs/superpowers/notes/asl-reference/asl_ref.sh
//! asl_run -xx -n -q -A -L -U -i .. <probe>.asm      # ASL_EXIT=0
//! p2bin <probe>.p <probe>.bin
//! ```
//!
//! by `run_asl.sh` in `docs/superpowers/notes/2026-09-28-as-symbol-size-suffix-probes/`,
//! whose `mkprobes.sh` writes each probe under `cpu 68000`; its output is
//! committed there as `asl.out`. Each row below names its probe, and its source
//! is that probe's verbatim. A refusal row cites a run that exited 2 and reads
//! only the accept-or-refuse verdict from it, never a byte.
//!
//! ## Deliberate divergence from asl, named
//!
//! - `move.w Foo.b(a0),d0` (probe `disp_sym_b`): asl exits 0 with no diagnostic
//!   and emits NO BYTES for the line, so the instruction silently vanishes.
//!   That is not an answer to match; sigil refuses it.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    let m = match assemble_root_located(&path, &Options::default()) {
        Ok(m) => m,
        Err(f) => return Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    };
    let msgs =
        |d: Vec<sigil_span::Diagnostic>| d.iter().map(|d| d.message.clone()).collect::<Vec<_>>();
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(msgs)?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).map_err(msgs)?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

/// `cpu 68000` then the probe's lines, the shape `mkprobes.sh` writes.
fn probe(lines: &str) -> String {
    format!("\tcpu 68000\n{lines}\n")
}

/// Assemble each `(probe, source, asl hex)` row and require asl's bytes.
fn assert_rows(rows: &[(&str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, asl_hex) in rows {
        match assemble(&probe(src)) {
            Ok(b) => {
                let got: String = b.iter().map(|x| format!("{x:02X}")).collect();
                if got != *asl_hex {
                    bad.push(format!("{name}: asl {asl_hex}, sigil {got}"));
                }
            }
            Err(d) => bad.push(format!("{name}: asl {asl_hex}, sigil refused: {d:?}")),
        }
    }
    assert!(bad.is_empty(), "rows that differ from asl:\n{}", bad.join("\n"));
}

/// Assemble each `(probe, source, needle)` row and require a refusal whose text
/// contains the needle, so a neighbouring refusal path cannot stand in for it.
fn assert_refusals(rows: &[(&str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, needle) in rows {
        match assemble(&probe(src)) {
            Ok(b) => bad.push(format!("{name}: expected a refusal, sigil emitted {b:02X?}")),
            Err(d) => {
                if !d.iter().any(|m| m.contains(needle)) {
                    bad.push(format!("{name}: refused, but no diagnostic names `{needle}`: {d:?}"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "rows that were not refused as expected:\n{}", bad.join("\n"));
}

#[test]
fn a_symbol_with_a_width_suffix_is_an_absolute_at_that_width() {
    assert_rows(&[
        ("jmp_sym_w", "Foo = $1234\n\tjmp\tFoo.w", "4EF81234"),
        ("jmp_sym_W", "Foo = $1234\n\tjmp\tFoo.W", "4EF81234"),
        ("jsr_sym_l", "Foo = $1234\n\tjsr\tFoo.l", "4EB900001234"),
        ("jsr_sym_L", "Foo = $1234\n\tjsr\tFoo.L", "4EB900001234"),
        ("lea_sym_w", "Foo = $1234\n\tlea\tFoo.w,a0", "41F81234"),
        ("move_sym_w_src", "Foo = $1234\n\tmove.w\tFoo.w,d0", "30381234"),
        ("move_sym_l_dst", "Foo = $1234\n\tmove.l\td0,Foo.l", "23C000001234"),
        ("move_sym_w_dst", "Foo = $FFFFF600\n\tmove.w\td0,Foo.w", "31C0F600"),
        ("move_sym_both", "Foo = $1234\nBar = $FFFF8000\n\tmove.w\tFoo.w,Bar.w", "31F812348000"),
        ("movem_sym_w", "Foo = $FFFFF600\n\tmovem.l\tFoo.w,d0-d1", "4CF80003F600"),
        ("movem_sym_w_dst", "Foo = $FFFFF600\n\tmovem.l\td0-d1,Foo.w", "48F80003F600"),
        ("clr_sym_w", "Foo = $FFFFF600\n\tclr.w\tFoo.w", "4278F600"),
        ("btst_sym_w", "Foo = $FFFFF600\n\tbtst\t#1,Foo.w", "08380001F600"),
        ("move_imm_sym_w", "Foo = $FFFFF600\n\tmove.w\t#1,Foo.w", "31FC0001F600"),
        ("pea_sym_l", "Foo = $1234\n\tpea\tFoo.l", "487900001234"),
        ("jmp_sym_w_neg", "Foo = $FFFF8000\n\tjmp\tFoo.w", "4EF88000"),
        ("jmp_sym_w_m2", "Foo = -2\n\tjmp\tFoo.w", "4EF8FFFE"),
        ("jmp_sym_expr_w", "Foo = $1000\n\tjmp\tFoo+$234.w", "4EF81234"),
        ("expr_tail_sym", "Foo = $10\n\tjmp\t2+Foo.w", "4EF80012"),
        ("sym_space_w", "Foo = $1234\n\tjmp\tFoo .w", "4EF81234"),
        ("num_space_w", "\tjmp\t$1234 .w", "4EF81234"),
        ("sym_areg_name_w", "A9 = $1234\n\tjmp\tA9.w", "4EF81234"),
        ("macro_param_w", "jw macro dst\n\tjmp\tdst.w\n\tendm\nFoo = $1234\n\tjw\tFoo", "4EF81234"),
        ("jmp_sym_paren_w", "Foo = $1234\n\tjmp\t(Foo).w", "4EF81234"),
        ("sym_imm_expr_w", "Foo = $1234\n\tjmp\t-(-Foo).w", "4EF81234"),
    ]);
    // asl `#1130 invalid operand size` (`.b`, `.s`), `#1340 short addressing
    // not allowed`, and `#1010 symbol undefined` naming `Undef`, not `Undef.w`.
    assert_refusals(&[
        ("jmp_sym_b", "Foo = $1234\n\tjmp\tFoo.b", "is not an absolute address width"),
        ("jmp_sym_s", "Foo = $1234\n\tjmp\tFoo.s", "is not an absolute address width"),
        ("jmp_sym_w_hi", "Foo = $8000\n\tjmp\tFoo.w", "does not fit abs.w"),
        ("jmp_sym_w_big", "Foo = $12345\n\tjmp\tFoo.w", "does not fit abs.w"),
        ("move_sym_w_hi", "Foo = $FF00\n\tmove.w\td0,Foo.w", "does not fit abs.w"),
        ("undef_sym_w", "\tjmp\tUndef.w", "`Undef`"),
    ]);
}

#[test]
fn a_forward_reference_takes_the_suffix_too() {
    assert_rows(&[
        ("fwd_equ_w", "\tjmp\tFoo.w\nFoo = $1234", "4EF81234"),
        ("fwd_equ_l", "\tjsr\tFoo.l\nFoo = $1234", "4EB900001234"),
        ("fwd_equ_move", "\tmove.w\tFoo.w,d0\nFoo = $FFFFF600", "3038F600"),
        ("fwd_label_w", "\tjmp\tT.w\n\tnop\nT:", "4EF800064E71"),
        ("fwd_label_l", "\tjmp\tT.l\n\tnop\nT:", "4EF9000000084E71"),
        ("back_label_w", "T:\tnop\n\tjmp\tT.w", "4E714EF80000"),
    ]);
    // asl `#1340 short addressing not allowed`.
    assert_refusals(&[
        ("fwd_equ_w_hi", "\tjmp\tFoo.w\nFoo = $8000", "does not fit abs.w"),
        ("fwd_label_w_hi", "\tjmp\tT.w\n\tds.b\t$8000\nT:", "does not fit abs.w"),
    ]);
}

/// The suffix reading beats a name spelled with the dot, and a dotted name is
/// read whole wherever asl reads it whole.
#[test]
fn a_dotted_name_is_read_the_way_asl_reads_it() {
    assert_rows(&[
        ("dot_equ_collide", "Foo = $1234\nFoo.w = $5678\n\tjmp\tFoo.w", "4EF81234"),
        ("dot_w_w", "Foo.w = $5678\n\tjmp\tFoo.w.w", "4EF85678"),
        ("local_w_collide", "Foo:\tnop\n.w:\tnop\n\tjmp\tFoo.w", "4E714E714EF80000"),
        ("local_l_collide", "Foo:\tnop\n.l:\tnop\n\tjsr\tFoo.l", "4E714E714EB900000000"),
        ("local_full_w", "Foo:\tnop\n.loop:\tnop\n\tjmp\tFoo.loop.w", "4E714E714EF80002"),
        ("local_full_l", "Foo:\tnop\n.loop:\tnop\n\tjmp\tFoo.loop.l", "4E714E714EF900000002"),
        ("local_loop_w", "Foo:\tnop\n.loop:\tnop\n\tjmp\t.loop.w", "4E714E714EF80002"),
        ("local_w_bare", "Foo:\tnop\n.w:\tnop\n\tjmp\t.w", "4E714E714EF80002"),
        ("local_full_dc", "Foo:\tnop\n.loop:\tnop\n\tdc.w\tFoo.loop", "4E714E710002"),
        ("dot_equ_def", "Foo.bar = $1234\n\tdc.w\tFoo.bar", "1234"),
        ("dot_equ_w_def", "Foo.w = $5678\n\tdc.w\t0", "0000"),
        ("dc_dot_w_defined", "Foo.w = $5678\n\tdc.w\tFoo.w", "5678"),
        ("imm_dot_w_defined", "Foo.w = $5678\n\tmove.w\t#Foo.w,d0", "303C5678"),
        ("expr_mid_dot", "Foo.w = $10\n\tjmp\tFoo.w+2", "4EF80012"),
        ("paren_dot_w_defined", "Foo = $1234\nFoo.w = $5678\n\tjmp\t(Foo.w).w", "4EF85678"),
    ]);
    // `dot_equ_w_use`: asl `#1010` for `Foo` although `Foo.w` is defined.
    // `dot_b_defined`: asl `#1130`. `dc_sym_w`, `imm_sym_w`: asl `#1010` for
    // the whole name `Foo.w`, which is not defined there.
    assert_refusals(&[
        ("dot_equ_w_use", "Foo.w = $5678\n\tjmp\tFoo.w", "`Foo`"),
        ("dot_b_defined", "Foo.b = $1234\n\tjmp\tFoo.b", "is not an absolute address width"),
        ("dc_sym_w", "Foo = $1234\n\tdc.w\tFoo.w", "`Foo.w`"),
        ("imm_sym_w", "Foo = $1234\n\tmove.w\t#Foo.w,d0", "`Foo.w`"),
    ]);
}

#[test]
fn a_width_suffix_inside_the_parens_of_an_absolute() {
    assert_rows(&[
        ("paren_sym_w", "Foo = $1234\n\tjmp\t(Foo.w)", "4EF81234"),
        ("paren_sym_l", "Foo = $1234\n\tjmp\t(Foo.l)", "4EF900001234"),
        ("paren_num_w", "\tjmp\t($1234.w)", "4EF81234"),
        ("paren_move_sym_w", "Foo = $FFFFF600\n\tmove.w\t(Foo.w),d0", "3038F600"),
        ("paren_abs_dot", "Foo = $1234\nFoo.w = $5678\n\tjmp\t(Foo.w)", "4EF81234"),
    ]);
    // asl `#1350 addressing mode not allowed here`, `#1340`.
    assert_refusals(&[
        ("paren_sym_b", "Foo = $1234\n\tjmp\t(Foo.b)", "is not an absolute address width"),
        ("paren_sym_w_hi", "Foo = $8000\n\tjmp\t(Foo.w)", "does not fit abs.w"),
    ]);
}

#[test]
fn a_displacement_takes_only_its_own_width() {
    assert_rows(&[
        ("disp_sym_w", "Foo = 4\n\tmove.w\tFoo.w(a0),d0", "30280004"),
        ("disp_sym_W", "Foo = 4\n\tmove.w\tFoo.W(a0),d0", "30280004"),
        ("disp_num_w", "\tmove.w\t4.w(a0),d0", "30280004"),
        ("disp_collide", "Foo = 4\nFoo.w = 6\n\tmove.w\tFoo.w(a0),d0", "30280004"),
        ("idx_sym_b", "Foo = 4\n\tmove.w\tFoo.b(a0,d0.w),d1", "32300004"),
        ("idx_num_b", "\tmove.w\t4.b(a0,d0.w),d1", "32300004"),
        ("idx_collide", "Foo = 4\nFoo.b = 6\n\tmove.w\tFoo.b(a0,d0.w),d1", "32300004"),
        ("pcrel_sym_w", "Foo:\tnop\n\tmove.w\tFoo.w(pc),d0", "4E71303AFFFC"),
        ("pcrel_num_w", "\tnop\n\tmove.w\t0.w(pc),d0", "4E71303AFFFC"),
        ("pcidx_sym_b", "Foo:\tnop\n\tmove.w\tFoo.b(pc,d0.w),d1", "4E71323B00FC"),
        ("paren_disp_sym", "Foo = 4\n\tmove.w\t(Foo.w,a0),d0", "30280004"),
        ("paren_disp_num", "\tmove.w\t(4.w,a0),d0", "30280004"),
        ("paren_disp_collide", "Foo = 4\nFoo.w = 6\n\tmove.w\t(Foo.w,a0),d0", "30280004"),
    ]);
    // asl `#1505 addressing mode not supported on 68000` (`.l`, `.w` on an
    // index form), `#1130` (`.s`), `#1350` (inside the parens of `(d,An)` and
    // `(d,An,Xn)`), `#1320 range overflow`, and `#1010` for `Foo`.
    assert_refusals(&[
        ("disp_sym_l", "Foo = 4\n\tmove.w\tFoo.l(a0),d0", "is not a displacement width"),
        ("disp_sym_s", "Foo = 4\n\tmove.w\tFoo.s(a0),d0", "is not a displacement width"),
        ("idx_sym_w", "Foo = 4\n\tmove.w\tFoo.w(a0,d0.w),d1", "is not a displacement width"),
        ("paren_disp_sym_l", "Foo = 4\n\tmove.w\t(Foo.l,a0),d0", "is not a displacement width"),
        ("paren_disp_sym_b", "Foo = 4\n\tmove.w\t(Foo.b,a0),d0", "is not a displacement width"),
        ("paren_idx_sym_b", "Foo = 4\n\tmove.w\t(Foo.b,a0,d0.w),d1", "is not a displacement width"),
        ("paren_idx_sym_w", "Foo = 4\n\tmove.w\t(Foo.w,a0,d0.w),d1", "is not a displacement width"),
        ("disp_sym_w_big", "Foo = $8000\n\tmove.w\tFoo.w(a0),d0", "out of range"),
        ("disp_dot_w_defined", "Foo.w = 4\n\tmove.w\tFoo.w(a0),d0", "`Foo`"),
        ("paren_disp_dot", "Foo.w = 4\n\tmove.w\t(Foo.w,a0),d0", "`Foo`"),
    ]);
    // asl exits 0 on this and emits no bytes for the line (see the module doc).
    assert_refusals(&[
        ("disp_sym_b", "Foo = 4\n\tmove.w\tFoo.b(a0),d0", "is not a displacement width"),
    ]);
}

#[test]
fn a_branch_or_dbcc_target_reads_the_name_whole() {
    assert_rows(&[
        ("bra_local_w", "T:\tnop\n.w:\tnop\n\tbra.s\tT.w", "4E714E7160FC"),
        ("bsr_local_l", "T:\tnop\n.l:\tnop\n\tbsr.w\tT.l", "4E714E716100FFFC"),
        ("dbf_local_w", "T:\tnop\n.w:\tnop\n\tdbf\td0,T.w", "4E714E7151C8FFFC"),
    ]);
    // asl `#1010` for the whole name (`T.w`, the local `.loop.w` under `T`),
    // and `#1146` for a register with a width.
    assert_refusals(&[
        ("bra_sym_w", "\tbra.w\tT.w\n\tnop\nT:", "`T.w`"),
        ("dbf_sym_w", "\tdbf\td0,T.w\n\tnop\nT:", "`T.w`"),
        ("bra_dot_loop_w", "T:\tnop\n.loop:\tnop\n\tbra.s\t.loop.w", "`T.loop.w`"),
        ("dreg_w", "\tmove.w\td0.w,d1", "`d0.w`"),
        ("areg_l", "\tmove.l\ta0.l,d1", "`a0.l`"),
    ]);
}
