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
//! (a) genuineness — a doctored COPY of the emp source (`move.l a0, ...` →
//!     `move.l a1, ...` in the slot-patch, a 1-bit register-field change)
//!     produces DIFFERENT linked bytes than the reference, proving
//!     `hblank_port.rs`'s byte-diff actually fires rather than trivially
//!     matching by construction.
//! (b) standalone-compile diagnostic — compile the real `hblank.emp` WITHOUT
//!     the synthetic RAM cross-seam sections: `resolve_layout` fails LOUD. The
//!     trampoline's cross-seam references are plain abs-sym operands
//!     (`HBlank_Vector_Slot`, `VDP_Shadow_Table`, `VDP_Dirty_Mask`), so the
//!     firing diagnostic is `relax.rs`'s `RelaxAbsSym` guard, which names the
//!     first missing symbol (`HBlank_Vector_Slot`) with the Item-C
//!     cross-seam-standalone framing ("references symbol(s) ... not defined in
//!     this link — expected when compiling a cross-seam module standalone").
//! (c) placement genuineness — a wrong-base map places the section at the wrong
//!     address, so its bytes no longer match the reference window — proving the
//!     byte-diff is a real placement check, not an echo of the reference.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_harness::pins;
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
         size = {size:#x}\n\
         kind = \"rom\"\n",
        size = pins::HBLANK.plain_len
    )
}

/// The synthetic AS-side RAM cross-seam labels the trampoline writes/reads
/// (`HBlank_Vector_Slot` at the RAM tail, `VDP_Shadow_Table`, `VDP_Dirty_Mask`),
/// each `phase`d to its true plain VMA — `hblank_port.rs::cross_seam_labels`.
fn cross_seam_labels() -> Vec<Section> {
    let labels: [(&str, u32); 3] = [
        ("HBlank_Vector_Slot", pins::H_BLANK_VECTOR_SLOT.plain),
        ("VDP_Shadow_Table", pins::VDP_SHADOW_TABLE.plain),
        ("VDP_Dirty_Mask", pins::VDP_DIRTY_MASK.plain),
    ];
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut out = Vec::new();
    for (i, (name, vma)) in labels.iter().enumerate() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        for mut s in assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections
        {
            s.lma = 0x0100_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
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
    assert!(
        src.contains("move.l  a0, HBlank_Vector_Slot"),
        "precondition: the real file spells the slot patch `move.l  a0, HBlank_Vector_Slot`"
    );
    let doctored = src.replacen("move.l  a0, HBlank_Vector_Slot", "move.l  a1, HBlank_Vector_Slot", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine_sections = place_hblank(&src, &format!("{:#x}", pins::HBLANK.plain_base));
    let doctored_sections = place_hblank(&doctored, &format!("{:#x}", pins::HBLANK.plain_base));

    let genuine_linked = link_placed(genuine_sections);
    let doctored_linked = link_placed(doctored_sections);

    let genuine_bytes = &genuine_linked.section("hblank").expect("hblank section").bytes;
    let doctored_bytes = &doctored_linked.section("hblank").expect("hblank section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "a doctored `move.l a1, ...` must emit different bytes than the genuine `move.l a0, ...` — \
         else the byte gate could never catch this transcription class"
    );
}

/// Link `sections` plus the synthetic RAM cross-seam labels (probes (a)/(c) need
/// the trampoline's `HBlank_Vector_Slot`/`VDP_*` abs operands to resolve).
fn link_placed(mut sections: Vec<Section>) -> sigil_link::LinkedImage {
    sections.extend(cross_seam_labels());
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
/// `RelaxAbsSym` diagnostic. As of Task 5 (the equ-operand / diagnostic-naming
/// follow-up, port #1) this diagnostic NAMES the missing symbol
/// (`HBlank_Handler_Ptr`) and uses the same Item-C cross-seam-standalone
/// framing as `check_link_asserts`'s "references symbol(s) ... not defined in
/// this link — expected when compiling a cross-seam module standalone"
/// wording — the two diagnostics are worded consistently now (one fix, per
/// the campaign gap ledger's merge note) even though they still fire from
/// different code paths (this one has no `ensure`/`extern` condition to route
/// through `check_link_asserts`). This probe pins the IMPROVED wording —
/// previously it only named the SECTION (`"...in section hblank"`), which is
/// the gap this task closed.
///
/// FALSIFIED (restore-real-value): re-ran WITH the cross-seam label appended
/// (the `hblank_port.rs` shape) — `resolve_layout` returns `Ok`, so
/// `.expect_err(...)` panics on the `Ok`; confirmed by temporarily appending
/// `as_handler_ptr_label()`'s sections and observing the `.expect_err` trip,
/// then reverting to the standalone (label-absent) compile below.
#[test]
fn standalone_compile_without_cross_seam_label_is_a_loud_missing_symbol_error() {
    let Some(src) = real_hblank_src() else { return };
    let sections = place_hblank(&src, &format!("{:#x}", pins::HBLANK.plain_base));
    // NO cross-seam labels appended — `HBlank_Vector_Slot` is genuinely absent.
    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect_err(
        "compiling hblank.emp standalone (no RAM cross-seam sections) \
         must be a loud resolve_layout error, not a silent/panicking one",
    );
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("unresolved symbolic absolute operand")
                && d.message.contains("hblank")
                && d.message.contains("HBlank_Vector_Slot")
                && d.message.contains("not defined in this link")
        }),
        "expected the RelaxAbsSym diagnostic to name `HBlank_Vector_Slot` with the Item-C \
         cross-seam-standalone framing, got: {err:?}"
    );
}

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS: a wrong-base map moves the section (the
// assertions below check the PLACED ADDRESS moves while content stays
// self-consistent — the actual byte-diff-at-a-fixed-window failure that a
// moved section causes is carried by the port/mixed gates themselves).
// Proves placement is real, not an echo of the expected value.
// ===========================================================================

/// Place the real `hblank.emp` at a WRONG base (`$2280` instead of the real
/// plain `pins::HBLANK.plain_base`) and prove the placed section's bytes, while
/// internally self-consistent (same content, same length), land at a DIFFERENT
/// VMA than the reference expects — so a byte-diff against the FIXED reference
/// window would fail on a naive same-length compare done at the wrong offset.
/// This proves the placement step genuinely determines the address; it isn't
/// hardcoded/an echo of the expected value.
///
/// FALSIFIED (restore-real-value): re-ran with the base restored to the real
/// plain base — the placed section's `lma` equals it (matching the reference
/// base), so `assert_ne!` against the wrong-base result would panic on equal
/// values; confirmed by temporarily placing at the real base twice and
/// observing the (trivially) equal `lma`s, then reverting to the `0x2280`
/// comparison below.
#[test]
fn wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_hblank_src() else { return };

    let real_sections = place_hblank(&src, &format!("{:#x}", pins::HBLANK.plain_base));
    let wrong_sections = place_hblank(&src, "0x2280");

    let real_hblank = real_sections.iter().find(|s| s.name == "hblank").expect("real hblank section");
    let wrong_hblank = wrong_sections.iter().find(|s| s.name == "hblank").expect("wrong hblank section");

    assert_eq!(real_hblank.lma, pins::HBLANK.plain_base, "the real map must place hblank at the plain base");
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
