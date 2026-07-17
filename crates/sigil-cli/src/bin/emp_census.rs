//! `emp_census` — the contract-census tool (Sigil diagnostics-tier prework).
//!
//! For every `.emp` file passed on the command line, parse it and emit ONE
//! machine-readable TSV row per `proc` (recursing into `section {}` blocks):
//!
//!   FILE  LINE  PROC  PUB  CLOBBERS  PRESERVES  OUT  COMPUTED_WRITES  CONTRACT
//!
//! - `CLOBBERS`/`OUT`: `-` = no attribute declared (`None`); `()` = the
//!   explicit empty form ("touches nothing" / "returns nothing"); else the
//!   declared reglist (`d0-d3/a1`). `PRESERVES`: `-` = none, else the reglist.
//! - `COMPUTED_WRITES`: the lint's own write set — the SAME
//!   [`proc_written_registers`](sigil_frontend_emp::lower::proc_written_registers)
//!   detector `check_clobbers`/`check_out` use, so the census can never drift
//!   from the lint (and the auto-inc/dec follow-up updates both at once). `?`
//!   marks a proc whose body did not evaluate to a resolved `Code` value
//!   (splice that needs an unresolved cross-module callee) — its write set is
//!   unknown to a single-file lower.
//! - `CONTRACT`: `yes` if the proc declares any of clobbers/preserves/out,
//!   else `no` (a no-contract proc — invisible to the clobber lint; its
//!   `COMPUTED_WRITES` is the retrofit demand data).
//!
//! All procs in the aeon corpus lower as 68000 (the `cpu: z80` sections are
//! zero-proc data banks), so the census evaluates every body as M68000 with no
//! `-D` defines — matching a plain `sigil emp <file>` single-file lower, whose
//! diagnostics are the authoritative `[proc.clobber-undeclared]` firing list
//! the census is read alongside.

use sigil_frontend_emp::ast::{Item, ProcDecl};
use sigil_frontend_emp::lower::proc_written_registers;
use sigil_frontend_emp::{ast, eval, parse_str};
use sigil_ir::backend::Cpu;
use sigil_span::SourceMap;

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: emp_census <file.emp>...");
        std::process::exit(2);
    }
    println!("FILE\tLINE\tPROC\tPUB\tCLOBBERS\tPRESERVES\tOUT\tCOMPUTED_WRITES\tCONTRACT");
    for path in &files {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {path}: {e}");
                continue;
            }
        };
        let (file, _diags) = parse_str(&src);
        let mut map = SourceMap::new();
        map.add(src.clone());
        let mut counter: u32 = 0;
        // Short display name = path with a common prefix trimmed if present.
        let short = path.strip_prefix("./").unwrap_or(path);
        walk(&file.items, &file, short, &map, &mut counter);
    }
}

/// Recurse the item list (into `section {}` blocks) reporting each proc.
fn walk(items: &[Item], file: &ast::File, path: &str, map: &SourceMap, counter: &mut u32) {
    for item in items {
        match item {
            Item::Proc(p) => report(p, file, path, map, counter),
            Item::Section(s) => walk(&s.items, file, path, map, counter),
            _ => {}
        }
    }
}

fn report(p: &ProcDecl, file: &ast::File, path: &str, map: &SourceMap, counter: &mut u32) {
    let (line, _col) = map.location(p.span);
    let pubcol = if p.public { "pub" } else { "-" };
    let clob = opt_reglist(&p.clobbers);
    let pres = seg_reglist(&p.preserves);
    let out = opt_reglist(&p.out);
    let contract =
        if p.clobbers.is_some() || !p.preserves.is_empty() || p.out.is_some() { "yes" } else { "no" };

    // Evaluate the body (single-file, 68000, no defines) → resolved CodeBuf →
    // the lint's own write set. `None` = the body did not resolve to Code.
    let (buf, _ds, next) =
        eval::eval_proc_body(file, &p.name, &p.params, &p.body, p.span, *counter, Cpu::M68000, &[]);
    *counter = next;
    let computed = match &buf {
        Some(b) => {
            // `a7` is dropped from the displayed set: every a7 write in this
            // corpus is stack push/pop (`-(sp)`/`(sp)+`), which `check_clobbers`
            // exempts as stack discipline — it is never a clobber/out retrofit
            // target, and showing it on every push/pop proc would only obscure
            // the clobber-relevant diff. (A genuine stack REPLACEMENT `movea.l
            // x, sp` would still surface as a live `[proc.clobber-undeclared]`
            // firing, which check_clobbers does NOT exempt.)
            let regs: Vec<String> =
                proc_written_registers(b).into_iter().filter(|r| r != "a7").collect();
            if regs.is_empty() { "(none)".to_string() } else { regs.join("/") }
        }
        None => "?".to_string(),
    };

    println!(
        "{path}\t{line}\t{}\t{pubcol}\t{clob}\t{pres}\t{out}\t{computed}\t{contract}",
        p.name
    );
}

/// Render an `Option<reglist>` attribute: `-` = None, `()` = explicit empty,
/// else the reglist spelling.
fn opt_reglist(segs: &Option<Vec<(String, Option<String>)>>) -> String {
    match segs {
        None => "-".to_string(),
        Some(v) if v.is_empty() => "()".to_string(),
        Some(v) => seg_reglist(v),
    }
}

/// Render a reglist as `lo-hi/lo/...` (empty → `-`).
fn seg_reglist(segs: &[(String, Option<String>)]) -> String {
    if segs.is_empty() {
        return "-".to_string();
    }
    segs.iter()
        .map(|(lo, hi)| match hi {
            Some(h) => format!("{lo}-{h}"),
            None => lo.clone(),
        })
        .collect::<Vec<_>>()
        .join("/")
}
