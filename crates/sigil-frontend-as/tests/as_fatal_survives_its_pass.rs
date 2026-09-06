//! A `fatal`, and a `warning` the source author wrote, raised on any pass reach
//! the returned diagnostics. A `message` reaches nothing, on any pass.
//!
//! ## What was silently wrong
//!
//! ```text
//!     if MOMPASS=1
//!     fatal "first-iteration problem"
//!     endif
//!     dc.b $11
//! ```
//!
//! | assembler | result |
//! |---|---|
//! | asl (md5 `61e672562465725a8c102288a7da9098`) | **exit 3**, assembly terminated |
//! | sigil before `MOMPASS` existed | exit 1, refuses the unresolved condition |
//! | sigil with `MOMPASS` and without this | **exit 0, no output at all** |
//!
//! The strongest refusal the language has, the author's own way of saying stop
//! the build and tell the user, was the quietest thing in the assembler. The
//! `fatal` set `aborted`, that pass's diagnostics were dropped as superseded by
//! a later pass, and the later pass did not raise it.
//!
//! ## Why `fatal` is separable from the general discard
//!
//! Returning only the CONVERGED pass's diagnostics is right for every other
//! diagnostic, and deliberately so: a name unresolved on iteration 1 has a
//! value by iteration 2, and reporting iteration 1's complaint would be
//! reporting a forward reference as an error. That reasoning is about a later
//! pass SUPERSEDING an earlier one, and `fatal` is the one directive for which
//! there is no later pass: asl prints it and terminates. So it is not an
//! exception carved out of a rule, it is a case the rule never covered.
//!
//! It is also mechanically separable, which `aborted` is not. `aborted` is set
//! by `fatal`, by `end`, by include-nesting overflow, by the `while` budget and
//! by the undeclared-processor refusal, and `end` sits at the bottom of every
//! well-formed file in all three corpora. A fix keyed on `aborted` would fire
//! on every one of them.
//!
//! ## Why it CARRIES rather than terminating
//!
//! asl literally terminates, and terminating was tried first and measured
//! rather than reasoned about. It throws away everything the run had already
//! found: on s1disasm it cut 50 located diagnostics to 1, on skdisasm 2132 to
//! 1, and in both cases the single survivor was a line the run already
//! reported. Carrying is strictly additive, so it can make a run louder and
//! never quieter. Over all six roots there are (s2disasm, s1disasm, skdisasm
//! and aeon's three AS roots) it changes stdout and stderr not at all.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble a root that may `include` siblings, and hand back the linked bytes
/// or the rendered `file(line): message` diagnostics.
///
/// The diagnostics are rendered through the same `SourceMap::label` the CLI
/// uses, because half of what these tests check is WHICH FILE a carried
/// diagnostic names.
fn assemble_tree(files: &[(&str, &str)]) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, body).expect("write");
    }
    let root = dir.path().join(files[0].0);
    match assemble_root_located(&root, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00))
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| match f.sources.label(d.primary) {
                Some(l) => format!("{l}: {}", d.message),
                None => d.message.clone(),
            })
            .collect()),
    }
}

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    assemble_tree(&[("probe.asm", body)])
}

/// Like [`assemble_tree`] but keeps the WARNINGS a successful assembly returned,
/// rendered through the same `SourceMap::label`. A run that carries a warning
/// still succeeds, so the plain helper's `Ok(bytes)` throws away the half these
/// tests are about.
fn assemble_tree_located(files: &[(&str, &str)]) -> Result<(Vec<u8>, Vec<String>), Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, body).expect("write");
    }
    let root = dir.path().join(files[0].0);
    let render = |sources: &sigil_span::SourceMap, diags: &[sigil_span::Diagnostic]| {
        diags
            .iter()
            .map(|d| match sources.label(d.primary) {
                Some(l) => format!("{l}: {}", d.message),
                None => d.message.clone(),
            })
            .collect::<Vec<_>>()
    };
    match sigil_frontend_as::assemble_root_located_warned(&root, &Options::default()) {
        Ok(a) => {
            let warnings = render(&a.sources, &a.warnings);
            let m = a.module;
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok((sigil_link::flatten(&linked, 0x00), warnings))
        }
        Err(f) => Err(render(&f.sources, &f.diags)),
    }
}

