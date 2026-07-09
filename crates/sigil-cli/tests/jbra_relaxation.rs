//! Spec 2 · Plan 7 #8 — end-to-end byte-exactness proof for the emp-only
//! auto-reaching branches `jbra`/`jbsr` (D2.18). A `.emp` source is compiled
//! through the FULL modern pipeline (parse → `lower_module` → `resolve_layout` →
//! `link` → `flatten`) and every asserted byte is HAND-DERIVED (the arithmetic in
//! comments, house style) and — where the resolved form has an AS spelling —
//! cross-checked against Sigil's INDEPENDENT AS front-end.
//!
//! `jbra L` lowers to ONE `Fragment::RelaxLadder` with four ordered candidates
//! the relaxation fixpoint width-selects by the resolved target address:
//!
//! ```text
//!   rung 0  bra.s  (60 dd)            PcRel8      @1   disp = target-(frag+2), i8, !=0
//!   rung 1  bra.w  (60 00 ddDD)       PcRelDisp16 @2   disp = target-(frag+2), i16
//!   rung 2  jmp.w  (4EF8 aaAA)        Abs16Be     @2   reaches iff asl_width_rule==W
//!   rung 3  jmp.l  (4EF9 aaAAaaAA)    Abs32Be     @2   always reaches
//! ```
//!
//! `jbsr` is identical with `61` / `4EB8` / `4EB9`. The linker picks the FIRST
//! reaching rung (grow-only). `bra.w` (rung 1) is ranked before `jmp abs.w`
//! (rung 2) though both are 4 bytes — PC-relative is preferred (D2.18).

use sigil_frontend_as::{assemble, Options};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_link::LinkedImage;
use sigil_span::Level;

// ---------------------------------------------------------------------------
// Harness — mirrors `symbolic_operands.rs`: link the modern pipeline, exposing
// the linked image (so high-VMA sections need not flatten a multi-MB gap) and a
// whole-flat-image path for the small low-address programs.
// ---------------------------------------------------------------------------

fn emp_link(emp: &str) -> LinkedImage {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse errors: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "emp lower errors: {ldiags:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("emp resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("emp link failed: {d:?}"))
}

fn as_link(asm: &str) -> LinkedImage {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"))
}

fn emp_flat(emp: &str) -> Vec<u8> {
    sigil_link::flatten(&emp_link(emp), 0x00)
}
fn as_flat(asm: &str) -> Vec<u8> {
    sigil_link::flatten(&as_link(asm), 0x00)
}
fn emp_section_bytes(emp: &str, name: &str) -> Vec<u8> {
    emp_link(emp)
        .section(name)
        .unwrap_or_else(|| panic!("emp section `{name}` not found"))
        .bytes
        .clone()
}

// ---------------------------------------------------------------------------
// 1. `jbra .near` → bra.s with a small forward displacement.
// ---------------------------------------------------------------------------

/// A proc-local `.near` target 4 bytes ahead. The whole program lays out at
/// VMA 0:
///   0: jbra .near   (rung 0: bra.s, 2 bytes, disp byte at offset 1)
///   2: nop          (4E 71)
///   4: .near: rts   (4E 75)
/// Rung 0 (bra.s) disp = target - (frag+2) = 4 - (0+2) = 2, fits i8 and != 0 →
/// bra.s reaches. Byte 1 = 0x02. Image = `60 02 4E 71 4E 75`.
#[test]
fn jbra_near_forward_lands_bra_s() {
    let emp = "module m\n\
               proc f() {\n\
                   jbra .near\n\
                   nop\n\
               .near:\n\
                   rts\n\
               }\n";
    let want = vec![0x60, 0x02, 0x4E, 0x71, 0x4E, 0x75];
    assert_eq!(emp_flat(emp), want, "jbra .near forward → bra.s disp 2");
    // Independent AS cross-check: the resolved form is exactly `bra.s .near`.
    let asm = "\tbra.s near\n\tnop\nnear:\n\trts\n";
    assert_eq!(as_flat(asm), want, "AS `bra.s near` equals the resolved jbra bytes");
}

// ---------------------------------------------------------------------------
// 2. Backward `jbra .back` → bra.s with a NEGATIVE displacement.
// ---------------------------------------------------------------------------

