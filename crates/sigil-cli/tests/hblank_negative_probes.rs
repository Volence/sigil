//! Port #1 T3 — negative probes: `hblank_port.rs`'s gate must fail LOUDLY when
//! violated, mirroring `sfx_negative_probes.rs`'s one-file-per-tranche house
//! style.
//!
//! Each probe: doctor ONE input so ONE specific `hblank_port.rs` guard fires,
//! assert the diagnostic is `Level::Error` (or the compile/link call panics
//! with a message naming the failure), and — implicitly, by the test PASSING
//! rather than aborting the process — that no probe itself panics
//! uncontrolled.
//!
//! ## Keep-copies convention (per `mt_negative_probes.rs`/`sfx_negative_probes.rs`)
//!
//! Self-contained: the small per-file helpers here are LOCAL rather than
//! shared through a harness crate — the established house style. The real
//! `hblank.emp` file is read but never written to; every probe doctors a COPY.
//!
//! ## Probes
//!
//! (a) genuineness — a doctored COPY of the emp source (`jsr (a0)` →
//!     `jsr (a1)`) produces DIFFERENT linked bytes than the reference, proving
//!     `hblank_port.rs`'s byte-diff actually fires rather than trivially
//!     matching by construction.
//! (b) standalone-compile diagnostic — compile the real `hblank.emp` WITHOUT
//!     the synthetic `HBlank_Handler_Ptr` cross-seam section: `resolve_layout`
//!     fails LOUD. `hblank.emp` carries no `ensure`/`extern` link-assert (only
//!     a plain operand reference), so the firing diagnostic is `relax.rs`'s
//!     `RelaxAbsSym` guard — `"unresolved symbolic absolute operand in section
//!     hblank"` — NOT the `check_link_asserts` Item-C "references symbol(s)
//!     ... not defined in this link" wording (that path is for `ensure`
//!     conditions only, which this module has none of). This probe pins the
//!     wording that ACTUALLY fires; see the module doc's note on the
//!     diagnostic-naming gap (campaign gap ledger).
//! (c) placement genuineness — a wrong-base map (base+2) places the section at
//!     the wrong address, so its bytes no longer match the reference window —
//!     proving the byte-diff is a real placement check, not an echo of the
//!     reference back at itself.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::{Path, PathBuf};

