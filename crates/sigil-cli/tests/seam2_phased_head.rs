//! seam-2 OQ-5 — THE PHASED-HEAD PLACEMENT MODE (the first hard test).
//!
//! The banked engine-table HEAD (`seq_opcode_tab`, `dac_sample_tab`, the generated
//! LUTs) is PHASED: its labels live at the `$8000` window (`vma:$8000+offset`) but
//! its bytes are placed physically INSIDE the `$58000` song bank (`lma` in a high
//! bank). Row-1620 blocker 3 flagged this VMA≠LMA-in-a-high-bank mode as UNPROBED —
//! neither `z80_init.emp` (`vma:$0`, LMA 0) nor `dac_samples.emp` (`bank:$8000`,
//! VMA==LMA) matched it.
//!
//! This test STANDS THE MODE UP on the REAL `seq_opcode_tab.emp`, co-linked against
//! the REAL resident-blob handler VMAs (seam-1's `native_sound_blob`), and proves:
//!   (1) the head's Value16Le cells resolve to the RESIDENT handler VMAs
//!       ($0-based phase-0 addresses), independent of the head's own high LMA —
//!       i.e. placing the head in the $58000 bank does NOT drag its cell targets
//!       to the bank; a cross-section ref resolves to the target's VMA (Pass 1 of
//!       `link` defines every symbol at `vma_origin + offset`);
//!   (2) the head's OWN bytes land at the bank LMA (`LinkedSection.lma`);
//!   (3) the head's own label (`SeqOpcodeTable`) resolves to its `$8000`-WINDOW
//!       VMA, NOT its bank LMA (the phase decoupling the reader depends on — the
//!       sequencer indexes `SeqOpcodeTable` as a window pointer).
//!
//! The machinery is the SAME `vma_base`/`lma` decoupling seam-1 uses for the
//! resident blob (VMA 0 / LMA $3DE); this proves it at (VMA $8000 / LMA $58000).
//! The cell resolution is placement-INVARIANT: the phased head emits the identical
//! bytes as the windowed oracle (`seq_opcode_tab_port`), so the scale-1 byte gate's
//! proof carries to the scale-2 placement with no new drift surface.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_phased_head
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{SymbolValue, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The REAL resident-blob handler VMAs (seam-1's exported contract) as a stub
/// symbol table — exactly what the whole-ROM co-link makes available to the head
/// (the resident sections define these; here they arrive as the seam-1 contract).
fn resident_handler_stubs(debug: bool) -> (SymbolTable, Vec<(String, u32)>) {
    let blob = sigil_harness::seam1::native_sound_blob(&aeon_dir(), debug);
    let mut syms = SymbolTable::new();
    for (name, vma) in &blob.symbols {
        syms.define(name, SymbolValue::Int(*vma as i64));
    }
    (syms, blob.symbols)
}

/// Lower the REAL `seq_opcode_tab.emp`, place it via `map`, co-link against the
/// resident handler stubs. Returns `(section_bytes, section_lma, seqtab_vma)`.
fn link_head_at(map_toml: &str, debug: bool) -> (Vec<u8>, u32, i64) {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("seq_opcode_tab.emp"))
        .unwrap_or_else(|e| panic!("read seq_opcode_tab.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "seq_opcode_tab.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "seq_opcode_tab.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = sigil_link::load_map(map_toml).expect("map loads");
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    assert!(
        pd.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pd:?}"
    );

    let (stubs, _) = resident_handler_stubs(debug);
    let resolved = sigil_link::resolve_layout(&sections, &stubs, true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &stubs)
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    let sec = linked.section("seq_opcode_tab").expect("linked seq_opcode_tab");

    // The head's own label VMA, from the resolved (placed) section: vma_origin +
    // the SeqOpcodeTable label offset (0 — it anchors the table's first cell).
    let placed = resolved.iter().find(|s| s.name == "seq_opcode_tab").expect("placed section");
    let origin = placed.vma_origin();
    let seqtab_off = placed
        .labels
        .iter()
        .find(|l| l.name == "SeqOpcodeTable")
        .map(|l| l.offset)
        .expect("SeqOpcodeTable label");
    (sec.bytes.clone(), sec.lma, (origin + seqtab_off) as i64)
}

/// The WINDOWED placement (the scale-1 oracle shape): VMA == LMA == the window
/// ($8000). The `.emp` section's `vma: $8000` and the map's `lma_base = 0x8000`
/// coincide, so labels and bytes share the window address (the pre-seam-2 world).
const MAP_WINDOWED: &str = "\
fill = 0x00

[[region]]
name = \"seq_opcode_tab\"
lma_base = 0x8000
size = 0x100
kind = \"rom\"
";

/// The PHASED-HEAD placement (the scale-2 seam): the WINDOW VMA is owned by the
/// `.emp` section attribute (`section seq_opcode_tab (cpu: z80, vma: $8000)`), so
/// its labels resolve to the $8000 window; the map/emitter places the BYTES
/// PHYSICALLY in the $58000 song bank (`lma_base = 0x5856D`). This is the mode
/// row-1620 called unprobed — VMA (source-owned) ≠ LMA (placement-owned), the LMA
/// deep in a high bank. `0x5856D` = a representative physical address inside the
/// engine-table bank. (A `vma_base` in the map would be OVERRIDDEN by the section's
/// own `vma:` attr, so the source is the single source of truth for the window.)
const MAP_PHASED: &str = "\
fill = 0x00

[[region]]
name = \"seq_opcode_tab\"
lma_base = 0x5856D
size = 0x100
kind = \"rom\"
";

#[test]
fn phased_head_cells_resolve_to_resident_vmas_bytes_at_bank_lma() {
    if !strict_gate() {
        eprintln!("skip: seam2_phased_head not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    for debug in [false, true] {
        let (phased_bytes, phased_lma, seqtab_vma) = link_head_at(MAP_PHASED, debug);

        // (2) the head's bytes land at the bank LMA (physical placement in $58000).
        assert_eq!(
            phased_lma, 0x5856D,
            "phased head bytes must be placed at the $58000-bank LMA (debug={debug})"
        );
        // (3) the head's own label resolves to its $8000-WINDOW VMA (source-owned),
        //     NOT the bank LMA. The isolated table anchors at the window base $8000;
        //     in the full head it sits at $8000 + the preceding tables' size.
        assert_eq!(
            seqtab_vma, 0x8000,
            "SeqOpcodeTable must resolve to its window VMA ($8000), not its bank LMA (debug={debug})"
        );

        // (1) the cells resolve to the RESIDENT handler VMAs — anchor the first cell
        //     ($E0 = MEV_VOL → Seq_Op_Vol) against seam-1's real exported VMA.
        let (_, symbols) = resident_handler_stubs(debug);
        let vol_vma = symbols
            .iter()
            .find(|(n, _)| n == "Seq_Op_Vol")
            .map(|(_, v)| *v)
            .expect("Seq_Op_Vol in the resident contract");
        assert_eq!(
            &phased_bytes[0..2],
            &[(vol_vma & 0xFF) as u8, ((vol_vma >> 8) & 0xFF) as u8],
            "first cell must be Seq_Op_Vol's RESIDENT VMA little-endian, not a bank address (debug={debug})"
        );
        // Every cell is a resident phase-0 address (< $8000), never dragged into
        // the head's own bank — a $58000-region byte pair would exceed the window.
        assert_eq!(phased_bytes.len(), 64, "32 × dc.w = 64 bytes (debug={debug})");
    }
}

#[test]
fn phased_head_emits_identical_bytes_to_the_windowed_oracle() {
    if !strict_gate() {
        eprintln!("skip: seam2_phased_head not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    // Cell resolution is PLACEMENT-INVARIANT: co-linked against the same resident
    // VMAs, the phased head (LMA in the $58000 bank) emits byte-for-byte the same
    // table as the windowed oracle (VMA==LMA==window). So the scale-1 byte gate
    // (`seq_opcode_tab_port`) carries to the scale-2 placement — no new drift.
    for debug in [false, true] {
        let (windowed, windowed_lma, windowed_seqtab) = link_head_at(MAP_WINDOWED, debug);
        let (phased, phased_lma, phased_seqtab) = link_head_at(MAP_PHASED, debug);
        assert_eq!(windowed, phased, "phased head bytes must equal the windowed oracle (debug={debug})");
        // The placements DIFFER only in the physical LMA; the window VMA is identical.
        assert_ne!(windowed_lma, phased_lma, "the two placements have distinct LMAs (debug={debug})");
        assert_eq!(windowed_seqtab, phased_seqtab, "both resolve SeqOpcodeTable to the same window VMA (debug={debug})");
        assert_eq!(phased_seqtab, 0x8000, "the source's `vma: $8000` owns the window VMA in both placements (debug={debug})");
    }
}
