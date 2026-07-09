//! Port #2 tranche, Task 1 — 68k PC-relative EAs (`Sym(pc,Xn.size)` /
//! `Sym(pc)`), the ONE addressing mode the `.emp` front-end is missing for the
//! math.asm port (`Sine_Table(pc,d0.w)`, aeon `engine/system/math.asm`).
//!
//! Two forms:
//!   - `Sym(pc,Xn.size)` — PC-indexed, brief extension word (mode 111 reg 011),
//!     disp8 signed (-128..=127), index size `.w` (sign-extended, DEFAULT) or
//!     `.l`. Matches AS's `(d,pc,Xn)` / `disp(pc,Xn)` forms (asl-verified in
//!     `sigil-frontend-as/src/eval.rs`'s `m68k_pcrelative_disp8_indexed_lowers`).
//!   - `Sym(pc)` — plain PC-relative, d16 extension word (mode 111 reg 010),
//!     disp16 signed. Matches AS's `(d,pc)` / `disp(pc)`.
//!
//! Both are EXACT (fixed-size) EAs — no relaxation. The backend already has
//! full encode + fixup support (`M68kBackend::lower_pcrel_ea` /
//! `lower_pcrel_idx_ea`, `FixupKind::PcRelDisp16` / `PcRelDisp8` in
//! `sigil-link`), proven out by the AS front-end — this task is front-end-only
//! (emp parse/eval/lower plumbing routing to the existing seam).
//!
//! Cross-section note: `sigil-link`'s fixup resolution for PC-relative kinds is
//! pure VMA arithmetic (`disp = target_vma - site_vma[-1]`) with NO
//! section-identity check anywhere — the same seam `bra`/`bsr`/`jbra` already
//! ride cross-section (see `sigil-link/src/relax.rs::cross_section_fixup_
//! targets_equ_symbol` for the sibling precedent on a different fixup kind, and
//! `sigil-frontend-as` has no cross-section restriction either — AS has no
//! section concept at all, just one flat assembly). So this implementation
//! does NOT add an artificial same-section restriction; `pcrel_cross_section_
//! target_resolves` below proves it resolves correctly.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Assemble a `.asm` source string through the AS front-end (68k mode), link
/// standalone, and flatten — the AS-parity reference bytes. Mirrors
/// `ports.rs::as_reference`.
fn as_reference(asm: &str) -> Vec<u8> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// The full emp pipeline (parse -> lower -> resolve_layout -> link -> flatten),
/// returning the flat image on success or every diagnostic message on failure.
/// Mirrors `here_relaxation_fix.rs::compile_full`.
fn compile_full(emp: &str) -> (Option<Vec<u8>>, Vec<String>) {
    let (file, pdiags) = parse_str(emp);
    let mut msgs: Vec<String> = pdiags.iter().map(|d| d.message.clone()).collect();
    if pdiags.iter().any(|d| d.level == Level::Error) {
        return (None, msgs);
    }
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    msgs.extend(ldiags.iter().map(|d| d.message.clone()));
    if ldiags.iter().any(|d| d.level == Level::Error) {
        return (None, msgs);
    }
    let empty = SymbolTable::new();
    let resolved = match sigil_link::resolve_layout(&module.sections, &empty, true) {
        Ok(r) => r,
        Err(ds) => {
            msgs.extend(ds.into_iter().map(|d| d.message));
            return (None, msgs);
        }
    };
    let linked = match sigil_link::link(&resolved, &empty) {
        Ok(img) => img,
        Err(ds) => {
            msgs.extend(ds.into_iter().map(|d| d.message));
            return (None, msgs);
        }
    };
    (Some(sigil_link::flatten(&linked, 0x00)), msgs)
}

/// Compile `emp` and panic with the diagnostics on any failure — the happy-path
/// helper for byte-pin tests.
fn emp_candidate(emp: &str) -> Vec<u8> {
    let (image, msgs) = compile_full(emp);
    image.unwrap_or_else(|| panic!("emp compile failed: {msgs:?}"))
}

