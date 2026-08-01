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

use std::collections::HashMap;
use std::path::Path;

use sigil_frontend_as::{assemble_root, assemble_root_relocating, Options as AsOptions};
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{self, place_sections};
use sigil_ir::{Cpu, Fragment, Module, Section, SectionPlacement, SymbolTable};
use sigil_link::LinkedImage;

use crate::pins::{self, Region};
use crate::{seam1, seam2};

// ── Flip Stage 2 · S1.2 — THE GAME PROFILE (the off-canonical driver parameters) ──
//
// The Stage-1 native driver is sonic4-shaped throughout (registry, defines, keystones,
// the OBJDEFS `text` guard, the drift-guard allowlist, sound-on). The three off-canonical
// targets (demo plain/debug, Config-A, Config-B) reuse the SAME chainer + split-golden
// machinery through a `GameProfile` that carries every sonic4-hardcoding as data.

/// Where a target's declared per-region SIZES come from (the load-bearing S1.2 finding:
/// the chainer must reserve each section's exact asl span or relaxation settles at a
/// different fixpoint — see the S1.2 chainer note).
pub enum SizeSource {
    /// Canonical sonic4 (retired default): the baked lmas ARE asl-correct, so a pinned
    /// resolve reproduces asl and each section's post-relax span is its exact asl size
    /// (the bootstrap). Kept for the asl-witness bootstrap path; the shipped canonical
    /// build uses `Computed` (§17 Wave-B B-0).
    PinnedBaked,
    /// Off-canonical (demo/Config): the AS residual carries WRONG sonic4 resume orgs,
    /// so ORDER and the org-island ANCHORS come from the FROZEN asl listing table
    /// (label → address); every non-island section's base is then PACKED from live-
    /// measured sizes (see `packed_true_bases`), so a size-changing `.emp` parcel
    /// shifts downstream sections automatically instead of colliding with stale pins.
    Frozen(HashMap<String, u32>),
}

/// One off-canonical / canonical target's full driver parameterization.
pub struct GameProfile {
    pub name: &'static str,
    /// The AS residual root, relative to the aeon tree (`games/<g>/main.asm`).
    pub game_root_rel: &'static str,
    /// The game's RAM module id (item #7c). Its region-form `vars` block chains
    /// `game_ram @ after(upper_ram)` onto the engine RAM, so it must be reachable
    /// from the synthetic entry (so its `pub vars` labels export to the joint
    /// link) and harvested (so eager AS reads of game RAM labels resolve).
    pub game_ram_module: &'static str,
    /// The game's `.emp` constants module, relative to the aeon tree (conversion
    /// Parcel F). `Some` when the game's config constants are `.emp`-authored
    /// (`games.sonic4.constants`): `harvest_game_constants` reads its `pub const`s
    /// and injects them as GUARDED AS `-D` defines + link EquSyms, so the residual
    /// AS and the game-agnostic engine `.emp` (rings/entity_window/camera drift
    /// guards) resolve them. `None` for a game whose config is still AS-authored.
    /// Both shipped games are `.emp`-config now (sonic4 Parcel F, demo Parcel
    /// H-demo = `games.demo.constants`).
    pub game_constants_rel: Option<&'static str>,
    /// The game's `.emp` sound-id module, relative to the aeon tree (conversion
    /// Parcel F2, `games.sonic4.sound_ids` = `games/sonic4/config/sound_ids.emp`).
    /// `Some` when the game's song / SFX ids + priority ladder are `.emp`-authored;
    /// harvested exactly like `game_constants_rel` (guarded `-D` + link EquSyms), so
    /// the residual AS + the game-agnostic engine `.emp` (sound_api's typed SFXID/
    /// SONG mirrors, boot's `moveq #SONG_MOVINGTRUCKS`) resolve them. Its `SONG_COUNT`
    /// is `if DEBUG == 1`, so the harvest seeds `DEBUG` from `debug`. `None` for a
    /// game whose sound ids are still AS-authored (demo has no sound stack).
    pub game_sound_ids_rel: Option<&'static str>,
    /// The game's SFX-bank data module, relative to the aeon tree (conversion
    /// Parcel F2, `data.sfx_bank` = `games/sonic4/data/sound/sfx/sfx_bank.emp`).
    /// `Some` when the SFX-bank id counts (`SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN`,
    /// DERIVED from the `SfxTable` rows) are `.emp`-authored: the harvest injects them
    /// as guarded `-D` + link EquSyms so the residual AS `soundBankHead` (sound_bank.inc)
    /// reads `SFX_TABLE_LEN` in the `SfxBlobWinTab` span guard. `None` for a game with
    /// no SFX bank (demo). The seam Z80 emit sources the same values via
    /// `sfx_bank_authority_consts`.
    pub game_sfx_bank_rel: Option<&'static str>,
    pub debug: bool,
    /// Sound ON: pass `-D SOUND_DRIVER_ENABLED`, the DAC/MT/SFX BINCLUDE gates, and
    /// run `ensure_generated`. OFF (demo/Config-B): no sound define at all (AS `ifdef`
    /// checks DEFINEDNESS — a `=0` would still take the sound arm), no BINCLUDE, no gen.
    pub sound_on: bool,
    /// Extra AS `-D`s beyond the sound + code gates (Config-A: SOUND_DEBUG_HOTKEYS /
    /// SOUND_DBG_MIRROR + the GAME_DEBUG / SOUND_DEBUG code gates).
    pub extra_as_defines: Vec<(&'static str, i64)>,
    /// The `SIGIL_EMP_*` code gates the AS side turns ON (each gate holes out an `.asm`
    /// twin the registry places natively).
    pub code_gates: Vec<&'static str>,
    pub registry: Vec<ModuleSpec>,
    /// The build-shape comptime defines the `.emp` modules read.
    pub emp_defines: Vec<(&'static str, i128)>,
    /// Enforce exactly ONE non-empty `text` section (OBJDEFS). OFF for demo (no objdefs).
    pub require_one_text: bool,
    /// The inapplicable-drift-guard allowlist (t24 both-directions), per target.
    pub inapplicable_guards: Vec<(&'static str, &'static str)>,
    pub size_source: SizeSource,
    /// `EndOfRom` — the assembled-bar length.
    pub assembled_len: usize,
}

impl GameProfile {
    pub fn game_root(&self, aeon: &Path) -> std::path::PathBuf {
        aeon.join(self.game_root_rel)
    }

    /// The per-game placement map (`games/<g>/map.toml`), a sibling of `main.asm`
    /// (`game_root_rel`). config_a/config_b reuse sonic4's (their root is sonic4's).
    pub fn map_path(&self, aeon: &Path) -> std::path::PathBuf {
        let root = std::path::Path::new(self.game_root_rel);
        let dir = root.parent().unwrap_or(std::path::Path::new(""));
        aeon.join(dir).join("map.toml")
    }
}

/// Load a frozen off-canonical size table (`golden/offcanonical_sizes/<name>.txt`):
/// the committed `LABEL 0xADDR` rows (comment lines skipped) → a `label → addr` map.
pub fn load_frozen_table(name: &str) -> HashMap<String, u32> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("golden/offcanonical_sizes")
        .join(name);
    let txt = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("read frozen table {}: {e}", p.display()));
    let mut m = HashMap::new();
    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, addr)) = line.rsplit_once(' ') {
            let a = addr.trim().trim_start_matches("0x");
            if let Ok(v) = u32::from_str_radix(a, 16) {
                m.insert(name.trim().to_string(), v);
            }
        }
    }
    m
}

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
        // Parcel K4: the $100-$1FF ROM header is native (was header.inc's gameHeader
        // macro). Game-specific (games.sonic4.header / games.demo.header); the
        // strings are typed `[u8; N]` (the width guard). Boundary key Checksum ($18E).
        m!("games.sonic4.header", "header", pins::HEADER),
        m!("engine.boot", "boot", pins::BOOT),
        // Parcel K2 — boot_data ports to `.emp` as TWO sections (the $3FE map
        // hole: the engine.z80_init idle packs between them in the no-sound
        // shapes; the resident driver blob rides `boot_head` in sound-on). Two
        // ModuleSpecs, one per section — the pinned bootstrap's emp_map_toml maps
        // one region per spec; the Frozen (shipped) path packs both from the
        // frozen BootData/BootData_End pins.
        m!("engine.boot_data", "boot_head", pins::BOOT_HEAD),
        m!("engine.boot_data", "boot_tail", pins::BOOT_TAIL),
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
        // Parcel K4 B1: the ROM terminus (EndOfRom + the 3 walls), was engine.inc's
        // `EndOfRom:` label + the `if … error` guards. Zero-length section placed
        // LAST (boundary key EndOfRom, already frozen in all six tables). The plane
        // wall is a comptime ensure; EndOfRom evenness/4MB are link-time asserts.
        m!("engine.epilogue", "epilogue", pins::EPILOGUE),
        // ── Game player ──
        // player_common fully flipped (conv-d #49): player_common.asm deleted. The
        // module owns the PlayerV overlay + PPHYS_*/macro templates (state files
        // import by `use`); camera.emp late-binds the one _pl_state offset it
        // link-exports as an `equ`.
        m!("games.sonic4.player_common", "player_common", pins::PLAYER_COMMON),
        m!("games.sonic4.player_sensors", "player_sensors", pins::PLAYER_SENSORS),
        m!("games.sonic4.player_ground", "player_ground", pins::PLAYER_GROUND),
        m!("games.sonic4.player_air", "player_air", pins::PLAYER_AIR),
        m!("games.sonic4.player_spindash", "player_spindash", pins::PLAYER_SPINDASH),
        m!("games.sonic4.sonic", "sonic", pins::SONIC),
        // ── Game objects ──
        // test_player + test_enemy fully flipped (conv-d #48/#47): both .asm deleted.
        // test_player.emp owns TPlayerV; test_animated.emp owns DplcV; STUB_FLOOR_Y
        // is object_test_state.emp's; ENEMY_PATROL_SPEED is test_objects.emp's.
        m!("games.sonic4.test_player", "test_player", pins::TEST_PLAYER),
        m!("games.sonic4.test_enemy", "test_enemy", pins::TEST_ENEMY),
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
        // The OJZ parallax block (conv-g): 6 deform tables + 20 parallax_config
        // records, authored via games.sonic4.parallax_configs (+ engine.level.parallax_dsl).
        m!("games.sonic4.parallax_configs", "parallax_configs", pins::PARALLAX_CONFIGS),
        // test_mappings (conv-h #35): the test-object sprite mapping index
        // (Map_TestObj word-offset table + 3 frame records), authored via the
        // `offsets` construct in games.sonic4.data.mappings.test_mappings.
        m!("games.sonic4.test_mappings", "test_mappings", pins::TEST_MAPPINGS),
        m!("games.sonic4.sonic_anims", "sonic_anims", pins::SONIC_ANIMS),
        m!("games.sonic4.particle_anims", "particle_anims", pins::PARTICLE_ANIMS),
        // Parcel K3 run A: the OJZ act1 interior island HEAD — the contiguous run
        // BEFORE the descriptor. Two native `.emp` sections (both generator-emitted):
        //   entity_data  — the 9-section type tables / object placements / ring lists
        //                  (objentry/objend replaced by packed 3-word records; the
        //                  last 2 macros.asm consumers gone)
        //   ojz_act_pool — 3 ZX0 page embeds + the OJZ_Act_Pool_PageTable
        // With these + the descriptor + the run-B tail native, act_descriptor.asm is
        // DELETED (the OJZ block is fully `.emp`).
        m!("games.sonic4.ojz_entity_data_act1", "entity_data", pins::ENTITY_DATA),
        m!("games.sonic4.ojz_act_pool_act1", "ojz_act_pool", pins::OJZ_ACT_POOL),
        // act_descriptor (kill row 93): the OJZ act1 descriptor table; the body/
        // section table places here.
        m!("games.sonic4.act_descriptor_ojz_act1", "act_descriptor", pins::ACT_DESCRIPTOR),
        // Parcel K3 run B: the OJZ act1 interior island TAIL — the contiguous run
        // after the descriptor. Three native `.emp` sections (the generators emit
        // #32/#28; the palette/BG BINCLUDEs dissolved into act_assets.emp), placed
        // by contiguity after the descriptor:
        //   sec_block_blobs — OJZ_Sec{0..8}_Blocks (Sec4=Sec2 dedup equ), 8 embeds
        //   ojz_act_assets  — OJZ_Palette / BGND_Palette / OJZ_Act1_BG_{Layout,Tiles}
        //   ojz_bg_anim     — BgAnim_Table (disabled stub) + BgAnim_Banks
        m!("games.sonic4.ojz_sec_block_blobs_act1", "sec_block_blobs", pins::SEC_BLOCK_BLOBS),
        m!("games.sonic4.ojz_act_assets_act1", "ojz_act_assets", pins::OJZ_ACT_ASSETS),
        m!("games.sonic4.ojz_bg_anim_act1", "ojz_bg_anim", pins::OJZ_BG_ANIM),
        // Parcel K4: the global collision + Sonic character data (HeightMaps ..
        // Art_Sonic), was the flat BINCLUDE island at the tail of main.asm's
        // gameDataIncludes. Native `embed()` section; boundary key HeightMaps.
        m!("games.sonic4.collision_data", "collision_data", pins::COLLISION_DATA),
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

/// The sonic4 code gates (the registry's gates, de-duplicated: TEST_OBJECTS is one
/// gate serving test_solid + test_particle). Shared by canonical sonic4 + Config-A/B.
fn sonic4_code_gates() -> Vec<&'static str> {
    code_gate_defines()
}

/// The 30 ENGINE code gates the demo turns on (engine-only registry; no game modules,
/// no sound). COMPRESSION_SELFTEST is added per-shape (debug-only).
fn demo_code_gates(debug: bool) -> Vec<&'static str> {
    let mut g = vec![
        "SIGIL_EMP_VECTORS", "SIGIL_EMP_BOOT", "SIGIL_EMP_VDP_INIT", "SIGIL_EMP_DMA_QUEUE",
        "SIGIL_EMP_BUFFERS", "SIGIL_EMP_VBLANK", "SIGIL_EMP_HBLANK", "SIGIL_EMP_CONTROLLERS",
        "SIGIL_EMP_GAME_LOOP", "SIGIL_EMP_S4LZ", "SIGIL_EMP_ZX0", "SIGIL_EMP_MATH",
        "SIGIL_EMP_DPLC", "SIGIL_EMP_CORE", "SIGIL_EMP_SPRITES", "SIGIL_EMP_ANIMATE",
        "SIGIL_EMP_COLLISION", "SIGIL_EMP_RINGS", "SIGIL_EMP_ENTITY_WINDOW", "SIGIL_EMP_CHILDREN",
        "SIGIL_EMP_LOAD_OBJECT", "SIGIL_EMP_PLANE_BUFFER", "SIGIL_EMP_TILE_CACHE",
        "SIGIL_EMP_COLLISION_LOOKUP", "SIGIL_EMP_SECTION", "SIGIL_EMP_CAMERA", "SIGIL_EMP_PARALLAX",
        "SIGIL_EMP_LOAD_ART", "SIGIL_EMP_BG", "SIGIL_EMP_BG_ANIM", "SIGIL_EMP_ERROR_HANDLER",
        "SIGIL_EMP_EPILOGUE",
    ];
    if debug {
        g.push("SIGIL_EMP_COMPRESSION_SELFTEST");
    }
    // The Z80 idle flips native in the no-sound demo too (kill row 55).
    g.push("SIGIL_EMP_Z80_INIT");
    // The demo GAME modules place natively (Parcel H-demo): each gate holes out its
    // `.asm` twin's include in games/demo/main.asm (org-resumed).
    g.push("SIGIL_EMP_DEMO_BOX");
    g.push("SIGIL_EMP_DEMO_DATA");
    g.push("SIGIL_EMP_DEMO_STATE");
    g
}

