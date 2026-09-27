//! sigil invoked under the program name `asl`: a drop-in for the AS macro
//! assembler in a community disassembly's own, unmodified build script.
//!
//! The scripts (`build_tools/lua/common.lua`, `assemble_file`, shared by
//! `s1disasm`, `s2disasm` and `skdisasm`) run
//!
//! ```text
//! asl -xx -n -q -A -L -U -E -i . [-c] [-D NAME=VALUE] X.asm
//! ```
//!
//! and read the result off the file system: `X.p` present is success, and then
//! `p2bin <-p/-z> X.p X.bin [X.h]` places and compresses; `X.p` absent with `X.log`
//! present is an assembly error, whose log the script prints; neither is "the
//! assembler crashed". With `-c`, `X.h` is the share file `p2bin` extends and
//! Sonic 2's `build.lua` patches the ROM from.
//!
//! So in this mode sigil assembles and links as `sigil <input.asm>` does, and
//! then, instead of placing the image itself, writes what asl writes: the object
//! file for the stock `p2bin` ([`sigil_link::code_file_records`]), the share file
//! under `-c`, and its diagnostics to the log under `-E`. It does not apply the
//! `-z` placement, compress, or fix the header: those are the script's own
//! later steps, as they are after asl.
//!
//! What each accepted option does here:
//!
//! | option | here |
//! |---|---|
//! | `-xx`, `-n`, `-q`, `-A` | nothing: they shape asl's own messages and its symbol storage. This mode prints no banner or pass count whether or not `-q` is given |
//! | `-U` | required. asl's `-U` makes symbols case-sensitive, and sigil's always are (the AS front end folds the case of directives and mnemonics, never of symbols), so without `-U` the two tools would read `Foo` and `foo` differently and the difference could be silent. It is refused by name rather than assumed |
//! | `-L` | writes no listing (a gap), and removes a listing an earlier run left at `X.lst`, so no stale listing stands beside a fresh object file |
//! | `-E [file]` | diagnostics go to `file`, by default `X.log`, instead of stderr. As asl does, the argument after `-E` is the file when it does not start with `-`, and a run with nothing to report removes the file |
//! | `-i <dir>` | sigil resolves `include` against the source file's own directory, which is where asl looks first; a `<dir>` that is that directory adds nothing, and any other is refused by name, since sigil does not search it |
//! | `-c` | writes the share file `X.h` in asl's layout |
//! | `-D` | asl's command-line define, as `sigil <input.asm> -D` |
//!
//! Every other option is refused by name (asl's `Invalid option`, exit 4), never
//! ignored. `X` is the input up to its first `.`, anywhere in the path, which is
//! how asl names its outputs and how `common.lua` finds them; an input whose file
//! name has no `.` is read as `<input>.asm`, as asl reads it.

use std::path::{Path, PathBuf};
use std::process;

use sigil_harness::stdout::println;

/// Whether `argv0`, the path this executable was run as, names asl: a file
/// name of `asl`, or `asl.exe` with the extension in any case. Only the file
/// name counts, so a copy or a symlink named `asl` anywhere is the drop-in, and
/// `sigil` under any other name is sigil.
pub(crate) fn invoked_as_asl(argv0: &str) -> bool {
    let Some(name) = Path::new(argv0).file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    match name.split_once('.') {
        None => name == "asl",
        Some((stem, ext)) => stem == "asl" && ext.eq_ignore_ascii_case("exe"),
    }
}

/// The options this mode accepts, as `-<name>` spellings, in the order the
/// refusal lists them. `-E`, `-i` and `-D` read a value; see [`parse`].
pub(crate) const ACCEPTED: &[&str] = &["-xx", "-n", "-q", "-A", "-L", "-U", "-E", "-i", "-c", "-D"];

/// What an asl command line asked for.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct AslArgs {
    /// The source argument as written.
    pub(crate) input: Option<String>,
    pub(crate) defines: Vec<(String, i64)>,
    pub(crate) share: bool,
    /// `-E`: `Some(None)` for the default `X.log`, `Some(Some(path))` for a named file.
    pub(crate) log: Option<Option<String>>,
    pub(crate) listing: bool,
    pub(crate) case_sensitive: bool,
    pub(crate) include_dirs: Vec<String>,
}