fn assert_byte_identical(reference: &[u8], candidate: &[u8], what: &str) {
    if reference == candidate {
        return;
    }
    let n = reference.len().min(candidate.len());
    if let Some(i) = (0..n).find(|&i| reference[i] != candidate[i]) {
        panic!(
            "{what}: first byte diff at offset {i:#x}: reference {:#04x} != candidate {:#04x}\n\
             reference[{i:#x}..] = {:02X?}\n candidate[{i:#x}..] = {:02X?}",
            reference[i],
            candidate[i],
            &reference[i..(i + 8).min(reference.len())],
            &candidate[i..(i + 8).min(candidate.len())],
        );
    }
    panic!(
        "{what}: lengths differ — reference {} bytes, candidate {} bytes (common prefix matches)",
        reference.len(),
        candidate.len()
    );
}

// ---------------------------------------------------------------------------
// T1 — the exact port shape: aeon's `math.asm` GetSineCosine, byte-pinned
// against the real listing (s4.lst, GetSineCosine @ $2468..$2480):
//
//   0240 00FF   andi.w  #$FF, d0
//   D040        add.w   d0, d0
//   0640 0080   addi.w  #$40*2, d0
//   323B 000C   move.w  Sine_Table(pc,d0.w), d1     ; cos, ext-word VMA $2474, target $2480, disp=$C
//   0440 0080   subi.w  #$40*2, d0
//   303B 0004   move.w  Sine_Table(pc,d0.w), d0     ; sin, ext-word VMA $247C, target $2480, disp=$4
//   4E75        rts
//   <Sine_Table: 4 bytes of table data follow, standing in for the real 320-word table>
// ---------------------------------------------------------------------------

const MATH_ASM: &str = "\tcpu 68000\n\
    \tphase $2468\n\
    GetSineCosine:\n\
    \tandi.w\t#$FF, d0\n\
    \tadd.w\td0, d0\n\
    \taddi.w\t#$40*2, d0\n\
    \tmove.w\tSine_Table(pc,d0.w), d1\n\
    \tsubi.w\t#$40*2, d0\n\
    \tmove.w\tSine_Table(pc,d0.w), d0\n\
    \trts\n\
    Sine_Table:\n\
    \tdc.w\t$0000, $0647, $0C8B, $12C7\n";

const MATH_EMP: &str = "module math\n\
    section s (cpu: m68000, vma: $2468) {\n\
    \tproc GetSineCosine () {\n\
    \t\tandi.w\t#$FF, d0\n\
    \t\tadd.w\td0, d0\n\
    \t\taddi.w\t#$40*2, d0\n\
    \t\tmove.w\tSine_Table(pc,d0.w), d1\n\
    \t\tsubi.w\t#$40*2, d0\n\
    \t\tmove.w\tSine_Table(pc,d0.w), d0\n\
    \t\trts\n\
    \t}\n\
    \tdata Sine_Table: [u16; 4] = [$0000, $0647, $0C8B, $12C7]\n\
    }\n";

#[test]
fn as_reference_math_asm_matches_hand_derived_listing_bytes() {
    // Sanity: the AS reference itself matches the real listing's bytes before
    // it's used as the golden for the emp port (mirrors T1's usual precondition
    // check — a bad reference would make every downstream comparison vacuous).
    let bytes = as_reference(MATH_ASM);
    let want: Vec<u8> = vec![
        0x02, 0x40, 0x00, 0xFF, // andi.w #$FF,d0
        0xD0, 0x40, // add.w d0,d0
        0x06, 0x40, 0x00, 0x80, // addi.w #$80,d0
        0x32, 0x3B, 0x00, 0x0C, // move.w Sine_Table(pc,d0.w),d1  (cos)
        0x04, 0x40, 0x00, 0x80, // subi.w #$80,d0
        0x30, 0x3B, 0x00, 0x04, // move.w Sine_Table(pc,d0.w),d0  (sin)
        0x4E, 0x75, // rts
        0x00, 0x00, 0x06, 0x47, 0x0C, 0x8B, 0x12, 0xC7, // Sine_Table
    ];
    assert_eq!(bytes, want, "AS reference must match the real s4.lst-derived bytes");
}

