//! Flip Stage 1 · S1.1 — THE ALL-GATES-ON NATIVE DRIVER.
//!
//! The dual-native whole-ROM build: assemble the residual root `.asm` with EVERY
//! `SIGIL_EMP_*` code gate ON (so the AS side org-resumes past each gated region,
//! leaving a hole), natively lower + place EVERY ported `.emp` module at its
//! `pins`-region base through ONE `resolve_layout` + `link` + `emit_rom`, and
//! compare the whole ROM byte-for-byte against the committed golden.
//!
//! WHAT THE COMPARAND IS, AND IS NOT. This header used to say the compare was
//! against "the live `asl` `s4.bin`" — and against `main.asm`. Both are gone:
//! `asl` left the pipeline at the Spec-5 Stage-2 flip and `main.asm` was deleted
//! with it, so the comparand is a SIGIL-BUILT golden. That makes this gate a
//! reproducibility and regression gate, NOT an independent-oracle one: it proves
//! the build still produces the frozen bytes, not that those bytes match a second
//! assembler. The genuinely `asl`-minted evidence still in the tree is the
//! ISA-level golden corpus (`sigil-isa`'s vectors, frozen from a real pre-flip
//! `asl` run) — that remains a real oracle, and it is a different claim from this
//! one (lens sweep, seats TEST/A2, finding S20).
//!
//! The sound stack (DAC/MT/SFX banks + the resident Z80 driver + tables) is NOT
//! placed here — it enters via the AS side's BINCLUDE of the seam-emitted `.bin`s
//! (the proven `seam2_sfx_rom` path). Only the CODE/DATA `.emp` modules listed in
//! the registry below are natively placed.
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

use sigil_frontend_as::{assemble_root_relocating_warned, Options as AsOptions};
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{self, place_sections};
use sigil_ir::{Cpu, Fragment, Module, Section, SectionPlacement, SymbolTable};

use crate::{seam1, seam2};

/// Format EVERY build error, one per line, instead of only the first.
///
/// A `build_program` failure is usually a small cluster of related diagnostics (a
/// struct-literal type mismatch reports both the offending field and the item that
/// failed to emit), and reporting only `first` hides whichever one names the actual
/// mistake. Byte spans rather than `file:line` because `build_program_open_embed`
/// does not hand back the `SourceMap` needed to resolve them — locating the span is
/// `head -c <end> <file> | tail -c <end-start>`.
fn fmt_diag_list(errs: &[&sigil_span::Diagnostic]) -> String {
    errs.iter()
        .map(|d| format!("  [{:?}] {} @ {:?}", d.level, d.message, d.primary))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Flip Stage 2 · S1.2 — THE GAME PROFILE (the off-canonical driver parameters) ──
//
// The Stage-1 native driver is sonic4-shaped throughout (registry, defines, keystones,
// the OBJDEFS `text` guard, the drift-guard allowlist, sound-on). The three off-canonical
// targets (demo plain/debug, Config-A, Config-B) reuse the SAME chainer + split-golden
// machinery through a `GameProfile` that carries every sonic4-hardcoding as data.

/// One off-canonical / canonical target's full driver parameterization.
pub struct GameProfile {
    pub name: &'static str,
    /// The AS residual root, relative to the aeon tree (`games/<g>/game_root.asm`).
    pub game_root_rel: &'static str,
    /// The game's RAM module id (item #7c). Its region-form `vars` block chains
    /// `game_ram @ after(upper_ram)` onto the engine RAM, so it must be reachable
    /// from the synthetic entry (so its `pub vars` labels export to the joint
    /// link) and harvested (so eager AS reads of game RAM labels resolve).
    pub game_ram_module: &'static str,
    /// The game's contract manifest module id (L1 P2), the one `implement Game`
    /// (`games.sonic4.game` / `games.demo.game` = `games/<g>/config/game.emp`). It
    /// rides the synthetic entry's `use` edges (alongside `engine.game_contract`,
    /// the interface) so both are reachable — the whole-program bind pass then
    /// resolves the interface against this manifest and the engine's `Game.MEMBER`
    /// / `invoke Game.hook` sites fold/lower. Both emit zero bytes; neither is a
    /// placed registry module.
    pub manifest_module: &'static str,
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
    /// CRASH-REPORT SHAPE (owner-ruled 2026-08-04): does this target ship the MD
    /// Debugger / `error_handler` island + the deb2 symbol appendix? TRUE for every
    /// profile except the opt-in `lean_profile` — the island and its symbol table are
    /// DIAGNOSTICS (a player's crash must be reportable), not debug EQUIPMENT, and the
    /// release ROM is ~9% of a 4 MB cart. Equipment (asserts / SOUND_DEBUG_HOTKEYS /
    /// SOUND_DBG_MIRROR / boot autoplay / CompressionSelfTest) rides `debug` alone.
    ///
    /// It drives three things in lockstep: the `CRASH_REPORT` comptime define in
    /// `emp_defines` (vectors.emp's fault cells), the `__MDDBG__` AS definedness define
    /// (the `debugger.asm` include in each game_root.asm), and the registry's
    /// `error_handler` / `release_fault` split. Read as `debug || crash_report`
    /// everywhere — `debug` alone never means "carries the debugger".
    pub crash_report: bool,
    /// Sound ON: pass `-D SOUND_DRIVER_ENABLED`, the DAC/MT/SFX BINCLUDE gates, and
    /// run `ensure_generated`. OFF (demo/Config-B): no sound define at all (AS `ifdef`
    /// checks DEFINEDNESS — a `=0` would still take the sound arm), no BINCLUDE, no gen.
    pub sound_on: bool,
    /// Extra AS `-D`s beyond the sound defines (Config-A: SOUND_DEBUG_HOTKEYS /
    /// SOUND_DBG_MIRROR).
    pub extra_as_defines: Vec<(&'static str, i64)>,
    pub registry: Vec<ModuleSpec>,
    /// The build-shape comptime defines the `.emp` modules read.
    pub emp_defines: Vec<(&'static str, i128)>,
    /// Enforce exactly ONE non-empty `text` section (OBJDEFS). OFF for demo (no objdefs).
    pub require_one_text: bool,
    /// The inapplicable-drift-guard allowlist (t24 both-directions), per target.
    pub inapplicable_guards: Vec<(&'static str, &'static str)>,
    /// The target's committed boundary table (label → address), the source of every
    /// section's declared per-region SIZE — the load-bearing S1.2 finding: the chainer
    /// must reserve each section's exact asl span or relaxation settles at a different
    /// fixpoint (see the S1.2 chainer note).
    ///
    /// The AS residual carries WRONG sonic4 resume orgs, so this table supplies each
    /// section's PROVISIONAL BASE. It is not the placement authority: since K5 the map's
    /// `order` list drives the byte-emitting sequence and its `[[anchor]]` entries
    /// declare the islands, and `validate_placement` confirms both on every build.
    /// The provisional bases identify which sections ARE islands and feed
    /// measurement; every non-island section's base is then PACKED from live-
    /// measured sizes (see `packed_true_bases`), so a size-changing `.emp` parcel
    /// shifts downstream sections automatically instead of colliding with stale pins.
    pub frozen_sizes: HashMap<String, u32>,
    /// FIXTURE-ONLY derived placement (Art-streaming P2c Task 11 stress-art). When true,
    /// `packed_true_bases` packs every non-island section GREEDILY from live-measured sizes
    /// with NO frozen provisional-base overrun check and NO island-reclassification guard —
    /// so a fixture whose art pool grew tens of KB in one section (uniquified OJZ pool, 41
    /// pages) places without a "hand ruling", while the org-anchored islands (object bank,
    /// DAC/sound phase banks, error_handler-last) are STILL held (island/phase-bank
    /// classification is unchanged) and any real anchor overrun still fails loud at the
    /// final `resolve_layout`. FALSE for every shipped shape (a shipped shape that needed
    /// this would be masking a real placement regression) — the CLI refuses to pair
    /// `--stress-art` with any shipped shape selector.
    pub fixture_placement: bool,
    /// EXTRA ENTRIES (`sigil build --extra-entry`): modules this build must
    /// EVALUATE although no reachable module `use`s them. Each rides the synthetic
    /// entry's `use` edges alongside the registry, so the module is lowered inside
    /// the real profile — the same manifest rewrites (helper publication + glob
    /// normalization), the same comptime `-D` set — and its module-level `ensure`s
    /// run, failing the build with their own message when they hold false.
    ///
    /// Each element is a dotted module id or a path to a `.emp` file under the scan
    /// root; [`build_emp`] resolves both and refuses a name that names nothing.
    /// The NAMED module must contribute nothing to the artifact — and only the named
    /// module is checked, so a module it imports from outside the closure can still
    /// pull bytes in; see `refuse_artifact_contribution` for that boundary.
    /// EMPTY in every shipping profile: a shape whose ROM depended on one would be
    /// a shape the `--extra-entry`-free build does not produce.
    pub extra_entries: Vec<String>,
}

impl GameProfile {
    pub fn game_root(&self, aeon: &Path) -> std::path::PathBuf {
        aeon.join(self.game_root_rel)
    }

    /// This profile with `ids` as its [`extra_entries`](Self::extra_entries).
    /// A builder rather than a constructor argument: the shipping profiles are the
    /// authority on a shape, and an extra entry is an INVOCATION's addition to one.
    pub fn with_extra_entries<I, S>(mut self, ids: I) -> GameProfile
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_entries = ids.into_iter().map(Into::into).collect();
        self
    }

    /// The `games.<game>` module-id prefix this shape's modules live under — the
    /// game RAM module's parent id names the game. The ONE spelling of that
    /// derivation: the L1 interface binding (each shape binds its own game's
    /// `implement`) and every game-partitioning corpus probe read this, never a
    /// hand-kept list.
    pub fn game_module_prefix(&self) -> &'static str {
        self.game_ram_module.rsplit_once('.').map_or(self.game_ram_module, |(p, _)| p)
    }

    /// `EndOfRom` — this shape's assembled-bar length, DERIVED at every call.
    ///
    /// It is the `EndOfRom` row of the profile's own committed boundary table, the
    /// table `derive_offcanon` regenerates from a live resolve.
    ///
    /// It is an INPUT, not an oracle. No derivation under test is checked against it:
    /// `offcanon_assembled_bar` cross-compares it with the provenance chain and the
    /// table header, which is agreement between artifacts rather than an independent
    /// expectation for a code path. So sourcing it from the generated artifact makes
    /// nothing circular — the contrast is `native_full_rom`'s `Ground_Move_Cap` row,
    /// which IS the independent expectation for the convsym resolve path and must stay
    /// literal, since `repin` generates the pins from the listing convsym consumes.
    /// A hand-typed spelling buys no witness here and only opens a silent desync
    /// window between a re-layout and the re-type, so there is none.
    ///
    /// Absent `EndOfRom` PANICS rather than falling back: a boundary table that lost
    /// its terminus is a regression, and a plausible number returned in its place
    /// would hide it.
    pub fn assembled_len(&self) -> usize {
        *self.frozen_sizes.get("EndOfRom").unwrap_or_else(|| {
            panic!(
                "profile `{}`: its frozen boundary table carries no `EndOfRom` row \
                 — the assembled bar is unmeasurable, not zero",
                self.name
            )
        }) as usize
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
/// `use` edge + `build_program` reachability) and its declared section name (the
/// `module … in <section>` name; `"text"` for a defaulted module).
///
/// A row carries no placement address. Every target chains: the chainer packs each
/// section from its live-measured size in the map's declared
/// `order`, so the registry is the module-REACHABILITY list and nothing else. Where
/// a section's ROM address is needed as an oracle — the port gates' reference
/// windows, `repin`'s regeneration — it comes from `pins`, not from here.
pub struct ModuleSpec {
    pub module_id: &'static str,
    pub section: &'static str,
}

/// THE REGISTRY — the code/data `.emp` modules Stage 1 places natively.
///
/// No count is written here on purpose. It said "52" for long enough to be wrong
/// by dozens, which is the failure mode A2 measured across this codebase's prose:
/// a headline number recorded at a landing commit and never revisited. The list
/// below is the count.
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
///
/// `crash_report` selects the FAULT-HANDLER shape, independently of `debug`: the
/// `error_handler` island rides `debug || crash_report`, `release_fault` rides the
/// `else`. Both canonical shapes set `crash_report`, so both carry the island.
pub fn registry(debug: bool, crash_report: bool) -> Vec<ModuleSpec> {
    macro_rules! m {
        ($id:literal, $sec:literal) => {
            ModuleSpec { module_id: $id, section: $sec }
        };
    }
    let mut specs = vec![
        // ── Engine system ──
        m!("engine.system.vectors", "vectors"),
        // Parcel K4: the $100-$1FF ROM header is native (was header.inc's gameHeader
        // macro). Game-specific (games.sonic4.header / games.demo.header); the
        // strings are typed `[u8; N]` (the width guard). Boundary key Checksum ($18E).
        m!("games.sonic4.header", "header"),
        m!("engine.boot", "boot"),
        // Parcel K2 — boot_data ports to `.emp` as TWO sections (the $3FE map
        // hole: the engine.z80_init idle packs between them in the no-sound
        // shapes; the resident driver blob rides `boot_head` in sound-on). Two
        // ModuleSpecs, one per section; the chainer packs both from the frozen
        // BootData/BootData_End boundary rows.
        m!("engine.boot_data", "boot_head"),
        m!("engine.boot_data", "boot_tail"),
        m!("engine.vdp_init", "vdp_init"),
        m!("engine.dma_queue", "dma_queue"),
        m!("engine.buffers", "buffers"),
        m!("engine.vblank", "vblank"),
        m!("engine.hblank", "hblank"),
        m!("engine.controllers", "controllers"),
        m!("engine.game_loop", "game_loop"),
        // Parcel I3 (2026-08-02) — the demo record/replay module (engine.replay:
        // Input_Tick + Replay_Hash), placed between game_loop and s4lz per the
        // map.toml order. Engine-agnostic (demo gets it via the engine.* filter).
        m!("engine.replay", "replay"),
        // ── Engine compression ──
        m!("engine.s4lz", "s4lz"),
        // engine.zx0 DELETED (aeon F-6): the blocking ZX0 decoder moved into
        // engine.compression_selftest (its sole consumer, DEBUG-only) — release
        // ships the streaming decoders only.
        // Art-streaming P2a — the resumable stack-flat ZX0 decoder (zx0_resume.emp).
        m!("engine.zx0_resume", "zx0_resume"),
        m!("engine.math", "math"),
        // ── Engine objects ──
        // Parcel K4 inc-6: the object-code-bank base (ObjCodeBase + the offset-0 safety
        // rts) — was engine.inc's `org $10000 / ObjCodeBase: rts`. Native so the org
        // retires. Placed at $10000 by the object_bank anchor. Engine-agnostic (demo too).
        m!("engine.objects.objcodebase", "objcodebase"),
        m!("engine.objects.dplc", "dplc"),
        m!("engine.objects.core", "core"),
        m!("engine.objects.sprites", "sprites"),
        m!("engine.objects.animate", "animate"),
        m!("engine.objects.collision", "collision"),
        m!("engine.objects.rings", "rings"),
        m!("engine.objects.entity_window", "entity_window"),
        m!("engine.objects.children", "children"),
        m!("engine.objects.load_object", "load_object"),
        // ── Engine level ──
        m!("engine.plane_buffer", "plane_buffer"),
        m!("engine.tile_cache", "tile_cache"),
        m!("engine.collision_lookup", "collision_lookup"),
        m!("engine.section", "section"),
        m!("engine.camera", "camera"),
        m!("engine.parallax", "parallax"),
        // Effects P1 module split: the sparse raster dispatcher and the per-section
        // palette load, moved out of engine.hblank / engine.buffers into the effects
        // suite's own modules. Placed between parallax and load_art per map `order`.
        m!("engine.effects.raster", "raster"),
        m!("engine.effects.palette", "palette"),
        // Effects P3 Parcel C2 — preset.emp: the EffectsPreset struct plus
        // Effects_InstallPreset, the single total-binding installer that replaced the
        // three per-field consumers at the section crossing.
        m!("engine.effects.preset", "preset"),
        m!("engine.load_art", "load_art"),
        // Art-streaming P2a — the VBlank-bookmark page-in dispatcher (page_in.emp),
        // placed between load_art and bg per map.toml `order`. Engine-agnostic
        // (demo gets it too; its DEBUG self-test scaffold is HAS_ACT_ART_POOL-gated).
        m!("engine.page_in", "page_in"),
        // Art-streaming P2b Task 6 — the VRAM page-frame residency cache
        // (page_cache.emp), placed between page_in and bg per map.toml `order`.
        // Engine-agnostic (demo links it too; tile_cache/page_in/load_art call
        // PageCache_* cross-seam). Shape-DEPENDENT length: PageCache_Audit and the
        // Ref/Unref/AllocFrame DEBUG asserts are DEBUG-only.
        m!("engine.page_cache", "page_cache"),
        m!("engine.bg", "bg"),
        m!("engine.bg_anim", "bg_anim"),
        // ── Engine debug / sound caller ──
        m!("engine.sound_api", "sound_api"),
        // Review item 29 part 4 (the MDDBG strip): null_interrupt.emp is DELETED
        // (its tolerant `rte` had had no vector referencer since item 27's ruling).
        // The tail placement slot it used to hold is now the FAULT-HANDLER slot,
        // filled per shape below: `error_handler` under `debug || crash_report`,
        // `release_fault` under the `else` (the lean shape only).
        // Parcel K4 B1: the ROM terminus (EndOfRom + the 3 walls), was engine.inc's
        // `EndOfRom:` label + the `if … error` guards. Zero-length section placed
        // LAST (boundary key EndOfRom, already frozen in all six tables). The plane
        // wall is a comptime ensure; EndOfRom evenness/4MB are link-time asserts.
        m!("engine.epilogue", "epilogue"),
        // ── Game player ──
        // player_common fully flipped (conv-d #49): player_common.asm deleted. The
        // module owns the PlayerV overlay + PPHYS_*/macro templates (state files
        // import by `use`); camera.emp late-binds the one _pl_state offset it
        // link-exports as an `equ`.
        m!("games.sonic4.player_common", "player_common"),
        m!("games.sonic4.player_sensors", "player_sensors"),
        m!("games.sonic4.player_ground", "player_ground"),
        m!("games.sonic4.player_air", "player_air"),
        m!("games.sonic4.player_spindash", "player_spindash"),
        // Character-dispatch C2: Tails' flight (player_fly.emp) — PSTATE_FLY's
        // body plus Ability_TailsFlight, the AbilityHook CharDef_Tails points at.
        // The fourth player STATE file, so it sits with the other three and ahead
        // of the character records per map.toml `order`; it is also the last
        // player code before them, which is why `player_spindash`'s end anchor
        // moves from CharDef_Sonic to PState_Fly.
        m!("games.sonic4.player_fly", "player_fly"),
        // Character-dispatch C4 Task 10: Knuckles' glide/slide (player_glide.emp) —
        // PSTATE_GLIDE/GLIDEFALL/SLIDE plus Ability_KnuxGlide, the AbilityHook
        // CharDef_Knuckles points at. The FIFTH player STATE file, placed right after
        // player_fly and ahead of the character records per map.toml `order` (which
        // moves player_fly's end anchor from CharDef_Sonic to PState_Glide).
        m!("games.sonic4.player_glide", "player_glide"),
        // Character-dispatch C4 Task 11: Knuckles' wall climb + ledge pull-up
        // (player_climb.emp) — PSTATE_CLIMB/LEDGE plus the glide wall-catch. The sixth
        // player STATE file, placed right after player_glide per map.toml `order`
        // (player_glide's end anchor moves to Climb_WallDist).
        m!("games.sonic4.player_climb", "player_climb"),
        m!("games.sonic4.sonic", "sonic"),
        // Character-dispatch C1 task 6: the Tails character RECORD (tails.emp) —
        // CharDef_Tails + PhysTable_Tails, pure data, the exact peer of `sonic`
        // and placed right after it per map.toml `order`. This is what makes
        // CHAR_TAILS a real roster row instead of a stub aimed at CharDef_Sonic;
        // the sprite data it points at (Map_/DPLC_/Art_Tails, Ani_Tails) landed in
        // task 5 as `tails_data` / `tails_anims`.
        m!("games.sonic4.tails", "tails"),
        // Character-dispatch C4 task 9: the Knuckles character RECORD
        // (knuckles.emp) — CharDef_Knuckles + PhysTable_Knuckles, the third peer
        // of `sonic` / `tails`, placed right after them per map.toml `order`
        // (which moves `tails`' end anchor from CharacterDefs to
        // CharDef_Knuckles). Landing it retires the TEMP roster stub that aimed
        // CHAR_KNUCKLES at the Sonic record, so every CHAR_* id now resolves to
        // its own complete record. His `cd_ability` is still Ability_None —
        // glide/climb are tasks 10-11 — but his art, mappings, animations, boxes,
        // physics row and PALETTE are his own.
        m!("games.sonic4.knuckles", "knuckles"),
        // The character ROSTER (characters.emp): the CharacterDefs table, the
        // character-agnostic asset/art loaders (Player_InitAssets / Player_LoadArt),
        // the AbilityHook type and Ability_None. A PEER of `sonic` (and of the coming
        // `tails`/`knuckles` records), deliberately NOT part of `player_common` —
        // player_common owns the shared player FRAME, this owns the roster the frame
        // dispatches through. Placed right after `sonic` per map.toml `order`.
        m!("games.sonic4.characters", "characters"),
        // Character-dispatch C1: Tails' twin tails (tails_appendage.emp) — the
        // appendage CHILD OBJECT (its reconcile, its effect-pool spawn, and the
        // per-frame parent copy + own DPLC stream + draw). Tails-owned GAME
        // content, so it is placed with the character records rather than among
        // the test objects, and being the last player-side content before them it
        // takes the slot `characters`' end anchor used to name (TestStatic_Main).
        m!("games.sonic4.tails_appendage", "tails_appendage"),
        // Dust-effect Task 4: the skid dust. dust_puff.emp is the fire-and-forget
        // puff object (world-coord spawn + animate/draw Main — resident art, so
        // it queues no DMA in its whole life); dust_spindash.emp carries
        // Dust_Tick, the per-frame skid cadence Player_Display calls (Task 5
        // adds the charge-dust follower to the same section). Player-side game
        // content, placed right after tails_appendage per map.toml `order` —
        // tails_appendage's end anchor moves to DustPuff_Spawn.
        m!("games.sonic4.dust_puff", "dust_puff"),
        m!("games.sonic4.dust_spindash", "dust_spindash"),
        // ── Game objects ──
        // test_player + test_enemy fully flipped (conv-d #48/#47): both .asm deleted.
        // test_player.emp owns TPlayerV; test_animated.emp owns DplcV; STUB_FLOOR_Y
        // is object_test_state.emp's; ENEMY_PATROL_SPEED is test_objects.emp's.
        // objtest-gate (2026-08-05): the eight scene-only test objects moved to
        // the DEBUG-only block below. test_static + test_solid STAY — the shipped
        // OJZ entity data places both (Sec0/1/2), so they are live PLAIN content.
        m!("games.sonic4.test_static", "test_static"),
        m!("games.sonic4.test_solid", "test_solid"),
        m!("games.sonic4.path_swap", "path_swap"),
        // ── Game data ──
        // OBJDEFS: `module … .test_objects` has NO `in <section>`, so its
        // `pub data` lands in the default `"text"` section (verified: the only
        // reachable non-empty `"text"` producer — the sound data modules are
        // unreachable from this set).
        m!("games.sonic4.data.objdefs.test_objects", "text"),
        // The OJZ parallax block (conv-g): 6 deform tables + 20 parallax_config
        // records. RE-HOMED 2026-08-18 by scanline-P1: the block is no longer hand-authored
        // in games.sonic4.parallax_configs (deleted) — it is LOWERED from authored scenes
        // (games.sonic4.data.effects.ojz_scenes authors, engine.level.scene_dsl lowers,
        // games.sonic4.scene_registry emits). Same bytes at the same address: the migration
        // returned all four shapes to their pre-migration crcs and the 0xACE block at
        // $121C8 is byte-equal, so this row is a RENAME, not a re-measure.
        m!("games.sonic4.scene_registry", "scene_registry"),
        // Effects P3 Parcel C2 — the game-side effects library, carved out of
        // configs.emp's bottom half (gate fixtures, the five starter palette variants,
        // and the five OJZ presets).
        m!("games.sonic4.ojz_effects", "ojz_effects"),
        // test_mappings (conv-h #35): the test-object sprite mapping index
        // (Map_TestObj word-offset table + 3 frame records), authored via the
        // `offsets` construct in games.sonic4.data.mappings.test_mappings.
        m!("games.sonic4.test_mappings", "test_mappings"),
        // Dust sprite data (dust_data.emp, dust-effect Task 3): mappings x2, the
        // charge DPLC, and the 88-tile art blob whose tail 16 tiles are the
        // resident puff block. Placed right after test_mappings per map.toml
        // `order` (2816 B of art fits the data region's headroom, so unlike
        // Tails' 132 KB it needs no ROM-tail exile).
        m!("games.sonic4.dust_data", "dust_data"),
        m!("games.sonic4.sonic_anims", "sonic_anims"),
        // Character-dispatch C1 task 5: the Tails animation scripts
        // (tails_anims.emp) — `Ani_Tails` + `Ani_TailsAppendage`, both indexed by
        // the shared ANIM_* ids. A peer of `sonic_anims`, placed right after it
        // per map.toml `order`.
        m!("games.sonic4.tails_anims", "tails_anims"),
        // Character-dispatch C4 task 9: the Knuckles animation scripts
        // (knuckles_anims.emp) — `Ani_Knuckles` on the shared ANIM_* ids, the
        // third peer of sonic_anims / tails_anims and placed right after them per
        // map.toml `order`.
        m!("games.sonic4.knuckles_anims", "knuckles_anims"),
        // particle_anims: DEBUG-only below (sole consumer test_particle is).
        // Dust animation scripts (dust_anims.emp, dust-effect Task 3): the charge
        // loop + the puff one-shot. Sits right after particle_anims per map.toml
        // `order` (union; in plain, where Ani_Particle is absent, it follows
        // tails_anims directly). BOTH shapes — the dust objects are shipped
        // content, unlike the debug-only particle scripts.
        m!("games.sonic4.dust_anims", "dust_anims"),
        // Parcel K3 run A: the OJZ act1 interior island HEAD — the contiguous run
        // BEFORE the descriptor. Two native `.emp` sections (both generator-emitted):
        //   entity_data  — the 9-section type tables / object placements / ring lists
        //                  (objentry/objend replaced by packed 3-word records; the
        //                  last 2 macros.asm consumers gone)
        //   ojz_act_pool — 3 ZX0 page embeds + the OJZ_Act_Pool_PageTable
        // With these + the descriptor + the run-B tail native, act_descriptor.asm is
        // DELETED (the OJZ block is fully `.emp`).
        m!("games.sonic4.ojz_entity_data_act1", "entity_data"),
        m!("games.sonic4.ojz_act_pool_act1", "ojz_act_pool"),
        // act_descriptor (kill row 93): the OJZ act1 descriptor table; the body/
        // section table places here.
        m!("games.sonic4.act_descriptor_ojz_act1", "act_descriptor"),
        // Parcel K3 run B: the OJZ act1 interior island TAIL — the contiguous run
        // after the descriptor. Three native `.emp` sections (the generators emit
        // #32/#28; the palette/BG BINCLUDEs dissolved into act_assets.emp), placed
        // by contiguity after the descriptor:
        //   sec_block_blobs — OJZ_Sec{0..8}_Blocks (Sec4=Sec2 dedup equ), 8 embeds
        //   ojz_act_assets  — OJZ_Palette / BGND_Palette / OJZ_Act1_BG_{Layout,Tiles}
        //   ojz_bg_anim     — BgAnim_Table (disabled stub) + BgAnim_Banks
        m!("games.sonic4.ojz_sec_block_blobs_act1", "sec_block_blobs"),
        // art-streaming-p2-task5 — per-section local->global tile-index tables
        // (sec_local_maps.emp), placed after the block blobs per map.toml `order`.
        m!("games.sonic4.ojz_sec_local_maps_act1", "sec_local_maps"),
        m!("games.sonic4.ojz_act_assets_act1", "ojz_act_assets"),
        m!("games.sonic4.ojz_bg_anim_act1", "ojz_bg_anim"),
        // Parcel K4: the global collision + Sonic character data (HeightMaps ..
        // Art_Sonic), was the flat BINCLUDE island at the tail of main.asm's
        // gameDataIncludes. Native `embed()` section; boundary key HeightMaps.
        m!("games.sonic4.collision_data", "collision_data"),
        // Character-dispatch C1 task 5: the Tails sprite data (tails_data.emp) —
        // body + twin-tail appendage mappings/DPLC/art, a PEER of the Sonic trio
        // that rides `collision_data`, hence this registry slot. Its map.toml
        // `order` slot is NOT here though: 132 KB does not fit between Art_Sonic
        // and the $48000 dac_banks anchor, so the section is placed at the ROM
        // tail (map.toml carries the why). Registry position is organizational;
        // the map drives placement (K5).
        // Nothing consumes it yet (the roster still points CHAR_TAILS at the Sonic
        // record); it is the data half of the character split, landed first.
        m!("games.sonic4.tails_data", "tails_data"),
        // Character-dispatch C4 task 9: the Knuckles sprite data
        // (knuckles_data.emp) — mappings/DPLC/art plus the two CRAM line-0
        // palettes the per-character palette swap reads. Registry slot beside
        // `tails_data` because it is the same kind of thing; like Tails' it is
        // map-placed at the ROM TAIL, since the art does not fit between
        // Art_Sonic and the $48000 dac_banks anchor. Unlike Tails, Knuckles could
        // NOT be brought into our palette order by an index permutation — his art
        // uses an S3K colour our line 0 lacks — which is why this module ships a
        // palette at all.
        m!("games.sonic4.knuckles_data", "knuckles_data"),
        // Parcel K4 inc-5 Stage 2 (P2 DAC probe): the DAC sample banks are native —
        // dac_banks.emp embeds the seam-2 dac_blip_bank.bin @ $48000 + dac_shared_bank.bin
        // @ $50000 (the .bin ensure_generated emits). Sound-ON ONLY: filtered out of the
        // sound-off config_b (demo_registry already excludes it via the engine.* filter).
        m!("games.sonic4.dac_banks", "dac_banks"),
        // Parcel K4 inc-5 Stage 3 (P2 MT probe): the Moving-Trucks streaming bank body
        // is native — mt_bank_blob.emp embeds the seam-2 mt_bank{,_debug}.bin @ $58607
        // (after the phased soundBankHead; non-phased LMA labels). Sound-ON only.
        m!("games.sonic4.mt_bank_blob", "mt_bank_blob"),
        // Parcel K4 inc-5 Stage 4 (P2 SFX probe): the SFX block is native —
        // sfx_bank_blob.emp embeds the seam-2 sfx_bank{,_debug}.bin after the MT body
        // (non-phased LMA; no cross-seam labels). Sound-ON only.
        m!("games.sonic4.sfx_bank_blob", "sfx_bank_blob"),
        // Parcel K4 inc-5 Stage 4b (P2 soundBankHead probe): the engine-table bank HEAD
        // is native — soundbankhead.emp places the 5 heads as a PHASE-BANK section (vma
        // $8000, lma $58000). Was the soundBankHead macro (sound_bank.inc, deleted). The
        // FIRST native phase-bank section (the bank-anchor rule: labeled $8000-window
        // head, hard org, never repacks). Sound-ON only.
        m!("games.sonic4.soundbankhead", "soundbankhead"),
        // ── Game test states ──
        // object_test_state: DEBUG-only below (owner ruling 2026-08-05 — a
        // harness you drive is equipment, and equipment does not ship).
        m!("games.sonic4.ojz_scroll_test", "ojz_scroll_test"),
    ];
    if debug {
        specs.push(m!("engine.compression_selftest", "compression_selftest"));
        // objtest-gate (owner ruling 2026-08-05): the object-test scene and its
        // eight scene-only objects are DEBUG equipment (same idiom as
        // COMPRESSION_SELFTEST — no in-file gate, registry-only inclusion; each
        // module still emits its procs unconditionally, plain simply never links
        // them). TestStatic/TestSolid/Map_TestObj/objdefs(Static,Solid)/TestArt
        // remain unconditional above: shipped OJZ entity data and the release
        // debug-fly marker consume them.
        specs.push(m!("games.sonic4.test_player", "test_player"));
        specs.push(m!("games.sonic4.test_enemy", "test_enemy"));
        specs.push(m!("games.sonic4.test_animated", "test_animated"));
        specs.push(m!("games.sonic4.test_particle", "test_particle"));
        specs.push(m!("games.sonic4.test_emitter", "test_emitter"));
        specs.push(m!("games.sonic4.test_parent", "test_parent"));
        specs.push(m!("games.sonic4.test_stress_emitter", "test_stress_emitter"));
        specs.push(m!("games.sonic4.test_churn", "test_churn"));
        specs.push(m!("games.sonic4.particle_anims", "particle_anims"));
        specs.push(m!("games.sonic4.object_test_state", "object_test_state"));
    }
    if debug || crash_report {
        // The error_handler island (the 12 per-class CPU exception stubs + the
        // vendored MD Debugger v2.6 blob, ~4.2 KB). Owner-ruled 2026-08-04: this is
        // a DIAGNOSTIC, so it ships in BOTH canonical shapes — a player's crash has
        // to be reportable. Only the opt-in `lean` profile (crash_report = false)
        // omits it. Placed at its map-order slot (BusError), which must remain the
        // FINAL byte-emitting section (see `append_deb2_appendix`'s blob-end guard).
        specs.push(m!("engine.debug.error_handler", "error_handler"));
    } else {
        // The LEAN loud-failure handler (46 B: mask, display off, red backdrop,
        // freeze). It replaces the absent error_handler island as every fault
        // vector's target in the lean shape, at the same tail placement slot.
        // LEAN-ONLY — so it appears in NEITHER canonical listing, which is why it
        // has no `repin` region (repin can only resolve the canonical plain+debug
        // listings). The chainer sizes and places it live from `lean.txt` + map.toml
        // `order`.
        specs.push(m!("engine.system.release_fault", "release_fault"));
    }
    // I4: the OJZ replay fixture — pushed last in the REGISTRY, but map.toml's `order`
    // places it after all gameplay content and BEFORE the fault-handler island, which
    // the MDDBG blob-end contract requires to be the final byte-emitting section (see
    // `check_error_handler_is_last`). Re-recording (content+size change) therefore still
    // shifts zero gameplay addresses; it moves only the fault handler + EndOfRom/appendix.
    specs.push(m!("games.sonic4.replay_fixture", "replay_fixture"));
    specs
}

/// The engine-only registry (demo): the `engine.*` modules of the sonic4 registry,
/// minus `engine.sound_api` (demo is sound-OFF → the sound-caller `.asm`/`.emp` is
/// not in the demo layout at all). The chainer sources demo sizes from the frozen
/// listing table.
///
/// OWNER-RULED 2026-08-04: the demo's RELEASE shape CARRIES the debugger — no new
/// exclusion. `engine.debug.error_handler` is an `engine.*` module, so it rides the
/// existing prefix filter exactly like every other engine module.
fn demo_registry(debug: bool, crash_report: bool) -> Vec<ModuleSpec> {
    let mut r: Vec<ModuleSpec> = registry(debug, crash_report)
        .into_iter()
        .filter(|m| m.module_id.starts_with("engine.") && m.module_id != "engine.sound_api")
        .collect();
    // The Z80 idle places natively in the no-sound demo (kill row 55); `z80_init`
    // is not in the shared `registry()` (sound-on shapes must not place it), so add
    // it here explicitly.
    r.push(ModuleSpec { module_id: "engine.z80_init", section: "z80_idle" });
    // The demo GAME modules (Parcel H-demo): the object-code-bank island the demo
    // main.asm holes out. `demo_data` lands its `pub data` in the named `demo_data`
    // section (not the default `text`), so the require_one_text guard stays off.
    // The chainer packs them from the frozen demo tables'
    // DemoBox_Main/ObjDef_DemoBox/GameState_Demo_Init anchors.
    r.push(ModuleSpec { module_id: "games.demo.demo_box", section: "demo_box" });
    r.push(ModuleSpec { module_id: "games.demo.data.demo_data", section: "demo_data" });
    r.push(ModuleSpec { module_id: "games.demo.demo_state", section: "demo_state" });
    // Parcel K4: the demo's $100-$1FF ROM header is native too (games.demo.header;
    // the shared engine.inc no longer invokes the gameHeader macro). Boundary key
    // Checksum→GameHeader.
    r.push(ModuleSpec { module_id: "games.demo.header", section: "header" });
    r
}

/// CANONICAL sonic4 (the Stage-1 shape) as a profile — the regression harness for the
/// GameProfile refactor: `native_rom` / `native_declared_chain` / `native_full_rom`
/// build through this and must stay byte-green.
pub fn sonic4_profile(debug: bool) -> GameProfile {
    GameProfile {
        name: if debug { "sonic4_debug" } else { "sonic4" },
        game_root_rel: "games/sonic4/game_root.asm",
        game_ram_module: "games.sonic4.ram",
        manifest_module: "games.sonic4.game",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug,
        crash_report: true,
        sound_on: true,
        extra_as_defines: vec![],
        registry: registry(debug, true),
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 1),
            ("DEBUG", if debug { 1 } else { 0 }),
            ("CRASH_REPORT", 1),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            // The game ships an act-wide ZX0 art pool (OJZ) — gates the DEBUG
            // CompressionSelfTest act-pool ZX0R equivalence walk (engine module,
            // demo-shared: demo has no pool, so it defines this 0).
            ("HAS_ACT_ART_POOL", 1),
            // Forced-eviction dev fixture (Art-streaming P2b Task 7): 0 in every shipped
            // shape (PAGE_FRAMES_CLAMP == PAGE_FRAMES, byte-inert); 1 only in the
            // off-canonical `stress_evict` profile.
            ("STRESS_EVICT", 0),
            // sonic4 game-config (games/sonic4/config/constants.asm); the engine
            // `.emp` reads these as -D (rings.emp / entity_window.emp), the
            // `ensure(extern(..)==..)` cross-checks them against the AS config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        frozen_sizes: load_frozen_table(if debug {
            "s4_debug.txt"
        } else {
            "s4.txt"
        }),
        fixture_placement: false,
        extra_entries: Vec::new(),
    }
}