/// A backward branch to an EARLIER label lands bra.s with a negative disp.
///   0: .back: nop   (4E 71)
///   2: jbra .back   (rung 0: bra.s, frag @ 2)
/// disp = target - (frag+2) = 0 - (2+2) = -4 → 0xFC. Image = `4E 71 60 FC`.
#[test]
fn jbra_backward_lands_bra_s_negative() {
    let emp = "module m\n\
               proc f() {\n\
               .back:\n\
                   nop\n\
                   jbra .back\n\
               }\n";
    let want = vec![0x4E, 0x71, 0x60, 0xFC];
    assert_eq!(emp_flat(emp), want, "backward jbra → bra.s disp -4");
    let asm = "back:\n\tnop\n\tbra.s back\n";
    assert_eq!(as_flat(asm), want, "AS `bra.s back` equals the resolved jbra bytes");
}

// ---------------------------------------------------------------------------
// 3. Far intra-module target → bra.w (out of i8 range, within i16).
// ---------------------------------------------------------------------------

/// A forward target padded 200 bytes away: too far for bra.s, within bra.w.
///   0: jbra Far      (relaxes to rung 1: bra.w, 4 bytes)
///   4: Pad           (200 bytes of data)
///   204: Far: rts    (4E 75)
/// bra.s (2B) would put Far @ 202, disp = 202-2 = 200 > 127 → grow.
/// bra.w (4B) puts Far @ 204, disp = target-(frag+2) = 204-2 = 202 = 0x00CA,
/// fits i16 → bra.w. Image head = `60 00 00 CA`, Far byte @ 204 = `4E 75`.
#[test]
fn jbra_far_intramodule_lands_bra_w() {
    // 200 zero bytes of padding data between the branch and its target.
    let pad = std::iter::repeat_n("$00", 200).collect::<Vec<_>>().join(", ");
    let emp = format!(
        "module m\n\
         proc f() {{\n\
             jbra Far\n\
         }}\n\
         data Pad: [u8; 200] = [{pad}]\n\
         data Far: [u8; 2] = [$4E, $75]\n"
    );
    let bytes = emp_flat(&emp);
    // frag @ 0; Far @ 4 (bra.w) + 200 (pad) = 204. disp = 204 - 2 = 202 = 0x00CA.
    assert_eq!(&bytes[0..4], &[0x60, 0x00, 0x00, 0xCA], "jbra Far → bra.w disp 202");
    assert_eq!(&bytes[204..206], &[0x4E, 0x75], "Far data at offset 204");
    // AS cross-check with an explicit bra.w to the same layout.
    let asm = format!("\tbra.w Far\nPad:\n\tdc.b {pad}\nFar:\n\tdc.b $4E, $75\n");
    assert_eq!(as_flat(&asm), bytes, "AS `bra.w Far` equals the resolved jbra image");
}

// ---------------------------------------------------------------------------
// 4. Low-address far target → jmp abs.w (PC-relative out of reach, addr ≤ $7FFF).
// ---------------------------------------------------------------------------

/// The branch sits at a HIGH VMA ($100000) and the target at a LOW absolute
/// address ($40): the PC-relative displacement is a huge negative (out of i16),
/// so neither bra.s nor bra.w reaches, but `asl_width_rule($40) == W` → jmp
/// abs.w reaches (rung 2). jmp abs.w = `4E F8 00 40` (operand word = $0040).
#[test]
fn jbra_low_far_lands_jmp_abs_w() {
    let emp = "module m\n\
               section code (cpu: m68000, vma: $100000) {\n\
                   proc f() {\n\
                       jbra Target\n\
                   }\n\
               }\n\
               section tgt (cpu: m68000, vma: $40) {\n\
                   data Target: [u8; 2] = [$00, $00]\n\
               }\n";
    // disp = $40 - ($100000+2) ≈ -$FFFFC2, out of i16 → bra.* fail; abs.w reaches.
    assert_eq!(
        &emp_section_bytes(emp, "code")[..4],
        &[0x4E, 0xF8, 0x00, 0x40],
        "jbra low-far → jmp abs.w to $0040"
    );
}

// ---------------------------------------------------------------------------
// 5. High-address far target → jmp abs.l (asl_width_rule == L).
// ---------------------------------------------------------------------------