#[test]
fn emp_port_matches_as_reference_math_asm() {
    let reference = as_reference(MATH_ASM);
    let candidate = emp_candidate(MATH_EMP);
    assert_byte_identical(&reference, &candidate, "math.asm GetSineCosine port");
}

// ---------------------------------------------------------------------------
// T2 — the plain `Sym(pc)` form, AS-parity byte pin (the strongest proof: two
// independent front-ends agreeing byte-for-byte, like T1 did for movem).
// `move.w Target(pc),d0` at VMA 0: ext word at VMA 2, target at VMA 8,
// disp = 8 - 2 = 6.
// ---------------------------------------------------------------------------

#[test]
fn plain_pcrel_form_matches_as_reference() {
    // Instruction is 4 bytes (opcode word + d16 ext word) at VMA 0..4; ext
    // word's own VMA = 2. Pad (3 words = 6 bytes) sits at VMA 4..10; Target at
    // VMA 10. disp = 10 - 2 = 8.
    let asm = "\tcpu 68000\n\tphase 0\n\tmove.w\tTarget(pc), d0\n\tdc.w 0,0,0\n\
               Target:\n\tdc.w $1234\n";
    let reference = as_reference(asm);
    assert_eq!(
        reference[0..4],
        [0x30, 0x3A, 0x00, 0x08],
        "AS reference: move.w (d16,PC),d0 = 30 3A, disp word 00 08"
    );

    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tTarget(pc), d0\n\
        \t}\n\
        \tdata Pad: [u16; 3] = [0, 0, 0]\n\
        \tdata Target: [u16; 1] = [$1234]\n\
        }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "plain Sym(pc) vs AS (d16,PC)");
}

// ---------------------------------------------------------------------------
// T3 — forward AND backward targets for the indexed form.
// ---------------------------------------------------------------------------

#[test]
fn indexed_pcrel_forward_target() {
    // `move.w Fwd(pc,d0.w),d1` at VMA 0: opcode word [0..2), ext word at VMA 2,
    // disp8 byte at VMA 3. Pad (2 words = 4 bytes) sits at VMA 4..8; Fwd at VMA
    // 8. disp = 8 - ext_word_vma(2) = 6.
    let asm = "\tcpu 68000\n\tphase 0\n\tmove.w\tFwd(pc,d0.w), d1\n\tdc.w 0,0\n\
               Fwd:\n\tdc.w $2222\n";
    let reference = as_reference(asm);
    assert_eq!(reference[0..4], [0x32, 0x3B, 0x00, 0x06]);

    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tFwd(pc,d0.w), d1\n\
        \t}\n\
        \tdata Pad: [u16; 2] = [0, 0]\n\
        \tdata Fwd: [u16; 1] = [$2222]\n\
        }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "indexed Sym(pc,Xn) forward target");
}

#[test]
fn indexed_pcrel_backward_target() {
    // `Back` (2 bytes) at VMA 0, `Pad` (2 bytes) at VMA 2, instruction at VMA
    // 4: ext word's own VMA = 6. Back is at VMA 0, disp = 0 - 6 = -6.
    let asm = "\tcpu 68000\n\tphase 0\n\
               Back:\n\tdc.w $3333\n\tdc.w 0\n\
               \tmove.w\tBack(pc,d0.w), d1\n";
    let reference = as_reference(asm);
    assert_eq!(&reference[4..8], &[0x32, 0x3B, 0x00, 0xFA]); // -6 as u8 = 0xFA

    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tdata Back: [u16; 1] = [$3333]\n\
        \tdata Pad: [u16; 1] = [0]\n\
        \tproc p () {\n\
        \t\tmove.w\tBack(pc,d0.w), d1\n\
        \t}\n\
        }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "indexed Sym(pc,Xn) backward target");
}

