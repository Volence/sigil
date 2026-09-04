//! Parcel K3 run B — the OJZ act1 interior-island TAIL, region-level byte gate.
//!
//! The contiguous AS run after the native descriptor became three native `.emp`
//! sections (embed-dominated; every island even-length so the old `align 2`s are
//! no-ops):
//!
//! - **sec_block_blobs** (`games.sonic4.ojz_sec_block_blobs_act1`): 8 `embed()`d
//!   per-section block blobs + the `OJZ_Sec4_Blocks = extern("OJZ_Sec2_Blocks")`
//!   content-dedup alias (zero bytes). `OJZ_Sec0_Blocks` .. `OJZ_Palette`.
//! - **ojz_act_assets** (`games.sonic4.ojz_act_assets_act1`): the four palette/BG
//!   `embed()`s that dissolved out of act_descriptor.asm — OJZ_Palette /
//!   BGND_Palette / OJZ_Act1_BG_Layout / OJZ_Act1_BG_Tiles. `OJZ_Palette` .. `BgAnim_Table`.
//! - **ojz_bg_anim** (`games.sonic4.ojz_bg_anim_act1`): the disabled BgAnim_Table
//!   stub (`u16 = 0`) + the empty BgAnim_Banks base. `BgAnim_Table` .. `Map_TestObj`.
//!
//! Each section is byte-compared against the reference ROM at its pin base
//! (sourced from `sigil_harness::pins` — regenerate via repin). Content is
//! shape-invariant; the debug shape is the same bytes at the +0x88 offset.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test ojz_run_b_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins::{self, Region};
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::PathBuf;

fn aeon_root() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// A synthesized dep source this oracle prepends to a section's own items,
/// because the name it carries arrives by `use` from a module a single-file
/// lower has no way to follow. Derived from the aeon tree at test runtime —
/// see `sigil_harness::test_support` §4.
type Prelude = fn(&std::path::Path) -> String;

/// (module `.emp` path relative to the aeon tree, its section name, its pin,
/// any synthesized preludes it needs).
/// The alignment pad a section may differ by when its emitted bytes are identical in
/// both shapes: a changed successor can leave a few bytes of tail padding inside the
/// measured span. Anything past this is a content difference, not padding.
const ALIGN_PAD: usize = 15;

/// `ojz_bg_anim` is the one section here whose generated module emits DIFFERENT BYTES
/// per shape, by design and by exactly this much.
///
/// `games/sonic4/data/generated/ojz/act1/bg_anim.emp` sets
/// `BGANIM_VIEW_EMIT = if DEBUG == 1 { 1 } else { 0 }` and gates six arrays on it, so
/// the DEBUG shape carries a camera-motion view record the plain shape does not:
///
/// ```text
///   BgAnim_View_H          [u16; 1]   2       BgAnim_View_V          [u16; 1]   2
///   _BgAnim_ViewH0_hdr     [u16; 6]  12       _BgAnim_ViewV0_hdr     [u16; 6]  12
///   _BgAnim_ViewH0_banks   [*u8; 8]  32       _BgAnim_ViewV0_banks   [*u8; 8]  32
///                                    --                                        --
///                                    46   x2                                   46   = 92
/// ```
///
/// Derived from the module rather than measured off the pins on purpose: read off the
/// pins it would equal the divergence by construction and assert nothing. If the view
/// record gains or loses a field, this number is wrong and the gate says so.
const BG_ANIM_VIEW_BYTES: usize = 92;

/// `(emp, section, pin, preludes, max cross-shape length divergence)`.
///
/// THE FIFTH FIELD IS A CONTRACT, not a tolerance dial. Every section here emits
/// shape-invariant content, so its plain and debug spans may differ only by the short
/// alignment pad a changed successor leaves — `ALIGN_PAD`. A section that legitimately
/// emits DIFFERENT BYTES per shape declares exactly how many, and the assert is then
/// tight against that number rather than loosened for everyone.
const SECTIONS: &[(&str, &str, Region, &[Prelude], usize)] = &[
    (
        "games/sonic4/data/generated/ojz/act1/sec_block_blobs.emp",
        "sec_block_blobs",
        pins::SEC_BLOCK_BLOBS,
        &[],
        ALIGN_PAD,
    ),
    (
        "games/sonic4/data/generated/ojz/act1/sec_local_maps.emp",
        "sec_local_maps",
        pins::SEC_LOCAL_MAPS,
        &[],
        ALIGN_PAD,
    ),
    (
        // `use engine.bg.{BG_LAYOUT_SIZE}` — the module's BG-layout embed is
        // TYPED `[u8; BG_LAYOUT_SIZE]` (the length is the guard against a
        // wrong-geometry blob), so the standalone lower needs that one const.
        "games/sonic4/data/levels/ojz/act1/act_assets.emp",
        "ojz_act_assets",
        pins::OJZ_ACT_ASSETS,
        &[sigil_harness::test_support::bg_layout_size_const_src as Prelude],
        ALIGN_PAD,
    ),
    (
        "games/sonic4/data/generated/ojz/act1/bg_anim.emp",
        "ojz_bg_anim",
        pins::OJZ_BG_ANIM,
        &[],
        BG_ANIM_VIEW_BYTES,
    ),
];

