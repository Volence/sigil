//! Parcel K3 run A — the OJZ act1 interior-island HEAD, region-level byte gate.
//!
//! The contiguous AS run BEFORE the native descriptor became two native `.emp`
//! sections:
//!
//! - **entity_data** (`games.sonic4.ojz_entity_data_act1`): the 9-section type
//!   tables (count-prefixed `dc.l ObjDef_*` pointer arrays — the objentry/objend
//!   macros replaced by packed 3-word records + a `$FFFF` terminator; ring lists
//!   are `dc.w X,Y` pairs + a longword-0 terminator). `OJZ_Sec0_TypeTable` ..
//!   `OJZ_Act_Pool_Page0`. The `ObjDef_*` archetypes are cross-module link labels
//!   (test_objects / path_swap), injected here as a synthetic seam at their ROM
//!   addresses (Abs32 fixups bake addresses, so the pointer cells are load-bearing).
//! - **ojz_act_pool** (`games.sonic4.ojz_act_pool_act1`): the 10 ZX0/raw page
//!   `embed()`s + the manifest-v2 `OJZ_Act_Pool_PageTable` — a
//!   `[PageManifest; 10]` array of `{pm_source, pm_tiles, pm_form, pm_flags}`
//!   records (stride 8; same-module `extern()`s resolved at link). The module
//!   `use`s the `PageManifest` type from `engine.structs`; the standalone compile
//!   can't resolve that cross-module `use`, so the isolated source injects the
//!   `PageManifest` struct definition (the type analog of the `ObjDef` symbol
//!   seam below). `OJZ_Act_Pool_Page0` .. `OJZ_Act1_Descriptor`.
//!
//! Each section is byte-compared against the reference ROM at its pin base, plus a tail
//! check that the gap to the next section is map fill AND bounded. Content is
//! shape-invariant: the debug shape is the same bytes at its own base, with the Abs32
//! pointer cells relocated by the base delta. The REGION LENGTHS are not shape-invariant
//! and must not be asserted equal — they are `end-is-next-placement` spans that include
//! placer slack. See the block in `gate()`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test ojz_run_a_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins::{self, Region};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_root() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// One `ObjDef` record — the stride between `ObjDef_Static` and `ObjDef_Solid`.
const OBJDEF_LEN: u32 = 0x1A;

/// The fill byte `games/sonic4/map.toml` places these sections with (`fill = 0x00`).
/// Inter-section slack is written with it; emitted data is not.
const MAP_FILL: u8 = 0x00;

/// Largest inter-section gap the PLACER can legitimately leave after these sections.
///
/// This is the bound that keeps the tail check honest. A fill-value scan alone cannot
/// distinguish placer slack from emitted data **that happens to be zero** — a
/// DEBUG-fenced `align`, a `ds.b` reservation, or a trailing `dc.l 0` sentinel at the
/// tail of a level-data module would read as slack and pass. Bounding the gap closes
/// that: the knuckles-c4 failure (48 bytes of debug-only object records) is caught by
/// SIZE here even in the all-zero case, and today's real slack is 2 bytes
/// (`ojz_act_pool`, debug) against 0 everywhere else, so the margin is wide.
///
/// Raise this ONLY with a placement change that justifies it, never to make a failure
/// go away — a gap this big is the signal, whatever the bytes say.
const MAX_PLACER_SLACK: usize = 16;

/// The `ObjDef_*` seam entity_data's type-table pointers resolve against — the
/// per-shape ROM addresses the Abs32 pointer cells bake in.
///
/// PIN-DERIVED as of `cheat-flag` (2026-08-05). These were three hand-typed literal
/// pairs that had to be re-shifted by hand on every re-baseline, and they silently
/// rotted whenever someone forgot — this parcel moved the object bank +0x20 and the
/// gate failed with a one-byte diff deep inside a data blob, which is an expensive
/// way to learn that a constant is stale. The old comment already recorded the exact
/// relations (`ObjDef_Static == OBJDEFS base`, `ObjDef_Solid == base + one ObjDef`,
/// `ObjDef_PathSwap == PATH_SWAP base`), so they are now simply computed.
///
/// This is NOT circular: the seam is an INPUT to the standalone compile, and the
/// assertion compares that compile's bytes against the real built ROM. Deriving the
/// input from pins removes hand-maintenance without weakening the check.
fn objdef_seam(debug: bool) -> Vec<(&'static str, u32)> {
    let (objdefs, path_swap) = if debug {
        (pins::OBJDEFS.debug_base, pins::PATH_SWAP.debug_base)
    } else {
        (pins::OBJDEFS.plain_base, pins::PATH_SWAP.plain_base)
    };
    vec![
        ("ObjDef_Solid", objdefs + OBJDEF_LEN),
        ("ObjDef_Static", objdefs),
        ("ObjDef_PathSwap", path_swap),
    ]
}

