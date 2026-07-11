//! Tranche 3 — negative probes for `vdp_init_port.rs` +
//! `collision_lookup_port.rs`. Mirrors the one-file-per-tranche house style
//! (`tranche2_negative_probes.rs`), reusing its probe classes for EACH file
//! where they apply, plus one class this tranche adds:
//!
//! (a) genuineness — a doctored COPY of the `engine.constants` TWIN produces
//!     DIFFERENT linked bytes than the reference, proving the byte-diff gate
//!     is non-vacuous AND the twin's values are load-bearing for the emitted
//!     immediates (`CTYPE_AIR` 0 -> 1 feeds collision_lookup's `moveq`;
//!     `VDP_Shadow_len` 19 -> 18 feeds vdp_init's two loop counters). The
//!     `extern()` drift guards would ALSO catch these (tranche 2's probe
//!     class, exercised by the port gates' `check_link_asserts`); this probe
//!     proves the BYTES change too — two independent tripwires.
//! (b) standalone-compile missing-symbol diagnostic — compile the real file
//!     WITHOUT its synthetic cross-seam sections: the link fails LOUD,
//!     naming a genuinely-missing symbol from the module's own cross-seam
//!     surface.
//! (c) placement genuineness — a wrong-base map moves the section; the
//!     placed LMA genuinely tracks the map, not an echo/hardcode.
//! (d) NEW this tranche: PC-RELATIVE TARGET-POSITION genuineness — both
//!     files carry a cross-seam pc-relative reference (`jbra
//!     Tile_Cache_GetCollision` — the step-5 tail call; `lea.l
//!     BootData_VDPRegs(pc), a0`) whose
//!     bytes encode the DISTANCE to the target. Re-linking with the target
//!     label phased at a wrong VMA (+4) must change the linked bytes —
//!     proving the displacement is genuinely computed from the supplied
//!     symbol position, not echoed from the reference.
//!
//! ## Keep-copies convention (per the prior probe files)
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

