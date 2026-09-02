//! Shared test-support for the strict gates and the CLI port tests.
//!
//! Two idioms lived hand-copied across ~9 port/probe test files (`sigil-cli`
//! tests and `sigil-harness` tests). Both crates depend on `sigil-harness`, so
//! this is the one seam both can reach without a `#[path]` include or a new
//! test-only crate; the CLI tests call `sigil_harness::test_support::…` and the
//! harness's own integration tests call the same path.
//!
//! ## 1. The AS-truth equ blob for the `engine.constants` twin
//!
//! `engine/system/constants.emp` (aeon tree) is a drift-guarded MIRROR of
//! AS-side constants. Every sigil test that compiles it must synthesise an
//! AS-side `equ` blob supplying the truth values its `ensure(extern(…))` guards
//! read back through the link seam. **`engine/system/constants.asm` (and
//! `structs.asm` for the `SST_*` field pins) is the SOURCE OF TRUTH** — this
//! module carries those values in ONE place.
//!
//! ### Twin-growth procedure
//!
//! When the `constants.emp` twin grows a new guarded constant: (1) grow the twin
//! in the aeon tree, (2) add the matching `(name, rhs)` pair to
//! [`engine_constant_equs`] (or [`sst_field_equs`]) here, (3) done. No per-file
//! blobs — every gate reads this one list.
//!
//! ### Doctoring seam
//!
//! A drift PROBE deliberately doctors ONE value to prove a guard fires. Rather
//! than re-inventing the whole blob, a probe takes the `(name, rhs)` pairs from
//! [`engine_constant_equs`], post-edits the one entry it wants wrong, and
//! assembles via [`assemble_equ_pairs`] — see `with_engine_constant_override`.
//!
//! ## 2. The drift-guard filter
//!
//! `module.link_asserts` carries BOTH the twin drift guards AND the D2.29
//! `[layout.odd-item]` parity asserts. Counting/checking guards means excluding
//! the parity asserts; [`drift_guards_only`] / [`guard_assert_count`] are the
//! shared idiom.
//!
//! ## 3. The REFERENCE-DEPENDENT guard
//!
//! A port gate compiles sources out of the sibling `aeon` working copy, which a
//! checkout of this repo alone does not have. [`reference_tree`] is the one
//! guard those tests open with: skip green when the named paths are absent, and
//! panic instead under `SIGIL_STRICT_GATE=1` so the pre-merge run cannot skip.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::assert::MsgPart;
use sigil_ir::{Cpu, LinkAssert, Section};
use std::path::PathBuf;

// ── 1. The AS-truth equ blob ────────────────────────────────────────────────

/// The `SST_*` struct-field equs that a standalone gate SUPPLIES so a module's
/// legitimate field-address `extern()`s resolve across the link seam — chiefly
/// `ojz_scroll_test.emp`'s `Player_1 + SST_x_pos` player-field addresses. Post
/// the conv-a structs flip, `sst.emp` OWNS this layout (its 30 drift guards
/// retired; the build harvests the offsets from the struct), so this is a
/// SUPPLY-ONLY blob — the equs match what the harvest injects. `SST_interact`
/// ($4E) is the derived tail word. Ordered as the struct declares them.
/// SOURCE OF TRUTH: `engine/objects/sst.emp`.
pub fn sst_field_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("SST_code_addr", "$00"),
        ("SST_x_pos", "$02"),
        ("SST_y_pos", "$06"),
        ("SST_x_vel", "$0A"),
        ("SST_y_vel", "$0C"),
        ("SST_render_flags", "$0E"),
        ("SST_collision_resp", "$0F"),
        ("SST_mappings", "$10"),
        ("SST_art_tile", "$14"),
        ("SST_width_pixels", "$16"),
        ("SST_height_pixels", "$17"),
        ("SST_anim", "$18"),
        ("SST_subtype", "$19"),
        ("SST_anim_table", "$1A"),
        ("SST_status", "$1E"),
        ("SST_angle", "$1F"),
        ("SST_prev_anim", "$20"),
        ("SST_anim_frame", "$21"),
        ("SST_anim_timer", "$22"),
        ("SST_mapping_frame", "$23"),
        ("SST_prev_frame", "$24"),
        ("SST_sprite_piece_count", "$25"),
        ("SST_parent_ptr", "$26"),
        ("SST_sibling_ptr", "$28"),
        ("SST_slot_tag", "$2A"),
        ("SST_entity_section_id", "$2B"),
        ("SST_entity_list_index", "$2C"),
        ("SST_layer", "$2D"),
        // sst-fold (2026-08-05): frame_off moved into the engine block at $2E
        // (its bug005 bolt-on position was $50), the custom window is the record
        // tail at $30-$4F, and the record is back to $50.
        ("SST_frame_off", "$2E"),
        ("SST_sst_custom", "$30"),
        ("SST_len", "$50"),
        // The engine-owned player-slot tail word (structs.asm: SST_sst_custom +
        // SST_CUSTOM_SIZE - 2 = $4E). Not one of sst.emp's 30 field guards —
        // supplied here so `collision.emp`'s `interact_off()` SST_interact guard
        // resolves its `extern("SST_interact")` across the link seam.
        ("SST_interact", "$4E"),
    ]
}

/// The `Act_*` / `Sec_*` / `DMAEntry_*` / `parallax_config_*` struct-field equs
/// a standalone gate may SUPPLY. Post the conv-a structs flip, `engine.structs`
/// OWNS these layouts (the per-field drift wall retired; the build harvests the
/// offsets from the structs), so this is a SUPPLY-ONLY blob matching what the
/// harvest injects — the values track the struct declarations. Act's `$21` pad is
/// anonymous (covered by `Act_len`). SOURCE OF TRUTH: `engine/structs.emp`.
pub fn act_sec_field_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        // Act (40 bytes / $28) — tracked by `tests/act_fixture_drift.rs` against
        // `harvest_engine_struct_offsets`, which reads the live struct.
        ("Act_sec_grid_ptr", "$00"),
        ("Act_grid_w", "$04"),
        ("Act_grid_h", "$06"),
        ("Act_start_local_x", "$08"),
        ("Act_start_local_y", "$0A"),
        ("Act_start_sec_x", "$0C"),
        ("Act_start_sec_y", "$0D"),
        ("Act_act_bg_layout", "$0E"),
        ("Act_act_bg_tiles", "$12"),
        ("Act_act_parallax_config", "$16"),
        ("Act_act_art_pool_table", "$1A"),
        ("Act_act_art_pool_pages", "$1E"),
        ("Act_edge_mode", "$20"),
        ("Act_pad_21", "$21"),
        ("Act_act_sec_local_maps", "$22"),
        ("Act_act_art_budget", "$26"),
        ("Act_len", "$28"),
        // Sec (66 bytes / $42)
        ("Sec_sec_block_index", "$00"),
        ("Sec_sec_objects", "$04"),
        ("Sec_sec_rings", "$08"),
        ("Sec_sec_plc", "$0C"),
        ("Sec_sec_pal", "$10"),
        ("Sec_sec_parallax_config", "$14"),
        ("Sec_sec_raster_table", "$18"),
        ("Sec_sec_bg_layout", "$1C"),
        ("Sec_sec_type_table", "$20"),
        ("Sec_sec_pal_cycle", "$24"),
        ("Sec_sec_sound_bank", "$28"),
        ("Sec_sec_block_dict", "$2C"),
        ("Sec_sec_anim_blocks", "$30"),
        ("Sec_sec_effects", "$34"),
        ("Sec_sec_flags", "$38"),
        ("Sec_sec_music", "$3A"),
        ("Sec_sec_pcfg_pad_3C", "$3C"),
        ("Sec_sec_camera_lookahead", "$3D"),
        ("Sec_sec_pcfg_pad_3E", "$3E"),
        ("Sec_sec_pcfg_pad_3F", "$3F"),
        ("Sec_sec_block_dict_len", "$40"),
        ("Sec_len", "$42"),
        // DMAEntry (the 14-byte DMA-queue entry twin, tranche 20) — the
        // structs.emp per-field wall reads these like the Act/Sec walls.
        ("DMAEntry_Reg94", "0"),
        ("DMAEntry_SizeH", "1"),
        ("DMAEntry_Reg93", "2"),
        ("DMAEntry_SizeL", "3"),
        ("DMAEntry_Reg97", "4"),
        ("DMAEntry_SrcH", "5"),
        ("DMAEntry_Reg96", "6"),
        ("DMAEntry_SrcM", "7"),
        ("DMAEntry_Reg95", "8"),
        ("DMAEntry_SrcL", "9"),
        ("DMAEntry_Command", "10"),
        // parallax_config (30 bytes / $1E) — moved to engine.structs at the
        // tranche-21 buffers port (2nd .emp consumer).
        // band-ceiling-16 (2026-08-27): MAX_PARALLAX_BANDS 8 -> 16 forced
        // `pcfg_layer_mask` (one bit per band) from u8 to u16, and a u16 needs an
        // EVEN slot. It took $02 — the slot `pcfg_v_factor_fg` held — so the mask's
        // low byte stays at $03 where the u8 sat and bytes $00..$1B of every shipped
        // record are byte-identical. `pcfg_v_factor_fg` is not deleted (it is still a
        // live authoring/schema field, read by both lowerings and scene_equiv_proof's
        // differ); it moved to the tail at $1C. 29 payload bytes do not round to an
        // even record, so $1D is the one byte the evenness costs; `pcfg_bob` occupies
        // it and carries the vertical-bob selector, so the byte is spent rather than
        // reserved and `sizeof(parallax_config)` stays 30.
        ("parallax_config_len", "$1E"),
        ("parallax_config_pcfg_band_count", "0"),
        ("parallax_config_pcfg_v_factor_bg", "1"),
        ("parallax_config_pcfg_layer_mask", "2"),
        ("parallax_config_pcfg_v_center_y", "4"),
        ("parallax_config_pcfg_v_offset", "6"),
        ("parallax_config_pcfg_transition", "8"),
        ("parallax_config_pcfg_deform_speed_fg", "9"),
        ("parallax_config_pcfg_deform_speed_bg", "10"),
        ("parallax_config_pcfg_anchor_ch", "11"),
        ("parallax_config_pcfg_deform_table_fg", "12"),
        ("parallax_config_pcfg_deform_table_bg", "16"),
        ("parallax_config_pcfg_v_deform_table_bg", "20"),
        ("parallax_config_pcfg_v_deform_speed_bg", "24"),
        ("parallax_config_pcfg_v_deform_shift_bg", "25"),
        ("parallax_config_pcfg_anchor_dsa", "26"),
        ("parallax_config_pcfg_anchor_dsb", "27"),
        // The record tail. $1C = 28 is the RESERVED `pcfg_v_factor_fg` rehomed from
        // $02; $1D = 29 is `pcfg_bob`, the byte the even-size requirement costs, now
        // carrying the vertical-bob selector.
        ("parallax_config_pcfg_v_factor_fg", "28"),
        ("parallax_config_pcfg_bob", "29"),
        ("DMAEntry_len", "14"),
    ]
}

