//! T1 — the Z80 operand model end-to-end: `.emp` Z80 source → image bytes,
//! checked against the asl-golden bytes (`sigil-isa`'s `z80.rs` +
//! `z80_golden_vectors.txt`) for the same instruction. These are FRONTEND tests
//! — the mapper (`eval/asm.rs`) + the Z80 lowering (`lower/code.rs`) — not new
//! ISA facts. The design note is
//! `docs/superpowers/notes/2026-07-28-z80-t1-operand-model.md`.
//!
//! Every code fragment rides in a `proc` inside a `(cpu: z80, vma: $0)` section:
//! a proc is the vehicle whose body runs the SAME `eval_asm` → `lower_code_buf`
//! path a raw `asm { }` block uses, so the operand model is exercised
//! identically. Each proc ends in `ret` (`0xC9`) — a Z80 terminator, so no
//! `[proc.undeclared-fallthrough]` — and the tests pin the leading bytes.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` for the 68k default (a Z80 section opts in explicitly),
/// asserting a clean parse. Returns the module and the lowering diagnostics.
fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

/// Every fixup across a named section's data fragments.
fn fixups_of(module: &Module, name: &str) -> Vec<Fixup> {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.fixups.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The linked bytes of a named section.
fn section_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

/// Wrap one Z80 body line in a `(cpu: z80, vma: $0)` section proc `P`, lower it,
/// and return the section's linked bytes (the proc body + the trailing `ret`).
fn z80_body(body: &str) -> Vec<u8> {
    let src = format!(
        "module m\nsection s (cpu: z80, vma: $0) {{\n  proc P() {{\n    {body}\n    ret\n  }}\n}}\n"
    );
    let (module, diags) = lower(&src);
    assert!(diags.is_empty(), "lower diagnostics for `{body}`: {diags:?}");
    section_bytes(&module, "s")
}

/// Lower one Z80 body line (same `(cpu: z80, vma: $0)` proc wrapper as
/// [`z80_body`]) and return the lowering diagnostics WITHOUT asserting they are
/// empty — for the negative controls.
fn z80_body_diags(body: &str) -> Vec<Diagnostic> {
    let src = format!(
        "module m\nsection s (cpu: z80, vma: $0) {{\n  proc P() {{\n    {body}\n    ret\n  }}\n}}\n"
    );
    lower(&src).1
}

// ---- item 1: the probe -----------------------------------------------------

#[test]
fn probe_ld_a_imm8() {
    // `ld a, 5` = 3E 05 (z80.rs golden `ld r,n`), then `ret` = C9.
    assert_eq!(z80_body("ld a, 5"), vec![0x3E, 0x05, 0xC9]);
}

// ---- item 2: one test per §1.1 operand-form row (bytes from the golden) -----

#[test]
fn ld_sp_hl() {
    // pair ← pair: F9.
    assert_eq!(z80_body("ld sp, hl"), vec![0xF9, 0xC9]);
}

#[test]
fn ld_indhl_reg() {
    // (hl) ← reg: 77.
    assert_eq!(z80_body("ld (hl), a"), vec![0x77, 0xC9]);
}

#[test]
fn ld_indhl_imm8() {
    // (hl) ← imm8 ($E9 = 233, the asl `0E9h`): 36 E9.
    assert_eq!(z80_body("ld (hl), $E9"), vec![0x36, 0xE9, 0xC9]);
}

#[test]
fn pop_ix() {
    // index pair pop: DD E1.
    assert_eq!(z80_body("pop ix"), vec![0xDD, 0xE1, 0xC9]);
}

#[test]
fn pop_de() {
    // plain pair pop: D1.
    assert_eq!(z80_body("pop de"), vec![0xD1, 0xC9]);
}

#[test]
fn ld_i_a() {
    // RegI ← A: ED 47.
    assert_eq!(z80_body("ld i, a"), vec![0xED, 0x47, 0xC9]);
}

#[test]
fn ld_r_a() {
    // RegR ← A: ED 4F.
    assert_eq!(z80_body("ld r, a"), vec![0xED, 0x4F, 0xC9]);
}

#[test]
fn ex_af_afshadow() {
    // shadow swap: 08.
    assert_eq!(z80_body("ex af, af'"), vec![0x08, 0xC9]);
}

#[test]
fn im_1() {
    // mode-1 imm: ED 56.
    assert_eq!(z80_body("im 1"), vec![0xED, 0x56, 0xC9]);
}

#[test]
fn jp_indhl() {
    // jump (hl): E9.
    assert_eq!(z80_body("jp (hl)"), vec![0xE9, 0xC9]);
}

#[test]
fn xor_a() {
    // one-op ALU, reg A (A's code is 7, so AF, not A8): AF.
    assert_eq!(z80_body("xor a"), vec![0xAF, 0xC9]);
}

#[test]
fn ld_bc_imm16_comptime() {
    // pair ← comptime imm16 ($1234): 01 34 12 (little-endian). Proves the
    // form-directed imm8-vs-imm16 split (a pair destination forces imm16).
    assert_eq!(z80_body("ld bc, $1234"), vec![0x01, 0x34, 0x12, 0xC9]);
}

// ---- item 3: symbolic imm16 → 2-byte LE hole + Value16Le fixup (§4) ---------

/// `ld hl, Target` with a label operand cannot fold at eval — it defers to link
/// as a 2-byte little-endian hole. FRAGMENT level: exactly one `Value16Le`
/// fixup (byte width 2) targeting the bare symbol `Target`, with the two
/// immediate bytes reserved as zero.
#[test]
fn symbolic_ld_hl_defers_value16le_fixup() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld hl, Target\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    let fixups = fixups_of(&module, "code");
    assert_eq!(fixups.len(), 1, "one symbolic fixup, got: {fixups:?}");
    assert_eq!(fixups[0].kind, FixupKind::Value16Le);
    assert_eq!(fixups[0].kind.byte_width(), 2);
    assert_eq!(fixups[0].target, Expr::Sym("Target".into()));
    assert_eq!(fixups[0].offset, 1, "the imm hole is the last 2 of the 3-byte `21 nn nn`");
}

