//! DECLARED SECTION ALIGNMENT — the alignment each ROM section REQUIRES, stated as a
//! reviewed fact with its source, instead of inferred from where the section happens to
//! sit in a frozen address table.
//!
//! ── WHY THIS EXISTS ──
//!
//! `native::packed_align_of` derives a section's packing quantum from its FROZEN
//! PROVISIONAL BASE: the largest power of two in `{16, 8, 4, 2}` that divides it, else
//! 1. Stated as a bound rather than as a procedure: **it only distinguishes residues mod
//! 16** — a base divisible by 32 or by 65536 infers exactly 16, the same as a base
//! divisible by 16 alone.
//!
//! That makes alignment a SIDE EFFECT of where a section landed at the last refreeze,
//! not a property of the section. The function's own doc comment records the incident:
//! commit `2c49f538` moved the SFX pin from `$5BAE8` (`%16 == 8`, quantum 8) to `$5BB10`
//! (`%16 == 0`, quantum 16) and silently doubled the requirement, invalidating the mod-8
//! structural pads aeon had built against the old value. The frozen tables are scheduled
//! to stop being placement authority, and every constraint they encode today must be
//! recaptured as an explicit rule BEFORE that happens or it silently stops being
//! enforced. This table is that recapture for the alignment constraint.
//!
//! ── WHAT IS DECLARED, AND WHAT IS DELIBERATELY NOT ──
//!
//! A row declares the alignment the section REQUIRES — the property that outlives the
//! frozen tables — and names the source the requirement comes from. It does NOT declare
//! the quantum today's inference happens to produce. Those are different objects, and
//! the difference is measured, not assumed:
//!
//!   * Across the seven shipped shapes, **38 of the 86 pinned sections infer a DIFFERENT
//!     quantum in different shapes** — `Ani_Tails` infers 16 in `config_a`/`config_b`/
//!     `s4_debug` and 2 in `s4`/`lean`; `Collected_Init` (`entity_window`) infers all four
//!     of 2, 4, 8 and 16 depending on the shape. A per-section scalar that "equals what
//!     `packed_align_of` infers" therefore does not exist. A per-(section, shape) table
//!     that did would be a mechanical re-encoding of the frozen tables themselves —
//!     an expectation copied from the pin it is checking, which asserts nothing.
//!
//!   * The inferred quantum is not a requirement in the other direction either. No
//!     section's REQUIREMENT exceeds 16 by way of a residue the cap hides: the sections
//!     whose frozen base is divisible by 32 or more are divisible by wildly different
//!     powers in different shapes (`BG_Init`: 16, 32, 512; `Tile_Cache_GetTile`: 16, 32,
//!     64, 256, 2048), which is what coincidence looks like. The three sections that DO
//!     require more than 16 require **`$8000`** (the two Z80 bank heads, one `SetBank`
//!     window) and **`$10000`** (`ObjCodeBase`, aeon's R1 ruling) — never 32, and the cap
//!     hides all three completely (`packed_align_of($90000)` and `packed_align_of($10000)`
//!     are both 16). All three are held at declared `[[anchor]]` addresses, so the
//!     inference is never applied to them; the requirements are recorded here so they
//!     survive the flip.
//!
//! ── HOW THE REQUIREMENT IS CHECKED ── (`native::validate_declared_alignment`,
//! `native::validate_resolved_alignment`)
//!
//! For every pinned ROM section: `required` divides the section's FROZEN PROVISIONAL
//! BASE, and `required` divides the section's RESOLVED LMA in the built layout.
//!
//! The first of those subsumes the inference-facing check without re-deriving the
//! packer's island classification. Proof, for `r ∈ {2,4,8,16}`: `packed_align_of(p)`
//! returns the largest element of `{16,8,4,2}` dividing `p`, so `r | p` implies that
//! element is `≥ r` and (being a power of two) a multiple of `r`, i.e. `r | inferred`;
//! and conversely `inferred | p` always, so `r | inferred` implies `r | p`. The two
//! statements are EQUIVALENT for every requirement the inference can express, and
//! `r | p` additionally stays meaningful for `r > 16`, where the inference cannot.
//!
//! The second is measured against a different artifact — the resolved layout the ROM is
//! emitted from — and so is not a restatement of the first: it also covers the sections
//! the packer places by a rule OTHER than `packed_align_of` (declared anchors, phase
//! banks, the zero-byte-marker cap-at-2 path, label-less contiguity blobs), and it is
//! the assertion that reads identically after the frozen tables are retired.
//!
//! ── AFTER THE FLIP ──
//!
//! `required` becomes the packer's input: `align_up(running, required_for(section))`
//! replaces `align_up(running, packed_align_of(prov))`, and `packed_align_of` is
//! deleted along with the provisional bases it reads. That WILL move bytes — most
//! sections require 2 and are being handed 16 today — which is the flip's own paired
//! freeze, not this table's.
//!
//! ── THE KEY IS THE HEAD LABEL ──
//!
//! Not the section name: section names are NOT unique in the resolved layout (`text`
//! names both the `ObjDef_Static` section and a label-less blob), whereas a head label is
//! a defined symbol and unique by construction. It is also the spelling `map.toml`'s
//! `order` already uses for a label row, including the compiler-minted
//! `__align$games.sonic4.replay_fixture$0`.

