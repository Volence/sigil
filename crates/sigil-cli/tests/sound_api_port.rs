//! Tranche 5 — the REAL `sound_api.emp` port, region-level byte gate.
//!
//! `game_loop_port.rs`'s sibling for the EIGHTH code port (tranche-5 #2):
//! compiles the ACTUAL ported file from aeon's tree —
//! `engine/sound/sound_api.emp` — through the production pipeline and asserts
//! the `sound_api` section's flattened bytes equal the reference ROM window
//! at the pinned addresses, in BOTH build shapes.
//!
//! ## What this port exercises that the prior seven did not
//!
//! - **Sum-of-externs absolute EAs** — `(SND_Z80_BASE+SND_STAT_ALIVE).l` and
//!   eleven siblings: pinned-`.l` EAs whose addresses are ARITHMETIC over
//!   AS-side equs, kept AS-OWNED deliberately (the `SND_MUSIC_PARAM_*` block
//!   derives from a Z80-driver RAM label — `Snd_SpindashRev + 1` — and floats
//!   with every driver resize; a comptime mirror would churn). Only
//!   IMMEDIATE-position constants are mirrored (7 consts + 7 drift guards —
//!   kill-list row 10).
//! - **The stopZ80/startZ80 macro expansions spelled inline** — four holder
//!   procs each carry their own `.wait_z80` poll loop (proc-local hygiene
//!   keeps the four names from colliding).
//! - **The R3 imm32 deferral flipping direction** — `movea.l #SongTable`/
//!   `#SongPatchTable` were the deferral's original motivating sites (their
//!   symbols are native labels in `games.sonic4.mt_bank_blob`); here the
//!   REFERENCING side goes .emp too, so the mixed gate exercises
//!   .emp-defines/.emp-consumes through the shared link. (This isolated test
//!   supplies them as synthetic AS labels — the mixed gates prove the
//!   .emp↔.emp direction.)
//! - **`sr` save/mask sequences and `movem` contract spelling**
//!   (`preserves(d1/a0)` on Sound_PlaySFX — the hblank precedent).
//!
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Plain: the `s4.bin` window at `pins::SOUND_API`'s plain base/len.
//! Debug: the `s4.debug.bin` window at `pins::SOUND_API`'s debug base/len (—
//! per-shape as of retro-fix batch 2; the debug song-id + SFX-ring asserts).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, or
//! `EMPYREAN_SUITE_ROOT`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sound_api_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

