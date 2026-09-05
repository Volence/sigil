//! Flip Stage 2 · S1.2 — THE FROZEN-CHAINER PLACEMENT GATE (off-canonical).
//!
//! The GameProfile + frozen-table chainer (`native::build_rom_chained` under
//! the frozen boundary table) computes every off-canonical section's ROM base from the
//! committed listing table (`golden/offcanonical_sizes/*.txt`) instead of the baked
//! sonic4 resume orgs. This gate proves the PLACEMENT half of that mechanism on
//! Config-B (sonic4, sound-off): the chainer builds end-to-end (no overlap, no drift
//! guard fires) AND every frozen-labeled section resolves to its exact frozen address —
//! `frozen_placement_mismatches` returns EMPTY.
//!
//! Why placement, not byte-identity: Config-B's full byte-identity is BLOCKED pre-flip
//! by assembly-time-FOLDED sonic4 constants (`Game_Entry = $5C7EC`,
//! `ErrorHandler: equ $5CC0A` — the brief's rows 52/90), which are numeric constants the
//! chainer cannot re-place; they only "resolve natively" at the flip. See
//! `docs/superpowers/notes/2026-07-30-flip-stage2-drivers-blocker-checkpoint.md`. This
//! gate isolates the chainer's correctness (every DECLARED section placed exactly right)
//! from that independent fold blocker, and keeps the Frozen machinery under test.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_offcanonical_placement
//! ```
use sigil_harness::native;
use sigil_harness::test_support::reference_tree_for_profile;

// The frozen build touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Config-B: the frozen chainer places EVERY frozen-labeled section byte-correctly
/// (empty mismatch set). Proves the declared-order computed-base mechanism from the
/// committed `config_b.txt` — TestPlayer/OJZ/error-handler/EndOfRom all land at their
/// config_b addresses, not sonic4's.
#[test]
fn config_b_frozen_placement_exact() {
    let profile = native::config_b_profile();
    let Some(aeon) = reference_tree_for_profile(&profile) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mismatches =
        native::frozen_placement_mismatches(&aeon, &profile).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        mismatches.is_empty(),
        "config_b frozen chainer misplaced {} declared section(s); first: {:?}",
        mismatches.len(),
        mismatches.first()
    );
}

// ── P4a — THE LMA-CORRECT SIZE DERIVATION (the asl-`.lst` parse retires) ──
//
// `native::derive_frozen_table` reads each boundary label's ROM address off sigil's OWN
// resolved layout (`section.lma + label.offset`), synthesizing the section-END markers
// from section geometry. This reproduces the committed table (now sigil-native
// provenance) EXACTLY for every off-canonical target — the fixpoint that makes sigil's
// resolve the size authority (kill-list rows 34/95; no asl listing is parsed).

/// The committed table's label→address map (comment lines stripped) for `<stem>.txt`.
fn committed_table(profile: &native::GameProfile) -> std::collections::BTreeMap<String, u32> {
    profile.frozen_sizes.iter().map(|(k, v)| (k.clone(), *v)).collect()
}

/// For each off-canonical target: the LMA-native derivation reproduces the committed
/// size table byte-for-byte (same label set, same addresses). This is the P4a proof —
/// sigil's own resolve now carries every boundary address asl once listed, including the
/// phased z80 idle (ROM LMA `$3d8`, not VMA `$0`) and the synthesized `*_End` markers.
fn rederives_native(profile: native::GameProfile) {
    let Some(aeon) = reference_tree_for_profile(&profile) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let committed = committed_table(&profile);
    let derived = native::derive_frozen_table(&aeon, &profile).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        derived, committed,
        "{}: LMA-native derivation diverged from the committed table (re-derive the \
         offcanonical_sizes/*.txt with `derive_offcanon`?)",
        profile.name
    );
}

#[test]
fn demo_size_table_rederives_native() {
    rederives_native(native::demo_profile(false));
}
#[test]
fn demo_debug_size_table_rederives_native() {
    rederives_native(native::demo_profile(true));
}
#[test]
fn config_a_size_table_rederives_native() {
    rederives_native(native::config_a_profile());
}
#[test]
fn config_b_size_table_rederives_native() {
    rederives_native(native::config_b_profile());
}
/// LEAN (crash-report OFF) — the 7th frozen table. It is the only committed table that
/// carries a `ReleaseFault` row and no `BusError`, so this is where a regression that
/// quietly re-attached the error_handler island to a CRASH_REPORT=0 build would surface
/// as a label-set divergence rather than a byte diff.
#[test]
fn lean_size_table_rederives_native() {
    rederives_native(native::lean_profile());
}