/// DEMO (plain / debug) — engine-only registry, sound OFF, sizes from the frozen
/// `demo.txt` / `demo_debug.txt`. GAME_CAMERA_JUMP_LOCK=0 (demo's config selects the
/// inert camera path). No objdefs → the one-text guard is OFF.
pub fn demo_profile(debug: bool) -> GameProfile {
    GameProfile {
        name: if debug { "demo_debug" } else { "demo" },
        game_root_rel: "games/demo/game_root.asm",
        game_ram_module: "games.demo.ram",
        manifest_module: "games.demo.game",
        // Conversion Parcel H-demo (#14): the demo's game config is `.emp`-authored
        // (`games.demo.constants` = games/demo/config/constants.emp), harvested into
        // guarded AS `-D` + link EquSyms exactly like the sonic4 side.
        game_constants_rel: Some("games/demo/config/constants.emp"),
        game_sound_ids_rel: None,
        game_sfx_bank_rel: None,
        debug,
        crash_report: true,
        sound_on: false,
        extra_as_defines: vec![],
        registry: demo_registry(debug, true),
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 0),
            ("DEBUG", if debug { 1 } else { 0 }),
            ("CRASH_REPORT", 1),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            ("HAS_ACT_ART_POOL", 0),   // demo ships no act art pool (skips the ZX0R act-pool selftest walk)
            ("STRESS_EVICT", 0),       // dev-fixture define (Task 7); byte-inert at 0
            // demo game-config (games/demo/config/constants.emp) engine-VARYING
            // interface values — homed here (not the `.emp`) per the `-D`-not-in-
            // `.emp` rule; the values that DIFFER from sonic4.
            ("MAX_RING_BUFFER", 16),
            ("VRAM_RING_PLACEHOLDER", 0x3E4),
            ("COLLECTED_WINDOW_SLOTS", 4),
        ],
        require_one_text: false,
        inapplicable_guards: DEMO_INAPPLICABLE_GUARDS.to_vec(),
        frozen_sizes: load_frozen_table(if debug {
            "demo_debug.txt"
        } else {
            "demo.txt"
        }),
        fixture_placement: false,
        extra_entries: Vec::new(),
    }
}

/// CONFIG-B (off-canonical no-sound): sonic4 game, SOUND_DRIVER_ENABLED OFF, plain.
/// Registry = the sonic4 set MINUS `engine.sound_api` (no sound caller) PLUS the Z80
/// idle (kill row 55): with `SIGIL_EMP_Z80_INIT` on, boot_data.asm's no-sound else-arm
/// takes the numeric-size path and `z80_init.emp`'s `z80_idle` section places at the
/// frozen `Z80_IdleProgram` base (0x3d8). Sizes from `config_b.txt`.
pub fn config_b_profile() -> GameProfile {
    // Config-B is SOUND-OFF: drop the sound caller AND the sound-on-only DAC banks
    // (dac_banks.emp; its .bin are emitted only in sound-on builds — ensure_generated).
    let mut registry: Vec<ModuleSpec> = registry(false, true)
        .into_iter()
        .filter(|m| {
            m.module_id != "engine.sound_api"
                && m.module_id != "games.sonic4.dac_banks"
                && m.module_id != "games.sonic4.mt_bank_blob"
                && m.module_id != "games.sonic4.sfx_bank_blob"
                && m.module_id != "games.sonic4.soundbankhead"
        })
        .collect();
    registry.push(ModuleSpec {
        module_id: "engine.z80_init",
        section: "z80_idle",
    });
    GameProfile {
        name: "config_b",
        game_root_rel: "games/sonic4/game_root.asm",
        game_ram_module: "games.sonic4.ram",
        manifest_module: "games.sonic4.game",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug: false,
        // Config-B follows the RELEASE default (owner-ruled): it carries the debugger.
        crash_report: true,
        sound_on: false,
        extra_as_defines: vec![],
        registry,
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 0),
            ("DEBUG", 0),
            ("CRASH_REPORT", 1),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            ("HAS_ACT_ART_POOL", 1),
            ("STRESS_EVICT", 0),       // dev-fixture define (Task 7); byte-inert at 0
            // Config-B is the sonic4 game (sound off), so sonic4's game-config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        frozen_sizes: load_frozen_table("config_b.txt"),
        fixture_placement: false,
        extra_entries: Vec::new(),
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
    let mut registry = registry(true, true);
    registry.push(ModuleSpec {
        module_id: "games.sonic4.game_debug",
        section: "game_debug",
    });
    registry.push(ModuleSpec {
        module_id: "engine.debug.sound_debug",
        section: "sound_debug",
    });
    GameProfile {
        name: "config_a",
        game_root_rel: "games/sonic4/game_root.asm",
        game_ram_module: "games.sonic4.ram",
        manifest_module: "games.sonic4.game",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug: true,
        crash_report: true,
        sound_on: true,
        extra_as_defines: vec![("SOUND_DEBUG_HOTKEYS", 1), ("SOUND_DBG_MIRROR", 1)],
        registry,
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 1),
            ("DEBUG", 1),
            ("CRASH_REPORT", 1),
            ("SOUND_DEBUG_HOTKEYS", 1),
            ("SOUND_DBG_MIRROR", 1),
            ("HAS_ACT_ART_POOL", 1),
            ("STRESS_EVICT", 0),       // dev-fixture define (Task 7); byte-inert at 0
            // Config-A is the sonic4 game (debug + sound), so sonic4's game-config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        frozen_sizes: load_frozen_table("config_a.txt"),
        fixture_placement: false,
        extra_entries: Vec::new(),
    }
}

/// LEAN (off-canonical, the 7th profile): the sonic4 release shape with the crash-report
/// axis OFF. `debug: false, crash_report: false, sound_on: true` — everything the shipped
/// release ROM has, MINUS the MD Debugger / `error_handler` island and its deb2 symbol
/// appendix; every fault vector routes at `ReleaseFault` instead (release_fault.emp, the
/// only profile that places it).
///
/// It exists because "no debugger at all" is a real shape someone will want, and the
/// engine-debts rule is that an unbuilt shape is an untested shape: making lean a full
/// gated profile with its own golden + frozen size table keeps `CRASH_REPORT == 0`
/// honest. It is NOT reachable from `build.sh` (which refuses `CRASH_REPORT=0` and points
/// here) — `sigil build --aeon . --native --lean`.
///
/// It starts byte-identical to the pre-ruling release ROM, since that ROM was exactly
/// this shape; the crash-report plumbing must be inert at `CRASH_REPORT=0` or the
/// `lean.bin` golden moves.
pub fn lean_profile() -> GameProfile {
    GameProfile {
        name: "lean",
        game_root_rel: "games/sonic4/game_root.asm",
        game_ram_module: "games.sonic4.ram",
        manifest_module: "games.sonic4.game",
        game_constants_rel: Some("games/sonic4/config/constants.emp"),
        game_sound_ids_rel: Some("games/sonic4/config/sound_ids.emp"),
        game_sfx_bank_rel: Some("games/sonic4/data/sound/sfx/sfx_bank.emp"),
        debug: false,
        crash_report: false,
        sound_on: true,
        extra_as_defines: vec![],
        registry: registry(false, false),
        emp_defines: vec![
            ("SOUND_DRIVER_ENABLED", 1),
            ("DEBUG", 0),
            // THE one profile with CRASH_REPORT off — vectors.emp's fault cells take the
            // ReleaseFault arm, and no `__MDDBG__` is pushed to the AS side.
            ("CRASH_REPORT", 0),
            ("SOUND_DEBUG_HOTKEYS", 0),
            ("SOUND_DBG_MIRROR", 0),
            ("HAS_ACT_ART_POOL", 1),
            ("STRESS_EVICT", 0),       // dev-fixture define (Task 7); byte-inert at 0
            // Lean is the sonic4 game, so sonic4's game-config.
            ("MAX_RING_BUFFER", 128),
            ("VRAM_RING_PLACEHOLDER", 0x3E8),
            ("COLLECTED_WINDOW_SLOTS", 9),
        ],
        require_one_text: true,
        inapplicable_guards: STAGE1_INAPPLICABLE_GUARDS.to_vec(),
        frozen_sizes: load_frozen_table("lean.txt"),
        fixture_placement: false,
        extra_entries: Vec::new(),
    }
}

/// STRESS_EVICT (off-canonical DEV shape, Art-streaming P2b Task 7): the sonic4 DEBUG
/// profile with the `STRESS_EVICT` comptime define flipped 0 -> 1. That clamps the
/// residency cache's usable frame count (`PAGE_FRAMES_CLAMP`) below the OJZ pool size,
/// so the pool can NO LONGER be fully resident and every circuit forces continuous
/// evict/reload traffic through the page cache — the fixture the controller's
/// forced-eviction soak drives.
///
/// It is DELIBERATELY UNFROZEN: no `golden/stress_evict.bin`, no `provenance.toml`
/// entry, not a `refreeze`/`shipped_shapes` target. The clamp is an immediate operand,
/// not a size change, so the DEBUG frozen size table (`s4_debug.txt`) resolves it
/// exactly — the chainer builds it with no new table. Everything else (registry,
/// crash-report island, sound) is identical to `sonic4_profile(true)`, so nothing but
/// the residency-cache behaviour differs.
pub fn stress_evict_profile() -> GameProfile {
    let mut p = sonic4_profile(true);
    p.name = "stress_evict";
    for d in p.emp_defines.iter_mut() {
        if d.0 == "STRESS_EVICT" {
            d.1 = 1;
        }
    }
    p
}

/// STRESS_ART (Art-streaming P2c Task 11 stress fixture): sonic4 DEBUG built against a
/// UNIQUIFIED act art pool (`ojz_strip_gen --stress-uniquify`, 41 pages) that inflates the
/// `ojz_act_pool` section by tens of KB — far past the packed-placement spread step, so the
/// canonical `Frozen` placement fails with a "hand ruling" collision. Unlike `stress_evict`
/// (an immediate-operand clamp the frozen size table resolves exactly), this fixture CHANGES
/// SECTION SIZES, so it opts into `fixture_placement`: greedy pack from measured sizes, no
/// frozen provisional-base overrun / island-reclass guard. The org-anchored islands (object
/// bank $10000, DAC/sound phase banks $48000/$58000, error_handler-last) are UNCHANGED.
///
/// DELIBERATELY UNFROZEN like `stress_evict`: no golden, no `provenance.toml` entry, not a
/// `refreeze`/`shipped_shapes` target, and NOT in `shape_defines`/`shipped_shapes` (its
/// placement waiver must never gate or describe a shipped shape). Everything else matches
/// `sonic4_profile(true)`; the pool inflation is a build-time data swap, not a define.
pub fn stress_art_profile() -> GameProfile {
    let mut p = sonic4_profile(true);
    p.name = "stress_art";
    p.fixture_placement = true;
    p
}

/// EVERY SHIPPED SHAPE, in `capture_goldens.sh` order — the one table a gate meaning
/// "all shipped shapes" reads, so such a gate cannot drift from the shapes the byte
/// bar actually builds. The other target tables in the tree (`BuildTarget::
/// label_and_profile`, `derive_offcanon`, `refreeze`, the three off-canonical byte
/// gates) select ONE artifact each rather than enumerating the set, and are tracked
/// on their own gap-ledger row.
///
/// The labels are the ones the golden capture and the `--report` header already use.
/// The set is exhaustive over both polarities of every comptime toggle the profiles
/// carry — `SOUND_DRIVER_ENABLED` (1 in sonic4/config_a/lean, 0 in demo/config_b),
/// `DEBUG` (1 in the three debug shapes), `CRASH_REPORT` (0 only in lean),
/// `SOUND_DEBUG_HOTKEYS`/`SOUND_DBG_MIRROR` (1 only in config_a) — which is what
/// makes "empty under every shape" a statement about the whole corpus rather than
/// about whichever arms one define set happens to keep.
pub fn shipped_shapes() -> Vec<(&'static str, GameProfile)> {
    vec![
        ("sonic4 plain", sonic4_profile(false)),
        ("sonic4 debug", sonic4_profile(true)),
        ("demo plain", demo_profile(false)),
        ("demo debug", demo_profile(true)),
        ("config_a", config_a_profile()),
        ("config_b", config_b_profile()),
        ("lean", lean_profile()),
    ]
}