/// The engine-only registry (demo): the `engine.*` modules of the sonic4 registry,
/// minus `engine.sound_api` (demo is sound-OFF → the sound-caller `.asm`/`.emp` is
/// not in the demo layout at all). The region bases/lens are sonic4-shape and are
/// IGNORED under `SizeSource::Frozen` (the chainer sources demo sizes from the frozen
/// listing table); only the module id + section name are load-bearing.
fn demo_registry(debug: bool) -> Vec<ModuleSpec> {
    let mut r: Vec<ModuleSpec> = registry(debug)
        .into_iter()
        .filter(|m| m.module_id.starts_with("engine.") && m.module_id != "engine.sound_api")
        .collect();
    // The Z80 idle places natively in the no-sound demo (kill row 55); `z80_init`
    // is not in the shared `registry()` (sound-on shapes must not place it), so add
    // it here explicitly.
    r.push(ModuleSpec { module_id: "engine.z80_init", section: "z80_idle", region: DUMMY_REGION });
    // The demo GAME modules (Parcel H-demo): the object-code-bank island the demo
    // main.asm holes out. `demo_data` lands its `pub data` in the named `demo_data`
    // section (not the default `text`), so the require_one_text guard stays off.
    // The regions are DUMMY_REGION — cosmetic under Frozen (the chainer packs from
    // the frozen demo tables' DemoBox_Main/ObjDef_DemoBox/GameState_Demo_Init
    // anchors); only the module id + section name are load-bearing.
    r.push(ModuleSpec { module_id: "games.demo.demo_box", section: "demo_box", region: DUMMY_REGION });
    r.push(ModuleSpec { module_id: "games.demo.data.demo_data", section: "demo_data", region: DUMMY_REGION });
    r.push(ModuleSpec { module_id: "games.demo.demo_state", section: "demo_state", region: DUMMY_REGION });
    // Parcel K4: the demo's $100-$1FF ROM header is native too (games.demo.header;
    // the shared engine.inc no longer invokes the gameHeader macro). Boundary key
    // Checksum→GameHeader, base $100 (cosmetic pin under Frozen).
    r.push(ModuleSpec { module_id: "games.demo.header", section: "header", region: pins::HEADER });
    r
}

/// CANONICAL sonic4 (the Stage-1 shape) as a profile — the regression harness for the
/// GameProfile refactor: `native_rom` / `native_declared_chain` / `native_full_rom`
/// build through this and must stay byte-green.
pub fn sonic4_profile(debug: bool) -> GameProfile {
    sonic4_profile_with(
        SizeSource::Frozen(load_frozen_table(if debug { "s4_debug.txt" } else { "s4.txt" })),
        debug,
    )
}

/// The canonical literal with an explicit placement source — the shared body of
/// `sonic4_profile` (Frozen, the shipped shape) and `sonic4_pinned_profile` (the
/// PinnedBaked bootstrap, which must not touch the table file it exists to mint).
pub fn sonic4_profile_with(size_source: SizeSource, debug: bool) -> GameProfile {
    GameProfile {
        name: if debug { "sonic4_debug" } else { "sonic4" },
        game_root_rel: "games/sonic4/main.asm",
        game_ram_module: "games.sonic4.ram",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug,
        sound_on: true,
        extra_as_defines: vec![],
        code_gates: sonic4_code_gates(),
        registry: registry(debug),
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 1),
            ("DEBUG", if debug { 1 } else { 0 }),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            ("GAME_CAMERA_JUMP_LOCK", 1),
            // sonic4 game-config (games/sonic4/config/constants.asm); the engine
            // `.emp` reads these as -D (rings.emp / entity_window.emp), the
            // `ensure(extern(..)==..)` cross-checks them against the AS config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        size_source,
        assembled_len: if debug { pins::DEBUG_ASSEMBLED_LEN } else { pins::ASSEMBLED_LEN },
    }
}

/// CANONICAL sonic4 at PINNED-BAKED placement — the BOOTSTRAP profile. Exists for
/// exactly one job: deriving the canonical frozen tables (`s4.txt` / `s4_debug.txt`)
/// from the committed pins layout the first time (and for any deliberate re-bootstrap
/// off a pinned resolve). Every shipped canonical build uses `sonic4_profile` (Frozen).
pub fn sonic4_pinned_profile(debug: bool) -> GameProfile {
    sonic4_profile_with(SizeSource::PinnedBaked, debug)
}

/// §17 Wave-B B-0 BOOTSTRAP: derive the canonical boundary table (one head label per
/// ROM section, at `lma + offset`, plus `EndOfRom`) from the PINNED canonical resolve —
/// the committed pins layout is the provisional-base authority the packing walk needs.
/// Runs against unchanged sources; thereafter `derive_frozen_table` (over the Frozen
/// profile) refreshes the committed table like any other target's.
pub fn derive_canonical_bootstrap_table(
    aeon: &Path,
    debug: bool,
) -> Result<std::collections::BTreeMap<String, u32>, String> {
    let profile = sonic4_pinned_profile(debug);
    ensure_generated(aeon);
    let as_side = assemble_as_side(aeon, &profile)?;
    let (emp, _asserts) = build_emp(aeon, &profile)?;
    let mut sections: Vec<Section> = as_side.sections;
    sections.extend(emp);
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &stubs, true)
        .map_err(|d| format!("bootstrap resolve_layout: {} diag(s); first {:?}", d.len(), d.first()))?;
    let mut out = std::collections::BTreeMap::new();
    for sec in &resolved {
        if !is_rom_section(sec) {
            continue;
        }
        if let Some(head) = sec.labels.iter().min_by_key(|l| l.offset) {
            out.insert(head.name.clone(), sec.lma.wrapping_add(head.offset));
        }
        for l in &sec.labels {
            if l.name == "EndOfRom" {
                out.insert(l.name.clone(), sec.lma.wrapping_add(l.offset));
            }
        }
    }
    if !out.contains_key("EndOfRom") {
        return Err("bootstrap: EndOfRom label absent from the pinned resolve".into());
    }
    Ok(out)
}

/// DEMO (plain / debug) — engine-only registry, sound OFF, sizes from the frozen
/// `demo.txt` / `demo_debug.txt`. GAME_CAMERA_JUMP_LOCK=0 (demo's config selects the
/// inert camera path). No objdefs → the one-text guard is OFF.
pub fn demo_profile(debug: bool) -> GameProfile {
    GameProfile {
        name: if debug { "demo_debug" } else { "demo" },
        game_root_rel: "games/demo/main.asm",
        game_ram_module: "games.demo.ram",
        // Conversion Parcel H-demo (#14): the demo's game config is `.emp`-authored
        // (`games.demo.constants` = games/demo/config/constants.emp), harvested into
        // guarded AS `-D` + link EquSyms exactly like the sonic4 side.
        game_constants_rel: Some("games/demo/config/constants.emp"),
        game_sound_ids_rel: None,
        game_sfx_bank_rel: None,
        debug,
        sound_on: false,
        extra_as_defines: vec![],
        code_gates: demo_code_gates(debug),
        registry: demo_registry(debug),
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 0),
            ("DEBUG", if debug { 1 } else { 0 }),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            ("GAME_CAMERA_JUMP_LOCK", 0),
            // demo game-config (games/demo/config/constants.emp) engine-VARYING
            // interface values — homed here (not the `.emp`) per the `-D`-not-in-
            // `.emp` rule; the values that DIFFER from sonic4.
            ("MAX_RING_BUFFER", 16),
            ("VRAM_RING_PLACEHOLDER", 0x3E4),
            ("COLLECTED_WINDOW_SLOTS", 4),
        ],
        require_one_text: false,
        inapplicable_guards: DEMO_INAPPLICABLE_GUARDS.to_vec(),
        size_source: SizeSource::Frozen(load_frozen_table(if debug {
            "demo_debug.txt"
        } else {
            "demo.txt"
        })),
        assembled_len: 0x11224,
    }
}

/// A cosmetic region for the Config-A off-canonical debug modules (game_debug /
/// sound_debug): their placement base/len are IGNORED under `SizeSource::Frozen`
/// (the chainer computes them from the config listing), so only the id + section name
/// are load-bearing.
const DUMMY_REGION: Region =
    Region { plain_base: 0, debug_base: 0, plain_len: 1, debug_len: 1 };

/// CONFIG-B (off-canonical no-sound): sonic4 game, SOUND_DRIVER_ENABLED OFF, plain.
/// Registry = the sonic4 set MINUS `engine.sound_api` (no sound caller) PLUS the Z80
/// idle (kill row 55): with `SIGIL_EMP_Z80_INIT` on, boot_data.asm's no-sound else-arm
/// takes the numeric-size path and `z80_init.emp`'s `z80_idle` section places at the
/// frozen `Z80_IdleProgram` base (0x3d8). Sizes from `config_b.txt`.
pub fn config_b_profile() -> GameProfile {
    let mut registry: Vec<ModuleSpec> =
        registry(false).into_iter().filter(|m| m.module_id != "engine.sound_api").collect();
    registry.push(ModuleSpec {
        module_id: "engine.z80_init",
        section: "z80_idle",
        region: DUMMY_REGION,
    });
    let mut code_gates = sonic4_code_gates();
    code_gates.push("SIGIL_EMP_Z80_INIT");
    GameProfile {
        name: "config_b",
        game_root_rel: "games/sonic4/main.asm",
        game_ram_module: "games.sonic4.ram",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug: false,
        sound_on: false,
        extra_as_defines: vec![],
        code_gates,
        registry,
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 0),
            ("DEBUG", 0),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            ("GAME_CAMERA_JUMP_LOCK", 1),
            // Config-B is the sonic4 game (sound off), so sonic4's game-config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        size_source: SizeSource::Frozen(load_frozen_table("config_b.txt")),
        assembled_len: 0x434d0,
    }
}

/// CONFIG-A with the player keystones flipped into the chained set. Post Stage-3 P2
/// the flip IS the shipped `config_a_profile` (the keystones place natively by
/// default), so this is now a thin alias retained as the row-94 fold-vs-placement
/// regression bar (`keystone_flip_relocation.rs`): its assembled anchor MUST equal
/// `config_a.bin`'s prefix.
pub fn config_a_keystones_flipped_profile() -> GameProfile {
    config_a_profile()
}

/// CONFIG-A (off-canonical debug + sound + hotkeys + mirror): sonic4 game, __DEBUG__ +
/// SOUND_DRIVER_ENABLED + SOUND_DEBUG_HOTKEYS + SOUND_DBG_MIRROR, so `game_debug` +
/// `sound_debug` (canonically empty) become NON-empty placed modules. Registry = the
/// sonic4 DEBUG set PLUS those two. Sizes from `config_a.txt`.
pub fn config_a_profile() -> GameProfile {
    let mut registry = registry(true);
    registry.push(ModuleSpec {
        module_id: "games.sonic4.game_debug",
        section: "game_debug",
        region: DUMMY_REGION,
    });
    registry.push(ModuleSpec {
        module_id: "engine.debug.sound_debug",
        section: "sound_debug",
        region: DUMMY_REGION,
    });
    let mut code_gates = sonic4_code_gates();
    code_gates.push("SIGIL_EMP_GAME_DEBUG");
    code_gates.push("SIGIL_EMP_SOUND_DEBUG");
    GameProfile {
        name: "config_a",
        game_root_rel: "games/sonic4/main.asm",
        game_ram_module: "games.sonic4.ram",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug: true,
        sound_on: true,
        extra_as_defines: vec![("SOUND_DEBUG_HOTKEYS", 1), ("SOUND_DBG_MIRROR", 1)],
        code_gates,
        registry,
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 1),
            ("DEBUG", 1),
            ("SOUND_DEBUG_HOTKEYS", 1),
            ("SOUND_DBG_MIRROR", 1),
            ("GAME_CAMERA_JUMP_LOCK", 1),
            // Config-A is the sonic4 game (debug + sound), so sonic4's game-config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        size_source: SizeSource::Frozen(load_frozen_table("config_a.txt")),
        assembled_len: 0x5f65a,
    }
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

/// The AS-side residual for `profile`: `main.asm` with the profile's sound state,
/// code gates, and extras. Sound ON adds `-D SOUND_DRIVER_ENABLED` + the DAC/MT/SFX
/// BINCLUDE gates (the `seam2_sfx_rom` sound path). Sound OFF (demo/Config-B) passes
/// NO sound define — AS `ifdef` tests DEFINEDNESS, so a `=0` would wrongly take the
/// sound arm; the demo golden builds by leaving it undefined.
/// Harvest `engine/system/constants.emp`'s resolved `pub const` values (Stage-3
/// P5 ownership flip, Option A). The `.emp` module is the SOLE author of these
/// engine constants; this reads their resolved integer values so they can be
/// injected as GUARDED AS `-D` defines — the residual AS (`ram.asm`'s `ds`
/// sizes, `macros.asm`'s `if`/shift, `constants.asm`'s derived equates) reads
/// them at COMPTIME, which link-deferral structurally cannot serve.
///
/// `VDP_Shadow_len` is EXCLUDED: it is not a `constants.asm` `=`, it is
/// STRUCT-GENERATED by `structs.asm`'s `VDP_Shadow endstruct`. Injecting it as a
/// guarded define would (correctly) collide with that struct-generated symbol.
/// It stays the struct twin (its `.emp` `pub const` + drift guard), retiring
/// with the structs ownership flip (kill-list row 1's `VDP_Shadow_len` note).
///
/// ORDERING (the architectural fact, stated plainly): the `.emp` constant
/// definitions now flow INTO the residual AS assembly. So this harvest LOWERS
/// `engine.constants` FIRST, then `assemble_as_side` seeds the harvested values
/// before the AS residual assembles. The dependency direction is `.emp` →
/// residual AS, one way (the module is self-contained — every `pub const` folds
/// within it), so a single standalone lower suffices; it does not need the full
/// emp build.
pub fn harvest_engine_constants(aeon: &Path) -> Result<Vec<(String, i64)>, String> {
    let path = aeon.join("engine/system/constants.emp");
    let src = std::fs::read_to_string(&path)
        .map_err(|e| format!("harvest_engine_constants: read {}: {e}", path.display()))?;
    let (file, pdiags) = sigil_frontend_emp::parse_str(&src);
    if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("harvest_engine_constants: parse: {:?}", pdiags.first()));
    }
    let (vals, diags) = sigil_frontend_emp::eval::eval_all_pub_consts(&file, Some(aeon), &[]);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("harvest_engine_constants: resolve: {:?}", diags.first()));
    }
    // VDP_Shadow_len is a STRUCT length, owned by the struct-offset harvest
    // (harvest_engine_struct_offsets, VdpShadow), so it is excluded here to keep
    // exactly one injector — the same reason it was excluded when structs.asm
    // owned it, re-homed to the struct twin.
    Ok(vals.into_iter().filter(|(n, _)| n != "VDP_Shadow_len").collect())
}

/// Conversion Parcel F: harvest the game's `.emp` constants module (row 21,
/// `games.sonic4.constants` = `games/sonic4/config/constants.emp`). Its `pub
/// const`s are the SOLE authority for the Sonic 4 game constants (player-state /
/// animation ids, ring-buffer + collected-window sizing, test-scaffold VRAM);
/// this reads their resolved values so `assemble_as_side` injects them as GUARDED
/// AS `-D` defines + link EquSyms, exactly as [`harvest_engine_constants`] does
/// for the engine.
///
/// Unlike the engine constants module, the game module is NOT self-contained: its
/// `COLLECTED_PARK_ENTRY_SIZE = 1 + 2 * COLLECTED_MASK_BYTES` reads the engine
/// constant `COLLECTED_MASK_BYTES` (via `use engine.constants`), and its VRAM
/// consts are typed `VramTile` (via `use engine.system.types`). `eval_all_pub_consts`
/// resolves each `pub const`'s VALUE from the file's own items only (it does not
/// load `use`-d modules) and IGNORES the type annotation — so the one cross-module
/// value dependency is served by seeding the engine constants FIRST as defines
/// (`eval_path` falls back to defines after consts/equs), the same pattern the
/// `-D` seam already uses.
pub fn harvest_game_constants(aeon: &Path, rel: &str, debug: bool) -> Result<Vec<(String, i64)>, String> {
    // Seed the engine constants as defines so the game module's lone cross-module
    // reference (`COLLECTED_MASK_BYTES`) folds inside the standalone eval. Also seed
    // `DEBUG` (the build shape) — `sound_ids.emp`'s `SONG_COUNT = if DEBUG == 1 {..}`
    // is shape-dependent; the constants module ignores it (harmless).
    let engine = harvest_engine_constants(aeon)?;
    let mut seed: Vec<(String, i128)> = engine.iter().map(|(n, v)| (n.clone(), *v as i128)).collect();
    seed.push(("DEBUG".to_string(), if debug { 1 } else { 0 }));

    let path = aeon.join(rel);
    let src = std::fs::read_to_string(&path)
        .map_err(|e| format!("harvest_game_constants: read {}: {e}", path.display()))?;
    let (file, pdiags) = sigil_frontend_emp::parse_str(&src);
    if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("harvest_game_constants: parse: {:?}", pdiags.first()));
    }
    let (vals, diags) = sigil_frontend_emp::eval::eval_all_pub_consts(&file, Some(aeon), &seed);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("harvest_game_constants: resolve: {:?}", diags.first()));
    }
    Ok(vals)
}