/// Wave-B B-0b — the RAM-packing invariant guard. RAM is placed by AS `phase`/
/// `dephase` (`engine/ram.asm` at `$FFFF0000`/`$FFFF8000`) plus `phase Engine_RAM_End`
/// (game RAM chains onto the engine block) — the RAM analog of B-0's contiguous
/// packing, executed natively by sigil's AS frontend. This gate asserts the three
/// structural properties that keep a RAM-growing parcel (entity_window #1, tile_cache
/// #2) safe, in BOTH shapes:
///   (a) every RAM section (`vma_origin >= $FFFF0000`) is EVEN-based — the 68k
///       address-error guard;
///   (b) NO ROM section lands in RAM (`is_rom_section` partition holds) — ROM bases
///       are independent of RAM sizes, so RAM growth never perturbs the ROM layout;
///   (c) upper RAM PACKS CONTIGUOUSLY — sections at `vma >= $FFFF8000`, sorted, each
///       successor base == predecessor end (game RAM abuts `Engine_RAM_End`). A gap
///       or overlap means the packing broke.
/// See `docs/superpowers/notes/2026-08-01-waveb-b0b-ram-packing.md` for the growth
/// probe that exercised these under a live +2 RAM growth.
fn ram_packing_invariants(debug: bool) {
    // `resolve_canonical_sections` resolves the sonic4 shape — its profile names the
    // sources this walk reads.
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(debug)) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let secs = native::resolve_canonical_sections(&aeon, debug).unwrap_or_else(|e| {
        panic!("resolve canonical sections ({}): {e}", if debug { "debug" } else { "plain" })
    });

    let mut ram: Vec<&sigil_ir::Section> =
        secs.iter().filter(|s| s.vma_origin() >= 0xFFFF_0000).collect();
    ram.sort_by_key(|s| s.vma_origin());

    // (a) even-based, and there IS a RAM region (guards against a resolve that lost it).
    assert!(!ram.is_empty(), "no RAM sections resolved, the phase blocks vanished");
    for s in &ram {
        assert_eq!(
            s.vma_origin() % 2,
            0,
            "RAM section `{}` base {:#x} is ODD, a 68k word/long access there address-errors",
            s.name,
            s.vma_origin()
        );
    }

    // (b) partition: no ROM section carries a high (RAM) lma; RAM sizes cannot move ROM.
    let rom_in_ram = secs
        .iter()
        .filter(|s| s.vma_origin() < 0xFFFF_0000 && s.lma >= 0xFFFF_0000)
        .count();
    assert_eq!(
        rom_in_ram, 0,
        "{rom_in_ram} ROM section(s) placed at a RAM lma, the ROM/RAM partition broke; \
         RAM growth could perturb ROM bases"
    );

    // (c) upper RAM (vma >= $FFFF8000) packs contiguously: successor base == predecessor end.
    let upper: Vec<&sigil_ir::Section> =
        ram.iter().copied().filter(|s| s.vma_origin() >= 0xFFFF_8000).collect();
    for w in upper.windows(2) {
        let end = w[0].vma_origin() + w[0].reserved_span;
        assert_eq!(
            w[1].vma_origin(),
            end,
            "upper-RAM packing broke: `{}` ends {:#x} but `{}` begins {:#x} \
             (game RAM must abut Engine_RAM_End with no gap/overlap)",
            w[0].name,
            end,
            w[1].name,
            w[1].vma_origin()
        );
    }
}

#[test]
fn ram_packing_invariants_plain() {
    ram_packing_invariants(false);
}
#[test]
fn ram_packing_invariants_debug() {
    ram_packing_invariants(true);
}

