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
fn a_call_to_a_masking_routine_is_refused() {
    // Piece 2b closes this (in 2a this was `a_call_to_a_masking_routine_still_builds`).
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
    assert_eq!(
        lower(src),
        vec![err(not_the_callers("sr.mask", "7"), line_named(src, "rts", 0))],
    );
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

/// Measured, not assumed: since piece 2b, the `sr_level` walk runs in the
/// module-wide post-pass after per-proc `context` and `stack` checks.
const PINNED_WITH_INTS_OFF_EARLY_RETURN: &[(Level, &str)] = &[
    (Level::Error, "[context.escape]"),
    (Level::Error, "[stack.unbalanced]"),
    (Level::Error, "[proc.preserves-sr-unbalanced]"),
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
    assert_eq!(
        d,
        vec![
            err(untracked(), line_named(&src, "jmp .tbl(pc, d1.w) targets(.mid)", 0)),
            err(untracked(), line_named(&src, "rts", 0)),
        ],
        "{d:#?}"
    );
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

const AT_X: &str = "
.x:
    move.w sr, -(sp)
    move.w #$2700, sr
    move.w (sp)+, sr
    rts";

/// Probe y1 (fix wave, F1c): `jmp 6(a0)` lands on the restore at `.x+6`.
#[test]
fn a_displaced_jump_after_taking_an_address_is_refused() {
    let src = routine("clobbers(d0, d1, a0) preserves(sr.mask)", &format!("    lea .x(pc), a0\n    jmp 6(a0){AT_X}"));
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// Probe y3: `jmp .x+6(pc)` on one path. Since piece 2b, the offset jump
/// itself is also reported as an untracked transfer, beside `.x`'s `rts`.
#[test]
fn a_pc_relative_jump_with_an_offset_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        &format!("    tst.w d0\n    beq .y\n    jmp .x+6(pc)\n.y:\n    rts{AT_X}"),
    );
    let d = lower(&src);
    assert_eq!(
        d,
        vec![
            err(untracked(), line_named(&src, "jmp .x+6(pc)", 0)),
            err(untracked(), line_named(&src, "rts", 1)),
        ],
        "{d:#?}"
    );
}

/// The offset hidden in an immediate expression (`ImmLink`).
#[test]
fn an_immediate_address_with_an_offset_is_refused() {
    let src = routine("clobbers(d0, d1, a0) preserves(sr.mask)", &format!("    movea.l #.x+6, a0\n    jmp (a0){AT_X}"));
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

// ---- Re-review N2 and N3 ----

/// The reached restore of the N2 probes, entered from `.via`.
fn into_reached_restore(via: &str) -> String {
    routine(
        "clobbers(d0, d1, a0, a1) preserves(sr.mask)",
        &format!(
            "    tst.w d0
    beq .via
    move.w sr, -(sp)
    move.w #$2700, sr
.r:
    move.w (sp)+, sr
    rts
.via:
{via}"
        ),
    )
}

/// Probe b01 (re-review N2): `jmp .r(pc)` straight into a reached restore.
#[test]
fn a_pc_relative_jump_into_a_reached_restore_is_refused() {
    let src = into_reached_restore("    jmp .r(pc)");
    let d = lower(&src);
    assert_eq!(
        d,
        vec![
            err(untracked(), line_named(&src, "rts", 0)),
            err(untracked(), line_named(&src, "jmp .r(pc)", 0)),
        ],
        "{d:#?}"
    );
}

/// Probe b05 (re-review N2): the same through `jsr .r(pc)`. Since piece 2b,
/// `.r`'s `Unknown` + `touches_caller_stack` summary flags the call site and
/// propagates back into `.via`'s `rts`.
#[test]
fn a_pc_relative_call_into_a_reached_restore_is_refused() {
    let src = into_reached_restore("    jsr .r(pc)\n    rts");
    let d = lower(&src);
    assert_eq!(
        d,
        vec![
            err(untracked(), line_named(&src, "rts", 0)),
            err(untracked(), line_named(&src, "jsr .r(pc)", 0)),
            err(untracked(), line_named(&src, "rts", 1)),
        ],
        "{d:#?}"
    );
}

/// Probe r2: a balanced local helper called as `jsr .h(pc)` builds.
#[test]
fn a_balanced_helper_called_pc_relative_builds() {
    let src = routine(
        "clobbers(d0, d1, a0) preserves(sr.mask)",
        "    jsr .h(pc)
    rts
.h:
    move.w sr, -(sp)
    move.w #$2700, sr
    move.w (sp)+, sr
    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

/// Probe u2_btst_pop_min (re-review N3): `btst #0, (sp)+` has no size suffix and
/// pops 2 bytes on the machine; counted as 4, the save looked like the top and
/// the routine returned at level 7 with a clean build.
#[test]
fn a_pop_on_a_sizeless_instruction_is_refused() {
    let src = routine(
        "clobbers(d0, d1, a0, a1) preserves(sr.mask)",
        "    move.w sr, -(sp)
    move.w #$2700, sr
    move.l #$27002700, -(sp)
    btst #0, (sp)+
    move.w (sp)+, sr
    addq.l #2, sp
    rts",
    );
    let d = lower(&src);
    assert_eq!(d, vec![err(untracked(), line_named(&src, "rts", 0))], "{d:#?}");
}

/// The object-dispatch shape must not be refused.
#[test]
fn a_masked_displaced_call_builds() {
    let src = routine(
        "clobbers(d0, d1, a0, a1) preserves(sr.mask)",
        "    move.w sr, -(sp)\n    move.w #$2700, sr\n    jsr 4(a1)\n    move.w (sp)+, sr\n    rts",
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

// ---- Step A piece 2b: Task 1 local helpers (red-first) ----

/// STEPA-P2B-CALLS M5 (`h01_helper_rewrites_save`): a local helper called
/// between save and restore rewrites the caller's saved word via `4(sp)`.
#[test]
fn local_helper_rewriting_caller_saved_sr_is_refused() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    bsr .h
    move.w (sp)+, sr
    rts
.h:
    move.w #$2700, 4(sp)
    rts",
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.line == line_named(&src, "rts", 0)
            && x.message == untracked()),
        "expected Untracked error at caller's rts for h01, got: {d:#?}"
    );
}

/// STEPA-P2B-CALLS M5 (`q08_pcrel_helper_stack`): same through `jsr .h(pc)`.
#[test]
fn pcrel_local_helper_rewriting_caller_saved_sr_is_refused() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    jsr .h(pc)
    move.w (sp)+, sr
    rts
.h:
    move.w #$2700, 4(sp)
    rts",
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.line == line_named(&src, "rts", 0)
            && x.message == untracked()),
        "expected Untracked error at caller's rts for q08, got: {d:#?}"
    );
}