/// Read an asl command line. An error is the message to report, which names the
/// argument it is about.
pub(crate) fn parse(args: &[String]) -> Result<AslArgs, String> {
    let mut a = AslArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-xx" | "-n" | "-q" | "-A" => {}
            "-L" => a.listing = true,
            "-U" => a.case_sensitive = true,
            "-c" => a.share = true,
            "-E" => {
                let named = args.get(i + 1).filter(|v| !v.starts_with('-')).cloned();
                if named.is_some() {
                    i += 1;
                }
                a.log = Some(named);
            }
            "-i" | "-D" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err(format!("asl option '{arg}' needs a value after it"));
                };
                if arg == "-i" {
                    a.include_dirs.push(value.clone());
                } else {
                    a.defines.extend(sigil_frontend_as::cli_define::parse_define_arg(value)?);
                }
            }
            opt if opt.len() > 1 && opt.starts_with('-') => {
                return Err(format!(
                    "asl option '{opt}' is not one sigil implements when run as asl; it takes {}",
                    ACCEPTED.join(" ")
                ));
            }
            source => {
                if let Some(first) = &a.input {
                    return Err(format!(
                        "a second source file '{source}' after '{first}': sigil run as asl assembles one source per call"
                    ));
                }
                a.input = Some(source.to_string());
            }
        }
        i += 1;
    }
    Ok(a)
}

/// asl's output name stem for a source argument: everything before the first `.`.
pub(crate) fn output_stem(input: &str) -> &str {
    input.split_once('.').map_or(input, |(stem, _)| stem)
}

/// The file asl reads for a source argument: the argument itself, or with
/// `.asm` appended when its file name has no `.`.
pub(crate) fn source_path(input: &str) -> String {
    let has_ext = Path::new(input).file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains('.'));
    if has_ext {
        input.to_string()
    } else {
        format!("{input}.asm")
    }
}

/// The share file asl's `-c` writes, for the source argument `source` and the
/// shared symbols in the order written: a comment naming the source, one
/// `#define NAME 0xVALUE` per symbol (upper-case hex, a negative value as its
/// 64-bit two's complement), and a closing comment. Derived from the reference
/// build's own output (md5 `61e672562465725a8c102288a7da9098`, probes `sh1`,
/// `sh2`, `sh5`), and the same layout `s2disasm`'s own asl writes for `s2.h`.
pub(crate) fn share_file_text(source: &str, symbols: &[(String, i64)]) -> String {
    let mut out = format!("/* {source}-Include File for C Program */\n");
    for (name, value) in symbols {
        out.push_str(&format!("#define {name} 0x{:X}\n", *value as u64));
    }
    out.push_str("/* Ende Include File for C Program */\n");
    out
}

/// Where this run's diagnostics go: the log `-E` names, or stderr.
struct Sink {
    log: Option<String>,
    lines: Vec<String>,
}

impl Sink {
    fn push(&mut self, line: String) {
        match self.log {
            Some(_) => self.lines.push(line),
            None => eprintln!("{line}"),
        }
    }

    fn render(&mut self, diags: &[sigil_span::Diagnostic], sources: &sigil_span::SourceMap) {
        for d in diags {
            let line = match sources.label(d.primary) {
                Some(loc) => format!("{loc}: {}: {}", d.level, d.message),
                None => format!("{}: {}", d.level, d.message),
            };
            self.push(line);
        }
    }

    /// Write the collected lines to the log, or remove a log an earlier run left
    /// when there is nothing to report, as asl does: `common.lua` reads a log
    /// beside a successful object file as a report of warnings.
    fn finish(&self) {
        let Some(path) = &self.log else { return };
        if self.lines.is_empty() {
            remove_if_present(path);
        } else {
            let mut text = self.lines.join("\n");
            text.push('\n');
            if let Err(e) = super::install_artifact(path, text.as_bytes()) {
                eprintln!("error: cannot write {path}: {e}");
                for line in &self.lines {
                    eprintln!("{line}");
                }
            }
        }
    }
}

fn remove_if_present(path: &str) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("error: cannot remove {path}: {e}");
        }
    }
}

/// The exit status asl gives a command line it refuses.
const EXIT_OPTION: i32 = 4;
/// The exit status asl gives an assembly with errors.
const EXIT_ERRORS: i32 = 2;

