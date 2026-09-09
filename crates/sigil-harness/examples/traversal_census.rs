// MEASUREMENT INSTRUMENT, not shipped code. Drives the AS front end (and, for a
// sound-off shape only, the whole chained ROM build) for each shipped aeon shape
// and reports wall-clock time. The pass COUNT comes from SIGIL_CENSUS_BUDGET
// lines on stderr, counted by the caller.
//
// READ-ONLY against AEON_DIR by construction: `assemble_as_side` writes nothing,
// and `build_rom_chained` writes only via `emit_generated`, which it calls only
// when `profile.sound_on`. WHOLE=1 is therefore refused for a sound-on shape.
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let aeon = PathBuf::from(std::env::var("AEON_DIR").expect("AEON_DIR"));
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let only = std::env::var("SHAPE").unwrap_or_default();
    let whole = std::env::var("WHOLE").is_ok();
    for (name, profile) in sigil_harness::native::shipped_shapes() {
        if !only.is_empty() && name != only {
            continue;
        }
        if whole && profile.sound_on {
            println!("SKIP\t{name}\tsound_on: a whole build would write into the reference tree");
            continue;
        }
        for r in 0..reps {
            let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
            let load = load.split_whitespace().take(3).collect::<Vec<_>>().join(",");
            let t = Instant::now();
            let what = if whole {
                match sigil_harness::native::build_rom_chained(&aeon, &profile) {
                    Ok(rom) => format!("ok rom_bytes={}", rom.len()),
                    Err(e) => format!("ERR {}", &e[..e.len().min(160)]),
                }
            } else {
                match sigil_harness::native::assemble_as_side(&aeon, &profile) {
                    Ok(a) => format!("ok sections={}", a.module.sections.len()),
                    Err(e) => format!("ERR {}", &e[..e.len().min(160)]),
                }
            };
            let dt = t.elapsed();
            let kind = if whole { "WHOLE" } else { "ASSIDE" };
            println!("{kind}\t{name}\trep={r}\tsecs={:.4}\tload={load}\t{what}", dt.as_secs_f64());
        }
    }
}