/// STEPA-P2B-CALLS re-review (`c01_helper_drops_return_bsr`): a local helper
/// pops its own return address and the caller's save, returning at level 7.
#[test]
fn local_helper_dropping_return_and_save_via_bsr_is_refused() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    bsr .h
    move.w (sp)+, sr
    rts
.h:
    addq.l #6, sp
    rts",
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.line == line_named(&src, "rts", 0)
            && x.message == untracked()),
        "expected Untracked error at caller's rts for c01, got: {d:#?}"
    );
}

/// STEPA-P2B-CALLS re-review (`c02_helper_drops_return_ptr`): same through
/// `lea .h(pc), a0 / jsr (a0)` taken BEFORE the save.
#[test]
fn local_helper_dropping_return_via_pointer_is_refused() {
    let src = routine(
        "clobbers(d0, a0) preserves(sr.mask)",
        "    lea .h(pc), a0
    move.w sr, -(sp)
    move.w #$2700, sr
    jsr (a0)
    move.w (sp)+, sr
    rts
.h:
    addq.l #6, sp
    rts",
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.line == line_named(&src, "rts", 0)
            && x.message == untracked()),
        "expected Untracked error at caller's rts for c02, got: {d:#?}"
    );
}

/// STEPA-P2B-CALLS re-review (`c03_helper_jumps_back_into_caller_tail`):
/// helper drops return+save and branches to `.out: rts`.
#[test]
fn local_helper_dropping_return_and_branching_to_tail_is_refused() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    bsr .h
    move.w (sp)+, sr
