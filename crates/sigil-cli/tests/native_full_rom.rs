//! Flip Stage 1 · S1.4 — THE SPLIT-GOLDEN FULL-FILE GATES (Option A).
//!
//! The full native ROM = the assembled image (checksum-folded by `emit_rom`) + the
//! SIGIL-CANONICAL deb2 symbol appendix, produced by driving the REAL
//! `tools/convsym` + `tools/fixheader` over sigil's OWN listing (`build.sh:169-175`, fed sigil's
//! `.lst` instead of asl's). Under Volence/overseer Option A the appendix is NOT a
//! byte-imitation of asl's name set — the `.emp` names are the source names going
//! forward. So the bar SPLITS (2026-07-30-flip-stage1-S1.4-appendix-fork.md):
//!
//!   - ASSEMBLED PREFIX `[0, EndOfRom)` == asl (header-neutral) — the correctness
//!     anchor, asl-witnessed, unchanged. Proven by `native_rom_{plain,debug}`.
//!   - FULL FILE == the sigil-canonical golden (deterministic; CRC-pinned here,
//!     blob-frozen in the golden-freeze stage).
//!   - FUNCTIONAL TRUTH (condition 2): determinism, assert-PRESENCE (`de b2` + size
//!     band), convsym rc 0, and a load-bearing spot-check resolving to exact known
//!     addresses through the REAL convsym consumer path — plus a t24 doctored-symbol
//!     negative control.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_full_rom
//! ```

