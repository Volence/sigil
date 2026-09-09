// MEASUREMENT INSTRUMENT, not shipped code. Decomposes one `sigil <root.asm>`
// run into the phases the CLI actually performs, on the CLI's own route.
//
// The route mirrors `crates/sigil-cli/src/main.rs`: `assemble_root_located_warned`
// (NOT the relocating entry point, which only the aeon harness uses), then, on
// success only, `resolve_layout` / `link` / `check_image_bounds` / `flatten`. On
// a corpus root the front end returns `Err` and the CLI exits before any of the
// back half, so on that input this instrument reports the front half plus the
// diagnostic rendering and nothing else, which is exactly what the CLI spends.
//
// Phases timed separately:
//   FRONT   assemble_root_located_warned
//   RENDER  the `sources.label(d.primary)` call the CLI makes per diagnostic,
//           plus the `format!` the renderer builds, with the result consumed so
//           it cannot be optimized away
//   REREAD  a bare `fs::read_to_string` over every distinct file the run
//           spliced, once per traversal, which is the I/O and allocation half
//           of a traversal with no lexing or evaluation attached
//
// It writes nothing anywhere. LOOP=n repeats the whole front end n times in one
// process so an external sampling profiler has a long-lived subject.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Process CPU time (user + system) in seconds, from `/proc/self/stat` fields
/// 14 and 15, divided by the kernel's clock tick.
///
/// Wall-clock time on this machine is not a measurement: several lanes build on
/// it and the 1-minute load average moves between 5 and 40, which moves a
/// wall-clock figure by more than a factor of two on an unchanged subject. CPU
/// time is what the assembler spent, and it is what every ratio below is taken
/// over. Wall is reported beside it so the contamination is visible rather than
/// hidden.
fn cpu_secs() -> f64 {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    // Field 2 (comm) is parenthesised and may contain spaces; split after it.
    let after = match s.rfind(')') {
        Some(i) => &s[i + 1..],
        None => return 0.0,
    };
    let f: Vec<&str> = after.split_whitespace().collect();
    // After `comm` the next field is `state` (index 0 here), so `utime` is
    // field 14 of the full record => index 11 here, `stime` => index 12.
    let utime: f64 = f.get(11).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let stime: f64 = f.get(12).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    (utime + stime) / 100.0
}

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(",")
}

fn main() {
    let root = PathBuf::from(std::env::var("ROOT").expect("ROOT=<path to root .asm>"));
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let traversals: usize =
        std::env::var("TRAVERSALS").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    let opts = sigil_frontend_as::Options::default();
    let dir = root.parent().unwrap_or(Path::new(".")).to_path_buf();

    for r in 0..reps {
        let load = loadavg();
        let c_front = cpu_secs();
        let t_front = Instant::now();
        let res = sigil_frontend_as::assemble_root_located_warned(&root, &opts);
        let front = t_front.elapsed().as_secs_f64();
        let front_cpu = cpu_secs() - c_front;

        match res {
            Err(failure) => {
                // Exactly what `render_as_diags` does, minus the write to the
                // terminal, so the figure is the LOOKUP and FORMAT cost and not
                // the cost of a pipe.
                let c_render = cpu_secs();
                let t_render = Instant::now();
                let mut sink = 0usize;
                for d in &failure.diags {
                    match failure.sources.label(d.primary) {
                        Some(loc) => sink += format!("{loc}: {}: {}", d.level, d.message).len(),
                        None => sink += format!("{}: {}", d.level, d.message).len(),
                    }
                }
                let render = t_render.elapsed().as_secs_f64();
                let render_cpu = cpu_secs() - c_render;

                // Source-map shape: how much text one traversal put through the
                // lexer, and how many distinct files it opened.
                let n_sources = failure.sources.len();
                let mut bytes = 0usize;
                let mut names: BTreeSet<String> = BTreeSet::new();
                for i in 0..n_sources {
                    let id = sigil_span::SourceId(i as u32);
                    bytes += failure.sources.text(id).len();
                    let n = failure.sources.name(id);
                    if !n.is_empty() {
                        names.insert(n.to_string());
                    }
                }

                // The I/O-and-allocation half of a traversal, priced on its own:
                // read every distinct file the run spliced, once per traversal.
                let c_read = cpu_secs();
                let t_read = Instant::now();
                let mut read_bytes = 0usize;
                for _ in 0..traversals {
                    for n in &names {
                        if let Ok(s) = std::fs::read_to_string(dir.join(n)) {
                            read_bytes += s.len();
                        }
                    }
                }
                let reread = t_read.elapsed().as_secs_f64();
                let reread_cpu = cpu_secs() - c_read;

                println!(
                    "rep={r}\tload={load}\tFRONT={front:.4}\tFRONT_CPU={front_cpu:.4}\tRENDER={render:.6}\tRENDER_CPU={render_cpu:.4}\tREREAD_x{traversals}={reread:.4}\tREREAD_CPU={reread_cpu:.4}\tdiags={}\tsources={n_sources}\tdistinct_files={}\tsource_bytes={bytes}\treread_bytes={read_bytes}\tsink={sink}",
                    failure.diags.len(),
                    names.len()
                );
            }
            Ok(a) => {
                let empty = sigil_ir::SymbolTable::new();
                let t_back = Instant::now();
                let resolved = sigil_link::resolve_layout(&a.module.sections, &empty, true)
                    .expect("resolve_layout");
                let t_layout = t_back.elapsed().as_secs_f64();
                let t_link = Instant::now();
                let linked = sigil_link::link(&resolved, &empty).expect("link");
                let image = sigil_link::flatten(&linked, 0x00).expect("flatten");
                let link = t_link.elapsed().as_secs_f64();
                println!(
                    "rep={r}\tload={load}\tFRONT={front:.4}\tLAYOUT={t_layout:.4}\tLINK={link:.4}\timage_bytes={}\tsources={}",
                    image.len(),
                    a.sources.len()
                );
            }
        }
    }
}
