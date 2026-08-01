//! Cross-module `offsets` targets (§4.7's `Ref` by-reference form, L9): an
//! `offsets` table whose entries point at labels/procs defined in OTHER modules
//! (bare identifiers, the L11 cross-module link-ref idiom). This is the shape
//! the three player-state jump tables (Player_States / PState_EnterHooks /
//! PState_ExitHooks) adopt — their targets are `pub proc`s in the sibling
//! player modules.
//!
//! Key property proven here: the `offsets` `Ref` form takes the target's symbol
//! NAME by shape (it does not resolve the name locally), so a bare cross-module
//! identifier lowers to the exact same `RelOffset { base, target }` cell — and
//! thus the exact same self-relative `dc.w target - base` word — that the hand
//! `extern(target) - extern(base)` data form produces. Byte-identity therefore
//! holds regardless of WHERE the target symbol is ultimately defined; a genuine
//! typo is caught at link (an unresolved-symbol error naming the target), which
//! is unavoidable — at lower time a cross-module ref is structurally
//! indistinguishable from a typo (both are bare names absent from this module).

use sigil_frontend_emp::ast;
use sigil_frontend_emp::layout::eval_offsets_with_root;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Cell;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

/// Try to link, returning the error messages (or panicking on unexpected success).
fn link_errs(m: &Module) -> Vec<String> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    match sigil_link::link(&resolved, &SymbolTable::new()) {
        Ok(_) => Vec::new(),
        Err(ds) => ds.into_iter().map(|d| d.message).collect(),
    }
}

fn offsets_decl<'a>(file: &'a ast::File, name: &str) -> &'a ast::OffsetsDecl {
    file.items
        .iter()
        .find_map(|it| match it {
            ast::Item::Offsets(o) if o.name == name => Some(o),
            _ => None,
        })
        .expect("offsets decl")
}

// ---- 1. the equivalence proof: offsets Ref == the extern-difference form ------

#[test]
fn cross_module_ref_lowers_like_the_extern_difference_form() {
    // The `offsets` Ref form with a bare target and the hand
    // `extern(target) - extern(base)` data form emit the IDENTICAL bytes. The
    // target is defined in-file here so the single-module link resolves; the
    // RelOffset cell is identical regardless of where the target lives, which is
    // exactly why the player-table flip is byte-identical.
    let offsets_src = "\
module m
pub offsets Tbl { A: Target }
data Target: [u8; 1] = [$AA]
";
    let hand_src = "\
module m
pub data Tbl: [i16; 1] = [ extern(\"Target\") - extern(\"Tbl\") ]
data Target: [u8; 1] = [$AA]
";
    let (mo, mo_msgs) = lower(offsets_src);
    let (mh, mh_msgs) = lower(hand_src);
    assert!(mo_msgs.is_empty(), "offsets form clean lower: {mo_msgs:?}");
    assert!(mh_msgs.is_empty(), "hand form clean lower: {mh_msgs:?}");
    assert_eq!(
        linked_bytes(&mo),
        linked_bytes(&mh),
        "offsets Ref form and extern-difference form must emit identical bytes"
    );
    // Concretely: Tbl (word) at 0, Target at 2 → offset word = 2, then $AA.
    assert_eq!(linked_bytes(&mo), vec![0x00, 0x02, 0xAA]);
}

// ---- 2. a cross-module target needs NO local definition -----------------------

#[test]
fn cross_module_ref_target_needs_no_local_definition() {
    // The Ref path takes the target name by shape; a bare identifier absent from
    // this module (the real cross-module case — the target is a `pub proc`
    // elsewhere) lowers CLEANLY to one `RelOffset { base, target }` per member.
    // No local-resolution diagnostic fires (that would false-flag every legit
    // cross-module ref).
    let src = "module m\npub offsets Tbl { A: PState_Ground, B: PState_Air }\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "clean parse: {pdiags:?}");
    let (buf, _bodies, diags) =
        eval_offsets_with_root(&file, offsets_decl(&file, "Tbl"), None, &[]);
    assert!(diags.is_empty(), "a cross-module Ref must not be flagged at lower: {diags:?}");
    let buf = buf.expect("buf");
    let rel: Vec<(&str, &str)> = buf
        .cells
        .iter()
        .filter_map(|c| match c {
            Cell::RelOffset { base, target } => Some((base.as_str(), target.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        rel,
        vec![("Tbl", "PState_Ground"), ("Tbl", "PState_Air")],
        "each member is a self-relative word `target - Tbl` naming the raw cross-module symbol"
    );
}

// ---- 3. a repeated cross-module target (the PHook_AirBallEnter x3 pattern) -----

#[test]
fn repeated_cross_module_target_emits_a_word_per_member() {
    // PState_EnterHooks routes JUMP/ROLLJUMP/AIRBALL all through
    // PHook_AirBallEnter; a repeated target is fine — each member is its own
    // self-relative word to the same handler.
    let src = "module m\npub offsets Tbl { A: H, B: H, C: H }\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "clean parse: {pdiags:?}");
    let (buf, _bodies, diags) =
        eval_offsets_with_root(&file, offsets_decl(&file, "Tbl"), None, &[]);
    assert!(diags.is_empty(), "repeated cross-module targets are fine: {diags:?}");
    let n = buf.expect("buf").cells.iter().filter(|c| matches!(c, Cell::RelOffset { target, .. } if target == "H")).count();
    assert_eq!(n, 3, "three members, three self-relative words to the same handler");
}

// ---- 4. a genuinely-undefined target fails loudly at link ---------------------

#[test]
fn undefined_cross_module_target_fails_loudly_at_link() {
    // At lower time a typo is indistinguishable from a valid cross-module ref, so
    // it lowers clean; the linker catches a genuinely-undefined target and names
    // the offending symbol (§4.7: "a genuinely-undefined target fails loudly at
    // link").
    let src = "module m\npub offsets Tbl { A: PState_Typpo }\n";
    let (m, lower_msgs) = lower(src);
    assert!(lower_msgs.is_empty(), "no false lower-time flag for a bare ref: {lower_msgs:?}");
    let errs = link_errs(&m);
    assert!(
        errs.iter().any(|e| e.contains("PState_Typpo")),
        "the link error names the unresolved target: {errs:?}"
    );
}

// ---- 5. the `.count` guard survives (replaces the hand ensure) ----------------

#[test]
fn count_ordinal_available_for_the_sync_guard() {
    // The flip replaces `ensure(7 == PSTATE_COUNT)` with
    // `ensure(Tbl.count == PSTATE_COUNT)`; `.count` is available on a Ref-only
    // (cross-module) table exactly as on any offsets table.
    let src = "\
module m
pub offsets Tbl { A: X, B: Y, C: Z }
const N = Tbl.count
data D: [u8; 1] = [N]
data X: [u8; 1] = [1]
data Y: [u8; 1] = [2]
data Z: [u8; 1] = [3]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let bytes = linked_bytes(&m);
    assert_eq!(*bytes.last().unwrap(), 3, "Tbl.count == 3 on a cross-module Ref table");
}