use sigil_harness::{native, pins};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
fn read_ref(name: &str) -> Option<Vec<u8>> {
    let path = aeon_dir().join(name);
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

// The native full-file build touches the shared `engine/sound/generated` dir and
// spawns convsym over temp files — serialize the shapes.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The load-bearing spot-check set (condition 2c): one Game_Entry-class, one
/// ErrorHandler-class, a player proc, a level proc, a Z80-adjacent, plus engine
/// keystones — each MUST resolve to its exact per-shape address through the REAL
/// `convsym -output log` consumer. Addresses are listing-truth (pins.rs / s4.lst).
/// `(name, plain, debug)`.
const LOAD_BEARING: &[(&str, u32, u32)] = &[
    ("EntryPoint", 0x200, 0x200),        // Game_Entry-class
    ("GameLoop", pins::GAME_LOOP.plain_base, pins::GAME_LOOP.debug_base), // the main loop (pin-sourced)
    // ErrorHandler-class spot-check, back in this both-shapes list: since the
    // crash-report ruling (owner-ruled 2026-08-04) the error_handler island ships in
    // BOTH canonical shapes, so `BusError` (the island head) has a real per-shape
    // address again and the region pin carries both. It replaces the shape-split
    // BusError/ReleaseFault pair that item 29 part 4 forced.
    ("BusError", pins::ERROR_HANDLER.plain_base, pins::ERROR_HANDLER.debug_base),
    // A player proc. Re-derived at the bug005 refreeze (aeon master ec8a1cc,
    // listings s4.lst/s4.debug.lst): +0x22 plain / +0x7A debug — player_common's
    // parcel growth (+0x18/+0x70) slid player_ground's base, and the G1
    // jump-headroom carry (277f384) grew the ground code ahead of the cap proc
    // (+0xA both shapes). No pin exists for an intra-region player label, and it
    // must STAY unpinned: the whole point of this row is to be an INDEPENDENT
    // expectation checking the convsym resolve path, so deriving it from a pin
    // would make the assertion circular. Hand-update it, with the reason.
    // `cheat-flag` (2026-08-05): +0x10 BOTH shapes. player_common gained the three
    // debug-fly gate sites (0x8 in Player_Main, 0x6 in Player_Init) ahead of
    // player_ground; 16-byte region alignment rounds that to a uniform 0x10.
    // `b-jumps` (2026-08-05): +0x10 PLAIN ONLY. Player_Main's jump latch grew by
    // 0xE (B joined BUTTON_JUMP_MASK, gated on CHEAT_DEBUG_FLY) ahead of
    // player_ground, and 16-byte region alignment rounds that to 0x10. The DEBUG
    // value is unchanged because player_common's debug region carried enough pad
    // slack to absorb 0xE without crossing an alignment boundary — the shapes
    // legitimately move by different amounts, which is why this row is a
    // per-shape pair and not a single delta.
    // `defect-batch-8` (2026-08-05): +0x10 DEBUG ONLY. player_common's C1c swap
    // (bare ori -> set_priority_band's andi+ori pair, +0x6) crossed a 16-byte
    // alignment boundary in the debug region; plain absorbed the same +0x6 in
    // existing pad slack — the mirror image of the b-jumps shift above.
    // `art-streaming-p2-task2` (2026-08-08): -0x2 DEBUG ONLY. The zx0_resume section
    // (+0x80) inserted ahead of the object bank + the three DEBUG PageIn counters (+0x6
    // RAM) flip a cross-bank branch reach in the debug shape, netting -0x2 at this
    // intra-bank label; plain is unchanged (the object bank is absolutely anchored and
    // the plain shape kept the same branch widths). Per-shape pair, hand-updated.
    // `art-streaming-p2-task3` (2026-08-08): a further -0x2 DEBUG ONLY. The Task-3 page_in
    // section (+0x166 debug) + the VBlank hook + the boot-debug reach ripple push the
    // cross-bank distance again, flipping another debug branch reach at this intra-bank
    // label (0x107C8 -> 0x107C6); plain 0x10766 unchanged (object bank anchored).
    // `player-polish-trio` (2026-08-09): the .ball roll-hold recompute grows
    // player_common upstream of this label — +0x20 plain / +0x10 debug (a
    // debug branch-reach flip absorbs 0x10, the same class as the p2-task3
    // note above).
    // `character-dispatch-c1` (2026-08-10): plain 0x10786 -> 0x107B6, debug
    // 0x107D6 -> 0x108C6. Two effects stack. (1) player_common grows ahead of
    // player_ground, sliding its base +0x20 plain / +0x70 debug. (2) player_ground
    // itself repacks: its physics/quadrant/jump-buffer reads left absolute EAs for
    // a4-relative PBLK_* displacements, so the module now carries NO shape-
    // dependent operand at all and its intra-region layout is identical in both
    // shapes — this label sits at region base + 0x2E6 in BOTH. The old debug
    // literal implied 0x266, which the shape-invariance says was already stale; it
    // survived because the `assembled length` assert above fires first and masked
    // this row on every run since it drifted. The row stays UNPINNED on purpose
    // (an independent check of the convsym resolve path — deriving it from a pin
    // would make the assertion circular), so it is hand-updated, with the reason.
    // `tails-flight` (2026-08-11): plain 0x107A6 -> 0x107F2, debug 0x108C6 ->
    // 0x10912 — +0x4C in BOTH shapes, and it decomposes exactly: player_common
    // grows (0x4CE -> 0x4FE plain, 0x5DE -> 0x61E debug) and slides player_ground's
    // base +0x40 in each, then player_ground's own body grows +0xC where the roll
    // hold and the unroll ceiling probe started deriving the curl geometry from the
    // record instead of folding a constant. The intra-region offset lands at 0x2F2
    // in BOTH shapes (it was 0x2E6 in both) — still shape-invariant, which is the
    // property this row exists to witness, and the reason the pair could be
    // re-derived rather than guessed.
    // `tails-character` (2026-08-10): plain 0x107B6 -> 0x107A6, DEBUG UNCHANGED.
    // Making Tails a real record shrinks player_common's PLAIN span 0x4CE -> 0x4BE
    // (a 16-byte region-alignment step), sliding player_ground's plain base
    // 0x104D0 -> 0x104C0; the debug region carried enough pad slack to absorb the
    // same change without crossing a boundary, so its base holds at 0x105E0. The
    // intra-region offset is 0x2E6 in BOTH shapes, unchanged from chain 89 — this
    // label still moves only with its region base, which is the property the row
    // is here to witness. Per-shape pair, hand-updated (unpinned on purpose).
    // `dust` (2026-08-11, aeon 26344203/24e7f6a0, chains 99-100): plain 0x107F2 ->
    // 0x10802, debug 0x10912 -> 0x10922 — +0x10 in BOTH shapes. Tasks 4+5 grew
    // player_common ahead of player_ground (Player_Display's `jbsr Dust_Tick` +
    // PHook_SpindashEnter's `jbsr DustSpindash_Spawn`): 0x4FE -> 0x50E plain,
    // 0x61E -> 0x62E debug, sliding player_ground's base 0x10500 -> 0x10510 /
    // 0x10620 -> 0x10630. The intra-region offset holds at 0x2F2 in BOTH shapes
    // (unchanged since tails-flight) — still shape-invariant, so the pair was
    // re-derived from base + 0x2F2 rather than guessed.
    // `knuckles-def` (2026-08-12, C4 task 9): plain 0x10802 -> 0x10822, debug
    // 0x10922 -> 0x10942 — +0x20 in BOTH shapes. The per-character CRAM line 0
    // added ~0x20 to Player_RefreshPhysics (re-load the record, test the pointer,
    // an eight-long copy loop, the dirty bit), and that proc is in player_common,
    // ahead of player_ground: the region slides 0x10510 -> 0x10530 plain /
    // 0x10630 -> 0x10650 debug. Symmetric because the copy is not DEBUG-fenced.
    // The intra-region offset holds at 0x2F2 in BOTH shapes (unchanged since
    // tails-flight) — still shape-invariant, which is the property this row exists
    // to witness, so the pair was re-derived from base + 0x2F2 rather than guessed.
    // Note this label moved for a reason ENTIRELY separate from the parcel's big
    // number: the 0x226D0 of Knuckles art lands at the ROM tail, far behind here.
    // `knuckles-c4` (2026-08-12, C4 tasks 10-11): plain 0x10822 -> 0x108F2, debug
    // 0x10942 -> 0x10A12 — +0xD0 in BOTH shapes. Knuckles' glide family grew
    // player_common ahead of player_ground: five new state rows in each of the
    // three parallel tables (Player_States / PState_EnterHooks / PState_ExitHooks
    // for GLIDE/GLIDEFALL/SLIDE/CLIMB/LEDGE), their enter hooks, and the PlayerV
    // scratch the climb needs. player_ground's base slides 0x10530 -> 0x10600
    // plain / 0x10650 -> 0x10720 debug (repin's P_STATE_GROUND, same +0xD0).
    // Symmetric because none of it is DEBUG-fenced. The intra-region offset holds
    // at 0x2F2 in BOTH shapes (unchanged since tails-flight) — still
    // shape-invariant, which is the property this row exists to witness, so the
    // pair was RE-DERIVED as base + 0x2F2, not guessed: 0x10600 + 0x2F2 = 0x108F2
    // and 0x10720 + 0x2F2 = 0x10A12.
    ("Ground_Move_Cap", 0x108F2, 0x10A12),
    ("Section_Init", pins::SECTION.plain_base, pins::SECTION.debug_base), // a level proc (rides the m1-budget-fix vblank growth; pin-sourced so downstream shifts don't rot the fixture)
    ("BG_Init", pins::BG.plain_base, pins::BG.debug_base),                // a level proc (after PARALLAX + SECTION, so it rides their growth; pin-sourced)
    ("AnimateSprite", pins::ANIMATE.plain_base, pins::ANIMATE.debug_base), // an objects keystone (pin-sourced)
    ("TouchResponse", pins::COLLISION.plain_base, pins::COLLISION.debug_base), // a collision keystone (pin-sourced)
    ("Z80_Sound_Start", pins::BOOT_HEAD.plain_base + 0x36, pins::BOOT_HEAD.debug_base + 0x36), // Z80-adjacent = BootData+54 (pin-sourced)
];

/// The golden directory (holds the frozen blobs + `provenance.toml`, the single source
/// of the expected CRC/size — the hand-edited const surface is retired).
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden")
}

