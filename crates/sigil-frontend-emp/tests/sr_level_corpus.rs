//! Whole-program inter-procedural 68000 interrupt-level & Rule 3 tests (step A piece 2b Task 4).

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::parse_str;

const INTS_OFF: &str = "context ints_off {
    acquire = asm { move.w sr, -(sp)
                    ori.w #$0700, sr }
    release = asm { move.w (sp)+, sr }
}
";

fn analyze_files(srcs: &[&str]) -> ContractReport {
    let files: Vec<_> = srcs
        .iter()
        .map(|s| {
            let (f, diags) = parse_str(s);
            assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
            f
        })
        .collect();
    analyze_corpus(&files)
}

#[test]
fn cross_file_three_hop_chain_to_masking_proc_fires() {
    let f1 = "module a
proc A () clobbers(d0) preserves(sr.mask) {
    jsr B
    rts
}
";
    let f2 = "module b
proc B () clobbers(d0) {
    jsr C
    rts
}
";
    let f3 = "module c
proc C () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    let r = analyze_files(&[f1, f2, f3]);
    assert_eq!(r.sr_level_firings.len(), 1, "{:#?}", r.sr_level_firings);
    assert_eq!(r.sr_level_firings[0].proc, "A");
    assert_eq!(r.sr_level_firings[0].code, "[proc.preserves-sr-unbalanced]");
}

#[test]
fn cross_file_call_to_undeclared_clean_proc_builds() {
    let f1 = "module a
proc A () clobbers(d0) preserves(sr.mask) {
    move.w sr, -(sp)
    ori.w #$0700, sr
    jsr Helper
    move.w (sp)+, sr
    rts
}
";
    let f2 = "module b
proc Helper () clobbers(d0) {
    moveq #1, d0
    rts
}
";
    let r = analyze_files(&[f1, f2]);
    assert!(r.sr_level_firings.is_empty(), "{:#?}", r.sr_level_firings);
}

#[test]
fn cross_file_mid_body_tail_and_branch_out_and_jra_fire() {
    let f1 = "module a
proc TailBad () clobbers(d0) preserves(sr.mask) {
    tst.w d0
    beq .ok
    jmp Masker
.ok:
    rts
}
proc BranchBad () clobbers(d0) preserves(sr.mask) {
    tst.w d0
    beq Masker
    rts
}
proc JraBad () clobbers(d0) preserves(sr.mask) {
    jra Masker
}
";
    let f2 = "module b
proc Masker () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    let r = analyze_files(&[f1, f2]);
    let procs: Vec<&str> = r.sr_level_firings.iter().map(|f| f.proc.as_str()).collect();
    assert_eq!(procs, vec!["BranchBad", "JraBad", "TailBad"], "{:#?}", r.sr_level_firings);
}

#[test]
fn cross_file_exported_mid_body_label_clean_vs_bad() {
    let owner = "module owner
proc Owner () clobbers(d0) preserves(sr.mask) {
    moveq #0, d0
export .clean:
    move.w sr, -(sp)
    ori.w #$0700, sr
export .after_save:
    move.w (sp)+, sr
    rts
}
";
    let caller_ok = "module ok
proc CallerOk () clobbers(d0) preserves(sr.mask) {
    jmp Owner.clean
}
";
    let r_ok = analyze_files(&[owner, caller_ok]);
    assert!(
        r_ok.sr_level_firings.iter().all(|f| f.proc != "CallerOk"),
        "entering Owner.clean before save should be clean: {:#?}",
        r_ok.sr_level_firings
    );

    let caller_bad = "module bad
proc CallerBad () clobbers(d0) preserves(sr.mask) {
    jmp Owner.after_save
}
";
    let r_bad = analyze_files(&[owner, caller_bad]);
    assert!(
        r_bad.sr_level_firings.iter().any(|f| f.proc == "CallerBad"
            && f.code == "[proc.preserves-sr-unbalanced]"),
        "entering Owner.after_save skips save and must fire on CallerBad: {:#?}",
        r_bad.sr_level_firings
    );
}

