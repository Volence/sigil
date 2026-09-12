//! A diagnostic raised inside a macro or loop expansion names the line that
//! CALLED it, then the chain of expansions in between, in the reference
//! assembler's own spelling.
//!
//! ## What was wrong
//!
//! Every such diagnostic named the macro BODY's line. On the Sonic 2
//! disassembly all 34 rows raised inside `dac_sample_metadata` named
//! `s2.sounddriver.asm(3905)`, the one body line, and none named which of its
//! 17 calls had failed; the reader had to find the call by elimination.
//!
//! ## The reference
//!
//! asl (md5 `61e672562465725a8c102288a7da9098`, through `asl_run`) prints the
//! OUTERMOST call's `file(line)`, then one frame per expansion, outermost
//! first, then the column. Each fixture below is the probe it was measured on,
//! byte for byte, and each expectation is asl's line with ONE difference, the
//! column: asl counts a tab as eight columns inside an expansion (`:9` for a
//! tab-indented mnemonic, where the same line outside one is `:2`), and sigil
//! counts characters everywhere, as it always has. The probes and asl's full
//! transcripts are in `docs/superpowers/notes/2026-09-12-as-macro-diag-call-site/`.

use sigil_frontend_as::{assemble_root_located, assemble_root_located_warned, Options};

/// Write `files` into a fresh directory and assemble the first one. Every
/// diagnostic comes back as `label: level: message`, the line the CLI prints;
/// a diagnostic with no label comes back as `<none>: …`, so a lost location
/// fails an assertion rather than vanishing.
fn diags(files: &[(&str, &str)]) -> Vec<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).expect("write fixture");
    }
    let root = dir.path().join(files[0].0);
    let prefix = format!("{}/", dir.path().display());
    let render = |sources: &sigil_span::SourceMap, d: &sigil_span::Diagnostic| {
        let label = sources.label(d.primary).unwrap_or_else(|| "<none>".to_string());
        let label = label.strip_prefix(&prefix).unwrap_or(&label).to_string();
        format!("{label}: {}: {}", d.level, d.message)
    };
    match assemble_root_located_warned(&root, &Options::default()) {
        Ok(a) => a.warnings.iter().map(|d| render(&a.sources, d)).collect(),
        Err(f) => f.diags.iter().map(|d| render(&f.sources, d)).collect(),
    }
}

/// Only the location half of each line: everything before `: error:` or
/// `: warning:`.
fn labels(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            let cut = [": error: ", ": warning: "].iter().filter_map(|m| l.find(m)).min().unwrap_or(l.len());
            l[..cut].to_string()
        })
        .collect()
}

/// asl, `p1_simple.asm`:
/// `p1_simple.asm(7) mymac(2):9` and `p1_simple.asm(8) mymac(2):9`.
/// One body line, two calls, two different reports.
#[test]
fn each_call_of_a_macro_names_its_own_line() {
    let got = diags(&[(
        "p1_simple.asm",
        "\tcpu 68000\nmymac macro\n\tnop\n\tbogus_in_body\n\tendm\n\tnop\n\tmymac\n\tmymac\n",
    )]);
    assert_eq!(labels(&got), ["p1_simple.asm(7) mymac(2):2", "p1_simple.asm(8) mymac(2):2"], "{got:#?}");
    assert!(got.iter().all(|l| l.contains("`bogus_in_body`")), "{got:#?}");
}

/// asl, `p3_incbody.asm` including `p3_mac.inc`:
/// `p3_incbody.asm(3) mymac(1):9`. The file holding the BODY is not named.
#[test]
fn a_body_written_in_an_included_file_is_named_by_its_call() {
    let got = diags(&[
        ("p3_incbody.asm", "\tcpu 68000\n\tinclude \"p3_mac.inc\"\n\tmymac\n"),
        ("p3_mac.inc", "; line 1\nmymac macro\n\tbogus_in_inc_body\n\tendm\n"),
    ]);
    assert_eq!(labels(&got), ["p3_incbody.asm(3) mymac(1):2"], "{got:#?}");
}

/// asl, `p11_inccall.asm`: the CALL is written in an included file, so that
/// file is the one named: `p11_call.inc(3) mymac(1):9`.
#[test]
fn a_call_written_in_an_included_file_names_that_file() {
    let got = diags(&[
        ("p11_inccall.asm", "\tcpu 68000\n\tinclude \"p11_def.inc\"\n\tinclude \"p11_call.inc\"\n"),
        ("p11_def.inc", "mymac macro\n\tbogus_from_inc_call\n\tendm\n"),
        ("p11_call.inc", "; one\n; two\n\tmymac\n"),
    ]);
    assert_eq!(labels(&got), ["p11_call.inc(3) mymac(1):2"], "{got:#?}");
}