fn map_toml(section: &str, base: u32, len: usize) -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"{section}\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile_section(
    emp_rel: &str,
    section: &str,
    base: u32,
    len: usize,
    preludes: &[Prelude],
    debug: bool,
) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let path = aeon.join(emp_rel);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (main, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} parse errors: {pdiags:?}"
    );

    // Prepend the synthesized dep items (consts only — they emit no bytes, so
    // the byte comparison below still sees exactly this module's image). The
    // raster_port/parallax_port idiom.
    let mut items = Vec::new();
    for prelude in preludes {
        let psrc = prelude(&aeon);
        let (pfile, pdiags) = parse_str(&psrc);
        assert!(
            pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "{emp_rel} prelude parse errors: {pdiags:?}\n--- prelude ---\n{psrc}"
        );
        items.extend(pfile.items);
    }
    items.extend(main.items.clone());
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };

    // embed() paths in these modules are aeon-root-relative.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: None,
        // THE SHAPE THE COMPARISON IS ABOUT. A generated module may gate its emission
        // on `DEBUG` — `bg_anim.emp` does — so lowering it without the define both
        // fails outright on the name and, if it resolved, would build the wrong shape's
        // bytes and diff them against the other shape's ROM window.
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(section, base, len)).expect("map must load");
    let mut sections = module.sections;
    let place = place_sections(&mut sections, &map);
    assert!(
        place.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} place_sections errors: {place:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{emp_rel} resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{emp_rel} link failed: {d:?}"))
}

fn gate(debug: bool, rom_name: &str) {
    let rom_path = aeon_root().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    for (emp_rel, section, region, preludes, max_shape_delta) in SECTIONS {
        let base = if debug { region.debug_base } else { region.plain_base };
        let len = if debug { region.debug_len } else { region.plain_len };
        let (lo, hi) = (region.plain_len.min(region.debug_len), region.plain_len.max(region.debug_len));
        assert!(
            hi - lo <= *max_shape_delta,
            "{section} plain/debug spans differ by {:#x}, more than the {max_shape_delta:#x} this \
             section declares: plain {:#x} debug {:#x}. Either the emission changed or the \
             declaration is stale — re-derive it from the module, do not raise it to fit.",
            hi - lo,
            region.plain_len,
            region.debug_len
        );

        let linked = compile_section(emp_rel, section, base, len, preludes, debug);
        let sec = linked
            .section(section)
            .unwrap_or_else(|| panic!("linked image must carry `{section}`"));
        // A gate over an EMPTY image proves nothing, and the pad tolerance below
        // would hide that: with no emitted bytes it shrinks `len` to zero, the
        // length assert compares 0 == 0, and the diff loop runs over an empty
        // range. This is where the vacuity was confirmed — `ojz_bg_anim`'s plain
        // window is 14 all-zero bytes, so the gate passed whether or not
        // `bg_anim.emp` emitted anything (lens sweep, seat GATE, S15).
        assert!(
            !sec.bytes.is_empty(),
            "`{section}` emitted NO BYTES — a pin over an empty window proves nothing. \
             Either the module stopped emitting, or this pin should not exist."
        );
        // Packed placement (Wave-B B-0): the pin LEN spans to the NEXT section's
        // aligned base, so the window may end in a short all-zero align pad
        // beyond the lowered image (bug005: sec_block_blobs 0xB08A emitted vs a
        // 0xB08E pin — 4 pad bytes before ojz_act_assets' aligned base). Same
        // tolerance as the region gates' assert_region_matches.
        let len = if len > sec.bytes.len()
            && len - sec.bytes.len() < 16
            && refrom[base as usize + sec.bytes.len()..base as usize + len]
                .iter()
                .all(|&b| b == 0)
        {
            sec.bytes.len()
        } else {
            len
        };
        assert_eq!(
            sec.bytes.len(),
            len,
            "`{section}` must emit exactly {len:#x} bytes (modulo a short align pad)"
        );
        let expected = &refrom[base as usize..base as usize + len];
        if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
            panic!(
                "`{section}` ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
                if debug { "debug" } else { "plain" },
                &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
                &expected[i.saturating_sub(4)..(i + 8).min(len)]
            );
        }
    }
}

#[test]
fn ojz_run_b_regions_match_reference() {
    gate(false, "s4.bin");
}

#[test]
fn ojz_run_b_debug_regions_match_reference() {
    gate(true, "s4.debug.bin");
}
