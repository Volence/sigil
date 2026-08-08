//! `out(dN: T)` — the WIDTH half of a typed register result (§G4.5).
//!
//! A bare `out(rN)` claims all 32 bits; a typed one claims `sizeof(T)` and is
//! satisfied by a write that wide OR WIDER. These gates run the whole corpus walk
//! (parse → `analyze_corpus` → `[proc.out-unverified]`) rather than calling the
//! dataflow directly, because the declared width has to TRAVEL — parser →
//! `out_types` → the corpus width map → the proc's own obligation AND every
//! caller's credit — and a unit test of the last step alone would pass over a map
//! that was never built.
//!
//! Every gate here is a PAIR: the typed body that must verify, and a body one
//! width narrower that must still fire. A single-sided assert would go quietly
//! true under a checker that stopped charging widths at all.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;

/// Parse each source (demanding a clean parse) and run the corpus contract walk.
fn analyze(srcs: &[&str]) -> ContractReport {
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

/// The lowering diagnostics for one source — the per-file contract gate, which is
/// where an author meets a refused declaration. The corpus walk above never sees
/// them, so a refusal needs its own path.
fn lower_diag_messages(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "the source must PARSE; a refusal is a lowering check: {perrs:?}");
    let (_, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    diags.into_iter().map(|d| d.message).collect()
}

/// Does `[proc.out-unverified]` fire on `(proc, reg)`?
fn out_fires(r: &ContractReport, proc: &str, reg: &str) -> bool {
    r.out_firings.iter().any(|f| f.proc == proc && f.reg == reg)
}

/// The `reason` text of the firing on `(proc, reg)`.
fn out_reason(r: &ContractReport, proc: &str, reg: &str) -> String {
    r.out_firings
        .iter()
        .find(|f| f.proc == proc && f.reg == reg)
        .unwrap_or_else(|| panic!("no firing on {proc}::{reg}; firings: {:?}", r.out_firings))
        .reason
        .clone()
}

/// Is `(proc, reg)` in the VERIFIED-out map — the fixpoint's positive answer?
///
/// The gates below assert this rather than the absence of a firing. "No firing"
/// is also what a proc the walk never parsed produces, and what a checker that
/// stopped charging outs entirely produces; a name in the verified map is a
/// statement that the claim was examined and carried.
fn verified(r: &ContractReport, proc: &str, reg: &str) -> bool {
    r.verified_uncond_out.get(proc).is_some_and(|s| s.contains(reg))
}

/// Is `(proc, reg if cc)` in the VERIFIED CONDITIONAL-out map? A conditional out
/// lands there and never in the unconditional map, so a gate on a `... if cc`
/// declaration must ask this one or it is asking about a key that cannot exist.
fn verified_cond(r: &ContractReport, proc: &str, reg: &str, cc: &str) -> bool {
    r.verified_cond_out
        .get(proc)
        .is_some_and(|v| v.iter().any(|(x, c)| x == reg && c == cc))
}

/// Assert the walk saw every named proc at all — the guard for the FIRING
/// asserts, which a skipped module would otherwise turn into a silent failure of
/// the wrong kind.
fn assert_subjects(r: &ContractReport, procs: &[&str]) {
    for p in procs {
        assert!(
            r.verified_uncond_out.contains_key(*p),
            "the corpus walk never saw `{p}` — every assert below it proves nothing"
        );
    }
}

// === the claim's own width =================================================

/// The pair that defines the feature: ONE body, TWO declarations. `move.w` leaves
/// d0's high word stale, so the bare 32-bit claim is unprovable and the 16-bit
/// claim is exactly right.
///
/// MUTANT: make `OutWidths::required` ignore its map and always answer `L`.
/// `Typed` then fires and this test goes RED. Run: confirmed RED.
#[test]
fn a_word_write_proves_a_word_claim_and_not_a_bare_one() {
    let r = analyze(&[
        "module m\n\
         proc Bare () out(d0) {\n move.w #1, d0\n rts\n}\n\
         proc Typed () out(d0: u16) {\n move.w #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Bare", "Typed"]);
    assert!(out_fires(&r, "Bare", "d0"), "a `.w` write cannot prove all 32 bits");
    assert!(verified(&r, "Typed", "d0"), "firings: {:?}", r.out_firings);
}

/// Narrowing is not a blanket excuse: a `.b` write still does not reach a `u16`
/// claim. The `u8` sibling shows the same body DOES satisfy the honest claim, so
/// the fire is about the width and not about the body.
///
/// MUTANT: drop the width comparison in `check_return` (credit any production).
/// `TooWide` stops firing and this test goes RED. Run: confirmed RED.
#[test]
fn a_byte_write_does_not_reach_a_word_claim() {
    let r = analyze(&[
        "module m\n\
         proc TooWide () out(d0: u16) {\n move.b #1, d0\n rts\n}\n\
         proc Honest () out(d0: u8) {\n move.b #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["TooWide", "Honest"]);
    assert!(out_fires(&r, "TooWide", "d0"), "`.b` cannot prove 16 bits");
    assert!(verified(&r, "Honest", "d0"), "firings: {:?}", r.out_firings);
}

/// "Or wider" is the credit rule, so a `moveq`/`.l` production satisfies a byte
/// claim. Without this a narrowing type would REJECT the honest full-width bodies
/// it sits beside — `Section_FlatIDXY`'s `moveq #0, d0` under `out(d0:
/// SectionId)` is the live one.
///
/// MUTANT: change the comparison from `have < need` to `have != need`. Both procs
/// fire and this test goes RED. Run: confirmed RED.
#[test]
fn a_wider_write_satisfies_a_narrower_claim() {
    let r = analyze(&[
        "module m\n\
         proc ViaMoveq () out(d0: u8) {\n moveq #1, d0\n rts\n}\n\
         proc ViaLong () out(d0: u8) {\n move.l #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["ViaMoveq", "ViaLong"]);
    assert!(verified(&r, "ViaMoveq", "d0"), "firings: {:?}", r.out_firings);
    assert!(verified(&r, "ViaLong", "d0"), "firings: {:?}", r.out_firings);
}

/// At a merge the claim is charged against the NARROWER path — a MUST analysis
/// answers with what both paths deliver. The bare sibling proves the join is
/// really meeting (if it took the wider path the bare claim would verify off the
/// `.l` arm alone).
///
/// MUTANT: make `join` take the wider of the two widths. `Bare` stops firing and
/// this test goes RED. Run: confirmed RED.
#[test]
fn a_merge_charges_the_narrower_incoming_width() {
    let body = "\n tst.b d1\n beq .wide\n move.w #1, d0\n bra .out\n.wide:\n move.l #2, d0\n.out:\n rts\n";
    let r = analyze(&[&format!(
        "module m\n\
         proc Bare (d1: u8) out(d0) {{{body}}}\n\
         proc Typed (d1: u8) out(d0: u16) {{{body}}}\n"
    )]);
    assert_subjects(&r, &["Bare", "Typed"]);
    assert!(out_fires(&r, "Bare", "d0"), "one arm is only `.w` — 32 bits are unprovable");
    assert!(verified(&r, "Typed", "d0"), "firings: {:?}", r.out_firings);
}

/// Production is gen-only ACROSS widths: a later `.b` write does not retract the
/// bytes an earlier `moveq` already wrote. This is the ruling the four
/// `Collision_Probe*` bodies rest on — their surviving path is `moveq #16, d0`
/// followed by `sub.w d3, d0`, and it must not be charged as a `.w`-only
/// production.
///
/// MUTANT: make `produce` overwrite the recorded width instead of keeping the
/// widest. `Narrowed` fires and this test goes RED. Run: confirmed RED.
#[test]
fn a_later_narrower_write_does_not_retract_a_wider_production() {
    let r = analyze(&[
        "module m\n\
         proc Narrowed (d1: u8) out(d0) {\n moveq #0, d0\n move.b d1, d0\n rts\n}\n\
         proc NeverWide (d1: u8) out(d0) {\n move.b d1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Narrowed", "NeverWide"]);
    assert!(verified(&r, "Narrowed", "d0"), "firings: {:?}", r.out_firings);
    assert!(out_fires(&r, "NeverWide", "d0"), "with no wide write there is nothing to keep");
}

// === the credit a caller draws from a callee ===============================

/// A callee's out credits its caller only as WIDE as the callee promised.
/// Crediting a byte-wide out as a long would be the dangerous direction: the
/// verified map is consumed as a must-def definition, so a caller could publish a
/// 32-bit claim resting on a byte.
///
/// MUTANT: credit every callee out at `L` in `credit_target_outs`. `CallerBare`
/// stops firing and this test goes RED. Run: confirmed RED.
#[test]
fn callee_out_credit_is_capped_at_the_callees_declared_width() {
    let r = analyze(&[
        "module m\n\
         proc Callee () out(d0: u8) {\n move.b #1, d0\n rts\n}\n\
         proc CallerBare () out(d0) {\n jbsr Callee\n rts\n}\n\
         proc CallerByte () out(d0: u8) {\n jbsr Callee\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Callee", "CallerBare", "CallerByte"]);
    assert!(verified(&r, "Callee", "d0"), "the callee's own claim is honest");
    assert!(out_fires(&r, "CallerBare", "d0"), "a byte of credit cannot prove 32 bits");
    assert!(verified(&r, "CallerByte", "d0"), "firings: {:?}", r.out_firings);
}

/// The same cap on a TAIL transfer. The tail arm builds its credit separately
/// from the call arm, so a width honoured at one and not the other is a live
/// shape; both go through one helper precisely so they cannot diverge.
///
/// MUTANT: restore an uncapped credit in the `Edge::TailOut` arm only.
/// `TailBare` stops firing and this test goes RED. Run: confirmed RED.
#[test]
fn tail_target_credit_is_capped_at_the_targets_declared_width() {
    let r = analyze(&[
        "module m\n\
         proc Target () out(d0: u8) {\n move.b #1, d0\n rts\n}\n\
         proc TailBare () out(d0) {\n jbra Target\n}\n\
         proc TailByte () out(d0: u8) {\n jbra Target\n}\n",
    ]);
    assert_subjects(&r, &["Target", "TailBare", "TailByte"]);
    assert!(out_fires(&r, "TailBare", "d0"), "a byte of tail credit cannot prove 32 bits");
    assert!(verified(&r, "TailByte", "d0"), "firings: {:?}", r.out_firings);
}

/// And on a declared `falls_into` successor — the third transfer-out charge.
///
/// MUTANT: restore an uncapped credit in the `Edge::FallOff` arm only. `FallBare`
/// stops firing and this test goes RED. Run: confirmed RED.
#[test]
fn falls_into_credit_is_capped_at_the_successors_declared_width() {
    let r = analyze(&[
        "module m\n\
         proc FallBare () out(d0) falls_into Successor {\n nop\n}\n\
         proc Successor () out(d0: u8) {\n move.b #1, d0\n rts\n}\n\
         proc FallByte () out(d0: u8) falls_into Successor2 {\n nop\n}\n\
         proc Successor2 () out(d0: u8) {\n move.b #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["FallBare", "Successor", "FallByte", "Successor2"]);
    assert!(out_fires(&r, "FallBare", "d0"), "a byte of successor credit cannot prove 32 bits");
    assert!(verified(&r, "FallByte", "d0"), "firings: {:?}", r.out_firings);
}

// === which types narrow ====================================================

/// A DOMAIN NEWTYPE narrows to its underlying type, transitively. This is the
/// ruling that lets `out(d0: EntryRef)` state a width at all: a typed slot has
/// room for exactly one type, so if the domain name did not carry the width, the
/// only way to declare one would be deleting the domain name.
///
/// MUTANT: answer `L` for a `Named` type that resolves to a newtype. `ViaNewtype`
/// fires and this test goes RED. Run: confirmed RED.
#[test]
fn a_newtype_narrows_to_its_underlying_type_transitively() {
    let r = analyze(&[
        "module m\n\
         newtype Inner = u16\n\
         newtype Outer = Inner\n\
         proc ViaNewtype () out(d0: Outer) {\n move.w #1, d0\n rts\n}\n\
         proc TooNarrow () out(d0: Outer) {\n move.b #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["ViaNewtype", "TooNarrow"]);
    assert!(verified(&r, "ViaNewtype", "d0"), "firings: {:?}", r.out_firings);
    assert!(
        out_fires(&r, "TooNarrow", "d0"),
        "`Outer` erases to u16, so a `.b` write must still fire — otherwise the \
         chain resolved to a byte, or to nothing at all"
    );
}

/// A niche-option newtype narrows to its PAYLOAD's width — the live
/// `EntityWindow_EntryForSection` shape (`EntryRef = EntryIndex ? -1`,
/// `EntryIndex = i16 where 0..3`), where the sentinel arrives via `moveq #-1` (a
/// long) and the payload via `move.w`. The word claim covers both; the refinement
/// narrows values, never storage.
///
/// MUTANT: answer `L` for a `Named` type that resolves to a newtype (breaking the
/// `Ref` -> `Idx` -> `i16` chain). `Lookup` fires and this test goes RED. Run:
/// confirmed RED. NOTE the mutant this test does NOT catch: a `newtype ... where
/// LO..HI` stores its range in the declaration, not as a `Type::Refined`, so the
/// refinement arm is never reached from here — it is covered on its own below.
#[test]
fn a_niche_option_narrows_to_its_payload_width() {
    let r = analyze(&[
        "module m\n\
         newtype Idx = i16 where 0..3\n\
         newtype Ref = Idx ? -1\n\
         proc Lookup (d1: u8) out(d0: Ref) {\n\
             tst.b d1\n beq .none\n move.w d1, d0\n rts\n\
         .none:\n moveq #-1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Lookup"]);
    assert!(verified(&r, "Lookup", "d0"), "firings: {:?}", r.out_firings);
}

/// `fixed<I, F>` narrows by its total bit count — `GetSineCosine`'s adopted
/// `fixed<8,8>` is a word. The `fixed<16,16>` sibling keeps the pair honest: the
/// bits are read, not the name.
///
/// MUTANT: answer `L` for every `Fixed`. `Half` fires and this test goes RED.
/// Run: confirmed RED.
#[test]
fn a_fixed_point_type_narrows_by_its_bit_count() {
    let r = analyze(&[
        "module m\n\
         proc Half () out(d0: fixed<8,8>) {\n move.w #1, d0\n rts\n}\n\
         proc Full () out(d0: fixed<16,16>) {\n move.w #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Half", "Full"]);
    assert!(verified(&r, "Half", "d0"), "firings: {:?}", r.out_firings);
    assert!(out_fires(&r, "Full", "d0"), "32 fixed-point bits are a long, not a word");
}

/// A type whose width is not derivable answers `L` — the BARE claim. That is the
/// conservative direction: an unresolvable name relaxes nothing, so a typo in a
/// type name can never quietly weaken a contract.
///
/// MUTANT: default an unresolved `Named` to `B`. `Unknown` stops firing and this
/// test goes RED. Run: confirmed RED.
#[test]
fn an_underivable_out_type_keeps_the_bare_32_bit_claim() {
    let r = analyze(&[
        "module m\n\
         proc Unknown () out(d0: NotAnyDeclaredType) {\n move.w #1, d0\n rts\n}\n\
         proc Known () out(d0: u16) {\n move.w #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Unknown", "Known"]);
    assert!(out_fires(&r, "Unknown", "d0"), "an underivable type must not relax the claim");
    assert!(verified(&r, "Known", "d0"), "firings: {:?}", r.out_firings);
}

// === composition with the conditional form =================================

/// `out(dN: T if cc)` carries BOTH facets. They answer different questions — how
/// much of the register the result occupies, and on which return edges the result
/// exists — so they compose. The `.b`-bodied sibling proves the TYPE half is
/// still charged inside the conditional form rather than being parsed and
/// dropped.
///
/// MUTANT: parse the type and discard the trailing `if` clause. The source stops
/// parsing (the `if` lands where a `,`/`/`/`)` must follow), the `assert!(diags.
/// is_empty())` in `analyze` trips, and this test goes RED. Run: confirmed RED.
#[test]
fn a_typed_out_composes_with_the_conditional_form() {
    let r = analyze(&[
        "module m\n\
         proc Cond () clobbers(d1) out(d0: u16 if eq) {\n\
             tst.b d1\n bne .miss\n move.w #1, d0\n moveq #0, d1\n rts\n\
         .miss:\n moveq #1, d1\n rts\n}\n\
         proc CondTooNarrow () clobbers(d1) out(d0: u16 if eq) {\n\
             tst.b d1\n bne .miss\n move.b #1, d0\n moveq #0, d1\n rts\n\
         .miss:\n moveq #1, d1\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Cond", "CondTooNarrow"]);
    assert!(verified_cond(&r, "Cond", "d0", "eq"), "firings: {:?}", r.out_firings);
    assert!(
        out_fires(&r, "CondTooNarrow", "d0"),
        "the cc half must not swallow the width half — a `.b` write still misses a \
         u16 claim on the success edge"
    );
}

/// The cc half is still doing its own work under a type: the `!cc` return carries
/// no production obligation at all, which is why `Cond` above verifies despite
/// `.miss` writing nothing to d0. Stated separately as a bare-vs-typed pair so a
/// regression that made a typed conditional out obligate every return is visible
/// as its own failure and not as a confusing width message.
///
/// MUTANT: drop the `cond` entry when a type is present (push only to `types`).
/// `NoWriteOnMiss` fires — its `.miss` return produces nothing and the claim
/// becomes unconditional — and this test goes RED. Run: confirmed RED.
#[test]
fn a_typed_conditional_out_is_still_unobligated_on_the_not_cc_return() {
    let r = analyze(&[
        "module m\n\
         proc NoWriteOnMiss () clobbers(d1) out(d0: u16 if eq) {\n\
             tst.b d1\n bne .miss\n move.w #1, d0\n moveq #0, d1\n rts\n\
         .miss:\n moveq #1, d1\n rts\n}\n",
    ]);
    assert_subjects(&r, &["NoWriteOnMiss"]);
    assert!(verified_cond(&r, "NoWriteOnMiss", "d0", "eq"), "firings: {:?}", r.out_firings);
}

/// An INLINE `T where LO..HI` narrows to `T`. A refinement constrains values, not
/// storage, so it must forward the inner type's width rather than answering the
/// conservative default and silently re-imposing the 32-bit claim.
///
/// This gate exists because the niche-option gate above does NOT reach this arm:
/// a `newtype X = i16 where 0..3` parks its range in the newtype declaration and
/// hands `out_width_of` a bare `i16`. Written after the refinement mutant survived
/// the niche-option gate.
///
/// MUTANT: answer `L` for `Type::Refined` instead of recursing into the inner
/// type. `Refined` fires and this test goes RED. Run: confirmed RED.
#[test]
fn an_inline_where_refinement_narrows_to_its_inner_type() {
    let r = analyze(&[
        "module m\n\
         proc Refined () out(d0: u16 where 0..3) {\n move.w #1, d0\n rts\n}\n\
         proc RefinedTooNarrow () out(d0: u16 where 0..3) {\n move.b #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Refined", "RefinedTooNarrow"]);
    assert!(verified(&r, "Refined", "d0"), "firings: {:?}", r.out_firings);
    assert!(out_fires(&r, "RefinedTooNarrow", "d0"), "the refinement must not widen to `.b`");
}

// === forms that do not cover their own operand size =========================

/// `ext` WIDENS a production; it does not make one. `ext.w d0` writes bits 8-15
/// from bits 0-7 and never touches bits 0-7, so it cannot discharge a `u8` claim
/// — the bits it does not write ARE the claim — and it discharges a `u16` one only
/// where the byte beneath was already produced. `ExtChain` is the legitimate
/// promotion the rule must keep: a real byte, widened twice, is a long.
///
/// MUTANT: credit `ext` at its operand size like any other write (drop the
/// `covers_its_size` guard and `ext_promotion`). `ExtByte`, `ExtWord` and
/// `ExtLongBare` all stop firing and this test goes RED. Run: confirmed RED.
///
/// WHAT THIS TEST ALONE DOES NOT CATCH: the mutant "`ext` promotes from ANY
/// existing production" (ignoring the one-step-down `required` threshold) leaves
/// it GREEN — every body here either has no production at all or has a correctly
/// stepped chain. That threshold is guarded by
/// [`an_ext_after_a_call_cannot_launder_a_capped_credit`], where a BYTE
/// production meets an `ext.l`. Named here rather than left to be discovered: a
/// test whose name promises more than it proves is how a reader stops looking.
#[test]
fn ext_makes_no_production_of_its_own() {
    let r = analyze(&[
        "module m\n\
         proc ExtByte () out(d0: u8) {\n ext.w d0\n rts\n}\n\
         proc ExtWord () out(d0: u16) {\n ext.w d0\n rts\n}\n\
         proc ExtLongBare () out(d0) {\n ext.l d0\n rts\n}\n\
         proc ExtChain () out(d0) {\n move.b #1, d0\n ext.w d0\n ext.l d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["ExtByte", "ExtWord", "ExtLongBare", "ExtChain"]);
    assert!(out_fires(&r, "ExtByte", "d0"), "`ext.w` never writes the byte a u8 claim covers");
    assert!(out_fires(&r, "ExtWord", "d0"), "`ext.w` over an unproduced byte widens nothing");
    assert!(out_fires(&r, "ExtLongBare", "d0"), "`ext.l` over an unproduced word widens nothing");
    assert!(verified(&r, "ExtChain", "d0"), "firings: {:?}", r.out_firings);
}

/// A correctly-capped BYTE of credit followed by `ext.l` must not become a long.
///
/// This closes the `ext` route specifically, on COVERAGE grounds: `ext.l` writes
/// only bits 16-31, so it cannot make bits 0-15 that were never produced. It does
/// NOT close laundering in general — a `.l` RMW (`add.l`, `or.l`, `not.l`,
/// `movem.l (sp)+, d0`, …) still raises a byte of credit to a long, because a
/// `.l` RMW genuinely writes all 32 bits and the module's property boundary puts
/// value PROVENANCE out of scope. That residue is ledgered as its own class; do
/// not read this gate as proving the general property.
///
/// MUTANT: credit `ext` at its operand size. `Launder` stops firing and this test
/// goes RED. Run: confirmed RED.
#[test]
fn an_ext_after_a_call_cannot_launder_a_capped_credit() {
    let r = analyze(&[
        "module m\n\
         proc Byte () out(d0: u8) {\n move.b #1, d0\n rts\n}\n\
         proc Launder () out(d0) {\n jbsr Byte\n ext.l d0\n rts\n}\n\
         proc Honest () out(d0) {\n jbsr Byte\n ext.w d0\n ext.l d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Byte", "Launder", "Honest"]);
    assert!(out_fires(&r, "Launder", "d0"), "a byte of credit and one `ext.l` is not a long");
    assert!(verified(&r, "Honest", "d0"), "firings: {:?}", r.out_firings);
}

/// A single-bit write is a real register write and a real clobber, but it
/// discharges no width claim. `Scc` is the deliberate exclusion: `seq.b` writes
/// all eight bits of its byte ($00 or $FF), so it produces a byte exactly as
/// `move.b` does, and a rule that swept the whole "sets bits" family would be
/// wrong about it.
///
/// MUTANT: add `"scc"`-family mnemonics to `writes_partial_bits` (or drop the
/// function and credit every form at its size). Either direction breaks a half of
/// this test and it goes RED. Run: confirmed RED in both directions.
#[test]
fn single_bit_writers_produce_nothing_but_scc_produces_its_byte() {
    let r = analyze(&[
        "module m\n\
         proc Tas () out(d0: u8) {\n tas.b d0\n rts\n}\n\
         proc Bset () out(d0: u8) {\n bset.b #1, d0\n rts\n}\n\
         proc Bclr () out(d0) {\n bclr.l #1, d0\n rts\n}\n\
         proc Scc () out(d0: u8) {\n seq.b d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["Tas", "Bset", "Bclr", "Scc"]);
    assert!(out_fires(&r, "Tas", "d0"), "`tas.b` sets one bit");
    assert!(out_fires(&r, "Bset", "d0"), "`bset` sets one bit");
    assert!(out_fires(&r, "Bclr", "d0"), "`bclr` clears one bit");
    assert!(verified(&r, "Scc", "d0"), "firings: {:?}", r.out_firings);
}

// === the diagnostic, which the adoption evidence is read off =================

/// The width diagnostic names the produced width and the claimed width, IN THAT
/// ORDER. This is not a cosmetic gate: the per-site "body produces" evidence
/// behind every adoption in this parcel was read off this string, so a
/// transposition would invert the evidence while every other gate stayed green.
///
/// MUTANT: swap the two `suffix()` arguments in `check_return`'s `Some(have)`
/// arm. Before this test the whole strict suite stayed GREEN at 3532/0/4; now
/// this test goes RED. Run: confirmed RED.
#[test]
fn the_width_diagnostic_names_produced_then_claimed() {
    let r = analyze(&[
        "module m\n\
         proc ByteUnderWord () out(d0: u16) {\n move.b #1, d0\n rts\n}\n\
         proc WordUnderBare () out(d0) {\n move.w #1, d0\n rts\n}\n\
         proc Nothing () out(d0) {\n nop\n rts\n}\n",
    ]);
    assert_subjects(&r, &["ByteUnderWord", "WordUnderBare", "Nothing"]);
    assert_eq!(
        out_reason(&r, "ByteUnderWord", "d0"),
        "`d0` is produced only .b wide on a required return path, and the declaration \
         claims .w",
        "the produced width comes first and the claimed width second"
    );
    assert_eq!(
        out_reason(&r, "WordUnderBare", "d0"),
        "`d0` is produced only .w wide on a required return path, and the declaration \
         claims .l"
    );
    // A register produced NOWHERE takes the other arm, which names no width at
    // all — so the pair also pins which arm each case lands in.
    assert_eq!(
        out_reason(&r, "Nothing", "d0"),
        "`d0` not produced on a required return path"
    );
}

// === externs ================================================================

/// An EXTERN's typed out caps its callers like any other. Externs seed the
/// fixpoint VERIFIED by §3 axiom — there is no body to check — so a dropped
/// extern width does not merely lose precision: it credits a caller a full long
/// on a claim nothing ever examined, and that credit reaches D1b must-def on a
/// shipping ERROR gate.
///
/// MUTANT: delete the `Item::ExternProc` arm of `collect_out_widths`. Before this
/// test the whole strict suite stayed GREEN; now `CallerBare` stops firing and
/// this test goes RED. Run: confirmed RED.
#[test]
fn an_externs_typed_out_caps_its_callers() {
    let r = analyze(&[
        "module m\n\
         extern proc E () out(d0: u8)\n\
         proc CallerBare () out(d0) {\n jbsr E\n rts\n}\n\
         proc CallerByte () out(d0: u8) {\n jbsr E\n rts\n}\n",
    ]);
    assert_subjects(&r, &["CallerBare", "CallerByte"]);
    assert!(out_fires(&r, "CallerBare", "d0"), "a byte of extern credit cannot prove 32 bits");
    assert!(verified(&r, "CallerByte", "d0"), "firings: {:?}", r.out_firings);
}

/// A typed out declared inside a `section { }` is collected. Same class as the
/// extern arm — an unwalked declaration silently reverts to the 32-bit default,
/// which RELAXES nothing for the proc itself but UNCAPS every caller's credit.
///
/// MUTANT: drop the `Item::Section` recursion from `collect_out_widths`.
/// `SectionCallerBare` stops firing and this test goes RED. Run: confirmed RED.
#[test]
fn a_typed_out_inside_a_section_is_collected() {
    let r = analyze(&[
        "module m\n\
         section rom {\n\
             proc SectionByte () out(d0: u8) {\n move.b #1, d0\n rts\n}\n\
         }\n\
         proc SectionCallerBare () out(d0) {\n jbsr SectionByte\n rts\n}\n",
    ]);
    assert_subjects(&r, &["SectionByte", "SectionCallerBare"]);
    assert!(verified(&r, "SectionByte", "d0"), "firings: {:?}", r.out_firings);
    assert!(out_fires(&r, "SectionCallerBare", "d0"), "a section-nested type must still cap");
}

/// A newtype declared inside a `section { }` resolves. Weakest of the three
/// walk-coverage gates and included for the same reason: an unwalked newtype
/// falls back to the 32-bit default, so the failure is a claim that quietly stops
/// being checkable rather than one that fires.
///
/// MUTANT: drop the `Item::Section` recursion from `collect_newtype_underlying`.
/// `SecNewtype` fires and this test goes RED. Run: confirmed RED.
#[test]
fn a_newtype_declared_inside_a_section_resolves() {
    let r = analyze(&[
        "module m\n\
         section rom {\n\
             newtype SecWord = u16\n\
         }\n\
         proc SecNewtype () out(d0: SecWord) {\n move.w #1, d0\n rts\n}\n",
    ]);
    assert_subjects(&r, &["SecNewtype"]);
    assert!(verified(&r, "SecNewtype", "d0"), "firings: {:?}", r.out_firings);
}

// === scoping and collisions =================================================

/// A NARROWER-than-long type on an address-register result is refused, on EVERY
/// declaration form that can carry an `out` clause.
///
/// Every 68k address write covers all 32 bits, so `out(a0: u8)` claims something
/// the hardware cannot produce. A POINTER or domain type whose width IS a long is
/// PERMITTED and meaningful: it carries no width news, but it is the
/// output-direction dual of the typed address PARAMS the corpus already uses
/// (`ZX0_Decompress (a0: *u8, a1: *u8) … out(a0, a1)` types those very registers
/// as inputs), and `[call.slot-type-mismatch]` reads it.
///
/// The three forms are asserted together because this rule previously lived in
/// the per-proc contract check, which `extern proc` and `type X = proc (…)` never
/// reach — and the extern is the form that does live harm, its outs seeding
/// VERIFIED by axiom with no body to re-prove them.
///
/// MUTANT: restrict `validate_out_types`'s walk to `ast::Item::Proc`. The extern
/// and contract-type cases stop firing and this test goes RED. Run: confirmed RED.
#[test]
fn a_narrow_type_on_an_address_result_is_refused_on_every_declaration_form() {
    for (label, src) in [
        ("proc", "module m\nproc P () out(a0: u8) {\n lea 4, a0\n rts\n}\n"),
        ("extern proc", "module m\nextern proc E () out(a0: u8)\n"),
        ("contract type", "module m\ntype T = proc () out(a0: u8)\n"),
    ] {
        let msgs = lower_diag_messages(src);
        assert!(
            msgs.iter().any(|m| m.contains("[proc.out-invalid]") && m.contains("address-register")),
            "{label}: expected the narrow-address-result refusal, got: {msgs:?}"
        );
    }
    // PERMITTED on all three: a pointer type is a long and states a domain, not a
    // width. Refusing it would make the output dual of an in-use facet unsayable.
    for (label, src) in [
        ("proc", "module m\nproc R () out(a0: *u8) {\n lea 4, a0\n rts\n}\n"),
        ("extern proc", "module m\nextern proc F () out(a0: *u8)\n"),
        ("contract type", "module m\ntype U = proc () out(a0: *u8)\n"),
    ] {
        let msgs = lower_diag_messages(src);
        assert!(
            !msgs.iter().any(|m| m.contains("[proc.out-invalid]")),
            "{label}: a pointer type on an address result must be permitted, got: {msgs:?}"
        );
    }
    // A DATA-register type stays silent, or the rule refuses types rather than
    // refusing impossible widths.
    let quiet =
        lower_diag_messages("module m\nproc Q () out(d0: u8) {\n move.b #1, d0\n rts\n}\n");
    assert!(
        !quiet.iter().any(|m| m.contains("[proc.out-invalid]")),
        "a data-register type must not be refused, got: {quiet:?}"
    );
}

/// An address-register out CREDITS a full long whatever its type says, so a
/// domain type there can never cap a caller below what the hardware produces.
///
/// This is the soundness half and it does NOT depend on the diagnostic above
/// firing: a per-file lint can be silenced, and the extern form reached no lint at
/// all until this round. The width pin lives where every declaration form flows
/// through one function.
///
/// MUTANT: drop the address-register pin in `collect_out_widths`'s `row` (let
/// `out_claim_of` answer for `a0`). `CallerBare` fires and this test goes RED.
/// Run: confirmed RED.
#[test]
fn an_address_out_credits_a_full_long_whatever_its_type_says() {
    let r = analyze(&[
        "module m\n\
         extern proc E () out(a0: u8)\n\
         proc CallerBare () out(a0) {\n jbsr E\n rts\n}\n",
    ]);
    assert_subjects(&r, &["CallerBare"]);
    assert!(verified(&r, "CallerBare", "a0"), "firings: {:?}", r.out_firings);
}

/// A colliding newtype name resolves in OPPOSITE directions for the claim's two
/// consumers — WIDEST for the proc's own obligation, NARROWEST for a caller's
/// credit — and this gate exercises BOTH.
///
/// One gate over one consumer is what let the first version of this rule ship
/// unsound. `max()` alone is fail-safe for the obligation (it over-fires) and
/// unsafe for the credit (it over-credits), and a test that checked only the
/// obligation could not see the difference. There is no single width safe for
/// both, so there is no single-consumer test that can guard this.
///
/// MUTANT A: resolve a collision by taking the first reading instead of merging.
/// One of the file orders flips and this test goes RED. Run: confirmed RED.
/// MUTANT B: make `OutClaim::merge` take `max` on BOTH sides — the single-width
/// rule this replaced. The caller and extern halves stop firing under collision
/// and this test goes RED. Run: confirmed RED.
#[test]
fn a_colliding_newtype_name_is_strict_for_its_owner_and_stingy_for_its_callers() {
    let wide = "module a\npub newtype Dup = u32\n";
    // OWN: the body writes a BYTE, so the widest reading must still charge it.
    let own = "module b\npub newtype Dup = u8\n\
        proc Own () out(d0: Dup) {\n move.b #1, d0\n rts\n}\n";
    // CALLER: the callee's body writes a LONG, so it verifies under either
    // reading — but a caller may draw only the NARROWEST credit its callee's
    // ambiguous contract promises, and a bare claim needs four bytes.
    let caller = "module b\npub newtype Dup = u8\n\
        proc C () out(d0: Dup) {\n move.l #1, d0\n rts\n}\n\
        proc P () out(d0) {\n jbsr C\n rts\n}\n";
    // EXTERN: the sharpest case. An extern's outs seed VERIFIED by §3 axiom, so
    // no body ever re-proves an inflated credit.
    let ext = "module b\npub newtype Dup = u8\nextern proc E () out(d0: Dup)\n\
        proc Q () out(d0) {\n jbsr E\n rts\n}\n";

    // Per subject: the verdict ALONE, and the verdict under a collision. The two
    // sides are expected to differ from each other and NOT from file order.
    //
    // OWN is charged at the WIDEST reading, so a collision legitimately turns a
    // passing claim into a firing one — that is the fail-safe working. CALLER and
    // EXTERN draw the NARROWEST credit, so a collision must never turn a firing
    // into a pass; that is the direction the single-width rule got wrong.
    for (subject, src, fires_alone, fires_collided) in [
        ("Own", own, false, true),
        ("P", caller, true, true),
        ("Q", ext, true, true),
    ] {
        for (order_label, order, want) in [
            ("alone", vec![src], fires_alone),
            ("collision first", vec![wide, src], fires_collided),
            ("collision last", vec![src, wide], fires_collided),
        ] {
            let r = analyze(&order);
            assert_subjects(&r, &[subject]);
            assert_eq!(
                out_fires(&r, subject, "d0"),
                want,
                "{subject}/{order_label}: expected fires={want}. A collision may only \
                 ever tighten, never bless, and it may never depend on file order; \
                 firings: {:?}",
                r.out_firings
            );
        }
    }

    // NON-VACUITY: with only the WIDE reading present the caller genuinely
    // verifies, so the asserts above are not "everything always fires".
    let unambiguous = "module b\npub newtype Dup = u32\n\
        proc C () out(d0: Dup) {\n move.l #1, d0\n rts\n}\n\
        proc P () out(d0) {\n jbsr C\n rts\n}\n";
    let r = analyze(&[unambiguous]);
    assert!(verified(&r, "P", "d0"), "firings: {:?}", r.out_firings);
}

/// A duplicated PROC name cannot let one file's typed out relax another file's
/// BARE one. Duplicate proc names are ill-formed and flagged elsewhere, so this is
/// a fail-safe — but without it there is exactly one construction in which writing
/// a type somewhere changes a bare declaration's verdict somewhere else, and the
/// no-migration property is what this whole feature rests on.
///
/// MUTANT: drop the untyped registers from `row` (collect only `out_types`), or
/// let a later row replace an earlier one instead of merging to the widest. The
/// bare declaration verifies and this test goes RED. Run: confirmed RED.
#[test]
fn a_duplicated_proc_name_cannot_relax_a_bare_out() {
    let typed = "module a\nproc P () out(d0: u8) {\n move.b #1, d0\n rts\n}\n";
    let bare = "module b\nproc P () out(d0) {\n move.b #1, d0\n rts\n}\n";
    for (label, order) in [("typed first", [typed, bare]), ("bare first", [bare, typed])] {
        let r = analyze(&order);
        assert_subjects(&r, &["P"]);
        assert!(
            out_fires(&r, "P", "d0"),
            "{label}: the bare claim must survive the merge; firings: {:?}",
            r.out_firings
        );
    }
}
