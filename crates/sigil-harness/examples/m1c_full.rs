//! M1.C/M1.D full-build recon: point sigil at the REAL aeon main.asm and report
//! how far it gets. Prints the first N diagnostics.
use std::path::PathBuf;

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::Cpu;

fn main() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    );
    let root = std::env::var("ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| aeon.join("games/sonic4/main.asm"));

    let mut defines: Vec<(String, i64)> = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    // Optional extra stub defines from env: STUBS="A=0x1,B=2"
    if let Ok(s) = std::env::var("STUBS") {
        for kv in s.split(',').filter(|s| !s.is_empty()) {
            let (n, v) = kv.split_once('=').expect("STUBS entry needs =");
            let v = v.trim();
            let parsed = if let Some(h) = v.strip_prefix("0x") {
                i64::from_str_radix(h, 16).unwrap()
            } else {
                v.parse::<i64>().unwrap()
            };
            defines.push((n.trim().to_string(), parsed));
        }
    }

    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines,
        include_root: Some(aeon.clone()),
    };

    match assemble_root(&root, &opts) {
        Ok(m) => {
            println!("ASSEMBLED OK: {} sections", m.sections.len());
            for s in &m.sections {
                // NB: `image_len()` is intentionally NOT printed here — a raw
                // assembled section can still hold `JmpJsrSym` fragments (bare
                // jmp/jsr), whose length is only fixed once `resolve_layout`
                // width-selects them; calling `image_len()` pre-resolve panics.
                // This recon is a diagnostics collector (goal: 0 diagnostics);
                // the full assemble→resolve_layout→link→emit path is exercised
                // by the `m1c_rom` gate (M1.D T4), not here.
                println!(
                    "  section {} cpu={:?} vma_base={:?} lma={:#x} frags={}",
                    s.name,
                    s.cpu,
                    s.vma_base,
                    s.lma,
                    s.fragments.len(),
                );
            }
        }
        Err(diags) => {
            println!("FAILED: {} diagnostics", diags.len());
            // Histogram by normalized message class (strip trailing specifics).
            use std::collections::BTreeMap;
            let mut hist: BTreeMap<String, usize> = BTreeMap::new();
            for d in &diags {
                let key = normalize(&d.message);
                *hist.entry(key).or_default() += 1;
            }
            let mut v: Vec<_> = hist.into_iter().collect();
            v.sort_by_key(|b| std::cmp::Reverse(b.1));
            println!("--- gap classes (count) ---");
            for (k, c) in &v {
                println!("  {c:5}  {k}");
            }
            if let Ok(filter) = std::env::var("FILTER") {
                // Print DISTINCT full messages containing the filter substring.
                let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
                for d in &diags {
                    if d.message.contains(&filter) {
                        *seen.entry(d.message.clone()).or_default() += 1;
                    }
                }
                println!("--- distinct messages matching {filter:?} ({} distinct) ---", seen.len());
                let mut v: Vec<_> = seen.into_iter().collect();
                v.sort_by_key(|b| std::cmp::Reverse(b.1));
                for (m, c) in v {
                    println!("  {c:4}  {m}");
                }
            }
        }
    }
}

/// Collapse message specifics (backtick-quoted names, numbers) so similar gaps
/// bucket together.
fn normalize(msg: &str) -> String {
    let mut out = String::new();
    let mut in_tick = false;
    for ch in msg.chars() {
        if ch == '`' {
            if !in_tick {
                out.push_str("`…`");
            }
            in_tick = !in_tick;
            continue;
        }
        if in_tick {
            continue;
        }
        if ch.is_ascii_digit() {
            out.push('#');
        } else {
            out.push(ch);
        }
    }
    // collapse runs of '#'
    while out.contains("##") {
        out = out.replace("##", "#");
    }
    out
}