/// Harness-private LMA for the synthetic outbound consumer (the `bsr.w Sound_PlaySFX`
/// bare-name proof).
///
/// DERIVED, not hand-typed. It has to sit clear of the `sound_api` region in BOTH shapes
/// while staying inside `bsr.w`'s ±32K of `Sound_PlaySFX`, and a hand-typed literal has
/// now gone stale three times as the debug region grew underneath it — $8000 originally,
/// bumped to $9000 by the t19 bg_anim debug assert, to $A000 by art-streaming Task 6
/// (chain 63), and effects-p2 slid the debug region to [$9D30,$A182) which swallowed
/// $A000 again. Each bump cost a confusing overlap failure, so derive it: the debug
/// region's end rounded up to the next 4K boundary is always clear and always near.
///
/// Reach is NOT self-evident from this function and is asserted explicitly at the use
/// site. Do not lean on the displacement equality there for it: that assert truncates
/// both sides with `as i16`, so it cannot detect an out-of-range displacement.
fn consumer_lma() -> u32 {
    let debug_end = pins::SOUND_API.debug_base + pins::SOUND_API.debug_len as u32;
    (debug_end + 0xFFF) & !0xFFF
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// Per-shape gate geometry (sourced from `sigil_harness::pins` — regenerate
/// via repin). The constants (equ values) are SHAPE-INVARIANT — including the
/// `SND_MUSIC_PARAM_*` block (the Z80 driver's RAM layout is identical in
/// both shapes; only 68k-side placement moves).
struct Shape {
    base: u32,
    /// Region byte length — PER-SHAPE as of retro-fix batch 2 (the DEBUG
    /// song-id + SFX-ring asserts, findings 1/2, grow the debug region).
    len: usize,
    /// Whether to lower with `DEBUG == 1` — findings 1/2's asserts are
    /// DEBUG-shape-only, so DEBUG must be bound per shape.
    debug: bool,
    /// `Sound_PlaySFX`'s offset inside the region — per-shape as of batch 2
    /// (PlayMusic's two song-id asserts precede it in the debug shape).
    play_sfx_off: usize,
    ring_sfx_speaker: u32,
    sfx_ring_buf: u32,
    sfx_ring_wr: u32,
    sfx_ring_rd: u32,
    song_table: u32,
    song_patch_table: u32,
}

const PLAIN: Shape = Shape {
    base: pins::SOUND_API.plain_base,
    len: pins::SOUND_API.plain_len,
    debug: false,
    play_sfx_off: pins::SOUND_PLAY_SFX_OFF.plain,
    ring_sfx_speaker: pins::RING_SFX_SPEAKER.plain,
    sfx_ring_buf: pins::SFX_RING_BUF.plain,
    sfx_ring_wr: pins::SFX_RING_WR.plain,
    sfx_ring_rd: pins::SFX_RING_RD.plain,
    song_table: pins::SONG_TABLE.plain,
    song_patch_table: pins::SONG_PATCH_TABLE.plain,
};
const DEBUG: Shape = Shape {
    base: pins::SOUND_API.debug_base,
    len: pins::SOUND_API.debug_len,
    debug: true,
    play_sfx_off: pins::SOUND_PLAY_SFX_OFF.debug,
    ring_sfx_speaker: pins::RING_SFX_SPEAKER.debug,
    sfx_ring_buf: pins::SFX_RING_BUF.debug,
    sfx_ring_wr: pins::SFX_RING_WR.debug,
    sfx_ring_rd: pins::SFX_RING_RD.debug,
    song_table: pins::SONG_TABLE.debug,
    song_patch_table: pins::SONG_PATCH_TABLE.debug,
};

/// The AS-side constants the .emp still reads through the link: the z80_bus
/// template's bus register, the 2 typed-mirror SfxId drift-guard truths
/// (config/sound_ids.asm), and #SONG_COUNT. The SND_* sound contract (slot
/// addresses, immediate values, the MUSIC_PARAM RAM block) is authored in
/// engine/sound/sound_constants.emp now (prepended in compile_real_file), so it
/// folds at comptime — no AS equ seam. A trailing label+`dc.w` opens a section so
/// the equs flush via `pending_equ_syms` (the collision_lookup pattern).
fn as_constant_equs() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Z80_BUS_REQUEST = $A11100\n\
               SFXID_RING_RIGHT = $33\n\
               SFXID_RING_LEFT = $34\n\
               SONG_COUNT = 3\n\
               Stub:\n\
               \tdc.w 0\n";
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (constant equs): {d:?}")).sections
}

