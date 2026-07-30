//! Flip Stage 1 · S1.1 — THE ALL-GATES-ON NATIVE DRIVER.
//!
//! The dual-native whole-ROM build: assemble the residual `main.asm` with EVERY
//! `SIGIL_EMP_*` code gate ON (so the AS side org-resumes past each gated region,
//! leaving a hole), natively lower + place EVERY ported `.emp` module at its
//! `pins`-region base through ONE `resolve_layout` + `link` + `emit_rom`, and
//! compare the whole ROM byte-for-byte against the live `asl` `s4.bin` /
//! `s4.debug.bin`. This is the seam-2 whole-ROM template (`seam2_sfx_rom`)
//! generalized from the sound stack to all 53 gates.
//!
//! The sound stack (DAC/MT/SFX banks + the resident Z80 driver + tables) is NOT
//! placed here — it enters via the AS side's BINCLUDE of the seam-emitted `.bin`s
//! (the proven `seam2_sfx_rom` path). Only the 52 CODE/DATA `.emp` modules are
//! natively placed.
//!
//! Module resolution is REAL: the registry's module ids seed a synthetic entry
//! module whose `use` edges drive `build_program`'s reachability BFS, so every
//! placed module's comptime `use` dependencies (types/structs/coords/…) resolve
//! automatically — no hand-maintained ambient-dep lists.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_rom
//! ```

use std::path::Path;

use sigil_frontend_as::{assemble_root, Options as AsOptions};
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{self, place_sections};
use sigil_ir::{Cpu, Module, Section, SymbolTable};
use sigil_link::LinkedImage;

use crate::pins::{self, Region};
use crate::{seam1, seam2};

/// One natively-placed `.emp` module: its dotted id (for the synthetic entry's
/// `use` edge + `build_program` reachability), its declared section name (the
/// `module … in <section>` name; `"text"` for a defaulted module), and its
/// placement region (per-shape base + len from `pins`).
pub struct ModuleSpec {
    pub module_id: &'static str,
    pub section: &'static str,
    pub region: Region,
}

impl ModuleSpec {
    fn base(&self, debug: bool) -> u32 {
        if debug { self.region.debug_base } else { self.region.plain_base }
    }
    fn len(&self, debug: bool) -> usize {
        if debug { self.region.debug_len } else { self.region.plain_len }
    }
}

