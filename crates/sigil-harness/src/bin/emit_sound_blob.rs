//! `emit_sound_blob` — seam-1 (Option A): emit the canonical resident-sound-blob
//! build inputs that asl packs after the five `.asm` twins are deleted.
//!
//! Writes to `<out-dir>` (plus the seam-2 banked artifacts):
//!   * `z80_sound_blob.bin`       — the plain-shape native-linked blob ($181C B)
//!   * `z80_sound_blob_debug.bin` — the debug-shape blob ($189A B = +$7E)
//!
//! Byte-DETERMINISTIC from the tracked `.emp` sources + the sigil toolchain
//! version. The canonical ROM CRC is the provenance bar (the blob is tracked the
//! same way the ROMs are). This is the first hard aeon→sigil build dependency:
//! `build.sh` fails LOUDLY if this binary is missing/stale — there is no fallback,
//! the `.asm` is gone.
//!
//! ```text
//! emit_sound_blob --aeon <aeon-dir> --out-dir <dir>
//! ```

use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut aeon: Option<String> = None;
    let mut out_dir: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--aeon" => {
                i += 1;
                aeon = args.get(i).cloned();
            }
            "--out-dir" => {
                i += 1;
                out_dir = args.get(i).cloned();
            }
            other => {
                eprintln!("error: unexpected argument '{other}'");
                eprintln!("usage: emit_sound_blob --aeon <dir> --out-dir <dir>");
                process::exit(2);
            }
        }
        i += 1;
    }
    let (Some(aeon), Some(out_dir)) = (aeon, out_dir) else {
        eprintln!("usage: emit_sound_blob --aeon <dir> --out-dir <dir>");
        process::exit(2);
    };

    let aeon_path = Path::new(&aeon);
    let out_path = Path::new(&out_dir);

    // This binary is handed its reference tree on the command line — aeon's own
    // `build.sh` runs it as `--aeon . --out-dir engine/sound/generated` from the tree
    // it is building. The emitters' write precondition
    // (`seam2::require_named_reference_tree`) asks that the tree a write goes into be
    // NAMED rather than resolved from a hardcoded fallback, and `--aeon` names it, so
    // the argument is published as this process's `AEON_DIR`. One rule covers every
    // writing process; the argv caller is not an exception to it. Set before any
    // emitter runs, while the process is still single-threaded.
    if std::env::var_os("AEON_DIR").is_none() {
        std::env::set_var("AEON_DIR", aeon_path);
    }

    if let Err(err) = sigil_harness::seam1::emit_sound_blob(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (seam-1 resident blob) failed: {err}");
        process::exit(1);
    }
    // seam-2: the DAC bank bodies + the co-linked descriptor head (shape-invariant,
    // no `_debug` variant). Written alongside the resident blob for the DAC wire.
    if let Err(err) = sigil_harness::seam2::emit_dac_artifacts(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (seam-2 DAC artifacts) failed: {err}");
        process::exit(1);
    }
    // seam-2 stage-2c: the Moving-Trucks streaming bank, three-way split (shape-
    // dependent): body + SongTable + SongPatchTable, each a native embed member.
    if let Err(err) = sigil_harness::seam2::emit_mt_artifacts(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (seam-2 MT bank) failed: {err}");
        process::exit(1);
    }
    // seam-2 stage-2d: the SFX block body + the co-linked window-pointer head
    // (both shape-dependent — the SFX block sits after the shape-dependent songs).
    if let Err(err) = sigil_harness::seam2::emit_sfx_artifacts(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (seam-2 SFX bank) failed: {err}");
        process::exit(1);
    }
    // seam-2 stage-3: the sequencer opcode jump table (shape-dependent — the
    // resident Seq_Op_* handlers re-base in the debug shape).
    if let Err(err) = sigil_harness::seam2::emit_seq_opcode_artifacts(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (seam-2 seq_opcode_tab) failed: {err}");
        process::exit(1);
    }
    // seam-2 stage-3: the generated FM/PSG data tables (shape-invariant).
    if let Err(err) = sigil_harness::seam2::emit_sound_tables_artifacts(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (seam-2 sound_tables_z80) failed: {err}");
        process::exit(1);
    }
    // flip Stage-0: the SndDefaultPitchTable banked head (the last AS sound head,
    // shape-invariant).
    if let Err(err) = sigil_harness::seam2::emit_pitchtable_artifacts(aeon_path, out_path) {
        eprintln!("error: emit_sound_blob (pitchtable) failed: {err}");
        process::exit(1);
    }
    println!(
        "emitted seam-1 resident blob (z80_sound_blob{{,_debug}}.bin) \
         + seam-2 DAC artifacts (dac_blip_bank.bin + dac_shared_bank.bin + dac_sample_tab.bin) \
         + seam-2 MT bank split (mt_bank_body{{,_debug}}.bin + mt_songtable{{,_debug}}.bin + mt_songpatchtable{{,_debug}}.bin) \
         + seam-2 SFX bank (sfx_bank{{,_debug}}.bin + sfx_blob_win_tab{{,_debug}}.bin) \
         + seam-2 head (seq_opcode_tab{{,_debug}}.bin + sound_tables_z80.bin) \
         + pitchtable (movingtrucks_pitchtable.bin) -> {out_dir}"
    );
}
