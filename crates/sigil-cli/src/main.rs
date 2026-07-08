//! sigil-cli: the `sigil` command-line assembler binary.
//!
//! Usage: `sigil <input.asm> [-o <output.bin>] [--hex]`
//!        `sigil parse <input.emp>`
//!        `sigil emp <input.emp> [-o <output.bin>] [--hex]`
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
        Some("emp") => return run_emp(&args[2..]),
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

/// Compile a Spec 2 `.emp` source string to its flat linked binary image.
/// Mirrors the top-level `.asm` path but through the emp front end: parse →
/// [`lower_module`](sigil_frontend_emp::lower::lower_module) (threading
/// `include_root` so comptime `embed`/`import` resolve against the source
/// directory, §6.7) → [`resolve_layout`](sigil_link::resolve_layout) (emp defers
/// jmp/jsr width + layout to link, D-P4.2) → [`link`](sigil_link::link) →
/// [`flatten`](sigil_link::flatten). Returns the image bytes (or `None` if a
/// hard error stopped compilation) plus ALL diagnostics collected; the caller
/// renders them and treats any `Error`-level diagnostic as fatal.
fn compile_emp(
    src: &str,
    include_root: Option<&std::path::Path>,
) -> (Option<Vec<u8>>, Vec<sigil_span::Diagnostic>) {
    let (file, mut diags) = sigil_frontend_emp::parse_str(src);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return (None, diags);
    }
    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::Cpu::M68000,
        include_root: include_root.map(std::path::Path::to_path_buf),
    };
    let (module, lower_diags) = sigil_frontend_emp::lower::lower_module(&file, &opts);
    diags.extend(lower_diags);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return (None, diags);
    }
    match link_sections(&module.sections, &module.link_asserts) {
        Ok(image) => (Some(image), diags),
        Err(mut ds) => {
            diags.append(&mut ds);
            (None, diags)
        }
    }
}

/// The shared emp link prefix: `resolve_layout` (emp defers jmp/jsr width +
/// layout to link) → `link` → the deferred link-assertion checker (D-H.6), against
/// one flat empty [`SymbolTable`] so cross-module (and cross-section) references
/// resolve. The two link tails — `flatten` (no map) and `emit_rom` (map) — reuse
/// this identical prefix, so they differ only in the final materialization step.
/// Byte-identical whether fed one module's sections or a whole concatenated
/// program. A failing deferred `ensure`/`ensure_fatal` (D-H.4) is an `Error`
/// diagnostic here — folded against the POST-relaxation symbol table (`asserts`
/// empty ⇒ no check, byte-neutral).
fn link_to_image(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
) -> Result<sigil_link::LinkedImage, Vec<sigil_span::Diagnostic>> {
    let empty = sigil_ir::SymbolTable::new();
    let resolved = sigil_link::resolve_layout(sections, &empty, true)?;
    let image = sigil_link::link(&resolved, &empty)?;
    // The link succeeded and labels are at their final post-relaxation VMAs — now
    // decide the deferred guards against exactly those addresses (D-H.6/D-H.7).
    let assert_diags = sigil_link::check_link_asserts(&resolved, &empty, asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(assert_diags);
    }
    Ok(image)
}

/// The no-map link seam: [`link_to_image`] then `flatten` (gap-fill 0x00, no
/// region validation).
fn link_sections(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
) -> Result<Vec<u8>, Vec<sigil_span::Diagnostic>> {
    Ok(sigil_link::flatten(&link_to_image(sections, asserts)?, 0x00))
}

/// The shared emp output tail: write `image` to `output` (if given), print it as
/// `--hex` (if set), and always report `built: N bytes`. Exits non-zero on a
/// write failure.
fn emit_image(image: &[u8], output: Option<&str>, hex: bool) {
    if let Some(out_path) = output {
        if let Err(err) = std::fs::write(out_path, image) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }
    if hex {
        let rendered: Vec<String> = image.iter().map(|b| format!("{b:02X}")).collect();
        println!("{}", rendered.join(" "));
    }
    println!("built: {} bytes", image.len());
}

/// Consume the value following a value-taking flag at `args[*i]`, advancing `i`.
/// A missing value — or one that looks like another flag (`-`-prefixed) — is a
/// usage error (exit 2), so e.g. `--root -o` cannot silently swallow `-o` as the
/// root directory.
fn flag_value(args: &[String], i: &mut usize, flag: &str) -> String {
    *i += 1;
    match args.get(*i) {
        Some(v) if !v.starts_with('-') => v.clone(),
        _ => {
            eprintln!("error: {flag} requires a value argument");
            process::exit(2);
        }
    }
}

