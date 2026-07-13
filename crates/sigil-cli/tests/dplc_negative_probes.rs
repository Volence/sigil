//! Tranche 10 (1a) — negative probes: `dplc_port.rs`'s gate must fail LOUDLY
//! when violated, mirroring `hblank_negative_probes.rs`'s one-file-per-tranche
//! house style.
//!
//! Each probe doctors ONE input so ONE specific guard fires, and — implicitly,
//! by the test PASSING rather than aborting — that no probe itself panics
//! uncontrolled. The real `dplc.emp` file is read but never written; every
//! probe doctors a COPY.
//!
//! ## Probes
//!
//! (a) genuineness — a doctored COPY (`lsr.w #4, d3` -> `lsr.w #3, d3`, a
//!     1-bit count-field change) produces DIFFERENT linked bytes than the
//!     reference, proving `dplc_port.rs`'s byte-diff actually fires rather than
//!     trivially matching by construction.
//! (b) standalone-compile diagnostic — compile the real `dplc.emp` WITHOUT the
//!     synthetic `QueueDMA_*` cross-seam sections: `resolve_layout` fails LOUD
//!     naming the missing symbol (the `RelaxAbsSym` Item-C cross-seam framing).
//! (c) placement genuineness — a wrong-base map (base+2) places the section at
//!     a different address, so a byte-diff against the FIXED reference window
//!     would fail — proving the byte-diff is a real placement check.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::{Path, PathBuf};

/// The `dplc.emp` module's own directory (honors `AEON_DIR`).
fn dplc_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/objects")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The ambient deps prepended so `Sst.<field>(a0)` resolves — types + sst,
/// under dplc.emp's module header (the ambient-injection technique).
fn dplc_with_ambient(dplc_src: &str) -> sigil_frontend_emp::ast::File {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    let read = |p: PathBuf| {
        let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        let (f, d) = parse_str(&s);
        assert!(d.iter().all(|x| x.level != Level::Error), "{} parse: {d:?}", p.display());
        f
    };
    let types = read(aeon.join("engine/system/types.emp"));
    let sst = read(aeon.join("engine/objects/sst.emp"));
    let (dplc, ddiags) = parse_str(dplc_src);
    assert!(ddiags.iter().all(|x| x.level != Level::Error), "dplc parse: {ddiags:?}");
    let mut items = Vec::new();
    items.extend(types.items);
    items.extend(sst.items);
    items.extend(dplc.items);
    sigil_frontend_emp::ast::File {
        module: dplc.module.clone(),
        attrs: dplc.attrs.clone(),
        items,
        docs: dplc.docs.clone(),
    }
}