// ---------------------------------------------------------------------------
// T4 — `.l` index form (`Sine_Table(pc,d0.l)`), AS-parity byte pin. The long
// index bit (bit 11 of the brief ext word) is set.
// ---------------------------------------------------------------------------

#[test]
fn indexed_pcrel_long_index_matches_as_reference() {
    let asm = "\tcpu 68000\n\tphase 0\n\tmove.w\tFwd(pc,d0.l), d1\n\tdc.w 0,0\n\
               Fwd:\n\tdc.w $4444\n";
    let reference = as_reference(asm);
    // brief ext word: D/A=0(d), reg=0(d0), size bit(11)=1 (.l), disp8=6.
    assert_eq!(reference[0..4], [0x32, 0x3B, 0x08, 0x06]);

    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tFwd(pc,d0.l), d1\n\
        \t}\n\
        \tdata Pad: [u16; 2] = [0, 0]\n\
        \tdata Fwd: [u16; 1] = [$4444]\n\
        }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "indexed .l Sym(pc,Xn.l) vs AS");
}

// ---------------------------------------------------------------------------
// T5 — errors: loud, never wrapped.
// ---------------------------------------------------------------------------

/// disp8 overflow: the indexed form's target is > 127 bytes past the ext
/// word's VMA. `far` sits 200 bytes after `p`'s single instruction (ext word
/// VMA = 2), so disp = 200 - 2 = 198, past +127.
#[test]
fn indexed_pcrel_disp8_overflow_is_loud() {
    let emp = "module m\n\
        const PadRange = 0..200\n\
        const PadArr = PadRange.map(|_| 0)\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tfar(pc,d0.w), d1\n\
        \t}\n\
        \tdata Pad: [u8; 200] = PadArr\n\
        \tdata far: [u16; 1] = [$5555]\n\
        }\n";
    let (image, msgs) = compile_full(emp);
    assert!(image.is_none(), "disp8 overflow must fail the build");
    assert!(
        msgs.iter().any(|m| m.contains("PC,Xn") && m.contains("out of range")),
        "expected a (d8,PC,Xn) out-of-range diagnostic, got: {msgs:?}"
    );
}

/// disp16 overflow: the plain form's target is > 32767 bytes past the ext
/// word's VMA. Constructed the same way as the disp8 overflow, at 40000 bytes.
#[test]
fn plain_pcrel_disp16_overflow_is_loud() {
    let emp = "module m\n\
        const PadRange = 0..40000\n\
        const PadArr = PadRange.map(|_| 0)\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tfar(pc), d0\n\
        \t}\n\
        \tdata Pad: [u8; 40000] = PadArr\n\
        \tdata far: [u16; 1] = [$5555]\n\
        }\n";
    let (image, msgs) = compile_full(emp);
    assert!(image.is_none(), "disp16 overflow must fail the build");
    assert!(
        msgs.iter().any(|m| m.contains("(d16,PC)") && m.contains("out of range")),
        "expected a (d16,PC) out-of-range diagnostic, got: {msgs:?}"
    );
}

