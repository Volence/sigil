//! Tranche 10 (1a) — the REAL `dplc.emp` port, region-level byte gate.
//!
//! `animate_port.rs`'s sibling for the TENTH code port (first of the two-file
//! tranche): compiles the ACTUAL ported file from aeon's tree —
//! `engine/objects/dplc.emp` — through the production parse -> lower -> place
//! -> resolve -> link pipeline, and asserts the `dplc` section's flattened
//! bytes equal the reference ROM window at the pinned addresses, in BOTH build
//! shapes.
//!
//! ## What this port exercises
//!
//! - **The leanest cross-seam set of the campaign** — dplc reads NO RAM cells,
//!   NO engine constants, NO game-contract symbols. Its only cross-seam surface
//!   is the two `jsr QueueDMA_{Important,Deferrable}` targets (bare names,
//!   width-selected to abs.w). The ambient deps are types + sst only.
//! - **Indexed EA `adda.w (a2,d0.w), a2`** and the `movem.l d2-d4/a2-a3`
//!   save/restore around the DMA call.
//! - **Two near-identical procs** (`Perform_DPLC` / `Perform_DPLC_Deferrable`,
//!   differing only in the QueueDMA target) — transcribed verbatim, NOT
//!   dedup'd (that's a step-2/3 retrospect item).
//! - **No SOUND / no DEBUG divergence** — the region len is shape-INVARIANT
//!   (item 11's carry-return restructure grew both shapes equally; item 6's
//!   single-entry assert was REMOVED after the oracle soak disproved the
//!   invariant, so dplc carries no DEBUG-only code). Single AS-twin check.
//!
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Both windows come from `pins::DPLC` at run time — base and length, per
//! shape. The numbers are deliberately not restated here: a bound copied into
//! prose is executed by nothing, so nothing can go red when it rots.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, or
//! `EMPYREAN_SUITE_ROOT`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test dplc_port
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
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// Per-shape geometry + TRUE cross-seam VMAs (sourced from
/// `sigil_harness::pins` — regenerate via repin).
struct Shape {
    base: u32,
    len: usize,
    /// `(name, vma)` for every INBOUND label this shape references.
    labels: &'static [(&'static str, u32)],
}

