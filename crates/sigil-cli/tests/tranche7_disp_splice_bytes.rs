//! Tranche 7 — F1 byte-parity against the independent AS front-end.
//!
//! A comptime template with a spliced displacement (`{off}({reg})`), instantiated
//! in a proc, must assemble byte-identical to the hand-expanded `.asm` equivalent
//! run through `sigil-frontend-as`.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Assemble a `.asm` source through the AS front-end (68k), link, flatten.
fn as_reference(asm: &str) -> Vec<u8> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Full emp pipeline (parse -> lower -> resolve_layout -> link -> flatten).
fn emp_candidate(emp: &str) -> Vec<u8> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "emp parse errors: {:?}",
        pdiags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        !ldiags.iter().any(|d| d.level == Level::Error),
        "emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &empty, true)
        .unwrap_or_else(|d| panic!("emp resolve failed: {d:?}"));
    let linked =
        sigil_link::link(&resolved, &empty).unwrap_or_else(|d| panic!("emp link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

fn assert_byte_identical(reference: &[u8], candidate: &[u8], what: &str) {
    if reference == candidate {
        return;
    }
    let n = reference.len().min(candidate.len());
    if let Some(i) = (0..n).find(|&i| reference[i] != candidate[i]) {
        panic!(
            "{what}: first byte diff at {i:#x}: ref {:#04x} != cand {:#04x}\n ref = {:02X?}\n cand = {:02X?}",
            reference[i],
            candidate[i],
            &reference[i..(i + 8).min(reference.len())],
            &candidate[i..(i + 8).min(candidate.len())],
        );
    }
    panic!("{what}: length differ — ref {} vs cand {}", reference.len(), candidate.len());
}

// The spliced-displacement template, called twice with different displacements
// and base registers (two instantiations — hygiene exercise even without labels).
const EMP_DISP: &str = concat!(
    "module m\n",
    "comptime fn ld(boff: int, breg: Reg, dst: Reg) -> Code {\n",
    "    return asm {\n",
    "        move.w  {boff}({breg}), {dst}\n",
    "        sub.w   {boff}({breg}), {dst}\n",
    "    }\n",
    "}\n",
    "pub proc Probe () {\n",
    "    ld(2, a3, d1)\n",
    "    ld(8, a2, d0)\n",
    "    rts\n",
    "}\n",
);

const AS_DISP: &str = concat!(
    "\tcpu 68000\n",
    "Probe:\n",
    "\tmove.w\t2(a3), d1\n",
    "\tsub.w\t2(a3), d1\n",
    "\tmove.w\t8(a2), d0\n",
    "\tsub.w\t8(a2), d0\n",
    "\trts\n",
);

#[test]
fn spliced_displacement_matches_as_reference() {
    let reference = as_reference(AS_DISP);
    let candidate = emp_candidate(EMP_DISP);
    assert_byte_identical(&reference, &candidate, "spliced displacement vs AS");
}
