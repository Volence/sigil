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
/// `BGANIM_VIEW_EMIT = if DEBUG == 1 { 1 } else { 0 }` and gates its view arrays on
/// it, so the DEBUG shape carries camera-motion view records the plain shape does
/// not, one 46-byte arm each (`BgAnim_View_H`, `BgAnim_View_V`, `BgAnim_View_T`):
///
/// ```text
///   BgAnim_View_<arm>        [u16; 1]   2
///   _BgAnim_View<arm>0_hdr   [u16; 6]  12
///   _BgAnim_View<arm>0_banks [*u8; 8]  32
///                                      --
///                                      46 per arm
/// ```
///
/// Derived from the module rather than measured off the pins on purpose: read off the
/// pins it would equal the divergence by construction and assert nothing. Read from
/// the MODULE TEXT rather than typed, because the record count is aeon's to change
/// (a third arm, `BgAnim_View_T`, joined the two above and a typed 92 stopped
/// describing it): every `data` line whose length is `BGANIM_VIEW_EMIT` or
/// `BGANIM_VIEW_EMIT * k` contributes k times its element width, and only those
/// lines exist in the debug shape. A line this reader cannot size fails by name.
fn bg_anim_view_bytes(aeon: &std::path::Path) -> usize {
    let path = aeon.join("games/sonic4/data/generated/ojz/act1/bg_anim.emp");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("bg_anim_view_bytes: cannot read {}: {e}", path.display()));
    let width = |elem: &str| -> usize {
        match elem.trim() {
            "u8" | "i8" => 1,
            "u16" | "i16" => 2,
            "u32" | "i32" => 4,
            t if t.starts_with('*') => 4,
            other => panic!(
                "bg_anim_view_bytes: element type `{other}` in {} has no width this reader knows",
                path.display()
            ),
        }
    };
    let mut total = 0usize;
    let mut lines = 0usize;
    for raw in src.lines() {
        let line = raw.split("//").next().unwrap_or("").trim();
        let Some(rest) = line.strip_prefix("data ").or_else(|| line.strip_prefix("pub data ")) else {
            continue;
        };
        // `NAME: [ELEM; BGANIM_VIEW_EMIT * k] = ...` or `[ELEM; BGANIM_VIEW_EMIT]`.
        let Some((_, ty)) = rest.split_once(':') else { continue };
        let Some(inner) = ty.trim().strip_prefix('[').and_then(|t| t.split_once(']')).map(|(i, _)| i) else {
            continue;
        };
        let Some((elem, len)) = inner.split_once(';') else { continue };
        let len = len.trim();
        let Some(len) = len.strip_prefix("BGANIM_VIEW_EMIT") else { continue };
        let count: usize = match len.trim() {
            "" => 1,
            k => k
                .strip_prefix('*')
                .map(str::trim)
                .and_then(|k| k.parse().ok())
                .unwrap_or_else(|| panic!("bg_anim_view_bytes: cannot read the multiplier in `{raw}`")),
        };
        total += width(elem) * count;
        lines += 1;
    }
    assert!(
        lines > 0,
        "bg_anim_view_bytes: {} declares no BGANIM_VIEW_EMIT-gated data line, the view record \
         this gate accounts for is gone",
        path.display()
    );
    total
}

/// `(emp, section, pin, preludes, max cross-shape length divergence)`.
///
/// THE FIFTH FIELD IS A CONTRACT, not a tolerance dial. Every section here emits
/// shape-invariant content, so its plain and debug spans may differ only by the short
/// alignment pad a changed successor leaves — `ALIGN_PAD`. A section that legitimately
/// emits DIFFERENT BYTES per shape declares exactly how many, and the assert is then
/// tight against that number rather than loosened for everyone.
fn sections(aeon: &std::path::Path) -> Vec<(&'static str, &'static str, Region, &'static [Prelude], usize)> {
    vec![
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
        bg_anim_view_bytes(aeon),
    ),
    ]
}

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

    for (emp_rel, section, region, preludes, max_shape_delta) in &sections(&aeon_root()) {
        let base = if debug { region.debug_base } else { region.plain_base };
        let len = if debug { region.debug_len } else { region.plain_len };
        let (lo, hi) = (region.plain_len.min(region.debug_len), region.plain_len.max(region.debug_len));
        assert!(
            hi - lo <= *max_shape_delta,
            "{section} plain/debug spans differ by {:#x}, more than the {max_shape_delta:#x} this \
             section declares: plain {:#x} debug {:#x}. Either the emission changed or the \
             declaration is stale, re-derive it from the module, do not raise it to fit.",
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
            "`{section}` emitted NO BYTES, a pin over an empty window proves nothing. \
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
