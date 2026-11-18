//! G0a Task 7: `.emp` template expansions. Bytes a template splice put in the stream
//! belong to the line that SPLICED them, not to the template body they were written in
//! (spec 4.4, "Adopted: give the `.emp` evaluator expansions in the same model"). The
//! evaluator records one `Frame::Template` expansion per splice run and rewrites the
//! spliced items' spans to it, so the export's one walker (`walk_lines` through
//! `ChainSources`) follows `Expansion.call` out to the splice, exactly as it does for an
//! AS macro.
//!
//! The fixtures are built in process through the whole-program driver
//! (`build_program_open_embed_counted`, the one `sigil build` lowers through), joined by
//! `ChainSources`, and walked by `walk_lines`: the same seam the build export uses, on
//! Sigil's own fixtures (no aeon tree). Expected lines are found in the fixture text by
//! a marker comment, never pinned as numbers.
//!
//! `splice_site_census_is_complete` is the census gate: every place `eval/asm.rs` puts
//! spliced Code into a stream is a listed splice site, so a new one cannot ship without
//! a decision about its attribution.

use sigil_frontend_emp::resolve::manifest::{Manifest, SourceIndex};
use sigil_frontend_emp::resolve::{build_program_open_embed_counted, BuiltProgram};
use sigil_harness::native::ChainSources;
use sigil_link::export::locate::SpanLocator;
use sigil_link::export::{walk_lines, RowKind};
use sigil_span::Level;
use std::path::Path;

/// One located byte run: `[lma, lma+len)` on `line` of the fixture.
#[derive(Debug)]
struct Row {
    lma: u32,
    len: u32,
    line: u32,
    kind: RowKind,
}

/// The 1-based line of the fixture holding `marker` (a `// @name` comment).
fn line_of(text: &str, marker: &str) -> u32 {
    let at = text.find(marker).unwrap_or_else(|| panic!("fixture lacks `{marker}`"));
    text[..at].matches('\n').count() as u32 + 1
}

/// Build module `m` (one file, `text`) through the whole-program driver and walk its
/// line table. Returns the rows plus the build (for its expansions and sections).
fn build(text: &str) -> (Vec<Row>, BuiltProgram, ChainSources) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("m.emp"), text).unwrap();
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(mdiags.iter().all(|d| d.level != Level::Error), "scan: {mdiags:?}");
    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::backend::Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: Vec::new(),
    };
    let mut built = build_program_open_embed_counted(&manifest, "m", None, &opts, &|_| None);
    let errs: Vec<_> = built.diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "build: {errs:?}");
    let index = SourceIndex::with_expansions(&manifest, &built.expansions);
    let sections = std::mem::take(&mut built.sections);
    let (chain, mut sections) = ChainSources::join(index, sigil_span::SourceMap::new(), Vec::new(), sections).unwrap();
    // Place the image sections back to back; the fixtures emit position-independent
    // bytes only, so placement moves no byte.
    let mut lma = 0u32;
    for s in &mut sections {
        s.lma = lma;
        lma += s.image_len();
    }
    let table = walk_lines(&sections, &chain, &[], &built.symbols.code).expect("the walk");
    let mut rows = Vec::new();
    for s in &table.sections {
        for r in &s.rows {
            if matches!(r.kind, RowKind::Code | RowKind::Data) {
                assert!(chain.file(r.file).path.ends_with("m.emp"), "{:?}", chain.file(r.file));
            }
            rows.push(Row { lma: r.lma, len: r.len, line: r.line, kind: r.kind });
        }
    }
    built.sections = sections;
    drop(dir);
    (rows, built, chain)
}

/// The line of the row covering byte `lma`.
fn line_at(rows: &[Row], lma: u32) -> u32 {
    rows.iter()
        .find(|r| r.lma <= lma && lma < r.lma + r.len)
        .unwrap_or_else(|| panic!("no row covers {lma:#X}: {rows:?}"))
        .line
}

