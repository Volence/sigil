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
//!   symbols are .emp-side under `SIGIL_EMP_MT` in the mixed build); here the
//!   REFERENCING side goes .emp too, so the mixed gate exercises
//!   .emp-defines/.emp-consumes through the shared link. (This isolated test
//!   supplies them as synthetic AS labels — the mixed gates prove the
//!   .emp↔.emp direction.)
//! - **`sr` save/mask sequences and `movem` contract spelling**
//!   (`preserves(d1/a0)` on Sound_PlaySFX — the hblank precedent).
//!
//! ## Reference windows
//!
//! Plain (map base `$5D94`): `s4.bin[0x5D94..0x5F7C]` (0x1E8 bytes).
//! Debug (map base `$7252`): `s4.debug.bin[0x7252..0x743A]` (0x1E8 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sound_api_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Per-shape gate geometry (listing symbol tables, 2026-07-10 pins). The
/// constants (equ values) are SHAPE-INVARIANT — including the
/// `SND_MUSIC_PARAM_*` block (the Z80 driver's RAM layout is identical in
/// both shapes; only 68k-side placement moves).
struct Shape {
    base: u32,
    ring_sfx_speaker: u32,
    sfx_ring_buf: u32,
    sfx_ring_wr: u32,
    sfx_ring_rd: u32,
    song_table: u32,
    song_patch_table: u32,
}

const PLAIN: Shape = Shape {
    base: 0x5D94,
    ring_sfx_speaker: 0xFFFF_AF30,
    sfx_ring_buf: 0xFFFF_AF32,
    sfx_ring_wr: 0xFFFF_AF3A,
    sfx_ring_rd: 0xFFFF_AF3B,
    song_table: 0x63AE0,
    song_patch_table: 0x63AE4,
};
const DEBUG: Shape = Shape {
    base: 0x7252,
    ring_sfx_speaker: 0xFFFF_AF52,
    sfx_ring_buf: 0xFFFF_AF54,
    sfx_ring_wr: 0xFFFF_AF5C,
    sfx_ring_rd: 0xFFFF_AF5D,
    song_table: 0x65522,
    song_patch_table: 0x6552E,
};
const REGION_LEN: usize = 0x1E8;

/// The AS-side constants the .emp reads through the link: the EA-position
/// equs (slot addresses — deliberately NOT mirrored) and the 7 drift-guard
/// truths (immediate-position mirrors). A trailing label+`dc.w` opens a
/// section so the equs flush via `pending_equ_syms` (the collision_lookup
/// pattern).
fn as_constant_equs() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Z80_BUS_REQUEST = $A11100\n\
               SND_Z80_BASE = $A00000\n\
               SND_STAT_ALIVE = $1F10\n\
               SND_REQ_PING = $1F00\n\
               SND_REQ_SAMPLE = $1F01\n\
               SND_REQ_MUSIC = $1F02\n\
               SND_REQ_SFX = $1F03\n\
               SND_REQ_FADE = $1F05\n\
               SND_REQ_TEMPO = $1F06\n\
               SND_MUSIC_PARAM_BANK = $1CA6\n\
               SND_MUSIC_PARAM_PTR = $1CA7\n\
               SND_MUSIC_PARAM_FLAGS = $1CA9\n\
               SND_MUSIC_PARAM_PATCHPTR = $1CAA\n\
               SND_ALIVE_MARKER = $5A\n\
               SND_MUSIC_STOP = $FF\n\
               SND_FADE_CMD_OUT = 1\n\
               SND_FADE_CMD_IN = 2\n\
               SFX_RING_MASK = $07\n\
               SFXID_RING_RIGHT = $33\n\
               SFXID_RING_LEFT = $34\n\
               Stub:\n\
               \tdc.w 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (constant equs): {d:?}")).sections
}

/// One synthetic AS-side label phased at `vma` (carrier LMA harness-private,
/// set by the caller).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
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
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_api.emp parse errors: {pdiags:?}"
    );

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
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
         size = 0x1E8\n\
         kind = \"rom\"\n",
        shape.base
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
    for (name, vma) in [
        ("Ring_Sfx_Speaker", shape.ring_sfx_speaker),
        ("Sfx_Ring_Buf", shape.sfx_ring_buf),
        ("Sfx_Ring_Wr", shape.sfx_ring_wr),
        ("Sfx_Ring_Rd", shape.sfx_ring_rd),
        ("SongTable", shape.song_table),
        ("SongPatchTable", shape.song_patch_table),
    ] {
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
    // consumer is PHASED at $8000 — inside bsr.w's ±32K of both shapes'
    // targets, so the asserted displacement is a real reachable one (an
    // unphased far carrier would only "pass" mod 2^16; sigil-link's missing
    // pc-rel16 range check is gap-ledgered, and port #1's review caught
    // exactly this vacuity).
    let asm = "cpu 68000\n\
               phase $8000\n\
               Consumer:\n\
               \tbsr.w   Sound_PlaySFX\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut consumer = assemble(asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}"))
        .sections;
    for sec in &mut consumer {
        sec.lma = 0x8000;
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

/// The 7 immediate-mirror drift guards must be captured and PASS against
/// `as_constant_equs`' truths.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = link_asserts
        .iter()
        .filter(|a| {
            !a.message.iter().any(|p| {
                matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("[layout.odd-item]"))
            })
        })
        .count();
    assert_eq!(guards, 7, "sound_api's seven drift guards must be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_api's drift guards must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus context on each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes",
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
    let expected = &refrom[lo..lo + REGION_LEN];
    let section = linked.section("sound_api").expect("linked image must carry sound_api");
    assert_region_matches(
        &section.bytes,
        expected,
        &format!("sound_api vs {rom_name}[{lo:#x}..{:#x}]", lo + REGION_LEN),
    );

    // Outbound proof: `bsr.w Sound_PlaySFX` resolves to base + 0x104
    // (Sound_PlaySFX's offset inside the block: $5E98 - $5D94).
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x8000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let target = shape.base as i64 + 0x104;
    let expected_disp = (target - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w Sound_PlaySFX` must resolve to {target:#x}"
    );
}

/// (plain) `sound_api` bytes == `s4.bin[0x5D94..0x5F7C]`.
#[test]
fn sound_api_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) `sound_api` bytes == `s4.debug.bin[0x7252..0x743A]`.
#[test]
fn sound_api_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}