/// t24 — the frozen table is NOT a placement input for either kind of row:
/// (a) an ISLAND row doctored (+2) stops being an island (islands are map anchors) and
///     the section packs to its DECLARED alignment, which for the object bank is the
///     same 0x10000 — the ROM stays byte-identical;
/// (b) a CONTIGUOUS entry is INERT — doctoring it (+2) leaves the ROM byte-identical
///     (bases repack from live sizes; a stale contiguous address is exactly what the
///     packing walk immunizes against).
/// (Condition-3 control from the S1.2 size-capture handoff, re-scoped for packing.)
#[test]
fn config_b_doctored_size_table_moves_no_bytes() {
    let Some(aeon) = reference_tree_for_profile(&native::config_b_profile()) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // The honest baseline: the undoctored chained ROM's anchor matches the golden.
    let golden = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../sigil-harness/golden/config_b.bin"),
    )
    .unwrap_or_else(|e| panic!("read golden: {e}"));
    // The anchor end comes from the PROVENANCE TIP, never from a literal and never
    // from the golden's own length. Both cheaper sources have now rotted once each:
    // a hand-typed `0x43470` survived the item-29 strip and indexed past the 4.2 KB
    // -smaller ROM, and its replacement `golden.len()` rested on "release ships
    // nothing past EndOfRom", which the crash-report ruling repealed — config_b is a
    // release shape and now carries a ~28 KB deb2 symbol appendix, so the golden is
    // far longer than the assembled image `build_rom_chained` returns. `tip_target`
    // is the same source `native_offcanonical_rom::anchor_end` uses, and
    // `provenance_chain` proves it against the committed blob.
    let eor = sigil_harness::provenance::tip_target(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden"),
        "config_b",
    )
    .unwrap_or_else(|e| panic!("provenance tip: {e}"))
    .anchor_end;
    assert!(
        golden.len() >= eor,
        "golden config_b.bin ({}) is shorter than its anchor end ({eor:#x})",
        golden.len()
    );
    let base_profile = native::config_b_profile();
    let base = native::build_rom_chained(&aeon, &base_profile).unwrap_or_else(|e| panic!("{e}"));
    let header = |i: usize| {
        sigil_harness::CHECKSUM_FIELD_RANGE.contains(&i) || sigil_harness::ROM_END_FIELD_RANGE.contains(&i)
    };
    assert!(
        (0..eor).all(|i| header(i) || base[i] == golden[i]),
        "control: undoctored config_b anchor must match the golden"
    );

    // (a) Doctor the ISLAND ANCHOR's frozen row (+2) and rebuild. The frozen row is not
    // the anchor authority: an island is a section whose provisional base IS a map
    // `[[anchor]]`, so the doctored 0x10002 is no anchor at all and the object bank packs
    // as a chained section — to `align_up(running, 0x10000)`, its declared alignment
    // (`section_align::OBJ_BANK_64K`), which is 0x10000 again. The ROM MUST be
    // byte-identical: neither the island's address nor its alignment came from the
    // table. (That the map anchor itself is load-bearing — held absolute, overrun
    // refused — is `native::derived_layout_tests::growth_into_a_declared_anchor_still_fails_loud`
    // and `stale_provisional_gap_is_an_island_only_when_declared`.)
    let mut profile = native::config_b_profile();
    let probe = "ObjCodeBase";
    *profile.frozen_sizes.get_mut(probe).unwrap_or_else(|| panic!("{probe} not in table")) += 2;
    let doctored = native::build_rom_chained(&aeon, &profile)
        .unwrap_or_else(|e| panic!("a doctored island row must still build (the map anchors, the declaration aligns): {e}"));
    assert!(
        doctored.len() == base.len()
            && (0..eor).all(|i| header(i) || doctored[i] == golden[i]),
        "doctoring the island row `{probe}`+2 moved the ROM, the frozen table is still a \
         placement input for the object bank (its anchor is the map's, its alignment the declaration's)"
    );

    // (b) Doctor a CONTIGUOUS entry (+2): packing must ignore it — byte-identical build.
    //
    // The probe entry must GENUINELY abut its predecessor in the CURRENT freeze —
    // contiguity is incidental layout, not a property of the symbol. The previous
    // hardcoded probe (`HeightMaps`) rotted at `defect-batch-8`: the parcel's
    // upstream shift left `Ani_Particle_End 0x25838` / `HeightMaps 0x25840` — an
    // 8-byte alignment hole — so `is_anchor` (tb > chain_end) started honoring the
    // TABLE for collision_data and the doctoring legitimately changed the ROM. The
    // precondition assert makes that rot mode loud instead of a byte-dump: if it
    // fires, the layout shifted — pick a head label that abuts again.
    let mut inert_profile = native::config_b_profile();
    // objtest-gate (2026-08-05): Ani_Particle left the table (particle_anims is
    // DEBUG-only) — and with it gone, HeightMaps abuts Ani_Sonic_End again, so
    // the original probe entry is valid once more, now precondition-guarded.
    // tails-data (2026-08-10): the precondition FIRED, exactly as designed —
    // `Ani_Tails` (the Tails anim scripts) now sits between `Ani_Sonic_End` and
    // `HeightMaps`, and its 0xEA length leaves a 6-byte align pad before
    // HeightMaps, so HeightMaps stopped abutting. The probe moved one section
    // EARLIER, to `Ani_Tails` itself.
    // tails-flight (2026-08-11): it fired AGAIN, and for the same reason one step
    // further up. `ANIM_FLY`/`ANIM_FLY_TIRED` joined the shared id space, so every
    // character's anim table grew two rows: `Ani_Sonic_End` stopped being
    // 16-aligned and `Ani_Tails` now sits behind an 8-byte pad. Chasing the same
    // section a third time is the wrong move — the anim tables grow whenever an
    // animation is added, so any pair inside that run rots on a schedule.
    // The probe therefore moves UPSTREAM of the growth, to `Ani_Sonic` anchored on
    // `Map_TestObj_End`: test_mappings is a fixed-size index (0x30 — a word-offset
    // table with one entry per test object, not per animation), so its end abuts
    // `Ani_Sonic` exactly and stays put across anim churn. Same property under
    // test; a predecessor that does not move when animations do.
    // dust (2026-08-11, aeon 6a2f26f2, refreeze chains 98-99): the precondition
    // fired a THIRD time, but this time the probe itself is still sound — the
    // dust sprite-data section (head `Map_DustSpindash`, per map.toml order
    // Map_TestObj -> Map_DustSpindash -> Ani_Sonic) landed between the pair.
    // `Map_DustSpindash` is NOT a boundary key (the frozen key set is inherited
    // from the committed table; the chainer doesn't need one for it), so the
    // table alone can no longer witness the abutment by key equality. The
    // section's image is the four generated dust blobs back-to-back
    // (map_dust_spindash + map_dust_puff + dplc_dust + art_dust = 0xBDA today),
    // and it fills the hole EXACTLY — `Ani_Sonic` still abuts, one section
    // later. The witness therefore adds the MEASURED blob lengths from the same
    // aeon tree the build embeds: it tracks dust-art regeneration instead of
    // rotting on it, and any padding/reorder between the pair still fails loud.
    let inert = "Ani_Sonic";
    let inert_pred_end = "Map_TestObj_End";
    let dust_data_len: u32 = ["map_dust_spindash.bin", "map_dust_puff.bin", "dplc_dust.bin", "art_dust.bin"]
        .iter()
        .map(|f| {
            let p = aeon.join("games/sonic4/data/generated/dust").join(f);
            std::fs::metadata(&p)
                .unwrap_or_else(|e| panic!("t24(b) precondition: dust blob {} unreadable: {e}", p.display()))
                .len() as u32
        })
        .sum();
    {
        let t = &inert_profile.frozen_sizes;
        let head = *t.get(inert).unwrap_or_else(|| panic!("{inert} not in table"));
        let prev = *t.get(inert_pred_end).unwrap_or_else(|| panic!("{inert_pred_end} not in table"));
        assert_eq!(
            head,
            prev + dust_data_len,
            "t24(b) precondition: `{inert}` ({head:#x}) no longer abuts `{inert_pred_end}` \
             ({prev:#x}) + the dust_data image ({dust_data_len:#x}) in the frozen table, \
             the freeze shifted the layout; choose a new contiguous probe entry"
        );
    }
    // The doctoring delta must PRESERVE the entry's inferred alignment class:
    // the packer derives a contiguous section's alignment as the largest power
    // of two <= 16 dividing its table value, so a naive +2 on a 2-aligned value
    // can UPGRADE it (0x...E+2 = 0x...0 -> align 16) and legitimately insert
    // pad. delta = 2*align keeps the 2-adic valuation (align*odd + 2*align =
    // align*odd'), so the only thing the doctoring can change is the base the
    // packing must anyway ignore. (Found at objtest-gate: the re-abutted
    // HeightMaps sits 2-aligned now, where the pre-defect-batch value was
    // 16-aligned and +2 happened to DOWNGRADE — inert by accident.)
    let delta;
    {
        let e = inert_profile
            .frozen_sizes
            .get_mut(inert)
            .unwrap_or_else(|| panic!("{inert} not in table"));
        let align =
            (1u32..=16).filter(|a| a.is_power_of_two() && (*e).is_multiple_of(*a)).max().unwrap();
        delta = 2 * align;
        *e += delta;
    }
    let repacked = native::build_rom_chained(&aeon, &inert_profile)
        .unwrap_or_else(|e| panic!("t24(b): contiguous doctoring must still build: {e}"));
    assert_eq!(
        repacked, base,
        "t24(b): doctoring the contiguous `{inert}`+{delta} changed the ROM, packing is \
         supposed to derive contiguous bases from live sizes, not the table"
    );
}