/// The engine-constant equs that `engine.constants`'s SURVIVING drift guards
/// read back through `extern()`. Post the Stage-3 P5 constants flip AND the
/// conv-a structs flip, `engine.constants` is the SOLE author of every engine
/// constant and the struct twins own the layouts, so EVERY mirror drift guard
/// retired — including `VDP_Shadow_len`'s (its bridge dissolved when the VdpShadow
/// struct became the length author; vdp_init.emp now pins the const to
/// `sizeof(VdpShadow)` in-file). The list is empty: a module consuming an engine
/// constant imports it via `use engine.constants` (comptime), never through a
/// blob. Kept as the zero-length base so `twin_guards()` derivations stay 0.
pub fn engine_constant_equs() -> Vec<(&'static str, &'static str)> {
    vec![]
}


/// Assemble a list of `(name, rhs)` equ pairs into `Vec<Section>`, appending a
/// `Stub:` label + `dc.w 0` so the equs (defined before any section) flush via
/// the AS front-end's `pending_equ_syms` into a real section. The universal
/// pattern behind every AS-truth-equ helper below.
pub fn assemble_equ_pairs(pairs: &[(&str, &str)]) -> Vec<Section> {
    let mut asm = String::from("cpu 68000\n");
    for (name, rhs) in pairs {
        asm.push_str(name);
        asm.push_str(" = ");
        asm.push_str(rhs);
        asm.push('\n');
    }
    asm.push_str("Stub:\n\tdc.w 0\n");
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (equ pairs): {d:?}"))
        .sections
}

/// The complete AS-truth equ blob for the `engine.constants` twin: the 24
/// engine constants its guards read. For gates that ALSO compile `sst.emp` (its
/// 30 `SST_*` guards), use [`as_engine_constants_and_sst_equs`].
pub fn as_engine_constants_equs() -> Vec<Section> {
    assemble_equ_pairs(&engine_constant_equs())
}

/// The AS-truth equ blob for gates that compile BOTH `constants.emp` and
/// `sst.emp` (e.g. the `collision.emp` / test-object gates): the 30 `SST_*`
/// field pins followed by the 24 engine constants.
pub fn as_engine_constants_and_sst_equs() -> Vec<Section> {
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    assemble_equ_pairs(&pairs)
}

/// The engine-constant pairs with EXACTLY ONE constant's RHS overridden — the
/// drift-probe seam. A probe passes the constant it wants wrong and a doctored
/// RHS; the guard for that constant must then fire loud (naming it), while every
/// other guard still passes. Panics if `name` isn't a real engine constant (so a
/// renamed constant can't silently turn the probe into a no-op).
pub fn with_engine_constant_override(name: &str, rhs: &str) -> Vec<(&'static str, String)> {
    let mut pairs: Vec<(&'static str, String)> =
        engine_constant_equs().into_iter().map(|(n, r)| (n, r.to_string())).collect();
    let slot = pairs
        .iter_mut()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("with_engine_constant_override: `{name}` is not an engine constant"));
    slot.1 = rhs.to_string();
    pairs
}

/// Assemble owned `(name, rhs)` pairs (the shape [`with_engine_constant_override`]
/// returns) — same flush pattern as [`assemble_equ_pairs`].
pub fn assemble_owned_equ_pairs(pairs: &[(&str, String)]) -> Vec<Section> {
    let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(n, r)| (*n, r.as_str())).collect();
    assemble_equ_pairs(&borrowed)
}

// ── 1b. The game-contract env for isolated port oracles (L1 P2) ──────────────

/// Build a resolved game-contract [`InterfaceEnv`](sigil_frontend_emp::contract::InterfaceEnv)
/// for a single-module port oracle. The whole-program build runs the bind pass
/// over its reachable module set; an oracle that lowers ONE engine module naming
/// `Game.MEMBER` / `invoke Game.hook` must synthesize the same env. `iface_src`
/// and `impl_src` are `.emp` sources for the interface and its one `implement`;
/// `defines` feeds any comptime-`if` binding group (the hotkeys shape). Asserts a
/// clean parse + bind — a malformed contract stub is a test bug, not a silent
/// empty env.
pub fn game_contract_env(
    iface_src: &str,
    impl_src: &str,
    defines: &[(String, i128)],
) -> sigil_frontend_emp::contract::InterfaceEnv {
    use sigil_frontend_emp::resolve::contract::{bind, ContractModule};
    let (ef, ed) = sigil_frontend_emp::parse_str(iface_src);
    let (gf, gd) = sigil_frontend_emp::parse_str(impl_src);
    assert!(
        ed.iter().chain(&gd).all(|d| d.level != sigil_span::Level::Error),
        "game_contract_env parse diags: iface={ed:?} impl={gd:?}"
    );
    let eid = ef.module.path.segments.join(".");
    let gid = gf.module.path.segments.join(".");
    let mods =
        [ContractModule { id: &eid, file: &ef }, ContractModule { id: &gid, file: &gf }];
    let (env, diags) = bind(&mods, defines);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_contract_env bind diags: {diags:?}"
    );
    env
}

// ── 1c. The DERIVED game-contract env (reads aeon's own contract) ────────────
//
// [`game_contract_env`] above takes HAND-WRITTEN `.emp` source. That is right for
// a probe that means a SYNTHETIC contract (a malformed stub, a one-member
// interface, a deliberately unbound hook), and wrong for an oracle that means THE
// REAL CONTRACT: a hand-written stub is a second, silent copy of
// `engine/system/game_contract.emp`, and it cannot see a member the engine grew.
// The ring-sparkle parcel proved the failure mode — aeon added
// `hook ring_collected` and `rings.emp` gained one `invoke Game.ring_collected`,
// and `rings_port.rs` (which lowered with NO env at all) died on
// `[contract.unknown-member]`.
//
// Everything below is DERIVED, nothing named twice:
//
//   * the interface is aeon's own `engine/system/game_contract.emp`;
//   * the `implement` is the profile's own manifest, at the path
//     [`GameProfile::game_root_rel`]'s directory + `config/game.emp` — the
//     `map_path` / `reference_tree_for_profile` derivation, one file over;
//   * the manifest's imports (`use games.<g>.constants.{…}`) are followed to
//     their own files and supplied as the bind ambient, so a binding whose value
//     is an imported const (`const ENTRY_ID = GS_OJZ_SCROLL_TEST`) folds;
//   * the parsed manifest's module id is checked against the profile's declared
//     [`GameProfile::manifest_module`], so a moved/renamed manifest is loud
//     rather than a quietly different file.
//
// SCOPE. This binds MEMBERS, not bodies: the bound procs/hooks live in the game's
// own modules, which are not in the two-file bind set, so the §4 subcontract check
// (`bound ⊑ declared`) silently passes here. That check is the WHOLE-PROGRAM
// build's job (and `contract_closure_corpus.rs` gates it); what an isolated port
// oracle needs from this env is that every `Game.MEMBER` / `invoke Game.hook` the
// engine module names RESOLVES, to the same binding the ROM was built with.

/// The engine's contract interface, relative to the aeon tree. ONE declaration
/// site (`engine/system/game_contract.emp`) — the engine half is not per-game, so
/// unlike the manifest it has nothing to derive from a profile.
pub const GAME_CONTRACT_IFACE_REL: &str = "engine/system/game_contract.emp";

/// The profile's game manifest (`games/<g>/config/game.emp`) — the one
/// `implement Game`. DERIVED from [`GameProfile::game_root_rel`] exactly as
/// [`GameProfile::map_path`] derives the placement map: the residual root's
/// directory, one known file inside it.
pub fn game_manifest_path(
    aeon: &std::path::Path,
    profile: &crate::native::GameProfile,
) -> PathBuf {
    let root = std::path::Path::new(profile.game_root_rel);
    let dir = root.parent().unwrap_or(std::path::Path::new(""));
    aeon.join(dir).join("config/game.emp")
}

/// Parse an `.emp` file, panicking (naming the path) if it is missing or carries
/// a parse error. The loudness half of the derived contract: a moved file must
/// never degrade into an empty env that satisfies nothing.
fn parse_emp_or_panic(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("game contract source missing at {}: {e}", path.display()));
    let (file, diags) = sigil_frontend_emp::parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors in {}: {:?}",
        path.display(),
        diags
    );
    file
}

/// Every member `interface Game` DECLARES, in declaration order, read out of
/// aeon's `engine/system/game_contract.emp`.
///
/// The DERIVED expectation for the coverage gate: a test asserting the env
/// carries "every member" must get the member list from the contract itself, or
/// it is the same hand-kept list one layer up — and the next hook walks past it
/// exactly as `ring_collected` walked past the stubs.
pub fn game_contract_declared_members(aeon: &std::path::Path) -> Vec<String> {
    let path = aeon.join(GAME_CONTRACT_IFACE_REL);
    let file = parse_emp_or_panic(&path);
    let members: Vec<String> = file
        .items
        .iter()
        .filter_map(|it| match it {
            sigil_frontend_emp::ast::Item::Interface(d) if d.name == "Game" => Some(d),
            _ => None,
        })
        .flat_map(|d| d.members.iter().map(|m| m.name.clone()))
        .collect();
    assert!(
        !members.is_empty(),
        "{} declares no `interface Game` members — the contract moved or the parse \
         stopped seeing it; an empty member list would make every coverage gate vacuous",
        path.display()
    );
    members
}