/// One section's declared alignment requirement.
pub struct AlignDecl {
    /// The section's HEAD LABEL — its lowest-offset label, the spelling a `map.toml`
    /// `order` label row uses.
    pub label: &'static str,
    /// The alignment the section's base must satisfy, in bytes. A power of two.
    pub required: u32,
    /// Where the requirement comes from. Never a pin, never a measurement of the
    /// current layout — a row whose only justification is "it is 16 today" is exactly
    /// the inference this table replaces.
    pub why: &'static str,
}

/// The universal baseline. The 68000 faults on a word or long access at an odd address,
/// and every ROM section here is either 68000 code or data reached by a word/long read,
/// so every section base must be even. Nothing in aeon's sources asks these sections for
/// more; the wider quanta they receive today are slack the packer's inference hands out,
/// not constraints they impose.
const WORD: &str = "68000 word/long access: section base must be even. No stronger \
                    requirement in aeon sources.";

/// The Z80 bank window. `games/sonic4/map.toml`'s BANK PLACEMENT RULE: "what IS fixed is
/// their alignment (0x8000: one Z80 `SetBank` window)", and `bankid()` is folded as
/// `(lma & $7F8000) >> 15` — a bank whose base is not `$8000`-aligned has no bank id.
/// `seam2::DAC_INTRA_BANK_ALIGN` reads the same `$8000` to place the shared drum bank at
/// `dac_banks + $8000` inside the one section.
const Z80_BANK_WINDOW: &str = "Z80 SetBank window: games/sonic4/map.toml bank placement \
                               rule + bankid() = (lma & $7F8000) >> 15.";

/// The sound fold's mod-8 wall. `games/sonic4/data/sound/sfx_bank_blob.emp`:
/// `ensure((winptr(Sfx_33) & 7) == 0, "SFX block base must be 8-aligned — seam-2's
/// pointer folds pack contiguously, a misaligned base desyncs them from the chainer's
/// 8-aligned placement")`.
const SFX_MOD8: &str = "aeon games/sonic4/data/sound/sfx_bank_blob.emp: \
                        ensure((winptr(Sfx_33) & 7) == 0).";

