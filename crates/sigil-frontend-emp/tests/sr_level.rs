//! Step A piece 2a: `preserves(sr.mask)` / `preserves(sr)` is judged by following
//! the interrupt level down every path (`sr_level::mask_findings`), not by the
//! line order of the SR writes. Whole-source tests: each pins every diagnostic a
//! source produces (level, message, and the source LINE it is anchored on), so a
//! doubled or a moved diagnostic cannot hide.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// One diagnostic as the tests read it: level, message, 1-based source line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct D {
    level: Level,
    message: String,
    line: usize,
}

/// The 1-based line of a byte offset in `src`.
fn line_of(src: &str, offset: u32) -> usize {
    src.as_bytes()[..(offset as usize).min(src.len())].iter().filter(|&&b| b == b'\n').count() + 1
}

/// The 1-based line of the `nth` (0-based) line whose trimmed text is `text`.
fn line_named(src: &str, text: &str, nth: usize) -> usize {
    src.lines()
        .enumerate()
        .filter(|(_, l)| l.trim() == text)
        .nth(nth)
        .unwrap_or_else(|| panic!("no line #{nth} reading `{text}` in:\n{src}"))
        .0
        + 1
}

/// Every diagnostic of a whole module source, lowered with `defines`.
fn lower_with(src: &str, defines: Vec<(String, i128)>) -> Vec<D> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines };
    let (_m, d) = lower_module(&file, &opts);
    d.iter().map(|d| D { level: d.level, message: d.message.clone(), line: line_of(src, d.primary.start) }).collect()
}

fn lower(src: &str) -> Vec<D> {
    lower_with(src, vec![])
}

/// The one-routine source the tests use: `proc P () {header} { body }`.
fn routine(header: &str, body: &str) -> String {
    format!("module m\nproc P () {header} {{\n{body}\n}}\n")
}

const MASK: &str = "clobbers(d0) preserves(sr.mask)";
const PREFIX: &str = "[proc.preserves-sr-unbalanced] `P` declares `preserves(sr.mask)` but ";

/// The review's W2 body: one path masks, frees the saved word and returns.
const W2: &str = "    move.w sr, -(sp)
    move.w #$2700, sr
    tst.w d0
    beq .full
    addq.l #2, sp
    rts
.full:
    move.w (sp)+, sr
    rts";

fn not_the_callers(token: &str, levels: &str) -> String {
    format!(
        "[proc.preserves-sr-unbalanced] `P` declares `preserves({token})` but this way out can be \
         reached with the interrupt level at {levels}, not the caller's (a path masks interrupts \
         and does not restore the saved level)"
    )
}

fn untracked() -> String {
    format!(
        "{PREFIX}the interrupt level cannot be followed to this way out (restore it with \
         `move.w (sp)+, sr` from the matching `move.w sr, -(sp)`, with the stack as it was at \
         the save)"
    )
}

fn err(message: String, line: usize) -> D {
    D { level: Level::Error, message, line }
}

#[test]
fn w2_is_refused_at_the_leaking_return() {
    let src = routine(MASK, W2);
    let d = lower(&src);
    assert_eq!(d, vec![err(not_the_callers("sr.mask", "7"), line_named(&src, "rts", 0))], "{d:#?}");
    assert!(d[0].message.contains("at 7"));
}

