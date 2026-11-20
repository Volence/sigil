// MEASUREMENT INSTRUMENT, not shipped code. Drives the AS front end (and, for a
// sound-off shape only, the whole chained ROM build) for each shipped aeon shape
// and reports wall-clock time. The pass COUNT comes from SIGIL_CENSUS_BUDGET
// lines on stderr, counted by the caller.
//
// READ-ONLY against AEON_DIR by construction: a described shape's build (and its
// residual AS side) writes its generated files into its own build directory outside
// the tree (G0b Task 6), so WHOLE=1 runs every shape. SHAPE names `<game> <row>`.
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let aeon = PathBuf::from(std::env::var("AEON_DIR").expect("AEON_DIR"));
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let only = std::env::var("SHAPE").unwrap_or_default();
    let whole = std::env::var("WHOLE").is_ok();
    for shape in sigil_harness::shapes::shipped(&aeon).expect("shipped shapes") {
        let name = shape.name();
        if !only.is_empty() && name != only {
            continue;
        }
        for r in 0..reps {
            let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
            let load = load.split_whitespace().take(3).collect::<Vec<_>>().join(",");
            let t = Instant::now();
            let what = if whole {
                match sigil_harness::shapes::image(&aeon, &shape) {
                    Ok(rom) => format!("ok rom_bytes={}", rom.len()),
                    Err(e) => format!("ERR {}", &e[..e.len().min(160)]),
                }
            } else {
                match sigil_harness::shapes::as_side(&aeon, &shape) {
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