/// asl, `p2_nested.asm`: `p2_nested.asm(9) outer(2) inner(1):9`, and
/// `q6_three.asm`: `q6_three.asm(14) outer(1) mid(3) inner(2):9`. Outermost
/// first; each number is the line of THAT macro's body the next frame was
/// entered from.
#[test]
fn nested_macros_print_every_frame_outermost_first() {
    let got = diags(&[(
        "p2_nested.asm",
        "inner macro\n\tbogus_inner\n\tendm\nouter macro\n\tnop\n\tinner\n\tendm\n\tcpu 68000\n\touter\n",
    )]);
    assert_eq!(labels(&got), ["p2_nested.asm(9) outer(2) inner(1):2"], "{got:#?}");
    let got = diags(&[(
        "q6_three.asm",
        "inner macro\n\tnop\n\tbogus_inner\n\tendm\nmid macro\n\tnop\n\tnop\n\tinner\n\tendm\nouter macro\n\tmid\n\tendm\n\tcpu 68000\n\touter\n",
    )]);
    assert_eq!(labels(&got), ["q6_three.asm(14) outer(1) mid(3) inner(2):2"], "{got:#?}");
}

/// A loop inside a macro is a frame of its own, entered from its CLOSING line.
/// asl:
///
/// ```text
/// p4_rept.asm(8) mymac(3) REPT 1(1):9       p4_rept.asm(8) mymac(3) REPT 2(1):9
/// p5_while.asm(10) mymac(5) WHILE 1/1:9     p5_while.asm(10) mymac(5) WHILE 2/1:9
/// p10_macirp.asm(8) mymac(3) IRP:2(1):9     p10_macirp.asm(8) mymac(3) IRP:(1):9
/// ```
#[test]
fn a_loop_in_a_macro_is_a_frame_entered_from_its_closing_line() {
    let got = diags(&[(
        "p4_rept.asm",
        "\tcpu 68000\nmymac macro\n\trept 2\n\tbogus_in_rept\n\tendm\n\tendm\n\tnop\n\tmymac\n",
    )]);
    assert_eq!(labels(&got), ["p4_rept.asm(8) mymac(3) REPT 1(1):2", "p4_rept.asm(8) mymac(3) REPT 2(1):2"], "{got:#?}");
    let got = diags(&[(
        "p5_while.asm",
        "\tcpu 68000\nmymac macro\ncnt set 0\n\twhile cnt<2\n\tbogus_in_while\ncnt set cnt+1\n\tendm\n\tendm\n\tnop\n\tmymac\n",
    )]);
    assert_eq!(
        labels(&got),
        ["p5_while.asm(10) mymac(5) WHILE 1/1:2", "p5_while.asm(10) mymac(5) WHILE 2/1:2"],
        "{got:#?}"
    );
    let got = diags(&[(
        "p10_macirp.asm",
        "\tcpu 68000\nmymac macro\n\tirp x,1,2\n\tbogus_irp x\n\tendm\n\tendm\n\tnop\n\tmymac\n",
    )]);
    assert_eq!(labels(&got), ["p10_macirp.asm(8) mymac(3) IRP:2(1):2", "p10_macirp.asm(8) mymac(3) IRP:(1):2"], "{got:#?}");
}

/// A macro frame is followed by a space and a loop frame is not. asl:
///
/// ```text
/// q7_nest_rept_space.asm(11) outer(4) REPT 1(2)inner(1):9
/// q8_rept_in_inner.asm(11) outer(2) inner(3) REPT 1(1):9      (and REPT 2)
/// q10_mac_in_toprept.asm(7) REPT 1(1)mymac(1):9                (and REPT 2)
/// ```
#[test]
fn a_space_follows_a_macro_frame_and_not_a_loop_frame() {
    let got = diags(&[(
        "q7_nest_rept_space.asm",
        "inner macro\n\tbogus_inner\n\tendm\nouter macro\n\trept 1\n\tnop\n\tinner\n\tendm\n\tendm\n\tcpu 68000\n\touter\n",
    )]);
    assert_eq!(labels(&got), ["q7_nest_rept_space.asm(11) outer(4) REPT 1(2)inner(1):2"], "{got:#?}");
    let got = diags(&[(
        "q8_rept_in_inner.asm",
        "inner macro\n\trept 2\n\tbogus_inner_rept\n\tendm\n\tendm\nouter macro\n\tnop\n\tinner\n\tendm\n\tcpu 68000\n\touter\n",
    )]);
    assert_eq!(
        labels(&got),
        ["q8_rept_in_inner.asm(11) outer(2) inner(3) REPT 1(1):2", "q8_rept_in_inner.asm(11) outer(2) inner(3) REPT 2(1):2"],
        "{got:#?}"
    );
    let got = diags(&[(
        "q10_mac_in_toprept.asm",
        "mymac macro\n\tbogus_m\n\tendm\n\tcpu 68000\n\trept 2\n\tmymac\n\tendm\n",
    )]);
    assert_eq!(
        labels(&got),
        ["q10_mac_in_toprept.asm(7) REPT 1(1)mymac(1):2", "q10_mac_in_toprept.asm(7) REPT 2(1)mymac(1):2"],
        "{got:#?}"
    );
}

