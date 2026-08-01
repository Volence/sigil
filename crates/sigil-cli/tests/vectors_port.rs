//! Tranche 26 (lane A) — the REAL `vectors.emp` port, region-level byte gate.
//!
//! Compiles `engine/system/vectors.emp` (the engine-owned 64-entry CPU/interrupt/
//! trap vector table, `org 0`, exactly $000-$0FF = 256 bytes) through the
//! production parse → lower → place → resolve → link pipeline and asserts the
//! `vectors` region's flattened bytes equal the reference ROM window at
//! `[0, 0x100)` in BOTH shapes. The region is FIXED SIZE and PRECEDES EVERYTHING
//! — a byte delta is impossible by construction — but the gate states the bar.
//!
//! The 64 entries are raw `dc.l` link pointers. SYSTEM_STACK is a constants.asm
//! equ; the other 63 are labels resolved at link — EntryPoint (boot.emp), the 12
//! exception stubs (error_handler.emp), VBlank_Handler (vblank.emp) are .emp-
//! owned; NullInterrupt (engine.inc inline) and HBlank_Vector_Slot (RAM
//! trampoline) stay AS-side. All 17 distinct targets are fed here as synthetic
//! pinned carriers at their per-shape listing VMAs (sourced from
//! `sigil_harness::pins`).
//!
//! P-A1 (step-0 blocking probe): `dc.l` accepts comma-lists of LABELS (the four
//! `dc.l ErrorTrap, ErrorTrap, ErrorTrap, ErrorTrap` lines, vectors.asm:40-47).
//! `dc_l_label_comma_list_resolves_each_element` proves it link-time (the real
//! binding class) with a positive control. VERDICT: already supported —
//! `lower_dc` loops over all operands, one `Cell::SymRef` per label element.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`). Absent, the
//! reference gates SKIP green unless `SIGIL_STRICT_GATE=1`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test vectors_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

const REGION_LEN: usize = pins::VECTORS.plain_len; // 0x100, shape-invariant (fixed region)

/// The 17 distinct cross-seam vector targets at their per-shape VMAs.
/// SYSTEM_STACK is a constants.asm equ ($FFFFFF00 both shapes) — fed as a phased
/// carrier so `dc.l SYSTEM_STACK` resolves to that value just like a label.
fn vector_targets(debug: bool) -> Vec<(&'static str, u32)> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    vec![
        ("SYSTEM_STACK", 0xFFFFFF00),
        ("EntryPoint", pick(pins::ENTRY_POINT)),
        ("BusError", pick(pins::BUS_ERROR)),
        ("AddressError", pick(pins::ADDRESS_ERROR)),
        ("IllegalInstr", pick(pins::ILLEGAL_INSTR)),
        ("ZeroDivide", pick(pins::ZERO_DIVIDE)),
        ("ChkInstr", pick(pins::CHK_INSTR)),
        ("TrapvInstr", pick(pins::TRAPV_INSTR)),
        ("PrivilegeViol", pick(pins::PRIVILEGE_VIOL)),
        ("Trace", pick(pins::TRACE)),
        ("Line1010Emu", pick(pins::LINE1010_EMU)),
        ("Line1111Emu", pick(pins::LINE1111_EMU)),
        ("ErrorExcept", pick(pins::ERROR_EXCEPT)),
        ("ErrorTrap", pick(pins::ERROR_TRAP)),
        ("VBlank_Handler", pick(pins::V_BLANK_HANDLER)),
        ("NullInterrupt", pick(pins::NULL_INTERRUPT)),
        ("HBlank_Vector_Slot", pick(pins::H_BLANK_VECTOR_SLOT)),
    ]
}

/// One phased one-byte carrier per (name, vma), each on its own harness-private
/// LMA (the error_handler_port synthetic-handler pattern).
fn carriers(targets: &[(&str, u32)], start_lma: u32) -> Vec<Section> {
    let mut out = Vec::new();
    let mut lma = start_lma;
    for (name, vma) in targets {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble (carrier {name}): {d:?}"))
            .sections;
        for sec in &mut secs {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        out.extend(secs);
        lma += 0x1_0000;
    }
    out
}

fn map_toml() -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"vectors\"\n\
         lma_base = {:#x}\n\
         size = {:#x}\n\
         kind = \"rom\"\n",
        pins::VECTORS.plain_base, pins::VECTORS.plain_len
    )
}