/// `sigil emp <input.emp> [-o <output.bin>] [--hex]` — compile a Spec 2 `.emp`
/// module to a flat binary image. `embed`/`import` paths resolve against the
/// source file's own directory (the capability-sandbox include-root, §6.7),
/// canonicalized so a comptime capture path is stable regardless of cwd.
fn run_emp(args: &[String]) {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut root_arg: Option<String> = None;
    let mut prelude: Option<String> = None;
    let mut map_arg: Option<String> = None;
    let mut hex = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => output = Some(flag_value(args, &mut i, "-o")),
            "--root" => root_arg = Some(flag_value(args, &mut i, "--root")),
            "--prelude" => prelude = Some(flag_value(args, &mut i, "--prelude")),
            "--map" => map_arg = Some(flag_value(args, &mut i, "--map")),
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
            eprintln!("usage: sigil emp <input.emp> [--root <dir>] [--prelude <module.id>] [-o <output.bin>] [--hex]");
            process::exit(2);
        }
    };

    // Multi-module path: `--root <dir>` gathers, resolves, and links the whole
    // reachable program. Single-file path (no `--root`) is unchanged.
    if let Some(root_dir) = root_arg {
        run_emp_program(
            &input,
            &root_dir,
            prelude.as_deref(),
            map_arg.as_deref(),
            output.as_deref(),
            hex,
        );
        return;
    }
    if map_arg.is_some() {
        eprintln!("error: --map requires --root (region placement is a multi-module concern)");
        process::exit(2);
    }

    let src = match std::fs::read_to_string(&input) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {input}: {err}");
            process::exit(1);
        }
    };

    // Include-root = the source file's own directory (empty parent → cwd),
    // canonicalized so the sandbox and capture ledger see a stable absolute path.
    let parent = std::path::Path::new(&input).parent().unwrap_or(std::path::Path::new(""));
    let root_dir = if parent.as_os_str().is_empty() { std::path::Path::new(".") } else { parent };
    let root = std::fs::canonicalize(root_dir).ok();
    let (image, diags) = compile_emp(&src, root.as_deref());

    if !diags.is_empty() {
        let mut map = sigil_span::SourceMap::new();
        map.add(src);
        for d in &diags {
            let (line, col) = map.location(d.primary);
            eprintln!("{input}:{line}:{col}: {}", d.message);
        }
    }

    let fatal = diags.iter().any(|d| d.level == sigil_span::Level::Error);
    let image = match image {
        Some(img) if !fatal => img,
        _ => process::exit(1),
    };

    emit_image(&image, output.as_deref(), hex);
}

/// The multi-module `sigil emp <entry> --root <dir>` path: scan the root, derive
/// the entry module id from the entry path, build the whole reachable program
/// ([`build_program`](sigil_frontend_emp::resolve::build_program)), and — if no
/// error diagnostics — run the same `resolve_layout` → `link` → `flatten` seam as
/// the single-file path. Diagnostics render as `path:line:col: message` using a
/// [`SourceMap`](sigil_span::SourceMap) rebuilt in the manifest's SourceId order.
fn run_emp_program(
    input: &str,
    root_dir: &str,
    prelude: Option<&str>,
    map_path: Option<&str>,
    output: Option<&str>,
    hex: bool,
) {
    use sigil_frontend_emp::resolve;
    use std::path::Path;

    let (manifest, mut diags) = resolve::manifest::Manifest::scan(Path::new(root_dir));

    let entry_id = match resolve::entry_id_for_path(&manifest, Path::new(input)) {
        Some(id) => id,
        None => {
            // Surface the manifest's own diagnostics FIRST: a mistyped/nonexistent
            // `--root` makes `scan` emit `cannot read module root …` AND yields no
            // modules (so entry-id resolution fails) — rendering only the generic
            // "not a module under --root" would bury the real cause.
            render_program_diags(&manifest, &diags);
            if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
                process::exit(1);
            }
            eprintln!("error: entry file {input} is not a module under --root {root_dir}");
            process::exit(1);
        }
    };

    let include_root = std::fs::canonicalize(root_dir).ok();
    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::Cpu::M68000,
        include_root,
    };

    // `link_asserts`: deferred link-time guards (D-H.4), decided by the link tails
    // below against the post-relaxation symbol table.
    let (mut sections, link_asserts, mut pdiags) =
        resolve::build_program(&manifest, &entry_id, prelude, &opts);
    diags.append(&mut pdiags);

    render_program_diags(&manifest, &diags);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        process::exit(1);
    }

    // `--map`: load the region map, place each section into its named region, then
    // link and emit through `emit_rom` (which validates each section's region
    // budget, §7.3). Without `--map`, keep today's `flatten` behavior unchanged.
    let image = match map_path {
        Some(path) => {
            let toml = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(err) => {
                    eprintln!("error: cannot read {path}: {err}");
                    process::exit(1);
                }
            };
            let map = match sigil_link::load_map(&toml) {
                Ok(m) => m,
                Err(err) => {
                    eprintln!("error: cannot load map {path}: {err}");
                    process::exit(1);
                }
            };
            let pdiags = resolve::place_sections(&mut sections, &map);
            render_program_diags(&manifest, &pdiags);
            if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
                process::exit(1);
            }
            match link_rom(&sections, &link_asserts, &map) {
                Ok(rom) => rom,
                Err(ds) => {
                    render_program_diags(&manifest, &ds);
                    process::exit(1);
                }
            }
        }
        None => {
            // No `--map`: nothing would otherwise place these sections, so every
            // module's section would keep `lma == 0` and overlap at the origin
            // (BUG I3). Pack them sequentially from 0 so cross-module branches
            // resolve to distinct, non-overlapping addresses (single reachable
            // module → one section at 0, unchanged).
            resolve::place_sequential(&mut sections, 0);
            match link_sections(&sections, &link_asserts) {
                Ok(image) => image,
                Err(ds) => {
                    render_program_diags(&manifest, &ds);
                    process::exit(1);
                }
            }
        }
    };

    emit_image(&image, output, hex);
}

