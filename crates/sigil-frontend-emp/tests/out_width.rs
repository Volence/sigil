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
use sigil_frontend_emp::parse_str;

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

/// Does `[proc.out-unverified]` fire on `(proc, reg)`?
fn out_fires(r: &ContractReport, proc: &str, reg: &str) -> bool {
    r.out_firings.iter().any(|f| f.proc == proc && f.reg == reg)
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