/// Run as asl on `args` (the arguments after the program name). Never returns.
pub(crate) fn run(args: &[String]) -> ! {
    let parsed = parse(args);
    // The stem and the log are known as soon as the source argument is, so even
    // a refused command line reaches the script as an error it prints rather
    // than as a crash.
    let input = match &parsed {
        Ok(a) => a.input.clone(),
        Err(_) => args.iter().rev().find(|a| !a.starts_with('-')).cloned(),
    };
    let stem = input.as_deref().map(output_stem).map(str::to_string);
    let log_path = |a_log: &Option<Option<String>>| -> Option<String> {
        match a_log {
            Some(Some(path)) => Some(path.clone()),
            Some(None) => stem.as_ref().map(|s| format!("{s}.log")),
            None => None,
        }
    };
    let refuse = |message: String, log: Option<String>| -> ! {
        if let Some(s) = &stem {
            remove_if_present(&format!("{s}.p"));
        }
        let mut sink = Sink { log, lines: Vec::new() };
        sink.push(format!("error: {message}"));
        sink.finish();
        if sink.log.is_some() {
            eprintln!("error: {message}");
        }
        process::exit(EXIT_OPTION);
    };
    let a = match parsed {
        Ok(a) => a,
        Err(message) => {
            let log = args.iter().any(|x| x == "-E").then(|| stem.as_ref().map(|s| format!("{s}.log"))).flatten();
            refuse(message, log)
        }
    };
    let log = log_path(&a.log);
    let Some(input) = a.input.clone() else {
        refuse("no source file: sigil run as asl assembles the one source named on its command line".to_string(), log)
    };
    let stem = output_stem(&input).to_string();
    let object = format!("{stem}.p");
    // Removed first, so that whatever ends this run, a `.p` left behind is this
    // run's: the script reads its presence as success.
    remove_if_present(&object);
    if !a.case_sensitive {
        refuse(
            "asl without -U folds the case of symbols, and sigil's symbols are always case-sensitive, \
             so the two would read `Foo` and `foo` differently; pass -U, which is what sigil does"
                .to_string(),
            log,
        );
    }
    let source = source_path(&input);
    let source_dir = match Path::new(&source).parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    for dir in &a.include_dirs {
        let same = match (std::fs::canonicalize(dir), std::fs::canonicalize(&source_dir)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        };
        if !same {
            refuse(
                format!(
                    "-i {dir}: sigil resolves `include` against the source file's directory ({}) and searches no \
                     other, so it cannot honour an include directory that is not that one",
                    source_dir.display()
                ),
                log,
            );
        }
    }
    if a.listing {
        remove_if_present(&format!("{stem}.lst"));
    }

    let mut sink = Sink { log, lines: Vec::new() };
    let fail = |sink: &mut Sink, shown: super::Shown, stopped_at: super::Stage| -> ! {
        if let Some(rest) = stopped_at.stages_not_run() {
            sink.push(format!(
                "this error list may be incomplete: sigil stopped at {}, so {rest} did not run",
                stopped_at.name()
            ));
        }
        sink.push(super::failure_line(shown).replace(" (reported on stderr)", ""));
        sink.finish();
        process::exit(EXIT_ERRORS);
    };

    let opts = sigil_frontend_as::Options { cli_defines: a.defines.clone(), share_file: a.share, ..Default::default() };
    let assembled = match sigil_frontend_as::assemble_root_located_warned(Path::new(&source), &opts) {
        Ok(assembled) => assembled,
        Err(failure) => {
            for m in &failure.messages {
                println!("{m}");
            }
            sink.render(&failure.diags, &failure.sources);
            fail(&mut sink, super::Shown::default().plus(&failure.diags), super::Stage::Frontend);
        }
    };
    for m in &assembled.messages {
        println!("{m}");
    }
    sink.render(&assembled.warnings, &assembled.sources);
    let shown = super::Shown::default().plus(&assembled.warnings);
    let sources = &assembled.sources;

    // p2bin, not sigil, places each second address space, from the `-z` the
    // script hands it, so none is refused here for want of a placement.
    let placed: Vec<u32> = assembled
        .module
        .sections
        .iter()
        .filter(|s| matches!(s.space, sigil_ir::AddressSpace::Foreign { cpu: sigil_ir::Cpu::Z80, .. }))
        .map(|s| s.lma)
        .collect();
    let empty = sigil_ir::SymbolTable::new();
    let resolved = match sigil_link::resolve_layout_placing(&assembled.module.sections, &empty, true, &placed) {
        Ok(secs) => secs,
        Err(diags) => {
            sink.render(&diags, sources);
            fail(&mut sink, shown.plus(&diags), super::Stage::Layout);
        }
    };
    let linked = match sigil_link::link(&resolved, &empty) {
        Ok(img) => img,
        Err(diags) => {
            sink.render(&diags, sources);
            fail(&mut sink, shown.plus(&diags), super::Stage::Link);
        }
    };
    let bounds = sigil_link::check_image_bounds(&linked, &resolved);
    if !bounds.is_empty() {
        sink.render(&bounds, sources);
        fail(&mut sink, shown.plus(&bounds), super::Stage::Image);
    }
    let records = sigil_link::code_file_records(&resolved, &linked);

    // The share file's values: a symbol the front end could not fold (a label
    // kept symbolic for the linker) is folded against the linked program.
    let mut shared: Vec<(String, i64)> = Vec::new();
    if a.share {
        let table: std::collections::HashMap<String, i64> =
            sigil_link::resolved_symbols(&resolved, &empty).into_iter().collect();
        let mut unresolved = Vec::new();
        for s in &assembled.shared {
            match s.value.fold(&|n| table.get(n).copied()) {
                sigil_ir::expr::Fold::Value(v) => shared.push((s.name.clone(), v)),
                _ => unresolved.push(sigil_span::Diagnostic {
                    level: sigil_span::Level::Error,
                    message: format!("`shared {}`: no value after linking", s.name),
                    primary: s.span,
                }),
            }
        }
        if !unresolved.is_empty() {
            sink.render(&unresolved, sources);
            fail(&mut sink, shown.plus(&unresolved), super::Stage::Image);
        }
    }

    // The object file is written LAST, so its presence means every output of
    // this run is on disk.
    let write = |sink: &mut Sink, path: &str, bytes: &[u8]| {
        if let Err(e) = super::install_artifact(path, bytes) {
            sink.push(format!("error: cannot write {path}: {e}"));
            fail(sink, shown.plus_printed_error(), super::Stage::Image);
        }
    };
    if a.share {
        write(&mut sink, &format!("{stem}.h"), share_file_text(&source, &shared).as_bytes());
    }
    let creator = format!("sigil {} ({}) as asl", env!("CARGO_PKG_VERSION"), env!("SIGIL_REVISION_SHORT"));
    write(&mut sink, &object, &sigil_link::encode_code_file(&records, &creator));
    sink.finish();
    process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn the_program_name_selects_the_mode() {
        for yes in ["asl", "/x/build_tools/Linux-x86_64/asl", "asl.exe", "C:\\t\\asl.EXE", "./asl.Exe"] {
            let yes = yes.replace('\\', "/");
            assert!(invoked_as_asl(&yes), "{yes}");
        }
        for no in ["sigil", "/x/sigil", "ASL", "asl.bin", "asl.exe.bak", "xasl", "aslx", "asl2", ""] {
            assert!(!invoked_as_asl(no), "{no}");
        }
    }

    #[test]
    fn the_scripts_command_line_parses() {
        let a = parse(&args("-xx -n -q -A -L -U -E -i . -c -D Sonic3_Complete=1 sonic3k.asm")).unwrap();
        assert_eq!(a.input.as_deref(), Some("sonic3k.asm"));
        assert_eq!(a.defines, vec![("Sonic3_Complete".to_string(), 1)]);
        assert!(a.share && a.listing && a.case_sensitive);
        assert_eq!(a.log, Some(None));
        assert_eq!(a.include_dirs, vec![".".to_string()]);
    }

    #[test]
    fn e_takes_a_following_file_name_as_asl_does() {
        let a = parse(&args("-U -E my.log x.asm")).unwrap();
        assert_eq!(a.log, Some(Some("my.log".to_string())));
        assert_eq!(a.input.as_deref(), Some("x.asm"));
    }

    #[test]
    fn every_other_option_is_refused_by_name() {
        for opt in ["-x", "-o", "-P", "-g", "-h", "-cpu", "-Z", "-DFOO", "--help", "-olist"] {
            let e = parse(&args(&format!("-U {opt} x.asm"))).unwrap_err();
            assert!(e.contains(&format!("'{opt}'")), "{opt}: {e}");
        }
    }

    #[test]
    fn a_second_source_is_refused() {
        let e = parse(&args("-U a.asm b.asm")).unwrap_err();
        assert!(e.contains("'b.asm'"), "{e}");
    }

    #[test]
    fn outputs_are_named_as_asl_names_them() {
        assert_eq!(output_stem("sonic.asm"), "sonic");
        assert_eq!(output_stem("sh3.x.asm"), "sh3");
        assert_eq!(output_stem("d.x/f.asm"), "d");
        assert_eq!(output_stem("./sub/in.asm"), "");
        assert_eq!(source_path("nolog"), "nolog.asm");
        assert_eq!(source_path("sonic.asm"), "sonic.asm");
    }

    #[test]
    fn the_share_file_is_asls_layout() {
        let text = share_file_text(
            "sh2.asm",
            &[("zeta".into(), 0xABCDEF), ("Neg".into(), -2), ("Alpha".into(), 0), ("Big".into(), 0xFFFF_FFFF)],
        );
        assert_eq!(
            text,
            "/* sh2.asm-Include File for C Program */\n\
             #define zeta 0xABCDEF\n\
             #define Neg 0xFFFFFFFFFFFFFFFE\n\
             #define Alpha 0x0\n\
             #define Big 0xFFFFFFFF\n\
             /* Ende Include File for C Program */\n"
        );
    }
}
