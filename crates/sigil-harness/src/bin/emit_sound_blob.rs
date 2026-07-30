//! `emit_sound_blob` — seam-1 (Option A): emit the canonical resident-sound-blob
//! build inputs that asl packs after the five `.asm` twins are deleted.
//!
//! Writes to `<out-dir>`:
//!   * `z80_sound_blob.bin`       — the plain-shape native-linked blob ($181C B)
//!   * `z80_sound_blob_debug.bin` — the debug-shape blob ($189A B = +$7E)
//!   * `z80_sound_syms.asm`       — the exported-symbol CONTRACT (per-shape equs:
//!                                   the sequencer opcode-handler VMAs the banked
//!                                   seq_opcode_tab.asm references)
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

    match sigil_harness::seam1::emit_sound_blob(Path::new(&aeon), Path::new(&out_dir)) {
        Ok(()) => {
            println!(
                "emitted seam-1 sound blob: z80_sound_blob{{,_debug}}.bin + z80_sound_syms.asm -> {out_dir}"
            );
        }
        Err(err) => {
            eprintln!("error: emit_sound_blob failed: {err}");
            process::exit(1);
        }
    }
}
