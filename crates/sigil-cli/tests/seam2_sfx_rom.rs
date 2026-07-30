//! seam-2 stage-2d Option Y — THE WHOLE-ROM SFX BINCLUDE GATE (the real-build
//! path, both shapes). The dual proof for the SFX `.asm` deletion (row-5 sfx arm +
//! the win-tab): assemble the REAL `games/sonic4/main.asm` with `SIGIL_EMP_DAC` +
//! `SIGIL_EMP_MT` + `SIGIL_EMP_SFX` on and NO body stubs, so the DAC banks/head,
//! the Moving-Trucks bank, the SFX block (`sfx_bank{,_debug}.bin` @ $5BAE8/$5D53A)
//! AND the SfxBlobWinTab head (`sfx_blob_win_tab{,_debug}.bin` @ VMA $845F) all
//! BINCLUDE — exactly as `build.sh` assembles once the 20 SFX `.asm` are deleted.
//! The whole ROM must equal the assembled reference BOTH shapes.
//!
//! Both SFX halves are SHAPE-DEPENDENT (the block sits after the shape-dependent
//! songs; the `SfxTable` pointer cells and every win-tab `winptr(Sfx_NN)` shift
//! with `__DEBUG__`), so there are per-shape `.bin` artifacts. The region proofs
//! live in `sfx_port` (the body `.emp`) + `seam2_sfx_head_colink` (the co-linked
//! head); THIS gate proves the BINCLUDE path (the real build) is byte-identical.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_sfx_rom
//! ```

use sigil_harness::{assemble_seam2_sfx_rom_as_side, assert_rom_matches_convsym, pins, seam1, seam2};
use std::path::{Path, PathBuf};

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

static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Emit every generated BINCLUDE input the DAC + MT + SFX + blob arms read.
fn ensure_generated(aeon: &Path) {
    let gen = aeon.join("engine/sound/generated");
    seam1::emit_sound_blob(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_blob (blob): {e}"));
    seam2::emit_dac_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_dac_artifacts: {e}"));
    seam2::emit_mt_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_mt_artifacts: {e}"));
    seam2::emit_sfx_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sfx_artifacts: {e}"));
    seam2::emit_seq_opcode_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_seq_opcode_artifacts: {e}"));
    seam2::emit_sound_tables_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_tables_artifacts: {e}"));
    seam2::emit_pitchtable_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_pitchtable_artifacts: {e}"));
}

fn build_seam2_sfx_rom(debug: bool) -> Vec<u8> {
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    ensure_generated(&aeon);
    let module = assemble_seam2_sfx_rom_as_side(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (seam2 SFX BINCLUDE): {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link (seam2 SFX BINCLUDE): {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap())
        .unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (seam2 SFX BINCLUDE): {e}"))
}

/// (PLAIN) the DAC+MT+SFX-BINCLUDE build == the canonical assembled `s4.bin`.
#[test]
fn seam2_sfx_rom_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let rom = build_seam2_sfx_rom(false);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::ASSEMBLED_LEN,
        "seam2 SFX BINCLUDE (plain) vs s4.bin (assembled-ROM bar)",
    );
}

/// (DEBUG) the DAC+MT+SFX-BINCLUDE build == the canonical assembled `s4.debug.bin`.
#[test]
fn seam2_sfx_rom_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let rom = build_seam2_sfx_rom(true);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::DEBUG_ASSEMBLED_LEN,
        "seam2 SFX BINCLUDE (debug) vs s4.debug.bin (assembled-ROM bar)",
    );
}

/// t24 WHOLE-ROM positive control: a DOCTORED SfxBlobWinTab head (one byte
/// flipped) must make the ROM DIVERGE from canonical in the head window ($5845F) —
/// the co-linked head gate is not vacuous. Restores the honest inputs afterward.
#[test]
fn seam2_sfx_rom_diverges_when_head_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let gen = aeon.join("engine/sound/generated");
    ensure_generated(&aeon);
    let head_path = gen.join("sfx_blob_win_tab.bin");
    let mut head = std::fs::read(&head_path).expect("read honest sfx_blob_win_tab");
    head[0] ^= 0xFF; // flip the first window-pointer's low byte
    std::fs::write(&head_path, &head).expect("write doctored head");

    let module = assemble_seam2_sfx_rom_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    ensure_generated(&aeon); // restore honest inputs

    let lo = seam2::SFX_WIN_TAB_LMA as usize;
    assert_ne!(
        &rom[lo..lo + 0x10],
        &refrom[lo..lo + 0x10],
        "the whole-ROM SFX gate is vacuous if a doctored win-tab head still matches canonical"
    );
}

/// t24 WHOLE-ROM positive control for the seam-2 stage-3 seq-opcode head: a
/// DOCTORED seq_opcode_tab.bin (one byte flipped) must make the ROM DIVERGE from
/// canonical in the SeqOpcodeTable window ($5856D). Restores honest inputs after.
#[test]
fn seam2_sfx_rom_diverges_when_seq_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let gen = aeon.join("engine/sound/generated");
    ensure_generated(&aeon);
    let seq_path = gen.join("seq_opcode_tab.bin");
    let mut seq = std::fs::read(&seq_path).expect("read honest seq_opcode_tab");
    seq[0] ^= 0xFF; // flip the first handler pointer's low byte
    std::fs::write(&seq_path, &seq).expect("write doctored seq");

    let module = assemble_seam2_sfx_rom_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    ensure_generated(&aeon); // restore honest inputs

    let lo = seam2::SEQ_OPCODE_TAB_LMA as usize;
    assert_ne!(
        &rom[lo..lo + 0x10],
        &refrom[lo..lo + 0x10],
        "the whole-ROM gate is vacuous if a doctored seq-opcode head still matches canonical"
    );
}

/// t24 WHOLE-ROM positive control for the seam-2 stage-3 sound-tables head: a
/// DOCTORED sound_tables_z80.bin (one byte flipped) must make the ROM DIVERGE from
/// canonical in the FmPitchTableZ window ($58000). Restores honest inputs after.
#[test]
fn seam2_sfx_rom_diverges_when_soundtables_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let gen = aeon.join("engine/sound/generated");
    ensure_generated(&aeon);
    let tab_path = gen.join("sound_tables_z80.bin");
    let mut tab = std::fs::read(&tab_path).expect("read honest sound_tables_z80");
    tab[0] ^= 0xFF; // flip FmPitchTableZ[0]'s low byte
    std::fs::write(&tab_path, &tab).expect("write doctored sound_tables");

    let module = assemble_seam2_sfx_rom_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    ensure_generated(&aeon); // restore honest inputs

    let lo = seam2::SOUND_TABLES_Z80_LMA as usize;
    assert_ne!(
        &rom[lo..lo + 0x10],
        &refrom[lo..lo + 0x10],
        "the whole-ROM gate is vacuous if a doctored sound-tables head still matches canonical"
    );
}
