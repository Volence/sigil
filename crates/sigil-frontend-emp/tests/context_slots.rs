//! `context <name>(param: Code = asm {})` and `with <ctx>(param: asm { … })`,
//! the NAMED SLOT (decision `d-33`, answered `named-slot` by the owner on
//! 2026-09-16).
//!
//! A context author may name a place inside their `acquire`/`release` where a
//! consumer's code goes. The consumer fills it or leaves it out. The point of
//! the construct is that boot's one hand-spelled Z80 bus hold, which interleaves
//! a reset-line flick between the bus request and the grant spin, can stop being
//! the one hold in the ROM that nothing verifies.
//!
//! SO THE BAR IS NOT "IT COMPILES". It is that the three bracket proofs still
//! hold over the slot, and that the consumer's code stays the consumer's. The
//! two sections below are, in order:
//!
//!   1. THE SAFETY PROOFS. The compiler still owns the release on every exit
//!      path taken from inside a slot: an `rts`, a fall-out, a branch out and a
//!      tail call from slot code each fire `[context.escape]`, a branch back
//!      into the slot fires `[context.reacquire]`, and a branch INTO it from
//!      outside fires `[context.entry-skip]`. Each has a paired NEGATIVE
//!      control, because a rule that fires on everything proves nothing.
//!   2. THE AUTHORSHIP SEAM. Slot code is the CONSUMER's: it is charged to the
//!      consumer's proc by the consumer-side lints and is NOT charged to the
//!      context's declaration span in another file.
//!
//! plus the byte-identity pin for the no-argument spelling, which is what the
//! corpus's 22 existing brackets are.

use sigil_frontend_emp::context::ContextFiringKind;
use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// The engine's real bus bracket WITH a pre-grant slot: the shape aeon's
/// `engine/z80_bus.emp` would take to admit `boot.emp`'s reset flick. The slot
/// sits between the bus request and the grant spin, which is where the one
/// statement boot needs has to go, and defaults to the empty code block so a
/// bracket that passes nothing gets today's stream exactly.
const SLOT_CTX: &str = "context z80_stopped(interleave: Code = asm {}) {\n\
     \x20   acquire = asm { move.w #$0100, Z80_BUS_REQUEST } ++ interleave ++ asm { .wait_z80:\n\
     \x20                   btst #0, Z80_BUS_REQUEST\n\
     \x20                   bne .wait_z80 }\n\
     \x20   release = asm { move.w #$0000, Z80_BUS_REQUEST }\n\
     }\n";

/// The SAME context with no parameter at all, the corpus's shape today. Its
/// acquire is spelled as one `asm { }` rather than as a `++` chain, which is
/// deliberate: the byte-identity test below compares the two, so anything the
/// concatenation did differently would show up there.
const PLAIN_CTX: &str = "context z80_stopped {\n\
     \x20   acquire = asm { move.w #$0100, Z80_BUS_REQUEST\n\
     \x20                  .wait_z80:\n\
     \x20                   btst #0, Z80_BUS_REQUEST\n\
     \x20                   bne .wait_z80 }\n\
     \x20   release = asm { move.w #$0000, Z80_BUS_REQUEST }\n\
     }\n";

fn analyze(src: &str) -> ContractReport {
    let (f, diags) = parse_str(src);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "parse diagnostics: {errs:?}");
    analyze_corpus(&[f])
}

fn ctx_count(r: &ContractReport, proc: &str, kind: ContextFiringKind) -> usize {
    r.context_firings.iter().filter(|f| f.proc == proc && f.kind == kind).count()
}

fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    let errs: Vec<_> = perrs.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "unexpected parse diagnostics: {errs:?}");
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    )
}

fn flatten(module: &Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00).unwrap()
}

/// The diagnostics carrying `tag`.
fn tagged<'d>(diags: &'d [Diagnostic], tag: &str) -> Vec<&'d Diagnostic> {
    diags.iter().filter(|d| d.message.contains(tag)).collect()
}

// ---------------------------------------------------------------------------
// 1. THE SAFETY PROOFS: the compiler still owns the release on every exit path
// ---------------------------------------------------------------------------