/// The real `dplc.emp` source text, or a strict-gate panic / soft skip if the
/// sibling `aeon` tree isn't present.
fn real_dplc_src() -> Option<String> {
    let path = dplc_dir().join("dplc.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: dplc.emp not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

fn map_toml(base: &str) -> String {
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
         lma_base = {base}\n\
         size = 0x98\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side `QueueDMA_*` cross-seam labels, phased to their plain
/// VMAs — `dplc_port.rs::as_label_at` verbatim.
fn as_queue_dma_labels() -> Vec<Section> {
    let mut secs = Vec::new();
    for (name, vma) in [
        ("QueueDMA_Important", pins::QUEUE_DMA_IMPORTANT.plain),
        ("QueueDMA_Deferrable", pins::QUEUE_DMA_DEFERRABLE.plain),
    ] {
        let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        secs.extend(
            assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}")).sections,
        );
    }
    secs
}

/// Parse -> lower -> place `src` at `base`, WITHOUT any cross-seam sections
/// appended. Returns the placed sections for the caller to resolve itself.
fn place_dplc(src: &str, base: &str) -> Vec<Section> {
    let file = dplc_with_ambient(src);
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dplc_dir()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    let map = sigil_link::load_map(&map_toml(base)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");
    sections
}

/// Link `sections` plus the synthetic `QueueDMA_*` cross-seam labels (probes
/// (a)/(c) need both `jbsr` targets to resolve to compile at all).
fn link_placed(mut sections: Vec<Section>) -> sigil_link::LinkedImage {
    let mut cross_seam = as_queue_dma_labels();
    let mut lma = 0x0100_0000u32;
    for sec in &mut cross_seam {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        lma += 0x10_0000;
    }
    sections.extend(cross_seam);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

// ===========================================================================
// Probe (a) — GENUINENESS
// ===========================================================================

/// Doctor ONE instruction (`lsr.w   #4, d3` -> `lsr.w   #3, d3`, a shift-count
/// field change) in a COPY and prove the linked `dplc` section's bytes DIFFER
/// from the genuine compile — proving `dplc_port.rs`'s byte-diff gate is
/// non-vacuous.
#[test]
fn doctored_shift_count_produces_different_bytes_than_genuine() {
    let Some(src) = real_dplc_src() else { return };
    assert!(src.contains("lsr.w   #4, d3"), "precondition: the real file spells `lsr.w   #4, d3`");
    let doctored = src.replacen("lsr.w   #4, d3", "lsr.w   #3, d3", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine = link_placed(place_dplc(&src, &format!("{:#x}", pins::DPLC.plain_base)));
    let doctored = link_placed(place_dplc(&doctored, &format!("{:#x}", pins::DPLC.plain_base)));

    let genuine_bytes = &genuine.section("dplc").expect("dplc section").bytes;
    let doctored_bytes = &doctored.section("dplc").expect("dplc section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "a doctored `lsr.w #3` must emit different bytes than the genuine `lsr.w #4` — \
         else the byte gate could never catch this transcription class"
    );
}

// ===========================================================================
// Probe (b) — STANDALONE-COMPILE DIAGNOSTIC
// ===========================================================================

/// dplc.emp's cross-seam calls are `jbsr QueueDMA_*` (step-2 house format —
/// lowering to a pc-relative `bsr.w`), so compiling it standalone — no
/// synthetic RAM-label section supplied — must fail LOUD at `resolve_layout`
/// naming a missing branch target.
#[test]
fn standalone_compile_without_cross_seam_labels_is_a_loud_missing_symbol_error() {
    let Some(src) = real_dplc_src() else { return };
    let sections = place_dplc(&src, &format!("{:#x}", pins::DPLC.plain_base));
    // NO cross-seam labels appended — `QueueDMA_*` are genuinely absent.
    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect_err(
        "compiling dplc.emp standalone (no QueueDMA_* cross-seam sections) \
         must be a loud resolve_layout error, not a silent/panicking one",
    );
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("unresolved branch/ladder target")
                && d.message.contains("dplc")
                && d.message.contains("QueueDMA_")
        }),
        "expected the unresolved-branch-target diagnostic to name a `QueueDMA_*` symbol with \
         the section framing, got: {err:?}"
    );
}

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS
// ===========================================================================

/// Place the real `dplc.emp` at a WRONG base (`$2700` instead of the real plain
/// `$26FC`) and prove the placed section lands at a DIFFERENT VMA — so a
/// byte-diff against the FIXED reference window would fail. Placement is real,
/// not an echo of the expected value.
///
/// Since the step-2 house format, the cross-seam calls are `jbsr QueueDMA_*`
/// (pc-relative `bsr.w`), so the linked CONTENT is placement-DEPENDENT: the
/// `bsr.w` displacement is `target − site`, and the site moves with the base.
/// The two placements therefore differ in BOTH LMA and bytes (the four disp
/// bytes of the two `bsr.w` calls), each an independent genuineness signal.
#[test]
fn wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_dplc_src() else { return };

    let real_sections = place_dplc(&src, &format!("{:#x}", pins::DPLC.plain_base));
    let wrong_sections = place_dplc(&src, "0x2700");

    let real_dplc = real_sections.iter().find(|s| s.name == "dplc").expect("real dplc section");
    let wrong_dplc = wrong_sections.iter().find(|s| s.name == "dplc").expect("wrong dplc section");

    assert_eq!(real_dplc.lma, pins::DPLC.plain_base, "the real map must place dplc at $26FC");
    assert_eq!(wrong_dplc.lma, 0x2700, "the doctored map must place dplc at $2700");
    assert_ne!(
        real_dplc.lma, wrong_dplc.lma,
        "placement must genuinely move with the map base — not be an echo/hardcode"
    );

    let real_linked = link_placed(real_sections);
    let wrong_linked = link_placed(wrong_sections);
    let real_bytes = &real_linked.section("dplc").expect("dplc").bytes;
    let wrong_bytes = &wrong_linked.section("dplc").expect("dplc").bytes;
    // The pc-relative `bsr.w QueueDMA_*` disp tracks the site VMA, so the bytes
    // genuinely differ between placements (they were identical under the old
    // abs.w `jsr`, which is placement-invariant).
    assert_ne!(
        real_bytes, wrong_bytes,
        "the pc-relative jbsr disp must track placement — bytes differ with the base"
    );
    assert_ne!(
        real_linked.section("dplc").unwrap().lma,
        wrong_linked.section("dplc").unwrap().lma,
        "the LMA must differ between the two placements — placement is real, not an echo"
    );
}
