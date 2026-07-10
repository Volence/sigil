//! Port #2 Task 3 — negative probes for `controllers_port.rs` + `math_port.rs`.
//! Mirrors `hblank_negative_probes.rs`'s one-file-per-tranche house style
//! (all probes for the batch live in ONE file), reusing its three probe
//! classes for EACH file where they apply:
//!
//! (a) genuineness — a doctored COPY of the emp source produces DIFFERENT
//!     linked bytes than the reference, proving the byte-diff gate is
//!     non-vacuous. `controllers.emp`: `eor.b d0, d1` -> `eor.b d1, d0` (the
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
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    std::env::var("AEON_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/volence/sonic_hacks/aeon"))
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
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
         size = 0x72\n\
         kind = \"rom\"\n"
    )
}

/// `engine/constants.asm:17-18` (`HW_PORT_*_DATA`) PLUS `:89-92` (`BUTTON_*`)
/// verbatim — `engine.constants`'s eight drift-guard `ensure`s (riding along
/// via `constants_ambient_items`) read the `BUTTON_*` four back through
/// `extern(...)`, so they need real equs to check against here too (mirrors
/// `controllers_port.rs`'s `as_hw_port_equs`).
fn as_hw_port_equs() -> Vec<Section> {
    as_hw_port_equs_with_button_up("1<<0")
}

/// Like [`as_hw_port_equs`], but with the `BUTTON_UP` equ's RHS overridable
/// (`button_up_rhs`) — the drift-guard negative probe doctors this to a wrong
/// value to prove `engine.constants`'s `ensure(extern("BUTTON_UP") ==
/// BUTTON_UP, ...)` genuinely fails loud, naming the constant, when the
/// AS-side source of truth disagrees with the `.emp` twin.
fn as_hw_port_equs_with_button_up(button_up_rhs: &str) -> Vec<Section> {
    let asm = format!(
        "cpu 68000\n\
         HW_PORT_1_DATA = $A10003\n\
         HW_PORT_2_DATA = $A10005\n\
         BUTTON_UP = {button_up_rhs}\n\
         BUTTON_DOWN = 1<<1\n\
         BUTTON_LEFT = 1<<2\n\
         BUTTON_RIGHT = 1<<3\n\
         CTYPE_AIR = 0\n\
         VDP_Shadow_len = 19\n\
         Stub:\n\
         \tdc.w 0\n"
    );
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (hw port equs): {d:?}")).sections
}

fn as_ctrl_ram_labels() -> Vec<Section> {
    let asm = "cpu 68000\n\
               phase $FFFF802C\n\
               Ctrl_1_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_2_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_1_Press_Accum:\n\
               \tdc.b 0\n\
               Ctrl_2_Press_Accum:\n\
               \tdc.b 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (ctrl ram labels): {d:?}")).sections
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
        items: constants_ambient_items().into_iter().chain(file.items).collect(),
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
    let mut ram_labels = as_ctrl_ram_labels();
    for sec in &mut ram_labels {
        sec.lma = 0x0200_0000;
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
         size = 0x298\n\
         kind = \"rom\"\n"
    )
}

/// Parse -> lower (`include_root` = `engine/`, `embed_base` = `engine/system/`
/// — `math_port.rs`'s doc explains why math.emp needs the two-root split) ->
/// place `src` at `base` into the math map.
fn place_math(src: &str, base: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon_dir().join("engine")),
        embed_base: Some(engine_system_dir()),
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