/// The comptime `-D` set a shape's `.emp` sources are read under: the shipping
/// profile's built-in `emp_defines` rows merged with the game's own
/// `games/<g>/map.toml` `[defines]` rows (see [`crate::game_defines`]). Owned by
/// the profile + the game's config so an analysis can never describe a shape the
/// build does not make. `--report contracts`, the corpus gates, and the `.emp`
/// build/harvest paths all read THIS merge, so a gate's walk and the build's
/// walk are the same walk — a game-declared row is visible to every consumer or
/// to none.
///
/// A game row whose key matches a built-in row is a loud error in both
/// directions (neither source silently wins); a duplicated key inside the table
/// is a loud error naming both rows.
///
/// An ABSENT `games/<g>/map.toml` is an ERROR, naming the file. The map is where
/// a game homes its define rows, so its absence is a missing config rather than
/// an empty one — tolerating it would return a built-ins-only env and let the
/// shape walk define-complete but game-row-free, with the eventual symptom
/// (`unknown name X` at lower time, or an un-gated arm in a corpus walk) pointing
/// at the `.emp` consumer instead of at the missing file. There is no waiver: a
/// synthetic fixture tree supplies the file, and a map that EXISTS with no
/// `[defines]` table contributes no game rows — the byte-neutral state every
/// shipped map is in today. Any other read failure is loud.
pub fn shape_defines(profile: &GameProfile, aeon: &Path) -> Result<Vec<(String, i128)>, String> {
    let map_path = profile.map_path(aeon);
    let origin = map_path.display().to_string();
    let game_rows = match std::fs::read_to_string(&map_path) {
        Ok(src) => crate::game_defines::parse_game_defines(&src, &origin)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "shape `{}`: no game config at {origin} — a game homes its `[defines]` \
                 rows there, so a missing or renamed map is a MISSING config, not an \
                 empty one, and no shape may walk with the built-in rows alone. A \
                 synthetic fixture tree supplies the file; a map with no `[defines]` \
                 table declares no game rows, which is what every shipped map does today",
                profile.name
            ));
        }
        Err(e) => return Err(format!("read {origin}: {e}")),
    };
    crate::game_defines::merge_builtin_and_game(&profile.emp_defines, game_rows, &origin)
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
///
/// The output directory is INSIDE `aeon`, so this is a WRITE into the reference
/// tree. It refuses up front when that tree is not there
/// ([`seam2::require_reference_tree`]) rather than creating the path and failing
/// afterwards: a manufactured `$AEON_DIR` root turns every root-probing skip guard
/// in the suite into a run against an empty tree, which makes an absent-tree run
/// behave differently the second time than the first. Each emitter re-checks the
/// same precondition, so the `emit_sound_blob` binary and direct callers get the
/// refusal too.
pub fn ensure_generated(aeon: &Path) {
    seam2::require_reference_tree(aeon)
        .unwrap_or_else(|e| panic!("ensure_generated writes into the reference tree: {e}"));
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
    // STRESS_EVICT (Art-streaming Task-7 dev-fixture define) must be seeded: the new
    // PAGE_FRAMES_CLAMP pub const references it, and eval_all_pub_consts folds EVERY
    // pub const. This harvest is shape-agnostic and feeds the AS -D side only; the
    // .emp build takes STRESS_EVICT from the shape_defines merge (where the
    // profile's built-in rows carry it), so the harvested PAGE_FRAMES_CLAMP
    // (== PAGE_FRAMES at the seed 0) is an unused AS define.
    let (vals, diags) = sigil_frontend_emp::eval::eval_all_pub_consts(
        &file,
        Some(aeon),
        &[("STRESS_EVICT".to_string(), 0)],
    );
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
/// (the custom window's tail word) — mirrors sst.emp's `interact_off()`:
/// back to `sizeof(Sst) - 2` since the sst-fold parcel (2026-08-05) moved
/// frame_off into the engine block at $2E, making the custom window the
/// record's tail again (bug005 had it at `frame_off - 2` while the cache
/// was the tail).
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
        // derived `=` outside a struct block). Mirrors sst.emp's `interact_off()`
        // = offsetof(Sst, frame_off) - 2 (bug005: frame_off is the record tail
        // now, so the custom window's tail word sits immediately below it).
        if *sname == "Sst" {
            // sst-fold: interact = the record's tail word. Keep the frame_off
            // presence check — a struct that lost the cache entirely is a
            // different (loud) problem than one that moved it.
            assert!(
                layout.fields.iter().any(|f| f.name == "frame_off"),
                "harvest_engine_struct_offsets: Sst lost frame_off — \
                 re-derive SST_interact against sst.emp's interact_off()"
            );
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
///
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
/// path, one comptime env = the [`shape_defines`] merge), never merely tested.
/// Shape-specific: the merge carries `DEBUG` (0/1), so the debug prof block's
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
    assert!(
        !manifest.sources.contains_key(&source),
        "ram-harvest entry {source:?} collides with a scanned module"
    );
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

    let defines = shape_defines(profile, aeon)?;
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

pub fn assemble_as_side(aeon: &Path, profile: &GameProfile) -> Result<AsSide, String> {
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
        for g in ["SIGIL_EMP_DAC", "SIGIL_EMP_MT", "SIGIL_EMP_SFX", "SIGIL_EMP_SOUNDBANKHEAD"] {
            defines.push((g.to_string(), 1));
        }
    }
    for (k, v) in &profile.extra_as_defines {
        defines.push((k.to_string(), *v));
    }
    if profile.debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    // The CRASH-REPORT axis on the AS side (owner-ruled 2026-08-04). `__DEBUG__` keeps
    // meaning exactly "debug shape"; `__MDDBG__` means "this target places the
    // error_handler island", which is `debug || crash_report` — i.e. both canonical
    // shapes, and not `lean`. Each game_root.asm gates its `include
    // engine/debug/debugger.asm` on it: that include's MDDBG__* equ table derives off
    // error_handler.emp's `pub equ`s, so including it without the island is a hard link
    // error. PUSH-OR-OMIT, never `=0`: AS `ifdef` tests DEFINEDNESS, not value.
    if profile.debug || profile.crash_report {
        defines.push(("__MDDBG__".to_string(), 1));
    }
    let opts = AsOptions {
        initial_cpu: Some(Cpu::M68000),
        defines,
        include_root: Some(aeon.to_path_buf()),
        guarded_defines,
    };
    // Every build CHAINS: sections move after assembly, so the residual AS must keep
    // section-label references SYMBOLIC to relocate (the row-94 parallax pointer).
    assemble_root_relocating_warned(&root, &opts)
        .map(|a| AsSide {
            warnings: a.warnings.iter().map(|d| BuildWarning::from_as(d, &a.sources)).collect(),
            module: a.module,
        })
        .map_err(|f| {
            format!(
                "assemble (native AS side, {}): {} diagnostics; first: {:?}",
                profile.name,
                f.diags.len(),
                f.diags.first()
            )
        })
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
///
/// `extra_entries` are dotted ids appended to the same edge list, which is the WHOLE
/// mechanism behind `--extra-entry`: reachability is what makes a module's
/// module-level `ensure`s evaluate, and this file is where reachability is declared.
/// The edges are bare whole-module `use`s like every other line here — the shape
/// `[import.no-names]` warns about — and that is safe for exactly one reason: this
/// module's own diagnostics never reach the build report (`collect_warnings` filters
/// its `SourceId`, held by `warn_tier_corpus`'s
/// `the_generated_entry_module_is_not_reported`). An extra entry's own diagnostics
/// carry ITS source, so they report normally.
fn synthetic_entry_src(
    specs: &[ModuleSpec],
    game_ram_module: &str,
    manifest_module: &str,
    extra_entries: &[String],
) -> String {
    let mut src = String::from("module native_flip_entry\n\n");
    // L1 P2: the engine's Game contract (the interface — pure declaration) and
    // the game's one manifest (the `implement`). Both emit zero bytes and neither
    // is a placed registry module, but both MUST be reachable so the whole-program
    // bind pass collects the interface + its implement (an unreachable interface
    // would leave the implement unmatched; an unreachable implement would leave
    // the interface `[contract.unimplemented]`).
    src.push_str("use engine.game_contract\n");
    src.push_str(&format!("use {manifest_module}\n"));
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
    for id in extra_entries {
        src.push_str(&format!("use {id}\n"));
    }
    src
}

/// Resolve one `--extra-entry` argument to a manifest module id — an author names a
/// poison the way it is on disk, and a lane names it the way the module declares
/// itself, so both spellings resolve. PRECEDENCE, in order: a dotted module id wins;
/// then the argument as a path under the scan root (`<aeon>/<arg>`); then the argument
/// as a path in its own right (cwd-relative or absolute). So an argument that is both
/// a declared module id and a relative filename resolves as the id.
///
/// A name that resolves to nothing is an ERROR, never a skip: a lane whose subject
/// vanished (renamed, moved, deleted) must fail loudly rather than pass vacuously.
/// A file that failed to PARSE does not reach here: `Manifest::scan` registers such a
/// module anyway, but it raises an Error diagnostic doing so, and `build_emp` bails on
/// the scan's errors before any extra entry is resolved.
fn resolve_extra_entry(
    manifest: &resolve::manifest::Manifest,
    aeon: &Path,
    arg: &str,
) -> Result<String, String> {
    if manifest.by_id.contains_key(arg) {
        return Ok(arg.to_string());
    }

    // Path form: compare canonicalized paths against the scanned set, so `./a/b.emp`,
    // `a/b.emp` and an absolute path all name the same module.
    let candidates = [aeon.join(arg), std::path::PathBuf::from(arg)];
    for cand in candidates.iter().filter_map(|p| std::fs::canonicalize(p).ok()) {
        for pm in &manifest.modules {
            if std::fs::canonicalize(&pm.path).is_ok_and(|p| p == cand) {
                // `ParsedModule::id` is the key `by_id` (and so the reachability BFS)
                // is built on — the module's DECLARED id, which a `[module.path-mismatch]`
                // module spells differently from its path.
                return Ok(pm.id.clone());
            }
        }
    }

    Err(format!(
        "--extra-entry `{arg}`: no such module under the scan root {} — it names \
         neither a scanned module id nor a scanned `.emp` file path",
        aeon.display()
    ))
}

/// Refuse an extra entry that would CONTRIBUTE to the artifact. `--extra-entry`
/// exists to run a module's comptime guards inside the real build profile, and it is
/// byte-neutral by contract: a module reached only this way must contribute nothing.
/// A module NAMED to the flag that declared bytes would place into a shipping region
/// (or chain onto the RAM map and move every address after it) and silently change
/// the build, so it is refused by name here rather than emitted.
///
/// The refused kinds are the ROM-byte producers (`section`/`data`/`proc`/`table`/
/// `offsets`/`dispatch`/`script`/`align`), `equ` (which mints a link symbol), and
/// whatever the compiler's own RAM-allocator predicate
/// ([`file_declares_region`](sigil_frontend_emp::lower::file_declares_region))
/// recognizes — called rather than restated, so the refusal tracks the allocator if
/// that predicate grows a case.
///
/// Everything else an `.emp` module can declare folds to zero bytes and zero link
/// symbols, the line `publicize_helper_comptime` draws. `implement`/`interface` are
/// the one accepted kind that is not purely local: an extra entry carrying either
/// joins the whole-program contract bind pass. That cannot drift the artifact — a
/// second `implement` for a bound interface, or a second `interface` of one name, is a
/// hard error in [`contract::bind`](sigil_frontend_emp::resolve::contract) — so it
/// fails loudly rather than rebinding a member.
///
/// LIMITATION, stated because the flag's promise is otherwise flat: this check covers
/// the NAMED module ONLY. The named module's own `use` edges are ordinary imports, and
/// nothing re-runs this refusal over them — importing a module the profile already
/// reaches costs nothing, but an extra entry that imports a byte-emitting module from
/// OUTSIDE the closure does pull it in. What catches that is the placement layer (an
/// unmapped section name fails `map.toml` placement) and the golden CRC gates, not
/// this function.
fn refuse_artifact_contribution(
    manifest: &resolve::manifest::Manifest,
    module_id: &str,
    arg: &str,
) -> Result<(), String> {
    use sigil_frontend_emp::ast;
    let idx = manifest.by_id[module_id];
    let file = &manifest.modules[idx].file;
    let emitter = file
        .items
        .iter()
        .find_map(|item| match item {
            ast::Item::Section(_) => Some(("section", "emits into the ROM")),
            ast::Item::Data(_) => Some(("data", "emits into the ROM")),
            ast::Item::Proc(_) => Some(("proc", "emits into the ROM")),
            ast::Item::Table(_) => Some(("table", "emits into the ROM")),
            ast::Item::Offsets(_) => Some(("offsets", "emits into the ROM")),
            ast::Item::Dispatch(_) => Some(("dispatch", "emits into the ROM")),
            ast::Item::Script(_) => Some(("script", "emits into the ROM")),
            ast::Item::Align(_) => Some(("align", "emits into the ROM")),
            ast::Item::Equ(_) => Some(("equ", "mints a link symbol")),
            _ => None,
        })
        // RAM allocation is the compiler's predicate to define. The label below is
        // only for the message — a kind the predicate grows and this scan does not
        // recognize still refuses, under its generic name.
        .or_else(|| {
            if !sigil_frontend_emp::lower::file_declares_region(file) {
                return None;
            }
            let kind = file
                .items
                .iter()
                .find_map(|item| match item {
                    ast::Item::Region(_) => Some("region"),
                    // The overlay form (`vars Name: window { … }`) is a comptime view
                    // over existing bytes; the region form (no name) allocates.
                    ast::Item::Vars(v) if v.name.is_none() => Some("vars"),
                    _ => None,
                })
                .unwrap_or("a RAM allocator");
            Some((kind, "allocates RAM"))
        });
    match emitter {
        None => Ok(()),
        Some((kind, effect)) => Err(format!(
            "--extra-entry `{arg}`: module `{module_id}` declares `{kind}`, which {effect}. \
             --extra-entry evaluates a module's comptime guards inside the real build \
             profile and is byte-neutral by contract; a module that contributes to the \
             artifact must enter the build through the registry and a `use` edge instead."
        )),
    }
}

/// A non-error diagnostic the `.emp` build produced, with its source location
/// already resolved.
///
/// The [`Manifest`](sigil_frontend_emp::resolve::manifest::Manifest) that owns the
/// source texts lives and dies inside [`build_emp`], so the `path:line:col` is
/// rendered where it is knowable and travels with the diagnostic. `Warning` and
/// `Note` both ride this channel — errors abort the build instead.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildWarning {
    /// Severity: [`Level::Warning`](sigil_span::Level::Warning) or
    /// [`Level::Note`](sigil_span::Level::Note).
    pub level: sigil_span::Level,
    /// The bracketed lint id (`proc.sr-undeclared`), or the empty string for a
    /// diagnostic whose message carries no `[id]` prefix.
    pub id: String,
    /// `path:line:col` of the primary span, or `None` for a span with no source file.
    pub location: Option<String>,
    /// The full message, `[id]` prefix included.
    pub message: String,
    /// The primary span, kept so a consumer that needs more than the rendered
    /// location (a caret, an editor jump, a `-Werror` promotion) has it.
    pub primary: sigil_span::Span,
}

impl BuildWarning {
    /// Pair `d` with its location. The lint id is the leading `[...]` group, which
    /// is the corpus convention for every classified diagnostic.
    fn new(
        d: &sigil_span::Diagnostic,
        index: &sigil_frontend_emp::resolve::manifest::SourceIndex,
    ) -> BuildWarning {
        let id = d
            .message
            .strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .map(|(id, _)| id.to_string())
            .unwrap_or_default();
        BuildWarning {
            level: d.level,
            id,
            location: index.locate(d.primary),
            message: d.message.clone(),
            primary: d.primary,
        }
    }

    /// The same pairing for a diagnostic raised by the AS front end, located
    /// through ITS [`SourceMap`](sigil_span::SourceMap).
    ///
    /// A separate constructor rather than a second [`collect_warnings`] call,
    /// because the two location authorities are NOT interchangeable: a
    /// [`SourceIndex`](sigil_frontend_emp::resolve::manifest::SourceIndex) keys
    /// paths by `SourceId` from the `.emp` manifest, and an AS span's id counts
    /// the AS root and its `include` splices. Feeding one map's span to the
    /// other's index does not fail — it names a DIFFERENT FILE, confidently.
    fn from_as(d: &sigil_span::Diagnostic, sources: &sigil_span::SourceMap) -> BuildWarning {
        let id = d
            .message
            .strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .map(|(id, _)| id.to_string())
            .unwrap_or_default();
        BuildWarning {
            level: d.level,
            id,
            location: sources.label(d.primary),
            message: d.message.clone(),
            primary: d.primary,
        }
    }
}

/// A finished AS-side assembly: its sections plus any warn-tier diagnostic the
/// residual AS source raised (an author-written `warning` directive), already
/// located against the AS front end's own source map.
///
/// The warnings ride out of here rather than being dropped at the seam because
/// the whole point of the warn tier is that the build SUCCEEDS and still says
/// something. Aeon writes no `warning` today (measured 2026-09-04: zero
/// directive-column sites across its residual `.asm` files), so this vector is
/// empty on every shipped shape — which makes the first one anybody adds
/// visible in the build's tally line the day it is written, instead of silent.
pub struct AsSide {
    pub module: Module,
    pub warnings: Vec<BuildWarning>,
}

impl std::fmt::Display for BuildWarning {
    /// `path:line:col: <level>: <message>` — the shape
    /// `render_program_diags` gives errors, so both tiers read as one system.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "{loc}: {}: {}", self.level, self.message),
            None => write!(f, "{}: {}", self.level, self.message),
        }
    }
}

/// The placed `.emp` program: its sections, its deferred link asserts (drift
/// guards), and every non-error diagnostic the lowering produced.
pub struct EmpProgram {
    /// Every placed `.emp` section.
    pub sections: Vec<Section>,
    /// The whole program's deferred link asserts (drift guards).
    pub link_asserts: Vec<sigil_ir::LinkAssert>,
    /// Warnings and notes from all four lowering diagnostic sources (manifest scan,
    /// entry parse, program build, placement), deduplicated and location-resolved.
    pub warnings: Vec<BuildWarning>,
    /// The location authority these warnings resolved through. Handed out because
    /// the LINK tier produces warn-tier diagnostics of its own — a `Level::Warning`
    /// [`LinkAssert`](sigil_ir::LinkAssert) such as `[layout.odd-item]` fails at
    /// link time, long after the manifest that owns the source texts is gone — and
    /// they must render through the same index rather than a second one.
    pub sources: sigil_frontend_emp::resolve::manifest::SourceIndex,
}

/// Natively lower + place every registry `.emp` module for `profile`. Returns the
/// placed sections, the whole program's deferred link asserts (drift guards), and
/// every reportable non-error diagnostic the lowering produced. The placement map
/// bases are COSMETIC (the chainer recomputes every base from the frozen table), so a
/// nominal one-region-per-section map suffices.
pub fn build_emp(aeon: &Path, profile: &GameProfile) -> Result<EmpProgram, String> {
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
        // Effects Phase 3 (Parcel A): the raster + palette authoring vocabularies.
        // Pure comptime, no `in <section>`, so neither takes a registry entry, a pins
        // region, a `map.toml` slot, nor frozen-table rows — see the doc comment above.
        // Order matters: `normalize_helper_imports` prepends one glob per helper in
        // LIST order and the later helper silently wins a duplicate name, which is why
        // `tools/emp_helper_closure.py` gates the set for disjointness.
        "engine.effects.palette_dsl",
        "engine.effects.raster_dsl",
    ];
    publicize_helper_comptime(&mut manifest, COMPTIME_HELPERS);
    normalize_helper_imports(&mut manifest, COMPTIME_HELPERS, &[]);

    // `--extra-entry`: resolve each name against the SCANNED manifest (so an id and a
    // path both work) and refuse one that would contribute to the artifact, BEFORE the
    // entry is minted. Both refusals travel as this function's `Err`, so they reach the
    // reader as a single named `error:` line under the build-failure prefix (exit 1) —
    // NOT as a row to dig out of a diagnostic list, and not as the exit-2 usage error a
    // parse-time flag mistake gets, since resolving a name needs the scanned manifest.
    let extra_entries: Vec<String> = profile
        .extra_entries
        .iter()
        .map(|arg| {
            let id = resolve_extra_entry(&manifest, aeon, arg)?;
            refuse_artifact_contribution(&manifest, &id, arg)?;
            Ok(id)
        })
        .collect::<Result<_, String>>()?;

    // Inject the synthetic entry as a fresh module in the manifest.
    let entry_id = "native_flip_entry".to_string();
    let src = synthetic_entry_src(
        specs,
        profile.game_ram_module,
        profile.manifest_module,
        &extra_entries,
    );
    // `Manifest::scan` registers one `sources` entry per scanned FILE and pushes a
    // `modules` entry per file that parsed, so `modules.len()` is the next free id
    // only because an unreadable file is a hard error the caller already bailed on.
    // Assert it rather than rely on it: a collision would silently rebind a real
    // module's path to this nonexistent one and then filter its warnings away.
    let source = sigil_span::SourceId(manifest.modules.len() as u32);
    assert!(
        !manifest.sources.contains_key(&source),
        "synthetic entry {source:?} collides with a scanned module"
    );
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
    // defines them → 0). The set is the shape_defines merge, so a game's own
    // `map.toml [defines]` rows reach the comptime env here too.
    let defines = shape_defines(profile, aeon)?;
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
    // Every module's `embed(...)` resolves against the PROJECT ROOT — one convention,
    // no per-module base and no named exception. This is the end state of
    // EMBED-BASE-SKEW: the aeon tree briefly carried two spellings, sigil tolerated the
    // module-relative one behind a warning while the engine re-spelled math.emp, and with
    // the corpus advanced past that landing the tolerance is retired. Handing each module
    // its own directory (the transition shape) is WRONG once the fallback is gone: a
    // root-relative path would then join onto the module dir and resolve one level deep.
    let aeon_root = aeon.to_path_buf();
    let embed_base_for = move |_id: &str| -> Option<std::path::PathBuf> { Some(aeon_root.clone()) };
    let (mut sections, link_asserts, bdiags) =
        resolve::build_program_open_embed(&manifest, &entry_id, None, &opts, &embed_base_for);
    let berr: Vec<_> = bdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !berr.is_empty() {
        return Err(format!(
            "build_program: {} error(s);\n{}",
            berr.len(),
            fmt_diag_list(&berr)
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

    // The chainer recomputes every base from the frozen table, so the emp
    // placement map only needs one (cosmetic) region per DISTINCT section name.
    let map_toml = emp_map_frozen(&sections);
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

    // Every non-error diagnostic from ALL FOUR lowering sources — each is filtered
    // for errors above, and its warnings survive only through here. The index is
    // built here, not lazily, because the LINK tier needs it after the manifest is
    // gone (see `EmpProgram::sources`).
    let index = resolve::manifest::SourceIndex::new(&manifest);
    let warnings =
        collect_warnings(&index, &[&mdiags, &pdiags, &bdiags, &place_diags], Some(source));

    Ok(EmpProgram { sections, link_asserts, warnings, sources: index })
}

/// Location-resolve and deduplicate every non-error diagnostic in `sources`,
/// dropping those attributed to `generated`.
///
/// `generated` names a module the TOOL wrote — [`build_emp`]'s synthetic entry,
/// whose bare `use` lines drive the reachability BFS. A build report is for code
/// its reader can edit; that module has no on-disk file, so a diagnostic against it
/// is unactionable by construction and would only teach the reader to distrust the
/// channel. The exclusion is by exact [`SourceId`](sigil_span::SourceId) — the
/// tool/user boundary, not an exemption over corpus content, and it cannot widen.
/// `None` excludes nothing, for a caller that generates no module.
///
/// Deduplication is by `(level, message, span)` — the key
/// [`build_program_with`](sigil_frontend_emp::resolve) already uses for its own
/// cross-module merge. It collapses an exact repeat of one diagnostic at one site,
/// so the reported count is a count of DISTINCT problems. Order is stable — source
/// order, then first-seen within a source — so the rendered list is reproducible
/// build to build and a diff of two builds is meaningful.
pub fn collect_warnings(
    index: &resolve::manifest::SourceIndex,
    sources: &[&[sigil_span::Diagnostic]],
    generated: Option<sigil_span::SourceId>,
) -> Vec<BuildWarning> {
    let mut seen = std::collections::HashSet::new();
    sources
        .iter()
        .flat_map(|ds| ds.iter())
        .filter(|d| d.level != sigil_span::Level::Error && Some(d.primary.source) != generated)
        .filter(|d| seen.insert((d.level, d.message.clone(), d.primary)))
        .map(|d| BuildWarning::new(d, index))
        .collect()
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
/// EXACTLY the caller-supplied allowlist — both directions (§t24). Each profile homes
/// its own set ([`GameProfile::inapplicable_guards`]; demo/Config each home a different
/// twin subset). `inapplicable` are the "not defined in this link" diagnostics;
/// `link_asserts` supplies each guard's own message (its `.emp` site) via span match.
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
// THE placement authority. It computes EVERY section's ROM base by chaining in a
// DECLARED ORDER;
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

/// One measuring resolve's full witness: per-section image length, plus the lowered
/// length of every RELAXABLE fragment (keyed by stable section index, then by the
/// fragment's position among the section's relaxables, in order) — what the
/// non-convergence diagnostic reads the width-flipping sites off.
struct Measured {
    img: Vec<u32>,
    /// Per-section `(span, lowered length)` of each relaxable fragment, in order.
    sites: Vec<Vec<(sigil_span::Span, u32)>>,
}

/// Measure every ROM section's image length with each section pinned at
/// `pin_lma[idx]` — the EXACT length it takes at that base, whether or not the
/// pinned extents happen to intersect. Section lengths are relaxation-dependent (an
/// unsized `lea Table, a1` is abs.w when `Table` resolves under `$8000` on the
/// 24-bit bus, abs.l otherwise; a bare branch is `.s`/`.w` by distance), so the only
/// base a length is true at is the base the section will occupy — and the measuring
/// resolve (`resolve_layout_measuring`: the same placement⇄relaxation fixpoint with
/// the image-soundness checks skipped) makes that base measurable even while a grown
/// section's extent still runs into its successor's pin: label addresses are pin +
/// offset regardless of intersection, and the walk's next pack round moves the run
/// apart by exactly the lengths measured here.
///
/// THE CATCH THIS DESIGN REPLACES (2026-08-26, `player_sensors`): the previous rig
/// measured through the CHECKED resolve, so a colliding measuring layout had to be
/// re-measured at SUBSTITUTE bases — pure data at a scratch slot, else everything
/// under a cumulative +0x400/rank spread. A substitute base is a different address,
/// and a different address can select a different width: the scratch slots
/// (`0x70_0000 + k·0x10_0000`) wrap the 24-bit bus from k = 9, so `collision_data`
/// (slot 41 = `0x300_0000`, masked to `0x0` by `asl_width_rule`) looked
/// abs.w-reachable and twelve `lea` sites in `player_sensors` measured 4 B each
/// where the real base encodes 6. The walk packed the successor 24 B into the
/// section, lengths were "stable", and the build refused naming an innocent pair.
/// Measuring at the round's own pins has no substitute base to lie about — and no
/// spread step to outgrow (the old rig could not even measure a single section grown
/// past ~0x400).
///
/// A ROM section with NO pin keeps the LEGACY far-scratch slot (`0x70_0000 +
/// k·0x10_0000`): those are the sections outside the frozen table, and the far slot
/// is what reproduces asl's conservative widths for references that touch them (asl
/// encodes a forward reference abs.l; a near base would relax it abs.w and the
/// chained layout would settle tighter than the golden shapes). The slot arithmetic
/// is part of the frozen equilibrium the six byte gates prove — its own 24-bit alias
/// hazard is a ledger row for the next refreeze, not a live measuring input, because
/// every FROZEN-labeled section now measures at a real base in every round.
fn measure_pinned(
    sections: &[Section],
    pin_lma: &[Option<u32>],
) -> Result<Measured, String> {
    lens_pinned(sections, pin_lma, true)
}

/// [`measure_pinned`]'s body, shared with the checked form. `tolerate_overlap`
/// selects `resolve_layout_measuring` (measuring: exact widths at the given pins,
/// overlap permitted) vs `resolve_layout` (checked: the image-soundness witness —
/// the walk runs it once at the converged bases, and `declared_spans_by_index` at
/// the final ones). The unique-name tag makes the read-back unambiguous (the tree
/// carries same-named `text`/`sec<lma>` sections); names never affect bytes.
fn lens_pinned(
    sections: &[Section],
    pin_lma: &[Option<u32>],
    tolerate_overlap: bool,
) -> Result<Measured, String> {
    let mut tagged: Vec<Section> = sections.to_vec();
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
    let resolved = if tolerate_overlap {
        sigil_link::resolve_layout_measuring(&tagged, &stubs, true)
    } else {
        sigil_link::resolve_layout(&tagged, &stubs, true)
    }
    .map_err(|d| {
        format!("span pass: resolve_layout: {} diag(s); first {:?}", d.len(), d.first())
    })?;
    let mut img = vec![0u32; sections.len()];
    let mut sites = vec![Vec::new(); sections.len()];
    for s in &resolved {
        if let Some(idx) = s.name.rsplit('\u{0}').next().and_then(|t| t.parse::<usize>().ok()) {
            if is_rom_section(s) {
                img[idx] = s.image_len();
                // Relaxables lower 1:1 (the resolve maps each fragment to its chosen
                // candidate in place), so the input's relaxable positions read the
                // lowered lengths off the same indices.
                if s.fragments.len() == sections[idx].fragments.len() {
                    sites[idx] = sections[idx]
                        .fragments
                        .iter()
                        .zip(&s.fragments)
                        .filter(|(before, _)| is_relaxable(before))
                        .map(|(before, after)| (fragment_span(before), fragment_len(after)))
                        .collect();
                }
            }
        }
    }
    Ok(Measured { img, sites })
}

fn is_relaxable(f: &Fragment) -> bool {
    matches!(f, Fragment::RelaxAbsSym { .. } | Fragment::JmpJsrSym { .. } | Fragment::RelaxLadder { .. })
}

fn fragment_span(f: &Fragment) -> sigil_span::Span {
    match f {
        Fragment::Data(d) => d.span,
        Fragment::Fill { span, .. }
        | Fragment::Reserve { span, .. }
        | Fragment::Org { span, .. }
        | Fragment::JmpJsrSym { span, .. }
        | Fragment::RelaxAbsSym { span, .. }
        | Fragment::RelaxLadder { span, .. } => *span,
    }
}

/// The image bytes one LOWERED fragment contributes (a lowered relaxable is `Data`).
fn fragment_len(f: &Fragment) -> u32 {
    match f {
        Fragment::Data(d) => d.bytes.len() as u32,
        Fragment::Fill { count, .. } => *count,
        _ => 0,
    }
}

/// Per-section image length (exact, relaxables lowered to `Data`) from the CHECKED
/// resolve at `pin_lma` — the image-soundness witness form of [`measure_pinned`].
fn image_lens_pinned(
    sections: &[Section],
    pin_lma: &[Option<u32>],
) -> Result<Vec<u32>, String> {
    lens_pinned(sections, pin_lma, false).map(|m| m.img)
}

/// The TRUE per-section ROM base, keyed by stable index; `None` for RAM/phase-only
/// sections. The baked resume orgs are
/// WRONG sonic4 values, so each section's provisional base is `frozen[L] − offset[L]` for
/// a contained frozen label `L` (a label-less DATA blob derives by CONTIGUITY from its
/// frozen neighbour; a hard-org PHASE BANK keeps its baked =asl org) — and `packed_true_
/// bases` walks those provisional bases in the MAP's declared `order` (K5: the map drives
/// the sequence; the frozen provisional bases give only anchors + measurement pins — the
/// alignment each section packs to is its `crate::section_align` declaration).
fn true_bases_by_index(
    sections: &[Section],
    table: &HashMap<String, u32>,
    map_order: &[String],
    fixture: bool,
    anchor_addrs: &std::collections::HashSet<u32>,
    warnings: &mut Vec<BuildWarning>,
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
) -> Result<Vec<Option<u32>>, String> {
    let n = sections.len();
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
    // ALWAYS-ON, before the walk reads a single declaration: every ROM section that
    // carries a head label is declared in `crate::section_align`, so the walk is never
    // asked to place a section it has no alignment input for — and every such section
    // is named in ONE report rather than at the first one the walk reaches.
    validate_declared_alignment(sections)?;
    packed_true_bases(sections, &prov, &labeled, map_order, fixture, anchor_addrs, warnings, locate)
}

/// The §17 Wave-B B-0 packing walk (the rows-6/58 partial realization): the MAP's
/// declared `order` gives the section sequence (K5 — the map DRIVES; see the sort below);
/// the provisional bases give the org-island ANCHORS; each section's ALIGNMENT is its
/// `crate::section_align` declaration; every other ROM section's base is PACKED from
/// live-measured image lengths, so a size-changing parcel shifts its contiguous run
/// downstream instead of colliding with stale pins. Rules per section (walked in the
/// map-driven order):
///   - ISLAND (a DECLARED `[[anchor]]`, or the run head): absolute at prov; a packed
///     run that overflows past an island's base fails loud at the final
///     `resolve_layout` overlap check. A stale prov gap at a non-anchor is an
///     allotment and packs contiguously.
///   - PHASE BANK (vma ≥ 0x8000, vma ≠ lma) head, and label-less blobs inside its
///     hard-org run: absolute at prov (the sound banks never pack).
///   - label-less non-phase blob: contiguity from its neighbour (the Frozen
///     boot-region Z80-idle rule).
///   - everything else — every section with a head label, whether or not a frozen
///     table names it: `align_up(running, required_for(head label))`
///     ([`packed_chained_base`]). The declaration is the ONLY alignment input; a
///     section with no declaration is refused by name, never given a default.
///
/// Image lengths are relaxation-dependent (branch widths move with distance), so the
/// walk iterates measure → pack to a fixpoint (≤ 8 rounds; round 0 measures pure data
/// at disjoint scratch pins and code at its provisional pin). Island classification
/// must be IDENTICAL across rounds — a growth big enough to reach a declared anchor is
/// a hand-ruling, not a silent repack. A run that merely drifts past its stale
/// provisional base is reported as `[layout.provisional-drift]` (see `warnings`) and
/// packs on; the frozen tables are a check on the last refreeze, not a build input.
///
/// K5 — WHAT THE FROZEN TABLE STILL CARRIES (the demoted measurement-cache role): the
/// `order` AUTHORITY is now `map_order`, not the frozen provisional bases. The frozen
/// table survives ONLY as: (1) each section's provisional BASE — the org-island anchor
/// positions and the round-0 measurement pins; (2) the boundary keys the size derivation
/// (`derive_frozen_table`) reads back. It authors neither the sequence (reordering the
/// map reorders the layout; a byte-emitting section the map omits fails loud at the
/// post-resolve `validate_placement`) nor any section's alignment.
/// THE PACKING WALK'S ALIGNMENT RULE — the base a chained section headed by `head_label`
/// packs to when the running cursor arrives at `running`: `running` rounded up to the
/// alignment `crate::section_align` DECLARES for that section.
///
/// The SINGLE authority for that arithmetic, because `seam2::sound_layout` has to
/// predict the very same base: it bakes ABSOLUTE pointers into emitted blobs (the
/// SfxTable cells, the `SFX_WIN_*` window pointers, the MT song-table pointers) against
/// its own derivation, and if that derivation and this walk disagree the pointers are
/// short by the difference and the sound goes silent at runtime. A second copy of this
/// arithmetic is the bug, not the fix — that is the lesson of the three unmaintained
/// copies of the sound-bank addresses (commit 2c49f538). Both callers pass the section's
/// HEAD LABEL and nothing else, so neither can hand it a quantum of its own.
///
/// The alignment is a property of the SECTION, stated with its source in
/// `section_align::DECLARED`, never of where the section happened to land at the last
/// refreeze — so a repin cannot change it, in either direction. The always-on
/// `validate_sound_fold` gate still compares prediction against placement for the two
/// sound blobs, because agreement between two readers of one declaration is a claim
/// about the readers, not the declaration.
///
/// LOUD ON UNMEASURABLE: a head label with no declaration is a refusal naming it. It is
/// never rendered as 1, never as 2, never as 16, and never as a pass — the declaration is
/// the only alignment input there is.
pub fn packed_chained_base(running: u32, head_label: &str) -> Result<u32, String> {
    let Some(decl) = crate::section_align::required_for(head_label) else {
        return Err(format!(
            "[layout.undeclared-alignment] section headed by `{head_label}` has NO declared \
             alignment in `sigil_harness::section_align::DECLARED`, and the packing walk \
             has no other alignment input for it. Add one row naming the alignment the \
             section REQUIRES and the source that requires it — not the number the \
             current layout happens to give it"
        ));
    };
    let a = decl.required;
    Ok(running.div_ceil(a) * a)
}

/// THE DECLARATION GATE, first half — every ROM section that carries a head label has a
/// declared alignment in `crate::section_align`, checked BEFORE the packing walk so the
/// report names every undeclared section at once.
///
/// R7 of the placement-constraint inventory. The declaration is the walk's only
/// alignment input ([`packed_chained_base`]), so a section without one is a section the
/// walk cannot place — the walk refuses it too, at the first such section it reaches;
/// this pass exists so the refusal is one reviewed list rather than one name per build.
///
/// Scope is EVERY ROM section with a head label, pinned or not: a section no frozen
/// table names packs by the same declaration as one that is, so the frozen tables say
/// nothing about which sections need a row. A label-less blob has nothing to declare
/// under and packs by contiguity; `validate_resolved_alignment` still measures the base
/// it lands on.
///
/// LOUD ON UNMEASURABLE: an undeclared section is a REFUSAL naming the section and its
/// head label. It is never rendered as a number and never as a pass.
fn validate_declared_alignment(sections: &[Section]) -> Result<(), String> {
    let mut faults: Vec<String> = Vec::new();
    for s in sections.iter().filter(|s| is_rom_section(s)) {
        let Some(head) = head_label(s) else { continue };
        if crate::section_align::required_for(head).is_none() {
            faults.push(format!(
                "section `{}` (head label `{head}`) has NO declared alignment in \
                 `sigil_harness::section_align::DECLARED`. The packing walk has no other \
                 alignment input for it. Add one row naming the alignment the section \
                 REQUIRES and the source that requires it — not the number the current \
                 layout happens to give it",
                s.name
            ));
        }
    }
    if faults.is_empty() {
        return Ok(());
    }
    Err(format!(
        "[layout.undeclared-alignment] {} section(s):\n  - {}",
        faults.len(),
        faults.join("\n  - ")
    ))
}

/// THE DECLARATION GATE, second half — every ROM section's declared alignment against
/// the base it ACTUALLY LANDS ON in the resolved layout.
///
/// The independent instrument. The packing walk packs a chained section to its
/// declaration by construction, so asking the walk whether it honoured the declaration
/// proves nothing; this half measures the resolved layout the ROM is emitted from, which
/// is produced by the packing walk plus `declared_spans` plus `resolve_layout` — so it
/// covers every section the walk places by a rule OTHER than the declaration: a declared
/// `[[anchor]]` island, a phase-bank hard org, and the label-less contiguity blobs.
///
/// `lma % required == 0` is a statement about the artifact, not about which rule
/// produced it.
pub fn validate_resolved_alignment(resolved: &[Section]) -> Result<(), String> {
    let mut faults: Vec<String> = Vec::new();
    for s in resolved.iter().filter(|s| is_rom_section(s)) {
        let Some(head) = head_label(s) else { continue };
        let Some(decl) = crate::section_align::required_for(head) else {
            faults.push(format!(
                "section `{}` (head label `{head}`, resolved base {:#x}) has NO declared \
                 alignment in `sigil_harness::section_align::DECLARED` — add one row \
                 naming the alignment it REQUIRES and the source that requires it",
                s.name, s.lma
            ));
            continue;
        };
        if !s.lma.is_multiple_of(decl.required) {
            faults.push(format!(
                "section `{}` (head label `{head}`) declares alignment {} — {} — but the \
                 resolved layout places it at {:#x} (base % {} = {})",
                s.name,
                decl.required,
                decl.why,
                s.lma,
                decl.required,
                s.lma % decl.required
            ));
        }
    }
    if faults.is_empty() {
        return Ok(());
    }
    Err(format!(
        "[layout.alignment-violated] {} section(s) placed at a base their declared \
         alignment forbids:\n  - {}",
        faults.len(),
        faults.join("\n  - ")
    ))
}

/// The head label of a section — its lowest-offset label — the name an `order` row
/// spells when it is a label row. `None` for a label-less blob.
fn head_label(sec: &Section) -> Option<&str> {
    sec.labels.iter().min_by_key(|l| l.offset).map(|l| l.name.as_str())
}

/// The `order` row a section is declared by, and its rank: the head label's row when
/// there is one, else the `section:<name>` row. A section whose head label is CONTENT-
/// DERIVED (`ojz_effects_editor_act1`, minted by `tools/effects_gen.py`) is authorable
/// only by the second spelling. `validate_placement` rejects a section declared BOTH ways.
fn order_rank_of(sec: &Section, rank: &HashMap<&str, usize>) -> Option<usize> {
    head_label(sec)
        .and_then(|l| rank.get(l).copied())
        .or_else(|| rank.get(crate::map_placement::section_row_key(&sec.name).as_str()).copied())
}

#[allow(clippy::too_many_arguments)]
fn packed_true_bases(
    sections: &[Section],
    prov: &[Option<i64>],
    labeled: &[bool],
    map_order: &[String],
    fixture: bool,
    anchor_addrs: &std::collections::HashSet<u32>,
    warnings: &mut Vec<BuildWarning>,
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
) -> Result<Vec<Option<u32>>, String> {
    // ISLANDS ARE THE DECLARED ANCHORS. A section is an org island exactly when its
    // provisional base is a `[[anchor]]` the map declares (object bank / DAC / sound)
    // — the hardware-fixed addresses — or it heads the run. A provisional-base gap in
    // the frozen table (a neighbour that grew, or the fixture's relocated pool) is an
    // ALLOTMENT, never a hole to preserve: everything else packs contiguously. The
    // post-resolve `validate_placement` proves inferred == declared on every shipped
    // shape, so nothing but the declared anchors can ever be held absolute.
    let is_anchor_gap = |p: i64| -> bool { anchor_addrs.contains(&(p as u32)) };
    let n = sections.len();
    let mut order: Vec<usize> = (0..n).filter(|&i| prov[i].is_some()).collect();
    // ── K5: THE MAP DRIVES ORDER ──
    // The declared `order` list is the AUTHORITY for the byte-emitting section sequence.
    // Each ROM section carrying a map-declared head-label sorts by its MAP RANK (no longer
    // by its frozen provisional base); a zero-byte boundary section the map does not name
    // (the label-less boot blobs, the `EndOfRom` terminus) rides the rank of the nearest
    // preceding named section by prov, then prov, then its stable index — a pure
    // measurement-cache role (it emits no bytes, so its slot never moves a byte). With
    // `map_order` empty (a region-only fixture) every section
    // is unranked and this degenerates to the provisional-base sort (the pre-K5 order).
    //
    // WHY THIS IS FOLD-IDENTICAL: for every shipped shape the byte-emitting sections'
    // frozen provisional bases already ascend in exactly the declared order (K1 proved the
    // subsequence; K5's probe confirmed the map ranks strictly increase along the prov
    // walk on all six targets), so ranking by the map reproduces the prov order byte-for-
    // byte — while making the DECLARATION, not the frozen table, the thing that authored it.
    let rank: HashMap<&str, usize> =
        map_order.iter().enumerate().map(|(r, s)| (s.as_str(), r)).collect();
    let own_rank: Vec<Option<usize>> = (0..n).map(|i| order_rank_of(&sections[i], &rank)).collect();
    // (prov, rank) of every NAMED section, prov-sorted — the ladder an unnamed boundary
    // section reads its inherited rank off (the nearest declared run it sits within).
    let mut named_ladder: Vec<(i64, usize)> =
        order.iter().filter_map(|&i| own_rank[i].map(|r| (prov[i].unwrap(), r))).collect();
    named_ladder.sort_by_key(|&(p, _)| p);
    let eff_rank = |i: usize| -> i64 {
        if let Some(r) = own_rank[i] {
            return r as i64;
        }
        let p = prov[i].unwrap();
        match named_ladder.partition_point(|&(np, _)| np <= p) {
            0 => -1,
            k => named_ladder[k - 1].1 as i64,
        }
    };
    order.sort_by_key(|&i| (eff_rank(i), prov[i].unwrap(), i));
    // ANCHOR_GAP — the gap a LABEL-LESS blob's baked lma must open past the running
    // cursor before an anchor match at that address counts (see the unlabeled arm).
    const ANCHOR_GAP: i64 = 0x400;
    // GROWTH_DRIFT_TOLERANCE — how far a CONTIGUOUSLY-packed section's base may drift
    // above its (stale) frozen provisional before the packer REPORTS it. Drift is a
    // `[layout.provisional-drift]` WARNING, never a build stop: the frozen tables are an
    // after-the-fact check of the last refreeze, and content growth (an authored band,
    // a new object module) moving a downstream run is the normal workflow, not a
    // fault. The tolerance only decides which drifts are worth a line and never
    // touches an unchanged section (which packs at drift 0); islands are the declared
    // anchors, not a gap width (the post-resolve `validate_placement` keeps its own
    // ANCHOR_GAP inference as the `[map.undeclared-island]` lint). A real org-anchor
    // overrun still fails loud at the final `resolve_layout` overlap check, so anchors
    // stay protected regardless.
    const GROWTH_DRIFT_TOLERANCE: i64 = 0x1000;

    // Round 0: lengths at the PROVISIONAL bases — frozen-labeled sections pinned at
    // prov, never-pinned sections at the legacy scratch. EXACT for those pins even
    // when a grown section's extent runs into its successor's stale pin
    // (`measure_pinned` tolerates overlap), so there is no fallback measurement, no
    // substitute base that can select a wrong width, and no spread step a large
    // parcel can outgrow. The pack rounds below move the run apart from these
    // lengths and re-measure at the packed bases to a fixed point.
    let prov_pins: Vec<Option<u32>> = (0..n)
        .map(|i| if labeled[i] { prov[i].map(|v| v as u32) } else { None })
        .collect();
    let measured = measure_pinned(sections, &prov_pins)
        .map_err(|e| format!("{e} (provisional round)"))?;
    let mut img = measured.img;
    let mut sites = measured.sites;
    let mut prev_islands: Option<Vec<bool>> = None;
    let mut last_flip: Option<String> = None;
    for _round in 0..8 {
        let mut out: Vec<Option<u32>> = vec![None; n];
        let mut islands = vec![false; n];
        let mut running: Option<i64> = None;
        let mut in_phase_run = false;
        // This round's drift reports; only the CONVERGED round's reach the sink, so a
        // section drifting through several shrinking-length rounds is reported once,
        // with its final base.
        let mut round_drift: Vec<BuildWarning> = Vec::new();
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
                    Some(_) if is_anchor_gap(p) => {
                        // A declared anchor is held absolute even when the run behind it
                        // has crept up to (or past) it — an overrun is then a loud
                        // overlap at the final `resolve_layout`, never a silent repack
                        // of a hardware-fixed address.
                        islands[i] = true;
                        p
                    }
                    Some(r) => {
                        // The DECLARED alignment is the only input, through the one
                        // function `seam2::sound_layout` predicts with. A zero-byte marker
                        // (the EndOfRom terminus) gets no special case: its address is
                        // defined by the end of the emitted image, and its declaration says
                        // 2 for exactly that reason, so a wider quantum cannot reach it and
                        // open a fill gap the assembled-bar completeness guard would reject.
                        let head = head_label(&sections[i]).ok_or_else(|| {
                            format!(
                                "section `{}` is pinned by a frozen label but has no head \
                                 label to read a declared alignment under",
                                sections[i].name
                            )
                        })?;
                        let packed = packed_chained_base(r as u32, head)? as i64;
                        // A downstream run that overran its frozen provisional base by more
                        // than the tolerance is REPORTED, not refused: the frozen table is
                        // stale against real content, and the refreeze at landing is the
                        // remedy. A run that overruns a real ORG ANCHOR (island/phase bank)
                        // still fails loud at the final `resolve_layout` overlap check, so
                        // anchors stay protected. The stress fixture grows on purpose and
                        // is not reported.
                        if !fixture && packed > p + GROWTH_DRIFT_TOLERANCE {
                            round_drift.push(provisional_drift_warning(&sections[i], packed, p));
                        }
                        packed
                    }
                }
            } else if in_phase_run {
                p // hard-org phase-run tail: absolute
            } else {
                // A label-less blob's `p` is its BAKED lma (an order-only fallback, 0 for
                // every .emp section the frozen table does not name), so an anchor match
                // alone means nothing here — the blob is an island only when it also
                // opens a real gap past the running cursor.
                match running {
                    Some(r) if p > r + ANCHOR_GAP && is_anchor_gap(p) => {
                        islands[i] = true;
                        p
                    }
                    // Contiguity from the neighbour — rounded up to the section's declared
                    // alignment when it has a head label to declare under (a section no
                    // frozen table names packs by the same declaration as one that is);
                    // a label-less blob has none and packs flush.
                    Some(r) => match head_label(&sections[i]) {
                        Some(head) => packed_chained_base(r as u32, head)? as i64,
                        None => r,
                    },
                    None => {
                        islands[i] = true;
                        p
                    }
                }
            };
            out[i] = Some(tb as u32);
            running = Some(tb + img[i] as i64);
        }
        // FIXTURE (stress-art): a section growing tens of KB shifts downstream sections far
        // enough that borderline (non-anchor) island classification can wobble between the
        // shrinking-length rounds. That is expected for a fixture and harmless — the true
        // org anchors (huge prov gaps / phase banks) stay islands regardless — so the
        // reclass guard is waived; the walk still converges to a fixpoint or fails loud on
        // non-convergence below.
        if !fixture {
            if let Some(prev) = &prev_islands {
                if *prev != islands {
                    return Err(
                        "island classification changed between packing rounds — growth ate an org hole; hand ruling needed"
                            .to_string(),
                    );
                }
            }
        }
        prev_islands = Some(islands);
        // Re-measure with the round-0 pin discipline: FROZEN-LABELED sections at
        // their packed bases (correct relaxation at the very address each will
        // occupy), never-pinned sections at the legacy scratch (the asl-width
        // emulation, unchanged), and the phase banks at scratch too — a bank's
        // content is addressed through its vma, and the pre-bank blob's baked
        // align-pad is sized for its ORIGINAL position, so at a shifted packed base
        // its image overshoots the hard bank org until declared_spans clamps it.
        let bankish: Vec<bool> = sections
            .iter()
            .map(|s| s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false))
            .collect();
        let remeasure: Vec<Option<u32>> = (0..n)
            .map(|i| if labeled[i] && !bankish[i] { out[i] } else { None })
            .collect();
        // Exact at these pins (overlap tolerated — a length that grows only once the
        // layout is packed, the config_a `boot` knife-edge, simply measures longer
        // here and the next pack round absorbs it). `img2 == img` therefore means
        // every section's length was measured at the very base the walk assigns it —
        // a true fixed point, not a substitute-base plateau.
        let measured2 = measure_pinned(sections, &remeasure)
            .map_err(|e| format!("{e} (packed round)"))?;
        let img2 = measured2.img;
        let sites2 = measured2.sites;
        if img2 == img {
            // The fixed point — now prove the image sound with ONE checked resolve at
            // the same pins. A collision here is REAL: every length is the one
            // measured at its own base, so the only way two extents intersect is a
            // run packed up into something the walk holds ABSOLUTE — a declared
            // anchor (or a phase-bank org). The old rig reached this refusal through
            // a fallback measurement that could lie about a length, and then named
            // whatever innocent pair the stale lengths implied.
            if let Err(collision) = image_lens_pinned(sections, &remeasure) {
                return Err(format!(
                    "packed layout overlaps at its real bases — a run grew into a declared anchor; \
                     the anchor is hardware-fixed, so this content does not fit and needs a hand ruling \
                     (the map's re-layout parcel, or less content). {collision}"
                ));
            }
            warnings.extend(round_drift);
            return Ok(out);
        }
        last_flip = Some(width_flip_report(sections, &order, &img, &sites, &img2, &sites2, &remeasure, locate));
        img = img2;
        sites = sites2;
    }
    // Every round re-derived the bases from lengths measured at those very bases and
    // the lengths still moved: a RELAXING SITE is oscillating (an encoding that is
    // short at the base its own short form implies and long at the base its long
    // form implies). Name it — the flip report from the last two rounds — instead of
    // any pair of sections the transient layout happened to collide.
    Err(format!(
        "packed_true_bases did not converge in 8 rounds (relaxation oscillation) — hand ruling needed.{}",
        last_flip.map(|f| format!(" {f}")).unwrap_or_default()
    ))
}