fn seam_sections(debug: bool) -> Vec<Section> {
    let mut asm = String::from("cpu 68000\n");
    for (name, addr) in objdef_seam(debug) {
        asm.push_str(&format!("{name} = ${addr:X}\n"));
    }
    asm.push_str("Stub:\n\tdc.w 0\n");
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (seam): {d:?}")).sections
}

fn map_toml(section: &str, base: u32, len: usize) -> String {
    format!(
        "fill = 0x00\n\n[[region]]\nname = \"{section}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    )
}

fn compile_section(
    emp_rel: &str,
    section: &str,
    base: u32,
    len: usize,
    seam: bool,
    debug: bool,
) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let path = aeon.join(emp_rel);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    // ojz_act_pool.emp `use`s the PageManifest type (the manifest-v2 table records)
    // from engine.structs; a standalone single-module compile can't resolve that
    // cross-module `use`, so inject the struct definition in place of the `use`
    // (the type analog of the ObjDef symbol seam). Layout MUST match
    // engine/structs.emp::PageManifest exactly, or the emitted table bytes diverge.
    let src = src.replace(
        "use engine.structs.{PageManifest}",
        "struct PageManifest {\n\
         \tpm_source: *u8,\n\
         \tpm_tiles: u16,\n\
         \tpm_form: u8,\n\
         \tpm_flags: u8,\n\
         }",
    );
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} parse errors: {pdiags:?}"
    );

    // embed() paths in these modules are aeon-root-relative.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: None,
        defines: vec![],
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

    if seam {
        let mut equs = seam_sections(debug);
        for s in &mut equs {
            s.lma = 0x0100_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
        }
        sections.extend(equs);
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{emp_rel} resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{emp_rel} link failed: {d:?}"))
}