/// The `.emp` struct twins whose field offsets + total size the residual AS
/// consumes at comptime (`ram.asm`'s `ds` slot sizes, residual-AS `setVDPReg`
/// expansions over the VdpShadow offsets, `act_descriptor.asm`'s `Sec`/`Act`
/// field access). Each entry is `(emp file, struct name, AS symbol
/// prefix)`: the harvest emits `<prefix>_<field>` = offsetof and `<prefix>_len` =
/// sizeof, exactly the equs `engine/structs.asm`'s `struct … endstruct` generated.
/// The prefix is VERBATIM the AS spelling — `Sst` (the `.emp` type) carried the
/// `SST_*` AS equs, so its prefix is `SST`.
const STRUCT_OFFSET_TWINS: &[(&str, &str, &str)] = &[
    ("engine/structs.emp", "Act", "Act"),
    ("engine/structs.emp", "Sec", "Sec"),
    ("engine/structs.emp", "DMAEntry", "DMAEntry"),
    ("engine/structs.emp", "parallax_config", "parallax_config"),
    ("engine/structs.emp", "VdpShadow", "VDP_Shadow"),
    ("engine/objects/sst.emp", "Sst", "SST"),
    ("engine/objects/entity_window.emp", "EntityScanState", "EntityScanState"),
    ("engine/level/parallax.emp", "band_entry", "band_entry"),
];

/// Harvest the `.emp` struct twins' field offsets + total sizes and shape them as
/// AS `<Struct>_<field>` / `<Struct>_len` equs — the struct-offset sibling of
/// [`harvest_engine_constants`]. With `engine/structs.asm` deleted, the `.emp`
/// structs are the SOLE author of the object/section/DMA/parallax/VDP-shadow
/// layouts; this reads their resolved offsets so the residual AS reads them as
/// GUARDED defines, the same harvest→inject ordering the constants flip uses.
///
/// `engine/system/types.emp`'s newtypes are supplied as the shared ambient TYPE
/// ENVIRONMENT: `Sst`'s fields are `Coord`/`Velocity`/… (erasing to raw widths);
/// the other seven structs are all-primitive and ignore it. The lone DERIVED
/// equate `structs.asm` carried outside a `struct` block — `SST_interact`
/// (`SST_sst_custom + SST_CUSTOM_SIZE - 2` = the object record's tail word) — is
/// emitted as `sizeof(Sst) - 2`, matching `collision.emp` / `player_sensors.emp`'s
/// `interact_off()`.
pub fn harvest_engine_struct_offsets(aeon: &Path) -> Result<Vec<(String, i64)>, String> {
    use sigil_frontend_emp::layout::layout_struct_ambient;

    let types_src = std::fs::read_to_string(aeon.join("engine/system/types.emp"))
        .map_err(|e| format!("harvest_engine_struct_offsets: read types.emp: {e}"))?;
    let (types_file, tdiags) = sigil_frontend_emp::parse_str(&types_src);
    if tdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("harvest_engine_struct_offsets: types.emp parse: {:?}", tdiags.first()));
    }

    let mut out: Vec<(String, i64)> = Vec::new();
    for (rel, sname, prefix) in STRUCT_OFFSET_TWINS {
        let src = std::fs::read_to_string(aeon.join(rel))
            .map_err(|e| format!("harvest_engine_struct_offsets: read {rel}: {e}"))?;
        let (file, pdiags) = sigil_frontend_emp::parse_str(&src);
        if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
            return Err(format!("harvest_engine_struct_offsets: {rel} parse: {:?}", pdiags.first()));
        }
        let (layout, diags) = layout_struct_ambient(&file, &types_file.items, sname);
        if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
            return Err(format!("harvest_engine_struct_offsets: layout {sname}: {:?}", diags.first()));
        }
        let layout = layout
            .ok_or_else(|| format!("harvest_engine_struct_offsets: no struct `{sname}` in {rel}"))?;
        for f in &layout.fields {
            out.push((format!("{prefix}_{}", f.name), f.offset as i64));
        }
        out.push((format!("{prefix}_len"), layout.size as i64));
        // SST_interact: the engine-owned player-slot tail word (structs.asm's one
        // derived `=` outside a struct block). = sizeof(Sst) - 2.
        if *sname == "Sst" {
            out.push(("SST_interact".to_string(), layout.size as i64 - 2));
        }
    }
    Ok(out)
}

/// Item #7b/#7c (Option B bridge, spec §9): engine AND game RAM are authored in
/// `engine/ram.emp` + `games/<game>/config/ram.emp` now, and their `pub vars`
/// labels are the SOLE link authority. But residual-AS seams reference those
/// labels EAGERLY — positions the AS frontend cannot defer to the linker:
///   1. Residual-AS `move.x #imm, (RamLabel).w` absolute-EA writes + `setVDPReg`
///      (`move.b`/`ori.l` to an abs dest) referencing engine RAM (Camera_X,
///      Palette_Dirty, Game_State, VDP_Shadow_Table, …). (The demo's demo_state
///      was the canonical example; it is native `.emp` now — Parcel H-demo — so
///      the surviving eager consumers are the engine-side AS residual + sonic4.)
///   2. `games/sonic4/config/game.asm`'s `move.b #1,(Dbg_Music_On).w` — GAME RAM.
/// (The `phase Engine_RAM_End` in the old game `config/ram.asm` is gone — game RAM
/// is now the `.emp` region `game_ram @ after(upper_ram)`, so it no longer needs
/// harvesting for its own base.) So harvest every engine+game RAM label's ADDRESS
/// (item #7c: `profile.game_ram_module` reaches the game region too) and seed them
/// as PLAIN value defines (NOT `guarded_defines` → NOT re-exported as EquSyms), so
/// the AS side folds them at comptime with no duplicate-symbol collision against
/// the `.emp` `pub` labels.
///
/// ONE layout authority (spec §9 requirement): this lowers `ram.emp` through the
/// SAME `lower_regions` path the real build uses — a focused `build_program` over
/// a `use engine.ram`-only entry — then reads the resolved section labels. The
/// harvest-time and lower-time addresses are the same BY CONSTRUCTION (one code
/// path, one comptime env = the profile's `emp_defines`), never merely tested.
/// Shape-specific: `emp_defines` carries `DEBUG` (0/1), so the debug prof block's
/// shift flows through to every downstream label's harvested address.
pub fn harvest_engine_ram_addresses(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<Vec<(String, i64)>, String> {
    let (mut manifest, mdiags) = resolve::manifest::Manifest::scan(aeon);
    let merr: Vec<_> = mdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !merr.is_empty() {
        return Err(format!(
            "ram harvest manifest scan: {} error(s); first: {:?}",
            merr.len(),
            merr.first()
        ));
    }
    // `ram.emp`'s comptime deps (the constants + the struct twins whose `sizeof`
    // it reads + the type vocabulary Sst's fields erase through) must expose their
    // comptime items to the focused lower.
    const RAM_HELPERS: &[&str] = &[
        "engine.types",
        "engine.coords",
        "engine.constants",
        "engine.structs",
        "engine.objects.sst",
    ];
    publicize_helper_comptime(&mut manifest, RAM_HELPERS);

    // Synthetic entry: `use engine.ram` + the game's RAM module (item #7c) — the
    // focused reachable set. The game RAM chains `game_ram @ after(upper_ram)` onto
    // the engine RAM, so both are lowered through the SAME whole-program region
    // resolution the real build uses; the harvest then reads BOTH regions' labels
    // (so eager AS reads of engine RAM — demo_state writes — AND game RAM —
    // game.asm's `move.b #1,(Dbg_Music_On).w` — fold at comptime).
    let entry_id = "__ram_harvest_entry__".to_string();
    let src = format!(
        "module __ram_harvest_entry__\n\nuse engine.ram\nuse {}\n",
        profile.game_ram_module
    );
    let source = sigil_span::SourceId(manifest.modules.len() as u32);
    let (file, pdiags) = sigil_frontend_emp::parse_file(&src, source);
    let perr: Vec<_> = pdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !perr.is_empty() {
        return Err(format!("ram harvest entry parse: {perr:?}"));
    }
    let idx = manifest.modules.len();
    manifest.by_id.insert(entry_id.clone(), idx);
    manifest.sources.insert(source, aeon.join("__ram_harvest_entry__.emp"));
    manifest.modules.push(resolve::manifest::ParsedModule {
        id: entry_id.clone(),
        file,
        path: aeon.join("__ram_harvest_entry__.emp"),
    });

    let defines: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.to_path_buf()),
        embed_base: Some(aeon.to_path_buf()),
        defines,
    };
    let aeon_root = aeon.to_path_buf();
    let embed_base_for = move |_id: &str| -> Option<std::path::PathBuf> { Some(aeon_root.clone()) };
    let (sections, _asserts, bdiags) =
        resolve::build_program_open_embed(&manifest, &entry_id, None, &opts, &embed_base_for);
    let berr: Vec<_> = bdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !berr.is_empty() {
        return Err(format!(
            "ram harvest build_program: {} error(s); first: {:?}",
            berr.len(),
            berr.first()
        ));
    }

    // Collect every RAM label (`vma_origin >= $F00000`) + every alias equate as a
    // plain (name, address) value define.
    let mut out: Vec<(String, i64)> = Vec::new();
    for sec in &sections {
        if sec.vma_origin() < 0x00F0_0000 {
            continue;
        }
        for l in &sec.labels {
            out.push((l.name.clone(), (sec.vma_origin().wrapping_add(l.offset)) as i64));
        }
        for e in &sec.equ_syms {
            if let sigil_ir::expr::Expr::Int(v) = e.expr {
                out.push((e.name.clone(), v));
            }
        }
    }
    if out.is_empty() {
        return Err("ram harvest: no RAM labels found (ram.emp not reachable?)".to_string());
    }
    Ok(out)
}

pub fn assemble_as_side(aeon: &Path, profile: &GameProfile) -> Result<Module, String> {
    let root = profile.game_root(aeon);
    // The `.emp`→residual-AS export (Stage-3 P5): harvest the `.emp`-owned engine
    // constants FIRST, then seed them as GUARDED defines so the residual AS reads
    // them at comptime. `.emp` definitions flow into the AS assembly — the harvest
    // must precede the assemble (the ordering the flip makes real).
    let mut guarded_defines = harvest_engine_constants(aeon)?;
    // The struct-offset sibling flip: the `.emp` struct twins are the sole author
    // of the object/section/DMA/parallax/VDP-shadow layouts (structs.asm deleted),
    // so their field offsets + sizes inject the same way the constants do.
    guarded_defines.extend(harvest_engine_struct_offsets(aeon)?);
    // Conversion Parcel F: the game's `.emp` constants module (row 21) is the sole
    // authority for the game constants; harvest it the same way (guarded defines +
    // link EquSyms), so the residual AS reads them and the game-agnostic engine
    // `.emp`'s `ensure(extern("X") == X)` drift guards resolve against this
    // authority. `None` for AS-authored game config (demo, Parcel H).
    if let Some(rel) = profile.game_constants_rel {
        guarded_defines.extend(harvest_game_constants(aeon, rel, profile.debug)?);
    }
    // Parcel F2: the game's `.emp` sound-id module (song / SFX ids + priority
    // ladder + SFXID_REV_LOOP) harvested the same way. Its `SONG_COUNT` is
    // shape-dependent, so the harvest seeds `DEBUG` from `profile.debug`.
    if let Some(rel) = profile.game_sound_ids_rel {
        guarded_defines.extend(harvest_game_constants(aeon, rel, profile.debug)?);
    }
    // Parcel F2: the game's SFX-bank id counts (SFX_ID_BASE/SFX_COUNT/SFX_TABLE_LEN),
    // DERIVED in sfx_bank.emp from the SfxTable rows, harvested so the residual AS
    // soundBankHead reads SFX_TABLE_LEN. `eval_all_pub_consts` resolves the SfxTable
    // metadata standalone (shape-invariant), so the seed shape is immaterial.
    if let Some(rel) = profile.game_sfx_bank_rel {
        guarded_defines.extend(harvest_game_constants(aeon, rel, profile.debug)?);
    }
    // Item #7b (Option B bridge, spec §9): seed engine RAM label ADDRESSES as
    // PLAIN value defines — the AS side folds its eager absolute-EA operands +
    // `phase Engine_RAM_End`; the `.emp` `pub vars` labels stay the sole link
    // authority (plain defines never export EquSyms, so no duplicate symbol).
    let mut defines: Vec<(String, i64)> = harvest_engine_ram_addresses(aeon, profile)?;
    if profile.sound_on {
        defines.push(("SOUND_DRIVER_ENABLED".to_string(), 1));
        for g in ["SIGIL_EMP_DAC", "SIGIL_EMP_MT", "SIGIL_EMP_SFX"] {
            defines.push((g.to_string(), 1));
        }
    }
    for g in &profile.code_gates {
        defines.push((g.to_string(), 1));
    }
    for (k, v) in &profile.extra_as_defines {
        defines.push((k.to_string(), *v));
    }
    if profile.debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = AsOptions {
        initial_cpu: Cpu::M68000,
        defines,
        include_root: Some(aeon.to_path_buf()),
        guarded_defines,
    };
    // A CHAINED build (`SizeSource::Frozen`) moves sections after assembly, so its
    // residual AS must keep section-label references SYMBOLIC to relocate (the row-94
    // parallax pointer); a PinnedBaked build never moves and stays byte-for-byte asl.
    let assemble = |root: &Path, opts: &AsOptions| match profile.size_source {
        SizeSource::Frozen(_) => assemble_root_relocating(root, opts),
        SizeSource::PinnedBaked => assemble_root(root, opts),
    };
    assemble(&root, &opts).map_err(|d| {
        format!(
            "assemble (native AS side, {}): {} diagnostics; first: {:?}",
            profile.name,
            d.len(),
            d.first()
        )
    })
}

/// Back-compat wrapper: the canonical sonic4 AS side at the STAGE-1 PINNED shape
/// (baked lmas intact, non-relocating). The pinned-era proofs (native_chained_resume,
/// the bootstrap derivation) consume this; the SHIPPED canonical build is
/// `build_rom_chained_with_listing(sonic4_profile(..))` (Frozen, packed).
pub fn assemble_native_all_gates_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    assemble_as_side(aeon, &sonic4_pinned_profile(debug))
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
        "SIGIL_EMP_SOUND_API", "SIGIL_EMP_ERROR_HANDLER", "SIGIL_EMP_EPILOGUE",
        "SIGIL_EMP_PLAYER_SENSORS",
        "SIGIL_EMP_PLAYER_GROUND", "SIGIL_EMP_PLAYER_AIR", "SIGIL_EMP_PLAYER_SPINDASH",
        "SIGIL_EMP_SONIC", "SIGIL_EMP_TEST_STATIC", "SIGIL_EMP_TEST_ANIMATED",
        "SIGIL_EMP_TEST_OBJECTS", "SIGIL_EMP_TEST_EMITTER", "SIGIL_EMP_TEST_PARENT",
        "SIGIL_EMP_TEST_STRESS_EMITTER", "SIGIL_EMP_TEST_CHURN", "SIGIL_EMP_PATH_SWAP",
        "SIGIL_EMP_OBJDEFS", "SIGIL_EMP_PARALLAX_CONFIGS", "SIGIL_EMP_TEST_MAPPINGS",
        "SIGIL_EMP_SONIC_ANIMS", "SIGIL_EMP_PARTICLE_ANIMS",
        "SIGIL_EMP_OBJECT_TEST_STATE", "SIGIL_EMP_OJZ_SCROLL_TEST",
        // Stage-3 keystone flip (kill row 93): the last AS-owned code twins gate
        // off (bodies → `.emp`, always-emitted headers stay AS-side).
        "SIGIL_EMP_PLAYER_COMMON", "SIGIL_EMP_TEST_PLAYER", "SIGIL_EMP_TEST_ENEMY",
        "SIGIL_EMP_ACT_DESCRIPTOR",
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