#[test]
fn w2_through_a_pop_into_d0_is_refused() {
    let src = routine(MASK, &W2.replace("addq.l #2, sp", "move.w (sp)+, d0"));
    let d = lower(&src);
    assert_eq!(d, vec![err(not_the_callers("sr.mask", "7"), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Accepted before with only a `[proc.ccr-advisory]` warning. Now refused, and
/// the advisory (gated on the mask proof passing) is no longer raised.
#[test]
fn w2_under_bare_preserves_sr_is_refused() {
    let src = routine("clobbers(d0) preserves(sr)", W2);
    let d = lower(&src);
    assert_eq!(d, vec![err(not_the_callers("sr", "7"), line_named(&src, "rts", 0))], "{d:#?}");
}

#[test]
fn both_paths_restoring_builds() {
    let src = routine(MASK, &W2.replace("addq.l #2, sp", "move.w (sp)+, sr"));
    assert_eq!(lower(&src), vec![]);
}

#[test]
fn a_routine_that_never_touches_sr_builds() {
    assert_eq!(lower(&routine(MASK, "    moveq #0, d0\n    rts")), vec![]);
}

#[test]
fn a_balanced_stop_builds() {
    let src = routine(MASK, "    move.w sr, -(sp)\n    stop #$2500\n    move.w (sp)+, sr\n    rts");
    assert_eq!(lower(&src), vec![]);
}

/// Survey variant `f`: the branch re-masks to 3 and then restores.
#[test]
fn one_path_remasking_then_restoring_builds() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    tst.w d0
    beq .out
    move.w (sp)+, sr
    rts
.out:
    move.w #$2300, sr
    move.w (sp)+, sr
    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

/// Survey variant `d`: refused before (line order), still refused (now at the `rts`).
#[test]
fn ori_with_no_save_is_refused() {
    let src = routine(MASK, "    ori.w #$0700, sr\n    rts");
    let d = lower(&src);
    assert_eq!(d, vec![err(not_the_callers("sr.mask", "7"), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Survey variant `h`: the level is saved in a register, which the walk does
/// not follow; the error is on the way out.
#[test]
fn a_register_save_is_refused() {
    let src = routine(
        "clobbers(d0, d1) preserves(sr.mask)",
        "    move.w sr, d1\n    move.w #$2700, sr\n    moveq #0, d0\n    move.w d1, sr\n    rts",
    );
    let d = lower(&src);
    assert_eq!(
        d,
        vec![err(untracked(), line_named(&src, "rts", 0))],
        "{d:#?}"
    );
}

/// Survey variant `l`.
#[test]
fn an_rte_is_refused() {
    let src = routine(MASK, "    moveq #0, d0\n    rte");
    let d = lower(&src);
    assert_eq!(
        d,
        vec![err(
            format!("{PREFIX}leaves by `rte`, which loads the interrupt level from the stack"),
            line_named(&src, "rte", 0)
        )],
        "{d:#?}"
    );
}

/// Survey variant `j`.
#[test]
fn a_call_to_a_masking_routine_still_builds() {
    // 2a keeps today's reading of calls; piece 2b closes this
    let src = "module m
proc P () clobbers(d0) preserves(sr.mask) {
    jsr Q
    rts
}
proc Q () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    assert_eq!(lower(src), vec![]);
}

const INTS_OFF: &str = "context ints_off {
    acquire = asm { move.w sr, -(sp)
 move.w #$2700, sr }
    release = asm { move.w (sp)+, sr }
}
";

#[test]
fn with_ints_off_builds() {
    let src = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0) preserves(sr.mask) {{
    with ints_off {{
        moveq #0, d0
    }}
    rts
}}
"
    );
    assert_eq!(lower(&src), vec![]);
}

/// An `rts` inside the bracket leaves at level 7 with the saved word still on
/// the stack. The FULL set of diagnostics that fire is pinned here.
#[test]
fn with_ints_off_and_an_early_return_inside_is_refused() {
    let src = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0) preserves(sr.mask) {{
    with ints_off {{
        tst.w d0
        bne .stay
        rts
.stay:
        moveq #0, d0
    }}
    rts
}}
"
    );
    let d = lower(&src);
    let ours: Vec<&D> = d.iter().filter(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")).collect();
    assert_eq!(ours, vec![&err(not_the_callers("sr.mask", "7"), line_named(&src, "rts", 0))], "{d:#?}");
    let codes: Vec<(Level, &str)> =
        d.iter().map(|x| (x.level, x.message.split(' ').next().unwrap_or(""))).collect();
    assert_eq!(codes, PINNED_WITH_INTS_OFF_EARLY_RETURN, "{d:#?}");
}

/// Measured, not assumed (see the notes' Task 4 section).
const PINNED_WITH_INTS_OFF_EARLY_RETURN: &[(Level, &str)] = &[
    (Level::Error, "[proc.preserves-sr-unbalanced]"),
    (Level::Error, "[context.escape]"),
    (Level::Error, "[stack.unbalanced]"),
];

/// The routine itself is refused with no entrant: an entry at `.mid` pops a word
/// it never pushed, so the restore's level is not known.
#[test]
fn an_export_label_after_the_save_is_refused_on_its_own() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)\n    move.w #$2700, sr\nexport .mid:\n    move.w (sp)+, sr\n    rts",
    );
    let d = lower(&src);
    assert_eq!(
        d,
        vec![err(untracked(), line_named(&src, "rts", 0))],
        "{d:#?}"
    );
}

#[test]
fn a_masked_branch_to_the_closing_label_is_refused() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    tst.w d0
    beq .full
    addq.l #2, sp
    bra .end
.full:
    move.w (sp)+, sr
    rts
.end:",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(not_the_callers("sr.mask", "7"), line_named(&src, "bra .end", 0))], "{d:#?}");
}

#[test]
fn a_helper_entered_through_a_pointer_is_refused() {
    let src = routine(
        "clobbers(d0, a0) preserves(sr.mask)",
        "    lea .h(pc), a0\n    jsr (a0)\n    rts\n.h:\n    move.w #$2700, sr\n    rts",
    );
    let d = lower(&src);
    // The helper is walked as an extra entry and refused on its own `rts`.
    assert_eq!(d, vec![err(not_the_callers("sr.mask", "7"), line_named(&src, "rts", 1))], "{d:#?}");
}

/// The final review's I1 probe (`c02_ptr_entry_feeds_restore`): a block reached
/// only through `jmp (a0)` pushes `$2700` and joins the shared restore. Built
/// clean before the extra entries; the restore now pops an unknown word.
#[test]
fn a_dead_block_feeding_the_restore_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    tst.w d0
    beq .via
    move.w sr, -(sp)
    move.w #$2700, sr
.r:
    move.w (sp)+, sr
    rts
.via:
    lea .b(pc), a0
    jmp (a0)
.b:
    move.w #$2700, -(sp)
    bra .r",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// The final review's I3(a) probe (`s02_copy_top_over_save`): `move.w (sp), 2(sp)`
/// copies the pushed `$2700` over the saved word.
#[test]
fn copying_the_top_over_the_saved_word_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    move.w sr, -(sp)
    move.w #$2700, -(sp)
    move.w (sp), 2(sp)
    addq.l #2, sp
    move.w (sp)+, sr
    rts",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// The final review's I3(d) probe (`c03_cmpm_double_pop`): `cmpm.w (sp)+, (sp)+`
/// pops four bytes; the `pea` + `cmpm` pair hides the imbalance from the stack check.
#[test]
fn a_double_pop_on_one_line_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    move.w #$2700, -(sp)
    move.w sr, -(sp)
    move.w d0, -(sp)
    cmpm.w (sp)+, (sp)+
    move.w (sp)+, sr
    pea (a0)
    cmpm.w (sp)+, (sp)+
    rts",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Probe x1 (fix wave, F1b): a pointer lands after a dead save and mask.
#[test]
fn a_pointer_into_the_middle_of_dead_code_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    lea .mid(pc), a0
    jmp (a0)
.top:
    move.w sr, -(sp)
    move.w #$2700, sr
.mid:
    move.w (sp)+, sr
    rts",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Probe x1 with the address taken as an immediate (`ImmLink`).
#[test]
fn an_immediate_address_into_the_middle_of_dead_code_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    movea.l #.mid, a0
    jmp (a0)
.top:
    move.w sr, -(sp)
    move.w #$2700, sr
.mid:
    move.w (sp)+, sr
    rts",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Probe x3: the landing named by `targets(.mid)`, with the real jump table.
#[test]
fn a_targets_jump_into_the_middle_of_dead_code_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    moveq #0, d1
    move.w .tbl(pc, d1.w), d1
    jmp .tbl(pc, d1.w) targets(.mid)
.tbl:
    dc.w .mid-.tbl
.top:
    move.w sr, -(sp)
    move.w #$2700, sr
.mid:
    move.w (sp)+, sr
    rts",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Probe x4: a pointer into a reached restore.
#[test]
fn a_pointer_into_a_reached_restore_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    tst.w d0
    beq .x
    move.w sr, -(sp)
    move.w #$2700, sr
.mid:
    move.w (sp)+, sr
    rts
.x:
    lea .mid(pc), a0
    jmp (a0)",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// A data label whose address is taken inside the bracket is not an entry.
#[test]
fn a_data_label_whose_address_is_taken_builds() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    move.w sr, -(sp)
    move.w #$2700, sr
    lea .tbl(pc), a0
    bra .r
.tbl:
    dc.w 1, 2
.r:
    move.w (sp)+, sr
    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

/// The DEBUG assert rail (`subq #2, sp`, a `d(sp)` store, a jump that never
/// returns) inside a masked section is not a way out.
#[test]
fn an_assert_inside_a_masked_section_builds_in_debug() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)\n    move.w #$2700, sr\n    assert.w d0, eq\n    move.w (sp)+, sr\n    rts",
    );
    let debug = vec![("DEBUG".to_string(), 1)];
    assert_eq!(lower_with(&src, debug.clone()), vec![]);
    // The define took effect: the DEBUG build is not the release build.
    let (file, _) = parse_str(&src);
    let opts = |defines| LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines };
    let (m1, _) = lower_module(&file, &opts(debug));
    let (m0, _) = lower_module(&file, &opts(vec![("DEBUG".to_string(), 0)]));
    assert_ne!(format!("{:?}", m1.sections), format!("{:?}", m0.sections), "DEBUG=1 must emit the assert");
}