const PLAIN: Shape = Shape {
    base: pins::DPLC.plain_base,
    len: pins::DPLC.plain_len,
    labels: &[
        ("QueueDMA_Important", pins::QUEUE_DMA_IMPORTANT.plain),
        ("QueueDMA_Deferrable", pins::QUEUE_DMA_DEFERRABLE.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::DPLC.debug_base,
    len: pins::DPLC.debug_len,
    labels: &[
        ("QueueDMA_Important", pins::QUEUE_DMA_IMPORTANT.debug),
        ("QueueDMA_Deferrable", pins::QUEUE_DMA_DEFERRABLE.debug),
    ],
};

/// Parse one `.emp` file to an AST, failing loudly on parse errors.
fn parse_file(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

/// One synthetic file: `deps`' items prepended to `main`'s own, under `main`'s
/// module header (the ambient-injection technique).
fn with_ambient(
    deps: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(main.items);
    sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    }
}

/// The AS-side value seam: SST struct equs + the engine constants twin (sst.emp
/// pulls its 30 drift guards; dplc.emp itself declares no constants).
fn as_constant_equs() -> Vec<Section> {
    sigil_harness::test_support::as_engine_constants_and_sst_equs()
}

/// One synthetic AS-side label phased at `vma` — a `dc.b 0` carrier whose LABEL
/// address is load-bearing (the `jsr QueueDMA_*` operands must resolve to the
/// real per-shape addresses).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — a bare `jsr Perform_DPLC` from an AS unit
/// (undefined in-unit; the `.emp` owns it). Proves the `pub proc` export
/// surfaces as a bare link symbol relaxing to the abs.w encoding.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tjsr     Perform_DPLC\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// The map: a `text` region for the zero-byte default-section carrier, and the
/// real `dplc` region pinned at the per-shape base.
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
         name = \"dplc\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile the real `dplc.emp` with its ambient dependencies (types + sst),
/// place it at the per-shape base, append the synthetic cross-seam sections,
/// and link.
fn compile_real_file(
    shape: &Shape,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let dplc = parse_file(&aeon.join("engine/objects/dplc.emp"));

    let file = with_ambient(vec![types, sst], dplc);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "dplc.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs()];
    for (name, vma) in shape.labels {
        groups.push(as_label_at(name, *vma));
    }
    groups.push(as_outbound_consumer());
    for group in &mut groups {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// The prepended drift guards must be captured and PASS. dplc.emp `use`s only
/// `engine.objects.sst`, so the ambient deps are types + sst — sst.emp's 30
/// SST_* guards ride along; dplc.emp declares no constants and pulls no
/// constants.emp twin, so NO engine-constant guards appear here.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    assert_eq!(
        guards, 0,
        "sst.emp's SST_* drift wall retired at the conv-a structs flip; dplc pulls no constants twin"
    );
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus context on each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES, a region gate over an empty window \
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
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch, candidate {} bytes, expected {} bytes",
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

/// The region reference gate + cross-seam pins + the outbound bare-name proof +
/// the drift guards, shared body.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(shape);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("dplc").expect("linked image must carry dplc");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("dplc vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );

    // Outbound bare-name proof: the AS-side bare `jsr Perform_DPLC` must relax
    // to the abs.w encoding (`4EB8 base`). The consumer is the LAST synthetic
    // group: equ blob + N labels + consumer.
    let consumer_lma = 0x0100_0000u32 + (1 + shape.labels.len() as u32) * 0x10_0000;
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    assert_eq!(
        &consumer.bytes[0..4],
        &[0x4E, 0xB8, (shape.base >> 8) as u8, shape.base as u8],
        "bare-name proof: `jsr Perform_DPLC` must relax to abs.w at the region base"
    );
}

/// (plain) the `dplc` region == the `s4.bin` window at `pins::DPLC`'s plain
/// base/len.
#[test]
fn dplc_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) the `dplc` region == the `s4.debug.bin` window at `pins::DPLC`'s
/// debug base/len.
#[test]
fn dplc_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// The AS-twin oracle RETIRED (flip Stage-2): dplc.asm is deleted — the .emp is
// the only source. dplc's region correctness is subsumed by the region gates
// (dplc == frozen-golden slice) + the native whole-ROM golden gates.

// ===========================================================================
// Tranche 20 — the QueueDMA ownership FLIP (proof-mechanism feed-forward):
// dplc.emp's `jbsr QueueDMA_{Important,Deferrable}` re-resolve from the old
// .asm owner to the newly-ported engine.dma_queue module. Both modules compile
// together — NO QueueDMA address carriers, NO extern decls (deleted this
// tranche; kill-list rows 31/32) — and BOTH regions must still byte-match the
// reference ROM. The t15 section/entity_window two-module test is the template.
// ===========================================================================

fn flip_value_equs() -> Vec<Section> {
    // Union of dplc's seam (SST + engine constants) and dma_queue's
    // (engine.vdp bits/ports + the structs/constants twin truths) — ONE equ
    // carrier (assemble_equ_pairs emits a `Stub` label; two groups collide).
    let mut pairs: Vec<(&str, &str)> = vec![
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
        // VDP_Shadow offset twins (engine.vdp shadow-offset block)
        ("VDP_Shadow_vdp_mode1", "$00"),
        ("VDP_Shadow_vdp_mode2", "$01"),
        ("VDP_Shadow_vdp_mode3", "$0B"),
        ("VDP_Shadow_vdp_hint_rate", "$0A"),
    ];
    pairs.extend(sigil_harness::test_support::sst_field_equs());
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

fn flip_lower_and_place(
    emp_path: &std::path::Path,
    ambient: Vec<sigil_frontend_emp::ast::File>,
    include_root: PathBuf,
    region: &str,
    base: u32,
    len: usize,
    debug: bool,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let main = parse_file(emp_path);
    let file = with_ambient(ambient, main);
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(include_root),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} lower errors: {ldiags:?}",
        emp_path.display()
    );
    let mt = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"{region}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&mt).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} place errors: {pdiags:?}",
        emp_path.display()
    );
    (sections, module.link_asserts)
}

