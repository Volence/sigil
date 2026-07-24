//! Tranche 20 — the REAL `dma_queue.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/system/dma_queue.emp` — through
//! the production parse -> lower -> place -> resolve -> link pipeline and
//! asserts the `dma_queue` region's flattened bytes equal the reference ROM
//! window at the pinned base, in BOTH build shapes. The 3-priority DMA queue:
//! movep-interleaved 14-byte entries (DMAEntry twin), the shared enqueue core
//! with the carry drop-contract, the zero-branch Critical jump-table drain,
//! and the budgeted Important/Deferrable drain with compaction.
//!
//! ## Shape
//! Shape-DEPENDENT length ($302 plain / $306 debug — the debug surplus is the
//! `if DEBUG == 1` overflow-count bump in the enqueue's `.full` arm).
//!
//! ## Cross-seam symbols
//! - RAM (abs.w operands / word immediates): the sub-queue bases + end
//!   sentinels, the three slot vars, `DMA_Budget_Remaining`, and (debug only)
//!   `DMA_Overflow_Count` — `phase`d one-byte carriers at pinned per-shape
//!   VMAs. `DMA_Queue` is emitted as an ALIAS of the `DMA_CRITICAL` pin (same
//!   VMA — ram.asm defines it as the queue block base).
//! - VALUE mirrors (equ carriers): the engine.constants DMA/ART consts, the
//!   engine.structs Act/Sec/DMAEntry field walls, and the engine.vdp bit
//!   vocabulary + port addresses — every prepended twin's `ensure` reads its
//!   AS-side truth through the link seam.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test dma_queue_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::DMA_QUEUE.debug_base } else { pins::DMA_QUEUE.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::DMA_QUEUE.debug_len } else { pins::DMA_QUEUE.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `dma_queue` region pinned at the per-shape reference base + length.
fn map_toml(debug: bool) -> String {
    let base = region_base(debug);
    let len = region_len(debug);
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
         name = \"dma_queue\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The VALUE seam: the prepended twins' drift-lock truths. `doctor` overrides
/// ONE pair (the negative probes).
fn value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        // engine.vdp port addresses + target/op bit vocabulary (its ensures)
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
    ];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    if let Some((name, val)) = doctor {
        let mut hit = false;
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
                hit = true;
            }
        }
        assert!(hit, "doctor target `{name}` not in the value seam");
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — the queue RAM family, each a `phase`d
/// one-byte carrier at its pinned per-shape VMA.
fn addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut table: Vec<(&str, u32)> = vec![
        // DMA_Queue is the queue block base — same VMA as DMA_Critical
        // (ram.asm spells both labels on the same cell).
        ("DMA_Queue", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical_End", pick(pins::DMA_CRITICAL_END)),
        ("DMA_Important", pick(pins::DMA_IMPORTANT)),
        ("DMA_Important_End", pick(pins::DMA_IMPORTANT_END)),
        ("DMA_Deferrable", pick(pins::DMA_DEFERRABLE)),
        ("DMA_Deferrable_End", pick(pins::DMA_DEFERRABLE_END)),
        ("DMA_Critical_Slot", pick(pins::DMA_CRITICAL_SLOT)),
        ("DMA_Important_Slot", pick(pins::DMA_IMPORTANT_SLOT)),
        ("DMA_Deferrable_Slot", pick(pins::DMA_DEFERRABLE_SLOT)),
        ("DMA_Budget_Remaining", pick(pins::DMA_BUDGET_REMAINING)),
    ];
    if debug {
        // Debug shape only: the overflow counter exists only in debug RAM
        // (ram.asm's `ifdef __DEBUG__` profiling block).
        table.push(("DMA_Overflow_Count", pins::DMA_OVERFLOW_COUNT));
    }
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs.drain(..) {
            s.lma = 0x0200_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

/// Parse a .emp file, panicking on parse errors.
fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        path.display()
    );
    file
}

/// Lower the real `dma_queue.emp` (prepend the engine.structs + engine.constants
/// + engine.vdp twins its `use` lines read), place into the per-shape map,
/// append the value equs + address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/system");
    let main = parse_file(&dir.join("dma_queue.emp"));
    let structs_file = parse_file(&aeon.join("engine/structs.emp"));
    let consts_file = parse_file(&dir.join("constants.emp"));
    let vdp_file = parse_file(&aeon.join("engine/vdp.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: structs_file
            .items
            .into_iter()
            .chain(consts_file.items)
            .chain(vdp_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "dma_queue.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(value_equs(doctor));
    sections.extend(addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at offset {i:#x} (region-relative)\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn run(debug: bool) {
    let aeon = aeon_dir();
    let rom_name = if debug { "s4.debug.bin" } else { "s4.bin" };
    let rom_path = aeon.join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(debug, None);
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "dma_queue.emp drift guards must all PASS: {diags:?}"
    );

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("dma_queue").expect("linked image must carry dma_queue");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("dma_queue ({shape})"));
}

#[test]
fn dma_queue_region_matches_reference() {
    run(false);
}

#[test]
fn dma_queue_debug_region_matches_reference() {
    run(true);
}

/// Negative probe: a DOCTORED `DMA_CRITICAL_SLOTS` truth (7 AS-side while the
/// twin says 8) must FIRE the engine.constants drift guard, naming the const.
#[test]
fn doctored_dma_critical_slots_fires_its_guard() {
    if !strict_gate() && !aeon_dir().join("s4.bin").exists() {
        eprintln!("skip: reference ROM missing");
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file(false, Some(("DMA_CRITICAL_SLOTS", "7")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "the doctored DMA_CRITICAL_SLOTS truth must fire a drift guard");
    assert!(
        fired.iter().any(|d| d.message.contains("DMA_CRITICAL_SLOTS")),
        "the guard must NAME the constant: {fired:?}"
    );
}

/// Negative probe: a swapped-adjacent-field DOCTOR on the DMAEntry wall
/// (`DMAEntry_SizeH` claiming offset 2) must fire the per-field drift wall —
/// the stronger-than-sizeof property the Act/Sec wall established.
#[test]
fn doctored_dmaentry_field_fires_its_guard() {
    if !strict_gate() && !aeon_dir().join("s4.bin").exists() {
        eprintln!("skip: reference ROM missing");
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file(false, Some(("DMAEntry_SizeH", "2")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "the doctored DMAEntry_SizeH truth must fire the field wall");
    assert!(
        fired.iter().any(|d| d.message.contains("SizeH")),
        "the guard must NAME the field: {fired:?}"
    );
}