/// The resolved game-contract [`InterfaceEnv`](sigil_frontend_emp::contract::InterfaceEnv)
/// for `profile`, bound from AEON'S OWN CONTRACT — the derived counterpart to
/// [`game_contract_env`]'s hand-written stubs. See the section header above.
///
/// `defines` is the oracle's build shape (`DEBUG` / `SOUND_DEBUG_HOTKEYS` /
/// `SOUND_DRIVER_ENABLED` …), consulted by the manifest's comptime-`if` binding
/// groups exactly as the whole-program build consults it.
///
/// Loud on every degenerate outcome: a missing interface or manifest file, a parse
/// or bind error, a manifest whose module id is not the profile's declared
/// `manifest_module`, an env with no `Game` interface, and an env binding fewer
/// members than the interface declares.
pub fn game_contract_env_from_aeon(
    aeon: &std::path::Path,
    profile: &crate::native::GameProfile,
    defines: &[(String, i128)],
) -> sigil_frontend_emp::contract::InterfaceEnv {
    use sigil_frontend_emp::resolve::contract::{bind_with_ambient, ContractModule};

    let iface_path = aeon.join(GAME_CONTRACT_IFACE_REL);
    let impl_path = game_manifest_path(aeon, profile);
    let ef = parse_emp_or_panic(&iface_path);
    let gf = parse_emp_or_panic(&impl_path);

    let eid = ef.module.path.segments.join(".");
    let gid = gf.module.path.segments.join(".");
    assert_eq!(
        gid,
        profile.manifest_module,
        "manifest at {} declares module `{gid}`, but profile `{}` names \
         `{}` — the derivation found the wrong file",
        impl_path.display(),
        profile.name,
        profile.manifest_module
    );

    // The manifest's own imports, followed to their files: a binding value that
    // names an imported const (`const ENTRY_ID = GS_OJZ_SCROLL_TEST`) folds only
    // with those declarations in the bind ambient. Derived from the `use` edges,
    // so an import the manifest grows is followed by construction.
    let mut ambient: Vec<sigil_frontend_emp::ast::Item> = Vec::new();
    for item in &gf.items {
        let sigil_frontend_emp::ast::Item::Use(u) = item else { continue };
        // `base` is the module id in every `use` form (`use a.b.c`, `use a.b.c._`,
        // `use a.b.c.*`, `use a.b.c.{X}`) — the name list is not part of it.
        let rel: PathBuf =
            u.base.segments.iter().fold(PathBuf::new(), |p, s| p.join(s.as_str()));
        let candidate = aeon.join(&rel).with_extension("emp");
        if candidate.is_file() {
            ambient.extend(parse_emp_or_panic(&candidate).items);
        }
        // A `use` that resolves to no file is NOT an error here: the aeon module
        // id is not always the on-disk path (`games.sonic4.constants` lives at
        // `games/sonic4/config/constants.emp`). The bind below is the judge — an
        // unresolved binding value fails loud there, naming the name.
    }
    // Config modules do not sit at their dotted path. Sweep the game's own
    // `config/` directory, which is where a game's `.emp` declarations live, so
    // `games.sonic4.constants` is found without naming it.
    if let Some(cfg_dir) = impl_path.parent() {
        if let Ok(entries) = std::fs::read_dir(cfg_dir) {
            let mut paths: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "emp") && *p != impl_path)
                .collect();
            paths.sort();
            for p in paths {
                ambient.extend(parse_emp_or_panic(&p).items);
            }
        }
    }

    let mods = [ContractModule { id: &eid, file: &ef }, ContractModule { id: &gid, file: &gf }];
    let (env, diags) = bind_with_ambient(&mods, defines, &ambient);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "binding {} against {} failed: {diags:?}",
        impl_path.display(),
        iface_path.display()
    );

    assert!(
        env.interfaces.contains_key("Game"),
        "bind produced no `Game` interface from {}",
        iface_path.display()
    );
    let missing = game_contract_missing_members(aeon, &env);
    assert!(
        missing.is_empty(),
        "the derived env is missing {} of the {} members `interface Game` declares in {}: {missing:?} \
         — an env that under-covers the contract lets an engine module's `Game.MEMBER` \
         fail at lower instead of here",
        missing.len(),
        game_contract_declared_members(aeon).len(),
        iface_path.display()
    );
    env
}

/// The members `interface Game` DECLARES in aeon's contract that `env` does NOT
/// carry, by name — empty for a complete env.
///
/// The coverage predicate, factored out so the gate that proves it non-vacuous
/// runs the SAME code the env's own assertion runs. Both sides are derived: the
/// expectation from [`game_contract_declared_members`], the actual from the env.
pub fn game_contract_missing_members(
    aeon: &std::path::Path,
    env: &sigil_frontend_emp::contract::InterfaceEnv,
) -> Vec<String> {
    let declared = game_contract_declared_members(aeon);
    let Some(game) = env.interfaces.get("Game") else { return declared };
    declared.into_iter().filter(|m| !game.members.contains_key(m)).collect()
}

/// Every LINK SYMBOL the resolved `Game` interface binds — each bound hook's and
/// proc member's target name, sorted. An isolated oracle lowering an engine module
/// through the derived env emits a real `jsr <symbol>` at every bound `invoke`, so
/// it must supply an address for each; this is that list, READ OFF THE ENV rather
/// than hand-kept, so a newly bound hook arrives with it.
pub fn game_contract_bound_symbols(
    env: &sigil_frontend_emp::contract::InterfaceEnv,
) -> Vec<String> {
    use sigil_frontend_emp::contract::ResolvedMember;
    let Some(game) = env.interfaces.get("Game") else { return Vec::new() };
    let mut out: Vec<String> = game
        .members
        .values()
        .filter_map(|m| match m {
            ResolvedMember::Hook(Some(s)) | ResolvedMember::Proc(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// One symbol's address, read out of a sigil-canonical aeon LISTING (`s4.lst`) —
/// the `.bin`'s own sibling from the SAME build, so a symbol's address here and
/// the operand encoded in the reference ROM cannot disagree.
///
/// The listing's symbol table spells one entry per line as `NAME : HEX C |`.
/// `None` when the listing file is absent (a source-only tree); a PRESENT listing
/// that does not carry `name` is a hard error naming it — the caller wanted an
/// address, and a silent zero would encode a wrong operand into a byte gate.
pub fn listing_symbol_addr(listing: &std::path::Path, name: &str) -> Option<u32> {
    let text = std::fs::read_to_string(listing).ok()?;
    let needle = format!(" {name} : ");
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(&needle) else { continue };
        let hex = rest.split_whitespace().next().unwrap_or("");
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            return Some(v);
        }
    }
    panic!("listing {} carries no symbol `{name}`", listing.display());
}

// ── 2. The drift-guard filter ────────────────────────────────────────────────

/// `true` iff `a` is a twin DRIFT GUARD (not a D2.29 `[layout.odd-item]` parity
/// assert). Drift guards and parity asserts both ride `module.link_asserts`;
/// this is the predicate that tells them apart.
pub fn is_drift_guard(a: &LinkAssert) -> bool {
    // Exclude the D2.29 STRUCTURAL alignment asserts — both the `[layout.odd-item]`
    // odd-address parity asserts and the `[layout.align]` congruence asserts that
    // an `align` / `table item_align:` pad records. Neither is a user DRIFT guard
    // (an `ensure`/twin-mirror co-residency check); they are layout invariants.
    !a.message.iter().any(|p| {
        matches!(p, MsgPart::Text(t) if t.contains("[layout.odd-item]") || t.contains("[layout.align]"))
    })
}

/// The drift guards among `asserts`, excluding the `[layout.odd-item]` parity
/// asserts.
pub fn drift_guards_only(asserts: &[LinkAssert]) -> impl Iterator<Item = &LinkAssert> {
    asserts.iter().filter(|a| is_drift_guard(a))
}

/// Count the twin drift guards in `asserts` (excludes `[layout.odd-item]` parity
/// asserts). The established `guard_assert_count` idiom, now shared.
pub fn guard_assert_count(asserts: &[LinkAssert]) -> usize {
    drift_guards_only(asserts).count()
}

// ── 3. The REFERENCE-DEPENDENT guard ────────────────────────────────────────

// THE SUITE-PATHS RESOLVER. `empyrean` `contract/SUITE_PATHS.md` (ratified 2026-09-02,
// read here at `origin/main` = 82982b7ff3c057f347d538fcf61b7c62b18ee813) fixes ONE
// precedence for every resolver in the suite, and this is sigil's implementation of it.
//
// The contract resolves a CHECKOUT. Sigil's byte and port gates need a REFERENCE TREE.
// Those are two questions and this file keeps them two functions:
//
//   * [`aeon_checkout`] answers the contract's question in full — steps 1→4, for any
//     caller that legitimately wants the live sibling checkout;
//   * [`aeon_dir`] answers sigil's — and only steps 1 and 2 are acceptable answers to
//     it, because step 3 derives the owner's LIVE working checkout, whose revision moves
//     under a run. A measurement against a tree nobody named is attributable to whatever
//     that tree happened to contain.
//
// THE `env::var("AEON_DIR")` SPELLING BELOW IS LOAD-BEARING AND MUST STAY IN THIS FILE,
// INSIDE A PUBLIC FUNCTION. `scripts/nightly_source_gates.sh` derives both halves of its
// classifier from this source: `reference_env_var` extracts the variable name by matching
// that literal, and `accessor_closure` seeds on the PUBLIC function containing it and then
// closes over every public function of this file that calls one already in the set. A
// resolver moved to another module, a variable name reached through a constant, or a step
// 1 buried in a private helper leaves that derivation with no seed — and the lane then
// either refuses to run or, worse, classifies every routed test file as reading nothing.

/// The environment variable that names an aeon CHECKOUT — precedence step 1.
///
/// The contract makes `<TOOL>_DIR` the suite-wide checkout spelling and ratifies this
/// one because it was already the de-facto contract (100 sigil files, 60 aurora files,
/// sigil's CI and its landing wrapper).
pub const AEON_DIR_VAR: &str = "AEON_DIR";

/// The environment variable that names the SUITE ROOT — precedence step 2.
///
/// Suite-level and therefore deliberately un-branded: a suite fact carrying one tool's
/// name is how the same sentence ends up in five repos drifting independently.
pub const SUITE_ROOT_VAR: &str = "EMPYREAN_SUITE_ROOT";

/// aeon's directory name under the suite root.
pub const AEON_REPO_DIR: &str = "aeon";

/// Every sibling a directory must hold to BE the suite root.
///
/// The same marker set aeon's own `tools/suite_paths.py` uses, and for the reason it
/// gives: `empyrean` is the suite contract repo and `aeon` the engine, so a directory
/// holding both is the suite root by definition. Two resolvers answering the same
/// question must not answer it differently.
pub const SUITE_ROOT_MARKERS: [&str; 2] = [AEON_REPO_DIR, "empyrean"];

/// Which precedence step of `contract/SUITE_PATHS.md` produced an answer.
///
/// An enum and not a string: the one consumer that must branch on it — whether the
/// answer is a tree SOMEBODY NAMED — is a decision, and a decision taken by matching
/// prose is a decision that changes when the prose is reworded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStep {
    /// Step 1: the explicit checkout variable [`AEON_DIR_VAR`].
    CheckoutVar,
    /// Step 2: [`SUITE_ROOT_VAR`] joined with [`AEON_REPO_DIR`].
    SuiteRootVar,
    /// Step 3: derived from THIS repo's own location.
    Derived,
}

impl PathStep {
    /// The step's number in the contract's precedence list.
    pub fn number(self) -> u8 {
        match self {
            PathStep::CheckoutVar => 1,
            PathStep::SuiteRootVar => 2,
            PathStep::Derived => 3,
        }
    }

    /// How the step answered, in one clause, for the line a resolver owes its reader.
    pub fn describe(self) -> &'static str {
        match self {
            PathStep::CheckoutVar => "named by AEON_DIR",
            PathStep::SuiteRootVar => "named by EMPYREAN_SUITE_ROOT",
            PathStep::Derived => "DERIVED from this checkout's own location — nobody named it",
        }
    }

    /// `true` when the answer is a tree somebody NAMED, and therefore a tree a
    /// reference-dependent measurement may be attributed to.
    ///
    /// Step 3 is excluded on purpose. It derives `<suite root>/aeon`, the owner's live
    /// working checkout: its revision changes under a run without notice, so a pass or a
    /// failure measured against it is attributable to whatever it happened to contain
    /// rather than to the code under test.
    pub fn names_a_reference_tree(self) -> bool {
        !matches!(self, PathStep::Derived)
    }
}