/// The non-convergence diagnostic's payload: for every section whose measured length
/// differs between two consecutive rounds, the section, the base it was measured at,
/// both lengths, and the `file:line` of every relaxable site whose lowered width
/// differs between the rounds (with both widths) — the RELAXING SITE, so the author
/// sees the `lea`/branch whose encoding depends on where the section lands instead
/// of an innocent overlapping pair.
#[allow(clippy::too_many_arguments)]
fn width_flip_report(
    sections: &[Section],
    order: &[usize],
    img_before: &[u32],
    sites_before: &[Vec<(sigil_span::Span, u32)>],
    img_after: &[u32],
    sites_after: &[Vec<(sigil_span::Span, u32)>],
    pins: &[Option<u32>],
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
) -> String {
    let mut lines = Vec::new();
    for &i in order {
        if img_before[i] == img_after[i] {
            continue;
        }
        let head = head_label(&sections[i]).map(|l| format!(" (`{l}`)")).unwrap_or_default();
        let base = pins[i].map(|b| format!("{b:#x}")).unwrap_or_else(|| "scratch".to_string());
        let mut line = format!(
            "section `{}`{head} measures {:#x} then {:#x} at base {base}",
            sections[i].name, img_before[i], img_after[i]
        );
        let flips: Vec<String> = sites_before[i]
            .iter()
            .zip(&sites_after[i])
            .filter(|((_, a), (_, b))| a != b)
            .map(|((span, a), (_, b))| {
                let at = locate(*span)
                    .unwrap_or_else(|| format!("source {} @{}", span.source.0, span.start));
                format!("{at} ({a} B -> {b} B)")
            })
            .collect();
        if !flips.is_empty() {
            line.push_str("; width-flipping sites: ");
            line.push_str(&flips.join(", "));
        }
        lines.push(line);
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("Sections whose length depends on their base: {}", lines.join(" | "))
    }
}

