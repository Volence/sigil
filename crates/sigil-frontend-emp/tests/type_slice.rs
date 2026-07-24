//! G5 §7 tier 5 — the `[call.slot-type-mismatch]` domain-newtype slot check,
//! end to end over synthetic `.emp` corpora. Each test pins one lattice rule from
//! the ratified spec's acceptance list: the swap pin, untyped-into-typed,
//! `as`-blessing, out-born, copy propagation, arithmetic degrade, join-disagree
//! degrade, and the no-ceremony rule (untyped / primitive slots check nothing).

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::parse_str;

fn analyze(src: &str) -> ContractReport {
    let (f, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
    analyze_corpus(&[f])
}

/// Count `[call.slot-type-mismatch]` firings for `proc` (any callee/slot).
fn slot_count(r: &ContractReport, proc: &str) -> usize {
    r.slot_firings.iter().filter(|f| f.proc == proc).count()
}

/// A firing exists for `(proc, callee, reg)` with the given expected newtype.
fn slot_fires(r: &ContractReport, proc: &str, callee: &str, reg: &str, expected: &str) -> bool {
    r.slot_firings
        .iter()
        .any(|f| f.proc == proc && f.callee == callee && f.reg == reg && f.expected == expected)
}

/// Shared preamble: the three axis newtypes + a `GridX`-param callee (`TakeX`), a
/// two-axis callee mirroring the seam (`FlatIDXY`), a `SectionId` producer/consumer.
const PRE: &str = "module m\n\
     pub newtype GridX = u8\n\
     pub newtype GridY = u8\n\
     pub newtype SectionId = u16\n\
     pub proc TakeX (d2: GridX) clobbers() { rts }\n\
     pub proc FlatIDXY (d2: GridX, d3: GridY) clobbers(d1) out(d0: SectionId) {\n\
         moveq #0, d0\n\
         rts\n\
     }\n\
     pub proc MakeId () clobbers() out(d0: SectionId) {\n\
         moveq #0, d0\n\
         rts\n\
     }\n\
     pub proc Consume (d0: SectionId) clobbers() { rts }\n";

fn prog(caller: &str) -> String {
    format!("{PRE}{caller}")
}

#[test]
fn untyped_into_typed_fires() {
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d3) {\n\
             moveq #5, d2\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    ));
    assert!(slot_fires(&r, "C", "TakeX", "d2", "GridX"), "{:?}", r.slot_firings);
    assert_eq!(slot_count(&r, "C"), 1);
}

#[test]
fn as_bless_accepted() {
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d2 as GridX\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    ));
    assert_eq!(slot_count(&r, "C"), 0, "as-blessed GridX must satisfy the slot: {:?}", r.slot_firings);
}

#[test]
fn swap_pin_fires_both_axes() {
    // The class-closure pin: GridX/GridY swapped at the two-axis call.
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d2 as GridY\n\
             move.l d5, d3 as GridX\n\
             jbsr FlatIDXY\n\
             rts\n\
         }\n",
    ));
    assert!(slot_fires(&r, "C", "FlatIDXY", "d2", "GridX"), "d2 expects GridX: {:?}", r.slot_firings);
    assert!(slot_fires(&r, "C", "FlatIDXY", "d3", "GridY"), "d3 expects GridY: {:?}", r.slot_firings);
    assert_eq!(slot_count(&r, "C"), 2);
}

#[test]
fn correctly_typed_two_axis_passes() {
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d2 as GridX\n\
             move.l d5, d3 as GridY\n\
             jbsr FlatIDXY\n\
             rts\n\
         }\n",
    ));
    assert_eq!(slot_count(&r, "C"), 0, "{:?}", r.slot_firings);
}

#[test]
fn out_born_sectionid_accepted() {
    // d0 born as SectionId via MakeId's out, flows into Consume's SectionId slot.
    let r = analyze(&prog(
        "pub proc C () clobbers(d0) {\n\
             jbsr MakeId\n\
             jbsr Consume\n\
             rts\n\
         }\n",
    ));
    assert_eq!(slot_count(&r, "C"), 0, "out-born SectionId must satisfy the slot: {:?}", r.slot_firings);
}

#[test]
fn copy_propagates_type() {
    // A plain reg copy carries GridX from d5 into d2; the control (no copy) fires.
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d5 as GridX\n\
             move.l d5, d2\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    ));
    assert_eq!(slot_count(&r, "C"), 0, "copy must propagate GridX: {:?}", r.slot_firings);
}

#[test]
fn arithmetic_degrades_type() {
    // A blessed GridX in d2, then an arithmetic write, degrades to Untyped → fires.
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d2 as GridX\n\
             add.l d1, d2\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    ));
    assert!(slot_fires(&r, "C", "TakeX", "d2", "GridX"), "arithmetic must degrade: {:?}", r.slot_firings);
}

