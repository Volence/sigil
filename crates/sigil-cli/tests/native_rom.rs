//! Flip Stage 1 · S1.1 — THE ALL-GATES-ON NATIVE WHOLE-ROM GATE.
//!
//! Assemble `main.asm` with every `SIGIL_EMP_*` code gate ON, natively place every
//! ported `.emp` module at its `pins`-region base, and prove the whole ROM equals
//! the live `asl` `s4.bin` / `s4.debug.bin` (the assembled-ROM bar — the deb2
//! appendix full-file bar is S1.4). This is the first time all 53 gates flip at
//! once through one registry; the 14 first-time real-image placements (BG,
//! BG_ANIM, CAMERA, CORE, DPLC, ENTITY_WINDOW, LOAD_OBJECT, OBJDEFS, PARALLAX,
//! PLANE_BUFFER, SECTION, SPRITES, TILE_CACHE, VECTORS) enter the real image here.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_rom
//! ```

use sigil_harness::{assert_rom_matches_convsym, native, pins};
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

// The whole-ROM native build touches the shared `engine/sound/generated` dir
// (ensure_generated) — serialize the two shapes so they never race on it.
static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// (PLAIN) the all-gates-ON native ROM == the canonical assembled `s4.bin`.
///
/// The bar is the CONVSYM variant, not the exact-identity release variant, because
/// the crash-report ruling (owner-ruled 2026-08-04) put the deb2 symbol appendix back
/// into the shipped release ROM: `s4.bin` is now assembled-image + appendix, so the
/// two semantic header fields convsym rewrites (`$18E` checksum, `$1A4` ROM-end
/// pointer) legitimately differ from the pre-append image this test builds. What is
/// asserted is unchanged in strength — `assert_rom_matches_convsym` DERIVES that
/// allowlist from the actual diff and then requires the FULL diff set to be confined
/// to those two fields, and additionally requires each of them to genuinely differ
/// (so a reference silently rebuilt without the appendix fails here). The
/// no-appendix-past-EndOfRom bar it replaces now belongs to the `lean` shape alone.
#[test]
fn native_rom_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = native::build_native_rom(&aeon_dir(), false).unwrap_or_else(|e| panic!("{e}"));
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::ASSEMBLED_LEN,
        "native all-gates (plain) vs s4.bin (assembled-ROM bar)",
    );
}

/// (DEBUG) the all-gates-ON native `__DEBUG__` ROM == the canonical `s4.debug.bin`.
#[test]
fn native_rom_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = native::build_native_rom(&aeon_dir(), true).unwrap_or_else(|e| panic!("{e}"));
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::DEBUG_ASSEMBLED_LEN,
        "native all-gates (debug) vs s4.debug.bin (assembled-ROM bar)",
    );
}