/// Every byte's line, in address order: one entry per 68000 word (each fixture
/// instruction here is one word).
fn word_lines(rows: &[Row]) -> Vec<u32> {
    let end = rows.iter().map(|r| r.lma + r.len).max().unwrap_or(0);
    (0..end).step_by(2).map(|a| line_at(rows, a)).collect()
}

const ONE_SPLICE: &str = "\
module m
comptime fn twice() -> Code {
    return asm {
        nop // @body1
        nop // @body2
    }
}
proc p () {
    moveq #1, d0 // @before
    twice() // @call
    rts // @after
}
";

#[test]
fn template_bytes_report_the_call_line() {
    let (rows, built, _chain) = build(ONE_SPLICE);
    let l = |m| line_of(ONE_SPLICE, m);
    // moveq, nop, nop, rts: one word each.
    assert_eq!(
        word_lines(&rows),
        vec![l("@before"), l("@call"), l("@call"), l("@after")],
        "the template's two nops are the call line's bytes: {rows:?}"
    );
    assert!(rows.iter().all(|r| r.kind == RowKind::Code), "every row is an instruction: {rows:?}");
    // One splice run, one expansion (both nops share it).
    assert_eq!(built.expansions.expansion_count(), 1, "one splice, one expansion");
}

const TWO_SPLICES: &str = "\
module m
comptime fn twice() -> Code {
    return asm {
        nop
        nop
    }
}
proc p () {
    twice() // @first
    moveq #1, d0 // @between
    twice // @second
    rts // @after
}
";

/// Review Focus 4: two runs of ONE template are two expansions, each locating to its own
/// call, never to each other's or to the body. The paren call and the bare statement
/// call are two different splice sites; the `{expr}` site (legal only inside a template
/// body) is the inner splice of `code_value_built_in_one_place_spliced_in_another_…`.
#[test]
fn two_splices_report_their_own_call_lines() {
    let (rows, built, _chain) = build(TWO_SPLICES);
    let l = |m| line_of(TWO_SPLICES, m);
    assert_eq!(
        word_lines(&rows),
        vec![l("@first"), l("@first"), l("@between"), l("@second"), l("@second"), l("@after")],
        "each run's bytes are its own call line: {rows:?}"
    );
    assert_eq!(built.expansions.expansion_count(), 2, "two splices, two expansions");
}

const BUILT_ELSEWHERE: &str = "\
module m
comptime fn make() -> Code {
    return asm {
        nop // @built
    }
}
comptime fn wrap(c: Code) -> Code {
    return asm {
        moveq #2, d1 // @wrap_own
        {c} // @inner_splice
    }
}
proc p () {
    wrap(make()) // @outer_splice
    rts // @after
}
";

/// A `Code` value is first-class: `make()`'s nop is BUILT in `make`, passed as an
/// argument, and SPLICED inside `wrap`, whose output is spliced into `p`. Its bytes
/// report the splice (the outermost one, as an AS macro called from a macro does), and
/// the chain records every splice it went through: the nop's own expansion entered at
/// `{c}` in `wrap`, and that line itself sits in `wrap`'s run entered at `p`.
#[test]
fn code_value_built_in_one_place_spliced_in_another_reports_the_splice() {
    let (rows, built, chain) = build(BUILT_ELSEWHERE);
    let l = |m| line_of(BUILT_ELSEWHERE, m);
    assert_eq!(
        word_lines(&rows),
        vec![l("@outer_splice"), l("@outer_splice"), l("@after")],
        "wrap's moveq and make's nop are the outer splice's bytes: {rows:?}"
    );
    // The nop (the second word): its expansion's call is `{c}` in `wrap`, whose own span
    // is in `wrap`'s run, whose call is `p`'s line.
    let nop = data_span_at(&built, 2);
    let map = &built.expansions;
    let (_, _, inner_call, frame) = map.expansion_entry(nop.source).expect("the nop is in an expansion");
    assert_eq!(frame, &sigil_span::Frame::Template("<expr>".into()), "entered by the `{{c}}` splice");
    let (_, _, outer_call, frame) = map.expansion_entry(inner_call.source).expect("`{c}` is in wrap's run");
    assert_eq!(frame, &sigil_span::Frame::Template("wrap".into()), "entered by the `wrap(..)` call");
    assert!(!outer_call.source.is_expansion(), "the outermost call is a file line");
    let text_line = |span: sigil_span::Span| BUILT_ELSEWHERE[..span.start as usize].matches('\n').count() as u32 + 1;
    assert_eq!(text_line(inner_call), l("@inner_splice"));
    assert_eq!(text_line(outer_call), l("@outer_splice"));
    // Its diagnostics location is still the line it was WRITTEN on, in `make`.
    let loc = chain.locate(nop).expect("located");
    assert!(loc.ends_with(&format!("m.emp:{}:9", l("@built"))), "{loc}");
}

