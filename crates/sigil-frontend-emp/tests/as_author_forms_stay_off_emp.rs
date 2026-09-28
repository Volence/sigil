//! The AS front-end's author-form rewrites (`crates/sigil-frontend-as/tests/as_author_forms.rs`)
//! are AS-route only: what `.emp` source may spell is an owner-reviewed surface,
//! and it does not move with them. Each case below pins `.emp`'s answer as it was
//! BEFORE those rewrites, measured by running this file's `lowered_errors` over
//! the same bodies at sigil master `3ceed19b`, and requires the same answer now.
//!
//! The expected strings are sigil's own diagnostics at that baseline, not asl's
//! bytes: the claim here is "unchanged", so the baseline is the oracle.
//!
//! The two cases the parcel brief names are the alias (`add.w d0,a1` keeps its
//! loud "write `adda`" refusal, the fix for the silent ADDX mis-encode) and
//! `swap.w` (which `.emp` accepts, with no diagnostic, while the AS route now
//! refuses it as asl does).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Lower a one-instruction proc and return its error-level diagnostics.
fn lowered_errors(body: &str) -> Vec<String> {
    let emp = format!("module m\npub proc P () {{\n        {body}\n        rts\n}}\n");
    let (file, pdiags) = parse_str(&emp);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "parse errors for `{body}`: {:?}",
        pdiags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_module, ldiags) = lower_module(&file, &opts);
    ldiags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

fn assert_unchanged(body: &str, baseline: &[&str]) {
    assert_eq!(lowered_errors(body), baseline, "`.emp` answer for `{body}` moved");
}

#[test]
fn add_and_sub_to_an_address_register_keep_their_loud_refusal() {
    assert_unchanged(
        "add.w d0,a1",
        &["unsupported form: Add.W with an address-register destination is not encodable, \
           write `adda` (address arithmetic; word/long only, no `.b`)"],
    );
    assert_unchanged(
        "sub.w d0,a1",
        &["unsupported form: Sub.W with an address-register destination is not encodable, \
           write `suba` (address arithmetic; word/long only, no `.b`)"],
    );
    assert_unchanged("eor.w #1,d0", &["unsupported form: eor requires Dn source, got Imm(1)"]);
}

#[test]
fn swap_with_a_suffix_is_what_it_was() {
    assert_unchanged("swap.w d0", &[]);
    assert_unchanged("swap.b d0", &[]);
    assert_unchanged("swap d0", &[]);
}

#[test]
fn the_other_as_rewrites_do_not_reach_emp() {
    assert_unchanged("beq.b P", &["branch size suffix must be `.s` or `.w`"]);
    assert_unchanged("andi #$FE,ccr", &["instruction needs an explicit size suffix (.b/.w/.l)"]);
    assert_unchanged("andi.w #$F8FF,sr", &["unsupported form: sr is not a general EA"]);
    assert_unchanged("eori.b #1,ccr", &["unsupported form: ccr is not a general EA"]);
    for m in ["unlk a6", "rtr", "chk.w d0,d1", "reset", "stop #$2700", "trapv"] {
        let word = m.split(['.', ' ']).next().unwrap();
        assert_unchanged(m, &[&format!("`{word}` is not a recognized 68000 mnemonic")]);
    }
}

/// AS-SYMBOL-SIZE-SUFFIX's AS-route reading (`jmp Foo.w` is `Foo` at word
/// width, `.w`/`.b` on a displacement, a branch target read whole) does not
/// reach `.emp`, whose front end never calls the AS operand classifier. Pinned
/// against sigil master `4915361c`, the branch point of that parcel, measured
/// with this file's `lowered_errors`.
#[test]
fn a_width_suffix_on_a_name_is_what_it_was() {
    for m in [
        "jmp P.w",
        "jmp P.W",
        "jsr P.l",
        "lea P.w,a0",
        "move.w P.w,d0",
        "move.l d0,P.l",
        "jmp (P).w",
        "bra.s P.w",
        "dbf d0,P.w",
    ] {
        assert_unchanged(m, &[]);
    }
    assert_unchanged(
        "move.w (P.w),d0",
        &[
            "unknown name `P`",
            "instruction dropped, a mnemonic, size, or operand did not resolve \
             (a missing import or type in scope?)",
        ],
    );
    assert_unchanged("jmp $1234.w", &["illegal destination EA: #imm"]);
}