/// The branch at $100000 targets $200000: `asl_width_rule($200000) == L` (it is
/// above $7FFF and below $FF8000), and PC-relative is out of reach, so the ladder
/// must fall all the way to jmp abs.l (rung 3). jmp abs.l = `4E F9 00 20 00 00`
/// (32-bit operand = $00200000).
#[test]
fn jbra_high_far_lands_jmp_abs_l() {
    let emp = "module m\n\
               section code (cpu: m68000, vma: $100000) {\n\
                   proc f() {\n\
                       jbra Target\n\
                   }\n\
               }\n\
               section tgt (cpu: m68000, vma: $200000) {\n\
                   data Target: [u8; 2] = [$00, $00]\n\
               }\n";
    assert_eq!(
        &emp_section_bytes(emp, "code")[..6],
        &[0x4E, 0xF9, 0x00, 0x20, 0x00, 0x00],
        "jbra high-far → jmp abs.l to $00200000"
    );
}

// ---------------------------------------------------------------------------
// 6. `jbsr` — the CALL forms: bsr.s (near) and jsr abs.w (low-far).
// ---------------------------------------------------------------------------

/// `jbsr .near` mirrors case 1 with the bsr opcode: rung 0 = bsr.s = `61 dd`.
///   0: jbsr .near  (bsr.s, 2 bytes)  2: nop  4: .near: rts
/// disp = 4 - (0+2) = 2 → `61 02`. Image = `61 02 4E 71 4E 75`.
#[test]
fn jbsr_near_lands_bsr_s() {
    let emp = "module m\n\
               proc f() {\n\
                   jbsr .near\n\
                   nop\n\
               .near:\n\
                   rts\n\
               }\n";
    let want = vec![0x61, 0x02, 0x4E, 0x71, 0x4E, 0x75];
    assert_eq!(emp_flat(emp), want, "jbsr .near → bsr.s disp 2");
    let asm = "\tbsr.s near\n\tnop\nnear:\n\trts\n";
    assert_eq!(as_flat(asm), want, "AS `bsr.s near` equals the resolved jbsr bytes");
}

/// `jbsr` low-far → jsr abs.w = `4E B8 00 40` (the call-opcode twin of case 4).
#[test]
fn jbsr_low_far_lands_jsr_abs_w() {
    let emp = "module m\n\
               section code (cpu: m68000, vma: $100000) {\n\
                   proc f() {\n\
                       jbsr Target\n\
                   }\n\
               }\n\
               section tgt (cpu: m68000, vma: $40) {\n\
                   data Target: [u8; 2] = [$00, $00]\n\
               }\n";
    assert_eq!(
        &emp_section_bytes(emp, "code")[..4],
        &[0x4E, 0xB8, 0x00, 0x40],
        "jbsr low-far → jsr abs.w to $0040"
    );
}

// ---------------------------------------------------------------------------
// 7. Cross-module `jbra Draw_Sprite` via `use` — links to the right address.
//    Driven through the real `sigil emp` binary (the true multi-module pipeline),
//    mirroring `module_resolution.rs::two_modules_cross_reference_and_link`.
// ---------------------------------------------------------------------------

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// The entry module `use`s `Draw_Sprite` from another module and `jbra`s it. In
/// no-map mode the linker packs sections sequentially in discovery order, and the
/// ladder fragment reserves its MAX span (6 bytes, jmp abs.l) for PLACEMENT, so
/// `helpers` lands at LMA 6 (not overlapping at 0):
///   plant   @ 0: jbra Draw_Sprite  → Draw_Sprite @ 6
///   helpers @ 6: rts = 4E 75
/// Resolved: disp = 6 - (0+2) = 4, fits i8 and != 0 → bra.s = `60 04`. The 6-span
/// leaves a 2-byte gap (bytes 2..6) before the target — proving the cross-module
/// symbol resolves and the ladder reaches it via the smallest form.
#[test]
fn jbra_cross_module_links_via_use() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nuse engine.helpers.{Draw_Sprite}\n\
         proc init (a0: *u8) {\n    jbra Draw_Sprite\n}\n",
    );
    let out = root.join("out.bin");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "cross-module jbra compile should succeed");
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 8, "plant span 6 (max ladder) + helpers span 2");
    // plant @ 0: jbra Draw_Sprite → target @ 6, disp = 6-2 = 4 → bra.s `60 04`.
    assert_eq!(&bytes[0..2], &[0x60, 0x04], "cross-module jbra → bra.s to helpers @ 6");
    assert_eq!(&bytes[6..8], &[0x4E, 0x75], "engine.helpers Draw_Sprite rts at LMA 6");
}
