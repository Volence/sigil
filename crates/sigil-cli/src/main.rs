//! sigil-cli: the `sigil` command-line assembler binary.
//!
//! Usage: `sigil <input.asm> [-o <output.bin>] [--hex]`
//!        `sigil parse <input.emp>`
//!
//! Assembles the given Z80 source file. Writes the binary image to the path
//! given by `-o` (if supplied). When `--hex` is passed, prints the output
//! bytes as uppercase space-separated hex (e.g. `00 3E 05`) to stdout.
//!
//! `sigil parse <input.emp>` runs only the .emp lexer/parser front end
//! (Spec 2 Plan 1) and reports success or every diagnostic collected.

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("parse") {
        return run_parse();
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

    let image = match sigil_frontend_as::assemble_str(&src) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("error: assembly failed: {err}");
            process::exit(1);
        }
    };

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