/// A resolved aeon checkout and the precedence step that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCheckout {
    /// The directory.
    pub path: PathBuf,
    /// Which step answered.
    pub step: PathStep,
}

impl ResolvedCheckout {
    /// The line a resolver owes its reader before doing work: the resolved path and the
    /// step that produced it.
    ///
    /// It carries neither `skip:` nor `skipping`. `scripts/landing-run.sh` and
    /// `refreeze.rs` both count those spellings out of a run's log, and a resolution
    /// notice is not a skipped test; a notice that inflated the skip count would trade
    /// one wrong number for another.
    pub fn announcement(&self) -> String {
        format!(
            "reference-tree: {} (SUITE_PATHS step {} — {})",
            self.path.display(),
            self.step.number(),
            self.step.describe()
        )
    }
}

/// `true` when every marker in [`SUITE_ROOT_MARKERS`] is a directory under `p`.
fn is_suite_root(p: &std::path::Path) -> bool {
    SUITE_ROOT_MARKERS.iter().all(|m| p.join(m).is_dir())
}

/// The suite root derived from THIS repo's own location — the resolver's step 3.
///
/// `git rev-parse --path-format=absolute --git-common-dir`, NEVER `--show-toplevel`. From a
/// git worktree — and every sigil agent runs in one, under `.claude/worktrees/<name>/` —
/// `--show-toplevel` answers the worktree, whose parent chain does not reach the suite root,
/// so a resolver built on it derives a wrong answer confidently. `--git-common-dir` answers
/// the MAIN checkout's `.git` from a worktree and from the checkout alike, and its parent is
/// this repo's root either way — but only once `--path-format=absolute` has settled WHICH OF
/// THREE SHAPES it answers in. See the comment on that flag in the body; the shapes are
/// enumerated by a test rather than by this sentence, because this sentence had two of them.
///
/// Run from `CARGO_MANIFEST_DIR`, fixed at compile time, so the derivation is about the
/// checkout this code was BUILT from rather than whatever directory a test process
/// happens to have as its cwd.
///
/// Cached: the answer cannot change within a process (the manifest dir is a compile-time
/// constant) and ~100 test binaries would otherwise each pay a subprocess per call.
fn derived_suite_root() -> Result<PathBuf, String> {
    static ROOT: std::sync::OnceLock<Result<PathBuf, String>> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| derive_suite_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))))
        .clone()
}