/// Region-placed emp link seam: `resolve_layout` → `link` → deferred-assert check
/// (D-H.6) → `emit_rom` against the memory map, so each section is validated for
/// region containment/budget (§7.3) and gaps are filled with the map's default
/// byte. A failing deferred guard (D-H.4) surfaces as a proper span-carrying
/// diagnostic (same channel as the no-map tail); an `emit_rom` region/placement
/// error is wrapped as a single null-span diagnostic.
fn link_rom(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
    map: &sigil_ir::map::MemoryMap,
) -> Result<Vec<u8>, Vec<sigil_span::Diagnostic>> {
    let linked = link_to_image(sections, asserts)?;
    sigil_link::emit_rom(&linked, map).map_err(|msg| {
        vec![sigil_span::Diagnostic {
            level: sigil_span::Level::Error,
            message: msg,
            primary: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
        }]
    })
}

/// Render multi-module diagnostics as `path:line:col: message`. The manifest
/// parsed each file under a sequential [`SourceId`](sigil_span::SourceId) (sorted
/// order); we rebuild a [`SourceMap`](sigil_span::SourceMap) in that same order so
/// `map.location` is correct, and prefix with the file recorded in
/// `manifest.sources`. Diagnostics with an unmapped source id fall back to the
/// bare message.
fn render_program_diags(
    manifest: &sigil_frontend_emp::resolve::manifest::Manifest,
    diags: &[sigil_span::Diagnostic],
) {
    if diags.is_empty() {
        return;
    }
    // Rebuild the SourceMap so its internal index equals the SourceId for EVERY
    // id in `0..=max_id`, including any gap (a file that failed to read has a
    // `sources` entry — dense by construction in `Manifest::scan` — but a gap
    // could still arise defensively; fill it with empty text so `map.location`
    // never over-indexes). `SourceMap::add` assigns ids sequentially from 0, so
    // adding one text per k in order aligns index ↔ SourceId.
    let max_id = manifest.sources.keys().map(|id| id.0).max().unwrap_or(0);
    let mut map = sigil_span::SourceMap::new();
    for k in 0..=max_id {
        let text = manifest
            .sources
            .get(&sigil_span::SourceId(k))
            .map(|p| std::fs::read_to_string(p).unwrap_or_default())
            .unwrap_or_default();
        map.add(text);
    }
    for d in diags {
        // Only render a location when the id is both known to `sources` AND within
        // the rebuilt map's bounds — otherwise fall back to the bare message so a
        // stray/out-of-range source id can never panic mid-error-report.
        match manifest.sources.get(&d.primary.source) {
            Some(path) if d.primary.source.0 <= max_id => {
                let (line, col) = map.location(d.primary);
                eprintln!("{}:{line}:{col}: {}", path.display(), d.message);
            }
            _ => eprintln!("error: {}", d.message),
        }
    }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// `compile_emp` must resolve an `embed(...)` in the source against the
    /// file's own directory (the include-root the CLI supplies) and lower it to
    /// the embedded bytes — the end-to-end proof that the production emp path
    /// wires `include_root` (Plan 5's sandbox is otherwise `[sandbox.no-root]`).
    #[test]
    fn compile_emp_resolves_embed_against_source_dir() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
        let src = std::fs::read_to_string(dir.join("prog.emp")).expect("read prog.emp");
        let (image, diags) = crate::compile_emp(&src, Some(&dir));
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "unexpected error diagnostics: {diags:?}"
        );
        let blob = std::fs::read(dir.join("blob.bin")).expect("read blob.bin");
        assert_eq!(image.expect("image bytes"), blob);
    }
}
