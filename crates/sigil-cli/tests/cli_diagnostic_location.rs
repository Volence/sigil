//! Every diagnostic the `sigil <file.asm>` path prints must say WHERE — and for an
//! error inside an `include`d file, "where" is that file and its own line number,
//! not the includer's.
//!
//! ## Why this has its own gate
//!
//! A diagnostic with no location is a trust failure before it is an inconvenience:
//! against the community Sonic 2 disassembly (91,276 lines spliced from 332 files)
//! sigil reported 237 diagnostics and not one of them named a file or a line, while
//! AS — the assembler sigil proposes to replace — reports
//! `smps-bug.asm(9): error: StartOffset is located after EndOffset somehow!`.
//! Being behind the incumbent on "where is my mistake" is an adoption verdict.
//!
//! The half that is easy to get wrong is the include. `include` splices a file's
//! lines into the running assembly, and the naive implementation lets those lines
//! keep the includer's identity: the offsets then resolve against the wrong text, so
//! the report names the top-level file and a line that has nothing to do with the
//! mistake — or no line at all when the offset runs past the includer's end. The
//! fixture below is built so that failure mode cannot pass: the included file's error
//! is on line 4, and the root file is only 3 lines long, so an includer-attributed
//! report is BOTH wrong and detectably out of range.
//!
//! The assertions are about the CLI PROCESS rather than about the front-end library.
//! `Diagnostic::primary` always carried a span and `SourceMap::location` always
//! resolved one; what was missing was a map with the spliced files in it and a
//! renderer that used it, and only running the shipped command exercises both.

use std::process::Command;

/// A root that includes a file, with a distinct error in EACH — every diagnostic
/// names the file that actually contains it.
#[test]
fn a_diagnostic_names_its_own_file_and_line_across_an_include() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path();

    // part.asm: the error is on line 4 — past the end of root.asm, so attributing it
    // to the includer cannot accidentally land on a plausible line.
    std::fs::write(
        src.join("part.asm"),
        "; line 1\n; line 2\n; line 3\n\tnotamnemonic_in_part\n",
    )
    .expect("write part.asm");
    // root.asm: 3 lines, its own error on line 3.
    std::fs::write(
        src.join("root.asm"),
        "\tcpu 68000\n\tinclude \"part.asm\"\n\tnotamnemonic_in_root\n",
    )
    .expect("write root.asm");

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(src.join("root.asm"))
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Vacuity control: both mistakes must actually have been diagnosed, or the
    // location assertions below would pass over an empty report.
    assert!(
        !out.status.success(),
        "a source with two unrecognized mnemonics must fail.\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("notamnemonic_in_part") && stderr.contains("notamnemonic_in_root"),
        "both errors must be reported at all.\nstderr:\n{stderr}"
    );

    // The included file's error names the INCLUDED file, at its own line 4.
    let part_at_4 = format!("{}(4): error: ", src.join("part.asm").display());
    assert!(
        stderr.contains(&part_at_4),
        "an error inside an included file must be reported as `{part_at_4}…`.\nstderr:\n{stderr}"
    );

    // The root file's error names the ROOT file, at its own line 3.
    let root_at_3 = format!("{}(3): error: ", src.join("root.asm").display());
    assert!(
        stderr.contains(&root_at_3),
        "an error in the root file must be reported as `{root_at_3}…`.\nstderr:\n{stderr}"
    );

    // The included file's error must not be laid at the includer's door under ANY
    // line number: root.asm has three lines, and none of them is this mistake.
    let root_name = src.join("root.asm").display().to_string();
    for line in stderr.lines() {
        if line.contains("notamnemonic_in_part") {
            assert!(
                !line.starts_with(&root_name),
                "the included file's error was attributed to the includer: {line}"
            );
        }
    }
}

/// A macro body executes wherever it is called, but its text lives where it was
/// written — and that is the file the report names.
///
/// This is the reason each source line carries its own file rather than the
/// assembler carrying "the file currently being executed": a macro defined in an
/// included file and expanded in the root would otherwise be reported against the
/// root, at an offset into text that never contained the mistake. The line a reader
/// has to edit is the macro body's.
#[test]
fn an_error_in_a_macro_body_names_the_file_the_body_was_written_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path();

    // The bad line is line 3 of the definition file; the call is line 3 of the root.
    std::fs::write(
        src.join("mac.inc"),
        "; line 1\nmymac macro\n\tnotamnemonic_in_macro_body\n\tendm\n",
    )
    .expect("write mac.inc");
    std::fs::write(
        src.join("mroot.asm"),
        "\tcpu 68000\n\tinclude \"mac.inc\"\n\tmymac\n",
    )
    .expect("write mroot.asm");

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(src.join("mroot.asm"))
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        stderr.contains("notamnemonic_in_macro_body"),
        "the macro must have expanded and its bad line diagnosed, or this test is \
         vacuous.\nstderr:\n{stderr}"
    );
    let body_at_3 = format!("{}(3): error: ", src.join("mac.inc").display());
    assert!(
        stderr.contains(&body_at_3),
        "a macro body's error must be reported as `{body_at_3}…` — the file the body \
         was written in.\nstderr:\n{stderr}"
    );
}

/// Every line of a failing report carries a location — the property the corpus run
/// measures, asserted here on a case small enough to read.
#[test]
fn every_diagnostic_line_carries_a_file_and_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path();
    std::fs::write(src.join("inner.asm"), "\tbogus_one\n\tbogus_two\n").expect("write inner.asm");
    std::fs::write(
        src.join("outer.asm"),
        "\tcpu 68000\n\tinclude \"inner.asm\"\n\tbogus_three\n",
    )
    .expect("write outer.asm");

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(src.join("outer.asm"))
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&out.stderr);

    let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        3,
        "expected one diagnostic per bogus mnemonic.\nstderr:\n{stderr}"
    );
    for line in &lines {
        // `<path>(<line>): error: <message>` — the shape AS itself prints.
        let head = line.split(": error: ").next().unwrap_or("");
        assert!(
            head.ends_with(')') && head.contains(".asm("),
            "diagnostic carries no `file(line)` location: {line}"
        );
    }
}
