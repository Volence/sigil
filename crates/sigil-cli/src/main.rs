//! sigil-cli: the `sigil` command-line assembler binary.
//!
//! Usage: `sigil <input.asm> [-o <output.bin>] [--hex]`
//!        `sigil parse <input.emp>`
//!        `sigil build --aeon <dir> [-o <output.bin>]`
//!        `sigil diff --aeon <dir>`
//!
//! Assembles the given Z80 source file. Writes the binary image to the path
//! given by `-o` (if supplied). When `--hex` is passed, prints the output
//! bytes as uppercase space-separated hex (e.g. `00 3E 05`) to stdout.
//!
//! `sigil parse <input.emp>` runs only the .emp lexer/parser front end
//! (Spec 2 Plan 1) and reports success or every diagnostic collected.
//!
//! `sigil build --aeon <dir>` assembles the full non-debug Aeon ROM (the whole
//! `main.asm` include tree, no stubs) and, with `-o`, writes the emitted ROM to
//! disk.
//!
//! `sigil diff --aeon <dir>` assembles the same full ROM and compares the Z80
//! sound driver's Region A + Region B byte-for-byte against `aeon/s4.bin`,
//! exiting non-zero if either region diverges.

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("parse") => return run_parse(),
        Some("build") => return run_build(&args[2..]),
        Some("diff") => return run_diff(&args[2..]),
        _ => {}
    }

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut hex = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                match args.get(i) {
                    Some(path) => output = Some(path.clone()),
                    None => {
                        eprintln!("error: -o requires a path argument");
                        process::exit(2);
                    }
                }
            }
            "--hex" => hex = true,
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    eprintln!("error: unexpected argument '{other}'");
                    process::exit(2);
                }
            }
        }
        i += 1;
    }

    let input = match input {
        Some(path) => path,
        None => {
            eprintln!("usage: sigil <input.asm> [-o <output.bin>] [--hex]");
            process::exit(2);
        }
    };

    let src = match std::fs::read_to_string(&input) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {input}: {err}");
            process::exit(1);
        }
    };

    let opts = sigil_frontend_as::Options::default();
    let module = match sigil_frontend_as::assemble(&src, &opts) {
        Ok(m) => m,
        Err(diags) => {
            for d in &diags {
                eprintln!("error: {}", d.message);
            }
            process::exit(1);
        }
    };
    let linked = match sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()) {
        Ok(img) => img,
        Err(diags) => {
            for d in &diags {
                eprintln!("error: {}", d.message);
            }
            process::exit(1);
        }
    };
    let image = sigil_link::flatten(&linked, 0x00);

    if let Some(out_path) = output {
        if let Err(err) = std::fs::write(&out_path, &image) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }

    if hex {
        let rendered: Vec<String> = image.iter().map(|b| format!("{b:02X}")).collect();
        println!("{}", rendered.join(" "));
    }
}

/// `sigil parse <input.emp>` — run the .emp lexer/parser front end only and
/// report success (module path + item count) or every diagnostic collected,
/// rendered as `path:line:col: message` via `SourceMap::location`.
fn run_parse() {
    let path = match std::env::args().nth(2) {
        Some(path) => path,
        None => {
            eprintln!("usage: sigil parse <file.emp>");
            process::exit(2);
        }
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {path}: {err}");
            process::exit(1);
        }
    };

    let (file, diags) = sigil_frontend_emp::parse_str(&src);
    if diags.is_empty() {
        println!(
            "{path}: OK — module {}, {} items",
            file.module.path.segments.join("."),
            file.items.len()
        );
        return;
    }

    let mut map = sigil_span::SourceMap::new();
    map.add(src);
    for d in &diags {
        let (line, col) = map.location(d.primary);
        println!("{path}:{line}:{col}: {}", d.message);
    }
    process::exit(1);
}

