//! seam-2 stage-2b Option Y — THE WHOLE-ROM DAC BINCLUDE GATE (the real-build
//! path, both shapes). The dual proof for the coupled `dac_samples.asm` +
//! `dac_sample_tab.asm` deletion (rows 5-dac + 57): assemble the REAL
//! `games/sonic4/main.asm` with `SIGIL_EMP_DAC` on and NO body-stub, so the
//! `gameSoundDataIncludes` macro BINCLUDEs the emitted DAC banks
//! (dac_blip_bank.bin @ $48000, dac_shared_bank.bin @ $50000) and `soundBankHead`
//! BINCLUDEs the co-linked descriptor head (dac_sample_tab.bin @ VMA $85AD),
//! exactly as `build.sh` assembles once the two `.asm` twins are deleted. The
//! whole ROM must equal the assembled reference (`assert_rom_matches_convsym`).
//!
//! This is the seam-1 `mixed_seam1_rom_*` pattern applied to the DAC: sigil emits
//! the canonical build inputs, the AS front-end BINCLUDEs them, and the
//! assembled-ROM CRC is the provenance bar. The `.emp`-composition proof lives in
//! the DSM `mixed_dac_rom` tranches + the `seam2_dac_head_colink` slice gate; THIS
//! gate proves the BINCLUDE path (the real build) is byte-identical to it.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_dac_rom
//! ```

use sigil_harness::{assemble_seam2_dac_rom_as_side, assert_rom_matches_convsym, pins, seam1, seam2};
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

/// The three whole-ROM tests read/write the SAME generated-BINCLUDE path (the
/// aeon tree's `engine/sound/generated`), so they must not race under cargo's
/// in-binary parallel runner (cargo runs distinct test binaries sequentially).
static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Emit the generated BINCLUDE inputs the DAC + blob arms read: the resident sound
/// blob (seam-1) and the three seam-2 DAC artifacts. Deterministic; gitignored.
fn ensure_generated(aeon: &Path) {
    let gen = aeon.join("engine/sound/generated");
    seam1::emit_sound_blob(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_blob (blob): {e}"));
    seam2::emit_dac_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_dac_artifacts: {e}"));
    // Post MT/SFX/seq deletion the whole-ROM assemble BINCLUDEs the MT bank + the
    // SFX body + the SFX win-tab + the seq-opcode head (no more "DAC-only" build —
    // those .asm are gone), so this gate emits the full artifact set too.
    seam2::emit_mt_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_mt_artifacts: {e}"));
    seam2::emit_sfx_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sfx_artifacts: {e}"));
    seam2::emit_seq_opcode_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_seq_opcode_artifacts: {e}"));
    seam2::emit_sound_tables_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_tables_artifacts: {e}"));
    seam2::emit_pitchtable_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_pitchtable_artifacts: {e}"));
}

/// Assemble the WHOLE ROM through the AS front-end with the seam-2 DAC gate ON
/// (BINCLUDE arms) and emit the flat ROM.
fn build_seam2_dac_rom(debug: bool) -> Vec<u8> {
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    ensure_generated(&aeon);
    let module = assemble_seam2_dac_rom_as_side(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (seam2 DAC BINCLUDE): {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link (seam2 DAC BINCLUDE): {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap())
        .unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (seam2 DAC BINCLUDE): {e}"))
}

/// (PLAIN) the DAC-BINCLUDE build == the canonical assembled `s4.bin`.
#[test]
fn seam2_dac_rom_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let rom = build_seam2_dac_rom(false);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::ASSEMBLED_LEN,
        "seam2 DAC BINCLUDE (plain) vs s4.bin (assembled-ROM bar)",
    );
}

/// (DEBUG) the DAC-BINCLUDE build == the canonical assembled `s4.debug.bin`.
#[test]
fn seam2_dac_rom_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let rom = build_seam2_dac_rom(true);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::DEBUG_ASSEMBLED_LEN,
        "seam2 DAC BINCLUDE (debug) vs s4.debug.bin (assembled-ROM bar)",
    );
}

/// t24 WHOLE-ROM positive control: a DOCTORED head `.bin` (one descriptor byte
/// flipped) BINCLUDE'd into the ROM must DIVERGE from canonical at the
/// `DacSampleTable` LMA ($585AD) — the gate is not vacuous. Restores the honest
/// inputs afterward so sibling tests see the real artifacts.
#[test]
fn seam2_dac_rom_diverges_when_head_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let gen = aeon.join("engine/sound/generated");
    // Emit the honest inputs first, THEN overwrite ONLY the head with a doctored copy.
    ensure_generated(&aeon);
    let head_path = gen.join("dac_sample_tab.bin");
    let mut head = std::fs::read(&head_path).expect("read honest head");
    head[0] ^= 0xFF; // flip descriptor 0's ds_bank byte
    std::fs::write(&head_path, &head).expect("write doctored head");

    let module = assemble_seam2_dac_rom_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    // Restore the honest inputs for sibling tests / the DSM binary.
    ensure_generated(&aeon);

    let lo = seam2::DAC_SAMPLE_TAB_LMA as usize;
    let len = seam2::DAC_SAMPLE_TAB_LEN;
    assert_ne!(
        &rom[lo..lo + len],
        &refrom[lo..lo + len],
        "the whole-ROM DAC gate is vacuous if a doctored head still matches canonical"
    );
}