/// THE REGISTRY — the 52 code/data `.emp` modules Stage 1 places natively.
///
/// GAME_DEBUG / SOUND_DEBUG are Config-A-only (canonically empty in the shipped
/// shapes; their org-resumes assume the Config-A layout) and are NOT in this
/// canonical set. The DAC/MT/SFX sound banks enter via AS BINCLUDE. ACT_DESCRIPTOR
/// / PLAYER_COMMON / TEST_PLAYER / TEST_ENEMY are UNCONDITIONALLY AS-included
/// (no gate) and stay residual.
///
/// `debug` selects the shape: COMPRESSION_SELFTEST is a DEBUG-ONLY module (it emits
/// the `CompressionSelfTest` proc unconditionally and is included only when DEBUG=1;
/// plain carries zero bytes), so it is placed only in the debug shape.
pub fn registry(debug: bool) -> Vec<ModuleSpec> {
    macro_rules! m {
        ($id:literal, $sec:literal, $region:expr) => {
            ModuleSpec { module_id: $id, section: $sec, region: $region }
        };
    }
    let mut specs = vec![
        // ── Engine system ──
        m!("engine.system.vectors", "vectors", pins::VECTORS),
        m!("engine.boot", "boot", pins::BOOT),
        m!("engine.vdp_init", "vdp_init", pins::VDP_INIT),
        m!("engine.dma_queue", "dma_queue", pins::DMA_QUEUE),
        m!("engine.buffers", "buffers", pins::BUFFERS),
        m!("engine.vblank", "vblank", pins::VBLANK),
        m!("engine.hblank", "hblank", pins::HBLANK),
        m!("engine.controllers", "controllers", pins::CONTROLLERS),
        m!("engine.game_loop", "game_loop", pins::GAME_LOOP),
        // ── Engine compression ──
        m!("engine.s4lz", "s4lz", pins::S4LZ),
        m!("engine.zx0", "zx0", pins::ZX0),
        m!("engine.math", "math", pins::MATH),
        // ── Engine objects ──
        m!("engine.objects.dplc", "dplc", pins::DPLC),
        m!("engine.objects.core", "core", pins::CORE),
        m!("engine.objects.sprites", "sprites", pins::SPRITES),
        m!("engine.objects.animate", "animate", pins::ANIMATE),
        m!("engine.objects.collision", "collision", pins::COLLISION),
        m!("engine.objects.rings", "rings", pins::RINGS),
        m!("engine.objects.entity_window", "entity_window", pins::ENTITY_WINDOW),
        m!("engine.objects.children", "children", pins::CHILDREN),
        m!("engine.objects.load_object", "load_object", pins::LOAD_OBJECT),
        // ── Engine level ──
        m!("engine.plane_buffer", "plane_buffer", pins::PLANE_BUFFER),
        m!("engine.tile_cache", "tile_cache", pins::TILE_CACHE),
        m!("engine.collision_lookup", "collision_lookup", pins::COLLISION_LOOKUP),
        m!("engine.section", "section", pins::SECTION),
        m!("engine.camera", "camera", pins::CAMERA),
        m!("engine.parallax", "parallax", pins::PARALLAX),
        m!("engine.load_art", "load_art", pins::LOAD_ART),
        m!("engine.bg", "bg", pins::BG),
        m!("engine.bg_anim", "bg_anim", pins::BG_ANIM),
        // ── Engine debug / sound caller ──
        m!("engine.sound_api", "sound_api", pins::SOUND_API),
        m!("engine.debug.error_handler", "error_handler", pins::ERROR_HANDLER),
        // ── Game player ──
        m!("games.sonic4.player_sensors", "player_sensors", pins::PLAYER_SENSORS),
        m!("games.sonic4.player_ground", "player_ground", pins::PLAYER_GROUND),
        m!("games.sonic4.player_air", "player_air", pins::PLAYER_AIR),
        m!("games.sonic4.player_spindash", "player_spindash", pins::PLAYER_SPINDASH),
        m!("games.sonic4.sonic", "sonic", pins::SONIC),
        // ── Game objects ──
        m!("games.sonic4.test_static", "test_static", pins::TEST_STATIC),
        m!("games.sonic4.test_animated", "test_animated", pins::TEST_ANIMATED),
        m!("games.sonic4.test_solid", "test_solid", pins::TEST_SOLID),
        m!("games.sonic4.test_particle", "test_particle", pins::TEST_PARTICLE),
        m!("games.sonic4.test_emitter", "test_emitter", pins::TEST_EMITTER),
        m!("games.sonic4.test_parent", "test_parent", pins::TEST_PARENT),
        m!("games.sonic4.test_stress_emitter", "test_stress_emitter", pins::TEST_STRESS_EMITTER),
        m!("games.sonic4.test_churn", "test_churn", pins::TEST_CHURN),
        m!("games.sonic4.path_swap", "path_swap", pins::PATH_SWAP),
        // ── Game data ──
        // OBJDEFS: `module … .test_objects` has NO `in <section>`, so its
        // `pub data` lands in the default `"text"` section (verified: the only
        // reachable non-empty `"text"` producer — the sound data modules are
        // unreachable from this set).
        m!("games.sonic4.data.objdefs.test_objects", "text", pins::OBJDEFS),
        m!("games.sonic4.sonic_anims", "sonic_anims", pins::SONIC_ANIMS),
        m!("games.sonic4.particle_anims", "particle_anims", pins::PARTICLE_ANIMS),
        // ── Game test states ──
        m!("games.sonic4.object_test_state", "object_test_state", pins::OBJECT_TEST_STATE),
        m!("games.sonic4.ojz_scroll_test", "ojz_scroll_test", pins::OJZ_SCROLL_TEST),
    ];
    if debug {
        specs.push(m!(
            "engine.compression_selftest",
            "compression_selftest",
            pins::COMPRESSION_SELFTEST
        ));
    }
    specs
}