/// The step-3 mechanism itself, over an arbitrary directory inside a checkout.
///
/// Separated from [`derived_suite_root`] for one reason, and it is the contract's, not a
/// convenience: *"the step-3 proof runs from a linked worktree, or says in the run's own
/// output that it did not"* (`SUITE_PATHS.md`, added 2026-09-02 from aurora's O68). The
/// property step 3 exists for is only observable from a LINKED WORKTREE — in a plain
/// checkout `--git-common-dir` and `--show-toplevel` agree, so an assertion made there
/// proves nothing.
///
/// A test asserting against the process's own location can only prove the property when the
/// suite itself happens to be running from a worktree, and a row that proves its property
/// only from certain checkouts is a row that reads green from the others. So the mechanism
/// takes a directory, and `suite_paths_precedence` hands it one inside a linked worktree it
/// builds itself — the same assertion wherever `cargo test` is invoked from.
pub fn derive_suite_root_from(here: &std::path::Path) -> Result<PathBuf, String> {
    // `--path-format=absolute` IS LOAD-BEARING, and its absence was a live bug.
    //
    // `git rev-parse --git-common-dir` has THREE output shapes, not two:
    //
    //     anchored at a plain checkout's ROOT      -> `.git`            relative
    //     anchored at a plain checkout's SUBDIR    -> `../../.git`      relative, with `..`
    //     anchored anywhere in a linked WORKTREE   -> `/abs/path/.git`  absolute
    //
    // The middle one is the one production uses: `CARGO_MANIFEST_DIR` is always a crate
    // subdirectory. `Path::parent()` trims components LEXICALLY and does not canonicalise,
    // so joining `../../.git` and taking two parents yields `<crate>/..` — a path whose
    // `aeon/` sits one directory away from the tree being looked for. Step 3 then refused,
    // in the main checkout only, which is where the landing run and the nightly lane
    // invoke this suite.
    //
    // Asking git for the shape we want is the fix; canonicalising the joined path or
    // special-casing `..` would be this code guessing at what git meant. The flag is
    // already this repo's idiom — `crates/sigil-cli/build.rs` resolves its git dirs the
    // same way — and it has been in git since 2.31 (2021).
    //
    // `tests/suite_paths_precedence.rs::step_3_survives_every_shape_git_rev_parse_can_answer`
    // enumerates all four anchors a caller can hand this function and requires one answer
    // from all of them, so a shape nobody thought of is a failing row rather than a
    // sentence missing from this comment — which is how the third shape was lost.
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(here)
        .output()
        .map_err(|e| {
            format!("`git rev-parse --git-common-dir` in {} could not run: {e}", here.display())
        })?;
    if !out.status.success() {
        return Err(format!(
            "`git rev-parse --path-format=absolute --git-common-dir` in {} exited {}: {}",
            here.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let answered = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let common = std::path::Path::new(&answered);
    // REFUSE a relative answer rather than joining it. Joining is what silently produced a
    // lexically-wrong path last time; a git that stopped honouring the flag should stop
    // this resolver loudly instead of moving the walk one directory sideways.
    if !common.is_absolute() {
        return Err(format!(
            "`git rev-parse --path-format=absolute --git-common-dir` in {} answered `{answered}`,              which is not an absolute path. This resolver will not join a relative answer onto              its anchor: a relative answer carrying `..` is trimmed lexically by Path::parent()              rather than canonicalised, which lands the walk beside the tree it is looking for              instead of refusing.",
            here.display()
        ));
    }
    let repo_root = common
        .parent()
        .ok_or_else(|| format!("{} has no parent, so it names no repository root", common.display()))?;
    let root = repo_root.parent().ok_or_else(|| {
        format!("{} has no parent, so it hangs off no suite root", repo_root.display())
    })?;
    if !is_suite_root(root) {
        return Err(format!(
            "{} is this repository's parent but holds no {} — it is not a suite root",
            root.display(),
            SUITE_ROOT_MARKERS.map(|m| format!("{m}/")).join(" + ")
        ));
    }
    Ok(root.to_path_buf())
}

/// The contract's precedence from STEP 2 onward, given what step 1 already reported.
///
/// Step 1 lives in its two public callers rather than here, and that is deliberate on
/// both counts. [`aeon_checkout`] consults the checkout variable; [`unnamed_default_tree`]
/// deliberately does not, because its whole question is what a run resolves to when
/// nobody names a tree. And `scripts/nightly_source_gates.sh` derives its accessor set by
/// closure from the PUBLIC function of this file that reads `AEON_DIR`: a step 1 buried in
/// a private helper leaves that derivation with no seed, and the lane then classifies
/// every routed test file as reading nothing — quietly, which is the one direction in
/// which being wrong is invisible.
fn resolve_from_step_2(mut tried: Vec<String>) -> Result<ResolvedCheckout, String> {
    let markers = SUITE_ROOT_MARKERS.map(|m| format!("{m}/")).join(" + ");

    // ── Step 2: the suite root variable.
    match std::env::var(SUITE_ROOT_VAR) {
        Ok(v) if !v.is_empty() => {
            let root = PathBuf::from(&v);
            if !is_suite_root(&root) {
                return Err(format!(
                    "{SUITE_ROOT_VAR}={v} is not a suite root: a suite root holds {markers}. Set \
                     but wrong is a hard error at its own step, not a null that lets a derivation \
                     answer in its place."
                ));
            }
            let path = root.join(AEON_REPO_DIR);
            if !path.is_dir() {
                return Err(format!(
                    "{SUITE_ROOT_VAR}={v} holds {markers} but {} is not a directory",
                    path.display()
                ));
            }
            return Ok(ResolvedCheckout { path, step: PathStep::SuiteRootVar });
        }
        Ok(_) => tried.push(format!("{SUITE_ROOT_VAR} is set to the empty string")),
        Err(_) => tried.push(format!("{SUITE_ROOT_VAR} is unset")),
    }

    // ── Step 3: derivation from this repo's own location.
    match derived_suite_root() {
        Ok(root) => {
            let path = root.join(AEON_REPO_DIR);
            if path.is_dir() {
                return Ok(ResolvedCheckout { path, step: PathStep::Derived });
            }
            tried.push(format!(
                "derived the suite root {} from this checkout's own location, and {} is not a \
                 directory",
                root.display(),
                path.display()
            ));
        }
        // A derivation is not a variable, so a derivation that does not answer is not a
        // "set but wrong" hard error — it falls to step 4, which names why.
        Err(why) => tried.push(format!("derivation from this checkout's own location: {why}")),
    }

    // ── Step 4: refuse, naming what was looked for and where.
    Err(format!(
        "no aeon checkout could be resolved. {}. Set {AEON_DIR_VAR} to an aeon checkout, or \
         {SUITE_ROOT_VAR} to the directory holding {markers}.",
        tried.join("; ")
    ))
}

/// The aeon CHECKOUT, by the contract's full precedence, and the step that answered.
///
/// This is the answer for a caller that legitimately wants the live sibling checkout —
/// something reading aeon SOURCE at whatever revision the tree currently holds. A caller
/// that needs a tree a RESULT can be attributed to wants [`aeon_dir`] instead, which
/// refuses step 3's answer.
pub fn aeon_checkout() -> Result<ResolvedCheckout, String> {
    // ── Step 1: the explicit checkout variable.
    match std::env::var("AEON_DIR") {
        Ok(v) if !v.is_empty() => {
            let path = PathBuf::from(&v);
            if !path.is_dir() {
                // SET BUT WRONG IS A HARD ERROR AT THIS STEP, never a null that lets step
                // 2 run. A wrong value is evidence of a wrong environment, and a
                // fall-through would answer with a different tree than the one the
                // operator asked for while reporting success.
                //
                // "Wrong" is exactly "not a directory", and no wider. A tree's CONTENTS
                // are [`reference_tree`]'s question, asked per gate against the paths that
                // gate actually reads; answering it here would replace those precise
                // messages with a blunt one and would refuse the empty stand-in trees the
                // write-guard gates deliberately point this variable at.
                return Err(format!(
                    "{AEON_DIR_VAR}={v} does not name a directory. A checkout variable that \
                     is set but wrong is a hard error at its own step (SUITE_PATHS, \
                     'Precedence, the same in every resolver'), not a null that lets \
                     {SUITE_ROOT_VAR} or a derivation answer in its place — falling through \
                     would measure against a tree nobody asked for and call it a pass."
                ));
            }
            Ok(ResolvedCheckout { path, step: PathStep::CheckoutVar })
        }
        Ok(_) => resolve_from_step_2(vec![format!("{AEON_DIR_VAR} is set to the empty string")]),
        Err(_) => resolve_from_step_2(vec![format!("{AEON_DIR_VAR} is unset")]),
    }
}

/// The checkout a run resolves to when NOBODY names one: the contract's precedence with
/// step 1 skipped.
///
/// This is what a guard that must refuse to touch the live tree compares against —
/// resolved, never spelled, so it keeps working when the suite moves (SUITE_PATHS, "What
/// a resolver owes its reader"). [`crate::seam2::require_named_reference_tree`] is that
/// guard.
pub fn unnamed_default_tree() -> Result<ResolvedCheckout, String> {
    resolve_from_step_2(vec![format!(
        "{AEON_DIR_VAR} deliberately not consulted — this is the tree a run resolves to when \
         nobody names one"
    )])
}

/// `true` when a read would resolve a tree nobody named rather than one somebody did.
pub fn aeon_dir_is_unnamed() -> bool {
    !matches!(aeon_checkout(), Ok(c) if c.step.names_a_reference_tree())
}

/// The aeon REFERENCE tree — the tree every byte and port gate measures against.
///
/// Steps 1 and 2 of the contract's precedence are the acceptable answers. A step-3
/// derivation is NOT a reference tree: it resolves the owner's live working checkout,
/// whose revision moves under a run.
///
/// **A READ AGAINST A TREE NOBODY NAMED ANNOUNCES ITSELF.** `d-17` closed the WRITE side:
/// a write refuses and names the tree it refused. The read side stayed silent, and that
/// asymmetry was measured on 2026-08-30 — a control run bare resolved its oracle to the
/// owner's live checkout and was correct only because his working tree happened to sit at
/// the revision under test. Had he checked out anything else, the same command would have
/// produced a false red or a false green with identical output. The consequence worth
/// naming: **the owner's working directory is load-bearing for a verification he does not
/// know he is participating in**, and nothing linked the two ends.
///
/// Announced ONCE per process (261 call sites; per-call would be noise), on stderr, beside
/// the `skip:` lines a reader already scans.
///
/// **AND A BARE RUN NOW STOPS.** `d-18`, ruled `refuse` by the hub on 2026-09-02 under the
/// owner's widened delegation (`docs/OVERSEER.md`, R4; empyrean `4e8e865b`), against this
/// lane's own recommendation of say-only. The hub's reason is the better one: *a run that
/// prints how much it skipped still exits 0*, and a silent green is the class never
/// dropped, because a green is trusted the moment it is in the run. The parcel-1
/// announcement above is what that costs measured in one transcript — two passes, exit 0,
/// and the subject of the measurement was whatever the owner's working tree happened to
/// contain.
///
/// So when steps 1 and 2 do not answer, this panics with a message naming both variables,
/// the derived path it declined to use and why, and the opt-in. [`ALLOW_PARTIAL_VAR`]
/// takes the partial run instead: reference-dependent rows skip against
/// [`NO_REFERENCE_TREE`], and the banner says how many binaries that is.
pub fn aeon_dir() -> PathBuf {
    match aeon_checkout() {
        Ok(r) if r.step.names_a_reference_tree() => {
            announce_once(r.announcement());
            r.path
        }
        // Step 3 answered, or nothing did. Both are the same fact for a gate: nobody named
        // a tree this result could be attributed to.
        Ok(r) => no_named_reference_tree(&r.announcement(), Some(&r)),
        Err(refusal) => no_named_reference_tree(&refusal, None),
    }
}

/// One line per process on stderr, beside the `skip:` lines a reader already scans.
fn announce_once(line: String) {
    static ANNOUNCED: std::sync::Once = std::sync::Once::new();
    ANNOUNCED.call_once(|| eprintln!("{line}"));
}

/// The environment variable that opts a run in to running WITHOUT a named reference tree.
///
/// The ruling's shape: the refusal is the default and the partial run is explicit, because
/// the person who set this variable knows what the run does not cover and the person
/// reading its green does not.
pub const ALLOW_PARTIAL_VAR: &str = "SIGIL_ALLOW_PARTIAL";

/// The stand-in path a PARTIAL run resolves to.
///
/// Deliberately absent and deliberately self-describing. Every reference-dependent gate
/// opens with [`reference_tree`], which reports a missing path by name, so this spelling is
/// what a reader sees in each of those `skip:` lines — the reason for the skip is carried
/// by the path itself rather than inferred from a banner scrolled past hundreds of lines
/// earlier. Returning the DERIVED live checkout here instead would make the partial run
/// silently measure against it, which is the thing being refused.
pub const NO_REFERENCE_TREE: &str = "/nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named";

/// The message a bare run stops with.
///
/// Pure so a test can pin it without arranging the environment that produces it. Four
/// things have to be in it and each is asserted: both variables that would have answered,
/// the path step 3 derived and DECLINED (so the reader is not left wondering whether the
/// resolver simply failed), and the opt-in spelling. It carries neither `skip:` nor
/// `skipping` — `scripts/landing-run.sh:369` and `refreeze.rs:533` count those out of a
/// run's log, and this refusal is a FAILURE, not a skipped test; counting it as a skip
/// would let the very run that stopped report a skip total instead of a stop.
///
/// `context` is what the resolver itself said: its step-3 answer, or its step-4 refusal.
pub fn bare_run_refusal(context: &str, derived: Option<&ResolvedCheckout>) -> String {
    let declined = match derived {
        Some(r) => format!(
            "This run DECLINED to use {}, which step 3 derived from this checkout's own \
             location: it is a working checkout outside this repository, its revision changes \
             under a run without notice, and a result measured against it would be attributable \
             to whatever it happened to contain rather than to the code under test.",
            r.path.display()
        ),
        None => "Nothing was derived either.".to_string(),
    };
    format!(
        "NO REFERENCE TREE IS NAMED, so this run can measure nothing it could attribute, and \
         STOPS. {declined}\n\nThe resolver's own answer: {context}\n\nEither name a provisioned \
         tree — {AEON_DIR_VAR}=<aeon checkout> (scripts/provision-aeon-ref.sh), or \
         {SUITE_ROOT_VAR}=<the directory holding the suite> — or declare a partial run with \
         {ALLOW_PARTIAL_VAR}=1, in which case every reference-dependent row is left unmeasured \
         and the run says how many. Ruled d-18 (docs/OVERSEER.md, 2026-09-02): a run that only \
         PRINTS how much it did not measure still exits 0, and a green is trusted the moment it \
         is in the run."
    )
}

/// The `d-18` decision point: nobody named a reference tree, so either stop or take the
/// declared partial run.
///
/// `context` is what the resolver said — its step-3 answer, or its step-4 refusal.
fn no_named_reference_tree(context: &str, derived: Option<&ResolvedCheckout>) -> PathBuf {
    let partial = std::env::var_os(ALLOW_PARTIAL_VAR).is_some_and(|v| !v.is_empty());
    // `SIGIL_STRICT_GATE` is read directly rather than through `strict_gate()`. That
    // accessor RECORDS every reached consultation into the strict witness, and
    // `strict_census` diffs that population against the one it derives from the test tree;
    // a consultation from inside the resolver is not a strict-gated test body and would
    // enter the census as a site with no counterpart.
    let strict = std::env::var_os("SIGIL_STRICT_GATE").is_some();

    if partial && strict {
        panic!(
            "{ALLOW_PARTIAL_VAR} and SIGIL_STRICT_GATE are both set and no reference tree is \
             named. A strict run is the one that may not skip a gate, so it cannot also be the \
             partial one; the two flags describe opposite runs and the resolver will not pick \
             between them. Name a tree with {AEON_DIR_VAR}, or drop one flag.\n{context}"
        );
    }

    if !partial {
        panic!("{}", bare_run_refusal(context, derived));
    }

    announce_once(partial_run_banner(context));
    PathBuf::from(NO_REFERENCE_TREE)
}

/// The banner a declared partial run prints once, carrying the DERIVED size of what it is
/// not measuring.
///
/// The count comes from [`crate::reference_dependence`], the same walk
/// `reference_dependence_is_named` reports with — one derivation, two consumers, and no
/// number typed anywhere. A derivation that came back below its own floor would render an
/// unmeasured suite as a small one, so it says so instead of printing a number it cannot
/// stand behind.
pub fn partial_run_banner(context: &str) -> String {
    let ws = crate::reference_dependence::workspace_root();
    let gated = crate::reference_dependence::reference_dependent_binaries(&ws);
    let size = if gated.len() > crate::reference_dependence::FLOOR {
        format!("{} test binaries are reference-dependent and", gated.len())
    } else {
        format!(
            "the derivation of how many test binaries are reference-dependent returned only {} \
             and COULD NOT BE ESTABLISHED (floor {}), so the size below is unknown rather than \
             small —",
            gated.len(),
            crate::reference_dependence::FLOOR
        )
    };
    format!(
        "PARTIAL RUN ({ALLOW_PARTIAL_VAR} is set). No reference tree is named, so {size} every \
         row in them is left UNMEASURED. A green result from this run does NOT mean those rows \
         passed — it means they were not run. Name a tree with {AEON_DIR_VAR} to measure them.\
         \n{context}"
    )
}

/// `true` when `SIGIL_STRICT_GATE` is set — the pre-merge fidelity run, where a
/// missing reference is a FAILURE rather than a skip.
#[track_caller]
pub fn strict_gate() -> bool {
    let on = std::env::var("SIGIL_STRICT_GATE").is_ok();
    if on {
        witness_strict_body(std::panic::Location::caller());
    }
    on
}

/// The environment variable naming the strict-run WITNESS file. When it is set, every
/// strict-gated decision point that consults [`strict_gate`] AND finds the flag set
/// appends its own `file:line` here.
pub const STRICT_WITNESS_VAR: &str = "SIGIL_STRICT_WITNESS";

/// Record that a strict-gated body was reached with the flag ON.
///
/// # Why this exists at all
///
/// A suite run WITHOUT `SIGIL_STRICT_GATE=1` early-returns every one of these bodies
/// and is nevertheless fully green — which is precisely how two refreezes landed with
/// no strict run behind them and a stale constant rode through both. No aggregate the
/// run produces can tell the two apart: pass counts, exit codes and `ignored` totals
/// all read identically. The count of lines written here is the one quantity that
/// cannot: it is STRUCTURALLY ZERO when the flag is unset, because the only call that
/// writes is one that already observed the flag set.
///
/// `refreeze --attest` reads the distinct site count into the provenance chain as
/// `strict_bodies`, and — because a count can only be READ while a population can be
/// DIFFED — compares the whole population against the one
/// [`crate::strict_census`] derives from the test tree. Zero alone is a FLOOR, and a
/// floor is satisfiable by the very failure the witness exists to catch: a gate going
/// dark reads back as a smaller green.
///
/// # The line format
///
/// `file:line<TAB>test-name`, one per reached consultation. Both halves are load-bearing
/// and they answer different questions:
///
///   * `file:line` — WHICH consultation, from `#[track_caller]`, which propagates
///     through each test file's per-file `strict_gate()` wrapper to the test's own call
///     site. This is what dedupes into `strict_bodies`.
///   * the test name — WHICH TEST reached it. libtest names each test's thread after the
///     test (measured, including under `--test-threads=1`), so this costs nothing and is
///     the only thing that can see a gate whose guard is deleted while the test
///     survives: that edit removes the census's `file:line` expectation in the same
///     stroke, and only the test's absence from the population remains as evidence.
///     A guard reached off a libtest thread has no name and records
///     [`crate::strict_census::UNNAMED_THREAD`], which the comparison reports rather
///     than accepts.
///
/// Deliberately a FILE and not a printed marker: libtest captures the stdout and stderr
/// of PASSING tests, so a marker printed by a passing gate is invisible without
/// `--nocapture` — the same class of silent inertness this whole mechanism is about.
/// A file write is outside that capture.
///
/// `#[track_caller]` on [`strict_gate`] is what makes the recorded location the TEST'S
/// call site rather than this module's, so the witness names distinct strict-gated
/// bodies rather than counting one shared function.
///
/// Every failure here is swallowed on purpose: this is instrumentation, and a test
/// suite must not go red because a witness path was unwritable. The zero-count refusal
/// in `--attest` is what keeps a silently-unwritten witness from reading as a pass.
fn witness_strict_body(loc: &std::panic::Location<'_>) {
    let Ok(path) = std::env::var(STRICT_WITNESS_VAR) else { return };
    use std::io::Write;
    // ONE `write_all` of a pre-built line, never `writeln!`. `write_fmt` issues a
    // separate syscall per format fragment, and with O_APPEND from parallel test threads
    // and processes those fragments interleave: measured output included
    // `…dac_head_colink.rs…dac_head_colink.rs::91133`, two sites spliced into one
    // unparseable line. A single short `write_all` to an O_APPEND descriptor does not
    // tear, which is what makes the count trustworthy.
    let current = std::thread::current();
    let test = current.name().unwrap_or(crate::strict_census::UNNAMED_THREAD);
    let line = format!("{}:{}\t{}\n", loc.file(), loc.line(), test);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// THE ONE SPELLING a gate announces an early return with, and the single
/// definition both enforcers of the zero-skip landing bar read.
///
/// A skip line is a gate that measured nothing while reporting green, so the
/// landing bar fails on any occurrence of this prefix in a strict suite log.
/// A bar phrased as a literal string is only as wide as the spellings whoever
/// wrote it happened to guess: `skipping …` announced 29 early returns for
/// months without matching `skip: `, and every one of them read back as
/// coverage. So the marker lives HERE, once:
///
///   * `tests/skip_marker_lint.rs` holds every announced early return in the
///     test tree to this prefix, so a new site cannot be written in a spelling
///     the bar is blind to;
///   * `scripts/nightly_source_gates.sh` EXTRACTS this constant out of this
///     file rather than retyping it, and refuses to run if it cannot.
///
/// Changing the string here moves both consumers together. Retyping it anywhere
/// else re-opens the hole.
pub const SKIP_MARKER: &str = "skip: ";

/// The guard every REFERENCE-DEPENDENT test opens with: `Some(aeon)` when the
/// tree carries every path in `rels`, `None` — skip green — when it does not.
///
/// The skip exists for environments that check out THIS repo alone (CI): the
/// aeon tree is a sibling working copy, not a dependency sigil can vendor. It
/// must never quietly swallow a real gate, so `SIGIL_STRICT_GATE=1` turns a
/// missing reference into a panic naming the absent path — that is the flag the
/// pre-merge run sets, and under it these tests cannot skip.
///
/// `rels` are the aeon-relative paths the caller actually reads (`.emp` sources,
/// fixtures, ROMs). Naming them — rather than probing the tree root — is what
/// makes the guard honest against an `AEON_DIR` pointed at an incomplete tree.
pub fn reference_tree(rels: &[&str]) -> Option<PathBuf> {
    let aeon = aeon_dir();
    if let Some(missing) = rels.iter().find(|rel| !aeon.join(rel).exists()) {
        let path = aeon.join(missing);
        assert!(
            !strict_gate(),
            "SIGIL_STRICT_GATE set but reference missing: {}",
            path.display()
        );
        eprintln!("skip: reference not at {} (set AEON_DIR)", path.display());
        return None;
    }
    Some(aeon)
}

/// The guard a PROFILE-DRIVEN gate opens with: `Some(aeon)` when the tree carries the
/// two files `profile`'s build reads before anything else — its AS residual root
/// ([`GameProfile::game_root_rel`], `games/<g>/game_root.asm`) and its placement map
/// ([`GameProfile::map_path`]'s `map.toml`, that root's sibling). Both paths are
/// DERIVED FROM THE PROFILE the caller goes on to build, so the guard cannot name
/// inputs the gate does not use.
///
/// A built ROM is not one of them. These gates assemble aeon SOURCE and compare
/// against sigil's own committed goldens, so a source-only checkout — every `.emp`
/// present, nothing built — is a tree they run fully against; sentinelling on
/// `s4.bin` reports such a tree missing.
///
/// Skip/panic semantics are [`reference_tree`]'s: skip green when a path is absent,
/// panic naming it under `SIGIL_STRICT_GATE=1`.
pub fn reference_tree_for_profile(profile: &crate::native::GameProfile) -> Option<PathBuf> {
    let root = std::path::Path::new(profile.game_root_rel);
    let map = root.parent().unwrap_or(std::path::Path::new("")).join("map.toml");
    let map = map.to_str().expect("game_root_rel is UTF-8");
    reference_tree(&[profile.game_root_rel, map])
}

// ── 4. The scanline-capability contract seam (Scanline P2 Phase 1) ──────────
//
// Phase 1 gated whole blocks of `engine/effects/raster.emp`,
// `engine/level/parallax.emp` and `engine/system/buffers.emp` behind
// `if (Game.SCANLINE_CAPS & CAP_<BIT>) != 0 { … }`. Those modules' standalone
// port oracles concatenate a hand-picked dep list and lower ONE module, so they
// see neither the whole-program contract bind (which resolves `Game.MEMBER`)
// nor `engine/level/scene_dsl.emp` (which declares the `CAP_*` bits) — and the
// lower aborted with `unknown name Game.SCANLINE_CAPS` / `unknown name
// CAP_ANCHORS`.
//
// Both halves of the fix are DERIVED FROM THE AEON TREE AT TEST RUNTIME, never
// copied:
//
//   * the contract env is bound from aeon's own interface against sonic4's own
//     `implement Game` ([`scanline_caps_contract_env`], §1c) — the port oracles
//     compare against the SONIC4 reference ROM windows, so the binding must be
//     sonic4's actual declaration or the gate would be measuring a specialisation
//     the reference never took;
//   * the bit values are read from `engine/level/scene_dsl.emp`, whose comment
//     block names those five `pub const CAP_*` lines THE AUTHORITY (two aeon
//     tools already parse them the same way).
//
// A copied `$001F` in Rust is exactly the "copied expectation" defect this tree
// keeps re-finding, which is why [`emp_const_literal`] reads the file instead.

/// Read `[pub] const <name> = <rhs>` out of an `.emp` source and return the
/// right-hand side TEXT, verbatim, minus any trailing `// …` comment.
///
/// The scan half of [`emp_const_literal`], split out so a caller that wants the
/// EXPRESSION rather than a folded number can have it (the live case:
/// [`bg_layout_size_const_src`] re-declares `BG_LAYOUT_SIZE = 64*64*2` into a
/// synthesized module and lets sigil's own comptime folder do the arithmetic,
/// so no product of that expression is ever written down in Rust).
///
/// Fails loud two ways rather than returning a default — absent (renamed or
/// moved) and ambiguous (more than one declaration).
pub fn emp_const_rhs(path: &std::path::Path, name: &str) -> String {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("emp_const_rhs: cannot read {}: {e}", path.display()));
    let mut hits: Vec<String> = Vec::new();
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        let t = t.strip_prefix("pub ").unwrap_or(t);
        let Some(rest) = t.strip_prefix("const ") else { continue };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix(name) else { continue };
        // Guard against `SCANLINE_CAPS_EXTRA` matching a request for `SCANLINE_CAPS`.
        let rest = rest.trim_start();
        let Some(rhs) = rest.strip_prefix('=') else { continue };
        let rhs = rhs.split("//").next().unwrap_or("").trim();
        hits.push(rhs.to_string());
    }
    match hits.len() {
        1 => hits.pop().unwrap(),
        0 => panic!(
            "emp_const_rhs: no `const {name} = …` in {} — the const was renamed or moved, \
             and this gate's expectation is derived from it",
            path.display()
        ),
        n => panic!(
            "emp_const_rhs: {n} declarations of `const {name}` in {} — ambiguous, refuse to guess",
            path.display()
        ),
    }
}

/// Read `[pub] const <name> = <integer literal>` out of an `.emp` source and
/// return its value. Accepts AS-style `$hex`, `0x` hex, `%binary` and decimal,
/// with optional `_` separators; ignores a trailing `// …` comment and skips
/// commented-out lines entirely.
///
/// FAILS LOUD in three ways rather than returning a default, because every
/// caller is a gate whose expectation is this number:
/// - no such const  → the name moved or was renamed;
/// - more than one  → ambiguous, the caller cannot know which one it got;
/// - a NON-LITERAL right-hand side → the const became COMPUTED (the live case:
///   `games/sonic4/config/game.emp` records that `SCANLINE_CAPS` MAY flip from
///   the `$001F` literal to `= SceneRegistry_CapsFolded` once the registry
///   exists). A parse-the-literal helper cannot follow that flip, so it says so
///   by name instead of silently binding a wrong mask.
pub fn emp_const_literal(path: &std::path::Path, name: &str) -> i128 {
    let rhs = emp_const_rhs(path, name);
    parse_emp_int_literal(&rhs).unwrap_or_else(|| {
        panic!(
            "emp_const_literal: `const {name}` in {} has a NON-LITERAL right-hand side `{rhs}`. \
             This helper parses a literal and CANNOT follow a computed const — the live case is \
             SCANLINE_CAPS flipping from the `$001F` literal to `= SceneRegistry_CapsFolded`. \
             When that flip lands, this helper must be replaced by a real evaluation of the \
             const (lower the declaring module and read the folded value), not given a default.",
            path.display()
        )
    })
}

/// `$1F` / `0x1F` / `%0001` / `31` -> value; `None` for anything that is not a
/// bare integer literal (an expression, a name, a call).
fn parse_emp_int_literal(rhs: &str) -> Option<i128> {
    let s: String = rhs.chars().filter(|c| *c != '_').collect();
    let (radix, digits) = if let Some(d) = s.strip_prefix('$') {
        (16, d)
    } else if let Some(d) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, d)
    } else if let Some(d) = s.strip_prefix('%') {
        (2, d)
    } else {
        (10, s.as_str())
    };
    if digits.is_empty() {
        return None;
    }
    i128::from_str_radix(digits, radix).ok()
}