/// A cross-section target resolves correctly — the linker's PC-rel fixup
/// resolution is pure VMA arithmetic with no section-identity restriction (see
/// module doc). Section `a` (vma $1000) holds the instruction; section `b`
/// (vma $1034, close enough to stay in disp8 range) holds the target. ext
/// word VMA = $1002; disp = $1034 - $1002 = $32 (50).
#[test]
fn pcrel_cross_section_target_resolves() {
    let emp = "module m\n\
        section a (cpu: m68000, vma: $1000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tTarget(pc,d0.w), d1\n\
        \t}\n\
        }\n\
        section b (cpu: m68000, vma: $1034) {\n\
        \tdata Target: [u16; 1] = [$6666]\n\
        }\n";
    let (image, msgs) = compile_full(emp);
    let image = image.unwrap_or_else(|| panic!("cross-section pc-rel must resolve: {msgs:?}"));
    // Section `a`'s bytes come first in the flat image (lowest vma): opcode
    // word 32 3B, ext word 00 32.
    assert_eq!(&image[0..4], &[0x32, 0x3B, 0x00, 0x32]);
}

/// `Sym(pc,...)` with a non-register index is diagnosed, not silently
/// misencoded.
#[test]
fn pcrel_non_register_index_is_loud() {
    // `notareg` is a syntactically valid bareword in index position, but it
    // names no register — the index-register resolver must reject it loudly
    // rather than silently miscoding some default register.
    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tTarget(pc,notareg.w), d1\n\
        \t}\n\
        \tdata Target: [u16; 1] = [$7777]\n\
        }\n";
    let (image, msgs) = compile_full(emp);
    assert!(image.is_none(), "a non-register PC-relative index must fail the build");
    assert!(
        msgs.iter().any(|m| m.contains("index register")),
        "expected an index-register diagnostic, got: {msgs:?}"
    );
}

/// An unknown symbol as the PC-relative target is a loud, name-bearing error —
/// never a silent zero displacement.
#[test]
fn pcrel_unknown_symbol_is_loud_and_names_it() {
    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\tNoSuchLabel(pc,d0.w), d1\n\
        \t}\n\
        }\n";
    let (image, msgs) = compile_full(emp);
    assert!(image.is_none(), "an unknown pc-relative target must fail the build");
    assert!(
        msgs.iter().any(|m| m.contains("NoSuchLabel")),
        "expected a diagnostic naming `NoSuchLabel`, got: {msgs:?}"
    );
}

// ---------------------------------------------------------------------------
// T6 — non-regression: a comptime CALL in operand-adjacent position still
// works (the pc-rel carve-out keys on the literal `pc` token, not on shape
// alone), and `movea.l (sym).w`-class RelaxAbsSym behavior is untouched.
// ---------------------------------------------------------------------------

#[test]
fn comptime_call_in_operand_position_unaffected_by_pcrel_carveout() {
    // `double(21)` is a genuine comptime call whose result feeds `#imm` — NOT a
    // pc-relative shape (no `pc` token), so it must still evaluate as a call.
    let emp = "module m\n\
        comptime fn double(x: int) -> int {\n    return x * 2\n}\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\t#double(21), d0\n\
        \t}\n\
        }\n";
    let candidate = emp_candidate(emp);
    // move.w #imm,d0 = 303C, imm word 002A (42).
    assert_eq!(candidate, vec![0x30, 0x3C, 0x00, 0x2A]);
}

#[test]
fn relax_abs_sym_seam_still_works_after_pcrel_addition() {
    // A bare symbolic absolute operand (no `pc`) must still ride the existing
    // RelaxAbsSym seam (abs.w/abs.l relaxation) — the new pc-rel dispatch must
    // not intercept ordinary `Sym` operands. `move.w d0, Target` is the exact
    // shape `lower_code.rs::move_w_abs_dst_emits_relax` unit-pins.
    let asm = "\tcpu 68000\n\tphase 0\n\tmove.w\td0, Target\n\tdc.w 0\n\
               Target:\n\tdc.w $1111\n";
    let reference = as_reference(asm);

    let emp = "module m\n\
        section s (cpu: m68000, vma: $000000) {\n\
        \tproc p () {\n\
        \t\tmove.w\td0, Target\n\
        \t}\n\
        \tdata Pad: [u16; 1] = [0]\n\
        \tdata Target: [u16; 1] = [$1111]\n\
        }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "RelaxAbsSym seam post-pcrel");
}