/// The FROZEN-source emp placement map: one nominal region per DISTINCT section name
/// present in the lowered set. Bases are COSMETIC (base 0, huge size) — the frozen
/// chainer overrides every base from the listing table, so `place_sections` here only
/// needs to give each section a home so it survives to the chainer. `place_sections`
/// does not enforce the size, so the huge size never overflows.
fn emp_map_frozen(sections: &[Section]) -> String {
    let mut names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let mut out = String::from("fill = 0x00\n");
    for n in names {
        out.push_str(&format!(
            "\n[[region]]\nname = \"{n}\"\nlma_base = 0x0\nsize = 0x400000\nkind = \"rom\"\n"
        ));
    }
    out
}

/// A synthetic entry `.emp` source whose `use` edges reach every registry module,
/// so `build_program`'s reachability BFS pulls them all (and their comptime deps).
fn synthetic_entry_src(specs: &[ModuleSpec], game_ram_module: &str) -> String {
    let mut src = String::from("module native_flip_entry\n\n");
    // Item #7b: the engine RAM module owns no ROM section (its region-form `vars`
    // lower to reserve-only RAM sections, skipped by `place_sections`), so it has
    // no registry `ModuleSpec` — but it MUST be reachable so its `pub vars` labels
    // are built and exported as the joint-link RAM authority. Both sonic4 + demo
    // need engine RAM, so it rides the shared entry.
    src.push_str("use engine.ram\n");
    // Item #7c: the game RAM module likewise owns no ROM section — its region-form
    // `vars` block chains `game_ram @ after(upper_ram)` onto the engine RAM and
    // lowers to a reserve-only RAM section. It must be reachable so its `pub vars`
    // labels (Player_Phys, the rings, Game_RAM_End, the debug counters) export as
    // the joint-link authority the game `.emp` consumers resolve against.
    src.push_str(&format!("use {game_ram_module}\n"));
    for s in specs {
        src.push_str(&format!("use {}\n", s.module_id));
    }
    src
}

/// Back-compat wrapper: build the canonical sonic4 emp set (the Stage-1 shape).
pub fn build_native_emp(
    aeon: &Path,
    debug: bool,
) -> Result<(Vec<Section>, Vec<sigil_ir::LinkAssert>), String> {
    // The Stage-1 PINNED shape (see assemble_native_all_gates_as_side).
    build_emp(aeon, &sonic4_pinned_profile(debug))
}

/// Natively lower + place every registry `.emp` module for `profile`. Returns the
/// placed sections + the whole program's deferred link asserts (drift guards). Under
/// `SizeSource::Frozen` the placement map bases are COSMETIC (the chainer recomputes
/// every base from the frozen table), so a nominal one-region-per-section map suffices.
pub fn build_emp(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<(Vec<Section>, Vec<sigil_ir::LinkAssert>), String> {
    let debug = profile.debug;
    let specs = &profile.registry;

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
        "engine.level.parallax_dsl",
    ];
    publicize_helper_comptime(&mut manifest, COMPTIME_HELPERS);
    normalize_helper_imports(&mut manifest, COMPTIME_HELPERS, &[]);

    // Inject the synthetic entry as a fresh module in the manifest.
    let entry_id = "native_flip_entry".to_string();
    let src = synthetic_entry_src(specs, profile.game_ram_module);
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
    let defines: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(k, v)| (k.to_string(), *v)).collect();
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

    // Guard: exactly ONE non-empty `"text"` section (OBJDEFS). A second would mean
    // an unexpected defaulted-module data producer slipped into the reachable set
    // and would corrupt OBJDEFS's region — a STOP, not a silent pack. Demo has NO
    // objdefs (0 non-empty `text`), so the guard is profile-gated.
    if profile.require_one_text {
        let nonempty_text =
            sections.iter().filter(|s| s.name == "text" && !s.image_bytes().is_empty()).count();
        if nonempty_text != 1 {
            return Err(format!(
                "expected exactly 1 non-empty `text` section (OBJDEFS), found {nonempty_text}"
            ));
        }
    }

    let map_toml = match &profile.size_source {
        SizeSource::PinnedBaked => emp_map_toml(specs, debug),
        // The chainer recomputes every base from the frozen table, so the emp
        // placement map only needs one (cosmetic) region per DISTINCT section name.
        SizeSource::Frozen(_) => emp_map_frozen(&sections),
    };
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

/// THE Stage-1 INAPPLICABLE-drift-guard ALLOWLIST (t24 both-directions discipline).
///
/// Each entry is a `.emp` `ensure(extern("X") == mirror)` drift guard whose constant
/// `X` is homed ONLY in a `.asm` twin that the all-gates build gates off, so it
/// cannot resolve — it folds to Poison ("references symbol `X` not defined in this
/// link"), NOT a `Value(0)` drift failure, and is INAPPLICABLE (the whole-ROM byte
/// gate is the authoritative drift oracle for every USED constant). Pinned by
/// `(extern name, a distinguishing substring of the guard's own message = its `.emp`
/// site)`. The set of Poison guards MUST equal this list EXACTLY:
///   - a Poison guard NOT here → HARD FAIL (a typo'd/renamed extern, or a new
///     twin-parity guard that needs an explicit ruling), and
///   - an allowlisted guard that no longer folds to Poison (it now resolves, or
///     drifted to a `Value(0)` fail) → HARD FAIL (the allowlist is STALE).
///
/// **EMPTY post the Stage-3 P5 bg/camera flip (row 93 data-half).** The four
/// Poison guards retired: `VRAM_PLANE_B_BYTES` (plane_buffer/section) and
/// `CAM_SCREEN_HALF_{W,H}` (ojz_scroll_test) moved into `engine.constants` as the
/// sole author (harvested + injected + link-exported like every flipped
/// constant), and their consumers now `use engine.constants.{…}` instead of
/// mirroring + guarding a gated-off `.asm` twin. With the list empty the
/// enforcement asserts a STRONGER invariant: NO drift guard folds to an
/// unresolvable-extern Poison — every guard resolves. (The field + this machinery
/// can be deleted outright as trivial follow-up; kept here as the no-Poison
/// invariant.)
const STAGE1_INAPPLICABLE_GUARDS: &[(&str, &str)] = &[];

/// DEMO inapplicable-drift-guard allowlist — EMPTY for the same reason (the two
/// `VRAM_PLANE_B_BYTES` guards it carried retired with the bg/camera flip).
const DEMO_INAPPLICABLE_GUARDS: &[(&str, &str)] = &[];

/// Enforce that the observed inapplicable (Poison-unresolvable) drift guards are
/// EXACTLY [`STAGE1_INAPPLICABLE_GUARDS`] — both directions (§t24). `inapplicable`
/// are the "not defined in this link" diagnostics; `link_asserts` supplies each
/// guard's own message (its `.emp` site) via span match.
fn enforce_inapplicable_allowlist(
    inapplicable: &[&sigil_span::Diagnostic],
    link_asserts: &[sigil_ir::LinkAssert],
) -> Result<(), String> {
    enforce_inapplicable_allowlist_against(inapplicable, link_asserts, STAGE1_INAPPLICABLE_GUARDS)
}

/// As [`enforce_inapplicable_allowlist`] but against a caller-supplied allowlist (the
/// per-profile set — demo/Config each home a different twin subset). Both directions.
fn enforce_inapplicable_allowlist_against(
    inapplicable: &[&sigil_span::Diagnostic],
    link_asserts: &[sigil_ir::LinkAssert],
    allowlist: &[(&str, &str)],
) -> Result<(), String> {
    // The guard's own message text (its Text parts), for the site match.
    let site_of = |d: &sigil_span::Diagnostic| -> String {
        link_asserts
            .iter()
            .find(|a| a.span == d.primary)
            .map(|a| {
                a.message
                    .iter()
                    .filter_map(|p| match p {
                        sigil_ir::MsgPart::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<String>()
            })
            .unwrap_or_default()
    };

    let mut matched = vec![false; allowlist.len()];
    for d in inapplicable {
        // The unresolved extern name(s) are the backtick-delimited tokens in the
        // Poison message ("references symbol(s) `X` not defined in this link").
        let externs: Vec<&str> = d.message.split('`').skip(1).step_by(2).collect();
        let site = site_of(d);
        let hit = allowlist.iter().enumerate().find(|(i, (name, token))| {
            !matched[*i] && externs.contains(name) && site.contains(token)
        });
        match hit {
            Some((i, _)) => matched[i] = true,
            None => {
                return Err(format!(
                    "unresolvable-extern drift guard NOT in the allowlist — either the \
                     extern name is wrong or a new twin-parity guard needs a ruling.\n  \
                     externs: {externs:?}\n  site: {site}"
                ));
            }
        }
    }
    if let Some(i) = matched.iter().position(|m| !m) {
        return Err(format!(
            "inapplicable-guard allowlist is STALE: the guard {:?} \
             (extern, site) no longer folds to an unresolvable-extern Poison — it now resolves \
             (or drifted to a Value(0) fail). Re-verify and update the allowlist.",
            allowlist[i],
        ));
    }
    if std::env::var("NATIVE_DEBUG").is_ok() {
        eprintln!(
            "NATIVE_DEBUG: {} inapplicable drift guard(s) matched the allowlist exactly",
            inapplicable.len()
        );
    }
    Ok(())
}

// ── Flip Stage 2 · S1.2 — THE DECLARED-ORDER, COMPUTED-BASE CHAINER ──
//
// The generalization of `native_chained_resume`'s proof into a first-class placement
// authority. It computes EVERY section's ROM base by chaining in a DECLARED ORDER;
// the baked `org`/resume-org literals are discarded for chained sections (proving
// them redundant — the Stage-2 unblock). Only genuine fixed anchors (the object bank
// `org $10000`, the phased sound banks) keep a declared base.
//
// THE LOAD-BEARING REFINEMENT (empirically established — see the S1.2 note): pure
// content-size chaining does NOT reproduce asl. sigil's link-time branch relaxation
// grows short→long; when sections pack tighter than asl's layout, a branch sitting at
// the `.s`/`.w` boundary relaxes SHORTER, so the chained ROM settles at a *different*
// (tighter) relaxation fixpoint than asl — a byte divergence that cascades (first seen
// as a −2 at `hblank`, from `vblank.emp`'s bare branch). The fix: reserve each
// section's DECLARED SIZE = its exact asl per-region span, so the computed-base chain
// reproduces asl's addresses and thereby anchors relaxation to asl's widths. The
// declared sizes are what the map manifest must hold (SPEC2:199), sourced per target
// from its asl listing (sonic4: the pinned-resolve spans below; demo/Config: their
// `.lst` spans). Bases stay COMPUTED — the soundness condition is met.

/// Per-section image length (exact, relaxables lowered to `Data`) from a resolve where
/// each ROM section is pinned at `pin_lma[idx]` (falling back to its baked lma). Keyed
/// by the section's stable index. The unique-name tag makes the read unambiguous (the
/// tree carries same-named `text`/`sec<lma>` sections); names never affect bytes.
fn image_lens_pinned(sections: &[Section], pin_lma: &[Option<u32>]) -> Result<Vec<u32>, String> {
    let mut tagged: Vec<Section> = sections.to_vec();
    // ROM sections with NO pin (label-less data blobs whose true base is not yet known)
    // are pinned at DISJOINT high scratch slots: they are pure DATA, so image_len is
    // position-independent, and scratch pins keep the resolve overlap-free (a frozen
    // labeled section and a baked label-less one can otherwise collide — the config_a
    // sound-tail vs object_test_state case).
    let mut scratch: u32 = 0x0070_0000;
    for (i, s) in tagged.iter_mut().enumerate() {
        s.name = format!("{}\u{0}{i}", s.name);
        if is_rom_section(s) {
            // Force Pinned so `resolve_layout` honours the lma we set (a Chained section
            // would otherwise ignore it and pack within its group, defeating the pin).
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            match pin_lma.get(i) {
                Some(Some(p)) => s.lma = *p,
                _ => {
                    s.lma = scratch;
                    // keep vma tracking the scratch lma so labels don't leak a stale base
                    if s.vma_base.map(|v| v < 0x8000).unwrap_or(false) {
                        s.vma_base = None;
                    }
                    scratch += 0x10_0000;
                }
            }
        }
    }
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&tagged, &stubs, true).map_err(|d| {
        format!("span pass: resolve_layout: {} diag(s); first {:?}", d.len(), d.first())
    })?;
    let mut img = vec![0u32; sections.len()];
    for s in &resolved {
        if let Some(idx) = s.name.rsplit('\u{0}').next().and_then(|t| t.parse::<usize>().ok()) {
            if is_rom_section(s) {
                img[idx] = s.image_len();
            }
        }
    }
    Ok(img)
}

/// The TRUE per-section ROM base (the declared-order authority), keyed by stable index;
/// `None` for RAM/phase-only sections. For `PinnedBaked` (canonical sonic4) the baked
/// lma IS asl-correct so the true base is the baked lma. For `Frozen` (demo/Config) the
/// baked resume orgs are WRONG sonic4 values, so each section's true base is
/// `frozen[L] − offset[L]` for a contained frozen label `L`; a label-less DATA blob
/// derives by CONTIGUITY from its frozen neighbour (the phased Z80 idle, HeightMaps),
/// and a hard-org PHASE BANK (vma ≥ 0x8000 — the sound banks) keeps its baked (=asl) org.
fn true_bases_by_index(
    sections: &[Section],
    src: &SizeSource,
) -> Result<Vec<Option<u32>>, String> {
    let n = sections.len();
    match src {
        SizeSource::PinnedBaked => Ok(sections
            .iter()
            .map(|s| if is_rom_section(s) { Some(s.lma) } else { None })
            .collect()),
        SizeSource::Frozen(table) => {
            // Provisional base + labeled flag per ROM section.
            let mut prov = vec![None; n];
            let mut labeled = vec![false; n];
            for (i, s) in sections.iter().enumerate() {
                if !is_rom_section(s) {
                    continue;
                }
                let mut base: Option<i64> = None;
                for l in &s.labels {
                    if let Some(&a) = table.get(&l.name) {
                        let b = a as i64 - l.offset as i64;
                        base = Some(base.map_or(b, |x: i64| x.min(b)));
                    }
                }
                match base {
                    Some(b) => {
                        prov[i] = Some(b);
                        labeled[i] = true;
                    }
                    None => prov[i] = Some(s.lma as i64), // baked fallback (order only)
                }
            }
            packed_true_bases(sections, &prov, &labeled)
        }
    }
}

/// The §17 Wave-B B-0 packing walk (the rows-6/58 partial realization): provisional
/// bases give ORDER and the org-island ANCHORS; every other ROM section's base is
/// PACKED from live-measured image lengths, so a size-changing parcel shifts its
/// contiguous run downstream instead of colliding with stale pins. Rules per section
/// (walked in provisional order):
///   - ISLAND (prov > running + ANCHOR_GAP, or the run head): absolute at prov; a
///     packed run that overflows past an island's prov base fails loud.
///   - PHASE BANK (vma ≥ 0x8000, vma ≠ lma) head, and label-less blobs inside its
///     hard-org run: absolute at prov (the sound banks never pack).
///   - label-less non-phase blob: contiguity from its neighbour (the Frozen
///     boot-region Z80-idle rule, unchanged).
///   - everything else: `align_up(running, A)` with A = the largest power of two
///     ≤ 16 dividing prov — at unchanged sizes this reproduces prov exactly (the
///     fold-identity the six golden gates prove), and under growth it re-derives the
///     alignment pad the provisional layout implies.
/// Image lengths are relaxation-dependent (branch widths move with distance), so the
/// walk iterates measure → pack to a fixpoint (≤ 8 rounds; round 0 measures at
/// disjoint scratch pins). Island classification must be IDENTICAL across rounds —
/// a growth big enough to eat an org hole is a hand-ruling, not a silent repack.
fn packed_true_bases(
    sections: &[Section],
    prov: &[Option<i64>],
    labeled: &[bool],
) -> Result<Vec<Option<u32>>, String> {
    let n = sections.len();
    let mut order: Vec<usize> = (0..n).filter(|&i| prov[i].is_some()).collect();
    order.sort_by_key(|&i| prov[i].unwrap());
    const ANCHOR_GAP: i64 = 0x400;
    let align_of = |p: i64| -> i64 {
        for a in [16i64, 8, 4, 2] {
            if p % a == 0 {
                return a;
            }
        }
        1
    };

    // Round 0: lengths at the PROVISIONAL bases (labeled sections at prov, label-less
    // pure-data blobs at scratch — the proven Frozen measuring pins). When a section
    // GREW, prov pins collide and the resolve fails — retry with a small cumulative
    // spread (+0x80 per ROM section, order-preserving): big enough to absorb
    // parcel-scale growth, small enough that cross-section CONDITIONAL branches (no
    // long form) keep their reach. This spread is a MEASURING device only — final
    // bases come from the island/contiguity rounds below and re-measure to a
    // fixpoint, so widening it never moves an unchanged section. Widened 0x40->0x80
    // for collision_lookup #1 (the fused GetType+GetCollision grows the region by
    // 0x44, just past the old 0x40 spread — the demo game's tight layout overran it
    // by 4 bytes). A growth beyond this is a hand ruling.
    let prov_pins: Vec<Option<u32>> = (0..n)
        .map(|i| if labeled[i] { prov[i].map(|v| v as u32) } else { None })
        .collect();
    let mut img = match image_lens_pinned(sections, &prov_pins) {
        Ok(v) => v,
        Err(_collision) => {
            let mut spread_pins = prov_pins.clone();
            for (rank, &i) in order.iter().enumerate() {
                if let Some(Some(p)) = spread_pins.get_mut(i).map(|s| s.as_mut()) {
                    *p += 0x80 * rank as u32;
                }
            }
            image_lens_pinned(sections, &spread_pins)
                .map_err(|e| format!("span pass (spread round, post-growth): {e}"))?
        }
    };
    let mut prev_islands: Option<Vec<bool>> = None;
    for _round in 0..8 {
        let mut out: Vec<Option<u32>> = vec![None; n];
        let mut islands = vec![false; n];
        let mut running: Option<i64> = None;
        let mut in_phase_run = false;
        for &i in &order {
            let p = prov[i].unwrap();
            let is_phase_bank =
                sections[i].vma_base.map(|v| v != sections[i].lma && v >= 0x8000).unwrap_or(false);
            let tb = if is_phase_bank {
                // A PHASE BANK head is a HARD org even when labeled — the Z80 side
                // holds pointers into the bank, so bank content NEVER packs. Without
                // this precedence the labeled branch repacks the bank the moment the
                // pre-bank blob's untrimmed align-pad image creeps past the org (the
                // mt-gate catch: entity_window growth crossing the $58000 threshold).
                in_phase_run = true;
                islands[i] = true;
                p
            } else if labeled[i] {
                in_phase_run = false;
                match running {
                    None => {
                        islands[i] = true;
                        p
                    }
                    Some(r) if p > r + ANCHOR_GAP => {
                        islands[i] = true;
                        p
                    }
                    Some(r) => {
                        let a = align_of(p);
                        let packed = (r + a - 1) / a * a;
                        if packed > p + ANCHOR_GAP {
                            return Err(format!(
                                "packed base {packed:#x} for section `{}` overruns its provisional {p:#x} by more than the island margin — a run grew past its org hole; hand ruling needed",
                                sections[i].name
                            ));
                        }
                        packed
                    }
                }
            } else if in_phase_run {
                p // hard-org phase-run tail: absolute
            } else {
                match running {
                    Some(r) if p > r + ANCHOR_GAP => {
                        islands[i] = true;
                        p
                    }
                    Some(r) => r, // contiguity from the neighbour
                    None => {
                        islands[i] = true;
                        p
                    }
                }
            };
            out[i] = Some(tb as u32);
            running = Some(tb + img[i] as i64);
        }
        if let Some(prev) = &prev_islands {
            if *prev != islands {
                return Err(
                    "island classification changed between packing rounds — growth ate an org hole; hand ruling needed"
                        .to_string(),
                );
            }
        }
        prev_islands = Some(islands);
        // Re-measure with the round-0 pin discipline: LABELED sections at their packed
        // bases (correct relaxation), label-less pure-data blobs at scratch (position-
        // independent, and the align-padded pre-bank blob would otherwise collide with
        // the pinned bank org — the config_a HeightMaps case).
        // Label-less pure-data blobs AND the phase banks measure at scratch (the
        // pre-bank blob's baked align-pad is sized for its ORIGINAL position, so at
        // a shifted packed base its image overshoots the hard bank org until
        // declared_spans clamps it — the phase_region_mask rule keeps the measuring
        // resolve overlap-free).
        let bankish: Vec<bool> = sections
            .iter()
            .map(|s| s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false))
            .collect();
        let remeasure: Vec<Option<u32>> = (0..n)
            .map(|i| if labeled[i] && !bankish[i] { out[i] } else { None })
            .collect();
        let img2 = image_lens_pinned(sections, &remeasure)
            .map_err(|e| format!("span pass (packed round): {e}"))?;
        if img2 == img {
            return Ok(out);
        }
        img = img2;
    }
    Err("packed_true_bases did not converge in 8 rounds (relaxation oscillation) — hand ruling needed".to_string())
}