/// Read one of the two real ported files (`subdir` distinguishes
/// `engine/system` from `engine/level` — collision_lookup is the first port
/// outside `engine/system/`). Skip-green (or strict-fail) when the aeon tree
/// is absent.
fn real_src(subdir: &str, name: &str) -> Option<String> {
    let path = aeon_dir().join(subdir).join(name);
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

/// The real `engine.constants` twin source (`engine/system/constants.emp`).
fn twin_src() -> Option<String> {
    real_src("engine/system", "constants.emp")
}

fn read_reference(name: &str) -> Option<Vec<u8>> {
    let path = aeon_dir().join(name);
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

/// Parse + lower `src` (with `twin_src` — the `engine.constants` twin, real
/// or probe-doctored — prepended via the ambient technique, since both
/// tranche-3 files `use engine.constants` after step 2's migration) and
/// place it into a two-region map (`text` carrier + the named region at
/// `base`, `size`). Panics on any error diagnostic — probes doctor VALUES,
/// never break the compile.
fn place_module(src: &str, twin_src: &str, region: &str, base: &str, size: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let (twin_file, tdiags) = parse_str(twin_src);
    assert!(tdiags.iter().all(|d| d.level != Level::Error), "twin parse errors: {tdiags:?}");
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: twin_file.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon_dir()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");
    let map_toml = format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"{region}\"\n\
         lma_base = {base}\n\
         size = {size}\n\
         kind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    sections
}

/// Assemble a synthetic AS-side unit and pin its sections at `lma`.
fn as_sections(asm: &str, lma: u32) -> Vec<Section> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut sections =
        assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections;
    for sec in &mut sections {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections
}

/// The four cache-window RAM labels at the PLAIN base (probes run plain-shape
/// only — the shape axis is the port gates' job).
fn cache_ram_labels() -> Vec<Section> {
    as_sections(
        "cpu 68000\n\
         phase $FFFFA834\n\
         Cache_Left_Col:\n\
         \tdc.w 0\n\
         Cache_Head_Col:\n\
         \tdc.w 0\n\
         Cache_Top_Row:\n\
         \tdc.w 0\n\
         Cache_Bottom_Row:\n\
         \tdc.w 0\n",
        0x0200_0000,
    )
}

/// `Tile_Cache_GetCollision` phased at `vma` — the genuine plain VMA is
/// `$418E`; probe (d) supplies a WRONG one.
fn tile_cache_label(vma: &str) -> Vec<Section> {
    as_sections(
        &format!(
            "cpu 68000\n\
             phase {vma}\n\
             Tile_Cache_GetCollision:\n\
             \tdc.b 0\n"
        ),
        0x0280_0000,
    )
}

/// The `VDP_CTRL` equ + the two VDP RAM labels (shape-invariant).
fn vdp_cross_seam() -> Vec<Section> {
    let mut s = as_sections(
        "cpu 68000\n\
         VDP_CTRL = $C00004\n\
         Stub:\n\
         \tdc.w 0\n",
        0x0100_0000,
    );
    s.extend(as_sections(
        "cpu 68000\n\
         phase $FFFF800A\n\
         VDP_Shadow_Table:\n\
         \tdc.b 0\n\
         \tds.b 19\n\
         VDP_Dirty_Mask:\n\
         \tdc.l 0\n",
        0x0200_0000,
    ));
    s
}

/// `BootData_VDPRegs` phased at `vma` — the genuine plain VMA is `$3CE`;
/// probe (d) supplies a WRONG one.
fn bootdata_label(vma: &str) -> Vec<Section> {
    as_sections(
        &format!(
            "cpu 68000\n\
             phase {vma}\n\
             BootData_VDPRegs:\n\
             \tdc.b 0\n"
        ),
        0x0290_0000,
    )
}

fn link_all(sections: Vec<Section>) -> sigil_link::LinkedImage {
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

/// Full plain-shape collision_lookup link with the given twin source and
/// tile-cache VMA.
fn link_collision(src: &str, twin: &str, tile_cache_vma: &str) -> sigil_link::LinkedImage {
    let mut sections = place_module(src, twin, "collision_lookup", "0x4A76", "0x24");
    sections.extend(cache_ram_labels());
    sections.extend(tile_cache_label(tile_cache_vma));
    link_all(sections)
}

/// Full plain-shape vdp_init link with the given twin source and bootdata VMA.
fn link_vdp_init(src: &str, twin: &str, bootdata_vma: &str) -> sigil_link::LinkedImage {
    let mut sections = place_module(src, twin, "vdp_init", "0x1C14", "0x48");
    sections.extend(vdp_cross_seam());
    sections.extend(bootdata_label(bootdata_vma));
    link_all(sections)
}

// ===========================================================================
// Probe (a) — GENUINENESS via the step-1 local const twins
// ===========================================================================

/// Doctor the TWIN's `CTYPE_AIR` from the genuine `0` to `1` and prove the
/// linked collision_lookup bytes DIFFER from the reference window — the
/// twin's value is load-bearing for the `moveq #CTYPE_AIR, d0` immediate,
/// so a drifted twin cannot silently pass the byte gate (independent of the
/// `extern()` drift guard, which would ALSO fire at `check_link_asserts`).
///
/// FALSIFIED (restore-real-value): with the undoctored twin the same
/// compile path equals the reference window byte-for-byte (that is exactly
/// `collision_lookup_port.rs`'s plain gate, which stays green beside this
/// probe).
#[test]
fn collision_lookup_doctored_ctype_air_twin_produces_different_bytes() {
    let Some(src) = real_src("engine/level", "collision_lookup.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let Some(refrom) = read_reference("s4.bin") else { return };
    assert!(
        twin.contains("pub const CTYPE_AIR = 0"),
        "precondition: the twin spells `pub const CTYPE_AIR = 0`"
    );
    let doctored = twin.replace("pub const CTYPE_AIR = 0", "pub const CTYPE_AIR = 1");
    let linked = link_collision(&src, &doctored, "$418E");
    let section = linked.section("collision_lookup").expect("collision_lookup section");
    assert_ne!(
        section.bytes,
        &refrom[0x4A76..0x4C1E],
        "a drifted CTYPE_AIR twin must NOT byte-match the reference"
    );
}

/// Doctor the TWIN's `VDP_Shadow_len` from the genuine `19` to `18` and
/// prove the linked vdp_init bytes DIFFER from the reference window — the
/// twin's value feeds BOTH `moveq #VDP_Shadow_len-1` loop counters.
///
/// FALSIFIED (restore-real-value): the undoctored twin equals the reference
/// window byte-for-byte (`vdp_init_port.rs`'s plain gate).
#[test]
fn vdp_init_doctored_shadow_len_twin_produces_different_bytes() {
    let Some(src) = real_src("engine/system", "vdp_init.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let Some(refrom) = read_reference("s4.bin") else { return };
    assert!(
        twin.contains("pub const VDP_Shadow_len = 19"),
        "precondition: the twin spells `pub const VDP_Shadow_len = 19`"
    );
    let doctored = twin.replace("pub const VDP_Shadow_len = 19", "pub const VDP_Shadow_len = 18");
    let linked = link_vdp_init(&src, &doctored, "$3CE");
    let section = linked.section("vdp_init").expect("vdp_init section");
    assert_ne!(
        section.bytes,
        &refrom[0x1C14..0x1C5C],
        "a drifted VDP_Shadow_len twin must NOT byte-match the reference"
    );
}

// ===========================================================================
// Probe (b) — STANDALONE-COMPILE MISSING-SYMBOL DIAGNOSTIC
// ===========================================================================

/// Compile the real `collision_lookup.emp` WITHOUT its cross-seam sections:
/// the link must fail LOUD, naming a symbol from the module's own cross-seam
/// surface (the four `Cache_*` RAM labels widthed via RelaxAbsSym, or the
/// `Tile_Cache_GetCollision` pc-relative branch target).
#[test]
fn collision_lookup_standalone_compile_is_a_loud_missing_symbol_error() {
    let Some(src) = real_src("engine/level", "collision_lookup.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let sections = place_module(&src, &twin, "collision_lookup", "0x4A76", "0x24");
    let result = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .and_then(|resolved| sigil_link::link(&resolved, &SymbolTable::new()));
    let err = result.expect_err(
        "compiling collision_lookup.emp standalone (no cross-seam sections) must be a loud \
         link error, not a silent/panicking one",
    );
    let names = [
        "Cache_Left_Col",
        "Cache_Head_Col",
        "Cache_Top_Row",
        "Cache_Bottom_Row",
        "Tile_Cache_GetCollision",
    ];
    assert!(
        err.iter().any(|d| d.level == Level::Error && names.iter().any(|n| d.message.contains(n))),
        "expected a loud diagnostic naming one of collision_lookup.emp's five cross-seam \
         symbols, got: {err:?}"
    );
}

/// Compile the real `vdp_init.emp` WITHOUT its cross-seam sections: the link
/// must fail LOUD, naming a symbol from the module's own cross-seam surface
/// (`VDP_CTRL` equ, the two VDP RAM labels, or the `BootData_VDPRegs`
/// pc-relative EA target).
#[test]
fn vdp_init_standalone_compile_is_a_loud_missing_symbol_error() {
    let Some(src) = real_src("engine/system", "vdp_init.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let sections = place_module(&src, &twin, "vdp_init", "0x1C14", "0x48");
    let result = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .and_then(|resolved| sigil_link::link(&resolved, &SymbolTable::new()));
    let err = result.expect_err(
        "compiling vdp_init.emp standalone (no cross-seam sections) must be a loud link \
         error, not a silent/panicking one",
    );
    let names = ["VDP_CTRL", "VDP_Shadow_Table", "VDP_Dirty_Mask", "BootData_VDPRegs"];
    assert!(
        err.iter().any(|d| d.level == Level::Error && names.iter().any(|n| d.message.contains(n))),
        "expected a loud diagnostic naming one of vdp_init.emp's four cross-seam symbols, \
         got: {err:?}"
    );
}

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS
// ===========================================================================

/// Place the real `collision_lookup.emp` at a WRONG base (`$4C0A` instead of
/// the real plain `$4C08`) and prove the placed LMA tracks the map.
///
/// FALSIFIED (restore-real-value): placing at the real `0x4A76` yields
/// `lma == 0x4A76` (the port gate's compile path).
#[test]
fn collision_lookup_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src("engine/level", "collision_lookup.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let sections = place_module(&src, &twin, "collision_lookup", "0x4C0A", "0x24");
    let sec = sections
        .iter()
        .find(|s| s.name == "collision_lookup")
        .expect("placed collision_lookup section");
    assert_eq!(sec.lma, 0x4C0A, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x4A76, "…and therefore differ from the true pin");
}

/// Place the real `vdp_init.emp` at a WRONG base (`$1C16` instead of the
/// real plain `$1C14`) and prove the placed LMA tracks the map.
///
/// FALSIFIED (restore-real-value): placing at the real `0x1C14` yields
/// `lma == 0x1C14` (the port gate's compile path).
#[test]
fn vdp_init_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src("engine/system", "vdp_init.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let sections = place_module(&src, &twin, "vdp_init", "0x1C16", "0x48");
    let sec = sections.iter().find(|s| s.name == "vdp_init").expect("placed vdp_init section");
    assert_eq!(sec.lma, 0x1C16, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x1C14, "…and therefore differ from the true pin");
}

// ===========================================================================
// Probe (d) — PC-RELATIVE TARGET-POSITION GENUINENESS (new this tranche)
// ===========================================================================

/// Re-link the real `collision_lookup.emp` with `Tile_Cache_GetCollision`
/// phased at a WRONG VMA (`$4312`, +4 from the genuine `$418E`): the
/// tail-call `bra.w`'s displacement bytes must CHANGE — the cross-seam pc-relative
/// distance is genuinely computed from the supplied symbol position.
///
/// FALSIFIED (restore-real-value): with the genuine `$418E` the linked bytes
/// equal the reference window (the port gate).
#[test]
fn collision_lookup_wrong_tile_cache_vma_changes_the_bra_bytes() {
    let Some(src) = real_src("engine/level", "collision_lookup.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let Some(refrom) = read_reference("s4.bin") else { return };
    let linked = link_collision(&src, &twin, "$4312");
    let section = linked.section("collision_lookup").expect("collision_lookup section");
    assert_ne!(
        section.bytes,
        &refrom[0x4A76..0x4C1E],
        "a moved Tile_Cache_GetCollision must change the bra.w displacement bytes"
    );
}

/// Re-link the real `vdp_init.emp` with `BootData_VDPRegs` phased at a WRONG
/// VMA (`$3D2` — which happens to be the DEBUG shape's genuine address, the
/// most realistic wrong-shape mixup): the `lea.l (pc)` displacement bytes
/// must CHANGE relative to the plain reference.
///
/// FALSIFIED (restore-real-value): with the genuine plain `$3CE` the linked
/// bytes equal the reference window (the port gate).
#[test]
fn vdp_init_wrong_bootdata_vma_changes_the_pcrel_lea_bytes() {
    let Some(src) = real_src("engine/system", "vdp_init.emp") else { return };
    let Some(twin) = twin_src() else { return };
    let Some(refrom) = read_reference("s4.bin") else { return };
    let linked = link_vdp_init(&src, &twin, "$3D2");
    let section = linked.section("vdp_init").expect("vdp_init section");
    assert_ne!(
        section.bytes,
        &refrom[0x1C14..0x1C5C],
        "a moved BootData_VDPRegs must change the lea (pc) displacement bytes"
    );
}
