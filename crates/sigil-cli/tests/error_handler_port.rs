//! Tranche 25 — the REAL `error_handler.emp` port, region-level byte gate.
//!
//! Compiles `engine/debug/error_handler.emp` (the 12 exception-vector stubs via
//! the `raise_exception` construct + the vendored MD Debugger blob) through the
//! production parse → lower → place → resolve → link pipeline and asserts the
//! `error_handler` region's flattened bytes equal the reference ROM window at
//! `[BusError, EndOfRom)` in BOTH shapes. Length 0x10B0 both shapes (stub table
//! 0x15A + blob 0xF56) — same size, different base. Both arms are live again since
//! the crash-report ruling (owner-ruled 2026-08-04) put the island back in release.
//!
//! The `raise_exception` construct emits `jsr (MDDBG__ErrorHandler).l` / `jmp
//! (MDDBG__ErrorHandler_PagesController).l`, and the blob's extension-button
//! pointers are `dc.l MDDBG__Debugger_AddressRegisters/Backtrace`. All four are
//! now `pub equ`s OWNED BY error_handler.emp (`extern("ErrorHandlerBlob") + off`),
//! so the module self-resolves them at link — no synthetic injection (conv-i #7).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`). Absent, the
//! gates SKIP green unless `SIGIL_STRICT_GATE=1`.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

struct Shape {
    base: u32,
    len: usize,
    rom: &'static str,
    debug: i128,
}

// BOTH canonical shapes carry the island since the crash-report ruling (owner-ruled
// 2026-08-04): the MD Debugger + its deb2 symbol table are DIAGNOSTICS and they ship,
// so `s4.bin` has a real 0x10B0 error_handler region again and the PLAIN arm is back.
// (It was dropped by review item 29 part 4, when plain_len was 0.) The module has no
// internal `if DEBUG`, so the two arms compile identical CONTENT at different bases —
// which is exactly what makes the plain arm worth running: it proves the island
// RELOCATES correctly, and that the MDDBG__* equs (all `extern("ErrorHandlerBlob") +
// off`) re-fold against the release base rather than carrying debug addresses into a
// shipped ROM.
const PLAIN: Shape =
    Shape { base: pins::ERROR_HANDLER.plain_base, len: pins::ERROR_HANDLER.plain_len, rom: "s4.bin", debug: 0 };
const DEBUG: Shape =
    Shape { base: pins::ERROR_HANDLER.debug_base, len: pins::ERROR_HANDLER.debug_len, rom: "s4.debug.bin", debug: 1 };