#[test]
fn cross_file_rule3_with_ints_off_catches_unmasker_and_transient_unmask_and_allows_critical() {
    let f_ctx = format!("module ctx\n{INTS_OFF}");
    let f_bad = "module bad
proc CallUnmask () clobbers(d0) {
    with ints_off {
        jsr Unmask
    }
    rts
}
proc CallTransient () clobbers(d0) {
    with ints_off {
        jsr UnmaskBriefly
    }
    rts
}
proc CallCritical () clobbers(d0) {
    with ints_off {
        jsr QueueDMA
    }
    rts
}
";
    let f_callees = "module callees
proc Unmask () clobbers(sr) {
    move.w #$2000, sr
    rts
}
proc UnmaskBriefly () clobbers(d0) preserves(sr.mask) {
    move.w sr, -(sp)
    move.w #$2000, sr
    moveq #1, d0
    move.w (sp)+, sr
    rts
}
proc QueueDMA () clobbers(d0) preserves(sr.mask) {
    move.w sr, -(sp)
    ori.w #$0700, sr
    moveq #1, d0
    move.w (sp)+, sr
    rts
}
";
    let r = analyze_files(&[&f_ctx, f_bad, f_callees]);
    let procs: Vec<(&str, &str)> =
        r.sr_level_firings.iter().map(|f| (f.proc.as_str(), f.code)).collect();
    assert_eq!(
        procs,
        vec![
            ("CallTransient", "[proc.with-sr-level-broken]"),
            ("CallUnmask", "[proc.with-sr-level-broken]"),
        ],
        "{:#?}",
        r.sr_level_firings
    );
}

#[test]
fn cross_file_continuation_resumable_and_typed_indirect_hooks() {
    let f_caller = "module caller
proc Wait () clobbers(d0, a0, a3) preserves(sr.mask) {
    jsr Process
    jsr (a0) as CleanHook
    rts
}
proc BadDispatch () clobbers(a0) preserves(sr.mask) {
    jsr (a0) as BadHook
    rts
}
";
    let f_types = "module types
type CleanHook = proc () clobbers(d0)
type BadHook = proc () clobbers(sr)
";
    let f_impl = "module impl
proc Process () clobbers(d0, a3) {
    tst.w d0
    beq .r
    jbra Decoder
.r:
    jbra Resume
}
@resumable
proc Decoder () clobbers(d0) {
    moveq #0, d0
    jmp (a3)
}
@continuation
proc Resume () clobbers(d0) {
    moveq #1, d0
    rte
}
";
    let r = analyze_files(&[f_caller, f_types, f_impl]);
    assert_eq!(r.sr_trusted_continuations, vec!["Resume"]);
    let procs: Vec<&str> = r.sr_level_firings.iter().map(|f| f.proc.as_str()).collect();
    assert_eq!(procs, vec!["BadDispatch"], "{:#?}", r.sr_level_firings);
}

#[test]
fn cross_file_falls_into_masking_proc_fires() {
    let f1 = "module a
proc P () clobbers(d0) preserves(sr.mask) falls_into Next {
    moveq #1, d0
}
";
    let f2 = "module b
proc Next () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    let r = analyze_files(&[f1, f2]);
    assert_eq!(r.sr_level_firings.len(), 1, "{:#?}", r.sr_level_firings);
    assert_eq!(r.sr_level_firings[0].proc, "P");
}

#[test]
fn cross_file_mutual_recursion_clean_and_masking() {
    let f1 = "module a
proc Even () clobbers(d0) preserves(sr.mask) {
    tst.w d0
    beq .done
    jsr Odd
.done:
    rts
}
";
    let f2 = "module b
proc Odd () clobbers(d0) {
    jsr Even
    rts
}
";
    let r_clean = analyze_files(&[f1, f2]);
    assert!(r_clean.sr_level_firings.is_empty(), "{:#?}", r_clean.sr_level_firings);

    let f2_bad = "module b
proc Odd () clobbers(d0, sr) {
    move.w #$2700, sr
    jsr Even
    rts
}
";
    let r_bad = analyze_files(&[f1, f2_bad]);
    assert_eq!(r_bad.sr_level_firings.len(), 1, "{:#?}", r_bad.sr_level_firings);
    assert_eq!(r_bad.sr_level_firings[0].proc, "Even");
}

#[test]
fn cross_file_closing_export_label_before_falls_into_is_resolved() {
    let f1 = "module a
proc Caller () clobbers(d0) preserves(sr.mask) {
    jmp Host.tail
}
";
    let f2 = "module b
proc Host () clobbers(d0) falls_into Next {
    moveq #0, d0
export .tail:
}
proc Next () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    let r = analyze_files(&[f1, f2]);
    assert_eq!(r.sr_level_firings.len(), 1, "{:#?}", r.sr_level_firings);
    assert_eq!(r.sr_level_firings[0].proc, "Caller");
}