/// The MT bank carries the pad that keeps the SFX base 8-aligned, and that pad is only
/// correct if the MT bank's own base is 8-aligned. `games/sonic4/data/sound/mt_bank.emp`:
/// `MT_TAIL_PAD = (8 - (_MT_PRETABLE_LEN % 8)) % 8` — "The pad rounds it to ≡ 0 (mod 8);
/// with the section base ≡ 0 (mod 8) …". So `Sfx_33`'s requirement propagates upstream.
const MT_MOD8: &str = "aeon games/sonic4/data/sound/mt_bank.emp MT_TAIL_PAD: the SFX \
                       mod-8 pad is only correct with this section's base ≡ 0 (mod 8).";

/// The object-code bank. Aeon's R1 ruling (2026-08-26, recorded in
/// `docs/superpowers/notes/2026-08-26-placement-constraint-inventory.md`): "`ObjCodeBase`
/// requires a **64 KB-aligned** base (`0x10000` itself is a kept design choice, not a
/// hardware fact)". Every object's SST `code_addr` is a 16-bit `label - ObjCodeBase`
/// displacement, so the whole object-code span must sit in one 64 KB window off that base.
const OBJ_BANK_64K: &str = "aeon R1 ruling 2026-08-26: ObjCodeBase requires a 64 KB-aligned \
                            base — SST code_addr is a 16-bit `label - ObjCodeBase`.";

const fn d(label: &'static str, required: u32, why: &'static str) -> AlignDecl {
    AlignDecl { label, required, why }
}

