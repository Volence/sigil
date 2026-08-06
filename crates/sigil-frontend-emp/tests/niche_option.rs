//! Niche-sentinel Option (C1) — the `T ? sentinel` newtype form, `.none`
//! derivation, the three lints (`[option.niche-overlap]` error,
//! `[option.unguarded-use]` error, `[option.raw-sentinel]` warn), and the
//! trusted `assume_some!` extraction marker. Each test pins one rule from the
//! niche-option spec §1-§4; both polarities are exercised for every lint and the
//! byte-neutral claims are proven by comparing linked bytes.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::type_slice::FiringKind;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Lower `src` and return every diagnostic MESSAGE (all levels).
fn lower_msgs(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let (_m, ldiags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    ldiags.iter().map(|d| d.message.clone()).collect()
}

/// Lower `src`, resolve+link the `text` section, return its final bytes. Asserts
/// no lowering ERROR (a byte-comparison test wants a clean build).
fn text_bytes(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let (module, ldiags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower: {ldiags:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section("text").expect("text section").bytes.clone()
}

fn analyze(src: &str) -> ContractReport {
    let (f, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != Level::Error), "parse: {diags:?}");
    analyze_corpus(&[f])
}

fn has(msgs: &[String], id: &str) -> bool {
    msgs.iter().any(|m| m.contains(id))
}

// ---- §1 niche-overlap (error), both polarities + non-vacuity --------------

#[test]
fn niche_overlap_unranged_fires() {
    // No `where` range ⇒ full-range payload ⇒ the sentinel is a valid payload
    // value ⇒ immediate overlap.
    let msgs = lower_msgs("module m\npub newtype Opt = u8 ? $FF\n");
    assert!(has(&msgs, "[option.niche-overlap]"), "unranged option must overlap: {msgs:?}");
}

#[test]
fn niche_overlap_ranged_fires() {
    // A sentinel INSIDE the payload's `where` range overlaps.
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $40\n",
    );
    assert!(has(&msgs, "[option.niche-overlap]"), "$40 is inside 0..$7E: {msgs:?}");
}

#[test]
fn niche_valid_does_not_fire() {
    // Sentinel OUTSIDE the payload range — a sound niche. Non-vacuity: the build
    // is otherwise clean, so a missing overlap is a real accept, not a swallowed
    // parse error.
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n",
    );
    assert!(!has(&msgs, "[option.niche-overlap]"), "$FF is outside 0..$7E: {msgs:?}");
    assert!(msgs.iter().all(|m| !m.contains("niche")), "no niche complaint at all: {msgs:?}");
}

// ---- §1 `.none` derivation + wrap direction: byte-identity ----------------

#[test]
fn none_operand_is_byte_identical_to_raw_sentinel() {
    let base = "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n";
    let with_none = format!("{base}proc p() {{\n    move.b #Opt.none, d0\n    rts\n}}\n");
    let with_raw = format!("{base}proc p() {{\n    move.b #$FF, d0\n    rts\n}}\n");
    assert_eq!(text_bytes(&with_none), text_bytes(&with_raw), "`.none` must erase to the sentinel byte");
}

#[test]
fn wrap_direction_constructs_the_payload() {
    // `Opt(x)` wraps a valid payload (the always-safe direction) and emits the
    // wrapped value's byte.
    let base = "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n";
    let wrapped = format!("{base}proc p() {{\n    move.b #Opt(3), d0\n    rts\n}}\n");
    let plain = format!("{base}proc p() {{\n    move.b #3, d0\n    rts\n}}\n");
    assert_eq!(text_bytes(&wrapped), text_bytes(&plain), "Opt(3) emits the payload byte 3");
}

// ---- §2 `assume_some!` — parse, zero bytes, and the retype ----------------

#[test]
fn assume_some_emits_zero_bytes() {
    let base = "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n";
    let without = format!("{base}proc p() {{\n    moveq #0, d0\n    rts\n}}\n");
    let with = format!("{base}proc p() {{\n    moveq #0, d0\n    assume_some! d0, P\n    rts\n}}\n");
    assert_eq!(text_bytes(&with), text_bytes(&without), "the extraction marker must emit nothing");
}

#[test]
fn assume_some_on_non_register_errors() {
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n\
         proc p() {\n    assume_some! notareg, P\n    rts\n}\n",
    );
    assert!(has(&msgs, "[asm.assume-not-register]"), "a non-register target must be loud: {msgs:?}");
}

// ---- §1/§2 unguarded-use (error) via the type-slice engine ----------------

const OPT_PRE: &str = "module m\n\
     pub newtype Slot = u8 where 0..$7E\n\
     pub newtype SlotRef = Slot ? $FF\n\
     pub proc Find () clobbers() out(d0: SlotRef) { moveq #0, d0\n rts }\n\
     pub proc Use (d0: Slot) clobbers() { rts }\n";