/// The `pub const CAP_*` bits `engine/level/scene_dsl.emp` DECLARES, in file
/// order. The reserved bits underneath them are a comment on purpose (they have
/// no lowering), and [`emp_const_literal`]'s comment-skipping is what keeps them
/// out of this list — same rule aeon's own `tools/test_scene_span_labels.py` and
/// `tools/effects_gates.py` follow.
pub fn scene_dsl_cap_bits(aeon: &std::path::Path) -> Vec<(String, i128)> {
    let path = aeon.join("engine/level/scene_dsl.emp");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("scene_dsl_cap_bits: cannot read {}: {e}", path.display()));
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        let Some(rest) = t.strip_prefix("pub const CAP_") else { continue };
        let Some((name_tail, _)) = rest.split_once('=') else { continue };
        let name = format!("CAP_{}", name_tail.trim());
        let v = emp_const_literal(&path, &name);
        out.push((name, v));
    }
    assert!(
        !out.is_empty(),
        "scene_dsl_cap_bits: {} declared NO `pub const CAP_*` — the capability bits are the \
         authority this seam derives from; an empty set would silently elide every gated block",
        path.display()
    );
    out
}

/// A synthesized `.emp` source declaring the `CAP_*` bits as free top-level
/// consts, for a single-module oracle to PREPEND to its dep items.
///
/// Synthesized rather than prepending the real `scene_dsl.emp`: that file is the
/// authoring DSL (enums with payloads, the `scene()`/`fold_caps()` elaborators)
/// and pulling it wholesale into a byte-oracle's closure would widen these
/// deliberately minimal dep lists by a whole subsystem. Only the five integer
/// bits are load-bearing here, and their VALUES still come from that file.
pub fn scene_dsl_cap_consts_src(aeon: &std::path::Path) -> String {
    let mut src = String::from("module engine.level.scene_dsl_caps\n");
    for (name, v) in scene_dsl_cap_bits(aeon) {
        src.push_str(&format!("pub const {name} = ${v:04X}\n"));
    }
    src
}