/// The declared per-section SIZE (exact asl span the chainer reserves), keyed by stable
/// index. Each ROM section is pinned at its `true_base`; the resolved layout's per-run
/// advance (next.true_base − this.true_base within an abutting run; image_len before an
/// org hole) is its span. This is what anchors relaxation to asl's widths.
fn declared_spans_by_index(
    sections: &[Section],
    true_bases: &[Option<u32>],
) -> Result<Vec<Option<u32>>, String> {
    // A trailing `align $8000` on the pre-sound-bank data blob (HeightMaps) makes its
    // image OVERSHOOT the sound-bank anchor, so pinning both at their true bases collides
    // in the measuring resolve. Scratch the phase-region sections (the sound banks) for
    // the measurement — they are pure data (their own span comes from the frozen gap, not
    // this image) — so the resolve is overlap-free; the pre-bank blob is still measured at
    // its true base (its span then clamps to the gap-to-anchor above).
    let phase_region = phase_region_mask(sections, true_bases);
    let pin: Vec<Option<u32>> = (0..sections.len())
        .map(|i| if phase_region[i] { None } else { true_bases[i] })
        .collect();
    let img = image_lens_pinned(sections, &pin).map_err(|e| format!("span pass (declared): {e}"))?;
    let mut rom: Vec<(usize, u32, u32)> = (0..sections.len())
        .filter_map(|i| true_bases.get(i).and_then(|o| *o).map(|tb| (i, tb, img[i])))
        .collect();
    rom.sort_by_key(|&(_, tb, _)| tb);
    let mut span = vec![None; sections.len()];
    for i in 0..rom.len() {
        let (idx, tb, im) = rom[i];
        let adv = if i + 1 < rom.len() {
            let gap = rom[i + 1].1.saturating_sub(tb);
            // The declared size is the GAP to the next section when they abut (gap ≈
            // image), and also when the image OVERSHOOTS the next section — a trailing
            // `align $8000` (the sound bank) measures more padding at a scratch pin than
            // fits at the real base, so the next anchor clamps the reserved span. A gap
            // MUCH larger than the image is a genuine org hole: reserve only the image
            // (the anchor after it pins; the interval fills).
            if gap <= im + 0x10 { gap } else { im }
        } else {
            im
        };
        span[idx] = Some(if im == 0 { 0 } else { adv.max(1) });
    }
    Ok(span)
}

/// ALIGN-RECOMPUTE-ON-RELOCATION. A trailing `align $8000` (the pre-sound-bank data
/// blob HeightMaps) bakes its padding as a `Fill` sized for the section's AS-RESIDUAL
/// position. When the frozen chainer relocates the section, that baked pad is stale —
/// its image OVERSHOOTS the section's (correctly clamped) reserved span and collides
/// with the next hard-org anchor (`MovingTrucks_Bank_Start` at 0x58000). asl would have
/// recomputed the pad for the new position; the linker never shrank a baked `align`.
///
/// This recomputes it: for a PURE-DATA section (no width-variable fragment — the only
/// case whose image length is known pre-relaxation, and exactly the align-blob shape),
/// if the image exceeds `span`, trim the overshoot off the TRAILING zero-`Fill` (the
/// align padding). Only zero-Fill is trimmed and only down to `span`, so no real datum
/// is ever cut; an overshoot that is NOT trailing zero-Fill is left intact for
/// `resolve_layout` to reject loudly. Byte-neutral for the pinned (sonic4) path, where
/// `image_len == span` and the guard never fires.
fn trim_trailing_align_overshoot(s: &mut Section, span: u32) {
    let has_variable = s.fragments.iter().any(|f| {
        matches!(
            f,
            Fragment::JmpJsrSym { .. } | Fragment::RelaxAbsSym { .. } | Fragment::RelaxLadder { .. }
        )
    });
    if has_variable {
        return;
    }
    let img = s.image_len();
    if img <= span {
        return;
    }
    let mut overshoot = img - span;
    while overshoot > 0 {
        match s.fragments.last_mut() {
            Some(Fragment::Fill { value: 0, count, .. }) => {
                if *count > overshoot {
                    *count -= overshoot;
                    overshoot = 0;
                } else {
                    overshoot -= *count;
                    s.fragments.pop();
                }
            }
            // Not trailing zero-fill: leave the overshoot for resolve_layout to reject.
            _ => break,
        }
    }
}

/// The minimum alignment (bank size) an INTERNAL `align` must target for
/// `recompute_bank_aligns` to touch it. `align $8000` (the DAC/MT sound-bank pads)
/// is the only aeon align at or above this; `align 2` word-aligns and any small
/// bulk zero-fill fall below it and are left exactly as baked — so a relocated
/// section's HEAD (e.g. HeightMaps at its true base) never moves.
const BANK_ALIGN: u32 = 0x8000;

/// ALIGN-RECOMPUTE-ON-RELOCATION for INTERNAL bank aligns (the general form of
/// `trim_trailing_align_overshoot`, which handles only the trailing pad).
///
/// A pure-data section can carry `align $8000` pads MID-section (main.asm's
/// BINCLUDE arm: HeightMaps → art → `align $8000` → DAC blip → `align $8000` →
/// DAC shared → `align $8000` → MovingTrucks — ONE section). `directive_align`
/// bakes each as a fixed `Fill{0, pad}` computed against the section's AS-RESIDUAL
/// base. When the chainer re-pins the section at its true (frozen) base, that
/// fixed pad carries the following content off the absolute bank boundary by the
/// relocation delta (config_a's DAC banks landed +0xC at 0x4800c/0x5000c). asl
/// aligns to ABSOLUTE N-multiples independent of the base, so the pad must be
/// recomputed.
///
/// Replays the BAKED and TRUE absolute positions in parallel from `baked_base` /
/// `true_base`. For each zero-`Fill` that is a MINIMAL pad landing on a
/// `>= BANK_ALIGN` boundary (a bank align — never a word-align or bulk fill), the
/// pad is rewritten so the following content resumes on that SAME absolute
/// boundary (`new_pad = baked_after − true_pos`); the two cursors re-sync there.
/// Non-bank fragments advance both cursors by their length, preserving the delta.
/// Byte-neutral for the pinned path (`baked_base == true_base`, the loop is a
/// no-op) and for any section with no `>= BANK_ALIGN` internal align.
///
/// Pure-data only (guarded): a width-variable fragment makes image offsets
/// base-dependent in a way this static replay does not model — such a section is
/// left untouched (it has no bank align in practice).
fn recompute_bank_aligns(s: &mut Section, baked_base: u32, true_base: u32) {
    if baked_base == true_base {
        return;
    }
    // Only a section whose fragments advance the cursor monotonically by their own
    // length can be replayed statically here. A width-variable fragment (base-
    // dependent offsets) or an `Org` (cursor seek) is left untouched — neither
    // occurs in the sound-bank blob this targets.
    let simple = s.fragments.iter().all(|f| {
        matches!(f, Fragment::Data(_) | Fragment::Fill { .. } | Fragment::Reserve { .. })
    });
    if !simple {
        return;
    }
    let mut baked = baked_base;
    let mut tru = true_base;
    for f in &mut s.fragments {
        match f {
            Fragment::Fill { value: 0, count, .. } => {
                let c = *count;
                let baked_after = baked.wrapping_add(c);
                // A bank align = a minimal pad (`c < BANK_ALIGN`) whose post-position is a
                // >= BANK_ALIGN multiple. The boundary is `baked_after` itself (the absolute
                // bank start the residual align reached). Recompute so `tru` resumes there.
                let is_bank_align =
                    c < BANK_ALIGN && baked_after != 0 && baked_after % BANK_ALIGN == 0;
                if is_bank_align && baked_after >= tru {
                    *count = baked_after - tru;
                    tru = baked_after;
                    baked = baked_after;
                } else {
                    baked = baked_after;
                    tru = tru.wrapping_add(c);
                }
            }
            Fragment::Data(d) => {
                let l = d.bytes.len() as u32;
                baked = baked.wrapping_add(l);
                tru = tru.wrapping_add(l);
            }
            Fragment::Fill { count, .. } | Fragment::Reserve { count, .. } => {
                baked = baked.wrapping_add(*count);
                tru = tru.wrapping_add(*count);
            }
            _ => {}
        }
    }
}

