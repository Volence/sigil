//! Spec 2 · Plan 7 — end-to-end byte-exactness proof for **symbolic absolute
//! operands** in straight-line 68000 instructions (`move.w Foo, d0`, `lea Foo,
//! a0`, …). The safety net for the whole feature: a `.emp` source string is
//! compiled through the FULL modern pipeline (parse → `lower_module` →
//! `resolve_layout` → `link` → `flatten`) and its bytes are asserted
//! byte-identical to the AS-reference bytes for the same program.
//!
//! ## What the AS reference actually is (and how strong the diff is)
//!
//! `as_reference()` assembles against Sigil's OWN AS-syntax front-end,
//! [`sigil_frontend_as`] — NOT the external `asw`/`asl` Macro Assembler (that
//! toolchain is not invoked anywhere in these crates). That front-end DOES
//! handle bare symbolic absolute operands (`move.w Foo, d0`), but via a
//! **completely INDEPENDENT width-selection mechanism** from the emp path:
//!
//! - The **emp** path lowers a symbolic operand to a length-variable
//!   `Fragment::RelaxAbsSym` (both abs.w and abs.l candidates encoded) and defers
//!   the abs.w/abs.l choice to `resolve_layout`'s relaxation fixpoint (Task 1/2).
//! - The **AS** path folds the operand inside its own multi-pass front-end
//!   fixpoint (`abs_ea_from_expr`) and emits a *finished* `Fragment::Data` with
//!   the width already baked in — it never constructs a `RelaxAbsSym`.
//!
//! The only primitive the two share is the `asl_width_rule` function in
//! `sigil-ir` (the abs.w vs abs.l predicate). Because the surrounding lowering,
//! candidate encoding, and fixpoint machinery are otherwise independent, this
//! byte-diff is a **strong independent cross-check** of the emp lowering, not
//! merely a same-backend consistency check.

use sigil_frontend_as::{assemble, Options};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_link::LinkedImage;
use sigil_span::Level;

// ---------------------------------------------------------------------------
// Harness — mirrors `ports.rs`, but exposes the linked image (not just the flat
// bytes) so tests targeting HIGH VMAs can inspect a single code section without
// flattening a multi-megabyte gap.
// ---------------------------------------------------------------------------

/// Assemble a `.asm` source through the AS front-end and link it (empty external
/// table — these programs are self-contained). Panics with diagnostics on any
/// failure.
fn as_link(asm: &str) -> LinkedImage {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"))
}

