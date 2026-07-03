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
//! `sigil build --aeon <dir>` assembles the M0 integration harness's Region
//! A + Region B reference source (see `sigil-harness`) and, with `-o`, writes
//! the flattened image to disk.
//!
//! `sigil diff --aeon <dir>` assembles the same regions and compares them
//! byte-for-byte against the committed golden reference blobs, exiting
//! non-zero if either region diverges.

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

/// `sigil build --aeon <dir> [-o <output.bin>]` — assemble the M0 harness's
/// Region A + Region B reference source and, if `-o` is given, write the
/// flattened image to disk.
fn run_build(args: &[String]) {
    let (aeon, output) =
        parse_aeon_and_output(args, true, "sigil build --aeon <dir> [-o <output.bin>]");
    let aeon_path = std::path::Path::new(&aeon);

    let img = match sigil_harness::assemble_reference_regions(aeon_path) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    };

    let region_a = img.section("sec0");
    let region_b = img.section("sec32768");
    let len_a = region_a.map(|s| s.bytes.len()).unwrap_or(0);
    let len_b = region_b.map(|s| s.bytes.len()).unwrap_or(0);

    if let Some(out_path) = output {
        let flat = match sigil_link::flatten_checked(&img, 0x00) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("error: {err}");
                process::exit(1);
            }
        };
        if let Err(err) = std::fs::write(&out_path, &flat) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }

    println!("built: region A {len_a} B @ sec0, region B {len_b} B @ sec32768");
}

/// `sigil diff --aeon <dir>` — assemble the M0 harness's Region A + Region B
/// reference source and compare it byte-for-byte against the committed golden
/// blobs. Exits non-zero if either region diverges.
fn run_diff(args: &[String]) {
    let (aeon, _) = parse_aeon_and_output(args, false, "sigil diff --aeon <dir>");
    let aeon_path = std::path::Path::new(&aeon);

    let img = match sigil_harness::assemble_reference_regions(aeon_path) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    };

    let ref_a = match std::fs::read(sigil_harness::golden_path("region_a.bin")) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("error: cannot read golden region_a.bin: {err}");
            process::exit(1);
        }
    };
    let ref_b = match std::fs::read(sigil_harness::golden_path("region_b.bin")) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("error: cannot read golden region_b.bin: {err}");
            process::exit(1);
        }
    };

    let mut ok = true;
    match sigil_harness::diff_region(&img, "sec0", &ref_a) {
        Ok(()) => println!("region A (sec0):     MATCH ({} bytes)", ref_a.len()),
        Err(err) => {
            println!("region A (sec0):     {err}");
            ok = false;
        }
    }
    match sigil_harness::diff_region(&img, "sec32768", &ref_b) {
        Ok(()) => println!("region B (sec32768): MATCH ({} bytes)", ref_b.len()),
        Err(err) => {
            println!("region B (sec32768): {err}");
            ok = false;
        }
    }

    if !ok {
        process::exit(1);
    }
    println!("OK: both regions byte-identical");
}