/// Apply the declared-order computed-base chain to `sections`: split ROM/RAM, sort ROM
/// by ascending true base, and Pin each org anchor / Chain the rest reserving its
/// declared span. Returns the placed sections (ROM then RAM). Shared by the driver and
/// the layout diagnostic so they can never drift.
fn apply_declared_chain(
    sections: Vec<Section>,
    true_bases: &[Option<u32>],
    spans: &[Option<u32>],
) -> Vec<Section> {
    type IndexedSection = (usize, Section);
    let indexed: Vec<IndexedSection> = sections.into_iter().enumerate().collect();
    let (mut rom, ram): (Vec<IndexedSection>, Vec<IndexedSection>) =
        indexed.into_iter().partition(|(_, s)| is_rom_section(s));
    // DECLARED ORDER = ascending true base (baked for sonic4; frozen for demo/Config).
    rom.sort_by_key(|(idx, s)| (true_bases[*idx].unwrap_or(s.lma), s.placement_span()));

    let mut cur_group = String::new();
    let mut gi = 0usize;
    let mut chain_end: Option<u32> = None;
    for (idx, s) in rom.iter_mut() {
        let orig_lma = s.lma;
        let tb = true_bases[*idx].unwrap_or(orig_lma);
        let span = spans[*idx].unwrap_or_else(|| s.placement_span());
        // A GENUINELY-PHASED section runs at a VMA distinct from its ROM lma. Two kinds:
        //   - a HARD-ORG SOUND BANK (vma at a real phase ≥ 0x8000 — resident/moving-
        //     trucks): its labels use the phase VMA and its ROM org is absolute; ANCHOR.
        //   - an INLINE Z80 section (vma = 0 — the no-sound Z80 idle): its internal
        //     labels are Z80-relative (vma 0); the 68k reaches it via a neighbour label,
        //     so it CHAINS but KEEPS its vma=0.
        // A normal AS section carries vma == baked lma (NOT phased) — the check MUST
        // compare against the section's OWN baked lma, never the re-based `tb` (else a
        // baked object-bank address ≥ 0x8000 is misread as a phase and its stale sonic4
        // VMA leaks into every reference to its labels).
        let phase_bank = s.vma_base.map(|v| v != orig_lma && v >= 0x8000).unwrap_or(false);
        let keep_vma = phase_bank || s.vma_base == Some(0);
        let is_anchor =
            chain_end.is_none() || phase_bank || chain_end.map(|e| tb > e).unwrap_or(true);
        s.reserved_span = span;
        // Recompute stale INTERNAL bank-boundary aligns (the DAC/MT `align $8000`
        // pads inside a relocated pure-data blob) for the new base, then the
        // TRAILING align (see each fn's doc). A VMA-tracking (non-phase) section
        // only — a phase bank keeps its baked absolute image.
        if !keep_vma {
            recompute_bank_aligns(s, orig_lma, tb);
        }
        trim_trailing_align_overshoot(s, span);
        if !keep_vma {
            s.vma_base = None; // VMA tracks the (re-based) LMA — no stale baked address.
        }
        if is_anchor {
            gi += 1;
            cur_group = format!("declchain{gi}");
            s.group = Some(cur_group.clone());
            s.placement = SectionPlacement::Pinned;
            s.lma = tb;
            chain_end = Some(tb + span);
        } else {
            s.group = Some(cur_group.clone());
            s.placement = SectionPlacement::Chained;
            s.lma = 0; // base COMPUTED — the baked resume-org literal is discarded.
            chain_end = Some(chain_end.unwrap().max(tb) + span);
        }
    }

    let mut all: Vec<Section> = rom.into_iter().map(|(_, s)| s).collect();
    all.extend(ram.into_iter().map(|(_, s)| s));
    all
}

/// Mark the PHASE-REGION ROM sections (a phase bank — vma ≥ 0x8000 — and the label-less
/// data tail chaining after it, up to the next labeled section). These are the hard-org
/// sound banks whose image (with its own trailing align) would collide in the span-pass
/// resolve; their declared span comes from frozen gaps, not from a measured image, so
/// they are scratched during measurement.
fn phase_region_mask(sections: &[Section], true_bases: &[Option<u32>]) -> Vec<bool> {
    let n = sections.len();
    let mut order: Vec<usize> = (0..n).filter(|&i| true_bases[i].is_some()).collect();
    order.sort_by_key(|&i| true_bases[i].unwrap());
    let mut mask = vec![false; n];
    let mut in_phase = false;
    for &i in &order {
        let s = &sections[i];
        let is_phase_bank =
            s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false);
        if is_phase_bank {
            in_phase = true;
            mask[i] = true;
        } else if in_phase {
            // stay in the phase region only for label-less data tails; a real labeled
            // section (object_test_state) ends it.
            let has_own_label = !s.labels.is_empty();
            if has_own_label {
                in_phase = false;
            } else {
                mask[i] = true;
            }
        }
    }
    mask
}

/// True for an image-bearing ROM section (VMA below the RAM/phase floor), as opposed
/// to a RAM/phase section that never participates in ROM layout.
fn is_rom_section(s: &Section) -> bool {
    match s.vma_base {
        Some(v) => v < 0x00F0_0000,
        None => true,
    }
}

/// Build the whole native ROM with every base COMPUTED by declared-order chaining
/// (the S1.2 generalization). `debug` selects the shape. The DECLARED ORDER for the
/// canonical-sonic4 bootstrap is the address order (baked lmas known-correct — the
/// ratified sort-by-address bootstrap); for demo/Config it comes from their listing.
/// Genuine org anchors (object bank, phased sound banks) keep their declared base;
/// every other section is `Chained` with its baked lma zeroed (base computed).
pub fn build_native_rom_chained(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    build_rom_chained(aeon, &sonic4_profile(debug))
}

/// Build the whole native ROM for `profile` with every base COMPUTED by declared-order
/// chaining. The DECLARED ORDER + SIZES come from the profile's `SizeSource`: canonical
/// sonic4 from the baked (asl-correct) lmas; demo/Config from the frozen listing table.
/// Genuine org anchors (object bank, phase-address sound banks) keep their declared
/// base; every other section is `Chained` with its lma zeroed (base computed).
pub fn build_rom_chained(aeon: &Path, profile: &GameProfile) -> Result<Vec<u8>, String> {
    Ok(build_rom_chained_with_listing(aeon, profile)?.0)
}

/// Parcel-K1 · the map VALIDATION pass (R2). Checks the chainer's RESOLVED layout
/// against the game's declared placement map — fold-identical by construction (adds
/// checks, changes no placement), and any derivation change fails loud:
///   - `[map.undeclared-island]`: an ANCHOR_GAP-inferred island (a resolved ROM section
///     whose base opens a > `ANCHOR_GAP` gap past the running end, or a phase bank, or
///     the run head) whose base is not a declared `anchor` fails loud; and every
///     shape-applicable declared anchor must appear.
///   - order VALIDATION: the derived byte-emitting section order (min-offset label, image
///     bytes > 0) must be a SUBSEQUENCE of the declared `order` — a present id out of the
///     declared order, or a byte-emitting id absent from it, fails loud. Zero-byte markers
///     (e.g. `__BUDGET_DATA`) are excluded (byte-neutral, shape-varying tie position).
///   - HOLE (data; K2 enforces): a shape-applicable hole's `after` label must resolve.
/// The frozen tables stay the per-label provisional-base MEASUREMENT cache the order
/// derivation sorts; the map is the reviewed AUTHORITY the derivation must agree with.
pub fn validate_placement(
    resolved: &[Section],
    pmap: &crate::map_placement::PlacementMap,
    sound_on: bool,
) -> Result<(), String> {
    const ANCHOR_GAP: u32 = 0x400;
    // ROM sections, lma-sorted, with (stable_id, lma, byte_len, is_phase_bank).
    let mut rows: Vec<(String, u32, usize, bool)> = resolved
        .iter()
        .filter(|s| is_rom_section(s))
        .map(|s| {
            let id =
                s.labels.iter().min_by_key(|l| l.offset).map(|l| l.name.clone()).unwrap_or_default();
            let pb = s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false);
            (id, s.lma, s.image_bytes().len(), pb)
        })
        .collect();
    rows.sort_by_key(|r| r.1);

    // ── Anchors: recover the inferred island set from the resolved layout ──
    let declared: std::collections::HashMap<u32, &str> =
        pmap.anchors_for(sound_on).map(|a| (a.at, a.name.as_str())).collect();
    let mut inferred: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut prev_end: Option<u32> = None;
    for (_, lma, len, pb) in &rows {
        let is_anchor = prev_end.is_none() || *pb || *lma > prev_end.unwrap() + ANCHOR_GAP;
        if is_anchor {
            inferred.insert(*lma);
            if !declared.contains_key(lma) {
                return Err(format!(
                    "[map.undeclared-island] ROM section at {lma:#X} is an ANCHOR_GAP-inferred island but no `[[anchor]] at = {lma:#X}` is declared — add it to the placement map"
                ));
            }
        }
        prev_end = Some(lma.saturating_add(*len as u32));
    }
    for a in pmap.anchors_for(sound_on) {
        if !inferred.contains(&a.at) {
            return Err(format!(
                "[map.anchor-absent] declared anchor `{}` at {:#X} is not an inferred island in this build — the layout no longer anchors it (stale map or shape gate)",
                a.name, a.at
            ));
        }
    }

    // ── Order: the derived byte-emitting id sequence must be a subsequence of `order` ──
    if !pmap.order.is_empty() {
        let pos: std::collections::HashMap<&str, usize> =
            pmap.order.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
        let mut last: Option<(usize, &str)> = None;
        for (id, _, len, _) in &rows {
            if id.is_empty() || *len == 0 {
                continue;
            }
            let Some(&p) = pos.get(id.as_str()) else {
                return Err(format!(
                    "[map.order-undeclared] byte-emitting section `{id}` is absent from the declared `order` list — add it in its layout position"
                ));
            };
            if let Some((lp, lid)) = last {
                if p <= lp {
                    return Err(format!(
                        "[map.order-diverged] derived order places `{id}` after `{lid}`, but the declared `order` has `{id}` before it — the packer's order no longer matches the map"
                    ));
                }
            }
            last = Some((p, id.as_str()));
        }
    }

    // ── Holes (data; K2 enforces): the `after` anchor label must resolve ──
    for h in pmap.holes_for(sound_on) {
        let present = resolved.iter().any(|s| s.labels.iter().any(|l| l.name == h.after));
        if !present {
            return Err(format!(
                "[map.hole-anchor-missing] declared hole after `{}` (at {:#X}) — its `after` label is not in the resolved layout",
                h.after, h.at
            ));
        }
    }
    Ok(())
}

/// The chained whole-ROM build AND its sigil-canonical listing (the deb2-appendix
/// source for the off-canonical full-file layer). One `C` row per resolved section
/// label at its final VMA, de-duplicated and address-deterministic — mirrors
/// `build_native_rom_with_listing`'s listing derivation, on the chained (Frozen)
/// placement instead of the pinned one.
pub fn build_rom_chained_with_listing(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<(Vec<u8>, Vec<sigil_link::ListingSymbol>), String> {
    if profile.sound_on {
        ensure_generated(aeon);
    }
    let as_side = assemble_as_side(aeon, profile)?;
    let (emp, link_asserts) = build_emp(aeon, profile)?;
    let mut sections: Vec<Section> = as_side.sections;
    sections.extend(emp);

    // The declared-order authority: each ROM section's TRUE base, then its exact span.
    let true_bases = true_bases_by_index(&sections, &profile.size_source)?;
    let spans = declared_spans_by_index(&sections, &true_bases)?;

    let all = apply_declared_chain(sections, &true_bases, &spans);

    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&all, &stubs, true)
        .map_err(|d| format!("declared-chain: resolve_layout: {} diag(s); first {:?}", d.len(), d.first()))?;
    // Same drift partition as the pinned driver: real Value(0) drift is a hard fail;
    // gated-off-twin (unresolvable-extern) guards are inapplicable here.
    let adiags = sigil_link::check_link_asserts(&resolved, &stubs, &link_asserts);
    let (inapplicable, real): (Vec<_>, Vec<_>) = adiags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .partition(|d| d.message.contains("not defined in this link"));
    if !real.is_empty() {
        if std::env::var("NATIVE_DEBUG").is_ok() {
            for d in &real {
                eprintln!("REAL DRIFT: {}", d.message);
            }
        }
        return Err(format!("declared-chain drift guard FIRED: {} error(s); first {:?}", real.len(), real.first()));
    }
    enforce_inapplicable_allowlist_against(&inapplicable, &link_asserts, &profile.inapplicable_guards)?;

    // Sigil-canonical listing from the resolved image (one C row per label VMA).
    let mut listing: Vec<sigil_link::ListingSymbol> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for sec in &resolved {
        let origin = sec.vma_origin();
        for label in &sec.labels {
            if seen.insert(label.name.clone()) {
                listing.push(sigil_link::ListingSymbol {
                    name: label.name.clone(),
                    value: origin.wrapping_add(label.offset),
                    is_equate: false,
                    unused: false,
                });
            }
        }
    }

    let linked = sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("declared-chain: link: {} diag(s); first {:?}", d.len(), d.first()))?;
    // Parcel K1: the per-game placement map (`games/<g>/map.toml`) is the reviewed
    // authority — its regions (mirroring sigil.map.toml) drive emit_rom + the budget, and
    // its anchors/order/hole VALIDATE the resolved layout (fold-identical; drift fails loud).
    let map_path = profile.map_path(aeon);
    let map_src = std::fs::read_to_string(&map_path)
        .map_err(|e| format!("read {}: {e}", map_path.display()))?;
    let map = sigil_link::load_map(&map_src)
        .map_err(|e| format!("load {}: {e}", map_path.display()))?;
    let pmap = crate::map_placement::load_placement_map(&map_src)
        .map_err(|e| format!("placement {}: {e}", map_path.display()))?;
    validate_placement(&resolved, &pmap, profile.sound_on)?;
    check_object_bank_budget(&resolved, &map)?;
    let rom = sigil_link::emit_rom(&linked, &map).map_err(|e| format!("declared-chain: emit_rom: {e}"))?;
    Ok((rom, listing))
}

/// The off-canonical full-file build: the chained assembled ROM + the sigil-canonical
/// deb2 appendix (the same Option-A post-pipeline as `build_native_full_file`, over the
/// Frozen-placed profile). `debug` selects the convsym range/fixheader shape.
pub fn build_full_file_chained(aeon: &Path, profile: &GameProfile) -> Result<Vec<u8>, String> {
    let (rom, listing) = build_rom_chained_with_listing(aeon, profile)?;
    // Demo (engine-only) packs a smaller appendix than sonic4/config; floor per game.
    let floor = if profile.game_root_rel.contains("/demo/") {
        DEMO_APPENDIX_FLOOR
    } else {
        SONIC4_APPENDIX_FLOOR
    };
    append_deb2_appendix(aeon, &rom, &listing, profile.debug, floor)
}

/// Resolve `profile`'s frozen-table chained layout into its final ROM sections (the
/// SAME placement `build_rom_chained_with_listing` emits, minus the drift check / link /
/// emit). The shared substrate for the placement gate and the P4a LMA-correct
/// size-table derivation: both read `section.lma + label.offset` off these sections.
fn resolve_frozen_sections(aeon: &Path, profile: &GameProfile) -> Result<Vec<Section>, String> {
    if matches!(profile.size_source, SizeSource::PinnedBaked) {
        return Err("resolve_frozen_sections: profile is not a chained (Frozen) target".into());
    }
    if profile.sound_on {
        ensure_generated(aeon);
    }
    let as_side = assemble_as_side(aeon, profile)?;
    let (emp, _) = build_emp(aeon, profile)?;
    let mut sections: Vec<Section> = as_side.sections;
    sections.extend(emp);
    let true_bases = true_bases_by_index(&sections, &profile.size_source)?;
    let spans = declared_spans_by_index(&sections, &true_bases)?;
    let all = apply_declared_chain(sections, &true_bases, &spans);
    let stubs = SymbolTable::new();
    sigil_link::resolve_layout(&all, &stubs, true)
        .map_err(|d| format!("frozen resolve: resolve_layout: {} diag(s); first {:?}", d.len(), d.first()))
}