/// The `[layout.provisional-drift]` warning: `sec` packed at `packed`, a delta past
/// its frozen provisional base `prov` wider than the walk's drift tolerance. Names
/// the section (and its head label, the name an author sees in `map.toml`), both
/// addresses and the delta, so the refreeze the landing owes is a sentence, not a
/// diff hunt. Carries no source span: the drift is a property of the layout, not of
/// a line.
fn provisional_drift_warning(sec: &Section, packed: i64, prov: i64) -> BuildWarning {
    let head = sec
        .labels
        .iter()
        .min_by_key(|l| l.offset)
        .map(|l| format!(" (`{}`)", l.name))
        .unwrap_or_default();
    let delta = packed - prov;
    BuildWarning {
        level: sigil_span::Level::Warning,
        id: "layout.provisional-drift".to_string(),
        location: None,
        message: format!(
            "[layout.provisional-drift] section `{}`{head} packed at {packed:#x}, frozen provisional {prov:#x} (delta {delta:+#x}); the frozen placement tables are stale against this content — refreeze at landing",
            sec.name
        ),
        primary: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
    }
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
    // Declared spans measure at the FINAL true_bases — the packed layout is already
    // overlap-free (no scratch fallback: every length here is the real one), and each
    // pure-data span comes from its base-to-next gap.
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
pub fn is_rom_section(s: &Section) -> bool {
    match s.vma_base {
        Some(v) => v < 0x00F0_0000,
        None => true,
    }
}


/// FIXTURE-ONLY (stress-art): relocate the stress-GROWABLE OJZ generated sections past the
/// sound banks so their uniquified inflation extends the ROM TAIL instead of overrunning
/// OJZ's hard DAC anchor at $48000. OJZ's pre-DAC hole caps in-order act data at ~21 KB of
/// slack, and the fixture grows three sections there — the act art POOL (41 pages), the
/// per-section BLOCK blobs, and the local->global MAPS (both grow with the re-pointed clone
/// references). Relocating all three keeps the 116 KB `collision_data` section (adjacent to
/// the DAC anchor) at its canonical position, so the anchor's island gap survives.
///
/// Every relocated section is reached ONLY through a manifest / descriptor pointer
/// (`act_art_pool_table`, `sec_block_index`, `act_sec_local_maps` — all `extern`), so moving
/// them is faithful to the residency design's position-independence contract, not a hack.
///
/// They move (preserving their relative order) to IMMEDIATELY BEFORE the fault-handler island
/// (`ReleaseFault` release / `BusError` debug — consecutive at the tail ahead of `EndOfRom`),
/// so error_handler stays the LAST byte-emitting section (the MDDBG deb2 locator invariant
/// holds in the fixture too). The org anchors (object bank, DAC/sound phase banks) are
/// untouched and still fail loud on any real overrun. Gated by `fixture_placement`; shipped
/// shapes never call this.
fn relocate_fixture_pool(order: &mut Vec<String>) -> Result<(), String> {
    // Head-labels of the stress-growable OJZ generated sections, in ROM order.
    const GROWABLE: &[&str] = &["OJZ_Act_Pool_Page0", "OJZ_Sec0_Blocks", "OJZ_Sec0_LocalMap"];
    let mut moved: Vec<String> = Vec::new();
    for head in GROWABLE {
        match order.iter().position(|s| s == head) {
            Some(at) => moved.push(order.remove(at)),
            None => return Err(format!("fixture placement: growable section `{head}` not in the map order")),
        }
    }
    // Insert the moved run just before the fault-handler island (recompute after removals).
    let fault_at = order
        .iter()
        .position(|s| s == "ReleaseFault" || s == "BusError")
        .ok_or("fixture placement: no fault-handler island (ReleaseFault/BusError) in the map order")?;
    for (k, head) in moved.into_iter().enumerate() {
        order.insert(fault_at + k, head);
    }
    Ok(())
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
/// chaining. The DECLARED ORDER + SIZES come from the map's `order` and the profile's
/// `frozen_sizes` table.
/// Genuine org anchors (object bank, phase-address sound banks) keep their declared
/// base; every other section is `Chained` with its lma zeroed (base computed).
pub fn build_rom_chained(aeon: &Path, profile: &GameProfile) -> Result<Vec<u8>, String> {
    Ok(build_rom_chained_with_listing(aeon, profile)?.rom)
}

/// Parcel-K5 · the post-resolve DRIVE-CONFIRMATION pass. K1 landed this as a VALIDATION of
/// a frozen-derived order (`derived ⊆ declared`); K5 flipped the packer so the map's `order`
/// DRIVES the walk, and this pass now CONFIRMS the drive against the resolved layout — the
/// direction inverted (the declaration is the input; this checks the build honoured it and
/// is complete). Each class fails loud:
///   - `[map.undeclared-island]`: an ANCHOR_GAP-inferred island (a resolved ROM section
///     whose base opens a > `ANCHOR_GAP` gap past the running end, or a phase bank, or
///     the run head) whose base is not a declared `anchor` fails loud; and every
///     shape-applicable declared anchor must appear.
///   - `[map.order-undeclared]` (COMPLETENESS — the K5 inversion's teeth): the map DRIVES,
///     so every byte-emitting section (min-offset label, image bytes > 0) MUST be declared
///     in `order`; a byte-emitting section the map omits fails loud (it could not be driven,
///     so it fell to its frozen provisional slot — never silently placed). Zero-byte markers
///     (`__BUDGET_DATA`, the `EndOfRom` terminus) are excluded (they emit nothing, so the
///     map need not sequence them).
///   - `[map.order-diverged]`: the resolved byte-emitting sequence must follow the declared
///     `order` (strictly increasing declared position). Post-drive this can only fire on a
///     packer BUG (the walk did not honour the declaration) — it is the drive's own guard.
///   - HOLE (data; K2 enforces): a shape-applicable hole's `after` label must resolve.
///
/// The frozen tables are DEMOTED to the per-label provisional-base measurement cache
/// (anchors + alignment + boundary keys); the map is the sole ORDER + anchor AUTHORITY.
///
/// THE FOLD-VS-PLACEMENT GATE — the one aeon has been asking for in
/// `games/sonic4/data/sound/sfx_bank_blob.emp` ("the fold-vs-placement equality gate
/// is the sigil ask in DEFERRED_WORK").
///
/// seam-2 bakes ABSOLUTE pointers into the emitted sound blobs — the `SfxTable`
/// pointer cells inside `sfx_bank.bin`, the nine `SFX_WIN_*` window pointers
/// `sfx_blob_win_tab.bin` consumes, and the MT song/patch table pointers — against
/// the base `seam2::sound_layout` PREDICTS. The chainer then places those blobs
/// independently. If the two disagree by N, every one of those pointers is N bytes
/// short of the data it names and the sound is silent or garbled at runtime, with no
/// build error and no visible symptom short of listening to it.
///
/// The pre-existing aeon-side wall, `ensure((winptr(Sfx_33) & 7) == 0)`, cannot catch
/// this: it checks only that the PLACED base is 8-aligned, and in the real failure
/// both the predicted and the placed base were 8-aligned (`$5BB18` and `$5BB20`).
/// Only comparing the two numbers finds it.
///
/// ALWAYS-ON, deliberately, and NOT behind `SIGIL_STRICT_GATE` — the previous
/// instance of this failure (sound package 3, a persistent silent -2) shipped
/// precisely because the checks that would have caught it were opt-in.
fn validate_sound_fold(
    aeon: &Path,
    resolved: &[Section],
    profile: &GameProfile,
) -> Result<(), String> {
    if !profile.sound_on {
        return Ok(()); // no sound bank in this shape; the layout is meaningless
    }
    let layout = crate::seam2::sound_layout(aeon)?;

    // LMA, not vma_origin(): the sound-bank heads are phased (`vma: $8000`), so a
    // VMA read would hand back a window address and compare against nothing.
    let placed = |label: &str| -> Option<u32> {
        resolved.iter().find_map(|sec| {
            sec.labels
                .iter()
                .find(|l| l.name == label)
                .map(|l| sec.lma + l.offset)
        })
    };

    let sfx_predicted = if profile.debug {
        layout.sfx_bank_lma_debug
    } else {
        layout.sfx_bank_lma_plain
    };
    for (label, predicted) in
        [("Song_MovingTrucks", layout.mt_bank_lma), ("Sfx_33", sfx_predicted)]
    {
        let Some(actual) = placed(label) else { continue }; // not in this shape
        if actual != predicted {
            let quantum = crate::section_align::required_for(label)
                .map(|d| d.required.to_string())
                .unwrap_or_else(|| "<UNDECLARED>".to_string());
            return Err(format!(
                "[sound.fold-vs-placement] seam-2 folded absolute pointers against \
                 `{label}` = {predicted:#x} but the chainer placed it at {actual:#x} \
                 (delta {:+}). Every pointer cell in that blob is off by the same \
                 amount, so the sound would be silent or garbled at runtime with no \
                 other symptom. The chainer aligns this section's base to its DECLARED \
                 alignment {quantum} (sigil_harness::section_align) through \
                 native::packed_chained_base; seam2::sound_layout must reach the same \
                 base through the same function with the same head label. Fix the \
                 prediction, not the declaration.",
                actual as i64 - predicted as i64
            ));
        }
    }
    Ok(())
}

/// The post-resolve placement contract: the resolved layout against the shape's
/// declared `map.toml` — island anchors, the driving `order`, and both halves of every
/// `[[hole]]` (its `after` anchor is present, and its reserved interior is empty but for
/// the module `filled_by` names).
///
/// `registry` is the shape's module list — the same `profile.registry` the build places
/// from. The hole's interior half needs it to turn `filled_by`'s MODULE id into the set
/// of section names permitted inside the reserved span; taking that set from the
/// resolved layout instead would assert only that the layout agrees with itself.
pub fn validate_placement(
    resolved: &[Section],
    pmap: &crate::map_placement::PlacementMap,
    sound_on: bool,
    registry: &[ModuleSpec],
) -> Result<(), String> {
    const ANCHOR_GAP: u32 = 0x400;
    // ROM sections, lma-sorted, with (head_label, section_name, lma, byte_len, is_phase_bank).
    let mut rows: Vec<(String, &str, u32, usize, bool)> = resolved
        .iter()
        .filter(|s| is_rom_section(s))
        .map(|s| {
            let id = head_label(s).unwrap_or_default().to_string();
            let pb = s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false);
            (id, s.name.as_str(), s.lma, s.image_bytes().len(), pb)
        })
        .collect();
    rows.sort_by_key(|r| r.2);

    // ── Anchors: recover the inferred island set from the resolved layout ──
    //
    // LEDGER (2026-08-03, found while diagnosing the config_a packed-remeasure abort):
    // anchor NAMES are decorative here — the map is keyed by `a.at` (ADDRESS), which is
    // what makes a section rename transparent (see the dac_banks note in
    // games/sonic4/map.toml). That is load-bearing, because one declared name is
    // already a FALSE FRIEND: sonic4's `[[anchor]] name = "boot_head" / at = 0x0` is
    // commented "the run head (vectors + header), label-less" — it means the ROM's
    // first island at 0x0, NOT the actual emitted section called `boot_head`
    // (engine.boot_data, head label `BootData`, which lands at 0x39C/0x3A2 depending on
    // shape). Two different things share the string "boot_head". Harmless today
    // precisely because nothing matches anchors by name — but any future change that
    // starts keying anchors by name would silently bind the 0x0 anchor to the
    // boot_data section. Left as-is deliberately (aeon is not ours to edit from the
    // sigil gate); recorded so the next reader does not have to rediscover it.
    let declared: std::collections::HashMap<u32, &str> =
        pmap.anchors_for(sound_on).map(|a| (a.at, a.name.as_str())).collect();
    let mut inferred: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut prev_end: Option<u32> = None;
    for (_, _, lma, len, pb) in &rows {
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

    // ── Order (K5 DRIVE-CONFIRMATION): every byte-emitting section must be DECLARED in
    //    `order` (completeness — the map drives, so it must name every emitter), and the
    //    resolved byte-emitting sequence must follow the declared positions strictly. ──
    //
    //    A row spelled `section:<name>` declares a section by its NAME (the `module … in
    //    <name>` target) instead of its head label — the only authorable spelling for a
    //    section whose head label is content-derived. It ranks exactly where the row sits;
    //    a zero-byte section it names is inert (a reserved slot, authorable before the
    //    content exists); a name absent from this build, or a section declared both ways,
    //    is rejected. The `[map.order-undeclared]` text names the head label as before —
    //    aeon fixtures assert on it.
    if !pmap.order.is_empty() {
        let pos: std::collections::HashMap<&str, usize> =
            pmap.order.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
        for row in &pmap.order {
            if let Some(name) = crate::map_placement::section_row(row) {
                if !rows.iter().any(|r| r.1 == name) {
                    return Err(format!(
                        "[map.order-unknown-section] `order` row `{row}` names no ROM section in this build — the name is the `module … in <name>` target; fix the spelling or drop the row"
                    ));
                }
            }
        }
        for (id, name, _, _, _) in &rows {
            let key = crate::map_placement::section_row_key(name);
            if !id.is_empty() && pos.contains_key(id.as_str()) && pos.contains_key(key.as_str()) {
                return Err(format!(
                    "[map.order-double-declared] section `{name}` is declared twice in `order` — by its head label `{id}` and by `{key}`; keep exactly one row"
                ));
            }
        }
        let mut last: Option<(usize, String)> = None;
        for (id, name, _, len, _) in &rows {
            if *len == 0 {
                continue;
            }
            // The row this section is declared by: its `section:` row when one exists,
            // else its head label (a label-less blob is exempt, as before).
            let key = crate::map_placement::section_row_key(name);
            let declared_as = if pos.contains_key(key.as_str()) { key } else { id.clone() };
            if declared_as.is_empty() {
                continue;
            }
            let Some(&p) = pos.get(declared_as.as_str()) else {
                return Err(format!(
                    "[map.order-undeclared] byte-emitting section `{id}` is not in the declared `order` — the map DRIVES placement now, so every emitter must be declared; add it in its layout position"
                ));
            };
            if let Some((lp, lid)) = &last {
                if p <= *lp {
                    return Err(format!(
                        "[map.order-diverged] the resolved layout places `{declared_as}` after `{lid}`, but the declared `order` has `{declared_as}` before it — the packer did not honour the driving order (packer bug)"
                    ));
                }
            }
            last = Some((p, declared_as));
        }
    }

    // ── Holes, first half: the `after` anchor label must resolve ──
    //
    // PRESENCE. This arm proves the hole has a subject, and it runs first so a hole with
    // no left edge is refused by the name aeon fixtures assert on
    // (`[map.hole-anchor-missing]`) rather than by the interior half's own
    // `[map.hole-anchor-unresolved]`.
    for h in pmap.holes_for(sound_on) {
        let present = resolved.iter().any(|s| s.labels.iter().any(|l| l.name == h.after));
        if !present {
            return Err(format!(
                "[map.hole-anchor-missing] declared hole after `{}` (at {:#X}) — its `after` label is not in the resolved layout",
                h.after, h.at
            ));
        }
    }

    // ── Holes, second half: the reserved interior is empty but for its filler ──
    //
    // The `negative` half — see [`hole_interior_faults`], which owns the derivation and
    // the five refusals it makes instead of a silent pass. Every fault it finds is a
    // placement violation, so they are reported together rather than one at a time: a
    // stale `at` that swallows three sections is one declaration to correct, and a
    // caller shown only the first would fix it three times.
    let faults = hole_interior_faults(resolved, pmap, sound_on, registry)?;
    if !faults.is_empty() {
        return Err(faults.join("\n"));
    }
    Ok(())
}

/// THE HOLE'S INTERIOR IS RESERVED: over one shape's resolved layout, every
/// byte-emitting section that occupies part of a declared `[[hole]]` without being the
/// module that hole declares as its filler.
///
/// A `[[hole]]` is a reserved empty span: it opens at its `after` label, runs to `at`
/// (the address the post-hole data resumes at), and the module named by `filled_by`
/// is the one thing allowed inside it. This is the only reader of `at` as a BOUND and
/// the only reader of `filled_by` at all, so without it a packed layout that puts
/// another emitter in that span, or a declaration whose `at` has drifted away from the
/// layout, produces a ROM whose post-hole data no longer begins where the map says it
/// does, with no build diagnostic of any kind.
///
/// [`validate_placement`] calls this on the shipped ROM build path, so its faults are
/// build errors.
///
/// THE PERMITTED SET IS DERIVED, NEVER TRANSCRIBED. `filled_by` is a MODULE id
/// (`engine.z80_init`); the sections it may occupy come from `registry` — the module
/// list the build is handed, upstream of every section it goes on to place. So this
/// cannot drift from the build's own idea of which section belongs to that module.
///
/// LOUD WHEN IT CANNOT MEASURE (`Err`), never a silent pass, because a hole check that
/// passes by finding nothing to check is the exact defect it exists to close:
///
/// * `[map.hole-anchor-unresolved]` — the `after` label is in no resolved section (the
///   hole has no left edge). This is the one tag the shipped path cannot reach:
///   `validate_placement`'s presence arm refuses the same layout first, by name, with
///   `[map.hole-anchor-missing]`. It keeps the function honest when it is driven
///   directly.
/// * `[map.hole-anchor-ambiguous]` — the `after` label resolves in more than one
///   section, so the hole opens at two different addresses.
/// * `[map.hole-bounds-degenerate]` — `at` is at or before the `after` label, so the
///   declared interior is empty and nothing inside it can be judged.
/// * `[map.hole-filler-unknown]` — `filled_by` names no module in this shape's
///   registry, so the permitted set is empty and every occupant, including the intended
///   filler, would read as an intruder.
/// * `[map.hole-filler-absent]` — the filler's sections are all absent from the resolved
///   layout, so the hole is not filled by the thing that is supposed to fill it.
///
/// `Ok(vec![])` over a shape whose `when` gates every hole out is a correct empty
/// answer, not coverage: a CALLER claiming "no hole is violated" must first establish
/// that some shape declares a live hole (see the population guard in this function's
/// gate).
pub fn hole_interior_faults(
    resolved: &[Section],
    pmap: &crate::map_placement::PlacementMap,
    sound_on: bool,
    registry: &[ModuleSpec],
) -> Result<Vec<String>, String> {
    let mut faults = Vec::new();
    for h in pmap.holes_for(sound_on) {
        // ── The hole's left edge: where the `after` label actually resolved ──
        let mut sites = resolved.iter().filter_map(|s| {
            s.labels.iter().find(|l| l.name == h.after).map(|l| s.lma + l.offset)
        });
        let Some(start) = sites.next() else {
            return Err(format!(
                "[map.hole-anchor-unresolved] declared hole after `{}` (at {:#X}) has no left edge — its `after` label is in no resolved section, so the span it reserves cannot be measured and NOTHING about the hole was checked",
                h.after, h.at
            ));
        };
        if let Some(other) = sites.next() {
            return Err(format!(
                "[map.hole-anchor-ambiguous] declared hole after `{}` (at {:#X}) opens at two addresses ({start:#X} and {other:#X}) — one label resolved in more than one section, so the reserved span has no single left edge",
                h.after, h.at
            ));
        }
        if h.at <= start {
            return Err(format!(
                "[map.hole-bounds-degenerate] declared hole after `{}` opens at {start:#X} but declares `at = {:#X}`, which is at or before it — the declared interior is empty, so this hole can never refuse anything and reads as checked while checking nothing",
                h.after, h.at
            ));
        }

        // ── The permitted occupants: the filler module's sections, from the registry ──
        let permitted: Vec<&str> =
            registry.iter().filter(|m| m.module_id == h.filled_by).map(|m| m.section).collect();
        if permitted.is_empty() {
            return Err(format!(
                "[map.hole-filler-unknown] declared hole after `{}` (at {:#X}) is filled_by `{}`, which names no module in this shape's registry — the permitted-occupant set is empty, so the intended filler itself would read as an intruder and the answer would be a fault about the wrong thing",
                h.after, h.at, h.filled_by
            ));
        }
        if !resolved.iter().any(|s| permitted.contains(&s.name.as_str())) {
            return Err(format!(
                "[map.hole-filler-absent] declared hole after `{}` (at {:#X}) is filled_by `{}` (section(s) {:?}), and this build placed none of them — the hole's filler is not in the layout, so an empty interior would mean the hole is UNFILLED rather than reserved",
                h.after, h.at, h.filled_by, permitted
            ));
        }

        // ── The occupants: every byte-emitting ROM section overlapping [start, at) ──
        for s in resolved.iter().filter(|s| is_rom_section(s)) {
            let len = s.image_bytes().len() as u32;
            if len == 0 || permitted.contains(&s.name.as_str()) {
                continue;
            }
            let (lo, hi) = (s.lma.max(start), s.lma.saturating_add(len).min(h.at));
            if lo >= hi {
                continue;
            }
            faults.push(format!(
                "[map.hole-interior-occupied] the hole declared after `{}` — interior [{start:#X},{:#X}), reserved for `{}` — is occupied at [{lo:#X},{hi:#X}) by byte-emitting section `{}` (head `{}`). Either that section drifted into the reserved span, or the hole's declared `at` no longer matches this layout; either way the post-hole data does not resume at {:#X} the way the map says it does.",
                h.after,
                h.at,
                h.filled_by,
                s.name,
                head_label(s).unwrap_or("<label-less>"),
                h.at,
            ));
        }
    }
    Ok(faults)
}

/// An assembled ROM image with the artefacts derived from the same build: the
/// sigil-canonical symbol listing (the deb2-appendix source) and the `.emp`
/// lowering's [`BuildWarning`]s, carried out to whichever caller renders them.
pub struct RomBuild {
    /// The assembled ROM image (checksum already folded by `emit_rom`).
    pub rom: Vec<u8>,
    /// One `C` row per resolved section label at its final VMA.
    pub listing: Vec<sigil_link::ListingSymbol>,
    /// Every non-error diagnostic the `.emp` lowering produced.
    pub warnings: Vec<BuildWarning>,
}

/// The chained whole-ROM build AND its sigil-canonical listing (the deb2-appendix
/// source for the off-canonical full-file layer). One `C` row per resolved section
/// label at its final VMA, de-duplicated and address-deterministic — mirrors
/// `build_native_rom_with_listing`'s listing derivation, on the chained (Frozen)
/// placement instead of the pinned one.
pub fn build_rom_chained_with_listing(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<RomBuild, String> {
    if profile.sound_on {
        ensure_generated(aeon);
    }
    let as_side = assemble_as_side(aeon, profile)?;
    let EmpProgram { sections: emp_sections, link_asserts, mut warnings, sources } =
        build_emp(aeon, profile)?;
    // An author-written `warning` in the residual AS joins the build's warn tier
    // through the same vector as every `.emp` lint, so it reaches the CLI banner
    // and the tally line rather than stopping at the seam.
    warnings.extend(as_side.warnings);
    let mut sections: Vec<Section> = as_side.module.sections;
    sections.extend(emp_sections);

    // Parcel K5: the per-game placement map (`games/<g>/map.toml`) is loaded UP FRONT — its
    // declared `order` DRIVES the packing walk (the frozen provisional bases no longer
    // author the sequence), and the same file's regions/anchors/budget drive emit_rom and
    // the post-resolve `validate_placement`.
    let map_path = profile.map_path(aeon);
    let map_src = std::fs::read_to_string(&map_path)
        .map_err(|e| format!("read {}: {e}", map_path.display()))?;
    let map = sigil_link::load_map(&map_src)
        .map_err(|e| format!("load {}: {e}", map_path.display()))?;
    let mut pmap = crate::map_placement::load_placement_map(&map_src)
        .map_err(|e| format!("placement {}: {e}", map_path.display()))?;

    // FIXTURE-ONLY (stress-art) — the SECOND half of the fixture_placement waiver PAIR
    // (the first is the packing-guard waiver in `packed_true_bases`). The uniquified pool
    // grows past OJZ's pre-DAC-anchor hole, so relocate the manifest-pointed pool section
    // past the sound banks (see `relocate_fixture_pool`). The relocated order is fed to BOTH
    // the packer (below) and `validate_placement` (post-resolve), so the map-order
    // subsequence check passes against the fixture's ACTUAL order — one flag gates both
    // waivers, and neither is a blanket bypass. Shipped shapes never enter here
    // (fixture_placement is false, and the CLI refuses --stress-art with any shipped shape).
    if profile.fixture_placement {
        relocate_fixture_pool(&mut pmap.order)?;
    }

    // The declared org-anchor addresses (object bank / DAC / sound) — the fixture keeps ONLY
    // these as prov-gap islands after relocation; shipped shapes ignore the set.
    let anchor_addrs: std::collections::HashSet<u32> =
        pmap.anchors_for(profile.sound_on).map(|a| a.at).collect();
    // The declared order DRIVES the walk; each ROM section's TRUE base, then its exact span.
    // A `[layout.provisional-drift]` from the walk joins the build's warnings and reaches
    // the CLI banner through the same path as every other warning.
    let true_bases = true_bases_by_index(
        &sections,
        &profile.frozen_sizes,
        &pmap.order,
        profile.fixture_placement,
        &anchor_addrs,
        &mut warnings,
        &|span| sources.locate(span),
    )?;
    let spans = declared_spans_by_index(&sections, &true_bases)?;

    let all = apply_declared_chain(sections, &true_bases, &spans);

    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&all, &stubs, true)
        .map_err(|d| format!("declared-chain: resolve_layout: {} diag(s); first {:?}", d.len(), d.first()))?;
    // Same drift partition as the pinned driver: real Value(0) drift is a hard fail;
    // gated-off-twin (unresolvable-extern) guards are inapplicable here.
    let adiags = sigil_link::check_link_asserts(&resolved, &stubs, &link_asserts);
    // A `LinkAssert` carries its own severity: `[layout.odd-item]`'s data-item check
    // is `Level::Warning` and fails at LINK time, so the warn tier is only complete
    // once these join it.
    warnings.extend(collect_warnings(&sources, &[&adiags], None));
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

    // Sigil-canonical listing from the resolved image: one `C` row per label VMA
    // plus one `-` row per folded equate, plus one `-` row per COMMAND-LINE define.
    //
    // The define rows are the third population, and they come from a different place
    // than the other two: a define is supplied BY THE BUILD (`shape_defines` — the
    // profile's built-in rows merged with the game's `map.toml [defines]`), so it is
    // in no source file the resolver walks and had no listing row at all. Its value
    // is exactly what a reader cannot derive from the ROM: `MAX_RING_BUFFER` is 128
    // for sonic4 and 16 for demo, and a tool that wants the ceiling had to hardcode
    // one of the two and be wrong for the other. Listing-only, and refused on a
    // collision — see `game_defines::define_listing_rows`.
    let listing = {
        let mut listing = listing_from_resolved(&resolved, &stubs);
        let defines = shape_defines(profile, aeon)?;
        let rows = crate::game_defines::define_listing_rows(
            &defines,
            &profile.emp_defines,
            &listing,
            &profile.map_path(aeon).display().to_string(),
        )?;
        listing.extend(rows);
        listing
    };

    let linked = sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("declared-chain: link: {} diag(s); first {:?}", d.len(), d.first()))?;
    // Parcel K5: the map DROVE the order above; this post-resolve pass CONFIRMS the drive —
    // every byte-emitting section is declared (completeness) and the resolved layout honours
    // the declared sequence + island anchors + hole (a bug in the drive, or a section the map
    // omits, fails loud). Its regions drive emit_rom + the object-bank budget.
    validate_placement(&resolved, &pmap, profile.sound_on, &profile.registry)?;
    // R7: the declared per-section alignment against the base each section ACTUALLY
    // lands on — the independent instrument for the sections the walk places by a rule
    // other than the declaration (anchors, phase banks, label-less blobs).
    validate_resolved_alignment(&resolved)?;
    validate_sound_fold(aeon, &resolved, profile)?;
    check_object_bank_budget(&resolved, &map, &pmap)?;
    let rom = sigil_link::emit_rom(&linked, &map).map_err(|e| format!("declared-chain: emit_rom: {e}"))?;
    Ok(RomBuild { rom, listing, warnings })
}

/// The off-canonical full-file build: the chained assembled ROM + the sigil-canonical
/// deb2 appendix (the same Option-A post-pipeline as `build_native_full_file`, over the
/// Frozen-placed profile). `debug` selects the convsym range/fixheader shape.
/// SHAPE SPLIT (the crash-report axis, owner-ruled 2026-08-04): same rule as
/// `build_native_full_file` — this models the shipped artifact, and the appendix follows
/// the MD Debugger island, i.e. `debug || crash_report`. Only the `lean` profile ships
/// the assembled image alone.
pub fn build_full_file_chained(aeon: &Path, profile: &GameProfile) -> Result<Vec<u8>, String> {
    // The axis and the registry are reconciled BEFORE either decision below reads it,
    // so a shape whose two declarations disagree is refused rather than silently
    // appendix-less (or appendix-ed against an island it does not place).
    let island = declares_error_handler_island(profile)?;
    let RomBuild { rom, listing, .. } = build_rom_chained_with_listing(aeon, profile)?;
    if !island {
        return Ok(rom);
    }
    // Demo (engine-only) packs a smaller appendix than sonic4/config; floor per game.
    let floor = if profile.game_root_rel.contains("/demo/") {
        DEMO_APPENDIX_FLOOR
    } else {
        SONIC4_APPENDIX_FLOOR
    };
    append_deb2_appendix(aeon, &rom, &listing, profile.debug, floor, island)
}

/// Parcel K5: the profile's declared placement-map `order` (`games/<g>/map.toml`) — the
/// AUTHORITY the packing walk consumes to sequence the byte-emitting sections. A helper so
/// the emit path (which already parses the whole map) and the size-derivation path
/// (`resolve_frozen_sections`) drive from the same declaration.
fn placement_map(aeon: &Path, profile: &GameProfile) -> Result<crate::map_placement::PlacementMap, String> {
    let map_path = profile.map_path(aeon);
    let map_src = std::fs::read_to_string(&map_path)
        .map_err(|e| format!("read {}: {e}", map_path.display()))?;
    crate::map_placement::load_placement_map(&map_src)
        .map_err(|e| format!("placement {}: {e}", map_path.display()))
}

/// Resolve `profile`'s frozen-table chained layout into its final ROM sections (the
/// SAME placement `build_rom_chained_with_listing` emits, minus the drift check / link /
/// emit). The shared substrate for the placement gate and the P4a LMA-correct
/// size-table derivation: both read `section.lma + label.offset` off these sections.
fn resolve_frozen_sections(aeon: &Path, profile: &GameProfile) -> Result<Vec<Section>, String> {
    if profile.sound_on {
        ensure_generated(aeon);
    }
    // Warnings are the BUILD's to print; this resolve is a placement helper and
    // renders nothing, so `as_side.warnings` is dropped here on purpose — the
    // same throwaway treatment the drift warnings below get.
    let as_side = assemble_as_side(aeon, profile)?;
    let mut sections: Vec<Section> = as_side.module.sections;
    sections.extend(build_emp(aeon, profile)?.sections);
    // K5: the declared map order drives the walk, and the declared anchors are its
    // islands — the SAME inputs the emit path feeds it, so this resolve and the build
    // never diverge on a stale provisional gap. Drift warnings are the build's to
    // print; here they go to a throwaway sink.
    let pmap = placement_map(aeon, profile)?;
    let anchor_addrs: std::collections::HashSet<u32> =
        pmap.anchors_for(profile.sound_on).map(|a| a.at).collect();
    let mut drift_sink = Vec::new();
    let true_bases = true_bases_by_index(
        &sections,
        &profile.frozen_sizes,
        &pmap.order,
        profile.fixture_placement,
        &anchor_addrs,
        &mut drift_sink,
        &|_| None,
    )?;
    let spans = declared_spans_by_index(&sections, &true_bases)?;
    let all = apply_declared_chain(sections, &true_bases, &spans);
    let stubs = SymbolTable::new();
    sigil_link::resolve_layout(&all, &stubs, true)
        .map_err(|d| format!("frozen resolve: resolve_layout: {} diag(s); first {:?}", d.len(), d.first()))
}

/// The resolved section layout `build_rom_chained_with_listing` would emit for
/// `profile` — the placement half of the build, exposed so a gate can read every
/// section's final `lma` + labels off the SAME walk the ROM comes from (the
/// error-handler-is-last invariant, the derived-layout gates).
pub fn resolve_frozen_layout(aeon: &Path, profile: &GameProfile) -> Result<Vec<Section>, String> {
    resolve_frozen_sections(aeon, profile)
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
    let table = profile.frozen_sizes.clone();
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
/// The derivation bootstraps off the committed table (the profile's `frozen_sizes`
/// pins the labeled sections to place them), then reads the resolved positions back —
/// so for a byte-correct build it REPRODUCES the committed addresses exactly. That
/// fixpoint IS the proof: sigil's own resolve is now the authority; nothing parses asl.
pub fn derive_frozen_table(
    aeon: &Path,
    profile: &GameProfile,
) -> Result<std::collections::BTreeMap<String, u32>, String> {
    let want: std::collections::HashSet<String> =
        profile.frozen_sizes.keys().cloned().collect();
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
                out.insert(name.clone(), section_end(s));
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
    Ok(build_native_rom_with_listing(aeon, debug)?.rom)
}

/// The native whole-ROM build AND its symbol listing — the S1.4 source. The
/// listing is derived from the SAME resolved sections the ROM bytes come from (one
/// `resolve_layout`), so the `.lst` addresses are exactly the emitted addresses.
/// Every section LABEL becomes an as-`-L` `C` (code/address) row; equates (`-`) are
/// omitted (convsym's `as_lst` reader takes only `C` symbols, §S1.4 note), so the
/// listing is the sigil-canonical debug symbol set — NOT a byte-imitation of asl's
/// name set (Option A: the `.emp` names are the source names going forward).
pub fn build_native_rom_with_listing(aeon: &Path, debug: bool) -> Result<RomBuild, String> {
    // Canonical placement is COMPUTED (packed from live sizes over the map's declared
    // order/anchors), so the canonical build routes through the chained driver — one
    // placement authority for all seven targets.
    build_rom_chained_with_listing(aeon, &sonic4_profile(debug))
}

/// Build the sigil-native SYMBOL LISTING for one shape (Stage-3 P4c: the `pins.rs`
/// source that replaces parsing asl's `.lst`). Resolves the canonical layout,
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
            map.entry(end_name).or_insert_with(|| section_end(s));
        }
    }
    let end_addr = *map
        .get("EndOfRom")
        .ok_or("sigil_native_symbol_listing: `EndOfRom` absent from the resolved layout")?;
    Ok((map, end_addr))
}

/// A placed section's one-past-end ROM address: `lma + image_len` — THE derivation
/// behind every synthesized `<Base>_End` marker and the `section:<name>` region end.
pub fn section_end(s: &Section) -> u32 {
    s.lma.wrapping_add(s.image_len())
}

/// The placed ROM section table for `repin` (REPIN-END): every ROM section's name,
/// LMA and one-past-end, from the same canonical resolve the symbol listing reads.
/// A region boundary spelled `section:<name>` measures against THIS — its `end` is
/// the section's own end, so alignment pad before the successor's head never enters
/// a pin. Zero-byte sections are listed too (their `end == lma`); a `section:` naming
/// one measures exactly zero bytes, loudly visible in the pin.
pub fn section_extents(aeon: &Path, debug: bool) -> Result<Vec<crate::repin::SectionExtent>, String> {
    let resolved = resolve_canonical_sections(aeon, debug)?;
    Ok(resolved
        .iter()
        .filter(|s| is_rom_section(s))
        .map(|s| crate::repin::SectionExtent { name: s.name.clone(), lma: s.lma, end: section_end(s) })
        .collect())
}

/// Phase-bank label LOAD addresses (T4). For every PHASE-BANK ROM section — a
/// `vma:`-windowed bank whose labels resolve at a VMA distinct from where its bytes
/// physically land (`vma_base != lma && vma_base >= $8000`, the `soundbankhead`
/// precedent) — map each label to its LMA (`lma + offset`), NOT the VMA that
/// `sigil_native_symbol_listing` returns.
///
/// repin pins a `phase_bank` region's base to this LMA, so the emitted `Region` base
/// is uniformly the PLACEMENT address in every shape — the same meaning a non-phase
/// region's base already has (there `vma == lma`). Every consumer of a region base
/// (the port gates' reference windows, `repin`'s own re-derivation) therefore reads a
/// LOAD address and never has to ask which of the two a phase bank's pin holds. The
/// phase VMA stays the SOLE property of the section's own `vma:` declaration in the
/// `.emp`.
///
/// Non-phase sections contribute nothing (their `vma == lma`, so the plain VMA listing
/// already IS the LMA). Empty for a program with no phase-bank section.
/// Which ROM SECTION DEFINES each label (parcel R6): `label name → section name`, plus
/// each section's synthesized `<Base>_End` marker attributed to the section it measures.
///
/// This is the discriminator the ADDRESS cannot supply. A region whose `end` names the
/// successor's head label and whose window happens to sit flush against its own content
/// today measures ZERO pad — indistinguishable, by address alone, from a region ending at
/// a label it owns. The two are not the same contract: the first moves when the neighbour
/// moves. Ownership settles it, and it is read from the same resolve the section table is.
///
/// A name defined in more than one section (`text` carries twenty-odd zero-length
/// instances) is DROPPED rather than attributed to an arbitrary one — an ambiguous
/// ownership is no ownership, and the caller must treat it as unknown, never as a match.
pub fn section_label_owners(aeon: &Path, debug: bool) -> Result<HashMap<String, String>, String> {
    let resolved = resolve_canonical_sections(aeon, debug)?;
    let mut owners: HashMap<String, String> = HashMap::new();
    let mut ambiguous: std::collections::HashSet<String> = std::collections::HashSet::new();
    let note = |owners: &mut HashMap<String, String>,
                    ambiguous: &mut std::collections::HashSet<String>,
                    name: String,
                    sec: &str| {
        match owners.entry(name) {
            std::collections::hash_map::Entry::Occupied(e) => {
                if e.get() != sec {
                    ambiguous.insert(e.key().clone());
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(sec.to_string());
            }
        }
    };
    for s in &resolved {
        if !is_rom_section(s) {
            continue;
        }
        for lab in &s.labels {
            note(&mut owners, &mut ambiguous, lab.name.clone(), &s.name);
        }
        if let Some(base) = s.labels.iter().find(|l| l.offset == 0) {
            note(&mut owners, &mut ambiguous, format!("{}_End", base.name), &s.name);
        }
    }
    for name in ambiguous {
        owners.remove(&name);
    }
    Ok(owners)
}

pub fn phase_bank_lmas(aeon: &Path, debug: bool) -> Result<HashMap<String, u32>, String> {
    let resolved = resolve_canonical_sections(aeon, debug)?;
    let mut out = HashMap::new();
    for s in &resolved {
        if !is_rom_section(s) {
            continue;
        }
        let is_phase = s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false);
        if !is_phase {
            continue;
        }
        for l in &s.labels {
            out.insert(l.name.clone(), s.lma.wrapping_add(l.offset));
        }
    }
    Ok(out)
}