/// The sigil-canonical full-file CRC-32 + size, sourced from the provenance chain TIP
/// (`golden/provenance.toml`). The ASSEMBLED prefix is asl-identical (the PRIMARY
/// anchor); the FULL-FILE values are sigil-canonical (the appendix is sigil's) and move
/// on any golden re-freeze — a mismatch is the intended "re-freeze the golden" signal,
/// per the split-golden model. `provenance_chain` independently proves the tip equals
/// the committed blobs, so build == tip here means build == the frozen golden.
fn expected_full(key: &str) -> (u32, usize) {
    let t = sigil_harness::provenance::tip_target(&golden_dir(), key)
        .unwrap_or_else(|e| panic!("provenance tip: {e}"));
    let crc = sigil_harness::provenance::hex_u32(&t.full_crc).unwrap_or_else(|e| panic!("{e}"));
    (crc, t.full_size)
}

fn run_shape(debug: bool, refname: &str, key: &str) {
    let expect = expected_full(key);
    let Some(refrom) = read_ref(refname) else { return };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let shape = if debug { "debug" } else { "plain" };

    // The assembled prefix stays the asl-witnessed correctness anchor.
    let native::RomBuild { rom: assembled, listing, .. } =
        native::build_native_rom_with_listing(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let eor = if debug { pins::DEBUG_ASSEMBLED_LEN } else { pins::ASSEMBLED_LEN };
    assert_eq!(assembled.len(), eor, "{shape}: assembled length");

    // FULL FILE: assembled + sigil-canonical deb2 appendix (presence control lives
    // inside build_native_full_file — a silent convsym failure is a HARD error).
    let full = native::build_native_full_file(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));

    // (a) DETERMINISM — a second build is byte-identical.
    let full2 = native::build_native_full_file(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(full, full2, "{shape}: full file non-deterministic");

    // (b) PRESENCE — asserted in BOTH canonical shapes now. This assertion INVERTED
    // at the crash-report ruling (owner-ruled 2026-08-04): item 29 had release
    // shipping nothing past EndOfRom, and this arm proved that absence. The ruling
    // reclassified the MD Debugger symbol table as a DIAGNOSTIC that ships, so what
    // the release arm must now prove is the opposite — the appendix is THERE, with
    // the deb2 magic exactly at EndOfRom and a non-trivial size. That is the real
    // regression risk today: a release build that silently drops the symbol table
    // still boots and still crashes correctly, it just prints `<unknown>` for every
    // line of the crash screen a player would report. The no-appendix bar this
    // replaces now belongs to the `lean` shape (native_offcanonical_full).
    let appendix = full.len() - eor;
    assert_eq!(&full[eor..eor + 2], &native::DEB2_MAGIC, "{shape}: deb2 magic at EndOfRom");
    assert!(
        appendix >= 0x2000,
        "{shape}: appendix {appendix:#x} too small — has the deb2 symbol table been \
         dropped or collapsed? Both canonical shapes must ship it."
    );

    // ASSEMBLED PREFIX == asl (header-neutral): the full file's `[0, EndOfRom)`
    // matches the asl reference modulo the checksum ($18E) and ROM-end ($1A4) fields
    // — the sigil-canonical appendix has a different size than asl's, so those two
    // header fields legitimately differ, but every other assembled byte is identical.
    let asl_prefix = &refrom[..eor];
    let sig_prefix = &full[..eor];
    let is_header_field = |i: usize| {
        sigil_harness::CHECKSUM_FIELD_RANGE.contains(&i) || sigil_harness::ROM_END_FIELD_RANGE.contains(&i)
    };
    let bad: Vec<usize> = (0..eor)
        .filter(|&i| sig_prefix[i] != asl_prefix[i] && !is_header_field(i))
        .collect();
    assert!(
        bad.is_empty(),
        "{shape}: assembled prefix diverges from asl at {} offset(s); first {:#x} (sig {:#04x} != asl {:#04x})",
        bad.len(),
        bad.first().copied().unwrap_or(0),
        bad.first().map(|&i| sig_prefix[i]).unwrap_or(0),
        bad.first().map(|&i| asl_prefix[i]).unwrap_or(0),
    );

    // (c) FUNCTIONAL RESOLVE — load-bearing symbols → exact addresses via convsym.
    let resolved = native::convsym_resolve(&aeon, &listing).unwrap_or_else(|e| panic!("{e}"));
    for (name, plain, dbg) in LOAD_BEARING {
        let want = if debug { *dbg } else { *plain };
        let got = resolved
            .get(*name)
            .unwrap_or_else(|| panic!("{shape}: load-bearing symbol `{name}` absent from convsym output"));
        assert_eq!(*got, want, "{shape}: `{name}` resolved to {got:#X}, expected {want:#X}");
    }

    // FULL-FILE golden (sigil-canonical, CRC-pinned).
    let crc = native::crc32(&full);
    let (want_crc, want_len) = expect;
    eprintln!(
        "S1.4 {shape}: assembled={eor:#x} full={} appendix={appendix:#x} syms={} crc={crc:08x}",
        full.len(),
        listing.len()
    );
    assert_eq!(full.len(), want_len, "{shape}: full-file size (re-freeze the golden?)");
    assert_eq!(crc, want_crc, "{shape}: full-file CRC (re-freeze the golden?)");
}

/// (PLAIN) the split-golden full-file gate.
#[test]
fn native_full_sonic4_plain() {
    run_shape(false, "s4.bin", "s4");
}

/// (DEBUG) the split-golden full-file gate.
#[test]
fn native_full_sonic4_debug() {
    run_shape(true, "s4.debug.bin", "s4_debug");
}

/// t24 NEGATIVE CONTROL: doctoring one symbol's address in the listing MUST change
/// what the real convsym consumer resolves it to — proving the spot-check reads the
/// actual packed table, not a cached/hardcoded value. AND an empty listing MUST be
/// rejected (the assert-PRESENCE size band catches a collapsed/absent appendix).
#[test]
fn deb2_appendix_negative_controls() {
    if read_ref("s4.bin").is_none() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let native::RomBuild { mut listing, .. } =
        native::build_native_rom_with_listing(&aeon, false).unwrap_or_else(|e| panic!("{e}"));

    // Undoctored: BusError (the error_handler island head) resolves to its known
    // plain-shape address, pin-sourced. The probe is BusError again, not ReleaseFault:
    // since the crash-report ruling this PLAIN build carries the island, and
    // ReleaseFault is absent from it entirely (it is the lean shape's handler).
    let base = native::convsym_resolve(&aeon, &listing).unwrap();
    assert_eq!(
        base.get("BusError"),
        Some(&pins::ERROR_HANDLER.plain_base),
        "control: undoctored BusError"
    );

    // DOCTOR: move BusError to a bogus in-range address.
    for s in listing.iter_mut() {
        if s.name == "BusError" {
            s.value = 0x00BEEF;
        }
    }
    let doctored = native::convsym_resolve(&aeon, &listing).unwrap();
    assert_eq!(
        doctored.get("BusError"),
        Some(&0x00BEEF),
        "t24: convsym must reflect the doctored address (else the spot-check is vacuous)"
    );

    // A COLLAPSED listing (a handful of symbols) yields a sub-band appendix (~0x27 B)
    // → the size-band presence control HARD-errors, so a silently-appendix-less or
    // truncated ROM can never pass. Exercises the band directly.
    let dummy = vec![0u8; pins::ASSEMBLED_LEN];
    let tiny: Vec<sigil_link::ListingSymbol> = (0..3)
        .map(|i| sigil_link::ListingSymbol {
            name: format!("Sym{i}"),
            value: 0x1000 + i,
            is_equate: false,
            unused: false,
        })
        .collect();
    let err = native::append_deb2_appendix(&aeon, &dummy, &tiny, false, native::SONIC4_APPENDIX_FLOOR)
        .expect_err("collapsed listing must be rejected by the size-band presence control");
    assert!(
        err.contains("appendix size") && err.contains("band"),
        "t24: collapsed-listing rejection should name the size band; got: {err}"
    );

    // An EMPTY listing is ALSO rejected — convsym itself refuses (`No symbols passed`),
    // so the silent-failure path (`2>/dev/null || true` producing an appendix-less
    // ROM) surfaces as a HARD error here, never a pass.
    let empty: Vec<sigil_link::ListingSymbol> = Vec::new();
    native::append_deb2_appendix(&aeon, &dummy, &empty, false, native::SONIC4_APPENDIX_FLOOR)
        .expect_err("empty listing must be rejected (convsym aborts on zero symbols)");
}