/// THE DECLARATION. One row per ROM section that carries a frozen provisional base in
/// any shipped shape — i.e. every section whose placement the inference decides.
///
/// A section that is NOT here is not silently defaulted: the gate refuses the build and
/// names the section, its provisional base, and the quantum the inference would have
/// given it, so the fix is one reviewed line with a source rather than a number copied
/// off the layout.
///
/// No count is written here. The list is the count.
pub const DECLARED: &[AlignDecl] = &[
    // ── The Z80 banks: the only requirements above 16 in the corpus ──
    d("Dac_Temp_Blip", 0x8000, Z80_BANK_WINDOW),        // dac_banks
    d("SoundTablesZ80_Head", 0x8000, Z80_BANK_WINDOW),  // soundbankhead (phase bank)
    // ── The sound fold's mod-8 chain ──
    d("Sfx_33", 8, SFX_MOD8),            // sfx_bank_blob
    d("Song_MovingTrucks", 8, MT_MOD8),  // mt_bank_blob
    // ── The object-code bank ──
    d("ObjCodeBase", 0x10000, OBJ_BANK_64K), // objcodebase
    // ── Everything else: the 68000 word rule, and nothing stronger ──
    d("__align$games.sonic4.replay_fixture$0", 2, WORD), // replay_fixture
    d("AnimateSprite", 2, WORD),               // animate
    d("Ani_Particle", 2, WORD),                // particle_anims
    d("Ani_Sonic", 2, WORD),                   // sonic_anims
    d("Ani_Tails", 2, WORD),                   // tails_anims
    d("BgAnim_Init", 2, WORD),                 // bg_anim
    d("BgAnim_Table", 2, WORD),                // ojz_bg_anim
    d("BG_Init", 2, WORD),                     // bg
    d("BootData", 2, WORD),                    // boot_head
    d("BootData_PostBlob", 2, WORD),           // boot_tail
    d("BusError", 2, WORD),                    // error_handler
    d("Camera_Init", 2, WORD),                 // camera
    d("CharacterDefs", 2, WORD),               // characters
    d("CharDef_Sonic", 2, WORD),               // sonic
    d("CharDef_Tails", 2, WORD),               // tails
    d("Collected_Init", 2, WORD),              // entity_window
    d("Collision_GetType", 2, WORD),           // collision_lookup
    d("Collision_ProbeDown", 2, WORD),         // player_sensors
    d("CompressionSelfTest", 2, WORD),         // compression_selftest
    d("Debug_MusicToggle", 2, WORD),           // game_debug
    d("DeformTable_Zero", 2, WORD),            // scene_registry
    d("DemoBox_Main", 2, WORD),                // demo_box
    d("EndOfRom", 2, WORD),                    // epilogue (zero-byte terminus)
    d("EntryPoint", 2, WORD),                  // boot
    d("GameHeader", 2, WORD),                  // header
    d("GameLoop", 2, WORD),                    // game_loop
    d("GameState_Demo_Init", 2, WORD),         // demo_state
    d("GameState_ObjectTest_Init", 2, WORD),   // object_test_state
    d("GameState_OJZScroll_Init", 2, WORD),    // ojz_scroll_test
    d("GetSineCosine", 2, WORD),               // math
    d("HBlank_Install", 2, WORD),              // hblank
    d("HeightMaps", 2, WORD),                  // collision_data
    d("Init_DMA_Queue", 2, WORD),              // dma_queue
    d("InitObjectRAM", 2, WORD),               // core
    d("InitSpriteSystem", 2, WORD),            // sprites
    d("Init_SpriteTable", 2, WORD),            // buffers
    d("Level_LoadArt", 2, WORD),               // load_art
    d("Load_Object", 2, WORD),                 // load_object
    d("Map_Tails", 2, WORD),                   // tails_data
    d("Map_TestObj", 2, WORD),                 // test_mappings
    d("ObjDef_DemoBox", 2, WORD),              // demo_data
    d("ObjDef_PathSwap", 2, WORD),             // path_swap
    d("ObjDef_Static", 2, WORD),               // text
    d("OJZ_Act1_Descriptor", 2, WORD),         // act_descriptor
    d("OJZ_Act_Pool_Page0", 2, WORD),          // ojz_act_pool
    d("OJZ_Palette", 2, WORD),                 // ojz_act_assets
    d("OJZ_Sec0_Blocks", 2, WORD),             // sec_block_blobs
    d("OJZ_Sec0_LocalMap", 2, WORD),           // sec_local_maps
    d("OJZ_Sec0_TypeTable", 2, WORD),          // entity_data
    d("Parallax_Init", 2, WORD),               // parallax
    d("Perform_DPLC", 2, WORD),                // dplc
    d("Plane_Buffer_Reset", 2, WORD),          // plane_buffer
    d("Player_Init", 2, WORD),                 // player_common
    d("PopulateSpawnedPieceCount", 2, WORD),   // children
    d("PState_Air", 2, WORD),                  // player_air
    d("PState_Ground", 2, WORD),               // player_ground
    d("PState_Spindash", 2, WORD),             // player_spindash
    d("Read_Controllers", 2, WORD),            // controllers
    d("ReleaseFault", 2, WORD),                // release_fault
    d("RingBuffer_Add", 2, WORD),              // rings
    d("S4LZ_DecompressDict", 2, WORD),         // s4lz
    d("Section_Init", 2, WORD),                // section
    d("Sound_DebugMirror", 2, WORD),           // sound_debug
    d("Sound_PostByte", 2, WORD),              // sound_api
    d("TailsAppendage_Refresh", 2, WORD),      // tails_appendage
    d("TestAnimated", 2, WORD),                // test_animated
    d("TestChildPart", 2, WORD),               // test_parent
    d("TestChurnObj", 2, WORD),                // test_churn
    d("TestEmitter", 2, WORD),                 // test_emitter
    d("TestEnemy_Init", 2, WORD),              // test_enemy
    d("TestParticle", 2, WORD),                // test_particle
    d("TestPlayer", 2, WORD),                  // test_player
    d("TestSolid_Init", 2, WORD),              // test_solid
    d("TestStatic_Main", 2, WORD),             // test_static
    d("TestStressEmitter", 2, WORD),           // test_stress_emitter
    d("Tile_Cache_GetTile", 2, WORD),          // tile_cache
    d("TouchResponse", 2, WORD),               // collision
    d("VBlank_Handler", 2, WORD),              // vblank
    d("VDP_Shadow_Init", 2, WORD),             // vdp_init
    d("Vectors", 2, WORD),                     // vectors
    d("Z80_IdleProgram", 2, WORD),             // z80_idle
    // ── Sections no frozen table names in any shipped shape ──
    // These carry NO provisional base, so the inference never runs on them: the walk
    // packs them by contiguity from their neighbour. They are declared anyway because
    // the 68000 word rule binds them exactly as hard, and after the flip they are placed
    // by the same declaration as everything else. `validate_resolved_alignment` is what
    // measures them today.
    d("Ability_InstaShield", 2, WORD),         // player_instashield
    d("Ani_DustSpindash", 2, WORD),            // dust_anims
    d("Ani_Knuckles", 2, WORD),                // knuckles_anims
    d("CharDef_Knuckles", 2, WORD),            // knuckles
    d("Climb_WallDist", 2, WORD),              // player_climb
    d("Dust_Tick", 2, WORD),                   // dust_spindash
    d("DustPuff_Spawn", 2, WORD),              // dust_puff
    d("EditorSceneBinding_OJZ_Act1_Sec0", 2, WORD), // ojz_effects_editor_act1
    d("Effects_ResolveParallax", 2, WORD),     // preset
    d("Input_Tick", 2, WORD),                  // replay
    d("Map_DustSpindash", 2, WORD),            // dust_data
    d("Map_Knuckles", 2, WORD),                // knuckles_data
    d("OJZ_TestRaster", 2, WORD),              // ojz_effects
    d("PageCache_Init", 2, WORD),              // page_cache
    d("PageIn_Process", 2, WORD),              // page_in
    d("Palette_LoadPal", 2, WORD),             // palette
    d("PState_Fly", 2, WORD),                  // player_fly
    d("PState_Glide", 2, WORD),                // player_glide
    d("Raster_Install", 2, WORD),              // raster
    d("RingSparkle_Spawn", 2, WORD),           // ring_sparkle
    d("ZX0R_Decompress", 2, WORD),             // zx0_resume
];

