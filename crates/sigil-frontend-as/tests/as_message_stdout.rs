//! The `message` directive: a STREAM of its own, carried out of the front end
//! as `Assembled::messages` / `Failure::messages`, one line per firing, from
//! the CONVERGED pass only.
//!
//! asl writes a `message` to stdout, unprefixed and outside the diagnostic
//! stream, once per pass that reaches it (reference build md5
//! `61e672562465725a8c102288a7da9098`, probe `p4`: `fwd 0` on pass 1 and
//! `fwd 3` on pass 2 for one `message "fwd \{Later}"` above `Later:`). Sigil
//! runs a different number of passes than asl (4 to 5 evaluations where asl
//! runs 2, measured on all three corpora), so printing per pass would print
//! asl's pass count rather than the program's text. One line per firing on
//! the pass whose env is final, interpolated against that env, is what every
//! reader of the line wants: the value the program ends up with, once.
//!
//! Both corpus sites that print today are guarded by `if MOMPASS=2`
//! (`sound/z80.asm(231)` in s1disasm, `s2.asm(91272)` in s2disasm), so the
//! converged pass is the only pass whose line they intend anyway.
//!
//! Before this file existed `message` evaluated its string and discarded it on
//! every pass, and `a_message_is_dropped_on_every_pass_including_the_final_one`
//! pinned that as booked. That test was deleted on purpose when this landed.

use sigil_frontend_as::Options;

/// Assemble through the file entry point and hand back the linked bytes with
/// the message stream on success, or the rendered diagnostics with the message
/// stream on failure. Both arms carry the stream because asl prints a
/// `message` when it is reached whether or not the run later fails.
#[allow(clippy::type_complexity)]
fn assemble_messages(src: &str) -> Result<(Vec<u8>, Vec<String>, Vec<String>), (Vec<String>, Vec<String>)> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, src).expect("write probe");
    let render = |sources: &sigil_span::SourceMap, diags: &[sigil_span::Diagnostic]| {
        diags
            .iter()
            .map(|d| match sources.label(d.primary) {
                Some(l) => format!("{l}: {}", d.message),
                None => d.message.clone(),
            })
            .collect::<Vec<_>>()
    };
    match sigil_frontend_as::assemble_root_located_warned(&path, &Options::default()) {
        Ok(a) => {
            let warnings = render(&a.sources, &a.warnings);
            let resolved = sigil_link::resolve_layout(
                &a.module.sections,
                &sigil_ir::SymbolTable::new(),
                true,
            )
            .expect("resolve_layout");
            let linked =
                sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok((sigil_link::flatten(&linked, 0x00).unwrap(), a.messages, warnings))
        }
        Err(f) => Err((render(&f.sources, &f.diags), f.messages)),
    }
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const FWD: &str = "\tdc.w Later-*\nLater:\n";

/// The converged-pass rule, on the one input that distinguishes it: a
/// `message` whose text depends on a symbol defined BELOW it. asl prints two
/// lines for this program, `fwd 0` then `fwd 3`; the returned stream carries
/// exactly the second, and nothing else.
#[test]
fn a_message_prints_once_with_the_converged_value_of_a_forward_reference() {
    let src = format!("{HEAD}\tmessage \"fwd \\{{Later}}\"\n\tdc.b $11\n{FWD}\tend\n");
    let (bytes, messages, warnings) = assemble_messages(&src).expect("a message does not stop assembly");
    assert_eq!(bytes, vec![0x11, 0x00, 0x02], "a message emits no bytes");
    assert_eq!(messages, vec!["fwd 3".to_string()], "once, with the final value");
    assert!(warnings.is_empty(), "a message is not a diagnostic: {warnings:?}");
}

/// A `message` reached only on the FIRST pass is not the program's final word
/// on anything and does not print. This is the deliberate difference from asl,
/// which prints it (probe `cw1` shape), pinned so it reads as a decision.
#[test]
fn a_message_on_the_first_pass_only_is_not_printed() {
    let src = format!(
        "{HEAD}\tif MOMPASS=1\n\tmessage \"first-iteration message\"\n\tendif\n\tdc.b $11\n{FWD}\tend\n"
    );
    let (_, messages, _) = assemble_messages(&src).expect("a message does not stop assembly");
    assert!(messages.is_empty(), "nothing from a non-final pass: {messages:?}");
}

