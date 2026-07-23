//! `emp_contracts` — run the contract-grammar v2 transitive closure (§1) over a
//! set of `.emp` files and print the firing list + boundary stats. The driver
//! behind the G1 checkpoint: its firing list is compared against the census's
//! predicted debt (13 under-decl + the transitive additions).
//!
//!   emp_contracts <file.emp>...
//!
//! Prints, in order: stats, extern/proc collisions (§11 Q4), unresolved callees
//! (holes — missing `extern proc` decls), then every firing sorted (proc, reg).

use sigil_frontend_emp::parse_str;

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: emp_contracts <file.emp>...");
        std::process::exit(2);
    }

    let mut defines: Vec<(String, i128)> = Vec::new();
    let mut file_paths: Vec<String> = Vec::new();
    let mut it = paths.iter();
    while let Some(a) = it.next() {
        if a == "-D" {
            if let Some(kv) = it.next() {
                if let Some((k, v)) = kv.split_once('=') {
                    if let Ok(n) = v.parse::<i128>() {
                        defines.push((k.to_string(), n));
                        continue;
                    }
                }
                eprintln!("bad -D {kv} (want NAME=INT)");
            }
            continue;
        }
        file_paths.push(a.clone());
    }

    let mut files = Vec::new();
    for path in &file_paths {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {path}: {e}");
                continue;
            }
        };
        let (file, diags) = parse_str(&src);
        let errs = diags.iter().filter(|d| matches!(d.level, sigil_span::Level::Error)).count();
        if errs > 0 {
            eprintln!("warning: {path}: {errs} parse error(s) — analyzing anyway");
        }
        files.push(file);
    }

    let report = sigil_frontend_emp::corpus_contracts::analyze_corpus_with(&files, &defines);

    println!("== contract-closure report ==");
    println!(
        "procs (incl externs): {}   externs: {}   contract-types: {}",
        report.proc_count, report.extern_count, report.contract_type_count
    );

    println!("\n-- dropped instructions (must be 0): {} --", report.dropped_instrs);
    for (proc, n) in &report.dropped_by_proc {
        println!("  DROPPED {n:>3}  {proc}");
    }

    println!("\n-- extern/proc collisions (§11 Q4): {} --", report.extern_collisions.len());
    for (name, _span) in &report.extern_collisions {
        println!("  COLLISION  {name}  (declared both extern proc and proc)");
    }

    let holes = &report.closure.unresolved_callees;
    println!("\n-- unresolved callees (holes — missing extern proc?): {} --", holes.len());
    for h in holes {
        println!("  HOLE  {h}");
    }

    println!("\n-- firings ({}): --", report.firings.len());
    for f in &report.firings {
        let kind = if f.unbounded {
            "UNBOUNDED".to_string()
        } else if f.transitive {
            format!("transitive {}", f.reg.as_deref().unwrap_or("?"))
        } else {
            format!("direct     {}", f.reg.as_deref().unwrap_or("?"))
        };
        println!("  {:<28} {kind}", f.proc);
    }

    use sigil_frontend_emp::flag_check::FlagFiringKind;
    println!("\n-- flag-result firings (§6, {}): --", report.flag_firings.len());
    for f in &report.flag_firings {
        let kind = match &f.kind {
            FlagFiringKind::Unused => format!("[call.flag-result-unused] {} unconsumed", f.flag),
            FlagFiringKind::InvalidPathRead { reg, cc } => {
                format!("[call.result-invalid-path] {reg} read where !{cc}")
            }
        };
        println!("  {:<28} calls {:<24} {kind}", f.proc, f.callee);
    }

    println!(
        "\n-- [call.input-undefined] firings (D1b, {}): --",
        report.input_firings.len()
    );
    for f in &report.input_firings {
        println!("  {:<28} calls {:<24} input {} undefined on some path", f.proc, f.callee, f.reg);
    }

    println!(
        "\n-- [call.live-clobbered] firings (D1c, {}): --",
        report.live_clobbered_firings.len()
    );
    for f in &report.live_clobbered_firings {
        println!("  {:<28} calls {:<24} holds {} across clobber", f.proc, f.callee, f.reg);
    }

    println!("\n-- dead-saves (D1d worklist, {}): --", report.dead_saves.len());
    for d in &report.dead_saves {
        println!("  {:<28} {:<4} bracketing {}", d.proc, d.reg, d.callees.join(","));
    }

    println!(
        "\n-- [call.slot-type-mismatch] firings (G5, {}): --",
        report.slot_firings.len()
    );
    for f in &report.slot_firings {
        let found = f.found.as_deref().unwrap_or("an untyped value");
        println!(
            "  {:<28} calls {:<24} slot {} expects {} but found {}",
            f.proc, f.callee, f.reg, f.expected, found
        );
    }

    println!(
        "\n-- [branch.condition-constant] firings (item-4 rider, {}): --",
        report.branch_const_firings.len()
    );
    for f in &report.branch_const_firings {
        let dir = if f.always_taken { "ALWAYS taken" } else { "NEVER taken" };
        println!(
            "  {:<28} b{:<3} statically decided ({dir}) @ {}..{}",
            f.proc, f.cc, f.span.start, f.span.end
        );
    }
}