.out:
    rts
.h:
    addq.l #6, sp
    bra .out",
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.line == line_named(&src, "rts", 0)
            && x.message == untracked()),
        "expected Untracked error at .out rts for c03, got: {d:#?}"
    );
}

/// Control (C1): a local helper with internal stack loss called BEFORE the
/// save (no saved SR on the stack yet) must NOT poison a later save/restore.
#[test]
fn local_helper_with_stack_loss_before_save_still_builds() {
    let src = routine(
        MASK,
        "    bsr .h
    move.w sr, -(sp)
    move.w #$2700, sr
    move.w (sp)+, sr
    rts
.h:
    subq.l #2, sp
    addq.w #1, (sp)
    addq.l #2, sp
    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

/// Control: a balanced local helper called INSIDE a save/restore bracket builds.
#[test]
fn balanced_local_helper_inside_mask_still_builds() {
    let src = routine(
        MASK,
        "    move.w sr, -(sp)
    move.w #$2700, sr
    bsr .h
    move.w (sp)+, sr
    rts
.h:
    move.w d0, -(sp)
    move.w (sp)+, d0
    rts",
    );
    assert_eq!(lower(&src), vec![]);
}

// ---- Step A piece 2b: Task 2 same-file inter-procedural propagation ----

#[test]
fn a_two_hop_call_chain_to_a_masking_routine_is_refused() {
    let src = "module m
proc P () clobbers(d0) preserves(sr.mask) {
    jbsr M
    rts
}
proc M () clobbers(d0) {
    jbsr Q
    rts
}
proc Q () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    assert_eq!(
        lower(src),
        vec![err(not_the_callers("sr.mask", "7"), line_named(src, "rts", 0))],
    );
}

#[test]
fn a_call_to_an_undeclared_sr_clean_routine_builds() {
    let src = "module m
proc P () clobbers(d0) preserves(sr.mask) {
    move.w sr, -(sp)
    move.w #$2700, sr
    jbsr Helper
    move.w (sp)+, sr
    rts
}
proc Helper () clobbers(d0) {
    moveq #1, d0
    rts
}
";
    assert_eq!(lower(src), vec![]);
}

/// STEPA-P2B-CALLS M4 (`t01`): mid-body tail to a routine that masks.
#[test]
fn a_mid_body_tail_to_a_masking_routine_is_refused() {
    let src = "module m
proc P () clobbers(d0) preserves(sr.mask) {
    tst.w d0
    beq .x
    jmp Other
.x:
    rts
}
proc Other () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    assert_eq!(
        lower(src),
        vec![err(not_the_callers("sr.mask", "7"), line_named(src, "jmp Other", 0))],
    );
}

/// STEPA-P2B-CALLS M4 (`t02`): conditional branch out to a routine that masks.
#[test]
fn a_conditional_branch_out_to_a_masking_routine_is_refused() {
    let src = "module m
proc P () clobbers(d0) preserves(sr.mask) {
    tst.w d0
    beq Other
    rts
}
proc Other () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    assert_eq!(
        lower(src),
        vec![err(not_the_callers("sr.mask", "7"), line_named(src, "beq Other", 0))],
    );
}

#[test]
fn extern_clobbers_sr_is_refused_and_noreturn_extern_inside_mask_builds() {
    let bad = "module m
extern proc Masker () clobbers(sr)
proc P () clobbers(d0) preserves(sr.mask) {
    jsr Masker
    rts
}
";
    assert_eq!(
        lower(bad),
        vec![err(untracked(), line_named(bad, "rts", 0))],
    );

    let ok_div = "module m
@noreturn
extern proc Die () clobbers(sr)
proc P () clobbers(d0) preserves(sr.mask) {
    move.w sr, -(sp)
    move.w #$2700, sr
    tst.w d0
    beq .ok
    jsr Die
.ok:
    move.w (sp)+, sr
    rts
}
";
    assert_eq!(lower(ok_div), vec![]);
}

#[test]
fn typed_indirect_call_clobbering_sr_is_refused_and_clean_hook_builds() {
    let bad = "module m
type BadHook = proc () clobbers(sr)
proc P () clobbers(a0) preserves(sr.mask) {
    jsr (a0) as BadHook
    rts
}
";
    assert_eq!(
        lower(bad),
        vec![err(untracked(), line_named(bad, "rts", 0))],
    );

    let ok_hook = "module m
type CleanHook = proc () clobbers(d0)
proc P () clobbers(d0, a0) preserves(sr.mask) {
    jsr (a0) as CleanHook
    rts
}
";
    assert_eq!(lower(ok_hook), vec![]);
}

#[test]
fn continuation_rte_and_resumable_jmp_callees_build() {
    let src = "module m
proc Wait () clobbers(d0, a3) preserves(sr.mask) {
    jbsr Process
    rts
}
proc Process () clobbers(d0, a3) {
    tst.w d0
    beq .resume
    jbra Decoder
.resume:
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
    assert_eq!(lower(src), vec![]);
}

#[test]
fn bare_preserves_sr_calling_masker_emits_mask_error_without_ccr_advisory() {
    let src = "module m
proc P () clobbers(d0) preserves(sr) {
    moveq #1, d0
    jsr Q
    rts
}
proc Q () clobbers(sr) {
    move.w #$2700, sr
    rts
}
";
    assert_eq!(
        lower(src),
        vec![err(not_the_callers("sr", "7"), line_named(src, "rts", 0))],
    );
}

// ---- Step A piece 2b: Task 3 Rule 3 (`with <ctx> { }` level hold) & context round-trip ----

#[test]
fn rule3_direct_sr_write_inside_with_ints_off_is_refused_without_preserves_sr() {
    let src = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0, sr) {{
    with ints_off {{
        move.w #$2300, sr
        moveq #0, d0
    }}
    rts
}}
"
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.message.starts_with("[proc.with-sr-level-broken]")
            && x.line == line_named(&src, "move.w #$2300, sr", 0)),
        "expected [proc.with-sr-level-broken] on the offending sr write: {d:#?}"
    );
    // Must not cascade-blame the innocent instruction after it.
    assert_eq!(
        d.iter().filter(|x| x.message.starts_with("[proc.with-sr-level-broken]")).count(),
        1,
        "expected a single firing, not a cascade: {d:#?}"
    );
}