/// A loop written at file level is a frame too, and asl names ITS closing
/// line, not the line inside it. asl:
///
/// ```text
/// p8_toprept.asm(5) REPT 1(1):2             p8_toprept.asm(5) REPT 2(1):2
/// q3_topwhile.asm(7) WHILE 1/2:2            q3_topwhile.asm(7) WHILE 2/2:2
/// q1_irp3.asm(5) IRP:bb(2):9   q1_irp3.asm(5) IRP:cc(2):9   q1_irp3.asm(5) IRP:(2):9
/// r6_irpc_abc.asm(4) IRPC:'b'(1):9   IRPC:'c'(1):9   IRPC:'(1):9
/// q5_reptrept.asm(6) REPT 1(3)REPT 1(1):2  ...  REPT 2(3)REPT 2(1):2
/// ```
#[test]
fn a_file_level_loop_is_a_frame_named_by_its_closing_line() {
    let got = diags(&[("p8_toprept.asm", "\tcpu 68000\n\tnop\n\trept 2\n\tbogus_top_rept\n\tendm\n")]);
    assert_eq!(labels(&got), ["p8_toprept.asm(5) REPT 1(1):2", "p8_toprept.asm(5) REPT 2(1):2"], "{got:#?}");
    let got = diags(&[(
        "q3_topwhile.asm",
        "\tcpu 68000\ncnt set 0\n\twhile cnt<2\n\tnop\n\tbogus_while2\ncnt set cnt+1\n\tendm\n",
    )]);
    assert_eq!(labels(&got), ["q3_topwhile.asm(7) WHILE 1/2:2", "q3_topwhile.asm(7) WHILE 2/2:2"], "{got:#?}");
    let got = diags(&[("q1_irp3.asm", "\tcpu 68000\n\tirp x,aa,bb,cc\n\tnop\n\tbogus_irp3 x\n\tendm\n")]);
    assert_eq!(
        labels(&got),
        ["q1_irp3.asm(5) IRP:bb(2):2", "q1_irp3.asm(5) IRP:cc(2):2", "q1_irp3.asm(5) IRP:(2):2"],
        "{got:#?}"
    );
    let got = diags(&[("r6_irpc_abc.asm", "\tcpu 68000\n\tirpc x,\"abc\"\n\tbogus_irpc x\n\tendm\n")]);
    assert_eq!(
        labels(&got),
        ["r6_irpc_abc.asm(4) IRPC:'b'(1):2", "r6_irpc_abc.asm(4) IRPC:'c'(1):2", "r6_irpc_abc.asm(4) IRPC:'(1):2"],
        "{got:#?}"
    );
    let got = diags(&[("q5_reptrept.asm", "\tcpu 68000\n\trept 2\n\trept 2\n\tbogus_reptrept\n\tendm\n\tendm\n")]);
    assert_eq!(
        labels(&got),
        [
            "q5_reptrept.asm(6) REPT 1(3)REPT 1(1):2",
            "q5_reptrept.asm(6) REPT 1(3)REPT 2(1):2",
            "q5_reptrept.asm(6) REPT 2(3)REPT 1(1):2",
            "q5_reptrept.asm(6) REPT 2(3)REPT 2(1):2",
        ],
        "{got:#?}"
    );
}

/// An `include` inside a macro body splices a FILE, and asl names that file
/// alone, with no trail: `p15_body.inc(2):2`. This one matches asl to the
/// column.
#[test]
fn a_file_included_from_a_macro_body_is_named_alone() {
    let got = diags(&[
        ("p15_incinmac.asm", "mymac macro\n\tinclude \"p15_body.inc\"\n\tendm\n\tcpu 68000\n\tmymac\n"),
        ("p15_body.inc", "; one\n\tbogus_in_included_from_macro\n"),
    ]);
    assert_eq!(labels(&got), ["p15_body.inc(2):2"], "{got:#?}");
}