/// THE HEADLINE. An `rts` written in the SLOT is a return taken between the bus
/// request and the release, which is precisely the hazard the bracket exists to
/// make impossible. It fires `[context.escape]`.
///
/// This is the property `d-33` buys boot: today boot's hand-spelled hold has no
/// check of this kind at all, and a future early exit added between its request
/// and its release would leave the sound chip halted with nothing to say so.
#[test]
fn an_rts_in_the_slot_escapes() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ tst.w d0\n bne .out\n rts }}) {{\n\
                 nop\n\
             }}\n\
         .out:\n\
             rts\n\
         }}\n"
    ));
    assert!(
        ctx_count(&r, "P", ContextFiringKind::Escape) > 0,
        "a return from inside the slot skips the release: {:?}",
        r.context_firings
    );
}

/// NEGATIVE CONTROL for the test above, and it is the test that makes it mean
/// anything. The SAME bracket with straight-line slot code fires NOTHING: so the
/// escape above is about the exit path and not about the slot's mere presence.
#[test]
fn a_straight_line_slot_fires_nothing() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ move.w d7, (a2) }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        r.context_firings.iter().filter(|f| f.proc == "P").count(),
        0,
        "boot's actual shape, one straight-line statement in the slot: {:?}",
        r.context_firings
    );
}

/// A BRANCH OUT of the region taken from the slot is the same escape by a
/// different edge, the shape `bg.emp`'s header describes in prose ("a guard
/// branch taken from inside the bracket would leave the Z80 halted for the rest
/// of the level"), now written in the one place that had no bracket to put it in.
#[test]
fn a_branch_out_of_the_region_from_the_slot_escapes() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ tst.w d0\n bne .skip }}) {{\n\
                 nop\n\
             }}\n\
         .skip:\n\
             rts\n\
         }}\n"
    ));
    assert!(
        ctx_count(&r, "P", ContextFiringKind::Escape) > 0,
        "a branch out of the region from the slot skips the release: {:?}",
        r.context_firings
    );
}

/// A TAIL CALL out of the slot, which leaves by a `TailOut` edge rather than a
/// `Return` or a `Follow`. Named separately because the three edge classes reach
/// the firing through three different arms of `check_regions`.
#[test]
fn a_tail_call_out_of_the_slot_escapes() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc Q () {{ rts }}\n\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ jbra Q }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert!(
        ctx_count(&r, "P", ContextFiringKind::Escape) > 0,
        "a tail out of the slot never reaches the release: {:?}",
        r.context_firings
    );
}

/// A CALL OUT OF THE SLOT IS NOT AN ESCAPE, and this is the control that says
/// the escape rules are about LEAVING rather than about transferring. A `jbsr`
/// comes back, and the edge builders say so: every call mnemonic gets its
/// fall-through and nothing else. Without this test, a rule that fired on any
/// transfer instruction would pass all three escape tests above.
#[test]
fn a_call_out_of_the_slot_is_not_an_escape() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc Q () {{ rts }}\n\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ jbsr Q }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "P", ContextFiringKind::Escape),
        0,
        "a call returns, so it never skips the release: {:?}",
        r.context_firings
    );
}

/// AN EXPORTED LABEL IN THE SLOT IS AN ENTRY POINT THIS PROC'S ITEM LIST CANNOT
/// SEE: it takes the stable `Owner.name` symbol, so any other proc can branch
/// straight past the bus request and run the rest of the hold without taking it.
/// `[context.entry-skip]` fires.
///
/// This is also the test that says the slot's items land INSIDE the region
/// rather than beside it, by a rule that needs no label resolution: the
/// exported-label arm of `check_regions` tests `region.contains(idx)` on the
/// item's own index.
#[test]
fn an_exported_label_in_the_slot_is_an_entry_skip() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ export .mid:\n move.w d7, (a2) }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "P", ContextFiringKind::EntrySkip),
        1,
        "an exported label in the slot is a way into the hold: {:?}",
        r.context_firings
    );
}