#[test]
fn rule3_call_to_unmasker_inside_with_ints_off_is_refused() {
    let src = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0) {{
    with ints_off {{
        jbsr Unmask
    }}
    rts
}}
proc Unmask () clobbers(sr) {{
    move.w #$2000, sr
    rts
}}
"
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.message.starts_with("[proc.with-sr-level-broken]")
            && x.message.contains("Unmask")
            && x.line == line_named(&src, "jbsr Unmask", 0)),
        "expected [proc.with-sr-level-broken] naming Unmask: {d:#?}"
    );
}

#[test]
fn rule3_call_to_preserves_sr_callee_that_temporarily_unmasks_is_refused() {
    let src = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0) {{
    with ints_off {{
        jbsr UnmaskBriefly
    }}
    rts
}}
proc UnmaskBriefly () clobbers(d0) preserves(sr.mask) {{
    move.w sr, -(sp)
    move.w #$2000, sr
    moveq #1, d0
    move.w (sp)+, sr
    rts
}}
"
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.message.starts_with("[proc.with-sr-level-broken]")
            && x.message.contains("UnmaskBriefly")
            && x.line == line_named(&src, "jbsr UnmaskBriefly", 0)),
        "expected [proc.with-sr-level-broken] catching transient unmask via body_levels: {d:#?}"
    );
}

#[test]
fn rule3_call_to_critical_section_callee_staying_at_level_7_builds() {
    let src = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0) {{
    with ints_off {{
        jbsr QueueDMA
    }}
    rts
}}
proc QueueDMA () clobbers(d0) preserves(sr.mask) {{
    move.w sr, -(sp)
    ori.w #$0700, sr
    moveq #1, d0
    move.w (sp)+, sr
    rts
}}
"
    );
    assert_eq!(lower(&src), vec![]);
}