/// One synthetic AS-side label phased at `vma` (carrier LMA harness-private,
/// set by the caller).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// Compile the real `engine/sound/sound_api.emp` pinned at the shape's base
/// with all cross-seam symbols supplied synthetically at their true
/// per-shape positions. Returns (resolved, linked, link_asserts).
fn compile_real_file(
    shape: &Shape,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = aeon_dir().join("engine/sound");
    let src = std::fs::read_to_string(dir.join("sound_api.emp"))
        .unwrap_or_else(|e| panic!("cannot read sound_api.emp: {e}"));
    let (main, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_api.emp parse errors: {pdiags:?}"
    );
    // Prepend the shared engine.z80_bus templates (stop_z80/start_z80 moved
    // there at the t19 step-6 sweep).
    let z80_src = std::fs::read_to_string(aeon_dir().join("engine/z80_bus.emp"))
        .unwrap_or_else(|e| panic!("cannot read z80_bus.emp: {e}"));
    let (z80_file, zdiags) = parse_str(&z80_src);
    assert!(
        zdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "z80_bus.emp parse errors: {zdiags:?}"
    );
    // + the engine.irq bracket template (sr_masked adopted at the t21 step-6
    // sweep for the two paired label-free SR brackets).
    let irq_src = std::fs::read_to_string(aeon_dir().join("engine/irq.emp"))
        .unwrap_or_else(|e| panic!("cannot read irq.emp: {e}"));
    let (irq_file, idiags) = parse_str(&irq_src);
    assert!(
        idiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "irq.emp parse errors: {idiags:?}"
    );
    // sound_api.emp `use engine.sound_constants.*` for the slot addresses +
    // immediate command values; prepend the authority so they fold in this
    // standalone lower (its MUSIC_PARAM RAM block derives from the Z80-driver
    // layout — the same derivation the reference ROM baked).
    let snd_src = std::fs::read_to_string(dir.join("sound_constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read sound_constants.emp: {e}"));
    let (snd_file, sdiags) = parse_str(&snd_src);
    assert!(
        sdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_constants.emp parse errors: {sdiags:?}"
    );
    // + engine.types (sound_api `use`s SongId/SfxId — the `extern SFXID_RING_*:
    // SfxId` typed references resolve their newtype here; a pure-types module,
    // zero bytes, so prepending it is region-neutral).
    let types_src = std::fs::read_to_string(aeon_dir().join("engine/system/types.emp"))
        .unwrap_or_else(|e| panic!("cannot read types.emp: {e}"));
    let (types_file, tdiags) = parse_str(&types_src);
    assert!(
        tdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "types.emp parse errors: {tdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: types_file
            .items
            .into_iter()
            .chain(snd_file.items)
            .chain(z80_file.items)
            .chain(irq_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };

    // findings 1/2's asserts are DEBUG-shape-only: DEBUG must always be DEFINED
    // (house convention — the debug shape is explicit), 0 in plain (elides the
    // asserts) / 1 in debug (expands them). #SONG_COUNT resolves through the
    // synthetic AS equ seam (as_constant_equs). Z80_RAM (engine.constants) is the
    // base of SND_Z80_BASE — the auto-glob provides it in the real build.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![
            ("DEBUG".to_string(), i128::from(shape.debug)),
            ("Z80_RAM".to_string(), 0xA0_0000),
        ],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

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
         name = \"sound_api\"\n\
         lma_base = {:#x}\n\
         size = {:#x}\n\
         kind = \"rom\"\n",
        shape.base, shape.len
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = as_constant_equs();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    let mut lma = 0x0200_0000u32;
    // findings 1/2's DEBUG asserts (Sound_PlayMusic bounds, Sound_PlaySFX
    // ring-full raise_error) reference the debugger.asm error-handler tail —
    // supply those two symbols at their true debug VMAs in the debug shape only.
    let mut labels: Vec<(&str, u32)> = vec![
        ("Ring_Sfx_Speaker", shape.ring_sfx_speaker),
        ("Sfx_Ring_Buf", shape.sfx_ring_buf),
        ("Sfx_Ring_Wr", shape.sfx_ring_wr),
        ("Sfx_Ring_Rd", shape.sfx_ring_rd),
        ("SongTable", shape.song_table),
        ("SongPatchTable", shape.song_patch_table),
    ];
    if shape.debug {
        labels.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        labels.push((
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
    }
    for (name, vma) in labels {
        let mut secs = as_label_at(name, vma);
        for sec in &mut secs {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(secs);
        lma += 0x10_0000;
    }

    // Outbound bare-name proof: a real caller's `bsr.w Sound_PlaySFX`. The
    // consumer is PHASED at `consumer_lma()` — inside bsr.w's ±32K of both shapes'
    // targets, so the asserted displacement is a real reachable one (an
    // unphased far carrier would only "pass" mod 2^16, which is what port #1's review
    // caught. NOTE the old text here claimed sigil-link has no pc-rel16 range check —
    // that is STALE: `bsr.w` lowers to FixupKind::PcRelDisp16 and sigil-link DOES range
    // check it and errors, so link() fails loudly. The explicit reach assert at the use
    // site is belt-and-braces, and exists because the displacement EQUALITY cannot see
    // range at all.)
    let lma = consumer_lma();
    let asm = format!(
        "cpu 68000\n\
         phase ${lma:X}\n\
         Consumer:\n\
         \tbsr.w   Sound_PlaySFX\n\
         \trts\n"
    );
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    let mut consumer = assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}"))
        .sections;
    for sec in &mut consumer {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// sound_api carries NO immediate-mirror drift guards. The 5 untyped mirrors were
/// retired to bare `#extern(...)` link names at the flip Stage-0 touch (kill-list
/// row 10); the last two — the typed `SfxId` collect-ring ids — became `extern
/// SFXID_RING_*: SfxId` typed externs at L8, so the value crosses the seam once
/// from the game authority with no local copy to drift. A drift guard is a
/// cross-check between two copies; with one authority there is nothing to guard,
/// so the capture count is zero. (The single-authority property that replaces the
/// runtime guard — a missing authority is loud, not a stale fallback — is proven by
/// `typed_extern_has_no_mirror_so_a_missing_authority_is_loud` in
/// tranche5_negative_probes.)
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    assert_eq!(guards, 0, "sound_api holds no immediate-mirror drift guards (all values are single-authority)");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_api's surviving link asserts must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus context on each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES, a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base. Tolerate a short (< 16 B)
    // all-zero tail beyond the lowered image; every real byte still compares.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 16
        && expected[candidate.len()..].iter().all(|&b| b == 0)
    {
        &expected[..candidate.len()]
    } else {
        expected
    };
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch, candidate {} bytes, expected {} bytes",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at offset {i:#x} (region-relative)\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(shape);
    assert_drift_guards(&resolved, &link_asserts);

    let lo = shape.base as usize;
    let expected = &refrom[lo..lo + shape.len];
    let section = linked.section("sound_api").expect("linked image must carry sound_api");
    assert_region_matches(
        &section.bytes,
        expected,
        &format!("sound_api vs {rom_name}[{lo:#x}..{:#x}]", lo + shape.len),
    );

    // Outbound proof: `bsr.w Sound_PlaySFX` resolves to base + SOUND_PLAY_SFX_OFF
    // (Sound_PlaySFX's offset inside the block — invariant, the
    // block only slid -36 wholesale in the tranche-7b interact fix).
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma())
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let target = shape.base as i64 + shape.play_sfx_off as i64;
    let true_disp = target - (consumer.lma as i64 + 2);
    // REACH, checked on the UNTRUNCATED value and therefore separately from the equality
    // below. The `as i16` in the comparison truncates BOTH sides, so if the displacement
    // ever left `bsr.w` range the equality would still hold and the proof would go
    // vacuous — it cannot police its own reach. `consumer_lma()` is anchored to the DEBUG
    // region's end, which is the far side from the PLAIN target, so plain sets the margin:
    // 0x383E of 0x7FFF today, and SOUND_API.debug_base moved +0x570 in this one parcel.
    // Without this line the eventual failure would surface as a bare
    // "(d16,PC) displacement out of range" naming `bsr.w`, nowhere near sound_api.
    assert!(
        (-0x8000..=0x7FFF).contains(&true_disp),
        "`consumer_lma()` ({:#x}) has drifted out of `bsr.w` reach of Sound_PlaySFX \
         ({target:#x}): displacement {true_disp:#x}. Re-anchor the harness-private LMA \
         (below BOTH shapes' bases, or nearer the target).",
        consumer_lma()
    );
    let expected_disp = true_disp as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w Sound_PlaySFX` must resolve to {target:#x}"
    );
}

/// (plain) `sound_api` bytes == the pinned plain window.
#[test]
fn sound_api_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) `sound_api` bytes == the pinned debug window.
#[test]
fn sound_api_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}