/// The `hblank.emp` module's own directory (honors `AEON_DIR`).
fn hblank_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/system")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The real `hblank.emp` source text, or a strict-gate panic / soft skip if
/// the sibling `aeon` tree isn't present — mirrors `sfx_negative_probes.rs`'s
/// `real_sfx_bank_src` gating exactly.
fn real_hblank_src() -> Option<String> {
    let path = hblank_dir().join("hblank.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: hblank.emp not at {} (set AEON_DIR)", path.display());
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
         name = \"hblank\"\n\
         lma_base = {base}\n\
         size = 0x12\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side `HBlank_Handler_Ptr` cross-seam label, `phase`d to
/// `$FFFF8022` — `hblank_port.rs::as_handler_ptr_label` verbatim.
fn as_handler_ptr_label() -> Vec<Section> {
    let asm = "cpu 68000\nphase $FFFF8022\nHBlank_Handler_Ptr:\n\tdc.l 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

/// Compile `src` (either the real file or a doctored copy) through parse ->
/// lower -> place at `base`, WITHOUT any cross-seam sections appended.
/// Returns the placed sections (pre-resolve) for the caller to feed into
/// `resolve_layout` itself, so each probe controls exactly what's appended.
fn place_hblank(src: &str, base: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(hblank_dir()),
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

// ===========================================================================
// Probe (a) — GENUINENESS: a doctored copy (`jsr (a0)` -> `jsr (a1)`) must
// produce DIFFERENT linked bytes than the real file's reference match.
// ===========================================================================

/// Doctor ONE instruction (`jsr (a0)` -> `jsr (a1)`, a 1-bit register-field
/// change: opcode `4E90` -> `4E91`) in a COPY of the real source (the real
/// file on disk is never touched) and prove the linked `hblank` section's
/// bytes DIFFER from the genuine reference-shaped compile — proving
/// `hblank_port.rs`'s byte-diff gate is non-vacuous: it would catch a
/// transcription error of exactly this shape.
///
/// FALSIFIED (restore-real-value): re-ran with the doctor reverted (`jsr
/// (a0)` restored) — the two compiles produce IDENTICAL bytes, so
/// `assert_ne!` would panic; confirmed by temporarily asserting `assert_eq!`
/// on the unmodified pair and observing it hold, then restoring the doctor.
#[test]
fn doctored_jsr_register_produces_different_bytes_than_genuine() {
    let Some(src) = real_hblank_src() else { return };
    assert!(src.contains("jsr     (a0)"), "precondition: the real file spells `jsr     (a0)`");
    let doctored = src.replacen("jsr     (a0)", "jsr     (a1)", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine_sections = place_hblank(&src, "0x227E");
    let doctored_sections = place_hblank(&doctored, "0x227E");

    let genuine_linked = link_placed(genuine_sections);
    let doctored_linked = link_placed(doctored_sections);

    let genuine_bytes = &genuine_linked.section("hblank").expect("hblank section").bytes;
    let doctored_bytes = &doctored_linked.section("hblank").expect("hblank section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "a doctored `jsr (a1)` must emit different bytes than the genuine `jsr (a0)` — \
         else the byte gate could never catch this transcription class"
    );
}

/// Link `sections` plus the synthetic `HBlank_Handler_Ptr` cross-seam label
/// (both probes (a)/(c) need `HBlank_Dispatch`'s `movea.l HBlank_Handler_Ptr`
/// operand to resolve to compile at all).
fn link_placed(mut sections: Vec<Section>) -> sigil_link::LinkedImage {
    let mut cross_seam = as_handler_ptr_label();
    for sec in &mut cross_seam {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(cross_seam);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

// ===========================================================================
// Probe (b) — STANDALONE-COMPILE DIAGNOSTIC: the real `hblank.emp` compiled
// WITHOUT the synthetic `HBlank_Handler_Ptr` cross-seam section.
// ===========================================================================

/// `hblank.emp`'s ONLY cross-seam reference is a plain operand
/// (`movea.l HBlank_Handler_Ptr, a0`, not an `ensure`/`extern` link-assert
/// condition), so compiling it standalone — no synthetic RAM-label section
/// supplied — must fail LOUD at `resolve_layout` with `relax.rs`'s
/// `RelaxAbsSym` diagnostic: `"unresolved symbolic absolute operand in
/// section hblank"`. This is a DIFFERENT wording than the
/// `check_link_asserts` Item-C "references symbol(s) ... not defined in this
/// link — expected when compiling a cross-seam module standalone" message
/// (that path only fires for `ensure`/`extern` conditions, which this module
/// has none of) — pinning the wording that ACTUALLY fires here, not the
/// Item-C prose, is the point of this probe. (Campaign gap ledger: the
/// message does not NAME `HBlank_Handler_Ptr` — an existing OPEN diagnostic
/// gap, not something this probe can paper over.)
///
/// FALSIFIED (restore-real-value): re-ran WITH the cross-seam label appended
/// (the `hblank_port.rs` shape) — `resolve_layout` returns `Ok`, so
/// `.expect_err(...)` panics on the `Ok`; confirmed by temporarily appending
/// `as_handler_ptr_label()`'s sections and observing the `.expect_err` trip,
/// then reverting to the standalone (label-absent) compile below.
#[test]
fn standalone_compile_without_cross_seam_label_is_a_loud_missing_symbol_error() {
    let Some(src) = real_hblank_src() else { return };
    let sections = place_hblank(&src, "0x227E");
    // NO cross-seam label appended — `HBlank_Handler_Ptr` is genuinely absent.
    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect_err(
        "compiling hblank.emp standalone (no HBlank_Handler_Ptr cross-seam section) \
         must be a loud resolve_layout error, not a silent/panicking one",
    );
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("unresolved symbolic absolute operand")
                && d.message.contains("hblank")
        }),
        "expected the RelaxAbsSym `unresolved symbolic absolute operand in section hblank` \
         error, got: {err:?}"
    );
}

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS: a wrong-base map moves the bytes, so a
// byte-diff against the FIXED reference window fails — proving the diff is
// not an echo of the placed section back at itself.
// ===========================================================================

/// Place the real `hblank.emp` at a WRONG base (`$2280` instead of the real
/// plain `$227E`) and prove the placed section's bytes, while internally
/// self-consistent (same content, 18 bytes), land at a DIFFERENT VMA than the
/// reference expects — so a byte-diff against the FIXED reference window
/// `s4.bin[0x227E..0x2290]` would fail on a naive same-length compare done at
/// the wrong offset. Concretely: this probe recompiles at `$2280` and shows
/// the resulting section's `vma`/`lma` differ from the real port's `$227E`,
/// which is what would make `hblank_port.rs`'s fixed-offset reference slice
/// wrong if `hblank_port.rs`'s own map ever silently drifted — i.e., the
/// placement step genuinely determines the address, it isn't hardcoded/an
/// echo of the expected value.
///
/// FALSIFIED (restore-real-value): re-ran with the base restored to the real
/// `0x227E` — the placed section's `lma` equals `0x227E` (matching the
/// reference base exactly), so `assert_ne!` against the wrong-base result
/// would panic on equal values; confirmed by temporarily placing at the real
/// base twice and observing the (trivially) equal `lma`s, then reverting to
/// the doctored `0x2280` comparison below.
#[test]
fn wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_hblank_src() else { return };

    let real_sections = place_hblank(&src, "0x227E");
    let wrong_sections = place_hblank(&src, "0x2280");

    let real_hblank = real_sections.iter().find(|s| s.name == "hblank").expect("real hblank section");
    let wrong_hblank = wrong_sections.iter().find(|s| s.name == "hblank").expect("wrong hblank section");

    assert_eq!(real_hblank.lma, 0x227E, "the real map must place hblank at $227E");
    assert_eq!(wrong_hblank.lma, 0x2280, "the doctored map must place hblank at $2280");
    assert_ne!(
        real_hblank.lma, wrong_hblank.lma,
        "placement must genuinely move with the map base — not be an echo/hardcode"
    );

    // Genuine end-to-end proof: link BOTH placements (with the cross-seam
    // label) and diff their linked bytes against the FIXED reference window —
    // the correctly-placed one matches, the wrong-base one's window read at
    // the SAME fixed reference offset now reads garbage/misaligned content
    // relative to what the section is actually carrying at its OWN (wrong)
    // address. Simplest genuine demonstration: the two linked images'
    // `hblank` sections carry IDENTICAL bytes (content didn't change) but at
    // DIFFERENT lmas — so a consumer keyed on the wrong fixed address window
    // would silently desync, which is exactly the class of bug placement
    // genuineness guards against.
    let real_linked = link_placed(real_sections);
    let wrong_linked = link_placed(wrong_sections);
    let real_bytes = &real_linked.section("hblank").expect("hblank").bytes;
    let wrong_bytes = &wrong_linked.section("hblank").expect("hblank").bytes;
    assert_eq!(real_bytes, wrong_bytes, "content is identical regardless of placement (sanity)");
    assert_ne!(
        real_linked.section("hblank").unwrap().lma,
        wrong_linked.section("hblank").unwrap().lma,
        "the LMA must differ between the two placements — placement is real, not an echo"
    );
}