/// Give EVERY reachable module a glob (`.*`) import of ALL pure-comptime helper
/// modules, so each module's ambient set is the FULL transitively-closed helper
/// closure. This is required because ambient injection is SHALLOW (one `use` level):
/// a consumer of `sst` gets the `Sst` struct but not the `types.ObjRoutine` its
/// field references unless the consumer ALSO imports `types`. Blanketing every
/// module with every helper glob resolves all cross-helper comptime deps.
///
/// Byte-neutral: `collect_pub_comptime` injects only COMPTIME items (const/struct/
/// enum/bitfield/newtype/comptime-fn/vars) — which lower to ZERO bytes and ZERO
/// link symbols — and never `ensure`/`data`/`proc`, so no drift-guard is duplicated
/// and no data/label is re-emitted. Existing helper `use`s are removed first so a
/// helper is imported exactly once (a doubled glob would inject its decls twice).
fn normalize_helper_imports(
    manifest: &mut resolve::manifest::Manifest,
    helper_ids: &[&str],
    also_drop: &[&str],
) {
    use sigil_frontend_emp::ast;

    // Parse one synthetic file of `use <helper>.*` lines to mint real Use items
    // (with valid Path ASTs) — cheaper than hand-building the AST.
    let mut src = String::from("module native_helper_globs\n");
    for id in helper_ids {
        src.push_str(&format!("use {id}.*\n"));
    }
    let (glob_file, _d) = sigil_frontend_emp::parse_str(&src);
    let glob_uses: Vec<ast::Item> = glob_file
        .items
        .into_iter()
        .filter(|i| matches!(i, ast::Item::Use(_)))
        .collect();

    let is_helper_use = |item: &ast::Item| -> bool {
        if let ast::Item::Use(u) = item {
            let base = u.base.segments.join(".");
            helper_ids.contains(&base.as_str()) || also_drop.contains(&base.as_str())
        } else {
            false
        }
    };

    for pm in manifest.modules.iter_mut() {
        let own_id = pm.file.module.path.segments.join(".");
        // Drop existing helper imports (they'd double the glob-injected decls).
        pm.file.items.retain(|i| !is_helper_use(i));
        // Prepend one glob per helper, minus the module's own (ambient never
        // injects a module into itself, but a self-glob is a needless clone).
        let mut prepend: Vec<ast::Item> = glob_uses
            .iter()
            .filter(|i| {
                if let ast::Item::Use(u) = i {
                    u.base.segments.join(".") != own_id
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        prepend.append(&mut pm.file.items);
        pm.file.items = prepend;
    }
}

/// Publicize the PRIVATE comptime items (const/struct/enum/bitfield/newtype/
/// comptime-fn/vars-overlay) of the helper modules, so glob ambient injection pulls
/// a helper's full comptime closure — including private callees like `vdp.emp`'s
/// `target_bits`, invoked by the pub `vdp_comm`. Deliberately leaves `equ` (a
/// link-symbol emitter), `data`, and `proc` untouched: only comptime kinds, which
/// lower to ZERO bytes and ZERO link symbols, so visibility is byte-neutral.
fn publicize_helper_comptime(manifest: &mut resolve::manifest::Manifest, helpers: &[&str]) {
    use sigil_frontend_emp::ast;
    fn walk(items: &mut [ast::Item]) {
        for item in items.iter_mut() {
            match item {
                ast::Item::Const(d) => d.public = true,
                ast::Item::Struct(d) => d.public = true,
                ast::Item::Enum(d) => d.public = true,
                ast::Item::Bitfield(d) => d.public = true,
                ast::Item::Newtype(d) => d.public = true,
                ast::Item::ComptimeFn(d) => d.public = true,
                ast::Item::Vars(d) if d.name.is_some() => d.public = true,
                ast::Item::Section(sec) => walk(&mut sec.items),
                _ => {}
            }
        }
    }
    for pm in manifest.modules.iter_mut() {
        let id = pm.file.module.path.segments.join(".");
        if helpers.contains(&id.as_str()) {
            walk(&mut pm.file.items);
        }
    }
}

/// Emit every generated BINCLUDE the DAC + MT + SFX + resident-blob AS arms read
/// (identical to `seam2_sfx_rom`'s `ensure_generated`). The sound stack is native;
/// the `.bin`s are how those native bytes enter the AS-assembled residual.
pub fn ensure_generated(aeon: &Path) {
    let gen = aeon.join("engine/sound/generated");
    seam1::emit_sound_blob(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_blob (blob): {e}"));
    seam2::emit_dac_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_dac_artifacts: {e}"));
    seam2::emit_mt_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_mt_artifacts: {e}"));
    seam2::emit_sfx_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sfx_artifacts: {e}"));
    seam2::emit_seq_opcode_artifacts(aeon, &gen)
        .unwrap_or_else(|e| panic!("emit_seq_opcode_artifacts: {e}"));
    seam2::emit_sound_tables_artifacts(aeon, &gen)
        .unwrap_or_else(|e| panic!("emit_sound_tables_artifacts: {e}"));
    seam2::emit_pitchtable_artifacts(aeon, &gen)
        .unwrap_or_else(|e| panic!("emit_pitchtable_artifacts: {e}"));
}

/// The AS-side residual: `main.asm` with SOUND_DRIVER_ENABLED + every code gate
/// ON + the DAC/MT/SFX BINCLUDE gates (NO body stubs — the `seam2_sfx_rom`
/// sound path). GAME_DEBUG / SOUND_DEBUG are Config-A-only and stay OFF.
pub fn assemble_native_all_gates_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines: Vec<(String, i64)> = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    // The sound-bank BINCLUDE gates (seam2_sfx_rom): DAC + MT + SFX on, no stubs.
    for g in ["SIGIL_EMP_DAC", "SIGIL_EMP_MT", "SIGIL_EMP_SFX"] {
        defines.push((g.to_string(), 1));
    }
    // Every code gate in the registry.
    for g in code_gate_defines() {
        defines.push((g.to_string(), 1));
    }
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts =
        AsOptions { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (native all-gates AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// The `SIGIL_EMP_*` code-gate names Stage 1 turns ON (the registry's gates,
/// de-duplicated: TEST_OBJECTS is one gate serving test_solid + test_particle).
fn code_gate_defines() -> Vec<&'static str> {
    vec![
        "SIGIL_EMP_VECTORS", "SIGIL_EMP_BOOT", "SIGIL_EMP_VDP_INIT", "SIGIL_EMP_DMA_QUEUE",
        "SIGIL_EMP_BUFFERS", "SIGIL_EMP_VBLANK", "SIGIL_EMP_HBLANK", "SIGIL_EMP_CONTROLLERS",
        "SIGIL_EMP_GAME_LOOP", "SIGIL_EMP_S4LZ", "SIGIL_EMP_ZX0", "SIGIL_EMP_MATH",
        "SIGIL_EMP_DPLC", "SIGIL_EMP_CORE", "SIGIL_EMP_SPRITES", "SIGIL_EMP_ANIMATE",
        "SIGIL_EMP_COLLISION", "SIGIL_EMP_RINGS", "SIGIL_EMP_ENTITY_WINDOW", "SIGIL_EMP_CHILDREN",
        "SIGIL_EMP_LOAD_OBJECT", "SIGIL_EMP_PLANE_BUFFER", "SIGIL_EMP_TILE_CACHE",
        "SIGIL_EMP_COLLISION_LOOKUP", "SIGIL_EMP_SECTION", "SIGIL_EMP_CAMERA", "SIGIL_EMP_PARALLAX",
        "SIGIL_EMP_LOAD_ART", "SIGIL_EMP_BG", "SIGIL_EMP_BG_ANIM", "SIGIL_EMP_COMPRESSION_SELFTEST",
        "SIGIL_EMP_SOUND_API", "SIGIL_EMP_ERROR_HANDLER", "SIGIL_EMP_PLAYER_SENSORS",
        "SIGIL_EMP_PLAYER_GROUND", "SIGIL_EMP_PLAYER_AIR", "SIGIL_EMP_PLAYER_SPINDASH",
        "SIGIL_EMP_SONIC", "SIGIL_EMP_TEST_STATIC", "SIGIL_EMP_TEST_ANIMATED",
        "SIGIL_EMP_TEST_OBJECTS", "SIGIL_EMP_TEST_EMITTER", "SIGIL_EMP_TEST_PARENT",
        "SIGIL_EMP_TEST_STRESS_EMITTER", "SIGIL_EMP_TEST_CHURN", "SIGIL_EMP_PATH_SWAP",
        "SIGIL_EMP_OBJDEFS", "SIGIL_EMP_SONIC_ANIMS", "SIGIL_EMP_PARTICLE_ANIMS",
        "SIGIL_EMP_OBJECT_TEST_STATE", "SIGIL_EMP_OJZ_SCROLL_TEST",
    ]
}

/// Build the placement map (one region per registry section) for `place_sections`.
/// A defaulted-module `"text"` section is mapped to the OBJDEFS geometry (the
/// only reachable non-empty `"text"` producer); every empty comptime `"text"`
/// section packs there contributing zero bytes.
fn emp_map_toml(specs: &[ModuleSpec], debug: bool) -> String {
    let mut out = String::from("fill = 0x00\n");
    for s in specs {
        // Region size must be at least 1 for the map loader; a zero-len region
        // (COMPRESSION_SELFTEST plain) hosts only an empty section, so a nominal
        // size is byte-neutral.
        let size = s.len(debug).max(1);
        out.push_str(&format!(
            "\n[[region]]\nname = \"{}\"\nlma_base = {:#x}\nsize = {:#x}\nkind = \"rom\"\n",
            s.section,
            s.base(debug),
            size,
        ));
    }
    out
}

/// A synthetic entry `.emp` source whose `use` edges reach every registry module,
/// so `build_program`'s reachability BFS pulls them all (and their comptime deps).
fn synthetic_entry_src(specs: &[ModuleSpec]) -> String {
    let mut src = String::from("module native_flip_entry\n\n");
    for s in specs {
        src.push_str(&format!("use {}\n", s.module_id));
    }
    src
}

/// Natively lower + place every registry `.emp` module. Returns the placed
/// sections + the whole program's deferred link asserts (drift guards).
pub fn build_native_emp(
    aeon: &Path,
    debug: bool,
) -> Result<(Vec<Section>, Vec<sigil_ir::LinkAssert>), String> {
    let specs = registry(debug);

    let (mut manifest, mdiags) = resolve::manifest::Manifest::scan(aeon);
    let merr: Vec<_> = mdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !merr.is_empty() {
        return Err(format!("manifest scan: {} error(s); first: {:?}", merr.len(), merr.first()));
    }

    // The pure-comptime HELPER modules (types/consts/structs/fns — no `in
    // <section>`, emit no bytes) whose comptime items every placed module may need.
    const COMPTIME_HELPERS: &[&str] = &[
        "engine.types",
        "engine.coords",
        "engine.constants",
        "engine.structs",
        "engine.objects.sst",
        "engine.objects.aabb",
        "engine.objects.frames",
        "engine.objects.objdef",
        "engine.vdp",
        "engine.irq",
        "engine.z80_bus",
    ];
    // A lone id/path inconsistency in the aeon `.emp` tree: `objdef.emp` imports
    // `engine.system.constants`, but `constants.emp`'s header id is `engine.constants`.
    // Drop that stray import too (it is superseded by the `engine.constants.*` glob)
    // and add a manifest alias so any residual reference still resolves — byte-safe
    // (identical module, same `RF_PRIORITY_SHIFT`), no edit to the frozen aeon tree.
    const HELPER_ALIAS_DROP: &[&str] = &["engine.system.constants"];
    if let Some(&i) = manifest.by_id.get("engine.constants") {
        manifest.by_id.entry("engine.system.constants".to_string()).or_insert(i);
    }

    publicize_helper_comptime(&mut manifest, COMPTIME_HELPERS);
    normalize_helper_imports(&mut manifest, COMPTIME_HELPERS, HELPER_ALIAS_DROP);

    // Inject the synthetic entry as a fresh module in the manifest.
    let entry_id = "native_flip_entry".to_string();
    let src = synthetic_entry_src(&specs);
    let source = sigil_span::SourceId(manifest.modules.len() as u32);
    let (file, pdiags) = sigil_frontend_emp::parse_file(&src, source);
    let perr: Vec<_> = pdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !perr.is_empty() {
        return Err(format!("synthetic entry parse: {perr:?}"));
    }
    let idx = manifest.modules.len();
    manifest.by_id.insert(entry_id.clone(), idx);
    manifest.sources.insert(source, aeon.join("__native_flip_entry__.emp"));
    manifest.modules.push(resolve::manifest::ParsedModule {
        id: entry_id.clone(),
        file,
        path: aeon.join("__native_flip_entry__.emp"),
    });

    // The build-shape comptime defines the `.emp` modules read (mirrors the AS
    // side's `-D`s). SOUND_DRIVER_ENABLED always on; DEBUG is read as a VALUE
    // (`if DEBUG == 1`), so it must be defined in BOTH shapes (0 plain / 1 debug).
    // SOUND_DEBUG_HOTKEYS / SOUND_DBG_MIRROR are the Config-A test-harness flags,
    // read as VALUES (`if X == 1`); OFF in every canonical shape (build.sh never
    // defines them → 0).
    let defines: Vec<(String, i128)> = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("DEBUG".to_string(), if debug { 1 } else { 0 }),
        ("SOUND_DEBUG_HOTKEYS".to_string(), 0),
        ("SOUND_DBG_MIRROR".to_string(), 0),
        // sonic4 game-config comptime flag (games/sonic4/config/game.asm):
        // camera.emp COMPTIME-SELECTs on it.
        ("GAME_CAMERA_JUMP_LOCK".to_string(), 1),
    ];
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.to_path_buf()),
        // object_test_state's `embed(...)` blobs (ring_art.bin, sonic.bin) are
        // AEON-REPO-ROOT-relative (the BINCLUDE path model), so the embed base is
        // the aeon root — matching test_t1_harness_states_port.
        embed_base: Some(aeon.to_path_buf()),
        defines,
    };

    // OPEN program: the `.emp` engine references AS-residual symbols (RAM labels,
    // proc seams) resolved only in the joint link — defer them, don't error here.
    // Per-module embed_base reconciles the aeon tree's two `embed(...)` path
    // conventions: math.emp is module-relative (`"../data/sine.bin"`), everything
    // else here is repo-root-relative (object_test_state's blobs).
    let aeon_root = aeon.to_path_buf();
    let math_dir = aeon.join("engine/system");
    let embed_base_for = move |id: &str| -> Option<std::path::PathBuf> {
        if id == "engine.math" { Some(math_dir.clone()) } else { Some(aeon_root.clone()) }
    };
    let (mut sections, link_asserts, bdiags) =
        resolve::build_program_open_embed(&manifest, &entry_id, None, &opts, &embed_base_for);
    let berr: Vec<_> = bdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !berr.is_empty() {
        return Err(format!(
            "build_program: {} error(s); first: {:?}",
            berr.len(),
            berr.first()
        ));
    }

    // The internal KEYSTONES (player_common / test_player / test_enemy) are
    // UNCONDITIONALLY `include`d as `.asm` in main.asm — no gate — so the AS
    // residual owns their bytes. They are reachable here only because placed player/
    // test modules `use` their comptime overlays/equates (already consumed during
    // lowering); their emitted byte sections are redundant twins of the AS-side code
    // and MUST be dropped, or they double-place. Cross-module label references
    // resolve against the AS twin in the joint link.
    const AS_OWNED_KEYSTONES: &[&str] = &["player_common", "test_player", "test_enemy"];
    sections.retain(|s| !AS_OWNED_KEYSTONES.contains(&s.name.as_str()));

    // Guard: exactly ONE non-empty `"text"` section (OBJDEFS). A second would mean
    // an unexpected defaulted-module data producer slipped into the reachable set
    // and would corrupt OBJDEFS's region — a STOP, not a silent pack.
    let nonempty_text =
        sections.iter().filter(|s| s.name == "text" && !s.image_bytes().is_empty()).count();
    if nonempty_text != 1 {
        return Err(format!(
            "expected exactly 1 non-empty `text` section (OBJDEFS), found {nonempty_text}"
        ));
    }

    let map_toml = emp_map_toml(&specs, debug);
    let map = sigil_link::load_map(&map_toml).map_err(|e| format!("emp map load: {e}"))?;
    let place_diags = place_sections(&mut sections, &map);
    let perr: Vec<_> = place_diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !perr.is_empty() {
        return Err(format!(
            "place_sections: {} error(s); first: {:?}",
            perr.len(),
            perr.first()
        ));
    }

    Ok((sections, link_asserts))
}

/// THE native whole-ROM build: AS residual (all gates ON, sound BINCLUDE) + every
/// placed `.emp` module → ONE resolve_layout + link + emit_rom.
pub fn build_native_rom(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    ensure_generated(aeon);

    let as_side = assemble_native_all_gates_as_side(aeon, debug)?;
    let (emp_sections, link_asserts) = build_native_emp(aeon, debug)?;

    let mut sections = as_side.sections;
    sections.extend(emp_sections);

    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &stubs, true)
        .map_err(|d| format!("resolve_layout: {} diag(s); first: {:?}", d.len(), d.first()))?;

    // Drift-guard ensures. In the ALL-GATES native build every engine `.asm` twin
    // is gated off, so a guard of the form `ensure(extern("X") == X_mirror)` whose
    // constant `X` is homed ONLY in a now-skipped twin cannot resolve — it folds to
    // Poison ("references symbol(s) `X` not defined in this link"), NOT a drift
    // failure. Those guards are INAPPLICABLE here (the twin they compare against is
    // gone — the Stage-2 "guard retires with its twin" state, reached early because
    // Stage 1 flips all gates); the whole-ROM byte gate is the authoritative drift
    // oracle for every USED constant. A genuine drift FAILURE folds to a Value(0)
    // and carries the guard's OWN message — those stay HARD failures.
    let adiags = sigil_link::check_link_asserts(&resolved, &stubs, &link_asserts);
    let (inapplicable, real): (Vec<_>, Vec<_>) = adiags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .partition(|d| d.message.contains("not defined in this link"));
    if !real.is_empty() {
        return Err(format!(
            "drift-guard ensure(s) FIRED (real drift): {} error(s); first: {:?}",
            real.len(),
            real.first()
        ));
    }
    if std::env::var("NATIVE_DEBUG").is_ok() {
        eprintln!(
            "NATIVE_DEBUG: {} drift guard(s) inapplicable (twin .asm gated off; byte gate is the oracle)",
            inapplicable.len()
        );
    }

    let linked: LinkedImage = sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("link: {} diag(s); first: {:?}", d.len(), d.first()))?;

    let map_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("load sigil.map.toml: {e}"))?;
    sigil_link::emit_rom(&linked, &map).map_err(|e| format!("emit_rom: {e}"))
}