const BRACKET: &str = "\
module m
context irq_off {
    acquire = asm { move.w sr, -(sp) // @acq1
                    move.w #$2700, sr } // @acq2
    release = asm { move.w (sp)+, sr } // @rel
}
proc p () {
    with irq_off { // @with
        nop // @body
    } // @close
    rts // @after
}
";

/// A `with` bracket splices its context's acquire at its header and its release at its
/// closing `}`: each half's bytes are the line that splices it, so a step through the
/// bracket runs header, body, closing brace, in that order (never back up to the header
/// for the release). The bracketed body keeps its own lines (it is not a splice).
#[test]
fn with_bracket_halves_report_their_splice_lines() {
    let (rows, built, _chain) = build(BRACKET);
    let l = |m| line_of(BRACKET, m);
    // move.w sr,-(sp) (1 word), move.w #$2700,sr (2 words), nop, move.w (sp)+,sr, rts.
    assert_eq!(
        word_lines(&rows),
        vec![l("@with"), l("@with"), l("@with"), l("@body"), l("@close"), l("@after")],
        "{rows:?}"
    );
    assert_eq!(built.expansions.expansion_count(), 2, "the acquire run and the release run");
}

const BAD_IN_TEMPLATE: &str = "\
module m
comptime fn bad() -> Code {
    return asm {
        moveq #1000, d0 // @bad
    }
}
proc p () {
    bad() // @call
    bad() // @call2
    rts
}
";

/// The diagnostics surface does not move: an error raised on a spliced instruction AFTER
/// the splice (the encoder refusing it) names the line it was WRITTEN on, the template
/// body, as it did before splices recorded expansions; no diagnostic the driver returns
/// carries an expansion id; and the driver's errors are EXACTLY those of the same module
/// lowered without recording expansions (`lower_module`, the path that never rewrites a
/// span): same levels, messages, spans and order, duplicates included.
#[test]
fn a_diagnostic_inside_a_spliced_template_names_the_written_line() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("m.emp"), BAD_IN_TEMPLATE).unwrap();
    let (manifest, _) = Manifest::scan(dir.path());
    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::backend::Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: Vec::new(),
    };
    let built = build_program_open_embed_counted(&manifest, "m", None, &opts, &|_| None);
    assert_eq!(built.expansions.expansion_count(), 2, "both splices recorded an expansion");
    assert!(built.diags.iter().all(|d| !d.primary.source.is_expansion() || d.primary.source.0 == u32::MAX), "{:?}", built.diags);
    let index = SourceIndex::with_expansions(&manifest, &built.expansions);
    let errs: Vec<String> = built
        .diags
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| format!("{}: {}", index.locate(d.primary).unwrap_or_default(), d.message))
        .collect();
    let want = format!("m.emp:{}:", line_of(BAD_IN_TEMPLATE, "@bad"));
    assert!(!errs.is_empty() && errs.iter().all(|e| e.contains(&want)), "every error names the body line {want}: {errs:#?}");
    assert_eq!(errs.len(), 2, "one error per splice, as before: {errs:#?}");
    let (_, unrecorded) = sigil_frontend_emp::lower::lower_module(&manifest.modules[0].file, &opts);
    let only_errors = |ds: &[sigil_span::Diagnostic]| -> Vec<sigil_span::Diagnostic> {
        ds.iter().filter(|d| d.level == Level::Error).cloned().collect()
    };
    assert_eq!(only_errors(&built.diags), only_errors(&unrecorded), "recording expansions moved a diagnostic");
}