/// LINKED level: `Target` resolves to VMA $1234; the `ld hl,nn` immediate is
/// written little-endian (`34 12`) — the whole `code` section is `21 34 12 C9`.
#[test]
fn symbolic_ld_hl_links_little_endian() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld hl, Target\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(section_bytes(&module, "code"), vec![0x21, 0x34, 0x12, 0xC9]);
}

// ---- item 4: the module `(cpu: z80)` attribute (§5) ------------------------

/// (a) `module m in s (cpu: z80)` lowers its top-level (default-section) code as
/// Z80 — the module attribute seeds `initial_cpu`, so a top-level proc's Z80
/// mnemonics encode without any per-section `(cpu:)`.
#[test]
fn module_cpu_z80_lowers_top_level_as_z80() {
    let src = "module m in s (cpu: z80)\n\
               proc P() {\n\
                 ld a, 5\n\
                 ret\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(section_bytes(&module, "s"), vec![0x3E, 0x05, 0xC9]);
}

/// (b) a `(cpu: z80)` module that opens a `(cpu: m68000)` section is the
/// `[module.cpu-mismatch]` error naming both CPUs.
#[test]
fn module_cpu_z80_opening_m68000_section_mismatches() {
    let src = "module m (cpu: z80)\n\
               section blk (cpu: m68000, vma: $0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[module.cpu-mismatch]")
            && d.message.contains("z80")
            && d.message.contains("m68000")),
        "expected a module.cpu-mismatch naming both CPUs, got: {diags:?}"
    );
}

/// (c) an OMITTED module attribute defaults to M68000 with no warn — so a Z80
/// mnemonic under it stays unrecognized on the existing loud 68k path (NOT a
/// silent lower, NOT a `[lower.z80-unsupported]`).
#[test]
fn module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized() {
    let src = "module m\n\
               proc P() {\n\
                 ld a, 5\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("not a recognized 68000 mnemonic")),
        "expected the existing 68k unrecognized-mnemonic error, got: {diags:?}"
    );
    // And no misfired default warn about the omitted attribute.
    assert!(
        !diags.iter().any(|d| d.message.contains("cpu-mismatch")
            || d.message.to_lowercase().contains("module cpu")),
        "an omitted module (cpu:) must not warn: {diags:?}"
    );
}

/// `jp Label` also defers its 16-bit target through the same `Value16Le` path
/// (the immediate is the last 2 bytes of `C3 nn nn`).
#[test]
fn symbolic_jp_defers_and_links() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   jp Target\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $ABCD) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    let fixups = fixups_of(&module, "code");
    assert_eq!(fixups.len(), 1);
    assert_eq!(fixups[0].kind, FixupKind::Value16Le);
    assert_eq!(section_bytes(&module, "code"), vec![0xC3, 0xCD, 0xAB]);
}

// ---- item 5: positive/negative controls (the probe must be able to fail) ----

/// A 68k register (`Value::Reg`) spliced into a Z80 section is the CPU/kind
/// disagreement `[asm.splice-kind]` error — the check actually fires (t24
/// positive-control rule). Reached by a `Reg`-param comptime fn (which binds
/// `d0` to a `Value::Reg`) instantiated in a Z80 proc.
#[test]
fn reg68k_spliced_in_z80_section_is_splice_kind_error() {
    let src = "module m in s (cpu: z80)\n\
               comptime fn splice68(r: Reg) -> Code {\n\
                 return asm {\n\
                   ld {r}, a\n\
                 }\n\
               }\n\
               proc P() {\n\
                 splice68(d0)\n\
                 ret\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[asm.splice-kind]")),
        "expected a splice-kind error for a 68k register in a Z80 section, got: {diags:?}"
    );
}

/// `(ix+127)` is accepted — the i8 window's positive edge maps cleanly to a
/// `DD 7E 7F` (`ld a,(ix+d)`, base `ld r,(hl)`), no range error.
#[test]
fn indexed_disp_127_accepted() {
    assert_eq!(z80_body("ld a, (ix+127)"), vec![0xDD, 0x7E, 0x7F, 0xC9]);
}

/// `(ix+128)` is one past the i8 window — the same `-128..=127` check the 68k
/// brief-extension disp8 path runs. A loud range error, no bytes.
#[test]
fn indexed_disp_128_is_i8_range_error() {
    let diags = z80_body_diags("ld a, (ix+128)");
    assert!(
        diags.iter().any(|d| d.message.contains("-128..=127") && d.message.contains("128")),
        "expected an (ix+d) i8-range error, got: {diags:?}"
    );
}

/// A form the corpus does NOT demand — `bit 0,(ix+d)`, whose bit number needs a
/// `Z80Bit` operand the T1 model does not yet produce — still reports
/// `[lower.z80-unsupported]`. Proves T1's wired scope is BOUNDED: it never
/// silently accepts un-oracled bytes.
#[test]
fn unwired_bit_indexed_is_z80_unsupported() {
    let diags = z80_body_diags("bit 0, (ix+1)");
    assert!(
        diags.iter().any(|d| d.message.contains("[lower.z80-unsupported]")),
        "expected [lower.z80-unsupported] for the unwired bit form, got: {diags:?}"
    );
}
