//! Flip Stage 2 · S1.2 — THE FROZEN-CHAINER PLACEMENT GATE (off-canonical).
//!
//! The GameProfile + frozen-table chainer (`native::build_rom_chained` under
//! `SizeSource::Frozen`) computes every off-canonical section's ROM base from the
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
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
fn have_aeon(aeon: &PathBuf) -> bool {
    if aeon.join("s4.bin").exists() {
        return true;
    }
    if strict_gate() {
        panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", aeon.display());
    }
    eprintln!("skip: aeon tree not present (set AEON_DIR)");
    false
}

// The frozen build touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Config-B: the frozen chainer places EVERY frozen-labeled section byte-correctly
/// (empty mismatch set). Proves the declared-order computed-base mechanism from the
/// committed `config_b.txt` — TestPlayer/OJZ/error-handler/EndOfRom all land at their
/// config_b addresses, not sonic4's.
#[test]
fn config_b_frozen_placement_exact() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let profile = native::config_b_profile();
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
    match &profile.size_source {
        native::SizeSource::Frozen(t) => t.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        _ => panic!("{}: not a Frozen profile", profile.name),
    }
}

/// For each off-canonical target: the LMA-native derivation reproduces the committed
/// size table byte-for-byte (same label set, same addresses). This is the P4a proof —
/// sigil's own resolve now carries every boundary address asl once listed, including the
/// phased z80 idle (ROM LMA `$3d8`, not VMA `$0`) and the synthesized `*_End` markers.
fn rederives_native(profile: native::GameProfile) {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
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
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let secs = native::resolve_canonical_sections(&aeon, debug).unwrap_or_else(|e| {
        panic!("resolve canonical sections ({}): {e}", if debug { "debug" } else { "plain" })
    });

    let mut ram: Vec<&sigil_ir::Section> =
        secs.iter().filter(|s| s.vma_origin() >= 0xFFFF_0000).collect();
    ram.sort_by_key(|s| s.vma_origin());

    // (a) even-based, and there IS a RAM region (guards against a resolve that lost it).
    assert!(!ram.is_empty(), "no RAM sections resolved — the phase blocks vanished");
    for s in &ram {
        assert_eq!(
            s.vma_origin() % 2,
            0,
            "RAM section `{}` base {:#x} is ODD — a 68k word/long access there address-errors",
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
        "{rom_in_ram} ROM section(s) placed at a RAM lma — the ROM/RAM partition broke; \
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

/// t24 — the table's TWO authorities under packed placement (Wave-B B-0):
/// (a) an ISLAND ANCHOR is load-bearing — doctoring `ObjCodeBase` (+2) MUST move the
///     ROM (or fail to resolve): a corrupted anchor cannot be silently absorbed;
/// (b) a CONTIGUOUS entry is deliberately INERT — doctoring `HeightMaps` (+2) MUST
///     leave the ROM byte-identical (bases repack from live sizes; a stale contiguous
///     address is exactly what the packing walk immunizes against).
/// (Condition-3 control from the S1.2 size-capture handoff, re-scoped for packing.)
#[test]
fn config_b_doctored_size_table_breaks_the_build() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // The honest baseline: the undoctored chained ROM's anchor matches the golden.
    let golden = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../sigil-harness/golden/config_b.bin"),
    )
    .unwrap_or_else(|e| panic!("read golden: {e}"));
    // Release ships NOTHING past EndOfRom since item 29, so the golden's own length
    // IS the anchor end — a literal here rotted once already (it held the pre-strip
    // 0x43470 and indexed past the 4.2 KB-smaller post-strip ROM).
    let eor = golden.len();
    let base_profile = native::config_b_profile();
    let base = native::build_rom_chained(&aeon, &base_profile).unwrap_or_else(|e| panic!("{e}"));
    let header = |i: usize| {
        sigil_harness::CHECKSUM_FIELD_RANGE.contains(&i) || sigil_harness::ROM_END_FIELD_RANGE.contains(&i)
    };
    assert!(
        (0..eor).all(|i| header(i) || base[i] == golden[i]),
        "control: undoctored config_b anchor must match the golden"
    );

    // (a) Doctor an ISLAND ANCHOR (+2) and rebuild. The packing walk anchors the object
    // bank at the table address, so the ROM MUST diverge (or fail to resolve).
    let mut profile = native::config_b_profile();
    let probe = "ObjCodeBase";
    if let native::SizeSource::Frozen(t) = &mut profile.size_source {
        *t.get_mut(probe).unwrap_or_else(|| panic!("{probe} not in table")) += 2;
    }
    match native::build_rom_chained(&aeon, &profile) {
        Err(_) => { /* a resolve failure is a loud catch — acceptable */ }
        Ok(doctored) => {
            let diverges = doctored.len() != base.len()
                || (0..eor.min(doctored.len())).any(|i| !header(i) && doctored[i] != golden[i]);
            assert!(
                diverges,
                "t24: doctoring the island anchor `{probe}`+2 left the ROM byte-identical to \
                 the golden — the anchor authority is NOT load-bearing (the gate would be vacuous)"
            );
        }
    }

    // (b) Doctor a CONTIGUOUS entry (+2): packing must ignore it — byte-identical build.
    let mut inert_profile = native::config_b_profile();
    let inert = "HeightMaps";
    if let native::SizeSource::Frozen(t) = &mut inert_profile.size_source {
        *t.get_mut(inert).unwrap_or_else(|| panic!("{inert} not in table")) += 2;
    }
    let repacked = native::build_rom_chained(&aeon, &inert_profile)
        .unwrap_or_else(|e| panic!("t24(b): contiguous doctoring must still build: {e}"));
    assert_eq!(
        repacked, base,
        "t24(b): doctoring the contiguous `{inert}`+2 changed the ROM — packing is supposed \
         to derive contiguous bases from live sizes, not the table"
    );
}