#[test]
fn unguarded_use_fires_and_is_the_only_firing() {
    // A SlotRef (the option) flowing into a Slot (the payload) slot with no
    // extraction is `[option.unguarded-use]` — and exactly ONE firing (one engine,
    // one id per site, never also a plain slot-type mismatch).
    let r = analyze(&format!(
        "{OPT_PRE}pub proc C () clobbers(d0) {{\n    jbsr Find\n    jbsr Use\n    rts\n}}\n"
    ));
    let hits: Vec<_> = r.slot_firings.iter().filter(|f| f.proc == "C").collect();
    assert_eq!(hits.len(), 1, "exactly one firing at the site: {:?}", r.slot_firings);
    assert_eq!(hits[0].kind, FiringKind::OptionUnguarded, "must be the option id, not the generic: {hits:?}");
    assert_eq!(hits[0].expected, "Slot");
    assert_eq!(hits[0].found.as_deref(), Some("SlotRef"));
}

#[test]
fn assume_some_extraction_clears_the_firing() {
    // Same call, but the guarded `assume_some!` retypes d0 SlotRef→Slot on the
    // path — the payload slot is now satisfied, no firing.
    let r = analyze(&format!(
        "{OPT_PRE}pub proc C () clobbers(d0) {{\n\
             jbsr Find\n\
             tst.b d0\n\
             beq .none\n\
             assume_some! d0, Slot\n\
             jbsr Use\n\
         .none:\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        r.slot_firings.iter().filter(|f| f.proc == "C").count(),
        0,
        "assume_some! must satisfy the payload slot: {:?}",
        r.slot_firings
    );
}

#[test]
fn plain_slot_mismatch_keeps_its_own_id() {
    // A WRONG newtype (not the option-of-payload) still fires the generic id — the
    // option lint replaces it only at the niche-option-into-payload site.
    let r = analyze(&format!(
        "{OPT_PRE}pub newtype Other = u8\n\
         pub proc C () clobbers(d0) {{\n    moveq #1, d0\n    move.b d0, d0 as Other\n    jbsr Use\n    rts\n}}\n"
    ));
    let hits: Vec<_> = r.slot_firings.iter().filter(|f| f.proc == "C").collect();
    assert_eq!(hits.len(), 1, "one firing: {:?}", r.slot_firings);
    assert_eq!(hits[0].kind, FiringKind::SlotType, "a non-option wrong type is the generic id: {hits:?}");
}

// ---- §1 raw-sentinel (warn) at an option-typed field, both polarities -----

const FIELD_PRE: &str = "module m\n\
     pub newtype Tag = u8 where 0..$7E\n\
     pub newtype TagRef = Tag ? $FF\n\
     struct S { f: TagRef @ 0 }\n";

#[test]
fn raw_sentinel_into_option_field_warns() {
    let msgs = lower_msgs(&format!(
        "{FIELD_PRE}proc p() {{\n    move.b #$FF, S.f(a0)\n    rts\n}}\n"
    ));
    assert!(has(&msgs, "[option.raw-sentinel]"), "a raw sentinel into an option field must warn: {msgs:?}");
}

#[test]
fn none_into_option_field_does_not_warn() {
    let msgs = lower_msgs(&format!(
        "{FIELD_PRE}proc p() {{\n    move.b #TagRef.none, S.f(a0)\n    rts\n}}\n"
    ));
    assert!(!has(&msgs, "[option.raw-sentinel]"), "the `.none` spelling is sanctioned: {msgs:?}");
}

#[test]
fn non_sentinel_immediate_into_option_field_does_not_warn() {
    // Opposite polarity: an immediate that is NOT the sentinel is an ordinary tag
    // store — no nudge.
    let msgs = lower_msgs(&format!(
        "{FIELD_PRE}proc p() {{\n    move.b #$01, S.f(a0)\n    rts\n}}\n"
    ));
    assert!(!has(&msgs, "[option.raw-sentinel]"), "a non-sentinel value is fine: {msgs:?}");
}

#[test]
fn inline_where_then_sentinel_parses_and_resolves() {
    // `newtype X = u8 where LO..HI ? SENT` — the `where` refinement AND the `?`
    // niche clause on ONE decl. The field must resolve (no "unknown type").
    let msgs = lower_msgs(
        "module m\npub newtype T = u8 where 0..$FE ? $FF\nstruct S { f: T @ 0 }\nensure(offsetof(S, f) == 0, \"ok\")\n",
    );
    assert!(!msgs.iter().any(|m| m.contains("unknown type")), "inline where?sentinel must resolve: {msgs:?}");
    assert!(!msgs.iter().any(|m| m.contains("[option.niche-overlap]")), "$FF outside 0..$FE: {msgs:?}");
}