/// Doctor ONE instruction (`eor.b d0, d1` -> `eor.b d1, d0`, a register-field
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
    assert!(src.contains("eor.b   d0, d1"), "precondition: the real file spells `eor.b   d0, d1`");
    let doctored = src.replacen("eor.b   d0, d1", "eor.b   d1, d0", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine_linked = link_controllers_placed(place_controllers(&src, "0x228C"));
    let doctored_linked = link_controllers_placed(place_controllers(&doctored, "0x228C"));

    let genuine_bytes = &genuine_linked.section("controllers").expect("controllers section").bytes;
    let doctored_bytes = &doctored_linked.section("controllers").expect("controllers section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "a doctored `eor.b d1, d0` must emit different bytes than the genuine `eor.b d0, d1` — \
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

    let genuine_linked = link_math_placed(place_math(&src, "0x2464"));
    let doctored_linked = link_math_placed(place_math(&doctored, "0x2464"));

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
    let sections = place_controllers(&src, "0x228C");
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
    let names = ["HW_PORT_1_DATA", "HW_PORT_2_DATA", "Ctrl_1_Held", "Ctrl_2_Held", "Ctrl_1_Press_Accum", "Ctrl_2_Press_Accum"];
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
// Probe (d) — CONSTANTS-TWIN DRIFT GUARD
// ===========================================================================

/// `engine.constants`'s `ensure(extern("BUTTON_UP") == BUTTON_UP, ...)` drift
/// guard must genuinely fail — loudly, naming `BUTTON_UP` — when the AS-side
/// source of truth (`engine/constants.asm`) disagrees with the `.emp` twin's
/// value. Doctors the synthetic AS-side `BUTTON_UP` equ to `1<<4` ($10,
/// `BUTTON_B`'s real value — a plausible off-by-one-bit slip, not an
/// arbitrary garbage value) instead of the genuine `1<<0` ($01) and proves
/// `check_link_asserts` reports an Error naming `BUTTON_UP`, on the REAL
/// (undoctored) `controllers.emp` + `engine.constants` pair — this is the
/// twin's drift guard catching a real disagreement, not a probe-doctored
/// `.emp` source.
///
/// FALSIFIED (restore-real-value): re-ran with `as_hw_port_equs()` (the
/// genuine `1<<0`) — `check_link_asserts` returns no Error diagnostics,
/// confirmed by temporarily asserting `assert_diags.is_empty()` on the
/// undoctored pair and observing it hold, then reverting to the doctored
/// comparison below.
#[test]
fn constants_twin_drift_guard_fires_loudly_when_as_side_button_up_disagrees() {
    let Some(src) = real_src("controllers.emp") else { return };
    let (sections, link_asserts) = place_controllers_with_asserts(&src, "0x228C");

    let mut all_sections = sections;
    // Doctor ONLY `BUTTON_UP` — a wrong AS-side equate, exactly the drift
    // scenario the guard exists to catch (constants.asm changed without its
    // .emp twin following).
    let mut hw_equs = as_hw_port_equs_with_button_up("1<<4");
    for sec in &mut hw_equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    all_sections.extend(hw_equs);
    let mut ram_labels = as_ctrl_ram_labels();
    for sec in &mut ram_labels {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    all_sections.extend(ram_labels);

    let resolved = sigil_link::resolve_layout(&all_sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        assert_diags.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("BUTTON_UP")
                && d.message.contains("engine/constants.asm")
                && d.message.contains("engine/constants.emp")
        }),
        "expected the BUTTON_UP drift guard to fail loudly, naming both files, got: {assert_diags:?}"
    );
    // Every OTHER drift guard (HW_PORT_*_DATA, BUTTON_DOWN/LEFT/RIGHT) must
    // still PASS — this probe doctors exactly one constant, so exactly one
    // guard should fire, not all six (which would suggest the guards aren't
    // independently checking their own named constant).
    let error_count = assert_diags.iter().filter(|d| d.level == Level::Error).count();
    assert_eq!(
        error_count, 1,
        "doctoring only BUTTON_UP must fire exactly ONE drift guard, got: {assert_diags:?}"
    );
}

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS
// ===========================================================================

/// Place the real `controllers.emp` at a WRONG base (`$2292` instead of the
/// real plain `$228C`) and prove the placed section's bytes, while
/// internally self-consistent, land at a DIFFERENT VMA than the reference
/// expects — placement genuinely tracks the map, not an echo/hardcode.
///
/// FALSIFIED (restore-real-value): re-ran with the base restored to the real
/// `0x228C` — the placed section's `lma` equals `0x228C`, so `assert_ne!`
/// against the wrong-base result would panic on equal values; confirmed by
/// temporarily placing at the real base twice and observing the (trivially)
/// equal `lma`s, then reverting to the doctored `0x2292` comparison below.
#[test]
fn controllers_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src("controllers.emp") else { return };

    let real_sections = place_controllers(&src, "0x228C");
    let wrong_sections = place_controllers(&src, "0x2292");

    let real_controllers =
        real_sections.iter().find(|s| s.name == "controllers").expect("real controllers section");
    let wrong_controllers =
        wrong_sections.iter().find(|s| s.name == "controllers").expect("wrong controllers section");

    assert_eq!(real_controllers.lma, 0x228C, "the real map must place controllers at $228C");
    assert_eq!(wrong_controllers.lma, 0x2292, "the doctored map must place controllers at $2292");
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

/// Place the real `math.emp` at a WRONG base (`$246A` instead of the real
/// plain `$2464`) — the math analogue of the controllers probe above.
///
/// FALSIFIED (restore-real-value): same technique as the controllers probe —
/// re-ran at the real base twice and observed trivially-equal `lma`s before
/// reverting to the doctored `0x246A` comparison.
#[test]
fn math_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src("math.emp") else { return };

    let real_sections = place_math(&src, "0x2464");
    let wrong_sections = place_math(&src, "0x246A");

    let real_math = real_sections.iter().find(|s| s.name == "math").expect("real math section");
    let wrong_math = wrong_sections.iter().find(|s| s.name == "math").expect("wrong math section");

    assert_eq!(real_math.lma, 0x2464, "the real map must place math at $2464");
    assert_eq!(wrong_math.lma, 0x246A, "the doctored map must place math at $246A");
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