fn assemble_located(body: &str) -> Result<(Vec<u8>, Vec<String>), Vec<String>> {
    assemble_tree_located(&[("probe.asm", body)])
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const FWD: &str = "\tdc.w Later-*\nLater:\n";

/// The shape itself. asl, probe `co_fatal`, **exit 3**: it prints the author's
/// text and terminates the assembly. sigil must refuse too, and must name the
/// line the `fatal` is written on.
#[test]
fn a_fatal_on_a_non_final_pass_is_reported() {
    let src =
        format!("{HEAD}\tif MOMPASS=1\n\tfatal \"first-iteration problem\"\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    let diags = assemble(&src).expect_err("a fatal must not assemble to bytes");
    assert!(
        diags.iter().any(|d| d.contains("first-iteration problem")),
        "the author's own text must survive its pass: {diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.contains("probe.asm(5): first-iteration problem")),
        "and must name the line it is written on: {diags:?}"
    );
}

/// The control that says the shape above is about the PASS and not about
/// `fatal` being broken generally: the same `fatal`, unguarded, was already
/// reported before this change and still is. asl, probe `co_fatal_plain`,
/// exit 3.
#[test]
fn an_unguarded_fatal_is_still_reported() {
    let src = format!("{HEAD}\tfatal \"plain\"\n\tdc.b $11\n\tend\n");
    let diags = assemble(&src).expect_err("a fatal must not assemble to bytes");
    assert!(
        diags.iter().any(|d| d.contains("probe.asm(4): plain")),
        "{diags:?}"
    );
}

/// The other control. `if MOMPASS>1` is TRUE on the pass that returns, so this
/// `fatal` was never dropped and needs no carrying. It is here so that a change
/// which broke the ordinary path could not pass by fixing only the carried one.
/// asl, probe `co_fatal_gt`, exit 3.
#[test]
fn a_fatal_on_the_final_pass_is_reported_once() {
    let src =
        format!("{HEAD}\tif MOMPASS>1\n\tfatal \"later-pass problem\"\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    let diags = assemble(&src).expect_err("a fatal must not assemble to bytes");
    let hits = diags
        .iter()
        .filter(|d| d.contains("later-pass problem"))
        .count();
    assert_eq!(hits, 1, "carried once, not twice: {diags:?}");
}

/// A carried `fatal` must not be MISATTRIBUTED, which is worse than being bare.
///
/// The source map is rebuilt every pass and ids are handed out in splice order,
/// so a file spliced only on the raising pass leaves every later id shifted.
/// Carrying the bare span reported this `fatal`, written in `inc/c.asm`, as
/// `inc/b.asm(1)`: a real file, a real line, and the wrong one. The label is
/// therefore captured against the raising pass's own map, and used when the
/// returning map disagrees.
///
/// asl, probe `co_map2`, exit 3.
#[test]
fn a_carried_fatal_names_the_file_it_was_written_in() {
    let root = format!(
        "{HEAD}\tif MOMPASS=1\n\tinclude \"inc/c.asm\"\n\tendif\n\
         \tinclude \"inc/b.asm\"\n\tinclude \"inc/d.asm\"\n\
         \tinclude \"inc/e.asm\"\n\tinclude \"inc/f.asm\"\n\tdc.b $11\n{FWD}\tend\n"
    );
    let diags = assemble_tree(&[
        ("probe.asm", &root),
        ("inc/c.asm", "\tfatal \"fatal from inside an included file\"\n"),
        ("inc/b.asm", "\tdc.b $02\n"),
        ("inc/d.asm", "\tdc.b $03\n"),
        ("inc/e.asm", "\tdc.b $04\n"),
        ("inc/f.asm", "\tdc.b $05\n"),
    ])
    .expect_err("a fatal must not assemble to bytes");
    let row = diags
        .iter()
        .find(|d| d.contains("fatal from inside an included file"))
        .unwrap_or_else(|| panic!("the fatal was lost: {diags:?}"));
    assert!(
        row.contains("inc/c.asm(1)"),
        "must name the file it is written in: {row}"
    );
    assert!(
        !row.contains("inc/b.asm"),
        "must not name the file that inherited its source id: {row}"
    );
}

/// The `warning` DIRECTIVE carries too, and the argument that once separated it
/// from `fatal` was refuted by measurement rather than argued away.
///
/// That argument was: asl treats a `warning` as a diagnostic and keeps
/// assembling, so a later pass genuinely supersedes it. It does not. asl prints
/// a `warning` and never retracts it, and its output over a run is the UNION
/// over passes rather than the last pass's view. Probe `cw1`, asl md5
/// `61e672562465725a8c102288a7da9098`, exit 0, listing footer `2 passes`, no
/// `Additional necessary passes` line:
///
/// ```text
///     > > > cw1.asm(5): warning: canary warning first pass only
/// ```
///
/// sigil printed nothing at all for the same source. So the shape is `fatal`'s
/// shape: an author wrote a line in order to be told something, and the
/// assembler dropped it because the pass it fired on was not the one that
/// returned.
///
/// Carried ONE line per source position rather than one per firing, which is
/// where sigil deliberately differs from asl: asl printed the unguarded probe
/// `cw3` TWICE, once per pass, and sigil prints it once. That multiplicity is a
/// property of how many passes asl happened to run (asl runs 2 over every
/// corpus root; sigil runs 4 to 5 evaluations), so reproducing it would
/// reproduce an artifact of asl's pass loop rather than anything about the
/// program.
///
/// Corpus delta: ZERO. Across s2disasm, s1disasm and skdisasm the `warning`
/// directive fires on no pass at all (13 source sites, 0 firings over 12 pass
/// evaluations), and asl prints no author warning over the two of those it
/// assembles cleanly. Aeon names `warning` in no `.asm` or `.inc` file.
#[test]
fn a_warning_on_a_non_final_pass_is_reported() {
    let src = format!(
        "{HEAD}\tif MOMPASS=1\n\twarning \"first-iteration warning\"\n\tendif\n\tdc.b $11\n{FWD}\tend\n"
    );
    let a = assemble_located(&src).expect("a warning does not stop assembly");
    assert_eq!(a.0, vec![0x11, 0x00, 0x02], "and it emits no bytes of its own");
    assert!(
        a.1.iter()
            .any(|d| d.contains("probe.asm(5): [as.warning] first-iteration warning")),
        "the author's own text must survive its pass, naming its own line: {:?}",
        a.1
    );
}

/// The control that says the test above is about the PASS and not about the
/// directive being broken generally: a `warning` on the pass that returns was
/// always reported, still is, and is reported exactly ONCE rather than once per
/// pass it fired on. asl, probe `cw2`, exit 0, prints it once.
#[test]
fn a_warning_on_the_final_pass_is_reported_once() {
    let src = format!(
        "{HEAD}\tif MOMPASS>1\n\twarning \"later-pass warning\"\n\tendif\n\tdc.b $11\n{FWD}\tend\n"
    );
    let a = assemble_located(&src).expect("a warning does not stop assembly");
    let hits = a.1.iter().filter(|d| d.contains("later-pass warning")).count();
    assert_eq!(hits, 1, "once, not once per pass: {:?}", a.1);
}

/// The unguarded shape, which fires on EVERY pass. asl prints it once per pass
/// (probe `cw3`: two identical lines, footer `2 passes`); sigil prints one line
/// per source position. This pins the deliberate difference so a future change
/// that started emitting one per firing would be caught.
#[test]
fn a_warning_on_every_pass_is_reported_once() {
    let src = format!("{HEAD}\twarning \"every-pass warning\"\n\tdc.b $11\n{FWD}\tend\n");
    let a = assemble_located(&src).expect("a warning does not stop assembly");
    let hits = a.1.iter().filter(|d| d.contains("every-pass warning")).count();
    assert_eq!(hits, 1, "one line per site, not one per pass: {:?}", a.1);
}

/// A carried `warning` must not be MISATTRIBUTED, for the same reason a carried
/// `fatal` must not: the source map is rebuilt every pass and ids are handed out
/// in splice order, so a file spliced only on the raising pass leaves every
/// later id shifted, and a bare span renders to a real file and the wrong line.
#[test]
fn a_carried_warning_names_the_file_it_was_written_in() {
    let root = format!(
        "{HEAD}\tif MOMPASS=1\n\tinclude \"inc/c.asm\"\n\tendif\n\
         \tinclude \"inc/b.asm\"\n\tinclude \"inc/d.asm\"\n\
         \tinclude \"inc/e.asm\"\n\tinclude \"inc/f.asm\"\n\tdc.b $11\n{FWD}\tend\n"
    );
    let a = assemble_tree_located(&[
        ("probe.asm", &root),
        ("inc/c.asm", "\twarning \"warning from inside an included file\"\n"),
        ("inc/b.asm", "\tdc.b $02\n"),
        ("inc/d.asm", "\tdc.b $03\n"),
        ("inc/e.asm", "\tdc.b $04\n"),
        ("inc/f.asm", "\tdc.b $05\n"),
    ])
    .expect("a warning does not stop assembly");
    let row = a
        .1
        .iter()
        .find(|d| d.contains("warning from inside an included file"))
        .unwrap_or_else(|| panic!("the warning was lost: {:?}", a.1));
    assert!(
        row.contains("inc/c.asm(1)"),
        "must name the file it is written in: {row}"
    );
    assert!(
        !row.contains("inc/b.asm"),
        "must not name the file that inherited its source id: {row}"
    );
}

/// `message` is NOT the same case, and the row that treats it with `warning`
/// is wrong about it. sigil's `message` evaluates its string and discards it on
/// EVERY pass, the converged one included, so no pass-carrying rule reaches it.
/// asl writes it to STDOUT, unprefixed and outside the diagnostic stream, and
/// two corpus sites print there today while sigil prints nothing:
///
/// | corpus | asl stdout | sigil |
/// |---|---|---|
/// | s1disasm | `Uncompressed driver size: 1BC6h bytes.` | nothing |
/// | s2disasm | `ROM size is $100000 bytes (1024 KiB). About $8F1 bytes are padding.` | nothing |
///
/// Pinned here so the gap reads as booked rather than as an oversight, and so
/// that implementing `message` (a new output stream, not a pass question) has
/// to come here and delete this test on purpose.
#[test]
fn a_message_is_dropped_on_every_pass_including_the_final_one() {
    let src = format!("{HEAD}\tmessage \"unguarded message\"\n\tdc.b $11\n{FWD}\tend\n");
    let a = assemble_located(&src).expect("a message does not stop assembly");
    assert_eq!(a.0, vec![0x11, 0x00, 0x02]);
    assert!(
        !a.1.iter().any(|d| d.contains("unguarded message")),
        "sigil emits nothing for `message` on any pass: {:?}",
        a.1
    );
}
