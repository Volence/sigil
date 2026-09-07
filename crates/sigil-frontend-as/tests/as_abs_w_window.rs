//! An explicit `(addr).w` operand REFUSES an address outside the abs.w window
//! on EVERY route to the extension word, instead of writing its low 16 bits.
//!
//! ## What was silently wrong
//!
//! ```text
//!     move.w  d0,($C00004).w      ->  31C0 0004       a store to $000004
//!     lea     ($C00004).w,a0      ->  41F8 0004
//!     move.l  #Later,($C00004).w  ->  21FC .... 0004
//! ```
//!
//! exit 0, no diagnostic: `(v & 0xFFFF) as i16` at the eager fold.
//!
//! ## The four routes
//!
//! The window `[0, $7FFF] u [$FF8000, $FFFFFF]` on the low 24 bits (asl's
//! sign-extension test, `sigil_ir::asl_width_rule`) reaches an extension word
//! by four routes, named by the function that builds the operand:
//!
//! 1. `convert_one_atom_m68k`, `OperandAtom::M68kAbs` arm: the generic eager
//!    fold every instruction takes for a resolved `(addr).w`. UNCHECKED before
//!    this parcel.
//! 2. `try_defer_long_imm`, `[Imm, M68kAbs]` arm: `move.l #Sym,(addr).w` on
//!    the deferral pass, where the immediate defers and the destination folds
//!    eagerly. UNCHECKED before this parcel.
//! 3. `try_defer_lea_abs`: `lea (Sym).w,aN` with `Sym` unresolved in the front
//!    end; an `Abs16Be` fixup the linker range-checks. Checked.
//! 4. The `jmp`/`jsr (Sym).w` deferral in `lower_m68k`: the same `Abs16Be`
//!    fixup. Checked.
//!
//! Routes 1 and 2 now ask `sigil_ir::fits_abs_w`; the linker's `Abs16Be` arm
//! asks the same function, so routes 3 and 4 share the predicate rather than a
//! second spelling of the window.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) refuses `($C00004).w` and `($8000).w` on `move.w`,
//! `lea`, `move.l #imm` and `jmp` with `error #1340: short addressing not
//! allowed`, exit 2. It accepts `($FFFF8000).w` and `($FF8000).w` as `8000`,
//! `($7FFF).w` as `7FFF`, and `($1007FFF).w` as `7FFF` (exit 0: the test is on
//! the low 24 bits, so the 25th bit is ignored).

use sigil_frontend_as::{assemble_root_relocating_warned, Options};
use sigil_ir::{Module, SymbolTable, SymbolValue};