#[test]
fn join_disagreement_degrades() {
    // One edge blesses d2 GridX, the other leaves it untyped; at the merge the
    // meet is Untyped → the call fires.
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             tst.b d0\n\
             beq .other\n\
             move.l d4, d2 as GridX\n\
             bra .join\n\
         .other:\n\
             moveq #0, d2\n\
         .join:\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    ));
    assert!(slot_fires(&r, "C", "TakeX", "d2", "GridX"), "join-disagree must degrade: {:?}", r.slot_firings);
}

#[test]
fn join_agreement_passes() {
    // Both edges bless d2 GridX → the meet keeps GridX → no firing.
    let r = analyze(&prog(
        "pub proc C () clobbers(d0-d5) {\n\
             tst.b d0\n\
             beq .other\n\
             move.l d4, d2 as GridX\n\
             bra .join\n\
         .other:\n\
             move.l d5, d2 as GridX\n\
         .join:\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    ));
    assert_eq!(slot_count(&r, "C"), 0, "agreeing join must keep GridX: {:?}", r.slot_firings);
}

#[test]
fn primitive_and_no_param_slots_check_nothing() {
    // A primitive-typed (`u8`) callee and a no-param callee engage NO check — the
    // §7 no-ceremony rule (only domain newtypes are slots). A `proc` body param is
    // always typed, so the "untyped slot" for a body proc IS the primitive case;
    // the bare-register untyped form lives on `extern proc`/`ProcSig` (checked
    // there via the same map, absent from `typed_params`).
    let r = analyze(
        "module m\n\
         pub newtype GridX = u8\n\
         pub proc Prim (d2: u8) clobbers() { rts }\n\
         pub proc NoParam () clobbers() { rts }\n\
         pub proc C () clobbers(d2) {\n\
             moveq #5, d2\n\
             jbsr Prim\n\
             jbsr NoParam\n\
             rts\n\
         }\n",
    );
    assert_eq!(slot_count(&r, "C"), 0, "primitive/no-param slots must not fire: {:?}", r.slot_firings);
}

#[test]
fn clobber_across_call_degrades() {
    // A callee that clobbers d2 wipes a GridX held across it: the SECOND call to
    // TakeX must fire because the first (clobbering) call degraded d2.
    let r = analyze(
        "module m\n\
         pub newtype GridX = u8\n\
         pub proc TakeX (d2: GridX) clobbers() { rts }\n\
         pub proc Wipe () clobbers(d2) { rts }\n\
         pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d2 as GridX\n\
             jbsr TakeX\n\
             jbsr Wipe\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    );
    // First TakeX: d2 is GridX → ok. Wipe clobbers d2. Second TakeX: d2 untyped → fires.
    assert_eq!(slot_count(&r, "C"), 1, "exactly the post-clobber call fires: {:?}", r.slot_firings);
}

#[test]
fn preserved_across_call_keeps_type() {
    // A callee that does NOT clobber d2 preserves the GridX across it.
    let r = analyze(
        "module m\n\
         pub newtype GridX = u8\n\
         pub proc TakeX (d2: GridX) clobbers() { rts }\n\
         pub proc Keep () clobbers(d1) { rts }\n\
         pub proc C () clobbers(d0-d5) {\n\
             move.l d4, d2 as GridX\n\
             jbsr Keep\n\
             jbsr TakeX\n\
             rts\n\
         }\n",
    );
    assert_eq!(slot_count(&r, "C"), 0, "d2 preserved across Keep: {:?}", r.slot_firings);
}

// ---------------------------------------------------------------------------
// item-13 wave-1, FAMILY 1 — SongId / SfxId (the sound-id swap class). The real
// corpus enforces SfxId at Sound_PlaySFX's d0 slot (animate `.evt_sound`,
// Sound_PlayRing); there is no `.emp` Sound_PlayMusic caller, so the
// SfxId-into-SongId direction (the ratification's required negative pin) is
// pinned here synthetically. Mirrors the sound API's shape: a SongId-slot
// callee (PlayMusic) and an SfxId-slot callee (PlaySFX), both u8 but DISTINCT.
// ---------------------------------------------------------------------------
const SOUND_PRE: &str = "module m\n\
     pub newtype SongId = u8\n\
     pub newtype SfxId = u8\n\
     pub proc PlayMusic (d0: SongId) clobbers() { rts }\n\
     pub proc PlaySFX (d0: SfxId) clobbers() { rts }\n";

fn sound_prog(caller: &str) -> String {
    format!("{SOUND_PRE}{caller}")
}

#[test]
fn sfxid_into_songid_slot_fires() {
    // The ratification's required negative pin: an SfxId flowing into PlayMusic's
    // SongId slot is the wrong-sound class — it must fire naming d0/SongId.
    let r = analyze(&sound_prog(
        "pub proc C () clobbers(d0-d1) {\n\
             move.b d1, d0 as SfxId\n\
             jbsr PlayMusic\n\
             rts\n\
         }\n",
    ));
    assert!(slot_fires(&r, "C", "PlayMusic", "d0", "SongId"), "SfxId in SongId slot must fire: {:?}", r.slot_firings);
    // The found state is the WRONG newtype (SfxId), not merely untyped — a swap,
    // not a missing bless.
    let hit = r.slot_firings.iter().find(|f| f.proc == "C" && f.callee == "PlayMusic").unwrap();
    assert_eq!(hit.found.as_deref(), Some("SfxId"));
}

#[test]
fn songid_into_sfxid_slot_fires() {
    // The symmetric direction — a SongId into PlaySFX's SfxId slot.
    let r = analyze(&sound_prog(
        "pub proc C () clobbers(d0-d1) {\n\
             move.b d1, d0 as SongId\n\
             jbsr PlaySFX\n\
             rts\n\
         }\n",
    ));
    assert!(slot_fires(&r, "C", "PlaySFX", "d0", "SfxId"), "SongId in SfxId slot must fire: {:?}", r.slot_firings);
    let hit = r.slot_firings.iter().find(|f| f.proc == "C" && f.callee == "PlaySFX").unwrap();
    assert_eq!(hit.found.as_deref(), Some("SongId"));
}

#[test]
fn sfxid_bless_satisfies_playsfx() {
    // The positive: the real corpus idiom — a moveq of an sfx-id const blessed
    // `as SfxId` (Sound_PlayRing) satisfies PlaySFX's slot with zero ceremony.
    let r = analyze(&sound_prog(
        "pub proc C () clobbers(d0) {\n\
             moveq #$33, d0 as SfxId\n\
             jbra PlaySFX\n\
         }\n",
    ));
    assert_eq!(slot_count(&r, "C"), 0, "as-blessed SfxId must satisfy PlaySFX: {:?}", r.slot_firings);
}

// ---------------------------------------------------------------------------
// item-13 wave-1, FAMILY 2 — AnimId / AnimFrame / MappingFrame (the anim-frame
// swap class). In the CORPUS these values live in SST memory (accessed via
// `a0: *Sst`), so no register call-slot carries them today and the check does
// not engage — the field types are meaning-carrying, and the slice enforces the
// moment such a value crosses a typed register param. This pin proves the slice
// DOES distinguish AnimFrame from MappingFrame (the highest-risk pair) when they
// reach a slot — so the day a frame value is passed in a register, the swap
// fires. See the field-store domain-check ledger row.
// ---------------------------------------------------------------------------
const FRAME_PRE: &str = "module m\n\
     pub newtype AnimFrame = u8\n\
     pub newtype MappingFrame = u8\n\
     pub proc TakeAnimFrame (d0: AnimFrame) clobbers() { rts }\n\
     pub proc TakeMappingFrame (d0: MappingFrame) clobbers() { rts }\n";

#[test]
fn animframe_into_mappingframe_slot_fires() {
    // The ratification's required swap pin: a script CURSOR (AnimFrame) flowing
    // into a mapping-frame slot is the highest-risk mix — it must fire.
    let r = analyze(&format!(
        "{FRAME_PRE}pub proc C () clobbers(d0-d1) {{\n\
             move.b d1, d0 as AnimFrame\n\
             jbsr TakeMappingFrame\n\
             rts\n\
         }}\n"
    ));
    assert!(slot_fires(&r, "C", "TakeMappingFrame", "d0", "MappingFrame"), "AnimFrame in MappingFrame slot must fire: {:?}", r.slot_firings);
    let hit = r.slot_firings.iter().find(|f| f.proc == "C").unwrap();
    assert_eq!(hit.found.as_deref(), Some("AnimFrame"));
}

#[test]
fn mappingframe_into_animframe_slot_fires() {
    // The symmetric direction — a MappingFrame into an AnimFrame (cursor) slot.
    let r = analyze(&format!(
        "{FRAME_PRE}pub proc C () clobbers(d0-d1) {{\n\
             move.b d1, d0 as MappingFrame\n\
             jbsr TakeAnimFrame\n\
             rts\n\
         }}\n"
    ));
    assert!(slot_fires(&r, "C", "TakeAnimFrame", "d0", "AnimFrame"), "MappingFrame in AnimFrame slot must fire: {:?}", r.slot_firings);
    let hit = r.slot_firings.iter().find(|f| f.proc == "C").unwrap();
    assert_eq!(hit.found.as_deref(), Some("MappingFrame"));
}

#[test]
fn matching_frame_type_satisfies_slot() {
    // The positive control: the correctly-typed frame value satisfies its slot.
    let r = analyze(&format!(
        "{FRAME_PRE}pub proc C () clobbers(d0-d1) {{\n\
             move.b d1, d0 as MappingFrame\n\
             jbsr TakeMappingFrame\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(slot_count(&r, "C"), 0, "matching MappingFrame must satisfy the slot: {:?}", r.slot_firings);
}
