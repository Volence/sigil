//! Port #2 Task 3 — negative probes for `controllers_port.rs` + `math_port.rs`.
//! Mirrors `hblank_negative_probes.rs`'s one-file-per-tranche house style
//! (all probes for the batch live in ONE file), reusing its three probe
//! classes for EACH file where they apply:
//!
//! (a) genuineness — a doctored COPY of the emp source produces DIFFERENT
//!     linked bytes than the reference, proving the byte-diff gate is
//!     non-vacuous. `controllers.emp`: `eor.b d0, d3` -> `eor.b d3, d0` (the
//!     FIRST occurrence — the second, the P2 pad, is left alone so the probe
//!     stays a single-bit-field change). `math.emp`: `add.w d0, d0` dropped
//!     entirely (mirrors hblank's dropped-instruction doctor shape).
//! (b) standalone-compile missing-symbol diagnostic — compile the real file
//!     WITHOUT its synthetic cross-seam sections: `resolve_layout` fails
//!     LOUD, naming the missing symbol with the Item-C cross-seam-standalone
//!     wording (the same improved diagnostic hblank's Task 5 follow-up
//!     shipped). `controllers.emp` has FOUR candidate missing symbols
//!     (`HW_PORT_1_DATA`/`HW_PORT_2_DATA` equs, `Ctrl_*` RAM labels) — this
//!     probe supplies NEITHER cross-seam section and pins the diagnostic
//!     naming the FIRST one the relaxation fixpoint reports.
//!     `math.emp` carries NO cross-seam INBOUND reference at all (its only
//!     external dependency is the embed, resolved at LOWER time, not link
//!     time) — so this probe class does not apply to `math.emp` and is
//!     skipped for it (noted explicitly below, not silently omitted).
//! (c) placement genuineness — a wrong-base map moves the section; the
//!     placed LMA genuinely tracks the map, not an echo/hardcode.
//!
//! ## Keep-copies convention (per `hblank_negative_probes.rs`)
//!
//! Self-contained: small per-file helpers here are LOCAL rather than shared
//! through a harness crate. The real `.emp` files are read but never
//! written to; every probe doctors a COPY.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

fn engine_system_dir() -> PathBuf {
    aeon_dir().join("engine/system")
}

/// The real `controllers.emp`/`math.emp` source text, or a strict-gate panic
/// / soft skip if the sibling `aeon` tree isn't present — mirrors
/// `hblank_negative_probes.rs::real_hblank_src` exactly, parameterized over
/// the file name.
fn real_src(file_name: &str) -> Option<String> {
    let path = engine_system_dir().join(file_name);
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: {file_name} not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

// ===========================================================================
// controllers.emp maps + cross-seam helpers (mirrors controllers_port.rs)
// ===========================================================================

/// The region's real plain base and extent, read from `pins` (repin
/// regenerates them from the build's own listing) so a cartridge re-layout
/// cannot leave a stale literal behind.
fn controllers_base() -> u32 {
    pins::CONTROLLERS.plain_base
}

/// A base the region is provably NOT at, derived from the real one so it can
/// never coincide with it — the placement probes need a second, wrong base,
/// and a hand-typed one could silently become the real base at a re-layout.
fn controllers_wrong_base() -> u32 {
    controllers_base() + 6
}

fn controllers_map_toml(base: &str) -> String {
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
         name = \"controllers\"\n\
         lma_base = {base}\n\
         size = {size:#x}\n\
         kind = \"rom\"\n",
        size = pins::CONTROLLERS.plain_len
    )
}