#[test]
fn rule3_released_by_rte_clean_and_broken() {
    let ctx = "context ints_off_until_rte {
    acquire = asm { ori.w #$0700, sr }
    released_by_rte
}
";
    let clean = format!(
        "module m\n{ctx}proc VBlank () clobbers(d0) {{
    with ints_off_until_rte {{
        moveq #0, d0
        rte
    }}
}}
"
    );
    assert_eq!(lower(&clean), vec![]);

    let broken = format!(
        "module m\n{ctx}proc VBlank () clobbers(d0, sr) {{
    with ints_off_until_rte {{
        move.w #$2300, sr
        rte
    }}
}}
"
    );
    let d = lower(&broken);
    assert!(
        d.iter().any(|x| x.level == Level::Error
            && x.message.starts_with("[proc.with-sr-level-broken]")
            && x.line == line_named(&broken, "move.w #$2300, sr", 0)),
        "expected [proc.with-sr-level-broken] inside released_by_rte bracket: {d:#?}"
    );
}

#[test]
fn rule3_non_sr_context_is_untouched() {
    let src = "module m
context dummy {
    acquire = asm { nop }
    release = asm { nop }
}
proc P () clobbers(d0, sr) {
    with dummy {
        move.w #$2300, sr
        moveq #0, d0
    }
    rts
}
";
    assert_eq!(lower(src), vec![]);
}

#[test]
fn context_pair_mismatched_stack_warns_sr_undeclared() {
    // Old linear `sr_writes_round_trip` missed a stack push between save and restore.
    let src = "module m
context bad_stack {
    acquire = asm {
        move.w sr, -(sp)
        move.w d0, -(sp)
        ori.w #$0700, sr
    }
    release = asm {
        move.w (sp)+, sr
        move.w (sp)+, d0
    }
}
proc P () clobbers(d0) {
    with bad_stack {
        nop
    }
    rts
}
";
    let d = lower(src);
    assert!(
        d.iter().any(|x| x.message.starts_with("[proc.sr-undeclared]")),
        "mismatched stack in context acquire/release must warn [proc.sr-undeclared]: {d:#?}"
    );
}

#[test]
fn rule3_nested_non_sr_bracket_and_assert_do_not_fire_with_sr_level_broken() {
    let nested = format!(
        "module m\n{INTS_OFF}context dummy {{
    acquire = asm {{ nop }}
    release = asm {{ nop }}
}}
proc P () clobbers(d0) {{
    with ints_off {{
        with dummy {{
            moveq #0, d0
        }}
    }}
    rts
}}
"
    );
    assert_eq!(lower(&nested), vec![]);

    // `assert` desugars to an AssertDesugar transfer; Rule 3 must never blame it.
    let with_assert = format!(
        "module m\n{INTS_OFF}proc P () clobbers(d0) {{
    with ints_off {{
        assert.w d0, eq
        moveq #0, d0
    }}
    rts
}}
"
    );
    let d = lower_with(&with_assert, vec![("DEBUG".to_string(), 1)]);
    assert!(
        !d.iter().any(|x| x.message.starts_with("[proc.with-sr-level-broken]")),
        "AssertDesugar must not trigger Rule 3: {d:#?}"
    );
}

// ---- Step A piece 2b: blind-review regression tests (C1-C3, I1-I3, M3) ----

/// Review C1: `jmp .done(pc)` and `jmp (.done).l` while masked must NOT default
/// to DIVERGES (which would fail open on both `preserves(sr.mask)` and `with ints_off`).
#[test]
fn review_c1_pcrel_and_abs_local_tail_while_masked_is_refused() {
    let pcrel = routine(
        MASK,
        "    move.w #$2700, sr
    jmp .done(pc)
.done:
    rts",
    );
    let d_pc = lower(&pcrel);
    assert!(
        d_pc.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "jmp .done(pc) while masked must be refused: {d_pc:#?}"
    );

    let abs = routine(
        MASK,
        "    move.w #$2700, sr
    jmp (.done).l
.done:
    rts",
    );
    let d_abs = lower(&abs);
    assert!(
        d_abs.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "jmp (.done).l while masked must be refused: {d_abs:#?}"
    );
}