/// The declared requirement for a section, by head label. `None` means UNDECLARED — the
/// callers turn that into a refusal naming the section, never into 1, never into 0, and
/// never into a pass.
pub fn required_for(head_label: &str) -> Option<&'static AlignDecl> {
    DECLARED.iter().find(|d| d.label == head_label)
}

/// Every label declared twice. Empty in a well-formed table; a duplicate would let one
/// row shadow another silently, so the self-test below refuses it.
pub fn duplicate_labels() -> Vec<&'static str> {
    let mut seen: Vec<&str> = Vec::new();
    let mut dups: Vec<&str> = Vec::new();
    for row in DECLARED {
        if seen.contains(&row.label) {
            dups.push(row.label);
        } else {
            seen.push(row.label);
        }
    }
    dups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every declared alignment is a power of two and at least 1 — the only shape
    /// `align_up` and a divisibility test are both meaningful over.
    #[test]
    fn every_declared_alignment_is_a_power_of_two() {
        for row in DECLARED {
            assert!(
                row.required >= 1 && row.required.is_power_of_two(),
                "`{}` declares alignment {} — not a power of two",
                row.label,
                row.required
            );
        }
    }

    /// A duplicate row would shadow silently; the lookup takes the first match.
    #[test]
    fn no_label_is_declared_twice() {
        let dups = duplicate_labels();
        assert!(dups.is_empty(), "duplicate declaration rows: {dups:?}");
    }

    /// A row with no source is a number someone read off the layout — the exact thing
    /// this table replaces.
    #[test]
    fn every_row_names_its_source() {
        for row in DECLARED {
            assert!(
                row.why.len() > 20,
                "`{}` declares alignment {} with no usable source: {:?}",
                row.label,
                row.required,
                row.why
            );
        }
    }

    /// The lookup is loud on absence rather than defaulting.
    #[test]
    fn an_undeclared_label_reads_as_none() {
        assert!(required_for("NoSuchSectionHeadLabel").is_none());
    }
}