/// The span of the data fragment holding image byte `lma` (fixtures place one section).
fn data_span_at(built: &BuiltProgram, lma: u32) -> sigil_span::Span {
    let mut at = 0u32;
    for s in &built.sections {
        for f in &s.fragments {
            if let sigil_ir::Fragment::Data(d) = f {
                let len = d.bytes.len() as u32;
                if s.lma + at <= lma && lma < s.lma + at + len {
                    return d.span;
                }
                at += len;
            }
        }
        at = 0;
    }
    panic!("no data fragment holds {lma:#X}");
}

/// THE CENSUS GATE. Every place `eval/asm.rs` hands spliced Code to a stream (a
/// `reauthor_user_items(` or `.splice_items(` boundary, or an `items.extend(`/
/// `items.append(` of spliced items) carries a `// splice-site: NAME` tag, every tag names an entry of
/// `SPLICE_SITES` (which sits beside the rewrite helper and says, per site, how its
/// bytes are attributed), and every entry is tagged somewhere. A new splice site
/// without a decision is untagged, and this goes red.
///
/// The call-name scan sees only the boundary helpers, so a second scan (fix round
/// 1, item 3b) covers the values themselves: every `Value::Code(` line in
/// `eval/*.rs` (a destructure that could splice, and the constructors with it)
/// carries either a registered `// splice-site: NAME` or one `// code-value: WHY`
/// from [`NOT_A_SPLICE`]. A new place that takes Code apart has to say which it is.
#[test]
fn splice_site_census_is_complete() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sigil-frontend-emp/src/eval/asm.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let sites = sigil_frontend_emp::eval::SPLICE_SITES;
    assert!(!sites.is_empty(), "SPLICE_SITES is empty: unmeasurable");
    let mut problems = Vec::new();
    let mut tagged = std::collections::BTreeMap::<&str, usize>::new();
    let mut seen = 0;
    for (i, line) in src.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let hit = ["reauthor_user_items(", ".splice_items(", "items.extend(", "items.append("].iter().any(|p| code.contains(p));
        if !hit {
            continue;
        }
        seen += 1;
        match line.split("// splice-site: ").nth(1).map(|t| t.split_whitespace().next().unwrap_or("")) {
            Some(name) if sites.iter().any(|(n, _)| *n == name) => *tagged.entry(name).or_default() += 1,
            Some(name) => problems.push(format!("asm.rs:{}: tag `{name}` is not in SPLICE_SITES: {}", i + 1, line.trim())),
            None => problems.push(format!("asm.rs:{}: untagged splice site: {}", i + 1, line.trim())),
        }
    }
    assert!(seen > 0, "no splice boundary found in asm.rs: the census measures nothing");
    for (name, how) in sites {
        if !tagged.contains_key(name) {
            problems.push(format!("SPLICE_SITES lists `{name}` ({how}) but no line in asm.rs is tagged with it"));
        }
    }
    // Second scan: every `Value::Code(` in the evaluator.
    let eval_dir = path.parent().expect("eval/");
    let mut files: Vec<_> = std::fs::read_dir(eval_dir)
        .expect("read eval/")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    let mut values = 0;
    let mut whys = std::collections::BTreeMap::<&str, usize>::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let short = file.file_name().unwrap().to_string_lossy().into_owned();
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if !code.contains("Value::Code(") {
                continue;
            }
            values += 1;
            let tag = |key: &str| line.split(key).nth(1).map(|t| t.split_whitespace().next().unwrap_or("").to_string());
            match (tag("// splice-site: "), tag("// code-value: ")) {
                (Some(name), None) if sites.iter().any(|(n, _)| *n == name) => {
                    *tagged.entry(sites.iter().find(|(n, _)| *n == name).unwrap().0).or_default() += 1
                }
                (None, Some(why)) if NOT_A_SPLICE.iter().any(|(w, _)| *w == why) => {
                    *whys.entry(NOT_A_SPLICE.iter().find(|(w, _)| *w == why).unwrap().0).or_default() += 1
                }
                _ => problems.push(format!(
                    "{short}:{}: `Value::Code(` without one registered `splice-site:` or `code-value:` tag: {}",
                    i + 1,
                    line.trim()
                )),
            }
        }
    }
    assert!(values > 0, "no `Value::Code(` found under eval/: the second scan measures nothing");
    eprintln!(
        "CENSUS boundaries={seen} code_values={values} sites={} tagged={tagged:?} not_a_splice={whys:?}",
        sites.len()
    );
    assert!(problems.is_empty(), "splice-site census:\n{}", problems.join("\n"));
}

