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
fn negative_sentinel_on_unsigned_payload_overlaps() {
    // `u8 ? -1` STORES $FF, which is a valid `u8` — the niche is not carved. The
    // interval test must judge the value the payload's storage holds, not the
    // unbounded integer the author typed (`-1 ∉ [0,255]` would wrongly pass).
    let msgs = lower_msgs("module m\npub newtype O = u8 ? -1\n");
    assert!(has(&msgs, "[option.niche-overlap]"), "-1 stores as $FF, a valid u8: {msgs:?}");
}

#[test]
fn negative_sentinel_on_unsigned_ranged_payload_overlaps() {
    // Same hole one layer down: the payload's `where` range is what $FF must
    // escape, and `-1` normalized is $FF, which is inside `0..$FF`.
    let msgs = lower_msgs("module m\npub newtype P = u8 where 0..$FF\npub newtype O = P ? -1\n");
    assert!(has(&msgs, "[option.niche-overlap]"), "-1 stores as $FF, inside 0..$FF: {msgs:?}");
}

#[test]
fn negative_sentinel_on_signed_payload_is_a_real_niche() {
    // The flagship idiom: on a SIGNED payload `-1` is genuinely outside `0..3`,
    // so the same spelling that is unsound on u8 is sound here. Both polarities of
    // the normalization live in one pair.
    let msgs = lower_msgs("module m\npub newtype I = i16 where 0..3\npub newtype O = I ? -1\n");
    assert!(!has(&msgs, "[option.niche-overlap]"), "-1 is outside 0..3 on i16: {msgs:?}");
}

#[test]
fn sentinel_wider_than_the_payload_storage_is_refused() {
    // `$100` does not fit a u8 at all: it would truncate to 0 — a valid payload —
    // carving no niche. Refused rather than silently accepted.
    let msgs = lower_msgs("module m\npub newtype O = u8 where 0..$7E ? $100\n");
    assert!(has(&msgs, "[option.niche-overlap]"), "$100 cannot be stored in a u8: {msgs:?}");
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

#[test]
fn assume_some_on_unknown_payload_errors() {
    // An unresolvable payload name would otherwise bless the register with a type
    // the slice engine ignores — a silent no-op marker reading as a checked one.
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n\
         proc p() {\n    assume_some! d0, Nosuch\n    rts\n}\n",
    );
    assert!(has(&msgs, "[option.assume-not-option]"), "an unknown payload must be loud: {msgs:?}");
}

#[test]
fn assume_some_on_a_non_option_payload_errors() {
    // A real newtype that no option wraps: the marker could never discharge an
    // `[option.unguarded-use]`, so it is a lie about a check that happened.
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n\
         pub newtype Lonely = u8\n\
         proc p() {\n    assume_some! d0, Lonely\n    rts\n}\n",
    );
    assert!(has(&msgs, "[option.assume-not-option]"), "a non-payload newtype must be loud: {msgs:?}");
}

#[test]
fn assume_some_on_a_pointer_payload_names_what_it_got() {
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n\
         struct S { f: u8 @ 0 }\n\
         proc p() {\n    assume_some! d0, *S\n    rts\n}\n",
    );
    assert!(has(&msgs, "[asm.assume-payload]"), "a pointer payload must be loud: {msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("got `*S`")), "the diagnostic must name what it got: {msgs:?}");
}

#[test]
fn none_on_a_non_option_newtype_errors() {
    // Keying on the decl first means a `.none` on a plain newtype is a loud member
    // error, not a fall-through to the link-symbol path (an undefined symbol at
    // link, far from the mistake).
    let msgs = lower_msgs(
        "module m\npub newtype Plain = u8\nproc p() {\n    move.b #Plain.none, d0\n    rts\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("is not a niche-option")),
        "a `.none` on a non-option must be loud: {msgs:?}"
    );
}

#[test]
fn typo_member_on_an_option_errors() {
    let msgs = lower_msgs(
        "module m\npub newtype P = u8 where 0..$7E\npub newtype Opt = P ? $FF\n\
         proc p() {\n    move.b #Opt.nome, d0\n    rts\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("has no member `nome`")),
        "a typo'd option member must be loud: {msgs:?}"
    );
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
    // Same call, but the spec §2 guard — compare against the SENTINEL and branch
    // away on equal — then `assume_some!` retypes d0 SlotRef→Slot on the surviving
    // path, so the payload slot is satisfied and nothing fires.
    //
    // The guard is `cmpi #SlotRef.none` and NOT `tst`: the payload range here is
    // `0..$7E`, so 0 is a VALID payload — a `tst/beq` would branch away on the one
    // value that was already safe and let the $FF sentinel reach the marker. This
    // fixture is the repo's "how to use assume_some!" exemplar, so it must show
    // the shape a guard-dominance check would accept, not merely one C1 trusts.
    let r = analyze(&format!(
        "{OPT_PRE}pub proc C () clobbers(d0) {{\n\
             jbsr Find\n\
             cmpi.b #SlotRef.none, d0\n\
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
fn same_shape_without_the_marker_still_fires() {
    // NON-VACUITY twin of `assume_some_extraction_clears_the_firing`: the exact
    // same guarded body with the marker REMOVED must still fire, proving the
    // clearance above comes from the marker and not from the guard's control flow
    // (which C1 does not read) or from the call site becoming invisible.
    let r = analyze(&format!(
        "{OPT_PRE}pub proc C () clobbers(d0) {{\n\
             jbsr Find\n\
             cmpi.b #SlotRef.none, d0\n\
             beq .none\n\
             jbsr Use\n\
         .none:\n\
             rts\n\
         }}\n"
    ));
    let hits: Vec<_> = r.slot_firings.iter().filter(|f| f.proc == "C").collect();
    assert_eq!(hits.len(), 1, "without the marker the option is unguarded: {:?}", r.slot_firings);
    assert_eq!(hits[0].kind, FiringKind::OptionUnguarded);
}

#[test]
fn extraction_does_not_leak_across_the_join() {
    // A marker on ONE path must not type the register after the paths REJOIN: the
    // lattice's meet degrades a register the two edges disagree about. Here the
    // extraction happens only on the guarded path, but `Use` is called AFTER the
    // join — where d0 is SlotRef on the other edge — so it must still fire.
    let r = analyze(&format!(
        "{OPT_PRE}pub proc C () clobbers(d0) {{\n\
             jbsr Find\n\
             cmpi.b #SlotRef.none, d0\n\
             beq .join\n\
             assume_some! d0, Slot\n\
         .join:\n\
             jbsr Use\n\
             rts\n\
         }}\n"
    ));
    let hits: Vec<_> = r.slot_firings.iter().filter(|f| f.proc == "C").collect();
    assert_eq!(hits.len(), 1, "the extraction must not survive the join: {:?}", r.slot_firings);
    // The meet degrades d0 to UNTYPED at the join (one edge extracted to Slot, the
    // other still holds SlotRef), so the site reports the generic mismatch rather
    // than the option id — the engine cannot prove the option is what arrives. The
    // load-bearing property is that it fires at all; the id follows the lattice.
    assert_eq!(hits[0].kind, FiringKind::SlotType, "join degrades to untyped: {hits:?}");
    assert_eq!(hits[0].found, None, "untyped at the join, not SlotRef: {hits:?}");
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