/// Load the project region memory map — K5: the per-game `games/sonic4/map.toml` (the
/// sole owner of the region geometry + object-bank budget now that sigil.map.toml retired),
/// the same file the sonic4 emit path reads.
pub fn project_memory_map(aeon: &Path) -> Result<sigil_ir::map::MemoryMap, String> {
    let map_path = sonic4_profile(false).map_path(aeon);
    sigil_link::load_map(&std::fs::read_to_string(&map_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("load {}: {e}", map_path.display()))
}

/// Resolve the SHIPPED canonical layout (the packed chained placement) into its final
/// ROM sections — the substrate `pins.rs` regeneration and the object-bank budget gate
/// read addresses off — the chained resolve over `sonic4_profile`.
pub fn resolve_canonical_sections(aeon: &Path, debug: bool) -> Result<Vec<Section>, String> {
    resolve_frozen_sections(aeon, &sonic4_profile(debug))
}

/// The object code bank's used cursor = the resolved LMA of the map-declared budget
/// cursor label (`cursor_head`, e.g. `DeformTable_Zero`) — the head of the first section
/// PAST the object bank, whose start IS the bank terminus (object code ends where the data
/// region begins; they pack contiguously). This is the map-owned successor to the retired
/// `engine.inc` `__BUDGET_DATA` marker (K4 inc-6B): the object bank and the data region
/// share the `[$10000,$20000)` window and the data region extends BEYOND it, so an LMA
/// window scan cannot separate them — only the declared boundary label can. `None` if the
/// label is absent from the resolved layout (a game that declares no cursor).
pub fn object_bank_cursor(resolved: &[Section], cursor_head: &str) -> Option<u32> {
    for s in resolved {
        for l in &s.labels {
            if l.name == cursor_head {
                return Some(s.lma.wrapping_add(l.offset));
            }
        }
    }
    None
}

/// The `object_bank` budget's declared cursor head-label from a placement map, if any.
fn object_bank_cursor_head(pmap: &crate::map_placement::PlacementMap) -> Option<&str> {
    pmap.budgets
        .iter()
        .find(|b| b.region == "object_bank")
        .and_then(|b| b.cursor.as_deref())
}

/// Enforce the object-code-bank budget the map's `object_bank` region declares: the used
/// cursor (the resolved LMA of the placement map's declared budget cursor label) must not
/// exceed `lma_base + size` (`$20000`). Returns the used byte count. A missing region,
/// undeclared cursor, or absent cursor label is a no-op (`Ok(0)`) — additive, so this is
/// the map-owned successor to `engine.inc`'s `if * > $20000 / error`, not a new gate that
/// can spuriously fail a game that declares neither. Runs on every native build (both the
/// pinned canonical driver and the off-canonical chainer feed it their resolved sections).
pub fn check_object_bank_budget(
    resolved: &[Section],
    map: &sigil_ir::map::MemoryMap,
    pmap: &crate::map_placement::PlacementMap,
) -> Result<u32, String> {
    let Some(head) = object_bank_cursor_head(pmap) else {
        return Ok(0);
    };
    match object_bank_cursor(resolved, head) {
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
/// SHAPE SPLIT (the crash-report axis, owner-ruled 2026-08-04 — this SUPERSEDES the
/// review-item-29 release strip): this models the SHIPPED artifact, and the appendix
/// follows the MD Debugger island, i.e. `debug || crash_report`, mirroring
/// `run_build_native`. This function drives the CANONICAL sonic4 shapes, and both of
/// those set `crash_report` — so the condition is live-but-always-true here, and the
/// `lean` profile is the shape that actually takes the no-appendix arm (through
/// `build_full_file_chained`). Reading the flag through
/// [`declares_error_handler_island`] rather than hardcoding `true` keeps this call site
/// honest if the canonical shapes ever change, and carries the same answer into the
/// blob-label membership check.
pub fn build_native_full_file(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    let island = declares_error_handler_island(&sonic4_profile(debug))?;
    let RomBuild { rom, listing, .. } = build_native_rom_with_listing(aeon, debug)?;
    if !island {
        return Ok(rom);
    }
    append_deb2_appendix(aeon, &rom, &listing, debug, SONIC4_APPENDIX_FLOOR, island)
}

/// The sigil-canonical deb2 appendix size floor for the sonic4/config symbol set
/// (~11 KB and up). Demo — engine-only, far fewer symbols — floors lower
/// (`DEMO_APPENDIX_FLOOR`). Both are PRESENCE controls: a collapsed listing
/// (a handful of symbols → ~0x27 B) still trips them.
pub const SONIC4_APPENDIX_FLOOR: usize = 0x2000;
/// The demo appendix floor (demo's engine-only set packs to ~0x1a0f B).
pub const DEMO_APPENDIX_FLOOR: usize = 0x1000;

/// The label the vendored MD Debugger v2.6 blob is emitted under
/// (`engine/debug/error_handler.emp`). Present in BOTH canonical shapes since the
/// crash-report ruling (owner-ruled 2026-08-04), so the blob-end guard below now binds
/// the SHIPPED release ROM too — it is more load-bearing than it was, not less. The
/// opt-in `lean` profile is the one shape that carries no island and no such label.
pub const ERROR_HANDLER_BLOB_LABEL: &str = "ErrorHandlerBlob";

/// The registry module id of the MD Debugger island — the 12 CPU exception-vector
/// stubs plus the vendored blob ([`ERROR_HANDLER_BLOB_LABEL`] is the blob's label
/// INSIDE it). [`registry`] pushes this row under `debug || crash_report`.
pub const ERROR_HANDLER_MODULE_ID: &str = "engine.debug.error_handler";

/// The registry module id of the lean shape's loud-failure fault handler — the `else`
/// arm of the same [`registry`] split. Exactly one of this and
/// [`ERROR_HANDLER_MODULE_ID`] is placed in any shape: they are the two arms of one
/// `if`, and both occupy the same tail placement slot.
pub const RELEASE_FAULT_MODULE_ID: &str = "engine.system.release_fault";

/// Does `profile` DECLARE the MD Debugger island — i.e. must its build emit
/// [`ERROR_HANDLER_BLOB_LABEL`]?
///
/// THE POINT OF THIS FUNCTION IS THAT IT NEVER READS A LISTING. An expectation
/// derived from the same build output it is used to judge asserts only that the
/// output agrees with itself; the whole value of the blob-label check is that its
/// expectation comes from somewhere the build has not been consulted about yet.
/// Two such places exist, and this reconciles them rather than picking one:
///
///   * THE AXIS — `debug || crash_report` off the [`GameProfile`] literal. The
///     `crash_report` field documents itself as the crash-report shape switch and is
///     set per profile by hand; it is also what every OTHER consumer of the axis reads
///     (the `__MDDBG__` AS define, the `CRASH_REPORT` comptime define, the appendix
///     decision in `build_full_file_chained`).
///   * THE REGISTRY — whether `profile.registry` carries [`ERROR_HANDLER_MODULE_ID`].
///     This is the module list the build is actually handed, so it is upstream of
///     every section, label and listing row the build produces, and downstream of
///     nothing the build decides.
///
/// They are independent: the axis is a field on a struct literal, the registry is the
/// result of a gate expression over that field, and a profile may build its registry
/// from a DIFFERENT `(debug, crash_report)` pair than it stores (`config_b_profile`
/// and `lean_profile` both call [`registry`] with explicit arguments). So a
/// disagreement is a real defect and is reported as one — the two-directional half
/// that a single reading of either source could not give.
///
/// The exclusivity check is the second half: the two handlers are the arms of one
/// `if`, so a registry carrying both or neither has lost the split, and neither
/// "expected" answer would then mean anything.
pub fn declares_error_handler_island(profile: &GameProfile) -> Result<bool, String> {
    let axis = profile.debug || profile.crash_report;
    let island = profile.registry.iter().any(|m| m.module_id == ERROR_HANDLER_MODULE_ID);
    let lean_handler = profile.registry.iter().any(|m| m.module_id == RELEASE_FAULT_MODULE_ID);
    if island == lean_handler {
        return Err(format!(
            "shape `{}`: the fault-handler split is EXCLUSIVE — `{ERROR_HANDLER_MODULE_ID}` \
             and `{RELEASE_FAULT_MODULE_ID}` are the two arms of one `if` in `registry()` \
             and share the same tail placement slot, but this registry places {}. Whichever \
             answer this function returned would be meaningless, so it returns none.",
            profile.name,
            if island { "BOTH" } else { "NEITHER" },
        ));
    }
    if axis != island {
        return Err(format!(
            "shape `{}`: the crash-report AXIS and the REGISTRY disagree about the MD \
             Debugger island. `debug || crash_report` = {axis} (debug = {}, crash_report = \
             {}), but the registry {} `{ERROR_HANDLER_MODULE_ID}` and {} \
             `{RELEASE_FAULT_MODULE_ID}`. These are two independent declarations of one \
             shape fact — the profile literal and the module list the build is handed — so \
             a mismatch means one of them was changed alone. Reconcile them before any \
             gate reads either.",
            profile.name,
            profile.debug,
            profile.crash_report,
            if island { "PLACES" } else { "omits" },
            if lean_handler { "places" } else { "omits" },
        ));
    }
    Ok(axis)
}

/// The length, in bytes, of the vendored MD Debugger v2.6 blob — the opaque
/// `dc.l` transliteration in `engine/debug/error_handler.emp`, NOT the whole island
/// (that is the 0x15A of exception stubs + this 0xF56 blob = 0x10B0).
///
/// Load-bearing because the blob HARDCODES it: two PC-relative `lea`s baked into the
/// vendored bytes (`dc.l $43FA090A` at blob+0x64A and `dc.l $47FA0872` at blob+0x6E2)
/// both resolve to `ErrorHandlerBlob + 0xF56`, one byte past the blob's last byte, and
/// each follows with `cmpi.w #$DEB2,(a1)+`. Upstream's contract is "convsym appends the
/// symbol table immediately after me"; convsym appends at `EndOfRom`. So a shape that
/// carries the blob must satisfy `EndOfRom == ErrorHandlerBlob + 0xF56`.
pub const ERROR_HANDLER_BLOB_LEN: u32 = 0xF56;

/// The sigil-canonical symbol listing of a `resolve_layout` output — the single
/// derivation both native drivers (chained + full-file) use.
///
/// TWO row kinds, because the listing describes two different kinds of name:
///
///  * every section LABEL → an as-`-L` `C` (code/ADDRESS) row at its final VMA.
///    De-duplicated (a label is defined once) and deterministic (`emit_listing`
///    address-sorts). RAM labels (`$FFFFxxxx`) are kept — convsym's
///    `-range 0 FFFFFF` drops them from the deb2 table, but they belong in the
///    full listing for the `s4budget` RAM consumer.
///  * every folded EQUATE → an as-`-L` `-` (VALUE) row. `pub equ` mints a
///    link-level `EquSym` and no label, so before this these names existed in the
///    linker's symbol table and nowhere a tool could read them; a `.emp` module
///    could compute a constant at comptime but not publish it. The `-` marker is
///    the AS form for a value symbol and the discriminator every consumer keys on
///    — see `sigil_link::emit_listing`, which additionally keeps equates out of
///    the Oracle address-listing half.
///
/// Label rows win a name collision (`seen` is seeded by the label pass): a label
/// is an address the debugger must resolve, and `link()` already rejects a genuine
/// equ-vs-label duplicate through its `defined_here` channel, so this is a
/// belt-and-braces tie-break, not a policy.
fn listing_from_resolved(
    resolved: &[sigil_ir::Section],
    stubs: &SymbolTable,
) -> Vec<sigil_link::ListingSymbol> {
    let mut listing: Vec<sigil_link::ListingSymbol> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for sec in resolved {
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
    for (name, value) in sigil_link::resolved_equates(resolved, stubs) {
        if seen.insert(name.clone()) {
            listing.push(sigil_link::ListingSymbol {
                name,
                value: value as u32,
                is_equate: true,
                unused: false,
            });
        }
    }
    listing
}

/// HARD placement guard: for a shape that carries the MD Debugger blob, the deb2
/// appendix MUST start at exactly `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`, i.e. the
/// error_handler island must be the LAST byte-emitting section. `appendix_start` is
/// `EndOfRom` (the assembled image length; the ROM region is based at 0, the same
/// identity the positive control below already relies on).
///
/// `expect_island` says whether this shape carries the island AT ALL, and it must come
/// from somewhere upstream of `listing` — [`declares_error_handler_island`] for a
/// profile-driven caller. It is what makes the check non-vacuous: a listing WITHOUT the
/// blob label is a satisfied contract for the `lean` shape and a renamed label, a
/// re-layout that dropped the island, or a harvest that lost the symbol for every other
/// shape, and nothing in the listing distinguishes those. So both directions of the
/// mismatch are refusals — an expected island whose label never appeared, and an
/// unexpected island whose label did.
///
/// This fails the BUILD rather than warning: the failure it prevents is silent at
/// runtime (the ROM assembles, boots and crashes correctly — it just prints `<unknown>`
/// for every Offset/Caller), so a warning would be read past. It is the enforcement
/// arm of the INVARIANT declared in `games/<g>/map.toml` and documented in
/// `engine/debug/error_handler.emp`.
///
/// The per-shape half of the same contract — that the shapes DECLARING the island are
/// exactly the shapes whose listings define its label — is
/// `tests/error_handler_island_membership.rs`, because the `lean` shape never reaches
/// this function (its callers skip the appendix entirely).
fn check_error_handler_is_last(
    listing: &[sigil_link::ListingSymbol],
    appendix_start: usize,
    expect_island: bool,
) -> Result<(), String> {
    // Same lookup idiom the layout walk uses for `EndOfRom` — match by name.
    let found = listing.iter().find(|l| l.name == ERROR_HANDLER_BLOB_LABEL);
    let blob = match (expect_island, found) {
        (false, None) => return Ok(()),
        (true, None) => {
            return Err(format!(
                "MDDBG island MEMBERSHIP violated: this shape DECLARES the error_handler \
                 island, but its listing defines no `{ERROR_HANDLER_BLOB_LABEL}` among \
                 {} symbol(s), so the blob-end contract has no subject and would have \
                 passed by having nothing to check. The island is either not placed in \
                 this shape (a registry or `order` change), its blob label was renamed in \
                 engine/debug/error_handler.emp, or the listing harvest dropped it. Every \
                 one of those ships a ROM whose crash screen prints `<unknown>` for every \
                 Offset/Caller with no other symptom.",
                listing.len(),
            ));
        }
        (false, Some(b)) => {
            return Err(format!(
                "MDDBG island MEMBERSHIP violated: this shape declares NO error_handler \
                 island, yet its listing defines `{ERROR_HANDLER_BLOB_LABEL}` at {:#x}. \
                 The island and the lean fault handler are the two arms of one registry \
                 `if` and share a placement slot, so a shape carrying both has lost the \
                 split — and the deb2 appendix this shape is not supposed to need would \
                 land somewhere the blob's baked `lea` displacements do not point.",
                b.value,
            ));
        }
        (true, Some(b)) => b,
    };
    let expect = blob.value.wrapping_add(ERROR_HANDLER_BLOB_LEN) as usize;
    if appendix_start == expect {
        return Ok(());
    }
    let drift = appendix_start as i64 - expect as i64;
    Err(format!(
        "MDDBG blob-end contract VIOLATED: the deb2 appendix starts at EndOfRom \
         {appendix_start:#x}, but `{ERROR_HANDLER_BLOB_LABEL}` ({:#x}) + blob length \
         {ERROR_HANDLER_BLOB_LEN:#x} = {expect:#x} — a drift of {drift:+} byte(s). \
         The MD Debugger locates its symbol table with two `lea` displacements BAKED \
         into the vendored blob bytes, both pointing at blob end, so the appendix must \
         start exactly there. {} Fix the `order` list in games/<game>/map.toml so the \
         error_handler island (BusError../ErrorHandlerBlob) is the FINAL byte-emitting \
         section, immediately before EndOfRom — do not touch the blob bytes.",
        blob.value,
        if drift > 0 {
            "Some section is placed AFTER the blob; the blob's `cmpi.w #$DEB2,(a1)+` will \
             read those bytes instead of the table, fail, and every crash-screen \
             Offset/Caller line will silently print `<unknown>`."
        } else {
            "The appendix starts BEFORE blob end — the blob is truncated or \
             ERROR_HANDLER_BLOB_LEN no longer matches the vendored binary."
        }
    ))
}

/// Given the assembled ROM + listing, write both to a temp dir, run
/// `convsym … -output deb2 -a` then `fixheader`, and return the full appended file.
/// Split out so the t24 doctored-listing negative control can feed a mutated set.
///
/// `expect_island` declares whether the shape this (rom, listing) pair came from
/// carries the MD Debugger island, and is the expectation
/// [`check_error_handler_is_last`] is judged against. A profile-driven caller derives
/// it with [`declares_error_handler_island`]; a synthetic probe feeding a listing it
/// built itself declares what that listing represents. It is a parameter rather than
/// an assumption because a listing with no blob label is a satisfied contract for one
/// shape and a silent runtime failure for every other, and this function cannot tell
/// them apart from the listing.
pub fn append_deb2_appendix(
    aeon: &Path,
    rom: &[u8],
    listing: &[sigil_link::ListingSymbol],
    debug: bool,
    min_appendix: usize,
    expect_island: bool,
) -> Result<Vec<u8>, String> {
    // PLACEMENT PRECONDITION (checked BEFORE shelling convsym — the contract is about
    // where the appendix will land, so a violation is knowable from the layout alone).
    check_error_handler_is_last(listing, rom.len(), expect_island)?;

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
    //
    // EQUATES ARE DROPPED HERE, at the deb2 boundary, and the drop is STRUCTURAL —
    // not a reliance on convsym's `as_lst` reader taking only `C` rows. deb2 is the
    // MD Debugger's ADDRESS table (it answers "what code is at this PC?"), and an
    // equate is a value, so an equate has nothing to say to it. Making the exclusion
    // explicit is also what keeps every shipped ROM BYTE-IDENTICAL across this
    // parcel: the appendix is part of the ROM, so one leaked row would grow it and
    // move every golden.
    let addresses: Vec<sigil_link::ListingSymbol> =
        listing.iter().filter(|s| !s.is_equate).cloned().collect();
    let deb2_listing = sigil_link::demangle_symbols(&addresses);
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
        enforce_inapplicable_allowlist_against,
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
        assert!(
            enforce_inapplicable_allowlist_against(&[], &[], STAGE1_INAPPLICABLE_GUARDS).is_ok()
        );
    }

    #[test]
    fn any_poison_guard_is_rejected_against_the_empty_allowlist() {
        // With the allowlist empty, ANY Poison-folding drift guard is a HARD FAIL
        // (the strengthened no-Poison invariant — a new twin-parity guard needs a
        // ruling, not a silent vacation).
        let (ds, as_) = build(&[("SOME_NEW_TWIN_CONST", "camera.emp")]);
        let refs: Vec<&Diagnostic> = ds.iter().collect();
        let err =
            enforce_inapplicable_allowlist_against(&refs, &as_, STAGE1_INAPPLICABLE_GUARDS)
                .unwrap_err();
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
mod declared_alignment_tests {
    //! R7 — the DECLARED per-section alignment gate, both halves.
    //!
    //! The expectations here are derived from `section_align::DECLARED`'s own rows and
    //! from the arithmetic in the doc comments, never from a measurement of the current
    //! layout: `Sfx_33` is declared 8 because aeon's `ensure((winptr(Sfx_33) & 7) == 0)`
    //! says 8, so a base ≡ 4 (mod 8) must be refused whatever the frozen table holds.
    use super::{validate_declared_alignment, validate_resolved_alignment};
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement};
    use sigil_span::Span;

    fn span0() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }

    /// One ROM section with a single head label at offset 0.
    fn sec(name: &str, label: &str, lma: u32) -> Section {
        Section {
            name: name.into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0u8; 0x10],
                fixups: vec![],
                span: span0(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0x10,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    #[test]
    fn a_section_with_no_declaration_is_refused_by_name_before_the_walk() {
        let secs = vec![sec("mystery", "NoSuchHeadLabelAnywhere", 0x1000)];
        let e = validate_declared_alignment(&secs).unwrap_err();
        assert!(e.contains("[layout.undeclared-alignment]"), "{e}");
        assert!(e.contains("NoSuchHeadLabelAnywhere"), "{e}");
        assert!(e.contains("has NO declared alignment"), "{e}");
        // The absent declaration is never rendered as a number.
        assert!(!e.contains("declares alignment"), "an absent row must not read as a value: {e}");
        assert!(!e.contains("inferred"), "there is no inference to quote: {e}");
    }

    /// The absent declaration must never be rendered as a pass — the same section
    /// passes only once a declaration exists for it.
    #[test]
    fn a_declared_section_at_a_conforming_base_passes() {
        let secs = vec![sec("sfx_bank_blob", "Sfx_33", 0xA3B20)];
        validate_declared_alignment(&secs).unwrap();
        validate_resolved_alignment(&secs).unwrap();
    }

    /// The pre-walk half is about DECLARATION, not position: the frozen provisional base
    /// is not an alignment input, so a section whose pin sits four bytes off its
    /// requirement passes this half (the walk rounds it to the declaration) — and the
    /// resolved half is what refuses a base the ROM would actually be emitted at.
    #[test]
    fn a_resolved_base_that_violates_the_declaration_is_refused() {
        let secs = vec![sec("sfx_bank_blob", "Sfx_33", 0xA3B24)];
        validate_declared_alignment(&secs).unwrap();
        let e = validate_resolved_alignment(&secs).unwrap_err();
        assert!(e.contains("[layout.alignment-violated]"), "{e}");
        assert!(e.contains("declares alignment 8"), "{e}");
        assert!(e.contains("sfx_bank_blob.emp"), "source not named: {e}");
        assert!(e.contains("base % 8 = 4"), "residue not named: {e}");
    }

    /// The Z80 bank window (`$8000`) is a requirement the walk never rounds to — the
    /// bank heads are declared anchors, held absolute — so the resolved half is the only
    /// instrument on it, and it must read the requirement itself, not a capped quantum.
    #[test]
    fn the_bank_window_requirement_is_measured_on_the_resolved_anchor() {
        let ok = vec![sec("dac_banks", "Dac_Temp_Blip", 0x90000)];
        validate_resolved_alignment(&ok).unwrap();
        let bad = vec![sec("dac_banks", "Dac_Temp_Blip", 0x90010)];
        let e = validate_resolved_alignment(&bad).unwrap_err();
        assert!(e.contains("declares alignment 32768"), "{e}");
        assert!(e.contains("base % 32768 = 16"), "{e}");
    }

    /// A section no frozen table names is in the pre-walk half's scope exactly like a
    /// pinned one: the walk rounds it to its declaration too, so it needs a row. An
    /// undeclared one is refused; a declared one at a bad resolved base is refused by
    /// the resolved half.
    #[test]
    fn an_unpinned_section_needs_a_declaration_and_is_measured_after_the_walk() {
        let undeclared = vec![sec("mystery", "NoSuchHeadLabelAnywhere", 0x2000)];
        let e = validate_declared_alignment(&undeclared).unwrap_err();
        assert!(e.contains("NoSuchHeadLabelAnywhere"), "{e}");
        let secs = vec![sec("palette", "Palette_LoadPal", 0x2001)];
        validate_declared_alignment(&secs).unwrap();
        let e = validate_resolved_alignment(&secs).unwrap_err();
        assert!(e.contains("Palette_LoadPal"), "{e}");
        assert!(e.contains("base % 2 = 1"), "{e}");
    }

    /// A label-less blob has nothing to declare under: both halves skip it.
    #[test]
    fn a_label_less_blob_is_out_of_both_halves_scope() {
        let mut blob = sec("boot_blob", "unused", 0x3D6);
        blob.labels.clear();
        validate_declared_alignment(std::slice::from_ref(&blob)).unwrap();
        validate_resolved_alignment(std::slice::from_ref(&blob)).unwrap();
    }

    /// Every fault in one run, so a report names all of them rather than the first.
    #[test]
    fn every_faulting_section_is_named_in_one_report() {
        let secs = vec![
            sec("mystery_a", "NoSuchHeadLabelAnywhere", 0x1000),
            sec("mystery_b", "NorThisOne", 0x2000),
        ];
        let e = validate_declared_alignment(&secs).unwrap_err();
        assert!(e.contains("2 section(s)"), "{e}");
        assert!(e.contains("NoSuchHeadLabelAnywhere") && e.contains("NorThisOne"), "{e}");
    }

    /// `packed_chained_base` is the walk's alignment rule and seam2's prediction: its
    /// output is a function of the running cursor and the DECLARATION for the head label,
    /// and nothing else. Expectations are the declared rows (8 for `Sfx_33`, 2 for a WORD
    /// section, `$8000` for a bank head) applied to a cursor chosen off every quantum.
    #[test]
    fn packed_chained_base_rounds_to_the_declared_alignment_only() {
        use super::packed_chained_base;
        assert_eq!(packed_chained_base(0x1011, "Sfx_33").unwrap(), 0x1018);
        assert_eq!(packed_chained_base(0x1011, "GameLoop").unwrap(), 0x1012);
        assert_eq!(packed_chained_base(0x1011, "Dac_Temp_Blip").unwrap(), 0x8000);
        assert_eq!(packed_chained_base(0x1018, "Sfx_33").unwrap(), 0x1018, "already aligned: unmoved");
        let e = packed_chained_base(0x1011, "NoSuchHeadLabelAnywhere").unwrap_err();
        assert!(e.contains("[layout.undeclared-alignment]"), "{e}");
        assert!(e.contains("NoSuchHeadLabelAnywhere"), "{e}");
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
    //! Parcel K5 — negative probes proving the map DRIVE-CONFIRMATION has teeth: each lint
    //! (undeclared-island, anchor-absent, order-diverged, order-undeclared) fires on a
    //! doctored map, and the correct map passes. Synthetic resolved layout:
    //! boot head (label-less) @0x0, GameLoop @0x100, ObjCodeBase @0x10000 (ANCHOR_GAP island).
    //! The SEMANTICS inverted at K5 (the map now DRIVES): `order-undeclared` is the
    //! completeness guard (the map must name every emitter), and `order-diverged` is the
    //! drive's own guard (the resolved layout must follow the declared sequence). The
    //! `drives_order_by_map_rank` probe below proves the packer consumes the DECLARATION,
    //! not the frozen provisional bases.
    use super::{hole_interior_faults, packed_true_bases, validate_placement};
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
        assert!(validate_placement(&layout(), &good_map(), false, &[]).is_ok());
    }

    #[test]
    fn undeclared_island_fires() {
        // Drop the 0x10000 anchor — the inferred island is now undeclared.
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n[[anchor]]\nname=\"boot_head\"\nat=0x0\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
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
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.anchor-absent") && e.contains("0x99999"), "{e}");
    }

    #[test]
    fn order_diverged_fires() {
        // K5 drive-confirmation: the resolved layout (GameLoop@0x100 before ObjCodeBase)
        // must contradict a map declaring ObjCodeBase before GameLoop — the packer did not
        // honour the driving order (a packer bug the confirmation catches).
        let m = load_placement_map(
            "order = [\"ObjCodeBase\", \"GameLoop\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-diverged"), "{e}");
    }

    #[test]
    fn order_undeclared_fires() {
        // K5 completeness: omit GameLoop from the order — since the map DRIVES, a
        // byte-emitting section it fails to declare is rejected loud.
        let m = load_placement_map(
            "order = [\"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-undeclared") && e.contains("GameLoop"), "{e}");
    }

    // ── `section:<name>` rows (parcel SECTION-ROW) ──
    // `sec()` names a section `sec{lma}`: GameLoop's section is `sec256`, ObjCodeBase's
    // is `sec65536`. The rows below declare GameLoop by NAME instead of by head label.

    fn anchors() -> &'static str {
        "[[anchor]]\nname=\"boot_head\"\nat=0x0\n[[anchor]]\nname=\"object_bank\"\nat=0x10000\n"
    }

    fn map_with_order(order: &str) -> PlacementMap {
        load_placement_map(&format!("order = [{order}]\n{}", anchors())).unwrap()
    }

    #[test]
    fn section_row_satisfies_completeness() {
        // GameLoop emits bytes and is declared ONLY as `section:sec256` — that row satisfies
        // `[map.order-undeclared]` (the same map with the row dropped fails, per
        // `order_undeclared_fires`).
        let m = map_with_order("\"section:sec256\", \"ObjCodeBase\"");
        validate_placement(&layout(), &m, false, &[]).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn section_row_ranks_where_it_sits_in_validation() {
        // The row sits AFTER ObjCodeBase, but the layout places GameLoop before it: the
        // drive-confirmation must name the row by its `section:` spelling.
        let m = map_with_order("\"ObjCodeBase\", \"section:sec256\"");
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-diverged") && e.contains("`section:sec256`"), "{e}");
    }

    // Real `section_align::DECLARED` rows (WORD, alignment 2): the walk refuses an
    // undeclared head label, so the two packer-rank probes below use these.
    const L_LOW: &str = "GameLoop";
    const L_HIGH: &str = "Section_Init";

    #[test]
    fn section_row_drives_the_packer_rank() {
        // Twin of `drives_order_by_map_rank`, declared by section NAME: `High` (section
        // `sec512`) is named `section:sec512` ahead of `Low` — the walk must place High
        // first at its prov 0x200 and pack Low right after it at 0x210. Control: the
        // label spelling of the same order gives the same bases (one rank, two spellings).
        let secs = vec![sec(L_LOW, 0x100, 0x10), sec(L_HIGH, 0x200, 0x10)];
        let prov = vec![Some(0x100i64), Some(0x200i64)];
        let labeled = vec![true, true];
        let by_name = vec!["section:sec512".to_string(), L_LOW.to_string()];
        let by_label = vec![L_HIGH.to_string(), L_LOW.to_string()];
        let none = std::collections::HashSet::new();
        let got = packed_true_bases(&secs, &prov, &labeled, &by_name, false, &none, &mut Vec::new(), &|_| None).unwrap();
        let want = packed_true_bases(&secs, &prov, &labeled, &by_label, false, &none, &mut Vec::new(), &|_| None).unwrap();
        assert_eq!(got[1], Some(0x200), "High (declared first by name) heads the run at its prov");
        assert_eq!(got[0], Some(0x210), "Low (declared second) packs after High");
        assert_eq!(got, want, "a `section:` row ranks exactly as the label row would");
    }

    #[test]
    fn zero_byte_section_row_is_inert() {
        // A zero-byte section (`sec336`, no label, no bytes) declared by `section:` row is a
        // reserved slot: it trips neither completeness nor divergence, wherever the row sits
        // — including a position the resolved layout contradicts.
        let mut secs = layout();
        secs.push(sec("", 0x150, 0));
        for order in [
            "\"GameLoop\", \"section:sec336\", \"ObjCodeBase\"",
            "\"section:sec336\", \"GameLoop\", \"ObjCodeBase\"",
            "\"GameLoop\", \"ObjCodeBase\", \"section:sec336\"",
        ] {
            let m = map_with_order(order);
            validate_placement(&secs, &m, false, &[]).unwrap_or_else(|e| panic!("order [{order}]: {e}"));
        }
    }

    #[test]
    fn unknown_section_row_fires() {
        // A `section:` row naming a section absent from the build is a named error, and the
        // message carries the row as written so the author can find it.
        let m = map_with_order("\"GameLoop\", \"section:ghost\", \"ObjCodeBase\"");
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-unknown-section") && e.contains("`section:ghost`"), "{e}");
    }

    #[test]
    fn double_declared_section_fires() {
        // GameLoop declared by label AND by `section:sec256`: two rows, one section.
        let m = map_with_order("\"GameLoop\", \"section:sec256\", \"ObjCodeBase\"");
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(
            e.contains("map.order-double-declared") && e.contains("`GameLoop`") && e.contains("`section:sec256`"),
            "{e}"
        );
    }

    /// K5 DRIVE PROOF: the packing walk sequences byte-emitting sections by their MAP RANK,
    /// not by their frozen provisional base. Two labeled sections whose provisional bases
    /// would sort `Low` (prov 0x100) before `High` (prov 0x200) are declared in the OPPOSITE
    /// order (`High` then `Low`); the walk must place `High` first (as the run head, at its
    /// prov 0x200) and pack `Low` immediately after it — proving the declaration drove the
    /// sequence. Under the pre-K5 prov sort the bases would have been Low@0x100, High@0x200.
    #[test]
    fn drives_order_by_map_rank() {
        let secs = vec![sec(L_LOW, 0x100, 0x10), sec(L_HIGH, 0x200, 0x10)];
        let prov = vec![Some(0x100i64), Some(0x200i64)];
        let labeled = vec![true, true];
        let order = vec![L_HIGH.to_string(), L_LOW.to_string()];
        let bases = packed_true_bases(&secs, &prov, &labeled, &order, false, &std::collections::HashSet::new(), &mut Vec::new(), &|_| None).unwrap();
        // High is the run head (declared first) → its provisional base 0x200; Low packs
        // right after it at 0x210 — the layout follows the MAP, inverting the prov order.
        assert_eq!(bases[1], Some(0x200), "High (declared first) anchors at its prov");
        assert_eq!(bases[0], Some(0x210), "Low (declared second) packs after High");
        // Control: with the map order empty (no drive) the walk falls back to the prov
        // sort — Low@0x100 first, High packs after at 0x110.
        let none: Vec<String> = vec![];
        let baked = packed_true_bases(&secs, &prov, &labeled, &none, false, &std::collections::HashSet::new(), &mut Vec::new(), &|_| None).unwrap();
        assert_eq!(baked[0], Some(0x100), "prov fallback: Low at its prov");
        assert_eq!(baked[1], Some(0x110), "prov fallback: High packs after Low");
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
        assert!(validate_placement(&secs, &m, true, &[]).is_ok());
        // sound_off with the phase bank still present → it's an undeclared island (gate excludes it).
        let e = validate_placement(&secs, &m, false, &[]).unwrap_err();
        assert!(e.contains("map.undeclared-island") && e.contains("0x58000"), "{e}");
    }

    // ── The `[[hole]]` half (HOLE-INTERIOR-RESERVED) ────────────────────────────────
    //
    // Synthetic geometry modelled on the real one: a head section, then the hole's
    // filler at the `after` label, then the post-hole section. `sec()` names a section
    // `sec{lma}`, so the filler at 0x3D0 is section `sec976` — which is the name the
    // registry rows below bind to the filler MODULE id, exactly as a shipped profile's
    // registry binds `engine.z80_init` to `z80_idle`.

    /// The filler section's synthetic name (`sec()`'s naming rule at lma 0x3D0).
    const FILLER_SECTION: &str = "sec976";
    const FILLER_MODULE: &str = "engine.filler";

    fn filler_registry() -> Vec<super::ModuleSpec> {
        vec![super::ModuleSpec {
            module_id: FILLER_MODULE,
            section: FILLER_SECTION,
        }]
    }

    /// A map declaring one always-on hole: opens at `Filler`, runs to `at`, filled by
    /// `FILLER_MODULE`. `at` is a parameter so a probe can move the declared right edge
    /// without touching the layout.
    fn hole_map(at: u32) -> PlacementMap {
        load_placement_map(&format!(
            "order = [\"Head\", \"Filler\", \"PostHole\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = {at}\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap()
    }

    /// Head @0x0 (0x3D0 B) — filler @0x3D0 (0x28 B) — post-hole @0x3F8 (0xE B).
    /// One contiguous run, so the whole layout is a single island at 0x0.
    fn hole_layout() -> Vec<Section> {
        vec![sec("Head", 0x0, 0x3D0), sec("Filler", 0x3D0, 0x28), sec("PostHole", 0x3F8, 0xE)]
    }

    /// THE PIN FOR `[map.hole-anchor-missing]` — the presence half of the shipped path's
    /// hole arm. A layout whose `after` label is nowhere (the filler section dropped, as
    /// a shape gate or a rename would drop it) must be refused by name, and by THIS name
    /// rather than the interior half's `[map.hole-anchor-unresolved]`, which the same
    /// layout would otherwise reach.
    #[test]
    fn hole_anchor_missing_fires() {
        let layout: Vec<Section> =
            hole_layout().into_iter().filter(|s| s.name != FILLER_SECTION).collect();
        let m = load_placement_map(&format!(
            "order = [\"Head\", \"PostHole\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = 0x3F8\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap();
        let e = validate_placement(&layout, &m, false, &filler_registry()).unwrap_err();
        assert!(
            e.contains("map.hole-anchor-missing") && e.contains("`Filler`") && e.contains("0x3F8"),
            "{e}"
        );
        // CONTROL: the same map over the layout that DOES carry the label passes, so the
        // red above is the absent label and not the doctored order/anchor set.
        let full = hole_layout();
        let m_full = hole_map(0x3F8);
        validate_placement(&full, &m_full, false, &filler_registry())
            .unwrap_or_else(|e| panic!("control: {e}"));
    }

    /// THE CONTROL: a hole whose interior holds nothing but its declared filler.
    #[test]
    fn a_hole_holding_only_its_filler_has_no_faults() {
        let faults =
            hole_interior_faults(&hole_layout(), &hole_map(0x3F8), false, &filler_registry())
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(faults.is_empty(), "{faults:?}");
    }

    /// RED-FIRST: a byte-emitting section inside the declared interior is named — the
    /// probe shape that returned `Ok` before this predicate existed.
    #[test]
    fn a_section_inside_the_hole_interior_is_named() {
        // The hole is declared to run to 0x406, so the post-hole section at 0x3F8 sits
        // inside it. Nothing about the LAYOUT changes — only the declared right edge.
        let faults =
            hole_interior_faults(&hole_layout(), &hole_map(0x406), false, &filler_registry())
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(faults.len(), 1, "one intruder, one fault: {faults:?}");
        let f = &faults[0];
        assert!(f.contains("map.hole-interior-occupied"), "{f}");
        assert!(f.contains("`Filler`"), "the fault must name the hole it is about: {f}");
        assert!(f.contains("`sec1016`") && f.contains("`PostHole`"), "must name the intruding section and its head label: {f}");
        assert!(f.contains("[0x3F8,0x406)"), "must name the occupied span, not just the hole: {f}");
    }

    /// The occupied span is the INTERSECTION, so a section that only partly reaches into
    /// the hole is reported by the part that is inside it.
    #[test]
    fn a_partial_overlap_reports_the_overlapping_bytes_only() {
        // Declared right edge 0x400: the post-hole section spans [0x3F8,0x406) and only
        // [0x3F8,0x400) of it is inside.
        let faults =
            hole_interior_faults(&hole_layout(), &hole_map(0x400), false, &filler_registry())
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(faults.len(), 1, "{faults:?}");
        assert!(faults[0].contains("[0x3F8,0x400)"), "{}", faults[0]);
    }

    /// LOUD WHEN IT CANNOT MEASURE — the `after` label resolves nowhere. Driven directly
    /// (the shipped path refuses this first with `[map.hole-anchor-missing]`), so the
    /// function cannot report a clean sheet over a hole it never located.
    #[test]
    fn a_hole_whose_after_label_is_absent_cannot_be_measured() {
        let layout: Vec<Section> =
            hole_layout().into_iter().filter(|s| s.name != FILLER_SECTION).collect();
        let e = hole_interior_faults(&layout, &hole_map(0x406), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-anchor-unresolved") && e.contains("`Filler`"), "{e}");
    }

    /// LOUD WHEN IT CANNOT MEASURE — one `after` label defined in two sections has no
    /// single left edge, so the interior is two different spans.
    #[test]
    fn a_hole_whose_after_label_resolves_twice_cannot_be_measured() {
        let mut layout = hole_layout();
        layout.push(sec("Filler", 0x1000, 0x10));
        let e = hole_interior_faults(&layout, &hole_map(0x406), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-anchor-ambiguous") && e.contains("0x1000"), "{e}");
    }

    /// LOUD WHEN IT CANNOT MEASURE — a hole whose `at` is at or before its `after` label
    /// has an empty interior. That is the vacuous pass this predicate exists to remove,
    /// so it is a refusal rather than a clean sheet.
    #[test]
    fn a_hole_with_no_interior_is_refused_rather_than_passing_empty() {
        for at in [0x3D0u32, 0x100] {
            let e = hole_interior_faults(&hole_layout(), &hole_map(at), false, &filler_registry())
                .unwrap_err();
            assert!(
                e.contains("map.hole-bounds-degenerate") && e.contains("0x3D0"),
                "at={at:#X}: {e}"
            );
        }
    }

    /// LOUD WHEN IT CANNOT MEASURE — `filled_by` naming no module in the shape's registry
    /// leaves an empty permitted set, under which the intended filler itself reads as an
    /// intruder and the answer is a fault about the wrong thing.
    #[test]
    fn a_hole_whose_filler_module_is_unknown_is_refused() {
        let e = hole_interior_faults(&hole_layout(), &hole_map(0x406), false, &[]).unwrap_err();
        assert!(e.contains("map.hole-filler-unknown") && e.contains(FILLER_MODULE), "{e}");
    }

    /// LOUD WHEN IT CANNOT MEASURE — the filler module is declared but this build placed
    /// none of its sections, so an empty interior would mean UNFILLED, not reserved.
    #[test]
    fn a_hole_whose_filler_is_not_placed_is_refused() {
        // The `after` label survives on a section that is NOT the filler, so the left edge
        // still resolves and the refusal is specifically about the missing filler.
        let layout = vec![sec("Head", 0x0, 0x3D0), sec("Filler", 0x3D0, 0x28)];
        let registry = vec![super::ModuleSpec {
            module_id: FILLER_MODULE,
            section: "a_section_this_build_does_not_place",
        }];
        let e = hole_interior_faults(&layout, &hole_map(0x406), false, &registry).unwrap_err();
        assert!(e.contains("map.hole-filler-absent") && e.contains(FILLER_MODULE), "{e}");
    }

    /// A hole gated out by `when` contributes nothing — and the emptiness is the SHAPE's,
    /// not the predicate declining to look. The two shapes disagree over one layout.
    #[test]
    fn a_shape_gated_hole_is_absent_from_that_shape() {
        let m = load_placement_map(&format!(
            "order = [\"Head\", \"Filler\", \"PostHole\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = 0x406\nfilled_by = \"{FILLER_MODULE}\"\nwhen = \"sound_off\"\n",
        ))
        .unwrap();
        let off = hole_interior_faults(&hole_layout(), &m, false, &filler_registry()).unwrap();
        assert_eq!(off.len(), 1, "sound_off: the hole is live and occupied: {off:?}");
        let on = hole_interior_faults(&hole_layout(), &m, true, &filler_registry()).unwrap();
        assert!(on.is_empty(), "sound_on: the shape declares no hole at all: {on:?}");
    }

    // ── THROUGH `validate_placement` — the shipped ROM build path ────────────────────
    //
    // The predicate above is reached from the build, so each of its answers must be a
    // BUILD error. Four of its five refusals are reachable here; the fifth,
    // `[map.hole-anchor-unresolved]`, is shadowed by the presence arm that runs first
    // (`hole_anchor_missing_fires` pins that shadowing), so it is deliberately absent.

    /// A byte-emitting section inside the declared interior fails the build.
    #[test]
    fn the_build_path_refuses_an_occupied_hole_interior() {
        let e = validate_placement(&hole_layout(), &hole_map(0x406), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-interior-occupied"), "{e}");
        assert!(e.contains("`PostHole`") && e.contains("[0x3F8,0x406)"), "{e}");
        // CONTROL: the identical layout under the declaration that matches it passes, so
        // the red is the declared right edge and not the fixture.
        validate_placement(&hole_layout(), &hole_map(0x3F8), false, &filler_registry())
            .unwrap_or_else(|e| panic!("control: {e}"));
    }

    /// EVERY occupant is reported, not the first — a stale `at` that swallows two
    /// sections is one declaration to correct, and a caller shown one name fixes it twice.
    #[test]
    fn the_build_path_names_every_occupant_of_one_hole() {
        // `hole_layout` ends at 0x406; a second post-hole section abuts it, and a right
        // edge past both puts both inside. Both bounds are read off the fixture.
        let mut layout = hole_layout();
        layout.push(sec("Trailer", 0x406, 0x8));
        let end = 0x406 + 0x8;
        let m = load_placement_map(&format!(
            "order = [\"Head\", \"Filler\", \"PostHole\", \"Trailer\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = {end}\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap();
        let e = validate_placement(&layout, &m, false, &filler_registry()).unwrap_err();
        assert!(e.contains("`PostHole`") && e.contains("`Trailer`"), "both occupants: {e}");
        assert_eq!(e.lines().count(), 2, "one line per occupant: {e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: a hole whose declared interior is
    /// empty would read as checked while checking nothing.
    #[test]
    fn the_build_path_refuses_a_degenerate_hole() {
        let e = validate_placement(&hole_layout(), &hole_map(0x3D0), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-bounds-degenerate"), "{e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: a `filled_by` naming no module in
    /// the shape's registry leaves nothing permitted, so the answer would be a fault
    /// about the wrong thing. An EMPTY registry is the shape of that mistake.
    #[test]
    fn the_build_path_refuses_an_unknown_filler_module() {
        let e = validate_placement(&hole_layout(), &hole_map(0x406), false, &[]).unwrap_err();
        assert!(e.contains("map.hole-filler-unknown") && e.contains(FILLER_MODULE), "{e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: the filler module is known but this
    /// build placed none of its sections, so the interior is UNFILLED, not reserved. The
    /// `after` label still resolves, so the presence arm passes and this is the refusal.
    #[test]
    fn the_build_path_refuses_a_filler_the_build_did_not_place() {
        let registry = vec![super::ModuleSpec {
            module_id: FILLER_MODULE,
            section: "a_section_this_build_does_not_place",
        }];
        let e =
            validate_placement(&hole_layout(), &hole_map(0x406), false, &registry).unwrap_err();
        assert!(e.contains("map.hole-filler-absent") && e.contains(FILLER_MODULE), "{e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: one `after` label defined in two
    /// sections gives the hole two left edges. The presence arm is satisfied by either,
    /// so only the interior half can catch it.
    #[test]
    fn the_build_path_refuses_an_ambiguous_hole_anchor() {
        let mut layout = hole_layout();
        layout.push(sec("Filler", 0x1000, 0x10));
        // No `order` (the duplicate head label is not the subject here) and the second
        // island declared, so the refusal is the ambiguity and not a map-completeness lint.
        let m = load_placement_map(&format!(
            "[[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"second\"\nat=0x1000\n\
             [[hole]]\nafter = \"Filler\"\nat = 0x406\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap();
        let e = validate_placement(&layout, &m, false, &filler_registry()).unwrap_err();
        assert!(e.contains("map.hole-anchor-ambiguous") && e.contains("0x1000"), "{e}");
    }
}

#[cfg(test)]
mod error_handler_placement_tests {
    //! The MDDBG blob-end contract guard (`check_error_handler_is_last`): the deb2
    //! appendix must start at `ErrorHandlerBlob + 0xF56`, because the vendored blob's
    //! two baked `lea` displacements point there. Negative probe = the historical bug
    //! (`Replay_OJZ_Fixture`, 0x140 B, placed between the blob and `EndOfRom`), which
    //! made every crash-screen line print `<unknown>` with no build-time signal.
    use super::{check_error_handler_is_last, ERROR_HANDLER_BLOB_LEN};

    fn sym(name: &str, value: u32) -> sigil_link::ListingSymbol {
        sigil_link::ListingSymbol { name: name.into(), value, is_equate: false, unused: false }
    }

    /// The real debug-shape geometry: blob @0x5E688, so EndOfRom must be 0x5F5DE.
    fn listing() -> Vec<sigil_link::ListingSymbol> {
        vec![sym("BusError", 0x5E52E), sym("ErrorHandlerBlob", 0x5E688)]
    }

    #[test]
    fn blob_last_passes() {
        let end = (0x5E688 + ERROR_HANDLER_BLOB_LEN) as usize;
        assert!(check_error_handler_is_last(&listing(), end, true).is_ok());
    }

    #[test]
    fn section_after_blob_fires() {
        // The shipped bug: the 0x140-byte replay fixture between blob end and EndOfRom.
        let end = (0x5E688 + ERROR_HANDLER_BLOB_LEN) as usize + 0x140;
        let e = check_error_handler_is_last(&listing(), end, true).unwrap_err();
        assert!(e.contains("MDDBG blob-end contract VIOLATED"), "{e}");
        assert!(e.contains("+320"), "drift must be reported in bytes: {e}");
        assert!(e.contains("<unknown>"), "must name the silent runtime symptom: {e}");
        assert!(e.contains("map.toml"), "must name the fix site: {e}");
    }

    #[test]
    fn appendix_short_of_blob_end_fires() {
        let end = (0x5E688 + ERROR_HANDLER_BLOB_LEN) as usize - 2;
        let e = check_error_handler_is_last(&listing(), end, true).unwrap_err();
        assert!(e.contains("MDDBG blob-end contract VIOLATED") && e.contains("-2"), "{e}");
    }

    #[test]
    fn inert_for_a_shape_that_declares_no_island() {
        // The `lean` shape places the loud-failure handler instead of the island and
        // consults no symbol table, so the placement contract has no subject and the
        // guard must not fire on an arbitrary EndOfRom.
        let lean = vec![sym("ReleaseFault", 0x5CA40)];
        assert!(check_error_handler_is_last(&lean, 0x5CBAE, false).is_ok());
    }

    #[test]
    fn a_declared_island_with_no_blob_label_fires() {
        // The fail-open the `expect_island` parameter exists to close: the same
        // islandless listing, under a shape that DOES declare the island, is a renamed
        // label / dropped island / lost harvest row, not a satisfied contract.
        let no_blob = vec![sym("ReleaseFault", 0x5CA40)];
        let e = check_error_handler_is_last(&no_blob, 0x5CBAE, true).unwrap_err();
        assert!(e.contains("MDDBG island MEMBERSHIP violated"), "{e}");
        assert!(e.contains("<unknown>"), "must name the silent runtime symptom: {e}");
    }

    #[test]
    fn an_undeclared_island_whose_label_appears_fires() {
        // The other direction: a shape that declares no island whose listing defines
        // the blob label anyway has lost the registry's exclusive fault-handler split.
        let e = check_error_handler_is_last(&listing(), 0x5F5DE, false).unwrap_err();
        assert!(e.contains("MDDBG island MEMBERSHIP violated"), "{e}");
        assert!(e.contains("0x5e688"), "must name where the unexpected label sits: {e}");
    }
}

#[cfg(test)]
mod entry_synth_tests {
    //! THE `ItemAuthor::EntrySynth` GROUND. The synthetic entry module is a
    //! setting site of the CodeItem author field with ZERO live constructions:
    //! its every item is a bare `use` line, so it lowers no instruction an
    //! effect lint could charge. This pin is what keeps that true — the day the
    //! synthesis emits anything else, this fails and the emitting code must
    //! stamp its items `EntrySynth` (and declare where their obligations land)
    //! before it ships. The diagnostic-tier guard (the `SourceId` exclusion in
    //! [`super::collect_warnings`]) stays: `use`-statement diagnostics are not
    //! `Instr` items, so the author field cannot carry them.
    use super::synthetic_entry_src;

    #[test]
    fn synthetic_entry_emits_no_code() {
        let src = synthetic_entry_src(&[], "games.sonic4.ram", "games.sonic4.game", &[]);
        let (file, perrs) = sigil_frontend_emp::parse_file(&src, sigil_span::SourceId(0));
        assert!(
            perrs.iter().all(|d| d.level != sigil_span::Level::Error),
            "the synthetic entry must parse clean: {perrs:?}"
        );
        for item in &file.items {
            assert!(
                matches!(item, sigil_frontend_emp::ast::Item::Use(_)),
                "the synthetic entry holds a non-`use` item — it now emits code, so its \
                 items must be `ItemAuthor::EntrySynth`-stamped with a declared \
                 obligation home before shipping: {item:?}"
            );
        }
        assert!(!file.items.is_empty(), "the entry drives reachability via `use` lines");
    }

    /// An extra entry becomes one more `use` edge and nothing else — the flag's whole
    /// mechanism, pinned at the source it generates so a future edge form (a named
    /// import, a glob) cannot slip in unnoticed.
    #[test]
    fn extra_entries_ride_the_use_edge_list() {
        let extras = ["games.a.one".to_string(), "games.b.two".to_string()];
        let src = synthetic_entry_src(&[], "games.sonic4.ram", "games.sonic4.game", &extras);
        assert!(src.contains("\nuse games.a.one\n"), "{src}");
        assert!(src.contains("\nuse games.b.two\n"), "{src}");
        let (file, perrs) = sigil_frontend_emp::parse_file(&src, sigil_span::SourceId(0));
        assert!(
            perrs.iter().all(|d| d.level != sigil_span::Level::Error),
            "the synthetic entry must parse clean with extra entries: {perrs:?}"
        );
        for item in &file.items {
            assert!(matches!(item, sigil_frontend_emp::ast::Item::Use(_)), "{item:?}");
        }
        // With no extras the source carries no extra `use` line.
        let bare = synthetic_entry_src(&[], "games.sonic4.ram", "games.sonic4.game", &[]);
        assert!(!bare.contains("games.a.one"));
    }
}

#[cfg(test)]
mod extra_entry_tests {
    //! `--extra-entry`'s NAME RESOLUTION and its byte-neutrality REFUSAL, over a
    //! two-module scratch tree — so the per-kind verdicts are pinned without a
    //! reference tree, and the aeon-facing gate (`sigil-cli/tests/extra_entry.rs`)
    //! is free to assert the end-to-end contract instead of enumerating kinds.
    use super::{refuse_artifact_contribution, resolve_extra_entry};
    use sigil_frontend_emp::resolve::manifest::Manifest;

    /// Scan a scratch tree of `(relative path, source)` files.
    fn scan(files: &[(&str, &str)]) -> (tempfile::TempDir, Manifest) {
        let dir = tempfile::tempdir().unwrap();
        for (rel, src) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, src).unwrap();
        }
        let (manifest, diags) = Manifest::scan(dir.path());
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "the scratch tree must scan clean: {diags:?}"
        );
        (dir, manifest)
    }

    const GUARD_ONLY: &str = "module pkg.guard_only\nconst K = 1\nensure(K == 1, \"holds\")\n";

    /// A dotted id, an aeon-relative path, and an absolute path all name the same
    /// module — the AUTHORING spelling and the on-disk spelling are both accepted,
    /// and the resolved id is the module's DECLARED one either way.
    #[test]
    fn an_extra_entry_resolves_by_id_and_by_path() {
        let (dir, m) = scan(&[("pkg/guard_only.emp", GUARD_ONLY)]);
        let root = dir.path();
        let abs = root.join("pkg/guard_only.emp");
        for arg in ["pkg.guard_only", "pkg/guard_only.emp", abs.to_str().unwrap()] {
            assert_eq!(
                resolve_extra_entry(&m, root, arg).unwrap(),
                "pkg.guard_only",
                "spelling `{arg}` must resolve"
            );
        }
    }

    /// A name that resolves to nothing is an error naming the argument — never a
    /// silent skip, which would let a lane pass with its subject deleted.
    #[test]
    fn an_unresolvable_extra_entry_is_an_error() {
        let (dir, m) = scan(&[("pkg/guard_only.emp", GUARD_ONLY)]);
        for arg in ["pkg.gone", "pkg/gone.emp", "/nowhere/gone.emp"] {
            let e = resolve_extra_entry(&m, dir.path(), arg).unwrap_err();
            assert!(e.contains(arg), "the error must name the argument: {e}");
            assert!(e.contains("no such module under the scan root"), "{e}");
        }
    }

    /// The byte-neutrality line, EVERY declaration kind the refusal ranges over —
    /// both directions. The ACCEPT rows are the load-bearing half: they are what fails
    /// if the refusal is ever widened by accident (collapsing the region-form `vars`
    /// arm to a bare `Item::Vars(_)` would take the overlay form with it, and an
    /// overlay is a comptime view over bytes that already exist).
    #[test]
    fn only_a_comptime_only_extra_entry_is_accepted() {
        // Each body is appended to a `module pkg.m` line. `(body, refused-kind)`.
        let cases: &[(&str, Option<&str>)] = &[
            // ACCEPTED — comptime-only, zero bytes and zero link symbols.
            ("const K = 1\nensure(K == 1, \"holds\")\n", None),
            ("struct S { a: u8 }\n", None),
            ("enum E: u8 { A = 0, B = 1 }\n", None),
            ("comptime fn f(x: u8) -> u8 { return x }\n", None),
            ("newtype N = u8\n", None),
            ("context ints_off { granted }\n", None),
            ("comptime test \"holds\" { }\n", None),
            // The overlay form of `vars` — a comptime view over an existing window,
            // NOT an allocation. Refusing it would refuse every SST-carrying module.
            ("struct W { pad: [u8; 4] }\nvars V: W.pad { t: u8 }\n", None),
            // `interface`/`implement` reach the whole-program contract bind pass; a
            // duplicate of either is a hard error there, so neither can rebind.
            ("interface I {\n    const C: u8\n}\n", None),
            ("implement I {\n    const C = 1\n}\n", None),
            // REFUSED — ROM producers.
            ("data D: [u8; 2] = [1, 2]\n", Some("data")),
            ("proc P () clobbers() { rts }\n", Some("proc")),
            ("section scratch { }\n", Some("section")),
            ("align 2\n", Some("align")),
            ("offsets O {\n    F0: F = 1\n}\n", Some("offsets")),
            ("table T (cell: *u8, hole: 0) {\n    A: a,\n}\n", Some("table")),
            ("dispatch R (encoding: word_offsets) {\n    A: a,\n}\n", Some("dispatch")),
            (
                "struct S { n: u8 }\nscript B (a0: *S) (encoding: word_offsets) shows done { }\n",
                Some("script"),
            ),
            // REFUSED — a link symbol.
            ("equ Q = 1\n", Some("equ")),
            // REFUSED — RAM allocators (the compiler's own predicate).
            ("region upper_ram @ $FFFF8000 .. $FFFFFFFF\n", Some("region")),
            ("vars upper_ram { a: u8 }\n", Some("vars")),
        ];
        for (body, refused) in cases {
            let src = format!("module pkg.m\n{body}");
            let (_dir, m) = scan(&[("pkg/m.emp", &src)]);
            let got = refuse_artifact_contribution(&m, "pkg.m", "pkg.m");
            match refused {
                None => assert!(got.is_ok(), "`{body}` must be accepted: {got:?}"),
                Some(kind) => {
                    let e = got.expect_err(&format!("`{body}` must be refused"));
                    assert!(e.contains(&format!("`{kind}`")), "must name `{kind}`: {e}");
                    assert!(e.contains("byte-neutral by contract"), "{e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod warn_tier_tests {
    //! THE WARN TIER's collection contract. Every non-error diagnostic the `.emp`
    //! build produces reaches [`EmpProgram::warnings`] located and deduplicated,
    //! and nothing else does: errors abort the build through `Err`, and the
    //! synthetic entry module the harness writes itself is not the reader's code.
    use super::{collect_warnings, BuildWarning};
    use sigil_frontend_emp::resolve::manifest::{Manifest, SourceIndex};
    use sigil_span::{Diagnostic, Level, SourceId, Span};

    /// SourceId(0) is a real fixture on disk whose third line starts at offset 10;
    /// SourceId(1) is the generated entry — a path that does not exist, as
    /// `build_emp` records its synthetic module.
    const REAL: SourceId = SourceId(0);
    const GENERATED: SourceId = SourceId(1);

    /// The index over a manifest whose SourceId(0) is a real file and whose
    /// SourceId(1) is a generated module with no file on disk.
    fn index_over(dir: &std::path::Path) -> SourceIndex {
        SourceIndex::new(&manifest_over(dir))
    }

    fn manifest_over(dir: &std::path::Path) -> Manifest {
        let real = dir.join("real.emp");
        std::fs::write(&real, "module m\n\nproc P () {\n").unwrap();
        let mut sources = std::collections::HashMap::new();
        sources.insert(REAL, real);
        sources.insert(GENERATED, dir.join("__native_flip_entry__.emp"));
        Manifest { modules: Vec::new(), by_id: std::collections::HashMap::new(), sources }
    }

    fn diag(level: Level, source: SourceId, at: u32, message: &str) -> Diagnostic {
        Diagnostic {
            level,
            message: message.to_string(),
            primary: Span { source, start: at, end: at },
        }
    }

    /// The four-source collection: errors are excluded (the build already failed on
    /// them), the generated entry is excluded, and a real warning survives with its
    /// `path:line:col` and its bracketed id parsed out.
    #[test]
    fn collects_only_reportable_non_errors() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        // Offset 10 is on line 3 ("module m\n" = 9 bytes, "\n" = 1).
        let mdiags = vec![diag(Level::Warning, REAL, 10, "[module.path-mismatch] renamed")];
        let pdiags = vec![diag(Level::Error, REAL, 0, "[parse.bad] boom")];
        let bdiags = vec![diag(Level::Warning, GENERATED, 0, "[import.no-names] whole-module")];
        let place = vec![diag(Level::Note, REAL, 0, "[place.fyi] noted")];

        let got = collect_warnings(&m, &[&mdiags, &pdiags, &bdiags, &place], Some(GENERATED));

        let shown: Vec<&str> = got.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(
            shown,
            ["module.path-mismatch", "place.fyi"],
            "errors and generated-entry diagnostics must not be reported: {got:?}"
        );
        assert_eq!(got[0].level, Level::Warning);
        assert_eq!(got[1].level, Level::Note, "the Note tier rides the same channel");
        assert!(
            got[0].location.as_deref().is_some_and(|l| l.ends_with("real.emp:3:1")),
            "a real source must locate to path:line:col, got {:?}",
            got[0].location
        );
    }

    /// One lint firing at one site is ONE warning however many sources replay it —
    /// the reported count is a count of distinct problems. Two firings of the same
    /// lint at DIFFERENT offsets stay two.
    #[test]
    fn deduplicates_a_replayed_diagnostic_but_not_distinct_sites() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        let a = diag(Level::Warning, REAL, 0, "[proc.sr-undeclared] `P` writes `sr`");
        let b = diag(Level::Warning, REAL, 10, "[proc.sr-undeclared] `P` writes `sr`");

        let one = std::slice::from_ref(&a);
        let got = collect_warnings(&m, &[one, one, one, std::slice::from_ref(&b)], Some(GENERATED));

        assert_eq!(got.len(), 2, "replays collapse, distinct sites do not: {got:?}");
        assert_ne!(got[0].location, got[1].location);
    }

    /// Errors alone produce an EMPTY tier: the build fails on them through `Err`,
    /// so reporting them a second time as warnings would double-count.
    #[test]
    fn errors_alone_produce_an_empty_tier() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        let errs = vec![diag(Level::Error, REAL, 0, "[x.y] boom")];
        assert!(collect_warnings(&m, &[&errs], Some(GENERATED)).is_empty());
    }

    /// A FAILING warn-level `LinkAssert` reaches the warn tier.
    ///
    /// `[layout.odd-item]`'s data-item check is `Level::Warning`
    /// (`sigil_ir::LinkAssert::level`) and is evaluated at LINK time, past every
    /// lowering diagnostic source — so the ROM drivers must feed
    /// `check_link_asserts`' output through the collector, or the whole class is
    /// reported nowhere.
    ///
    /// NOT VACUOUS, and it is the only proof available: the aeon corpus records 75
    /// warn-level asserts per sonic4 build and every one of them PASSES, so a
    /// corpus measurement of this path reads identically whether or not the wiring
    /// exists. This drives the failure directly.
    #[test]
    fn a_failing_warn_level_link_assert_survives_the_error_filter() {
        let dir = tempfile::tempdir().unwrap();
        let index = index_over(dir.path());
        let assert_at_odd = sigil_ir::LinkAssert {
            // Folds to 0 — a failure.
            cond: sigil_ir::Expr::Int(0),
            message: vec![sigil_ir::MsgPart::Text(
                "[layout.odd-item] `T` lands at odd address".to_string(),
            )],
            fatal: false,
            level: Level::Warning,
            span: Span { source: REAL, start: 10, end: 10 },
        };
        let adiags = sigil_link::check_link_asserts(
            &[],
            &sigil_ir::SymbolTable::new(),
            std::slice::from_ref(&assert_at_odd),
        );
        assert_eq!(adiags.len(), 1, "a false cond must produce one diagnostic: {adiags:?}");
        assert_eq!(adiags[0].level, Level::Warning, "the assert's own level must ride through");

        let got = collect_warnings(&index, &[&adiags], None);
        assert_eq!(got.len(), 1, "the warn-tier assert must survive the error filter: {got:?}");
        assert_eq!(got[0].id, "layout.odd-item");
        assert!(got[0].location.as_deref().is_some_and(|l| l.ends_with("real.emp:3:1")));

        // The ERROR tier still aborts instead of being reported: the drivers' own
        // `filter(… == Level::Error)` partition must still see it.
        let fatal = sigil_ir::LinkAssert { level: Level::Error, ..assert_at_odd };
        let ediags = sigil_link::check_link_asserts(
            &[],
            &sigil_ir::SymbolTable::new(),
            std::slice::from_ref(&fatal),
        );
        assert!(collect_warnings(&index, &[&ediags], None).is_empty());
    }

    /// The id is the leading bracketed group; a message without one classifies as
    /// the empty id, which the summary shows as `unclassified` and the corpus gate
    /// refuses. Rendering names the level and reads like the error tier.
    #[test]
    fn an_unbracketed_message_has_no_id_and_an_unlocatable_span_renders_bare() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        let idless = vec![diag(Level::Warning, REAL, 0, "no bracket here")];
        let got = collect_warnings(&m, &[&idless], Some(GENERATED));
        assert_eq!(got[0].id, "");
        assert!(
            got[0].to_string().ends_with("real.emp:1:1: warning: no bracket here"),
            "rendered: {}",
            got[0]
        );

        // An unlocatable span degrades to `<level>: <message>`, never to a
        // fabricated position.
        let unlocatable = BuildWarning {
            level: Level::Note,
            id: "a.b".into(),
            location: None,
            message: "[a.b] hello".into(),
            primary: Span { source: GENERATED, start: 0, end: 0 },
        };
        assert_eq!(unlocatable.to_string(), "note: [a.b] hello");
    }
}

#[cfg(test)]
mod derived_layout_tests {
    //! Derived placement (2026-08-26, decision d-7): the frozen provisional bases are a
    //! CHECK on the last refreeze, never a build input that can refuse content growth.
    //! Every expectation below is DERIVED from the synthetic sections' sizes and the
    //! declared anchor set — no address is copied from a pin.
    use super::{packed_true_bases, provisional_drift_warning, BuildWarning};
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement};
    use sigil_span::Span;
    use std::collections::HashSet;

    fn span0() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }

    /// A pure-DATA ROM section (position-independent image) headed by `label`.
    fn data(label: &str, lma: u32, len: usize) -> Section {
        Section {
            name: format!("sec_{label}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::Data(DataFragment { bytes: vec![0u8; len], fixups: vec![], span: span0() })],
            placement: SectionPlacement::Pinned,
            reserved_span: len as u32,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    /// Three contiguous data sections as the frozen table last saw them: `Head` at
    /// 0x1000, `Grown` right after it with a 0x10 allotment, `Tail` after that. The
    /// provisional bases ARE the frozen layout (prov[i] == that layout's base).
    const HEAD: u32 = 0x1000;
    const HEAD_LEN: usize = 0x10;
    const GROWN_ALLOTMENT: usize = 0x10;
    const TAIL_LEN: usize = 0x10;
    const GROWN_PROV: u32 = HEAD + HEAD_LEN as u32;
    const TAIL_PROV: u32 = GROWN_PROV + GROWN_ALLOTMENT as u32;

    fn run(grown_len: usize) -> (Vec<Section>, Vec<Option<i64>>, Vec<bool>, Vec<String>) {
        let secs = vec![
            data(L_HEAD, HEAD, HEAD_LEN),
            data(L_GROWN, GROWN_PROV, grown_len),
            data(L_TAIL, TAIL_PROV, TAIL_LEN),
        ];
        let prov = secs.iter().map(|s| Some(s.lma as i64)).collect();
        let labeled = vec![true; 3];
        let order = vec![L_HEAD.to_string(), L_GROWN.to_string(), L_TAIL.to_string()];
        (secs, prov, labeled, order)
    }

    fn walk(grown_len: usize, anchors: &HashSet<u32>) -> (Vec<Option<u32>>, Vec<BuildWarning>) {
        let (secs, prov, labeled, order) = run(grown_len);
        let mut warnings = Vec::new();
        let bases = packed_true_bases(&secs, &prov, &labeled, &order, false, anchors, &mut warnings, &|_| None)
            .unwrap_or_else(|e| panic!("the walk must not refuse pure-data growth: {e}"));
        (bases, warnings)
    }

    /// The run head is the only anchor these fixtures declare (the boot-head rule).
    fn head_only() -> HashSet<u32> {
        HashSet::from([HEAD])
    }

    fn messages(w: &[BuildWarning]) -> Vec<&str> {
        w.iter().map(|w| w.message.as_str()).collect()
    }

    // The fixtures' head labels are REAL rows of `section_align::DECLARED` (all WORD,
    // alignment 2) because the walk's only alignment input is the declaration and it
    // refuses a head label that has none. The constants keep each section's ROLE in the
    // fixture readable; the value is only what lets the walk place it.
    const L_HEAD: &str = "Vectors";
    const L_GROWN: &str = "GameHeader";
    const L_TAIL: &str = "EntryPoint";
    const L_NEXT: &str = "BootData";
    const L_BANK: &str = "BootData_PostBlob";
    const L_WIDE: &str = "Vectors";
    const L_ALLOT: &str = "GameHeader";
    const L_CODE: &str = "EntryPoint";
    const L_T: &str = "BootData";

    /// THE ALIGNMENT INPUT IS THE DECLARATION, NOT THE PIN'S RESIDUE. `Sfx_33` declares 8
    /// (aeon's mod-8 fold wall) and `GameLoop` declares 2 (the 68000 word rule); both are
    /// pinned here at a 16-aligned provisional base, the residue a pin-reading walk would
    /// turn into a quantum of 16 and a base of 0x1020. After a 0x11-byte head the cursor
    /// is 0x1011, so — derived from the rows and nothing else — `Sfx_33` packs at 0x1018
    /// and `GameLoop` at 0x1012.
    #[test]
    fn the_walk_packs_to_the_declared_alignment_not_the_pin_residue() {
        for (label, want) in [("Sfx_33", 0x1018u32), ("GameLoop", 0x1012)] {
            let secs = vec![data(L_HEAD, HEAD, 0x11), data(label, 0x1020, 0x10)];
            let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
            let labeled = vec![true; 2];
            let order = vec![L_HEAD.to_string(), label.to_string()];
            let mut w = Vec::new();
            let bases =
                packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None)
                    .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(bases[1], Some(want), "`{label}` packs to its declaration, not its pin's residue");
        }
    }

    /// A section no frozen table names (unpinned: no frozen row, baked lma 0) packs by
    /// contiguity ROUNDED to its declaration — the same rule as a pinned section, so the
    /// frozen tables do not decide which sections the declaration binds.
    #[test]
    fn an_unpinned_section_packs_to_its_declaration_too() {
        let secs = vec![data(L_HEAD, HEAD, 0x11), data("Sfx_33", 0, 0x10)];
        let prov = vec![Some(HEAD as i64), Some(0)];
        let labeled = vec![true, false];
        let order = vec![L_HEAD.to_string(), "Sfx_33".to_string()];
        let mut w = Vec::new();
        let bases = packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bases[1], Some(0x1018), "unpinned `Sfx_33` rounds the contiguity cursor to 8");
    }

    /// LOUD ON UNMEASURABLE, in the walk itself: a chained section whose head label has
    /// no declaration is refused by the walk, never handed 1, 2 or 16.
    #[test]
    fn an_undeclared_head_label_is_refused_by_the_walk_itself() {
        let secs = vec![data(L_HEAD, HEAD, HEAD_LEN), data("NoSuchHeadLabelAnywhere", 0x1010, 0x10)];
        let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
        let labeled = vec![true; 2];
        let order = vec![L_HEAD.to_string(), "NoSuchHeadLabelAnywhere".to_string()];
        let mut w = Vec::new();
        let e = packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None)
            .expect_err("an undeclared section must not be placed");
        assert!(e.contains("[layout.undeclared-alignment]"), "{e}");
        assert!(e.contains("NoSuchHeadLabelAnywhere"), "{e}");
    }

    /// THE ZERO-BYTE TERMINUS lands exactly at the image end by its own declaration (2),
    /// with no special case in the walk: after an 0x12-byte head the cursor is 0x1012 and
    /// `EndOfRom` packs there, not at the 16-aligned 0x1020 its pin sits at.
    #[test]
    fn the_zero_byte_terminus_packs_to_the_image_end_by_declaration() {
        let secs = vec![data(L_HEAD, HEAD, 0x12), data("EndOfRom", 0x1020, 0)];
        let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
        let labeled = vec![true; 2];
        let order = vec![L_HEAD.to_string(), "EndOfRom".to_string()];
        let mut w = Vec::new();
        let bases = packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bases[1], Some(0x1012), "the terminus names the image end, with no fill gap");
    }

    /// (ii) FOLD IDENTITY: at unchanged sizes every base reproduces its provisional and
    /// no drift is reported — the byte-neutrality every shipped shape rides on.
    #[test]
    fn unchanged_sizes_reproduce_provisional_bases_with_no_warning() {
        let (bases, warnings) = walk(GROWN_ALLOTMENT, &head_only());
        assert_eq!(bases, vec![Some(HEAD), Some(GROWN_PROV), Some(TAIL_PROV)]);
        assert!(warnings.is_empty(), "no drift at unchanged sizes: {:?}", messages(&warnings));
    }

    /// (i) A pure-data section grown 8 KB past its allotment BUILDS: the walk measures
    /// it at scratch (no colliding-pins failure), packs its neighbour downstream by
    /// exactly the growth, and reports ONE `[layout.provisional-drift]` for the
    /// neighbour, carrying the delta. Before this parcel the same walk returned
    /// `Err(.. overruns its provisional .. hand ruling needed)`.
    #[test]
    fn grown_pure_data_packs_downstream_and_warns_with_the_delta() {
        let growth: usize = 0x2000;
        let (bases, warnings) = walk(GROWN_ALLOTMENT + growth, &head_only());
        let expect_tail = TAIL_PROV + growth as u32;
        assert_eq!(bases[1], Some(GROWN_PROV), "the grown section keeps its base");
        assert_eq!(bases[2], Some(expect_tail), "Tail packs after the grown section");
        assert_eq!(warnings.len(), 1, "exactly one drift report: {:?}", messages(&warnings));
        let w = &warnings[0];
        assert_eq!(w.id, "layout.provisional-drift");
        assert_eq!(w.level, sigil_span::Level::Warning);
        assert!(w.location.is_none(), "a layout drift has no source line");
        assert!(w.message.starts_with("[layout.provisional-drift]"), "{}", w.message);
        assert!(w.message.contains(&format!("`{L_TAIL}`")), "names the drifted section's head label: {}", w.message);
        assert!(w.message.contains(&format!("delta {:+#x}", growth)), "carries the delta: {}", w.message);
    }

    /// The warning TEXT names the section, its head label, both bases and the delta —
    /// what a landing needs to refreeze without a diff hunt.
    #[test]
    fn drift_warning_names_section_and_delta() {
        let sec = data(L_TAIL, TAIL_PROV, TAIL_LEN);
        let packed = TAIL_PROV as i64 + 0x2000;
        let w = provisional_drift_warning(&sec, packed, TAIL_PROV as i64);
        for needle in [
            "[layout.provisional-drift]",
            &format!("`sec_{L_TAIL}`"),
            &format!("`{L_TAIL}`"),
            &format!("{packed:#x}"),
            &format!("{:#x}", TAIL_PROV),
            "delta +0x2000",
        ] {
            assert!(w.message.contains(needle), "missing {needle:?} in {:?}", w.message);
        }
        // The Display form (what `sigil build` prints) carries the same message.
        assert!(w.to_string().contains("delta +0x2000"), "{w}");
    }

    /// Growth within the tolerance (the everyday parcel) is silent: the packer moves
    /// the neighbour and says nothing, exactly as before.
    #[test]
    fn growth_within_tolerance_moves_the_neighbour_silently() {
        let growth: usize = 0x800;
        let (bases, warnings) = walk(GROWN_ALLOTMENT + growth, &head_only());
        assert_eq!(bases[2], Some(TAIL_PROV + growth as u32));
        assert!(warnings.is_empty(), "{:?}", messages(&warnings));
    }

    /// (3) ISLANDS ARE THE DECLARED ANCHORS. A section whose provisional base sits a
    /// stale ANCHOR_GAP-wide hole past its neighbour packs CONTIGUOUSLY unless that
    /// base is a declared `[[anchor]]`; declaring it makes it absolute again. Same
    /// sections, same sizes — only the anchor set differs.
    #[test]
    fn stale_provisional_gap_is_an_island_only_when_declared() {
        let stale_prov: u32 = 0x2000; // > HEAD + HEAD_LEN + ANCHOR_GAP (0x400)
        let secs = vec![data(L_HEAD, HEAD, HEAD_LEN), data(L_NEXT, stale_prov, 0x10)];
        let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
        let labeled = vec![true; 2];
        let order = vec![L_HEAD.to_string(), L_NEXT.to_string()];
        let mut w = Vec::new();
        let undeclared = packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None).unwrap();
        assert_eq!(undeclared[1], Some(HEAD + HEAD_LEN as u32), "undeclared gap packs contiguously");
        let declared = packed_true_bases(&secs, &prov, &labeled, &order, false, &HashSet::from([HEAD, stale_prov]), &mut w, &|_| None).unwrap();
        assert_eq!(declared[1], Some(stale_prov), "a declared anchor stays absolute");
    }

    /// (iii) A DECLARED ANCHOR IS STILL HARD: the walk holds it absolute (it never
    /// repacks a declared island to make room), so growth that runs into it leaves the
    /// packed layout overlapping at its real bases — refused loud, naming the overlap,
    /// never silently moved. Growth that stops SHORT of the anchor builds.
    #[test]
    fn growth_into_a_declared_anchor_still_fails_loud() {
        let anchor: u32 = 0x2000;
        let room = (anchor - GROWN_PROV) as usize; // what fits between Grown's base and the anchor
        let build = |grown_len: usize| {
            let secs = vec![
                data(L_HEAD, HEAD, HEAD_LEN),
                data(L_GROWN, GROWN_PROV, grown_len),
                data(L_BANK, anchor, 0x10),
            ];
            let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
            let labeled = vec![true; 3];
            let order = vec![L_HEAD.to_string(), L_GROWN.to_string(), L_BANK.to_string()];
            let mut w = Vec::new();
            packed_true_bases(&secs, &prov, &labeled, &order, false, &HashSet::from([HEAD, anchor]), &mut w, &|_| None)
        };
        // Fills the room exactly: builds, and the anchor is where the map says.
        let fits = build(room).unwrap_or_else(|e| panic!("growth that fits must build: {e}"));
        assert_eq!(fits[2], Some(anchor), "the declared anchor is held absolute");
        // One byte past the room: refused, and the message says why.
        let err = build(room + 1).expect_err("overrunning a declared anchor must not build");
        assert!(err.contains("declared anchor") && err.contains("overlap"), "{err}");
    }

    // ── measure-at-packed-base (2026-08-26): a section whose LENGTH DEPENDS ON ITS
    // BASE — one `lea Table, aN`-shaped relaxable (RelaxAbsSym) whose operand width
    // is abs.w below $8000 and abs.l above it. Every expectation is DERIVED from the
    // sizes and the $8000 boundary; no address is copied from a run of the code.

    /// A CODE section holding exactly one RelaxAbsSym (`lea (T).w/.l, a0`): 4 bytes
    /// when `T` resolves under $8000 on the 24-bit bus, 6 above.
    fn code_abs(label: &str, lma: u32, target: &str) -> Section {
        use sigil_ir::{Expr, Fixup, FixupKind, RelaxCandidate};
        let sym = || Expr::Sym(target.into());
        let short = RelaxCandidate {
            bytes: vec![0x41, 0xF8, 0, 0],
            fixup: Fixup { kind: FixupKind::Abs16Be, offset: 2, target: sym() },
        };
        let long = RelaxCandidate {
            bytes: vec![0x41, 0xF9, 0, 0, 0, 0],
            fixup: Fixup { kind: FixupKind::Abs32Be, offset: 2, target: sym() },
        };
        Section {
            name: format!("sec_{label}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::RelaxAbsSym { short, long, target: sym(), span: span0() }],
            placement: SectionPlacement::Pinned,
            reserved_span: 4,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    /// The layout that straddles the $8000 boundary: `Wide` fills up to $7FE0, then a
    /// growable data allotment, then the code, then the data section the code's
    /// operand targets. At the frozen sizes everything sits below $8000 (abs.w); a
    /// grown allotment pushes the target past $8000 and the code's own length grows
    /// from 4 to 6 while it moves.
    const WIDE_LEN: usize = 0x6FE0; // Wide: [0x1000, 0x7FE0)
    const ALLOT_PROV: u32 = HEAD + WIDE_LEN as u32; // 0x7FE0
    const ALLOT: usize = 0x10;
    const CODE_PROV: u32 = ALLOT_PROV + ALLOT as u32; // 0x7FF0
    const TARGET_PROV: u32 = CODE_PROV + 4; // 0x7FF4 — the abs.w form's successor slot

    fn boundary_walk(allot_len: usize) -> Result<Vec<Option<u32>>, String> {
        let secs = vec![
            data(L_WIDE, HEAD, WIDE_LEN),
            data(L_ALLOT, ALLOT_PROV, allot_len),
            code_abs(L_CODE, CODE_PROV, L_T),
            data(L_T, TARGET_PROV, 0x10),
        ];
        let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
        let labeled = vec![true; 4];
        let order =
            vec![L_WIDE.to_string(), L_ALLOT.to_string(), L_CODE.to_string(), L_T.to_string()];
        let mut w = Vec::new();
        packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None)
    }

    /// FOLD IDENTITY with a base-dependent length: at the frozen sizes the walk
    /// reproduces every provisional base, and the target's slot proves the code
    /// measured its SHORT form (4 B) below the boundary.
    #[test]
    fn base_dependent_length_reproduces_provisional_bases_at_frozen_sizes() {
        let bases = boundary_walk(ALLOT).unwrap_or_else(|e| panic!("frozen sizes must build: {e}"));
        assert_eq!(
            bases,
            vec![Some(HEAD), Some(ALLOT_PROV), Some(CODE_PROV), Some(TARGET_PROV)],
            "unchanged sizes reproduce the frozen layout"
        );
    }

    /// THE FIXED POINT: growth pushes the code past $8000, where its own operand
    /// widens (4 -> 6 B). The walk must place the target section from the length the
    /// code has AT ITS PACKED BASE — the derived expectation below is the abs.l
    /// arithmetic, and the pre-fix substitute-base measurement had no way to see it.
    #[test]
    fn growth_across_the_boundary_places_the_successor_from_the_long_form() {
        let growth: usize = 0x20;
        let bases = boundary_walk(ALLOT + growth)
            .unwrap_or_else(|e| panic!("growth across the boundary must build: {e}"));
        // Derived: Code packs at align2(0x7FE0 + 0x30) = 0x8010 (its declaration is the
        // 68000 word rule); at 0x8010 its operand target is past $8000, so it measures
        // the LONG form (6 B), and T packs at align2(0x8010 + 6) = 0x8016. The abs.w
        // arithmetic would put T at 0x8014 — asserting 0x8016 is asserting the walk
        // measured the code at its packed base.
        let code = ALLOT_PROV + (ALLOT + growth) as u32; // 0x8010, already even
        assert_eq!(bases[2], Some(code));
        assert_eq!(bases[3], Some((code + 6).div_ceil(2) * 2), "T is placed from the LONG form");
    }

    /// LOUD ON UNMEASURABLE: an operand naming a symbol no section defines cannot be
    /// measured at any base — the walk refuses with the span-pass provenance instead
    /// of packing from a fabricated length.
    #[test]
    fn an_unresolvable_operand_refuses_loud() {
        let secs =
            vec![data(L_HEAD, HEAD, HEAD_LEN), code_abs(L_CODE, HEAD + HEAD_LEN as u32, "Nowhere")];
        let prov: Vec<Option<i64>> = secs.iter().map(|s| Some(s.lma as i64)).collect();
        let labeled = vec![true; 2];
        let order = vec![L_HEAD.to_string(), L_CODE.to_string()];
        let mut w = Vec::new();
        let err = packed_true_bases(&secs, &prov, &labeled, &order, false, &head_only(), &mut w, &|_| None)
            .expect_err("an unresolvable operand must not produce a layout");
        assert!(err.contains("span pass"), "{err}");
        assert!(err.contains("(provisional round)"), "names the round that failed: {err}");
    }

    /// The width-flip report (the non-convergence diagnostic's payload) names the
    /// section, both lengths, the base, and each flipping site as `file:line` with
    /// both encodings' widths.
    #[test]
    fn width_flip_report_names_the_relaxing_site() {
        use super::width_flip_report;
        let secs = vec![code_abs(L_CODE, 0x1000, L_T)];
        let order = vec![0usize];
        let report = width_flip_report(
            &secs,
            &order,
            &[0x4],
            &[vec![(span0(), 4)]],
            &[0x6],
            &[vec![(span0(), 6)]],
            &[Some(0x1000)],
            &|_| Some("games/sonic4/player/player_sensors.emp:202:17".to_string()),
        );
        for needle in [
            &format!("`sec_{L_CODE}`"),
            &format!("(`{L_CODE}`)"),
            "measures 0x4 then 0x6 at base 0x1000",
            "games/sonic4/player/player_sensors.emp:202:17 (4 B -> 6 B)",
        ] {
            assert!(report.contains(needle), "missing {needle:?} in {report:?}");
        }
    }
}