/// Why a `Value::Code(` line hands no Code to the stream, for the census's second
/// scan. Each reason is a claim the census cannot check itself, so the list is
/// short and each entry says why its spans need no rewrite.
const NOT_A_SPLICE: &[(&str, &str)] = &[
    ("build", "constructs a Code value; its items keep their written spans until a splice site takes it"),
    ("concat", "`++` joins two Code values; each side keeps its spans and the splice of the result attributes them"),
    ("compare", "a type-equality arm; the items are not touched"),
    ("measure", "`span(proc)` lowers a throwaway body with the expansion map set aside; nothing is emitted"),
    ("proc-body", "a proc's own body: written in place, emitted where written"),
];

/// `sigil emp --root`: a LINK diagnostic over spliced bytes still names the
/// template-body line it was written on (fix round 1, item 1). The splice gives
/// the `bra.s` an expansion span; the link tail renders through the manifest's
/// `SourceIndex`, and an index built without the program's expansions cannot
/// place that span, so the error printed with no location at all. The fixture is
/// the reviewer's: `far()` holds `bra.s Far` on line 4, column 9, and 200 `nop`s
/// put `Far` out of short range.
#[test]
fn sigil_emp_link_diagnostic_inside_a_splice_names_the_body_line() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors/template_link_diag/root");
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("root");
    std::fs::create_dir(&root).expect("mkdir root");
    std::fs::copy(fixture.join("m.emp"), root.join("m.emp")).expect("copy fixture");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(tmp.path())
        .args(["emp", "root/m.emp", "--root", "root", "-o"])
        .arg(tmp.path().join("out.bin"))
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "the out-of-range bra.s must fail the link.\nstderr:\n{stderr}");
    let want = "m.emp:4:9: error: bra.s/Bcc.s displacement to `m.Far` out of range (402)";
    assert!(
        stderr.lines().any(|l| l.contains(want)),
        "the link error must name the template-body line `{want}`.\nstderr:\n{stderr}"
    );
}

const MEASURED: &str = "\
module m
comptime fn pair() -> Code {
    return asm {
        dc.b 1, 2
    }
}
proc T () {
    pair() // @splice
}
proc p () {
    moveq #span(T), d0 // @measure
    rts
}
";

/// `span(T)` lowers `T`'s body as a throwaway measurement. That throwaway splice
/// must record nothing (fix round 1, item 3a): the only expansion is `T`'s real
/// splice, and the measured byte stays on the `span` line as written.
#[test]
fn a_span_measurement_records_no_orphan_expansion() {
    let (rows, built, _chain) = build(MEASURED);
    let l = |m| line_of(MEASURED, m);
    assert_eq!(line_at(&rows, 0), l("@splice"), "{rows:?}");
    assert_eq!(line_at(&rows, 2), l("@measure"), "{rows:?}");
    assert_eq!(built.expansions.expansion_count(), 1, "only T's own splice records an expansion");
}