/// `engine/constants.asm:17-18` (`HW_PORT_*_DATA`) PLUS `:89-92` (`BUTTON_*`)
/// verbatim — `engine.constants`'s eight drift-guard `ensure`s (riding along
/// via `constants_ambient_items`) read the `BUTTON_*` four back through
/// `extern(...)`, so they need real equs to check against here too (mirrors
/// `controllers_port.rs`'s `as_hw_port_equs`).
fn as_hw_port_equs() -> Vec<Section> {
    // The engine.constants twin's surviving drift-guard truth (`VDP_Shadow_len`),
    // shared via `sigil_harness::test_support`, PLUS the two hardware-port link
    // symbols `controllers.emp` reads as bare `lea` operands — .emp-owned consts
    // post the P5 flip, no longer in the shrunk constants-twin blob (SOURCE OF
    // TRUTH: `engine/constants.asm:17-18`).
    let mut pairs = sigil_harness::test_support::engine_constant_equs();
    pairs.push(("HW_PORT_1_DATA", "$A10003"));
    pairs.push(("HW_PORT_2_DATA", "$A10005"));
    // input-6button (2026-08-02): the burst's Z80 fence references the bare
    // link-resolved bus register (the vblank_port VALUE-seam precedent).
    pairs.push(("Z80_BUS_REQUEST", "$A11100"));
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

fn as_ctrl_ram_labels() -> Vec<Section> {
    // Pin-sourced rather than hardcoded, and TWO phased runs: the controller block,
    // plus the four HELD raw shadows at the RAM tail (aeon character-lens-sweep,
    // 2026-08-13). Read_Controllers writes the shadows every physical VBlank and
    // VInt_Level alone latches them into the published Ctrl_*_Held, so a lag VBlank
    // cannot overwrite a running tick's input — which means `controllers` references
    // the shadows and a standalone compile must define them.
    let asm = format!(
        "cpu 68000\n\
               phase ${:X}\n\
               Ctrl_1_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_2_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_1_Press_Accum:\n\
               \tdc.b 0\n\
               Ctrl_2_Press_Accum:\n\
               \tdc.b 0\n\
               Ctrl_1_Ext_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_2_Ext_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_1_Ext_Press_Accum:\n\
               \tdc.b 0\n\
               Ctrl_2_Ext_Press_Accum:\n\
               \tdc.b 0\n\
               Pad_1_Type:\n\
               \tdc.b 0\n\
               Pad_2_Type:\n\
               \tdc.b 0\n\
               phase ${:X}\n\
               Ctrl_1_Held_Raw:\n\
               \tdc.b 0\n\
               Ctrl_2_Held_Raw:\n\
               \tdc.b 0\n\
               Ctrl_1_Ext_Held_Raw:\n\
               \tdc.b 0\n\
               Ctrl_2_Ext_Held_Raw:\n\
               \tdc.b 0\n",
        sigil_harness::pins::CTRL_1_HELD.plain,
        sigil_harness::pins::CTRL_1_HELD_RAW.plain
    );
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (ctrl ram labels): {d:?}")).sections
}


/// `constants.emp`'s items (its six `pub const`s + six drift-guard
/// `ensure`s), read fresh each call so a doctored `src` never shares mutable
/// state with another probe. Mirrors `controllers_port.rs`'s
/// `controllers_with_ambient_constants` — `controllers.emp` now `use`s
/// `engine.constants`, and plain `lower_module` (used here, not the
/// whole-program resolver — see `controllers_port.rs`'s doc comment for why)
/// never resolves cross-module `use`, so the twin's items are prepended by
/// hand before lowering.
fn constants_ambient_items() -> Vec<sigil_frontend_emp::ast::Item> {
    let src = std::fs::read_to_string(engine_system_dir().join("constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (file, cdiags) = parse_str(&src);
    assert!(cdiags.iter().all(|d| d.level != Level::Error), "constants.emp parse errors: {cdiags:?}");
    file.items
}

/// input-6button (2026-08-02): `controllers.emp` splices `stop_z80()`/`start_z80()`
/// around the burst — `engine/z80_bus.emp`'s items must be ambient for the
/// standalone lower (same rationale as `constants_ambient_items`).
fn z80_bus_ambient_items() -> Vec<sigil_frontend_emp::ast::Item> {
    let src = std::fs::read_to_string(engine_system_dir().join("../z80_bus.emp"))
        .unwrap_or_else(|e| panic!("cannot read z80_bus.emp: {e}"));
    let (file, zdiags) = parse_str(&src);
    assert!(zdiags.iter().all(|d| d.level != Level::Error), "z80_bus.emp parse errors: {zdiags:?}");
    file.items
}

/// Parse `src` (a possibly-doctored copy of `controllers.emp`) -> prepend
/// `engine.constants`'s items so the `use`d `BUTTON_*`/`HW_PORT_*_DATA`
/// consts resolve -> lower (module-dir include_root, NO defines) -> place at
/// `base` into the controllers map. Returns the placed sections AND
/// `engine.constants`'s six drift-guard link asserts (captured before
/// `place_sections` consumes `module.sections`), so the drift-guard probe
/// below can `check_link_asserts` against a doctored AS-side equ.
fn place_controllers_with_asserts(src: &str, base: &str) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let merged = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: constants_ambient_items()
            .into_iter()
            .chain(z80_bus_ambient_items())
            .chain(file.items)
            .collect(),
        docs: file.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(engine_system_dir()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&merged, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&controllers_map_toml(base)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");
    (sections, link_asserts)
}

/// Parse `src` (a possibly-doctored copy of `controllers.emp`) -> prepend
/// `engine.constants`'s items so the `use`d `BUTTON_*`/`HW_PORT_*_DATA`
/// consts resolve -> lower (module-dir include_root, NO defines) -> place at
/// `base` into the controllers map. Returns the placed sections WITHOUT any
/// cross-seam section appended, so each probe controls exactly what's added.
fn place_controllers(src: &str, base: &str) -> Vec<Section> {
    place_controllers_with_asserts(src, base).0
}

/// Link `sections` plus BOTH synthetic cross-seam sections (equs + RAM
/// labels) at harness-private LMAs — both probes (a)/(c) need
/// `Read_Controllers`'s operands to resolve to compile at all.
fn link_controllers_placed(mut sections: Vec<Section>) -> sigil_link::LinkedImage {
    let mut hw_equs = as_hw_port_equs();
    for sec in &mut hw_equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(hw_equs);
    // One private LMA PER SECTION: `phase` starts a new section and the RAM labels
    // now come in two disjoint runs (controller block + the RAM-tail HELD shadows),
    // so a single shared LMA collides ("overlap in the image (colliding pins)").
    // The LMAs are harness-private; only the phased VMAs matter to the link.
    let mut ram_labels = as_ctrl_ram_labels();
    for (i, sec) in ram_labels.iter_mut().enumerate() {
        sec.lma = 0x0200_0000 + (i as u32) * 0x0001_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(ram_labels);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

// ===========================================================================
// math.emp maps (mirrors math_port.rs)
// ===========================================================================

/// `math`'s real plain base and extent — same `pins` sourcing as
/// `controllers_base`.
fn math_base() -> u32 {
    pins::MATH.plain_base
}

/// A base `math` is provably NOT at, derived from the real one.
fn math_wrong_base() -> u32 {
    math_base() + 6
}

fn math_map_toml(base: &str) -> String {
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
         name = \"math\"\n\
         lma_base = {base}\n\
         size = {size:#x}\n\
         kind = \"rom\"\n",
        size = pins::MATH.plain_len
    )
}

/// Parse -> lower (`include_root` = `engine/`, `embed_base` = the aeon ROOT) ->
/// place `src` at `base` into the math map.
///
/// `embed_base` is the project root because that is the convention every `embed`
/// path in the engine is written in — `math.emp` spells its tables
/// `embed("engine/data/sine.bin")`, root-relative. It must match what
/// `math_port.rs` passes: this probe compiles the same file, so a base that
/// disagrees with the port's does not test a weaker version of the port, it
/// tests a different program.
fn place_math(src: &str, base: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon_dir().join("engine")),
        embed_base: Some(aeon_dir()),
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors (embed?): {ldiags:?}");

    let map = sigil_link::load_map(&math_map_toml(base)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");
    sections
}

fn link_math_placed(sections: Vec<Section>) -> sigil_link::LinkedImage {
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

// ===========================================================================
// Probe (a) — GENUINENESS
// ===========================================================================

/// Doctor ONE instruction (`eor.b d0, d3` -> `eor.b d3, d0`, a register-field
/// swap: opcode `B301` -> the operand order in the source flips) in a COPY
/// of `controllers.emp` and prove the linked `controllers` section's bytes
/// DIFFER from the genuine reference-shaped compile.
///
/// FALSIFIED (restore-real-value): re-ran with the doctor reverted — the two
/// compiles produce IDENTICAL bytes, confirmed by temporarily asserting
/// `assert_eq!` on the unmodified pair and observing it hold, then reverting
/// to the doctored comparison below.
#[test]
fn controllers_doctored_eor_operand_order_produces_different_bytes_than_genuine() {
    let Some(src) = real_src("controllers.emp") else { return };
    assert!(src.contains("eor.b   d0, d3"), "precondition: the real file spells `eor.b   d0, d3` (input-6button edge merge)");
    let doctored = src.replacen("eor.b   d0, d3", "eor.b   d3, d0", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine_linked = link_controllers_placed(place_controllers(&src, &format!("{:#x}", controllers_base())));
    let doctored_linked = link_controllers_placed(place_controllers(&doctored, &format!("{:#x}", controllers_base())));

    let genuine_bytes = &genuine_linked.section("controllers").expect("controllers section").bytes;
    let doctored_bytes = &doctored_linked.section("controllers").expect("controllers section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "a doctored `eor.b d3, d0` must emit different bytes than the genuine `eor.b d0, d3` — \
         else the byte gate could never catch this transcription class"
    );
}

/// Doctor `math.emp` by DROPPING the `add.w d0, d0` line entirely (mirrors
/// `hblank_negative_probes.rs`'s dropped-instruction doctor shape) and prove
/// the linked `math` section's bytes DIFFER from the genuine compile.
///
/// FALSIFIED (restore-real-value): re-ran with the doctor reverted — the two
/// compiles produce IDENTICAL bytes, confirmed the same way as the
/// controllers probe above.
#[test]
fn math_doctored_dropped_add_produces_different_bytes_than_genuine() {
    let Some(src) = real_src("math.emp") else { return };
    assert!(src.contains("add.w   d0, d0\n"), "precondition: the real file spells `add.w   d0, d0`");
    let doctored = src.replacen("add.w   d0, d0\n", "", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine_linked = link_math_placed(place_math(&src, &format!("{:#x}", math_base())));
    let doctored_linked = link_math_placed(place_math(&doctored, &format!("{:#x}", math_base())));

    let genuine_bytes = &genuine_linked.section("math").expect("math section").bytes;
    let doctored_bytes = &doctored_linked.section("math").expect("math section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "dropping `add.w d0, d0` must emit different bytes than the genuine file — \
         else the byte gate could never catch this transcription class"
    );
}

// ===========================================================================
// Probe (b) — STANDALONE-COMPILE DIAGNOSTIC
// ===========================================================================

/// `controllers.emp` compiled standalone — NEITHER synthetic cross-seam
/// section supplied — must fail LOUD at `resolve_layout` with the
/// `RelaxAbsSym` diagnostic naming a missing symbol and using the Item-C
/// cross-seam-standalone framing (the same improved wording hblank's Task 5
/// follow-up shipped, now pinned for a SECOND real port file — proving the
/// fix generalizes, not a one-off).
///
/// FALSIFIED (restore-real-value): re-ran WITH both cross-seam sections
/// appended (the `controllers_port.rs` shape) — `resolve_layout` returns
/// `Ok`, so `.expect_err(...)` panics on the `Ok`; confirmed by temporarily
/// appending both `as_hw_port_equs()`/`as_ctrl_ram_labels()` and observing
/// the `.expect_err` trip, then reverting to the standalone compile below.
#[test]
fn controllers_standalone_compile_without_cross_seam_sections_is_a_loud_missing_symbol_error() {
    let Some(src) = real_src("controllers.emp") else { return };
    let sections = place_controllers(&src, &format!("{:#x}", controllers_base()));
    // NO cross-seam sections appended — every one of HW_PORT_1_DATA /
    // HW_PORT_2_DATA / Ctrl_1_Held / Ctrl_2_Held / Ctrl_1_Press_Accum /
    // Ctrl_2_Press_Accum is genuinely absent.
    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect_err(
        "compiling controllers.emp standalone (no cross-seam sections) must be a loud \
         resolve_layout error, not a silent/panicking one",
    );
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("unresolved symbolic absolute operand")
                && d.message.contains("controllers")
                && d.message.contains("not defined in this link")
        }),
        "expected the RelaxAbsSym diagnostic with the Item-C cross-seam-standalone framing, \
         got: {err:?}"
    );
    // The FIRST missing symbol the fixpoint reports must be one of the six
    // genuinely-undefined names (not some unrelated garbage) — pins that the
    // diagnostic names A REAL symbol from this module's cross-seam surface.
    // input-6button: the cross-seam surface grew (Z80 fence register + the ext/
    // type RAM cells) — any of these names proves the diagnostic is real.
    let names = ["HW_PORT_1_DATA", "HW_PORT_2_DATA", "Z80_BUS_REQUEST", "Ctrl_1_Held", "Ctrl_2_Held", "Ctrl_1_Press_Accum", "Ctrl_2_Press_Accum", "Ctrl_1_Ext_Held", "Ctrl_2_Ext_Held", "Pad_1_Type", "Pad_2_Type"];
    assert!(
        err.iter().any(|d| names.iter().any(|n| d.message.contains(n))),
        "expected the diagnostic to name one of controllers.emp's six cross-seam symbols, \
         got: {err:?}"
    );
}

// Probe (b) does NOT apply to `math.emp`: it carries no cross-seam INBOUND
// reference (its only external dependency — the `../data/sine.bin` embed —
// resolves at LOWER time via the sandbox, not as a link-time symbol
// reference), so there is no "standalone compile fails to resolve a
// cross-seam symbol" shape to probe here — `math_port.rs`'s doc comment
// makes the same "No cross-seam INBOUND" observation. Noted explicitly
// (per this file's header) rather than silently omitted.

// ===========================================================================
// Probe (d) — CONSTANTS-TWIN DRIFT GUARD — RETIRED
// ===========================================================================
//
// The former `constants_twin_drift_guard_fires_loudly_when_as_side_button_up_disagrees`
// probe drove `engine.constants`'s `ensure(extern("BUTTON_UP") == BUTTON_UP)`
// drift guard. The Stage-3 P5 ownership flip made `BUTTON_UP` (and the other
// 113 engine constants) SOLE-authored by `constants.emp` — the build harvests
// them as guarded AS defines, so there is no AS-side twin to drift and the
// BUTTON_UP mirror guard was deleted. With no guard to fire, this probe tested
// a retired mechanism and is removed (only `VDP_Shadow_len`'s struct-generated
// twin guard survives; it has no negative probe here).

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS
// ===========================================================================

/// Place the real `controllers.emp` at a WRONG base (the pinned base + 6)
/// instead of `pins::CONTROLLERS.plain_base`, and prove the placed section's
/// bytes, while internally self-consistent, land at a DIFFERENT VMA than the
/// reference expects — placement genuinely tracks the map, not an echo.
///
/// FALSIFIED (restore-real-value): re-ran with the base restored to the real
/// one — the placed section's `lma` equals it, so `assert_ne!` against the
/// wrong-base result panics on equal values; confirmed by temporarily placing
/// at the real base twice and observing the (trivially) equal `lma`s, then
/// reverting to the wrong-base comparison below.
#[test]
fn controllers_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src("controllers.emp") else { return };

    let real_sections = place_controllers(&src, &format!("{:#x}", controllers_base()));
    let wrong_sections = place_controllers(&src, &format!("{:#x}", controllers_wrong_base()));

    let real_controllers =
        real_sections.iter().find(|s| s.name == "controllers").expect("real controllers section");
    let wrong_controllers =
        wrong_sections.iter().find(|s| s.name == "controllers").expect("wrong controllers section");

    assert_eq!(
        real_controllers.lma,
        controllers_base(),
        "the real map must place controllers at the pinned base"
    );
    assert_eq!(
        wrong_controllers.lma,
        controllers_wrong_base(),
        "the doctored map must place controllers at the wrong base"
    );
    assert_ne!(
        real_controllers.lma, wrong_controllers.lma,
        "placement must genuinely move with the map base — not be an echo/hardcode"
    );

    let real_linked = link_controllers_placed(real_sections);
    let wrong_linked = link_controllers_placed(wrong_sections);
    let real_bytes = &real_linked.section("controllers").expect("controllers").bytes;
    let wrong_bytes = &wrong_linked.section("controllers").expect("controllers").bytes;
    assert_eq!(real_bytes, wrong_bytes, "content is identical regardless of placement (sanity)");
    assert_ne!(
        real_linked.section("controllers").unwrap().lma,
        wrong_linked.section("controllers").unwrap().lma,
        "the LMA must differ between the two placements — placement is real, not an echo"
    );
}

/// Place the real `math.emp` at a WRONG base (the pinned base + 6) instead of
/// `pins::MATH.plain_base` — the math analogue of the controllers probe above.
///
/// FALSIFIED (restore-real-value): same technique as the controllers probe —
/// re-ran at the real base twice and observed trivially-equal `lma`s before
/// reverting to the wrong-base comparison.
#[test]
fn math_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src("math.emp") else { return };

    let real_sections = place_math(&src, &format!("{:#x}", math_base()));
    let wrong_sections = place_math(&src, &format!("{:#x}", math_wrong_base()));

    let real_math = real_sections.iter().find(|s| s.name == "math").expect("real math section");
    let wrong_math = wrong_sections.iter().find(|s| s.name == "math").expect("wrong math section");

    assert_eq!(real_math.lma, math_base(), "the real map must place math at the pinned base");
    assert_eq!(
        wrong_math.lma,
        math_wrong_base(),
        "the doctored map must place math at the wrong base"
    );
    assert_ne!(
        real_math.lma, wrong_math.lma,
        "placement must genuinely move with the map base — not be an echo/hardcode"
    );

    let real_linked = link_math_placed(real_sections);
    let wrong_linked = link_math_placed(wrong_sections);
    let real_bytes = &real_linked.section("math").expect("math").bytes;
    let wrong_bytes = &wrong_linked.section("math").expect("math").bytes;
    assert_eq!(real_bytes, wrong_bytes, "content is identical regardless of placement (sanity)");
    assert_ne!(
        real_linked.section("math").unwrap().lma,
        wrong_linked.section("math").unwrap().lma,
        "the LMA must differ between the two placements — placement is real, not an echo"
    );
}