fn two_module_flip(debug: bool, rom_name: &str) {
    let aeon = aeon_dir();
    let Ok(refrom) = std::fs::read(aeon.join(rom_name)) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {rom_name}");
        }
        eprintln!("skip: reference ROM {rom_name} missing");
        return;
    };

    let dplc_base = if debug { pins::DPLC.debug_base } else { pins::DPLC.plain_base };
    let (mut dplc_sections, dplc_asserts) = flip_lower_and_place(
        &aeon.join("engine/objects/dplc.emp"),
        vec![
            parse_file(&aeon.join("engine/system/types.emp")),
            parse_file(&aeon.join("engine/objects/sst.emp")),
        ],
        aeon.join("engine/objects"),
        "dplc",
        dplc_base,
        if debug { pins::DPLC.debug_len } else { pins::DPLC.plain_len },
        debug,
    );

    let dq_base = if debug { pins::DMA_QUEUE.debug_base } else { pins::DMA_QUEUE.plain_base };
    let dq_len = if debug { pins::DMA_QUEUE.debug_len } else { pins::DMA_QUEUE.plain_len };
    let (mut dq_sections, dq_asserts) = flip_lower_and_place(
        &aeon.join("engine/system/dma_queue.emp"),
        vec![
            parse_file(&aeon.join("engine/structs.emp")),
            parse_file(&aeon.join("engine/system/constants.emp")),
            parse_file(&aeon.join("engine/vdp.emp")),
        ],
        aeon.join("engine/system"),
        "dma_queue",
        dq_base,
        dq_len,
        debug,
    );

    let mut sections = Vec::new();
    sections.append(&mut dplc_sections);
    sections.append(&mut dq_sections);

    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut labels: Vec<(&str, u32)> = vec![
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
        // P2c Task 8 byte cap seam (P-3 family).
        ("DMA_Enq_Bytes_Frame", pick(pins::DMA_ENQ_BYTES_FRAME)),
    ];
    if debug {
        // aeon's DMA straddle counter — the whole family, so a later cell needs no edit.
        // DEBUG-only cells, derived from the reference listing — see `listing_vma`.
        labels.push(("Dbg_DMA_Straddle_All", sigil_harness::test_support::listing_vma(debug, "Dbg_DMA_Straddle_All")));
        labels.push(("Dbg_DMA_Straddle_Frame", sigil_harness::test_support::listing_vma(debug, "Dbg_DMA_Straddle_Frame")));
        labels.push(("Dbg_DMA_Straddle_Peak", sigil_harness::test_support::listing_vma(debug, "Dbg_DMA_Straddle_Peak")));
        labels.push(("DMA_Overflow_Count", pins::DMA_OVERFLOW_COUNT));
        labels.push(("Dbg_DMA_Enq_Capped", pins::DBG_DMA_ENQ_CAPPED));
    }

    // The DEBUG-only DMA instrumentation family, swept from the reference listing so a
    // new diagnostic cell needs no edit here; rows above keep their pinned value.
    // See `test_support::extend_from_listing`.
    let mut labels: Vec<(String, u32)> =
        labels.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    if debug {
        sigil_harness::test_support::extend_from_listing(&mut labels, debug, &["Dbg_DMA_", "DMA_Peak_", "DMA_Split_"]);
    }

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![flip_value_equs()];
    for (name, vma) in labels {
        groups.push(as_label_at(&name, vma));
    }
    for group in &mut groups {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    let mut all = dplc_asserts;
    all.extend(dq_asserts);
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &all);
    assert!(
        adiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "drift guards: {adiags:?}"
    );

    let shape = if debug { "debug" } else { "plain" };
    let dplc_len = if debug { pins::DPLC.debug_len } else { pins::DPLC.plain_len };
    let dr = &refrom[dplc_base as usize..dplc_base as usize + dplc_len];
    let dsec = linked.section("dplc").expect("dplc region").bytes.clone();
    assert_region_matches(&dsec, dr, &format!("dplc ({shape} flip)"));
    let qr = &refrom[dq_base as usize..dq_base as usize + dq_len];
    let qsec = linked.section("dma_queue").expect("dma_queue region").bytes.clone();
    // m1-budget-fix grew Drain_Budgeted_Queue: dma_queue's content ends 2 bytes
    // before the 4-aligned next module (Init_SpriteTable), so the pinned region
    // window carries a short trailing alignment fill. Compare through the
    // fill-tolerant helper (the dma_queue CONTENT still matches byte-for-byte).
    assert_region_matches(&qsec, qr, &format!("dma_queue ({shape} flip)"));
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