/// (module `.emp` path relative to aeon, section name, pin, needs the ObjDef seam).
fn sections() -> Vec<(&'static str, &'static str, Region, bool)> {
    vec![
        (
            "games/sonic4/data/generated/ojz/act1/entity_data.emp",
            "entity_data",
            pins::ENTITY_DATA,
            true,
        ),
        (
            "games/sonic4/data/generated/ojz/act1/ojz_act_pool.emp",
            "ojz_act_pool",
            pins::OJZ_ACT_POOL,
            false,
        ),
    ]
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

    for (emp_rel, section, region, seam) in sections() {
        let base = if debug { region.debug_base } else { region.plain_base };
        // The MODULE'S OWN emitted length. Both modules here are generated and carry
        // no shape conditionals, and `compile_section` lowers them with
        // `defines: vec![]` — so this ONE compile is the authority for both shapes,
        // and the plain region (which has no trailing placer slack) states its length.
        // If a future parcel ever leaves slack after the PLAIN section too, the
        // "must emit exactly" assert below fails loudly rather than silently drifting.
        let len = region.plain_len;
        // The region pin's LENGTH IS NOT THE SECTION'S LENGTH. These regions are
        // declared `end-is-next-placement` (sigil-harness/repin.toml): the pin spans
        // start-symbol -> NEXT SECTION's start symbol, so it swallows whatever
        // inter-section fill the placer happened to leave. That slack is shape-varying
        // by nature — 49 of the 95 region pins already carry `debug_len != plain_len`
        // on master, including level data (`act_descriptor` +6, `sec_block_blobs` -12,
        // `objdefs` -4), and `act_descriptor` is this region's immediate neighbour
        // using the identical pattern.
        //
        // This test used to `assert_eq!(debug_len, plain_len)` here. That held for 100+
        // parcels by luck of placement and then fired on effects-p2 with a 2-byte delta
        // on `ojz_act_pool` — investigated 2026-08-13 and ruled a STALE FIXTURE, not a
        // shape leak: the section's 0x2F16 bytes are byte-identical between shapes
        // except the 10 Abs32 page pointers, each shifted by exactly the base delta
        // (0x850), and the 2 extra debug bytes are 0x00 map fill sitting OUTSIDE the
        // section at 0x1600E. Asserting on end-is-next-placement lengths measures the
        // placer, not the data.
        //
        // The invariant that IS load-bearing — and that caught the real knuckles-c4
        // mistake (a DEBUG-gated test platform appended to sec0's object list made
        // `entity_data` 48 bytes longer in the debug shape) — is that the module emits
        // the SAME BYTES in both shapes. Three checks enforce it:
        //   1. the byte comparison below. Both shapes are compiled with `defines: vec![]`,
        //      so NO shape conditional is selectable in either — the emitted body cannot
        //      differ by shape, and each shape's body is compared against its own ROM.
        //      (The two compiles are not bit-identical inputs: `objdef_seam(debug)` feeds
        //      per-shape ObjDef addresses to `entity_data`. What is shape-invariant is the
        //      SELECTION of source, which is the property this argument needs.) In the
        //      knuckles-c4 case the extra records sat mid-region, so every later byte
        //      shifted and this diffed on the first one.
        //   2. the tail check: everything between the module's last byte and the next
        //      section's start must be MAP FILL — catches non-zero data APPENDED to the
        //      tail, which check 1 cannot see.
        //   3. the slack BOUND (`MAX_PLACER_SLACK`) — catches appended data that check 2
        //      cannot see because it happens to be zero. Without it, a DEBUG-fenced
        //      `align`/`ds.b`/zero sentinel at the tail would read as fill and pass.
        // 2 and 3 together cover the tail in both value and size; 1 covers the interior.
        let shape_len = if debug { region.debug_len } else { region.plain_len };
        assert!(
            shape_len >= len,
            "`{section}` ({}) region is {shape_len:#x} but the module emits {len:#x} bytes — the \
             next section starts INSIDE this one. Either this shape's level data really is \
             SHORTER (a shape leak in the emitter), or the OTHER shape grew and this is the \
             baseline. Both are real; neither is placer slack. Do not re-baseline.",
            if debug { "debug" } else { "plain" }
        );

        let linked = compile_section(emp_rel, section, base, len, seam, debug);
        let sec = linked
            .section(section)
            .unwrap_or_else(|| panic!("linked image must carry `{section}`"));
        assert_eq!(sec.bytes.len(), len, "`{section}` must emit exactly {len:#x} bytes");
        let expected = &refrom[base as usize..base as usize + len];
        if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
            panic!(
                "`{section}` ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
                if debug { "debug" } else { "plain" },
                &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
                &expected[i.saturating_sub(4)..(i + 8).min(len)]
            );
        }

        // Tail slack must be BOUNDED (a gap bigger than the placer can leave is emitted
        // data whatever its bytes are) and pure map fill. See the block above.
        assert!(
            shape_len - len <= MAX_PLACER_SLACK,
            "`{section}` ({}) emits {len:#x} bytes but the next section starts {shape_len:#x} in \
             — a {:#x}-byte gap, past the {MAX_PLACER_SLACK:#x} the placer can leave. That is \
             emitted level DATA, not slack, even if it reads as fill (the knuckles-c4 \
             DEBUG-entity failure mode was 0x30 bytes). Level data must be shape-identical: \
             fix the emitter, do not raise the bound.",
            if debug { "debug" } else { "plain" },
            shape_len - len
        );
        let slack = &refrom[base as usize + len..base as usize + shape_len];
        if let Some(i) = slack.iter().position(|&b| b != MAP_FILL) {
            panic!(
                "`{section}` ({}) emits {len:#x} bytes but the next section starts {shape_len:#x} \
                 in, and the gap is NOT map fill — first non-fill at gap offset {i:#x} \
                 (ROM {:#x}): {:02x?}. Non-fill slack means this shape emitted extra level DATA \
                 (the knuckles-c4 DEBUG-entity failure mode). Level data must be shape-identical: \
                 fix the emitter, do not re-baseline.",
                if debug { "debug" } else { "plain" },
                base as usize + len + i,
                &slack[i..(i + 8).min(slack.len())]
            );
        }
    }
}

#[test]
fn ojz_run_a_regions_match_reference() {
    gate(false, "s4.bin");
}

#[test]
fn ojz_run_a_debug_regions_match_reference() {
    gate(true, "s4.debug.bin");
}