/// Review C2: `touches_caller_stack` must propagate transitively through an
/// intermediate proc `M` that has no SR saves of its own.
#[test]
fn review_c2_two_hop_caller_stack_rewrite_is_refused() {
    let src = "module m
proc P () clobbers(d0) preserves(sr.mask) {
    move.w sr, -(sp)
    move.w #$2700, sr
    jsr M
    move.w (sp)+, sr
    rts
}
proc M () clobbers(d0) {
    jsr H
    rts
}
proc H () clobbers(d0) {
    move.w #$2700, 8(sp)
    rts
}
";
    let d = lower(src);
    assert!(
        d.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "2-hop caller-stack overwrite must be refused: {d:#?}"
    );
}

/// Review C3: a stack-corrupting helper (`addq.l #4, sp; rts`) called while
/// masked and followed by diverging code must still be refused at the call site.
#[test]
fn review_c3_stack_drop_helper_before_divergent_code_is_refused() {
    let src = routine(
        MASK,
        "    move.w #$2700, sr
    bsr .h
.dead:
    bra .dead
.h:
    addq.l #4, sp
    rts",
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "c01 before divergent loop must still be refused: {d:#?}"
    );
}

/// Review I1: an unreached masking label/block placed after a `@noreturn` call
/// must still be walked and refused as an unreached entry.
#[test]
fn review_i1_unreached_masking_code_after_noreturn_call_is_refused() {
    let src = "module m
@noreturn
extern proc Die () clobbers(d0)
proc P () clobbers(d0) preserves(sr.mask) {
    jsr Die
.hidden:
    move.w #$2700, sr
    rts
}
";
    let d = lower(src);
    assert!(
        d.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "unreached masking block after @noreturn call must be refused: {d:#?}"
    );
}

/// Review I2: a callee with an enumerated `targets(...)` computed jump whose arm
/// masks or unmasks must be caught in both `preserves(sr.mask)` and `with ints_off`.
#[test]
fn review_i2_callee_enumerated_dispatch_targets_are_summarized() {
    let src = format!(
        "module m\n{INTS_OFF}proc Caller () clobbers(d0, d1) preserves(sr.mask) {{
    jsr Dispatch
    rts
}}
proc Bracketed () clobbers(d0, d1) {{
    with ints_off {{
        jsr DispatchUnmask
    }}
    rts
}}
proc Dispatch () clobbers(d0, d1, sr) {{
    jmp .tbl(pc, d1.w) targets(.a, .b)
.tbl:
    nop
.a:
    rts
.b:
    move.w #$2700, sr
    rts
}}
proc DispatchUnmask () clobbers(d0, d1, sr) {{
    jmp .tbl(pc, d1.w) targets(.a, .b)
.tbl:
    nop
.a:
    rts
.b:
    move.w #$2000, sr
    rts
}}
"
    );
    let d = lower(&src);
    assert!(
        d.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")
            && x.message.contains("`Caller`")),
        "enumerated targets() masking arm must fail Caller: {d:#?}"
    );
    assert!(
        d.iter().any(|x| x.message.starts_with("[proc.with-sr-level-broken]")
            && x.message.contains("`Bracketed`")),
        "enumerated targets() unmasking arm must fail Bracketed: {d:#?}"
    );
}

/// Review I3: `extern proc` and contract `type` with `out(sr.mask)` or `inout(sr.mask)`
/// license SR mask writes and must be treated as UNKNOWN.
#[test]
fn review_i3_extern_and_type_out_sr_mask_are_refused() {
    let ext_src = "module m
extern proc SetIrq () out(sr.mask)
proc P () clobbers(d0) preserves(sr.mask) {
    jsr SetIrq
    rts
}
";
    let d_ext = lower(ext_src);
    assert!(
        d_ext.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "extern proc out(sr.mask) must fail preserves(sr.mask): {d_ext:#?}"
    );

    let ty_src = "module m
type SetIrqHook = proc () out(sr.mask)
proc P () clobbers(a0) preserves(sr.mask) {
    jsr (a0) as SetIrqHook
    rts
}
";
    let d_ty = lower(ty_src);
    assert!(
        d_ty.iter().any(|x| x.message.starts_with("[proc.preserves-sr-unbalanced]")),
        "typed indirect out(sr.mask) must fail preserves(sr.mask): {d_ty:#?}"
    );
}