/// The corpus guard, `if MOMPASS=2`: the site fires on every sigil pass after
/// the first and prints exactly once.
#[test]
fn a_message_guarded_to_later_passes_prints_once() {
    let src = format!(
        "{HEAD}\tif MOMPASS=2\n\tmessage \"later-pass message\"\n\tendif\n\tdc.b $11\n{FWD}\tend\n"
    );
    let (_, messages, _) = assemble_messages(&src).expect("a message does not stop assembly");
    assert_eq!(messages, vec!["later-pass message".to_string()]);
}

/// Per FIRING, not per site: a `message` in a macro expanded three times is
/// three lines, which is what asl prints and what a reader counting entries
/// expects.
#[test]
fn a_message_in_a_macro_prints_once_per_expansion() {
    let src = format!(
        "{HEAD}say macro v\n\tmessage \"got \\{{v}}\"\n\tendm\n\tsay 1\n\tsay 2\n\tsay 3\n\tdc.b $11\n\tend\n"
    );
    let (_, messages, _) = assemble_messages(&src).expect("a message does not stop assembly");
    assert_eq!(messages, vec!["got 1".to_string(), "got 2".to_string(), "got 3".to_string()]);
}

/// s1disasm's shape: the driver-size `message` prints and the run then fails
/// on an unrelated line. The stream rides `Failure` as well as `Assembled`.
#[test]
fn a_failing_run_still_returns_the_messages_of_its_final_pass() {
    let src = format!(
        "{HEAD}\tmessage \"size \\{{Later}}h bytes\"\n\tdc.b $11\n{FWD}\terror \"an unrelated failure\"\n\tend\n"
    );
    let (diags, messages) = assemble_messages(&src).expect_err("the `error` fails the run");
    assert!(
        diags.iter().any(|d| d.contains("an unrelated failure")),
        "the failure is the `error` line: {diags:?}"
    );
    assert_eq!(messages, vec!["size 3h bytes".to_string()]);
}

/// The text goes through the same interpolation as `warning`, float branch
/// included: this is the corpus line that was uninterpolated before the float
/// branch existed.
#[test]
fn a_message_interpolates_a_parenthesised_float_division() {
    let src = format!(
        "{HEAD}StartOfRom:\n\tdc.b 1\n\tdc.w EndOfRom-*\npaddingSoFar equ 3\n\
         \tmessage \"ROM size is $\\{{EndOfRom-StartOfRom}} bytes (\\{{(EndOfRom-StartOfRom)/1024.0}} KiB). About $\\{{paddingSoFar}} bytes are padding. \"\n\
         \tdc.b 5,6\nEndOfRom:\n\tend\n"
    );
    let (_, messages, _) = assemble_messages(&src).expect("a message does not stop assembly");
    assert_eq!(
        messages,
        vec!["ROM size is $5 bytes (0.0048828125 KiB). About $3 bytes are padding. ".to_string()]
    );
}

/// A `message` inside an `include`d file is part of the same stream, in
/// execution order with the root's.
#[test]
fn messages_from_an_included_file_keep_execution_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("inc")).expect("mkdir");
    std::fs::write(dir.path().join("inc/a.asm"), "\tmessage \"from the include\"\n").expect("write");
    let root = format!(
        "{HEAD}\tmessage \"before\"\n\tinclude \"inc/a.asm\"\n\tmessage \"after\"\n\tdc.b $11\n\tend\n"
    );
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, root).expect("write");
    let a = sigil_frontend_as::assemble_root_located_warned(&path, &Options::default())
        .unwrap_or_else(|f| panic!("{:?}", f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()));
    assert_eq!(
        a.messages,
        vec!["before".to_string(), "from the include".to_string(), "after".to_string()]
    );
}