/// Parse `--aeon <dir>` (required) and, if `allow_output` is set, an optional
/// `-o <path>` out of a subcommand's argument slice. Any other/unexpected
/// argument is a usage error. Returns `(aeon_dir, output_path)`.
fn parse_aeon_and_output(args: &[String], allow_output: bool, usage: &str) -> (String, Option<String>) {
    let mut aeon: Option<String> = None;
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--aeon" => {
                i += 1;
                match args.get(i) {
                    Some(path) => aeon = Some(path.clone()),
                    None => {
                        eprintln!("error: --aeon requires a path argument");
                        process::exit(2);
                    }
                }
            }
            "-o" if allow_output => {
                i += 1;
                match args.get(i) {
                    Some(path) => output = Some(path.clone()),
                    None => {
                        eprintln!("error: -o requires a path argument");
                        process::exit(2);
                    }
                }
            }
            other => {
                eprintln!("error: unexpected argument '{other}'");
                eprintln!("usage: {usage}");
                process::exit(2);
            }
        }
        i += 1;
    }

    let aeon = match aeon {
        Some(path) => path,
        None => {
            eprintln!("usage: {usage}");
            process::exit(2);
        }
    };
    (aeon, output)
}

/// `sigil build --aeon <dir> [-o <output.bin>]` — assemble the full non-debug
/// Aeon ROM (no stubs) and, if `-o` is given, write the emitted ROM to disk.
fn run_build(args: &[String]) {
    let (aeon, output) =
        parse_aeon_and_output(args, true, "sigil build --aeon <dir> [-o <output.bin>]");
    let aeon_path = std::path::Path::new(&aeon);

    let img = match sigil_harness::assemble_full_rom(aeon_path) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    };

    let len_a = sigil_harness::region_at_lma(&img, sigil_harness::REGION_A_LMA)
        .map(<[u8]>::len)
        .unwrap_or(0);
    let len_b = sigil_harness::region_at_lma(&img, sigil_harness::REGION_B_LMA)
        .map(<[u8]>::len)
        .unwrap_or(0);

    if let Some(out_path) = output {
        let map_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
        let map = match std::fs::read_to_string(&map_path)
            .map_err(|e| e.to_string())
            .and_then(|s| sigil_link::load_map(&s))
        {
            Ok(map) => map,
            Err(err) => {
                eprintln!("error: load map {}: {err}", map_path.display());
                process::exit(1);
            }
        };
        let rom = match sigil_link::emit_rom(&img, &map) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("error: {err}");
                process::exit(1);
            }
        };
        if let Err(err) = std::fs::write(&out_path, &rom) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }

    println!(
        "built: full ROM, sound driver region A {len_a} B @ {:#x}, region B {len_b} B @ {:#x}",
        sigil_harness::REGION_A_LMA,
        sigil_harness::REGION_B_LMA
    );
}

/// `sigil diff --aeon <dir>` — assemble the full non-debug Aeon ROM (no stubs)
/// and compare the sound driver's Region A + Region B byte-for-byte against
/// `aeon/s4.bin`. Exits non-zero if either region diverges.
fn run_diff(args: &[String]) {
    let (aeon, _) = parse_aeon_and_output(args, false, "sigil diff --aeon <dir>");
    let aeon_path = std::path::Path::new(&aeon);

    let img = match sigil_harness::assemble_full_rom(aeon_path) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    };

    let refrom = match std::fs::read(aeon_path.join("s4.bin")) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("error: cannot read {}/s4.bin: {err}", aeon);
            process::exit(1);
        }
    };

    let mut ok = true;
    for (label, lma) in
        [("region A", sigil_harness::REGION_A_LMA), ("region B", sigil_harness::REGION_B_LMA)]
    {
        let bytes = match sigil_harness::region_at_lma(&img, lma) {
            Some(b) => b,
            None => {
                println!("{label} ({lma:#x}): no linked section at that LMA");
                ok = false;
                continue;
            }
        };
        let start = lma as usize;
        let end = start + bytes.len();
        match refrom.get(start..end) {
            Some(win) if win == bytes => println!("{label} ({lma:#x}): MATCH ({} bytes)", bytes.len()),
            Some(win) => {
                let i = (0..bytes.len()).find(|&i| bytes[i] != win[i]).unwrap();
                println!(
                    "{label} ({lma:#x}): diverged at region offset {i:#x} (ROM {:#x}): \
                     sigil {:#04x} != ref {:#04x}",
                    start + i,
                    bytes[i],
                    win[i]
                );
                ok = false;
            }
            None => {
                println!("{label} ({lma:#x}): window exceeds reference ROM ({} B)", refrom.len());
                ok = false;
            }
        }
    }

    if !ok {
        process::exit(1);
    }
    println!("OK: both regions byte-identical");
}