/// Resolve `profile`'s frozen-table chained layout and return, for every ROM section
/// carrying a frozen-table label whose RESOLVED base differs from the frozen truth, a
/// `(label, got, want)` mismatch. An empty result proves the frozen chainer placed every
/// declared section byte-correctly (the PLACEMENT invariant, independent of any residual
/// assembly-time-folded-constant divergence). `NATIVE_DEBUG` prints the full table.
pub fn frozen_placement_mismatches(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<Vec<(String, u32, u32)>, String> {
    let table = match &profile.size_source {
        SizeSource::Frozen(t) => t.clone(),
        _ => return Err("frozen_placement_mismatches: profile is not a Frozen target".into()),
    };
    let resolved = resolve_frozen_sections(aeon, profile)?;
    let mut rows: Vec<(String, u32, u32)> = Vec::new();
    for s in &resolved {
        if !is_rom_section(s) {
            continue;
        }
        // The frozen table stores ROM (LMA) addresses — asl listed every label at its
        // ROM position, including the Z80 idle (inline in the 68k boot region at
        // 0x3d8). So compare against the section's LMA base, not `vma_origin()`: a
        // phased `.emp` section (z80_idle, vma:$0) has vma_origin() == 0 but lands at
        // LMA 0x3d8. For non-phased sections lma == vma_origin, so this is unchanged.
        let origin = s.lma;
        for l in &s.labels {
            if let Some(&want) = table.get(&l.name) {
                let got = origin.wrapping_add(l.offset);
                if got != want {
                    rows.push((l.name.clone(), got, want));
                }
            }
        }
    }
    rows.sort_by_key(|r| r.1);
    if std::env::var("NATIVE_DEBUG").is_ok() {
        eprintln!("=== {} frozen-label mismatches: {} ===", profile.name, rows.len());
        for (name, got, want) in &rows {
            eprintln!("  {name} got={got:#x} want={want:#x} (Δ{})", *got as i64 - *want as i64);
        }
    }
    Ok(rows)
}

/// P4a — THE LMA-CORRECT SIZE DERIVATION. Re-derive `profile`'s frozen size table from
/// SIGIL'S OWN resolved layout, retiring the asl-`.lst` parse (`capture_offcanon`, row
/// 34). Resolves the frozen chain and reads each boundary label's ROM address =
/// `section.lma + label.offset`. This is LMA-correct where a naive `.lst` re-parse is
/// NOT: a phased section (z80 idle, vma `$0`) reports its ROM LMA `$3d8`, and a
/// section-END label (offset == the sigil-resolved `image_len`) reports the address
/// sigil COMPUTES, not one asl listed. The label SET is exactly the committed table's
/// keys (the boundary set the chainer needs); every one must be found or the derivation
/// fails loud (a vanished boundary label is a real regression, not a silent drop).
///
/// The derivation bootstraps off the committed table (the profile's `SizeSource::Frozen`
/// pins the labeled sections to place them), then reads the resolved positions back —
/// so for a byte-correct build it REPRODUCES the committed addresses exactly. That
/// fixpoint IS the proof: sigil's own resolve is now the authority; nothing parses asl.
pub fn derive_frozen_table(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<std::collections::BTreeMap<String, u32>, String> {
    let want: std::collections::HashSet<String> = match &profile.size_source {
        SizeSource::Frozen(t) => t.keys().cloned().collect(),
        _ => return Err("derive_frozen_table: profile is not a Frozen target".into()),
    };
    let resolved = resolve_frozen_sections(aeon, profile)?;
    let mut out: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for s in &resolved {
        if !is_rom_section(s) {
            continue;
        }
        for l in &s.labels {
            if want.contains(&l.name) {
                // LMA base (ROM address), NOT vma_origin — the phased-section fix.
                out.insert(l.name.clone(), s.lma.wrapping_add(l.offset));
            }
        }
    }
    // SECTION-END boundary labels (`<Stem>_End`) are the one-past-end markers asl listed
    // as symbols but sigil represents IMPLICITLY as a section terminus — the `.emp` data
    // modules (sonic_anims/particle_anims) and the native z80 idle emit no explicit `_End`
    // label, and the `.asm` twins that once carried them are deleted. Synthesize each from
    // its owning section's resolved geometry: the section whose base carries `<Stem>` ends
    // (one-past) at `section.lma + image_len` — LMA-correct for the phased z80 idle too.
    for name in want.iter() {
        if out.contains_key(name) {
            continue;
        }
        let Some(stem) = name.strip_suffix("_End") else { continue };
        let owner: Vec<&Section> = resolved
            .iter()
            .filter(|s| is_rom_section(s) && s.labels.iter().any(|l| l.name == stem))
            .collect();
        match owner.as_slice() {
            [s] => {
                out.insert(name.clone(), s.lma.wrapping_add(s.image_len()));
            }
            [] => {} // reported below
            many => {
                return Err(format!(
                    "derive_frozen_table({}): section-end label `{name}` stem `{stem}` names \
                     {} sections — ambiguous",
                    profile.name,
                    many.len()
                ));
            }
        }
    }
    // Fail loud if any committed boundary label is no longer resolvable.
    let missing: Vec<&String> = want.iter().filter(|n| !out.contains_key(*n)).collect();
    if !missing.is_empty() {
        return Err(format!(
            "derive_frozen_table({}): {} committed boundary label(s) absent from the resolved \
             layout (first: {:?}) — a real regression, not a silent drop",
            profile.name,
            missing.len(),
            missing.first()
        ));
    }
    Ok(out)
}

/// THE native whole-ROM build: AS residual (all gates ON, sound BINCLUDE) + every
/// placed `.emp` module → ONE resolve_layout + link + emit_rom.
pub fn build_native_rom(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    Ok(build_native_rom_with_listing(aeon, debug)?.0)
}

/// The native whole-ROM build AND its symbol listing — the S1.4 source. The
/// listing is derived from the SAME resolved sections the ROM bytes come from (one
/// `resolve_layout`), so the `.lst` addresses are exactly the emitted addresses.
/// Every section LABEL becomes an as-`-L` `C` (code/address) row; equates (`-`) are
/// omitted (convsym's `as_lst` reader takes only `C` symbols, §S1.4 note), so the
/// listing is the sigil-canonical debug symbol set — NOT a byte-imitation of asl's
/// name set (Option A: the `.emp` names are the source names going forward).
pub fn build_native_rom_with_listing(
    aeon: &Path,
    debug: bool,
) -> Result<(Vec<u8>, Vec<sigil_link::ListingSymbol>), String> {
    // §17 Wave-B B-0: canonical placement is COMPUTED (packed from live sizes over the
    // pins-derived order/anchors), so the canonical build routes through the chained
    // driver — one placement authority for all six targets. The pinned body below it
    // remains only for the PinnedBaked bootstrap path.
    if matches!(sonic4_profile(debug).size_source, SizeSource::Frozen(_)) {
        return build_rom_chained_with_listing(aeon, &sonic4_profile(debug));
    }
    ensure_generated(aeon);

    let as_side = assemble_native_all_gates_as_side(aeon, debug)?;
    let (emp_sections, link_asserts) = build_native_emp(aeon, debug)?;

    let mut sections = as_side.sections;
    sections.extend(emp_sections);

    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &stubs, true)
        .map_err(|d| format!("resolve_layout: {} diag(s); first: {:?}", d.len(), d.first()))?;

    // Derive the sigil-canonical listing from the resolved image: one `C` row per
    // section label at its final VMA. De-duplicated (a label defined once) and
    // deterministic (emit_listing address-sorts). RAM labels (`$FFFFxxxx`) are kept
    // — convsym's `-range 0 FFFFFF` drops them from the deb2 table, but they belong
    // in the full listing for the `s4budget` RAM consumer.
    let mut listing: Vec<sigil_link::ListingSymbol> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for sec in &resolved {
        let origin = sec.vma_origin();
        for label in &sec.labels {
            if seen.insert(label.name.clone()) {
                listing.push(sigil_link::ListingSymbol {
                    name: label.name.clone(),
                    value: origin.wrapping_add(label.offset),
                    is_equate: false,
                    unused: false,
                });
            }
        }
    }

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
    enforce_inapplicable_allowlist(&inapplicable, &link_asserts)?;

    let linked: LinkedImage = sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("link: {} diag(s); first: {:?}", d.len(), d.first()))?;

    let map_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("load sigil.map.toml: {e}"))?;
    check_object_bank_budget(&resolved, &map)?;
    let rom = sigil_link::emit_rom(&linked, &map).map_err(|e| format!("emit_rom: {e}"))?;
    Ok((rom, listing))
}

/// Build the sigil-native SYMBOL LISTING for one shape (Stage-3 P4c: the `pins.rs`
/// source that replaces parsing asl's `.lst`). Resolves the canonical pinned layout,
/// dumps the fully-resolved symbol table (labels + folded equates — incl. the
/// `MDDBG__*` link-external-base table), DEMANGLES the `.emp` locals (`$module$Parent$
/// local` → `Parent.local`, matching the deb2 appendix) so a dotted-local offset spec
/// like `AnimateSprite.cc_delete` resolves, and returns `(name → address, end_addr)`.
/// A demangle collision on distinct addresses is a hard error (the asl parser rejected
/// duplicate names; this preserves that guarantee).
pub fn sigil_native_symbol_listing(
    aeon: &Path,
    debug: bool,
) -> Result<(HashMap<String, u32>, u32), String> {
    let resolved = resolve_canonical_sections(aeon, debug)?;
    let stubs = SymbolTable::new();
    let raw = sigil_link::resolved_symbols(&resolved, &stubs);
    // Demangle via the deb2 path so dotted locals resolve; keep RAM + ROM + equates.
    let listing_syms: Vec<sigil_link::ListingSymbol> = raw
        .iter()
        .map(|(name, val)| sigil_link::ListingSymbol {
            name: name.clone(),
            value: *val as u32,
            is_equate: false,
            unused: false,
        })
        .collect();
    let demangled = sigil_link::demangle_symbols(&listing_syms);
    // Demangle can map two DISTINCT internal `.emp` locals (e.g. `raise_error` scratch
    // labels in sibling procs) to the same `Parent.local` — the asl `.lst` never did
    // (its names were already unique). Such an alias is AMBIGUOUS, so it is DROPPED, not
    // resolved to an arbitrary one. Every symbol the manifest queries is unique (a proc
    // name or a global), so it survives; a query for a dropped/absent name fails loudly
    // in `resolve()` (naming it) — never a silent wrong value.
    let mut values: HashMap<String, std::collections::HashSet<u32>> = HashMap::new();
    for s in &demangled {
        values.entry(s.name.clone()).or_default().insert(s.value);
    }
    let mut map: HashMap<String, u32> = values
        .into_iter()
        .filter_map(|(name, vs)| if vs.len() == 1 { Some((name, vs.into_iter().next().unwrap())) } else { None })
        .collect();
    // Section-END markers (`<Base>_End`): the one-past-end boundary labels asl listed but
    // sigil emits IMPLICITLY (the `.emp` data modules carry no `_End` label; the `.asm`
    // twins that did are deleted). Synthesize each from its section's resolved geometry —
    // the base (offset-0) label's name + `_End` at `lma + image_len` — matching P4a's
    // `derive_frozen_table`. Real labels win (only added when absent).
    for s in &resolved {
        if !is_rom_section(s) {
            continue;
        }
        if let Some(base) = s.labels.iter().find(|l| l.offset == 0) {
            let end_name = format!("{}_End", base.name);
            map.entry(end_name).or_insert_with(|| s.lma.wrapping_add(s.image_len()));
        }
    }
    let end_addr = *map
        .get("EndOfRom")
        .ok_or("sigil_native_symbol_listing: `EndOfRom` absent from the resolved layout")?;
    Ok((map, end_addr))
}