/// Compile vectors.emp, place at [0, 0x100), link against the synthetic targets.
fn compile(debug: bool) -> sigil_link::LinkedImage {
    let aeon = aeon_dir();
    let src = std::fs::read_to_string(aeon.join("engine/system/vectors.emp"))
        .unwrap_or_else(|e| panic!("read vectors.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "vectors.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/system")),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "vectors.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = sigil_link::load_map(&map_toml()).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    sections.extend(carriers(&vector_targets(debug), 0x0100_0000));
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

fn reference_gate(debug: bool, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let linked = compile(debug);
    let section = linked.section("vectors").expect("linked image carries vectors");
    assert_eq!(section.bytes.len(), REGION_LEN, "vectors region length");
    let expected = &refrom[0..REGION_LEN];
    if let Some(i) = (0..REGION_LEN).find(|&i| section.bytes[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(REGION_LEN);
        panic!(
            "vectors vs {rom_name}[0..0x100]: first diff at {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &section.bytes[lo..hi],
            &expected[lo..hi]
        );
    }
}

#[test]
fn vectors_region_matches_reference() {
    reference_gate(false, "s4.bin");
}

#[test]
fn vectors_debug_region_matches_reference() {
    reference_gate(true, "s4.debug.bin");
}

// ---------------------------------------------------------------------------
// P-A1 (step-0 blocking probe): `dc.l` comma-list of LABELS resolves each
// element as an independent link fixup (vectors.asm:40-47's four-per-line form).
// Binding class: LINK-TIME (the four labels are supplied as pinned AS carriers,
// not comptime consts) — the real vectors site's class. Positive control:
// undoctored matches the four VMAs; doctored diverges.
// ---------------------------------------------------------------------------

/// Compile a synthetic `dc.l L0, L1, L2, L3` .emp snippet with the four L
/// labels supplied at the given VMAs; return the 16 emitted bytes.
fn compile_comma_list(vmas: [u32; 4]) -> Vec<u8> {
    let src = "module probe.dcl in probe\npub proc Probe () clobbers() {\n\tdc.l L0, L1, L2, L3\n}\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "probe parse: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "probe lower: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = "fill = 0x00\n\n[[region]]\nname = \"probe\"\nlma_base = 0x1000\nsize = 0x10\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("probe map");
    let mut sections = module.sections;
    place_sections(&mut sections, &map);
    let targets: Vec<(&str, u32)> =
        vec![("L0", vmas[0]), ("L1", vmas[1]), ("L2", vmas[2]), ("L3", vmas[3])];
    sections.extend(carriers(&targets, 0x0200_0000));
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("probe resolve: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("probe link: {d:?}"));
    linked.section("probe").expect("probe region").bytes.clone()
}

#[test]
fn dc_l_label_comma_list_resolves_each_element() {
    let vmas = [0x0000_1234u32, 0x00AB_CDEF, 0x0055_0000, 0x0000_00FF];
    let bytes = compile_comma_list(vmas);
    assert_eq!(bytes.len(), 16, "four dc.l elements = 16 bytes");
    for (i, vma) in vmas.iter().enumerate() {
        let got = u32::from_be_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(got, *vma, "comma-list element {i} must resolve to its label VMA (big-endian)");
    }
}

/// Positive control for the probe: a DOCTORED third element must change the
/// emitted bytes (proves the elements genuinely ride the link fixup, not a
/// coincidence) — and the undoctored compile equals the four VMAs (above).
#[test]
fn dc_l_label_comma_list_doctored_diverges() {
    let base = [0x0000_1234u32, 0x00AB_CDEF, 0x0055_0000, 0x0000_00FF];
    let doctored = [0x0000_1234u32, 0x00AB_CDEF, 0x00DE_AD00, 0x0000_00FF];
    assert_ne!(
        compile_comma_list(base),
        compile_comma_list(doctored),
        "doctoring the third label VMA must change the emitted bytes"
    );
}

// ---------------------------------------------------------------------------
// The FIRST .emp→.emp VECTOR reference (both gate states): vectors.emp's 12
// exception entries resolve to error_handler.emp's .emp-owned stub labels when
// the two modules are compiled TOGETHER (gate-ON, module-to-module). The
// reverse of t25's `vector_labels_resolve_to_emp_ownership` (which drove a
// SYNTHETIC vectors table); here the REAL vectors.emp is the consumer.
// Gate-OFF (error_handler AS-side) is the region gate above (synthetic carriers
// → reference addresses).
// ---------------------------------------------------------------------------
#[test]
fn vector_labels_resolve_to_error_handler_emp() {
    let aeon = aeon_dir();
    if !strict_gate() && !aeon.join("s4.bin").exists() {
        eprintln!("skip: aeon tree not present");
        return;
    }
    const STUBS: &[&str] = &[
        "BusError", "AddressError", "IllegalInstr", "ZeroDivide", "ChkInstr", "TrapvInstr",
        "PrivilegeViol", "Trace", "Line1010Emu", "Line1111Emu", "ErrorExcept", "ErrorTrap",
    ];
    // vectors.emp placed at [0, 0x100).
    let v_src = std::fs::read_to_string(aeon.join("engine/system/vectors.emp"))
        .unwrap_or_else(|e| panic!("read vectors.emp: {e}"));
    let (v_file, _) = parse_str(&v_src);
    let (v_mod, _) = lower_module(
        &v_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("engine/system")),
            embed_base: None,
            defines: vec![("DEBUG".to_string(), 0)],
        },
    );
    // error_handler.emp placed at the plain error_handler base (the flip is
    // shape-independent — vectors.emp spells the same labels in both shapes).
    let eh_src = std::fs::read_to_string(aeon.join("engine/debug/error_handler.emp"))
        .unwrap_or_else(|e| panic!("read error_handler.emp: {e}"));
    let (eh_file, _) = parse_str(&eh_src);
    let (eh_mod, _) = lower_module(
        &eh_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("engine/debug")),
            embed_base: None,
            defines: vec![("DEBUG".to_string(), 0), ("SOUND_DRIVER_ENABLED".to_string(), 1)],
        },
    );
    let eh_base = pins::ERROR_HANDLER.plain_base;
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"vectors\"\nlma_base = 0x0\nsize = 0x100\nkind = \"rom\"\n\n[[region]]\nname = \"error_handler\"\nlma_base = {eh_base:#x}\nsize = {:#x}\nkind = \"rom\"\n",
        pins::ERROR_HANDLER.plain_len
    );
    let map = sigil_link::load_map(&map).expect("map loads");
    let mut sections = v_mod.sections;
    sections.extend(eh_mod.sections);
    place_sections(&mut sections, &map);

    // error_handler.emp's raise_exception jsr/jmp targets + blob dc.l pointers
    // (the MDDBG__ handler entry points) are now the module's OWN `pub equ`s (off
    // ErrorHandlerBlob, conv-i #7) — it self-resolves them, no injection. Only the
    // vectors' NON-error targets are fed here (EntryPoint, VBlank_Handler,
    // NullInterrupt, HBlank_Vector_Slot, SYSTEM_STACK).
    let mut extra: Vec<(&str, u32)> = vec![
        ("SYSTEM_STACK", 0xFFFFFF00),
        ("EntryPoint", pins::ENTRY_POINT.plain),
        ("VBlank_Handler", pins::V_BLANK_HANDLER.plain),
        ("NullInterrupt", pins::NULL_INTERRUPT.plain),
        ("HBlank_Vector_Slot", pins::H_BLANK_VECTOR_SLOT.plain),
    ];
    // Do NOT carry the 12 stub names — they must resolve to error_handler.emp.
    extra.retain(|(n, _)| !STUBS.contains(n));
    sections.extend(carriers(&extra, 0x0300_0000));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("flip resolve failed: {d:?}"));
    // The .emp stub label VMAs (expected resolutions).
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
    let vt = linked.section("vectors").expect("linked image carries vectors");
    // Entry offsets of the 12 exception stubs in the vector table (byte offset):
    // $08 BusError, $0C AddressError, $10 IllegalInstr, $14 ZeroDivide,
    // $18 ChkInstr, $1C TrapvInstr, $20 PrivilegeViol, $24 Trace,
    // $28 Line1010Emu, $2C Line1111Emu, $30 ErrorExcept, $80 ErrorTrap.
    let entry_off: &[(usize, &str)] = &[
        (0x08, "BusError"),
        (0x0C, "AddressError"),
        (0x10, "IllegalInstr"),
        (0x14, "ZeroDivide"),
        (0x18, "ChkInstr"),
        (0x1C, "TrapvInstr"),
        (0x20, "PrivilegeViol"),
        (0x24, "Trace"),
        (0x28, "Line1010Emu"),
        (0x2C, "Line1111Emu"),
        (0x30, "ErrorExcept"),
        (0x80, "ErrorTrap"),
    ];
    for (off, name) in entry_off {
        let got = u32::from_be_bytes(vt.bytes[*off..*off + 4].try_into().unwrap());
        assert_eq!(
            got,
            emp_vma(name),
            "vectors entry at {off:#x} (dc.l {name}) must resolve to the .emp-owned label"
        );
    }
}