/// The relocating entry point (the harness's chained-build shape), so the
/// deferral pass that routes 2 and 4 live on actually runs.
fn assemble(body: &str) -> Result<Module, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_relocating_warned(&path, &Options::default()) {
        Ok(a) => Ok(a.module),
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        l.rsplit_once('(')
                            .and_then(|(_, n)| n.trim_end_matches(')').parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

/// The first section's linked bytes, or the link diagnostics.
fn link_with(m: &Module, stubs: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let resolved = sigil_link::resolve_layout(&m.sections, stubs, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, stubs).map_err(|ds| ds.into_iter().map(|d| d.message).collect::<Vec<_>>())?;
    Ok(m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default())
}

fn bytes(body: &str) -> Vec<u8> {
    let m = assemble(body).expect("front end accepts");
    link_with(&m, &SymbolTable::new()).expect("link accepts")
}

fn assert_refused_in_front_end(body: &str, line: u32, needles: &[&str]) {
    match assemble(body) {
        Ok(m) => {
            let b = link_with(&m, &SymbolTable::new()).unwrap_or_default();
            panic!("assembled to {b:02X?} instead of refusing the abs.w address");
        }
        Err(diags) => {
            let hit = diags.iter().find(|(l, msg)| *l == line && needles.iter().all(|n| msg.contains(n)));
            assert!(hit.is_some(), "no diagnostic at line {line} naming {needles:?}; got {diags:?}");
        }
    }
}

const CPU: &str = "\tcpu 68000\n";

// Route 1: the generic eager fold.
#[test]
fn route_1_eager_fold_refuses_an_address_outside_the_window() {
    assert_refused_in_front_end(
        &format!("{CPU}\tmove.w d0,($C00004).w\n"),
        2,
        &["$C00004", "abs.w"],
    );
    assert_refused_in_front_end(&format!("{CPU}\tlea ($C00004).w,a0\n"), 2, &["$C00004", "abs.w"]);
    assert_refused_in_front_end(&format!("{CPU}\tmove.w d0,($8000).w\n"), 2, &["$8000", "abs.w"]);
}

// Route 2: `move.l #Sym,(addr).w` on the deferral pass, destination eager.
#[test]
fn route_2_deferred_long_immediate_refuses_an_eager_destination_outside_the_window() {
    assert_refused_in_front_end(
        &format!("{CPU}\tmove.l #Later,($C00004).w\nLater:\tdc.b 0\n"),
        2,
        &["$C00004", "abs.w"],
    );
}

// Route 3: `lea (Sym).w,aN` with `Sym` unresolved until link.
#[test]
fn route_3_deferred_lea_is_refused_by_the_linker_and_accepted_inside_the_window() {
    let m = assemble(&format!("{CPU}\tlea (Extern).w,a0\n")).expect("lea (Extern).w defers");
    let mut out = SymbolTable::new();
    out.define("Extern", SymbolValue::Int(0xC0_0004));
    let err = link_with(&m, &out).expect_err("$C00004 has no abs.w spelling");
    assert!(err.iter().any(|d| d.contains("abs.w")), "got {err:?}");
    let mut ok = SymbolTable::new();
    ok.define("Extern", SymbolValue::Int(0xFFFF_8000));
    assert_eq!(link_with(&m, &ok).expect("$FFFF8000 fits"), vec![0x41, 0xF8, 0x80, 0x00]);
}

// Route 4: `jmp (Sym).w` deferred to link with the same fixup kind.
#[test]
fn route_4_deferred_jmp_is_refused_by_the_linker_and_accepted_inside_the_window() {
    let src = format!("{CPU}\tjmp (Target).w\n\torg $C00000\nTarget:\tnop\n");
    let m = assemble(&src).expect("jmp (Target).w defers");
    let err = link_with(&m, &SymbolTable::new()).expect_err("a label at $C00000 has no abs.w spelling");
    assert!(err.iter().any(|d| d.contains("abs.w")), "got {err:?}");
    assert_eq!(bytes(&format!("{CPU}\tjmp (Target).w\nTarget:\tnop\n")), vec![0x4E, 0xF8, 0x00, 0x04, 0x4E, 0x71]);
}

/// Every address asl accepts still assembles to asl's word, including the
/// 25-bit one whose high bit the 24-bit test ignores.
#[test]
fn addresses_inside_the_window_keep_their_low_word() {
    assert_eq!(bytes(&format!("{CPU}\tmove.w d0,($FFFF8000).w\n")), vec![0x31, 0xC0, 0x80, 0x00]);
    assert_eq!(bytes(&format!("{CPU}\tmove.w d0,($FF8000).w\n")), vec![0x31, 0xC0, 0x80, 0x00]);
    assert_eq!(bytes(&format!("{CPU}\tmove.w d0,(-$8000).w\n")), vec![0x31, 0xC0, 0x80, 0x00]);
    assert_eq!(bytes(&format!("{CPU}\tmove.w d0,($7FFF).w\n")), vec![0x31, 0xC0, 0x7F, 0xFF]);
    assert_eq!(bytes(&format!("{CPU}\tmove.w d0,($1007FFF).w\n")), vec![0x31, 0xC0, 0x7F, 0xFF]);
    assert_eq!(
        bytes(&format!("{CPU}\tmove.l #Later,($FFFF8000).w\nLater:\tdc.b 0\n")),
        vec![0x21, 0xFC, 0x00, 0x00, 0x00, 0x08, 0x80, 0x00, 0x00]
    );
}