/// A synthesized `.emp` source re-declaring `engine/level/bg.emp`'s
/// `pub const BG_LAYOUT_SIZE`, for a single-module oracle to PREPEND to its dep
/// items.
///
/// `games/sonic4/data/levels/ojz/act1/act_assets.emp` types its BG-layout embed
/// `[u8; BG_LAYOUT_SIZE]` — the length IS the guard there (a wrong-sized blob
/// must be an `array length mismatch` at build time, not a transposed
/// background at runtime), so the type annotation, and therefore this const,
/// cannot simply be dropped from the oracle's view. `use engine.bg.{…}` has no
/// module to follow in a single-file lower, exactly like raster.emp's
/// `use engine.level.scene_dsl.{CAP_ANCHORS}` above.
///
/// Synthesized rather than prepending the real `bg.emp`: that module is CODE
/// (`BG_Init` and friends, itself `use`ing engine.constants / structs / vdp /
/// z80_bus), and its items lowered into this oracle's file would both drag a
/// subsystem of further unknown names in and EMIT BYTES into the very section
/// being byte-compared. Only the one integer is load-bearing.
///
/// The value is not written down here: the right-hand side is copied VERBATIM
/// out of `bg.emp` (`64*64*2` today) and folded by sigil's own comptime
/// evaluator, so a geometry change in the engine reaches this gate by itself.
/// Should that expression ever start naming other consts, the lower fails loud
/// with `unknown name` rather than binding a stale length.
pub fn bg_layout_size_const_src(aeon: &std::path::Path) -> String {
    let rhs = emp_const_rhs(&aeon.join("engine/level/bg.emp"), "BG_LAYOUT_SIZE");
    format!("module engine.bg_layout\npub const BG_LAYOUT_SIZE = {rhs}\n")
}

/// The resolved game-contract env the raster / parallax / buffers oracles lower
/// against — sonic4's WHOLE contract via [`game_contract_env_from_aeon`] at the
/// canonical shape, not a one-member stub.
///
/// Named for `Game.SCANLINE_CAPS` because that is the member those modules gate
/// their blocks on today; the env carries every other member too, so a second
/// member one of them starts reading resolves instead of aborting the lower. The
/// mask itself still comes from sonic4's own `implement Game` — now by binding
/// that file rather than by re-spelling one line of it.
pub fn scanline_caps_contract_env(
    aeon: &std::path::Path,
) -> sigil_frontend_emp::contract::InterfaceEnv {
    let profile = crate::native::sonic4_profile(false);
    let defines: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    game_contract_env_from_aeon(aeon, &profile, &defines)
}

// ── 5. The whole-path module rig ────────────────────────────────────────────
//
// A module's standalone oracle used to concatenate a HAND-LISTED ambient set and
// lower one synthetic file. That list is a second, silent copy of the module's
// `use` closure, and it rots the moment a dependency grows: aurora's first real
// scene gave the generated `effects_scenes.emp` a body that calls
// `scene_dsl` / `scene_registry` helpers and reads `Game.SCANLINE_CAPS`, and the
// act rig failed on names the real build resolves without a word.
//
// The rig below follows the module's own `use` edges instead, through the SAME
// native closure the ROM is built by ([`build_emp`](crate::native::build_emp) —
// manifest scan, helper normalisation, game-contract bind, `build_program`), and
// slices the one section the oracle gates out of the placed program. A dependency
// the module grows is followed by construction; one it loses stops being lowered.
//
// Doctoring rides a SHADOW TREE: a fresh root holding a COPY of every source the
// build reads, with the overridden files written doctored. Copies, not symlinks:
// `Manifest::scan` never descends a symlinked directory, and the lowering sandbox
// canonicalises every `embed(...)` path and refuses one that resolves outside the
// root (`[sandbox.path-escape]`), which a symlink back into the real tree does.
// What is copied is DERIVED, never named: every top-level directory that holds an
// `.emp` source, then every top-level directory the copied sources `embed(...)`
// from by a root-relative path; everything else (docs, tools, ROMs) is one symlink
// the build never opens. Nested checkouts are skipped exactly as the scan skips
// them, so the copy cannot re-expose a worktree the scan would have hidden.

/// A copied aeon source tree with doctored files — see [`shadow_aeon_tree`].
/// Removed on drop.
pub struct ShadowTree {
    root: PathBuf,
}

impl ShadowTree {
    /// The mirror's root: hand it to [`native_section`] as the aeon tree.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

impl Drop for ShadowTree {
    fn drop(&mut self) {
        // Best effort: a copy that survives is ~20 MB under the system temp dir,
        // never a correctness problem for the next run (each root is unique per
        // process + counter).
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Copy the sources of `aeon` into a fresh temp root, replacing each
/// `(aeon-relative path, contents)` in `overrides` with the doctored text. Every
/// override must name a file that exists under a copied directory — a typo
/// cannot doctor nothing.
pub fn shadow_aeon_tree(
    aeon: &std::path::Path,
    overrides: &[(&str, &str)],
) -> Result<ShadowTree, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn holds_emp(dir: &std::path::Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else { return false };
        for e in entries.flatten() {
            let p = e.path();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                if !is_nested_checkout(&p) && holds_emp(&p) {
                    return true;
                }
            } else if p.extension().is_some_and(|x| x == "emp") {
                return true;
            }
        }
        false
    }

    // The same two signatures `Manifest::scan` refuses to descend.
    fn is_nested_checkout(dir: &std::path::Path) -> bool {
        dir.file_name().is_some_and(|n| n == ".worktrees") || dir.join(".git").exists()
    }