/// NEGATIVE CONTROL for the test above: the SAME slot with the label NOT
/// exported fires nothing, so the entry-skip is about the export and not about
/// a label in a slot.
#[test]
fn a_local_label_in_the_slot_is_not_an_entry_skip() {
    let r = analyze(&format!(
        "module m\n{SLOT_CTX}\
         pub proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ .mid:\n move.w d7, (a2) }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        r.context_firings.iter().filter(|f| f.proc == "P").count(),
        0,
        "a private label in the slot reaches nobody: {:?}",
        r.context_firings
    );
}

/// A BODY BRANCH NAMING A SLOT LABEL DOES NOT PASS SILENTLY. A label written
/// inside an `asm { }` VALUE resolves within that value, and the bracket's body
/// is lowered separately, so `bne .mid` in the body does not reach a `.mid` the
/// slot defined. It is not accepted quietly: the branch reads as a transfer out
/// of the region and `[context.escape]` fires as an ERROR.
///
/// PRE-EXISTING AND NOT CAUSED BY THE SLOT: the same is true of a body branch
/// naming the acquire's own `.wait_z80`, but it is the first shape where
/// somebody might reasonably try it, so it is pinned here rather than left to be
/// rediscovered.
#[test]
fn a_body_branch_naming_a_slot_label_is_refused() {
    let (_m, diags) = lower(&format!(
        "module m\n{SLOT_CTX}\
         proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ .mid:\n move.w d7, (a2) }}) {{\n\
                 tst.w d0\n\
                 bne .mid\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    let escapes = tagged(&diags, "[context.escape]");
    assert!(!escapes.is_empty(), "the branch is not silently accepted: {diags:?}");
    assert!(
        escapes.iter().all(|d| d.level == Level::Error),
        "and it is an error, not a warning: {escapes:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. THE AUTHORSHIP SEAM: a consumer's slot code stays the consumer's
// ---------------------------------------------------------------------------

/// THE DEFINITION-SITE CHECK MUST NOT READ CONSUMER CODE. `lower_with`'s
/// `[context.rte-acquire-pushes]` check scans the spliced acquire for a push and
/// reports at the CONTEXT'S declaration span, in another file. A push written in
/// a consumer's slot is not the context author's push, and charging it to them
/// would point the diagnostic at code that does not contain the problem.
#[test]
fn a_push_in_the_slot_is_not_charged_to_the_context_declaration() {
    let rte_ctx = "context ints_off_until_rte(pre: Code = asm {}) {\n\
                   \x20   acquire = asm { move.w #$2700, sr } ++ pre\n\
                   \x20   released_by_rte\n\
                   }\n";
    let src = format!(
        "module m\n{rte_ctx}\
         proc H () clobbers(d0,sr) {{\n\
             with ints_off_until_rte(pre: asm {{ move.w d0, -(sp) }}) {{\n\
                 nop\n\
             }}\n\
             rte\n\
         }}\n"
    );
    let (_m, diags) = lower(&src);
    assert!(
        tagged(&diags, "[context.rte-acquire-pushes]").is_empty(),
        "the consumer's push is not the context's: {diags:?}"
    );

    // POSITIVE CONTROL, so the assertion above is not a vacuous zero: the SAME
    // push written by the CONTEXT AUTHOR, in the acquire itself, still fires.
    let author_push = "context ints_off_until_rte(pre: Code = asm {}) {\n\
                       \x20   acquire = asm { move.w #$2700, sr\n move.w d0, -(sp) } ++ pre\n\
                       \x20   released_by_rte\n\
                       }\n";
    let (_m, diags) = lower(&format!(
        "module m\n{author_push}\
         proc H () clobbers(d0,sr) {{\n\
             with ints_off_until_rte {{\n\
                 nop\n\
             }}\n\
             rte\n\
         }}\n"
    ));
    assert!(
        !tagged(&diags, "[context.rte-acquire-pushes]").is_empty(),
        "the context author's own push still fires: {diags:?}"
    );
}

/// THE CONSUMER-SIDE LINT MUST STILL SEE SLOT CODE, which is the same fact from
/// the other side. `ItemAuthor::Context` is an EXEMPTION from
/// `[proc.sr-undeclared]` at the consumer (a context's SR round trip is proven
/// at its declaration instead), so slot code left wearing that author would slip
/// past the consumer's contract silently. It does not: an SR write in the slot
/// is charged to the proc that wrote it.
#[test]
fn an_sr_write_in_the_slot_is_charged_to_the_consumer() {
    let src = format!(
        "module m\n{SLOT_CTX}\
         proc P () clobbers(d0,d7) {{\n\
             with z80_stopped(interleave: asm {{ move.w #$2700, sr }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    );
    let (_m, diags) = lower(&src);
    assert!(
        !tagged(&diags, "[proc.sr-undeclared]").is_empty(),
        "the consumer's undeclared SR write in the slot is the consumer's: {diags:?}"
    );

    // NEGATIVE CONTROL: the identical proc DECLARING `clobbers(sr)` is clean, so
    // the firing above is about the undeclared write and not about slots.
    let (_m, diags) = lower(&format!(
        "module m\n{SLOT_CTX}\
         proc P () clobbers(d0,d7,sr) {{\n\
             with z80_stopped(interleave: asm {{ move.w #$2700, sr }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    assert!(
        tagged(&diags, "[proc.sr-undeclared]").is_empty(),
        "a declared SR clobber is clean: {diags:?}"
    );
}

/// A CONTEXT'S PARAMETER NAMES BELONG TO THE CONTEXT. The parameter scope wraps
/// each spliced half and not the whole bracket, so a consumer's body goes on
/// meaning what it meant before the context author added a parameter, here a
/// module `const` of the same name, which the body must still see.
#[test]
fn a_parameter_name_does_not_leak_into_the_bracket_body() {
    let src = format!(
        "module m\n{SLOT_CTX}\
         const interleave = 3\n\
         proc P () clobbers(d0,d7) {{\n\
             with z80_stopped {{\n\
                 moveq #interleave, d0\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    );
    let (module, diags) = lower(&src);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "the body's own `interleave` still resolves: {errs:?}");
    // `moveq #3, d0` is `70 03`; the const, not the parameter.
    assert!(
        flatten(&module).windows(2).any(|w| w == [0x70, 0x03]),
        "the body's `interleave` is the module const"
    );
}

// ---------------------------------------------------------------------------
// 3. BYTE IDENTITY: the no-argument spelling is the corpus's 22 sites
// ---------------------------------------------------------------------------

/// THE ACCEPTANCE BAR IN MINIATURE. A bracket that passes no argument to a
/// context that declares a slot emits the IDENTICAL bytes to the same bracket
/// over the same context declared without one. The empty default concatenates
/// nothing, so the slot costs nothing when unused, which is what lets aeon's
/// 22 existing `with z80_stopped` sites go untouched.
#[test]
fn an_unfilled_slot_emits_the_bytes_of_a_context_with_no_slot() {
    let body = "proc P () clobbers(d0,d7) {\n\
                    with z80_stopped {\n\
                        moveq #1, d0\n\
                    }\n\
                    rts\n\
                }\n\
                data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n";
    let (with_slot, d1) = lower(&format!("module m\n{SLOT_CTX}{body}"));
    let (no_slot, d2) = lower(&format!("module m\n{PLAIN_CTX}{body}"));
    let e1: Vec<_> = d1.iter().filter(|d| d.level == Level::Error).collect();
    let e2: Vec<_> = d2.iter().filter(|d| d.level == Level::Error).collect();
    assert!(e1.is_empty() && e2.is_empty(), "clean on both shapes: {e1:?} / {e2:?}");

    let a = flatten(&with_slot);
    let b = flatten(&no_slot);
    assert_eq!(a, b, "a declared-but-unfilled slot moves no bytes");
    // NOT VACUOUS: both really assembled the bracket. The acquire's first line
    // is `move.w #$0100, <abs>`, whose opcode word and immediate are `31 FC`
    // and `01 00`, derived from that source line, not copied from a pin.
    assert_eq!(&a[0..4], &[0x31, 0xFC, 0x01, 0x00], "the bus request heads the proc: {a:02X?}");
}

/// AND THE FILLED SLOT IS THE DIFFERENCE, stated as bytes rather than as a
/// description of bytes: the same bracket with one statement in the slot emits
/// exactly the unfilled stream plus that statement, positioned between the bus
/// request and the grant spin. That position is the whole ask: boot's reset
/// flick has to land after the request and before the grant poll.
#[test]
fn a_filled_slot_adds_exactly_its_own_code_where_the_context_put_it() {
    let shape = |arg: &str| {
        format!(
            "module m\n{SLOT_CTX}\
             proc P () clobbers(d0,d7) {{\n\
                 with z80_stopped{arg} {{\n\
                     moveq #1, d0\n\
                 }}\n\
                 rts\n\
             }}\n\
             data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
        )
    };
    let (empty, _) = lower(&shape(""));
    let (filled, d) = lower(&shape("(interleave: asm { moveq #2, d7 })"));
    let errs: Vec<_> = d.iter().filter(|x| x.level == Level::Error).collect();
    assert!(errs.is_empty(), "the filled shape is clean: {errs:?}");

    let a = flatten(&empty);
    let b = flatten(&filled);
    assert_eq!(b.len(), a.len() + 2, "one `moveq` is two bytes");

    // Derived from the three source lines, in emission order:
    //   `move.w #$0100, <abs.w>` = 31 FC 01 00 xx xx   (6 bytes, the request)
    //   `moveq  #2, d7`          = 7E 02               (the slot)
    //   `btst   #0, <abs.w>`     = 08 38 …             (the grant spin's head)
    assert_eq!(&b[0..4], &[0x31, 0xFC, 0x01, 0x00], "the request: {b:02X?}");
    assert_eq!(&b[6..8], &[0x7E, 0x02], "the slot follows the request: {b:02X?}");
    assert_eq!(&b[8..10], &[0x08, 0x38], "the grant spin follows the slot: {b:02X?}");
    // And WITHOUT the slot the spin sits where the slot went, so the two bytes
    // were inserted rather than appended somewhere convenient.
    assert_eq!(&a[6..8], &[0x08, 0x38], "unfilled, the spin follows the request: {a:02X?}");
}

/// A FALSE COMPTIME GATE TAKES THE SLOT WITH THE ACQUIRE, and this pins it
/// because it is the one place where a consumer's own statement disappears on a
/// condition the consumer wrote.
///
/// `with ctx(slot: …) if COND { … }` with COND false lowers the body verbatim
/// and splices NEITHER half: there is no acquire, no release and no region, so
/// the context is genuinely not held in that shape. The slot lives inside the
/// acquire, so it goes with it. That is coherent rather than surprising once
/// stated (the gate's whole purpose is "this bracket does not exist in that
/// build shape"), but it is not obvious from the spelling, so it is a test and a
/// line in the spec rather than something to be rediscovered in an OFF build.
#[test]
fn a_false_gate_takes_the_slot_with_the_acquire() {
    let shape = |gate: &str| {
        format!(
            "module m\n{SLOT_CTX}\
             proc P () clobbers(d0,d7) {{\n\
                 with z80_stopped(interleave: asm {{ moveq #2, d7 }}) if {gate} {{\n\
                     moveq #1, d0\n\
                 }}\n\
                 rts\n\
             }}\n\
             data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
        )
    };
    let (off, d_off) = lower(&shape("0"));
    let (on, d_on) = lower(&shape("1"));
    for d in [&d_off, &d_on] {
        let errs: Vec<_> = d.iter().filter(|x| x.level == Level::Error).collect();
        assert!(errs.is_empty(), "both shapes are clean: {errs:?}");
    }
    let a = flatten(&off);
    let b = flatten(&on);
    // OFF: the body and the `rts`, and nothing else. `moveq #1, d0` = 70 01,
    // `rts` = 4E 75, both derived from the source lines.
    assert!(a.starts_with(&[0x70, 0x01, 0x4E, 0x75]), "the gated-off shape is the body: {a:02X?}");
    assert!(
        !a.windows(2).any(|w| w == [0x7E, 0x02]),
        "the slot is gone with the acquire it lived in: {a:02X?}"
    );
    // ON: the request, then the slot, as the filled-slot test already pins.
    assert_eq!(&b[0..4], &[0x31, 0xFC, 0x01, 0x00], "gated on, the request is emitted: {b:02X?}");
    assert_eq!(&b[6..8], &[0x7E, 0x02], "gated on, the slot follows it: {b:02X?}");
}

// ---------------------------------------------------------------------------
// 4. THE TWO DIAGNOSTICS ABOUT CONSUMER CODE THAT WOULD OTHERWISE VANISH
// ---------------------------------------------------------------------------

/// A PARAMETER THE CONTEXT NEVER SPLICES ACCEPTS THE CALLER'S INSTRUCTIONS AND
/// DROPS THEM. At a bus hold that is the difference between the sound chip's
/// reset line being released and not, so it is an error rather than a silence.
#[test]
fn a_slot_the_context_never_splices_is_an_error() {
    let unused = "context z80_stopped(interleave: Code = asm {}) {\n\
                  \x20   acquire = asm { move.w #$0100, Z80_BUS_REQUEST }\n\
                  \x20   release = asm { move.w #$0000, Z80_BUS_REQUEST }\n\
                  }\n";
    let (_m, diags) = lower(&format!(
        "module m\n{unused}\
         proc P () clobbers(d7) {{\n\
             with z80_stopped(interleave: asm {{ move.w d7, (a2) }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    assert!(
        !tagged(&diags, "[context.slot-dropped]").is_empty(),
        "code passed to a parameter nothing splices is assembled into nothing: {diags:?}"
    );

    // NEGATIVE CONTROL: the SAME argument to a context that DOES splice the
    // parameter is silent, so the error is about the dropping and not about the
    // argument.
    let (_m, diags) = lower(&format!(
        "module m\n{SLOT_CTX}\
         proc P () clobbers(d7) {{\n\
             with z80_stopped(interleave: asm {{ move.w d7, (a2) }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    assert!(tagged(&diags, "[context.slot-dropped]").is_empty(), "a spliced slot is silent: {diags:?}");
}

/// AN EMPTY ARGUMENT IS NOT A DROPPED ONE. `asm {}` carries no instruction, so a
/// context that never splices the parameter has dropped nothing and says nothing.
#[test]
fn an_empty_slot_argument_is_not_reported_as_dropped() {
    let unused = "context z80_stopped(interleave: Code = asm {}) {\n\
                  \x20   acquire = asm { move.w #$0100, Z80_BUS_REQUEST }\n\
                  \x20   release = asm { move.w #$0000, Z80_BUS_REQUEST }\n\
                  }\n";
    let (_m, diags) = lower(&format!(
        "module m\n{unused}\
         proc P () clobbers(d7) {{\n\
             with z80_stopped(interleave: asm {{}}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    assert!(tagged(&diags, "[context.slot-dropped]").is_empty(), "nothing was dropped: {diags:?}");
}

/// ARGUMENTS TO A CONTEXT THAT DECLARES NO PARAMETERS are named at the bracket,
/// because the reader's question is "where do I declare one" and the answer is
/// in a different file from the bracket they are looking at.
#[test]
fn arguments_to_a_parameterless_context_are_refused_by_name() {
    let (_m, diags) = lower(&format!(
        "module m\n{PLAIN_CTX}\
         proc P () clobbers(d7) {{\n\
             with z80_stopped(interleave: asm {{ move.w d7, (a2) }}) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    assert!(
        !tagged(&diags, "[context.no-parameters]").is_empty(),
        "a context with no parameters has nothing to pass to: {diags:?}"
    );
}

/// A `Code` PARAMETER HANDED SOMETHING ELSE is named at the ARGUMENT. Without
/// this the failure surfaces as `[context.not-code]` against the context's
/// acquire, at the declaration, in another file, naming neither the bracket nor
/// the value.
#[test]
fn a_non_code_argument_to_a_code_slot_is_named_at_the_argument() {
    let (_m, diags) = lower(&format!(
        "module m\n{SLOT_CTX}\
         proc P () clobbers(d7) {{\n\
             with z80_stopped(interleave: 7) {{\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n\
         data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
    ));
    assert!(
        !tagged(&diags, "[context.slot-not-code]").is_empty(),
        "an Int is not a Code slot's argument: {diags:?}"
    );
}