/// Load the project memory map (`sigil.map.toml`) — the same file `emit_rom` reads.
pub fn project_memory_map() -> Result<sigil_ir::map::MemoryMap, String> {
    let map_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    sigil_link::load_map(&std::fs::read_to_string(&map_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("load sigil.map.toml: {e}"))
}

/// Resolve the SHIPPED canonical layout (the packed chained placement) into its final
/// ROM sections — the substrate `pins.rs` regeneration and the object-bank budget gate
/// read addresses off. Post B-0 this is the chained resolve over `sonic4_profile`.
pub fn resolve_canonical_sections(aeon: &Path, debug: bool) -> Result<Vec<Section>, String> {
    resolve_frozen_sections(aeon, &sonic4_profile(debug))
}

/// Resolve the canonical PINNED layout into its final ROM sections (assemble all-gates-on
/// AS side + place every emp module at its pin + `resolve_layout`) — the Stage-1
/// bootstrap shape (`derive_offcanon --bootstrap-canonical`); the SHIPPED layout is
/// `resolve_canonical_sections`.
pub fn resolve_pinned_sections(aeon: &Path, debug: bool) -> Result<Vec<Section>, String> {
    ensure_generated(aeon);
    let as_side = assemble_native_all_gates_as_side(aeon, debug)?;
    let (emp_sections, _) = build_native_emp(aeon, debug)?;
    let mut sections = as_side.sections;
    sections.extend(emp_sections);
    let stubs = SymbolTable::new();
    sigil_link::resolve_layout(&sections, &stubs, true).map_err(|d| {
        format!("resolve_pinned_sections: resolve_layout: {} diag(s); first: {:?}", d.len(), d.first())
    })
}

/// The object code bank's used cursor = the resolved LMA of `__BUDGET_DATA`, the engine
/// marker (`engine.inc`) at the object bank's end / data region start. `None` if absent
/// (a game without the marker). `ObjCodeBase`/`__BUDGET_OBJBANK` sit at the bank BASE;
/// this cursor is the bank TERMINUS the `if * > $20000` guard checked.
pub fn object_bank_cursor(resolved: &[Section]) -> Option<u32> {
    for s in resolved {
        for l in &s.labels {
            if l.name == "__BUDGET_DATA" {
                return Some(s.lma.wrapping_add(l.offset));
            }
        }
    }
    None
}

/// Enforce the object-code-bank budget the map's `object_bank` region declares: the
/// `__BUDGET_DATA` cursor must not exceed `lma_base + size` (`$20000`). Returns the used
/// byte count. A missing region or marker is a no-op (`Ok(0)`) — additive, so this is
/// the map-owned successor to `engine.inc`'s `if * > $20000 / error`, not a new gate that
/// can spuriously fail a game that declares neither. Runs on every native build (both the
/// pinned canonical driver and the off-canonical chainer feed it their resolved sections).
pub fn check_object_bank_budget(
    resolved: &[Section],
    map: &sigil_ir::map::MemoryMap,
) -> Result<u32, String> {
    match object_bank_cursor(resolved) {
        Some(cursor) => map.check_budget("object_bank", cursor),
        None => Ok(0),
    }
}

/// CRC-32 (IEEE, the campaign provenance standard alongside byte-size). Small
/// table-per-call impl — the golden set is a handful of ROMs, so speed is moot.
pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

/// The header-neutral ASSEMBLED ANCHOR CRC over `bytes[0, eor)`: the checksum (`$18E`)
/// and ROM-end-pointer (`$1A4`) header fields are zeroed before the CRC, so it is the
/// drift-stable invariant the PROVENANCE anchors record (`e5765873` &c). `eor` is the
/// target's `EndOfRom`; `bytes` may be a full golden file (only its prefix is read).
pub fn assembled_anchor_crc(bytes: &[u8], eor: usize) -> u32 {
    let mut prefix = bytes[..eor.min(bytes.len())].to_vec();
    for i in crate::CHECKSUM_FIELD_RANGE.chain(crate::ROM_END_FIELD_RANGE) {
        if i < prefix.len() {
            prefix[i] = 0;
        }
    }
    crc32(&prefix)
}

/// The convsym `as_lst` filter build.sh passes verbatim (`build.sh:170-171`).
/// `z[A-Z].+` currently matches zero Aeon labels; passed for parity.
pub const CONVSYM_FILTER: &str = "z[A-Z].+";

/// The deb2 appendix magic (MD-Debugger symbol table) — the FIRST TWO bytes only.
/// convsym writes `de b2` then DATA-DEPENDENT header bytes (observed `04 02` in the
/// shipped ROMs, `00 1a`/`00 06` elsewhere — they encode the packing/offset, NOT a
/// fixed magic), so the presence control asserts these two bytes + a size band, not
/// the four-byte constant design §3.3 wrongly proposed (§S1.4 note).
pub const DEB2_MAGIC: [u8; 2] = [0xDE, 0xB2];

/// THE Option-A full-file native build: the assembled ROM (checksum-folded by
/// `emit_rom`) + the sigil-canonical deb2 appendix, produced by driving the REAL
/// `tools/convsym` over sigil's own listing, then re-fixing the Sega header NATIVELY
/// (the ROM-end pointer `$1A4` + the `$18E` checksum — was `tools/fixheader`, folded
/// in at Stage-3 P4d). Byte-for-byte the `build.sh:169-175` post-pipeline, fed sigil's
/// `.lst` instead of asl's. Deterministic. The assembled prefix `[0, EndOfRom)` stays
/// the asl-witnessed correctness anchor (header-neutral PRIMARY CRC); the appendix is
/// sigil-canonical (the frozen golden is sigil's OWN full file, not asl's).
///
/// Only `<aeon>/tools/convsym` is shelled now (fixheader retired); writes the ROM +
/// `.lst` to a fresh temp dir so parallel shapes never collide.
pub fn build_native_full_file(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    let (rom, listing) = build_native_rom_with_listing(aeon, debug)?;
    append_deb2_appendix(aeon, &rom, &listing, debug, SONIC4_APPENDIX_FLOOR)
}

/// The sigil-canonical deb2 appendix size floor for the sonic4/config symbol set
/// (~11 KB and up). Demo — engine-only, far fewer symbols — floors lower
/// (`DEMO_APPENDIX_FLOOR`). Both are PRESENCE controls: a collapsed listing
/// (a handful of symbols → ~0x27 B) still trips them.
pub const SONIC4_APPENDIX_FLOOR: usize = 0x2000;
/// The demo appendix floor (demo's engine-only set packs to ~0x1a0f B).
pub const DEMO_APPENDIX_FLOOR: usize = 0x1000;

/// Given the assembled ROM + listing, write both to a temp dir, run
/// `convsym … -output deb2 -a` then `fixheader`, and return the full appended file.
/// Split out so the t24 doctored-listing negative control can feed a mutated set.
pub fn append_deb2_appendix(
    aeon: &Path,
    rom: &[u8],
    listing: &[sigil_link::ListingSymbol],
    debug: bool,
    min_appendix: usize,
) -> Result<Vec<u8>, String> {
    let tools = aeon.join("tools");
    let convsym = tools.join("convsym");
    if !convsym.exists() {
        return Err(format!("convsym not found at {}", convsym.display()));
    }

    // A unique temp dir (pid + shape + a monotonic counter) so parallel builds
    // never share a `.bin`/`.lst`.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "sigil_s14_{}_{}_{}",
        std::process::id(),
        if debug { "debug" } else { "plain" },
        seq
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bin = dir.join("rom.bin");
    let lst = dir.join("rom.lst");
    std::fs::write(&bin, rom).map_err(|e| e.to_string())?;
    // Demangle the sigil-canonical `.emp` locals into `Parent.local` + drop compiler
    // plumbing (Stage-3 P2b, OQ-B) so the source-meaningful locals SURVIVE convsym's
    // `as_lst` name parser (which rejects the mangled `$` form) and reach the deb2
    // appendix. ROM-byte-neutral: the appendix is post-`EndOfRom` symbol data only.
    let deb2_listing = sigil_link::demangle_symbols(listing);
    std::fs::write(&lst, sigil_link::emit_listing(&deb2_listing)).map_err(|e| e.to_string())?;

    // convsym: append the deb2 table (build.sh:170-171 flags verbatim).
    let out = std::process::Command::new(&convsym)
        .arg(&lst)
        .arg(&bin)
        .args(["-input", "as_lst", "-range", "0", "FFFFFF", "-exclude", "-filter"])
        .arg(CONVSYM_FILTER)
        .arg("-a")
        .output()
        .map_err(|e| format!("spawn convsym: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "convsym failed (rc {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let mut full = std::fs::read(&bin).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dir);

    // Re-fix the Sega header over the APPENDED file (was `tools/fixheader`, retired at
    // Stage-3 P4d — a native fold, verified byte-identical to fixheader's output):
    //   (1) the ROM-end pointer at $1A4 (4 bytes BE) = the last byte's address (len-1);
    //   (2) the checksum at $18E over [$200, len) — AFTER (1), since $1A4 is in range.
    if full.len() >= 0x1A8 {
        let end = (full.len() as u32).wrapping_sub(1);
        full[0x1A4..0x1A8].copy_from_slice(&end.to_be_bytes());
        sigil_link::apply_header_checksum(&mut full);
    }

    // POSITIVE CONTROL (assert-PRESENCE, §S1.4 / condition 2b): the deb2 magic MUST
    // sit at EndOfRom and the appendix MUST be non-trivial — a silent convsym
    // failure (`2>/dev/null || true` in build.sh) yields an appendix-LESS ROM, which
    // this rejects as a HARD error rather than a smaller "valid" file.
    let eor = rom.len();
    if full.len() <= eor {
        return Err(format!(
            "deb2 appendix MISSING: full file {} <= assembled {} (convsym silently produced no append)",
            full.len(),
            eor
        ));
    }
    if full[eor..eor + 2] != DEB2_MAGIC {
        return Err(format!(
            "deb2 magic absent at EndOfRom {eor:#x}: found {:02X?}",
            &full[eor..eor + 2.min(full.len() - eor)]
        ));
    }
    let appendix = full.len() - eor;
    // Size band: the sigil-canonical appendix size is target-dependent (sonic4/config
    // ~11 KB+, demo ~7 KB); `min_appendix` is the per-target floor. A wildly-out-of-band
    // size means the symbol set collapsed or exploded (the t24 presence control).
    if !(min_appendix..=0x10000).contains(&appendix) {
        return Err(format!(
            "deb2 appendix size {appendix:#x} out of the expected band ({min_appendix:#x}..0x10000)"
        ));
    }
    Ok(full)
}

/// Run the REAL `tools/convsym … -output log` over sigil's listing — the functional
/// consumer path (condition 2c): the same parser that packs the deb2 table, asked
/// to resolve names→addresses. Returns the `HEXADDR: Name` map. A load-bearing
/// symbol resolving to its known address here is proof the appendix carries it
/// correctly; a doctored `.lst` address is detected (t24).
pub fn convsym_resolve(aeon: &Path, listing: &[sigil_link::ListingSymbol]) -> Result<std::collections::HashMap<String, u32>, String> {
    let convsym = aeon.join("tools/convsym");
    let dir = std::env::temp_dir().join(format!("sigil_s14_log_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let lst = dir.join("resolve.lst");
    std::fs::write(&lst, sigil_link::emit_listing(listing)).map_err(|e| e.to_string())?;
    let out = std::process::Command::new(&convsym)
        .arg(&lst)
        .arg("-")
        .args(["-input", "as_lst", "-output", "log", "-range", "0", "FFFFFF", "-exclude", "-filter"])
        .arg(CONVSYM_FILTER)
        .output()
        .map_err(|e| format!("spawn convsym -output log: {e}"))?;
    let _ = std::fs::remove_file(&lst);
    if !out.status.success() {
        return Err(format!("convsym -output log rc {:?}", out.status.code()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        // `HEXADDR: Name`
        if let Some((addr, name)) = line.split_once(": ") {
            if let Ok(v) = u32::from_str_radix(addr.trim(), 16) {
                map.insert(name.trim().to_string(), v);
            }
        }
    }
    Ok(map)
}

#[cfg(test)]
mod allowlist_tests {
    //! t24 both-directions negative controls for the Stage-1 inapplicable-drift-guard
    //! allowlist: the enforcement rejects an UNKNOWN Poison guard (a typo'd/renamed
    //! extern or a new twin-parity guard) AND a STALE allowlist (an entry that no
    //! longer folds to Poison), so the native gates can never silently vacate a guard.
    use super::{
        enforce_inapplicable_allowlist, enforce_inapplicable_allowlist_against,
        STAGE1_INAPPLICABLE_GUARDS,
    };
    use sigil_ir::{Expr, LinkAssert, MsgPart};
    use sigil_span::{Diagnostic, Level, Span, SourceId};

    fn span(n: u32) -> Span {
        Span { source: SourceId(0), start: n, end: n }
    }
    /// A synthetic Poison diagnostic + its matching LinkAssert (site text) for one
    /// `(extern, site_token)` allowlist-shaped guard.
    fn guard(n: u32, ext: &str, site: &str) -> (Diagnostic, LinkAssert) {
        let d = Diagnostic {
            level: Level::Error,
            message: format!(
                "link assertion condition references symbol(s) `{ext}` not defined in this link"
            ),
            primary: span(n),
        };
        let a = LinkAssert {
            cond: Expr::Int(0),
            message: vec![MsgPart::Text(format!("engine/level/twin.asm and {site} disagree"))],
            fatal: false,
            level: Level::Error,
            span: span(n),
        };
        (d, a)
    }

    fn build(entries: &[(&str, &str)]) -> (Vec<Diagnostic>, Vec<LinkAssert>) {
        let mut ds = Vec::new();
        let mut as_ = Vec::new();
        for (i, (e, s)) in entries.iter().enumerate() {
            let (d, a) = guard(i as u32, e, s);
            ds.push(d);
            as_.push(a);
        }
        (ds, as_)
    }

    #[test]
    fn empty_allowlist_passes_with_no_poison() {
        // Post the bg/camera flip STAGE1_INAPPLICABLE_GUARDS is empty; the
        // enforcement asserts NO Poison drift guard exists → OK on an empty set.
        assert!(STAGE1_INAPPLICABLE_GUARDS.is_empty(), "the allowlist retired to empty");
        assert!(enforce_inapplicable_allowlist(&[], &[]).is_ok());
    }

    #[test]
    fn any_poison_guard_is_rejected_against_the_empty_allowlist() {
        // With the allowlist empty, ANY Poison-folding drift guard is a HARD FAIL
        // (the strengthened no-Poison invariant — a new twin-parity guard needs a
        // ruling, not a silent vacation).
        let (ds, as_) = build(&[("SOME_NEW_TWIN_CONST", "camera.emp")]);
        let refs: Vec<&Diagnostic> = ds.iter().collect();
        let err = enforce_inapplicable_allowlist(&refs, &as_).unwrap_err();
        assert!(err.contains("NOT in the allowlist"), "got: {err}");
    }

    #[test]
    fn stale_allowlist_is_rejected() {
        // The staleness direction still holds (an allowlisted guard that no longer
        // folds to Poison → HARD FAIL), proven against a SYNTHETIC 1-entry allowlist
        // with no matching Poison diag (the empty shipped list can't be stale).
        let synthetic: &[(&str, &str)] = &[("SYNTHETIC_TWIN_CONST", "twin.emp")];
        let err = enforce_inapplicable_allowlist_against(&[], &[], synthetic).unwrap_err();
        assert!(err.contains("STALE"), "got: {err}");
    }
}

#[cfg(test)]
mod align_recompute_tests {
    //! Fix 3 — the trailing-`align` overshoot recompute-on-relocation. A pure-data
    //! section that ends in a stale `align` `Fill` (baked for its AS-residual position)
    //! must have that padding trimmed to its relocated reserved span, so its image no
    //! longer overshoots the next hard-org anchor. Byte-neutral when already in-bounds.
    use super::trim_trailing_align_overshoot;
    use sigil_ir::{Cpu, DataFragment, Fragment, Section, SectionPlacement};
    use sigil_span::Span;

    fn span0() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }
    /// A pure-data section: `data_len` bytes of data, then a trailing zero-`Fill` of
    /// `pad` bytes (the align padding).
    fn data_then_pad(data_len: u32, pad: u32) -> Section {
        Section {
            name: "heightmaps".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0xAB; data_len as usize], fixups: vec![], span: span0() }),
                Fragment::Fill { value: 0, count: pad, span: span0() },
            ],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![],
        }
    }

    #[test]
    fn overshoot_trims_trailing_pad_to_span() {
        // Image = 0x100 data + 0x40 pad = 0x140; relocated span clamps to 0x134
        // (0xC shorter). The 0xC overshoot must come off the trailing pad.
        let mut s = data_then_pad(0x100, 0x40);
        assert_eq!(s.image_len(), 0x140);
        trim_trailing_align_overshoot(&mut s, 0x134);
        assert_eq!(s.image_len(), 0x134, "image trimmed to the relocated span");
        // The data is untouched; only the pad shrank (0x40 → 0x34).
        match s.fragments.last().unwrap() {
            Fragment::Fill { count, value: 0, .. } => assert_eq!(*count, 0x34),
            other => panic!("expected trailing zero-fill, got {other:?}"),
        }
    }

    #[test]
    fn in_bounds_is_byte_neutral() {
        // image_len (0x140) <= span (0x140): nothing trimmed (the pinned-path case).
        let mut s = data_then_pad(0x100, 0x40);
        let before = s.image_bytes();
        trim_trailing_align_overshoot(&mut s, 0x140);
        assert_eq!(s.image_bytes(), before, "in-bounds section untouched");
    }

    #[test]
    fn overshoot_not_trailing_pad_is_left_for_the_linker() {
        // A section whose overshoot is real DATA (no trailing zero-fill) is NOT
        // trimmed — resolve_layout must reject it loudly, never silent corruption.
        let mut s = Section {
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0xAB; 0x140],
                fixups: vec![],
                span: span0(),
            })],
            ..data_then_pad(0, 0)
        };
        trim_trailing_align_overshoot(&mut s, 0x134);
        assert_eq!(s.image_len(), 0x140, "real data is never trimmed");
    }
}

#[cfg(test)]
mod placement_validation_tests {
    //! Parcel K1 — negative probes proving the map VALIDATION has teeth: each lint
    //! (undeclared-island, anchor-absent, order-diverged, order-undeclared) fires on a
    //! doctored map, and the correct map passes. Synthetic resolved layout:
    //! boot head (label-less) @0x0, GameLoop @0x100, ObjCodeBase @0x10000 (ANCHOR_GAP island).
    use super::validate_placement;
    use crate::map_placement::{load_placement_map, PlacementMap};
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement};
    use sigil_span::Span;

    fn span0() -> Span { Span { source: sigil_span::SourceId(0), start: 0, end: 0 } }

    fn sec(label: &str, lma: u32, len: usize) -> Section {
        Section {
            name: format!("sec{lma}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: if label.is_empty() { vec![] } else { vec![Label { name: label.into(), offset: 0 }] },
            fragments: vec![Fragment::Data(DataFragment { bytes: vec![0u8; len], fixups: vec![], span: span0() })],
            placement: SectionPlacement::Pinned,
            reserved_span: len as u32,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    fn layout() -> Vec<Section> {
        vec![sec("", 0x0, 0x100), sec("GameLoop", 0x100, 0x50), sec("ObjCodeBase", 0x10000, 0x10)]
    }

    fn good_map() -> PlacementMap {
        load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        )
        .unwrap()
    }

    #[test]
    fn correct_map_passes() {
        assert!(validate_placement(&layout(), &good_map(), false).is_ok());
    }

    #[test]
    fn undeclared_island_fires() {
        // Drop the 0x10000 anchor — the inferred island is now undeclared.
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n[[anchor]]\nname=\"boot_head\"\nat=0x0\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false).unwrap_err();
        assert!(e.contains("map.undeclared-island") && e.contains("0x10000"), "{e}");
    }

    #[test]
    fn anchor_absent_fires() {
        // Declare an anchor the layout has no island for.
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n\
             [[anchor]]\nname=\"ghost\"\nat=0x99999\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false).unwrap_err();
        assert!(e.contains("map.anchor-absent") && e.contains("0x99999"), "{e}");
    }

    #[test]
    fn order_diverged_fires() {
        // Declare ObjCodeBase before GameLoop — the derived order disagrees.
        let m = load_placement_map(
            "order = [\"ObjCodeBase\", \"GameLoop\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false).unwrap_err();
        assert!(e.contains("map.order-diverged"), "{e}");
    }

    #[test]
    fn order_undeclared_fires() {
        // Omit GameLoop from the order list.
        let m = load_placement_map(
            "order = [\"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false).unwrap_err();
        assert!(e.contains("map.order-undeclared") && e.contains("GameLoop"), "{e}");
    }

    #[test]
    fn shape_gated_sound_bank_anchor() {
        // A sound_on-gated anchor must be absent-checked only in sound_on shapes.
        let mut secs = layout();
        secs.push({
            let mut s = sec("SoundTablesZ80_Head", 0x58000, 0x20);
            s.vma_base = Some(0x8000); // phase bank
            s
        });
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\", \"SoundTablesZ80_Head\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n\
             [[anchor]]\nname=\"sound_bank\"\nat=0x58000\nvma=0x8000\nwhen=\"sound_on\"\n",
        ).unwrap();
        // sound_on: the phase-bank island is declared → ok.
        assert!(validate_placement(&secs, &m, true).is_ok());
        // sound_off with the phase bank still present → it's an undeclared island (gate excludes it).
        let e = validate_placement(&secs, &m, false).unwrap_err();
        assert!(e.contains("map.undeclared-island") && e.contains("0x58000"), "{e}");
    }
}