/// Compile a `.emp` source through the modern front-end and link it — the exact
/// pipeline the `sigil emp` CLI runs. Panics on any `Error`-level diagnostic.
fn emp_link(emp: &str) -> LinkedImage {
    let (file, pdiags) = parse_str(emp);
    assert!(
        pdiags.iter().all(|d| d.level != Level::Error),
        "emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != Level::Error),
        "emp lower errors: {ldiags:?}"
    );
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("emp resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("emp link failed: {d:?}"))
}

/// Whole flat image (fill `0x00`) — the strongest diff, used for small programs
/// where every emitted byte fits without a huge inter-section gap.
fn as_flat(asm: &str) -> Vec<u8> {
    sigil_link::flatten(&as_link(asm), 0x00)
}
fn emp_flat(emp: &str) -> Vec<u8> {
    sigil_link::flatten(&emp_link(emp), 0x00)
}

/// The bytes of a single named linked section — used when the referenced symbol
/// lives at a HIGH VMA (flattening the whole image would pad megabytes of gap).
fn emp_section_bytes(emp: &str, name: &str) -> Vec<u8> {
    emp_link(emp)
        .section(name)
        .unwrap_or_else(|| panic!("emp section `{name}` not found"))
        .bytes
        .clone()
}
/// The AS front-end names the single top-level code section `sec0`.
fn as_sec0_bytes(asm: &str) -> Vec<u8> {
    as_link(asm).sections[0].bytes.clone()
}

/// Assert byte-identity, reporting the first differing offset on failure.
fn assert_bytes(reference: &[u8], candidate: &[u8], what: &str) {
    if reference == candidate {
        return;
    }
    let n = reference.len().min(candidate.len());
    if let Some(i) = (0..n).find(|&i| reference[i] != candidate[i]) {
        panic!(
            "{what}: first byte diff at offset {i:#x}: reference {:#04x} != candidate {:#04x}\n\
             reference = {:02X?}\n candidate = {:02X?}",
            reference[i], candidate[i], reference, candidate,
        );
    }
    panic!(
        "{what}: lengths differ — reference {} bytes, candidate {} bytes",
        reference.len(),
        candidate.len()
    );
}

// ---------------------------------------------------------------------------
// 1. Both width outcomes end-to-end, with the resolved address in the bytes.
// ---------------------------------------------------------------------------

/// abs.w LOW address via a genuine FORWARD reference, proven as a WHOLE-IMAGE
/// byte-diff against AS. `move.w d0, Target` precedes `Target`'s definition, so
/// the relaxation fixpoint must place it: `Target` lands at $0006 (after the
/// 4-byte instr + 2-byte `rts`), ≤ $7FFF → abs.w, operand word `00 06`.
/// Image: `31 C0 00 06 4E 75 00 00`.
#[test]
fn abs_w_forward_ref_whole_image_matches_as() {
    let emp = "module m\n\
               proc f() {\n\
                   move.w d0, Target\n\
                   rts\n\
               }\n\
               data Target: [u8; 2] = [$00, $00]\n";
    let asm = "\tmove.w d0,Target\n\trts\nTarget:\n\tdc.w 0\n";
    let want = vec![0x31, 0xC0, 0x00, 0x06, 0x4E, 0x75, 0x00, 0x00];
    let emp_bytes = emp_flat(emp);
    assert_eq!(emp_bytes, want, "abs.w forward-ref resolved bytes");
    assert_bytes(&as_flat(asm), &emp_bytes, "abs.w forward-ref: emp vs AS");
}

/// abs.w controlled LOW address ($1000). Symbol placed by a `vma: $1000` section
/// (emp) / an equate (AS); both resolve `Target` to $1000. `move.w d0, Target` →
/// `31 C0 10 00` — the resolved $1000 in the abs.w operand word.
#[test]
fn abs_w_low_address_matches_as() {
    let emp = "module m\n\
               section code (cpu: m68000, vma: $0) {\n\
                   proc f() {\n\
                       move.w d0, Target\n\
                       rts\n\
                   }\n\
               }\n\
               section tgt (cpu: m68000, vma: $1000) {\n\
                   data Target: [u8; 2] = [$00, $00]\n\
               }\n";
    let asm = "Target equ $1000\n\tmove.w d0,Target\n\trts\n";
    let emp_bytes = emp_section_bytes(emp, "code");
    assert_eq!(emp_bytes, vec![0x31, 0xC0, 0x10, 0x00, 0x4E, 0x75], "abs.w $1000 bytes");
    assert_bytes(&as_sec0_bytes(asm), &emp_bytes, "abs.w $1000: emp vs AS");
}

/// abs.l HIGH address ($12345678, > $7FFF). `move.l Target, d0` selects the abs.l
/// candidate: 6-byte instruction `20 39 12 34 56 78` carrying the full 32-bit
/// resolved address. Section-byte diff (the VMA is far too high to flatten).
#[test]
fn abs_l_high_address_matches_as() {
    let emp = "module m\n\
               section code (cpu: m68000, vma: $0) {\n\
                   proc f() {\n\
                       move.l Target, d0\n\
                       rts\n\
                   }\n\
               }\n\
               section tgt (cpu: m68000, vma: $12345678) {\n\
                   data Target: [u8; 2] = [$00, $00]\n\
               }\n";
    let asm = "Target equ $12345678\n\tmove.l Target,d0\n\trts\n";
    let emp_bytes = emp_section_bytes(emp, "code");
    assert_eq!(
        emp_bytes,
        vec![0x20, 0x39, 0x12, 0x34, 0x56, 0x78, 0x4E, 0x75],
        "abs.l $12345678 bytes"
    );
    assert_bytes(&as_sec0_bytes(asm), &emp_bytes, "abs.l $12345678: emp vs AS");
}

/// abs.w RAM address ($FFFF8000, the sign-extension window `[$FF8000,$FFFFFF]`).
/// `asl_width_rule` selects abs.w even though the value is huge — the operand is
/// the sign-extended low word `80 00`. `move.w Target, d0` → `30 38 80 00`.
#[test]
fn abs_w_ram_address_matches_as() {
    let emp = "module m\n\
               section code (cpu: m68000, vma: $0) {\n\
                   proc f() {\n\
                       move.w Target, d0\n\
                       rts\n\
                   }\n\
               }\n\
               section tgt (cpu: m68000, vma: $FFFF8000) {\n\
                   data Target: [u8; 2] = [$00, $00]\n\
               }\n";
    let asm = "Target equ $FFFF8000\n\tmove.w Target,d0\n\trts\n";
    let emp_bytes = emp_section_bytes(emp, "code");
    assert_eq!(emp_bytes, vec![0x30, 0x38, 0x80, 0x00, 0x4E, 0x75], "abs.w RAM bytes");
    assert_bytes(&as_sec0_bytes(asm), &emp_bytes, "abs.w RAM: emp vs AS");
}

// ---------------------------------------------------------------------------
// 2. In-scope instruction-shape spread — every shape byte-diffed vs AS at $1000.
// ---------------------------------------------------------------------------

/// Each in-scope shape assembles to the AS-reference bytes with the resolved
/// $1000 address correctly placed. `Target equ $1000` on the AS side; a
/// `vma: $1000` section on the emp side. Both frontends independently
/// width-select abs.w and land the operand word `10 00`.
#[test]
fn instruction_shapes_match_as_at_low_address() {
    // (emp instruction, AS instruction, expected code bytes incl. resolved $1000)
    let cases: &[(&str, &str, &[u8])] = &[
        ("move.w Target, d0", "move.w Target,d0", &[0x30, 0x38, 0x10, 0x00]),
        ("move.w d0, Target", "move.w d0,Target", &[0x31, 0xC0, 0x10, 0x00]),
        ("move.l Target, d0", "move.l Target,d0", &[0x20, 0x38, 0x10, 0x00]),
        ("lea Target, a0", "lea Target,a0", &[0x41, 0xF8, 0x10, 0x00]),
        ("tst.w Target", "tst.w Target", &[0x4A, 0x78, 0x10, 0x00]),
        ("clr.w Target", "clr.w Target", &[0x42, 0x78, 0x10, 0x00]),
    ];
    for (emp_instr, as_instr, want) in cases {
        let emp = format!(
            "module m\n\
             section code (cpu: m68000, vma: $0) {{\n\
                 proc f() {{\n\
                     {emp_instr}\n\
                     rts\n\
                 }}\n\
             }}\n\
             section tgt (cpu: m68000, vma: $1000) {{\n\
                 data Target: [u8; 2] = [$00, $00]\n\
             }}\n"
        );
        let asm = format!("Target equ $1000\n\t{as_instr}\n\trts\n");
        // Compare only the leading instruction bytes (both are followed by `rts`
        // = 4E 75, already covered by the whole-section diff below).
        let emp_bytes = emp_section_bytes(&emp, "code");
        assert_eq!(&emp_bytes[..want.len()], *want, "{emp_instr}: resolved bytes");
        assert_bytes(&as_sec0_bytes(&asm), &emp_bytes, &format!("{emp_instr}: emp vs AS"));
    }
}

// ---------------------------------------------------------------------------
// 3. Mixed program — symbolic operands interleaved with ordinary data and
//    register/immediate instructions, plus a FORWARD reference. Whole-image
//    byte-diff proves the relaxation fixpoint + downstream placement in a real
//    end-to-end compile.
// ---------------------------------------------------------------------------

/// A realistic straight-line routine: leading `data`, a `moveq` immediate move,
/// three symbolic-operand instructions all FORWARD-referencing `Payload`
/// (`move.w`, `lea`, `tst.w`), `rts`, then the `Payload` data. The forward refs
/// force the relaxation fixpoint to resolve `Payload`'s address ($0014, abs.w)
/// and every instruction encodes `00 14`. Whole 24-byte image is byte-identical
/// to the AS reference:
/// `DE AD BE EF | 70 00 | 31 C0 00 14 | 43 F8 00 14 | 4A 78 00 14 | 4E 75 | CA FE F0 0D`.
#[test]
fn mixed_program_with_forward_refs_matches_as() {
    let emp = "module m\n\
               data Header: [u8; 4] = [$DE, $AD, $BE, $EF]\n\
               proc routine() {\n\
                   moveq #0, d0\n\
                   move.w d0, Payload\n\
                   lea Payload, a1\n\
                   tst.w Payload\n\
                   rts\n\
               }\n\
               data Payload: [u8; 4] = [$CA, $FE, $F0, $0D]\n";
    let asm = "Header:\n\
               \tdc.b $DE, $AD, $BE, $EF\n\
               routine:\n\
               \tmoveq #0, d0\n\
               \tmove.w d0, Payload\n\
               \tlea Payload, a1\n\
               \ttst.w Payload\n\
               \trts\n\
               Payload:\n\
               \tdc.b $CA, $FE, $F0, $0D\n";
    let want = vec![
        0xDE, 0xAD, 0xBE, 0xEF, // Header
        0x70, 0x00, // moveq #0, d0
        0x31, 0xC0, 0x00, 0x14, // move.w d0, Payload  (Payload = $0014)
        0x43, 0xF8, 0x00, 0x14, // lea Payload, a1
        0x4A, 0x78, 0x00, 0x14, // tst.w Payload
        0x4E, 0x75, // rts
        0xCA, 0xFE, 0xF0, 0x0D, // Payload
    ];
    let emp_bytes = emp_flat(emp);
    assert_eq!(emp_bytes, want, "mixed program resolved image");
    assert_bytes(&as_flat(asm), &emp_bytes, "mixed program: emp vs AS");
}