fn map_toml(base: u32, len: usize) -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"error_handler\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile(shape: &Shape) -> sigil_link::LinkedImage {
    let aeon = aeon_dir();
    let src = std::fs::read_to_string(aeon.join("engine/debug/error_handler.emp"))
        .unwrap_or_else(|e| panic!("read error_handler.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "error_handler.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/debug")),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), shape.debug), ("SOUND_DRIVER_ENABLED".to_string(), 1)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "error_handler.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // The four MDDBG__* handler entry points (the raise_exception jsr/jmp targets
    // + the blob's dc.l extension-button pointers) are error_handler.emp's own
    // `pub equ`s off ErrorHandlerBlob — the module self-resolves them, no injection.
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base. Tolerate a short (< 16 B)
    // all-zero tail beyond the lowered image; every real byte still compares.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 16
        && expected[candidate.len()..].iter().all(|&b| b == 0)
    {
        &expected[..candidate.len()]
    } else {
        expected
    };
    assert_eq!(candidate.len(), expected.len(), "{what}: length mismatch");
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at region offset {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn reference_gate(shape: &Shape) {
    let rom_path = aeon_dir().join(shape.rom);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let linked = compile(shape);
    let section = linked.section("error_handler").expect("linked image carries error_handler");
    let base = shape.base as usize;
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("error_handler vs {}[{base:#x}..{:#x}]", shape.rom, base + shape.len),
    );
}

#[test]
fn error_handler_region_matches_reference() {
    reference_gate(&PLAIN);
}

#[test]
fn error_handler_debug_region_matches_reference() {
    reference_gate(&DEBUG);
}

/// The 12 exception-vector labels flip to `.emp` ownership. vectors.asm (stays
/// `.asm`, out of scope) references all 12 via `dc.l`; with error_handler.asm
/// gated out, those references must resolve to the `.emp`-owned stub labels. This
/// proves the ownership flip in isolation: assemble a synthetic vector table
/// (`dc.l BusError, AddressError, …`) on the AS side, link it against the placed
/// error_handler.emp, and confirm every entry resolves to the `.emp` label's VMA.
/// (Bare-symbol references resolve — unlike the derived-equ table above.)
#[test]
fn vector_labels_resolve_to_emp_ownership() {
    let aeon = aeon_dir();
    if !strict_gate() && !aeon.join("s4.bin").exists() {
        eprintln!("skip: aeon tree not present");
        return;
    }
    const STUBS: &[&str] = &[
        "BusError", "AddressError", "IllegalInstr", "ZeroDivide", "ChkInstr", "TrapvInstr",
        "PrivilegeViol", "Trace", "Line1010Emu", "Line1111Emu", "ErrorExcept", "ErrorTrap",
    ];
    // Lower + place error_handler.emp at the DEBUG base. Its content carries no
    // internal DEBUG conditional, so the ownership flip this test checks is
    // shape-independent — one arm suffices (the byte gates above cover both bases).
    let src = std::fs::read_to_string(aeon.join("engine/debug/error_handler.emp"))
        .unwrap_or_else(|e| panic!("read error_handler.emp: {e}"));
    let (file, _) = parse_str(&src);
    let (module, _) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("engine/debug")),
            embed_base: None,
            defines: vec![("DEBUG".to_string(), 1), ("SOUND_DRIVER_ENABLED".to_string(), 1)],
        },
    );
    let map = sigil_link::load_map(&map_toml(DEBUG.base, DEBUG.len)).expect("map loads");
    let mut sections = module.sections;
    place_sections(&mut sections, &map);

    // Synthetic vector table (the vectors.asm dc.l class) referencing the 12
    // stub labels as externs — the AS side that must resolve against the .emp.
    let vec_src = format!(
        "cpu 68000\nphase $1000000\nVecTable:\n\tdc.l {}\n",
        STUBS.join(", ")
    );
    let mut vec_secs = assemble(&vec_src, &AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() })
        .unwrap_or_else(|d| panic!("assemble vectors: {d:?}"))
        .sections;
    for s in vec_secs.iter_mut() {
        s.lma = 0x0200_0000;
        s.placement = SectionPlacement::Pinned;
        s.group = None;
    }
    let vec_name = vec_secs[0].name.clone();
    sections.append(&mut vec_secs);

    // The stubs' raise_exception jsr/jmp targets are error_handler.emp's own
    // MDDBG__* pub equs (off ErrorHandlerBlob) — self-resolved, no injection.
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("flip resolve failed: {d:?}"));
    // The .emp stub label VMAs (the expected resolutions).
    let emp_vma = |want: &str| -> u32 {
        for sec in &resolved {
            if sec.name != "error_handler" {
                continue;
            }
            for l in &sec.labels {
                if l.name == want {
                    return sec.vma_origin().wrapping_add(l.offset);
                }
            }
        }
        panic!("error_handler.emp must export {want}");
    };
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("flip link failed: {d:?}"));
    let vt = linked.section(&vec_name).expect("linked image carries the vector table");
    for (i, name) in STUBS.iter().enumerate() {
        let got = u32::from_be_bytes(vt.bytes[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(
            got,
            emp_vma(name),
            "vector entry {i} (dc.l {name}) must resolve to the .emp-owned label"
        );
    }
}

/// t25 capability note (ledgered demand-1) — NOW DELIVERED (Flip Stage 2): sigil
/// resolves an AS-side `X: equ ExternalSym + const` at link, folding the derived
/// symbol `X` off the external base once `ExternalSym` is provided by another
/// module (equ-off-link-external-base). This is what lets the MDDBG__ equ table
/// derive off the `.emp`-owned `ErrorHandler` (aliased to `ErrorHandlerBlob`) —
/// engine.inc's gate-ON arm no longer needs a NUMERIC per-shape `ErrorHandler`
/// equ (rows 52/90 nativization). The front-end emits the unresolvable-RHS equate
/// as a deferred symbolic `equ_sym` (eval.rs `directive_equate`); `fold_equ_syms`
/// folds it post-placement.
#[test]
fn derived_equ_off_external_base_resolves() {
    use sigil_ir::SymbolValue;
    let asm = "cpu 68000\nMDDBG__X: equ ErrorHandler+$128\n dc.l MDDBG__X\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let m = assemble(asm, &opts).expect("assemble tolerates the external-base equ (defers)");
    let mut st = SymbolTable::new();
    st.define("ErrorHandler", SymbolValue::Int(0x5CC0A));
    let resolved = sigil_link::resolve_layout(&m.sections, &st, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &st)
        .expect("derived equ off an external base now resolves (the MDDBG__ flip capability)");
    // The `dc.l MDDBG__X` folds to ErrorHandler ($5CC0A) + $128 = $5CD32.
    let sec = &linked.sections[0];
    let got = u32::from_be_bytes(sec.bytes[0..4].try_into().unwrap());
    assert_eq!(got, 0x5CC0A + 0x128, "MDDBG__X must fold off the external ErrorHandler base");
}