#[test]
fn the_same_text_for_sr_and_sr_mask() {
    let bare = lower(&routine("clobbers(d0) preserves(sr)", W2));
    let mask = lower(&routine(MASK, W2));
    assert_eq!(bare.len(), 1, "{bare:#?}");
    assert_eq!(mask.len(), 1, "{mask:#?}");
    assert!(bare[0].message.contains("declares `preserves(sr)` but"), "{bare:#?}");
    assert!(mask[0].message.contains("declares `preserves(sr.mask)` but"), "{mask:#?}");
    assert_eq!(bare[0].message.replace("`preserves(sr)`", "`preserves(sr.mask)`"), mask[0].message);
    assert_eq!(bare[0].line, mask[0].line);
}

// ---- Final review M10 / F5: rows of the notes' table that now build ----

/// A write to `sr` before the save that leaves the level alone, every path in
/// order: refused by the line-order test (the save is not before the first
/// write), builds under the walk.
#[test]
fn an_sr_write_before_the_save_with_every_path_in_order_builds() {
    let src = routine(
        MASK,
        "    andi.w #$fffe, sr\n    move.w sr, -(sp)\n    move.w #$2700, sr\n    move.w (sp)+, sr\n    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

/// A write after the last restore that leaves the level alone (probe
/// `q06_mask_neutral_after`).
#[test]
fn a_write_after_the_last_restore_that_leaves_the_level_alone_builds() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)\n    move.w #$2700, sr\n    move.w (sp)+, sr\n    andi.w #$fffe, sr\n    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

#[test]
fn a_mask_neutral_write_with_no_save_builds() {
    let src = routine(MASK, "    andi.w #$fffe, sr\n    rts");
    assert_eq!(lower(&src), vec![]);
}

/// Pinned as it is today (probe `b01_bare_eori_s`): only the CCR advisory fires.
#[test]
fn an_s_bit_write_under_bare_sr_gets_only_the_ccr_advisory() {
    // bare `sr` is mask + ccr; the S and T bits are in neither. Recorded for the
    // owner in docs/QUEUE.md STEPA-P2A-OWNER-NOTES.
    let src = routine("clobbers(d0) preserves(sr)", "    eori.w #$2000, sr\n    rts");
    let d = lower(&src);
    assert_eq!(
        d,
        vec![D {
            level: Level::Warning,
            message: "[proc.ccr-advisory] `P` declares bare `preserves(sr)` (both halves), but its condition \
                      codes are not provably the caller's at return: `eori` affects the condition codes \
                      outside the bracket, declare `preserves(sr.mask)` if only the interrupt mask \
                      round-trips (path-sensitive flag liveness is S2-D7)"
                .to_string(),
            line: line_named(&src, "proc P () clobbers(d0) preserves(sr) {", 0),
        }],
        "{d:#?}"
    );
}