/// Warnings carry the trail as errors do. asl, `p18_warn_err.asm`:
///
/// ```text
/// p18_warn_err.asm(6) mymac(1): warning: w1
/// p18_warn_err.asm(6) mymac(2):9: error #1200: unknown instruction
/// ```
///
/// (asl prints no column on a `warning` directive's line; sigil prints the
/// one it prints for every other line, as it did before.)
#[test]
fn a_warning_in_a_macro_carries_the_trail_too() {
    let got = diags(&[(
        "p18_warn_err.asm",
        "\tcpu 68000\nmymac macro\n\twarning \"w1\"\n\tbogus_after_warn\n\tendm\n\tmymac\n",
    )]);
    assert_eq!(labels(&got), ["p18_warn_err.asm(6) mymac(1):2", "p18_warn_err.asm(6) mymac(2):2"], "{got:#?}");
    // A successful run's warnings are located the same way.
    let got = diags(&[("p6_warn.asm", "\tcpu 68000\nmymac macro\n\twarning \"warn from body\"\n\tendm\n\tnop\n\tmymac\n")]);
    assert_eq!(labels(&got), ["p6_warn.asm(6) mymac(1):2"], "{got:#?}");
}

/// A body line that has already been reported this pass is not reported again
/// from a second call. That is sigil's standing one-line-per-source-position
/// rule for these diagnostics (`cond_faults_seen`), and the expansion ids must
/// not turn it into one line per call: the rule keys on the PHYSICAL line. The
/// one report names the first call.
#[test]
fn a_body_line_reported_once_per_pass_stays_reported_once() {
    let got = diags(&[(
        "once.asm",
        "\tcpu 68000\nm macro\n\tif Undefined_cond\n\tnop\n\tendif\n\tendm\n\tm\n\tm\n",
    )]);
    let conds: Vec<&String> = got.iter().filter(|l| l.contains("unresolved if condition")).collect();
    assert_eq!(conds.len(), 1, "{got:#?}");
    assert!(conds[0].starts_with("once.asm(7) m(1):"), "{got:#?}");
}

/// An author's `warning` fired on an earlier pass is carried to the returned
/// report once, whatever run id it had on that pass. Run ids are handed out in
/// execution order, so the `m` call that only the first pass makes gives `w`'s
/// run a different id on the first pass than on the last; the carried copy is
/// matched to the returned one by source POSITION, and there is one line.
/// (One line per site is sigil's standing rule for a warning a pass repeats;
/// asl prints this one once per pass.)
#[test]
fn a_carried_warning_is_matched_by_position_when_run_ids_shift_between_passes() {
    let got = diags(&[(
        "carry.asm",
        "\tcpu 68000\nm macro\n\tnop\n\tendm\nw macro\n\twarning \"from w\"\n\tendm\n\tif MOMPASS=1\n\tm\n\tendif\n\tw\n",
    )]);
    let from_w: Vec<&String> = got.iter().filter(|l| l.contains("from w")).collect();
    assert_eq!(from_w.len(), 1, "{got:#?}");
    assert!(from_w[0].starts_with("carry.asm(11) w(1):"), "{got:#?}");
}

/// A root with no macro and no loop reports exactly as before: `file(line):col`.
/// asl, `p14_top_control.asm`: `p14_top_control.asm(3):2`.
#[test]
fn a_line_outside_every_expansion_is_located_as_before() {
    let got = diags(&[("p14_top_control.asm", "\tcpu 68000\n\tnop\n\tbogus_top\n\tdc.w undefined_top\n")]);
    assert_eq!(labels(&got)[0], "p14_top_control.asm(3):2", "{got:#?}");
}

/// The trail changes no byte: the same body assembles to the same image with
/// its lines under an expansion id as it would written out in place.
#[test]
fn an_expansion_assembles_the_bytes_its_body_says() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bytes.asm");
    std::fs::write(
        &path,
        "\tcpu 68000\nm macro v\n\trept 2\n\tdc.b v\n\tendm\n\tendm\n\tm $11\n\tirp x,$22,$33\n\tdc.b x\n\tendm\n",
    )
    .expect("write");
    let module = assemble_root_located(&path, &Options::default()).unwrap_or_else(|f| panic!("{:#?}", f.diags));
    let resolved = sigil_link::resolve_layout(&module.sections, &sigil_ir::SymbolTable::new(), true).expect("resolve");
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(sigil_link::flatten(&linked, 0x00).expect("flatten"), [0x11, 0x11, 0x22, 0x33]);
}