    fn mirror(
        src: &std::path::Path,
        dst: &std::path::Path,
        aeon: &std::path::Path,
        overrides: &[(&str, &str)],
        written: &mut Vec<String>,
    ) -> Result<(), String> {
        std::fs::create_dir(dst).map_err(|e| format!("mkdir {}: {e}", dst.display()))?;
        let entries =
            std::fs::read_dir(src).map_err(|e| format!("read_dir {}: {e}", src.display()))?;
        for e in entries.flatten() {
            let from = e.path();
            let to = dst.join(e.file_name());
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                if is_nested_checkout(&from) {
                    continue;
                }
                mirror(&from, &to, aeon, overrides, written)?;
                continue;
            }
            let rel = from.strip_prefix(aeon).map_err(|e| e.to_string())?;
            let rel = rel.to_string_lossy();
            if let Some((_, text)) = overrides.iter().find(|(r, _)| *r == rel.as_ref()) {
                std::fs::write(&to, text).map_err(|e| format!("write {}: {e}", to.display()))?;
                written.push(rel.into_owned());
            } else {
                std::fs::copy(&from, &to)
                    .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))?;
            }
        }
        Ok(())
    }

    /// The first path segment of every root-relative `embed("…")` under `dir`
    /// (a module-relative `"../…"` embed stays inside its own copied directory).
    fn embed_roots(dir: &std::path::Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                embed_roots(&p, out);
            } else if p.extension().is_some_and(|x| x == "emp") {
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                for tail in text.split("embed(\"").skip(1) {
                    let Some(path) = tail.split('"').next() else { continue };
                    let Some(first) = path.split('/').next() else { continue };
                    if first != ".." && !first.is_empty() && !out.iter().any(|o| o == first) {
                        out.push(first.to_string());
                    }
                }
            }
        }
    }

    let root = std::env::temp_dir().join(format!(
        "sigil-shadow-aeon-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).map_err(|e| format!("mkdir {}: {e}", root.display()))?;
    let tree = ShadowTree { root };
    let mut written = Vec::new();
    let mut copied: Vec<std::ffi::OsString> = Vec::new();
    // Pass 1: every top-level directory holding `.emp` sources, copied.
    let entries =
        std::fs::read_dir(aeon).map_err(|e| format!("read_dir {}: {e}", aeon.display()))?;
    for e in entries.flatten() {
        let name = e.file_name();
        if name == ".git" || name == ".worktrees" {
            continue;
        }
        let from = e.path();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir && !is_nested_checkout(&from) && holds_emp(&from) {
            mirror(&from, &tree.root.join(&name), aeon, overrides, &mut written)?;
            copied.push(name);
        }
    }
    // Pass 2: every top-level directory the copied sources embed from, copied.
    let mut roots = Vec::new();
    for name in &copied {
        embed_roots(&tree.root.join(name), &mut roots);
    }
    for first in roots {
        let from = aeon.join(&first);
        if copied.iter().any(|c| *c == *first.as_str()) || !from.is_dir() {
            continue;
        }
        mirror(&from, &tree.root.join(&first), aeon, overrides, &mut written)?;
        copied.push(first.into());
    }
    // Pass 3: everything else is one symlink the build never opens.
    let entries =
        std::fs::read_dir(aeon).map_err(|e| format!("read_dir {}: {e}", aeon.display()))?;
    for e in entries.flatten() {
        let name = e.file_name();
        if name == ".git" || name == ".worktrees" || copied.contains(&name) {
            continue;
        }
        let to = tree.root.join(&name);
        std::os::unix::fs::symlink(e.path(), &to)
            .map_err(|e| format!("symlink {}: {e}", to.display()))?;
    }
    if let Some((rel, _)) = overrides.iter().find(|(r, _)| !written.iter().any(|w| w == r)) {
        return Err(format!(
            "override `{rel}` names no file under a copied directory of {} — nothing was doctored",
            aeon.display()
        ));
    }
    Ok(tree)
}

/// One section sliced out of a native whole-program build — see [`native_section`].
pub struct NativeSection {
    /// The placed section, exactly as [`build_emp`](crate::native::build_emp)
    /// emitted it (the caller re-places it under its own map).
    pub section: Section,
    /// The program's deferred link asserts whose source lies in the files the
    /// caller named — the subset its own AS-side equ seam can decide.
    pub link_asserts: Vec<LinkAssert>,
}

/// Build `profile` natively from `aeon` (a real tree or a [`ShadowTree`] root)
/// and slice out the one section named `section`. `assert_files` are the
/// aeon-relative sources whose link asserts ride along; the rest of the
/// program's asserts reference AS-side labels a single-section oracle never
/// links, so they are the caller's to exclude by not naming their file.
///
/// A missing dependency reads as the resolver's own diagnostic — `unknown
/// function …`, `unknown name …` — inside the returned `Err`.
pub fn native_section(
    aeon: &std::path::Path,
    profile: &crate::native::GameProfile,
    section: &str,
    assert_files: &[&str],
) -> Result<NativeSection, String> {
    let program = crate::native::build_emp(aeon, profile)?;
    let mut hits: Vec<Section> =
        program.sections.into_iter().filter(|s| s.name == section).collect();
    if hits.len() != 1 {
        return Err(format!(
            "expected exactly one `{section}` section in the {} program, found {}",
            profile.name,
            hits.len()
        ));
    }
    let link_asserts = program
        .link_asserts
        .into_iter()
        .filter(|a| {
            let Some(loc) = program.sources.locate(a.span) else { return false };
            // `path:line:col` — the path never carries a colon on this tree.
            let path = loc.rsplitn(3, ':').nth(2).unwrap_or(&loc);
            std::path::Path::new(path)
                .strip_prefix(aeon)
                .map(|rel| assert_files.iter().any(|f| std::path::Path::new(f) == rel))
                .unwrap_or(false)
        })
        .collect();
    Ok(NativeSection { section: hits.remove(0), link_asserts })
}

/// The files whose link asserts the act-descriptor oracles decide against their
/// AS-side seam: the descriptor itself and the modules the old hand-listed rig
/// carried ambient (each of whose asserts that seam already resolved).
pub const ACT_DESCRIPTOR_ASSERT_FILES: &[&str] = &[
    "games/sonic4/data/levels/ojz/act1/act_descriptor.emp",
    "engine/structs.emp",
    "engine/system/constants.emp",
    "games/sonic4/data/generated/ojz/act1/ojz_act_pool_manifest.emp",
    "games/sonic4/data/generated/ojz/act1/sec_block_dicts.emp",
    "games/sonic4/data/generated/ojz/act1/effects_scenes.emp",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_constant_equs_is_empty_after_the_flips() {
        // Post the P5 constants flip AND the conv-a structs flip every engine
        // constant is .emp-owned and injected as a guarded define, so no mirror
        // drift guard reads a blob value — the list is the zero-length base.
        assert_eq!(
            engine_constant_equs().len(),
            0,
            "all engine-constant + struct-len mirror guards retired; VDP_Shadow_len \
             is now pinned in-file against sizeof(VdpShadow)"
        );
        // The `Stub:` carrier still flushes an (empty) section — a real Vec.
        let _ = as_engine_constants_equs();
    }

    #[test]
    fn sst_supply_blob_carries_the_field_addresses() {
        let _ = as_engine_constants_and_sst_equs();
        assert_eq!(
            sst_field_equs().len(),
            32,
            "the SUPPLY-ONLY SST_* blob carries 31 field offsets (bug005 added the \
             frame_off tail word) + SST_interact for a standalone gate's legitimate \
             field-address externs (ojz_scroll_test)"
        );
    }

    // The `override_doctors_exactly_one_and_keeps_the_rest` test retired with the
    // P5 ownership flip (it doctored an engine constant and asserted the rest kept
    // their truth values; `engine_constant_equs` is now empty — no "rest"). The
    // `override_of_unknown_constant_panics` test below still exercises
    // `with_engine_constant_override`'s typo guard against the empty list.

    #[test]
    #[should_panic(expected = "is not an engine constant")]
    fn override_of_unknown_constant_panics() {
        let _ = with_engine_constant_override("NOT_A_CONSTANT", "0");
    }

    /// EFX-6: the `act_sec_field_equs` supply-only blob had nothing cross-checking
    /// it against the live structs, so a renamed `Act`/`Sec` field could leave a
    /// STALE name in the blob — a dead equ that standalone port test oracles then
    /// resolve against nothing. `act_fixture_drift.rs` already checks the fixture's
    /// VALUES against the harvest and that the harvest's fields are all COVERED by
    /// the fixture, but neither direction catches a name the fixture still supplies
    /// that the live structs no longer declare. This is that missing direction.
    /// A read with no reference tree NAMED stops with a message a reader can act on, and
    /// that message is not countable as a skipped test.
    ///
    /// `d-17` closed the write side: a write into an unnamed reference tree refuses and
    /// prints the path it refused. The read side stayed silent until 2026-08-30, when a
    /// control run bare resolved its oracle to the live checkout and was right only because
    /// that working tree happened to sit at the revision under test. `d-18` closed the rest
    /// of it: the read side now stops rather than announcing, because a run that only says
    /// how much it skipped still exits 0.
    ///
    /// Pinned on the pure message rather than on a captured panic: the refusal fires from
    /// whichever test reaches the resolver first, so an output assertion would pass or fail
    /// on test ordering. The behaviour that the refusal actually FIRES is a subprocess gate,
    /// `crates/sigil-harness/tests/bare_run_refuses.rs`.
    #[test]
    fn a_read_with_no_named_tree_refuses_by_name_and_is_not_a_skip() {
        // A resolved value the assertions below can check the message against, built here
        // rather than read out of the environment: the message's job is to name whatever
        // was resolved, and a test that resolved it for real would assert a different
        // sentence on every box.
        let resolved = super::ResolvedCheckout {
            path: std::path::PathBuf::from("/a/derived/aeon"),
            step: super::PathStep::Derived,
        };
        let notice = super::bare_run_refusal(&resolved.announcement(), Some(&resolved));

        assert!(
            notice.contains("/a/derived/aeon"),
            "the refusal must name the tree it DECLINED, or a reader cannot tell a refusal to \
             use the live checkout from a resolver that simply failed; got: {notice}"
        );
        assert!(
            notice.contains("step 3"),
            "the refusal must say which precedence step answered, so the fix is readable from \
             the message; got: {notice}"
        );
        for name in [super::AEON_DIR_VAR, super::SUITE_ROOT_VAR, super::ALLOW_PARTIAL_VAR] {
            assert!(
                notice.contains(name),
                "the refusal must name `{name}` — the variables that would have answered and \
                 the opt-in that takes the partial run are the whole of what a reader can do \
                 about it; got: {notice}"
            );
        }
        // The landing bar counts BOTH spellings (scripts/landing-run.sh:369,
        // refreeze.rs:533). This refusal is a FAILURE, not a skipped test: counting it as a
        // skip would let the run that STOPPED report a skip total instead of a stop.
        assert!(
            !notice.contains("skip:") && !notice.contains("skipping"),
            "the refusal must not be countable as a skip; got: {notice}"
        );
    }

    /// The declared partial run says how big the hole is, and says so from a DERIVED count.
    ///
    /// The ruling's other half. A partial run that printed no size is the say-nothing
    /// behaviour d-18 replaced; one that printed a number it could not stand behind would
    /// be worse, so the banner reports an unestablished derivation as unknown rather than
    /// as small.
    #[test]
    fn the_partial_run_banner_carries_a_derived_size() {
        let banner = super::partial_run_banner("(resolver context)");
        let gated = crate::reference_dependence::reference_dependent_binaries(
            &crate::reference_dependence::workspace_root(),
        );
        assert!(
            gated.len() > crate::reference_dependence::FLOOR,
            "COULD NOT MEASURE: the reference-dependent derivation found only {}, below its own \
             floor, so this test cannot say what the banner should carry",
            gated.len()
        );
        assert!(
            banner.contains(&gated.len().to_string()),
            "the banner must carry the DERIVED count of what went unmeasured ({}); got: {banner}",
            gated.len()
        );
        assert!(
            banner.contains(super::ALLOW_PARTIAL_VAR) && banner.contains(super::AEON_DIR_VAR),
            "the banner must name the flag that produced this run and the one that ends it; \
             got: {banner}"
        );
        assert!(
            !banner.contains("skip:") && !banner.contains("skipping"),
            "the banner is one line about a whole run, not a skipped test; got: {banner}"
        );
    }

    #[test]
    fn sec_field_equ_names_match_the_harvest() {
        let Some(aeon) = reference_tree(&["engine/structs.emp"]) else { return };
        let harvested = crate::native::harvest_engine_struct_offsets(&aeon)
            .expect("harvest_engine_struct_offsets must succeed");
        let names: std::collections::BTreeSet<&str> =
            harvested.iter().map(|(n, _)| n.as_str()).collect();
        for (name, _) in act_sec_field_equs() {
            assert!(
                names.contains(name),
                "test_support.rs supplies `{name}`, which no longer exists in \
                 engine/structs.emp — a renamed Act/Sec/DMAEntry/parallax_config field \
                 leaves this blob supplying a DEAD equ that standalone port test oracles \
                 then resolve against nothing (EFX-6)"
            );
        }
    }
}


#[cfg(test)]
mod strict_witness_tests {
    /// THE INTERLEAVING REGRESSION. The witness is written from every test thread in
    /// every test binary at once. The first implementation used `writeln!`, whose
    /// `write_fmt` issues one syscall per format fragment, and the fragments spliced:
    /// two distinct call sites arrived as one unparseable line and the distinct-site
    /// count was garbage. `--attest` turns that count into a chain record, so a torn
    /// line is a wrong number in provenance.
    #[test]
    fn concurrent_writers_never_tear_a_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("witness.txt");
        std::thread::scope(|s| {
            for i in 0..16 {
                let path = path.clone();
                s.spawn(move || {
                    for j in 0..64 {
                        use std::io::Write;
                        let line = format!("crates/some/long/path/to/a/test_file_{i}.rs:{j}\n");
                        let mut f = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&path)
                            .unwrap();
                        f.write_all(line.as_bytes()).unwrap();
                    }
                });
            }
        });
        let body = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 16 * 64, "every write must land as exactly one line");
        for l in &lines {
            assert!(
                l.starts_with("crates/some/long/path/to/a/test_file_") && l.matches(".rs:").count() == 1,
                "torn line: {l:?}"
            );
        }
    }
}
