//! An empty operand group is refused AT ITS OWN LINE.
//!
//! `split_commas` can hand `classify` a group with no tokens in it — `move.l
//! #1,` splits into `#1` and nothing, `move.l ,d0` into nothing and `d0`. A
//! group with no tokens has no span, so the refusal it raises has to borrow
//! one, and the borrowed span used to be `SourceId(0)` at offset 0: the first
//! line of the ROOT source, whatever that happened to be. Assembling
//! `s1disasm/sonic.asm` printed thirty-six `bad operand expression` rows at
//! `sonic.asm(1)` — a line reading `; ---------` — for operands written in two
//! other files.
//!
//! The line number is the whole assertion here. The message text is unchanged
//! by the fix, so a test that only checked the message would pass either way.
//!
//! These sources do NOT come from the oracle: `asl` 1.42 Beta Bld 212 accepts
//! every one of them, reading an empty operand as absolute address zero
//! (`asl -xx -n -q -A -L -U -i . sp1.asm`, exit 0, listing row
//! `5/ 4 : 21FC 0000 1234 0000   move.l #$1234,` — `move.l #$1234,($0000).w`).
//! sigil refuses them instead, which is the stricter direction and is not what
//! this test is about; what it pins is that the refusal names the line the
//! operand was written on.

use sigil_frontend_as::{assemble, Options};

/// The 1-based line of `off` within `src`, the way a `file(line)` renderer
/// derives it. Panics rather than saturating: an offset past the end means the
/// span did not come from this source, and reporting that as "the last line"
/// would hide it.
fn line_of(src: &str, off: usize) -> usize {
    assert!(
        off <= src.len(),
        "span offset {off} is past the end of a {}-byte source",
        src.len()
    );
    src[..off].bytes().filter(|b| *b == b'\n').count() + 1
}

/// Assemble, require exactly one diagnostic, and return `(line, message)`.
///
/// Requiring exactly one is deliberate. A second diagnostic would mean the
/// source is refused for some further reason, and asserting on `diags[0]` alone
/// could then pin a line that belongs to a different refusal.
fn sole_refusal(src: &str) -> (usize, String) {
    match assemble(src, &Options::default()) {
        Ok(_) => panic!("assembled clean; expected one refusal:\n{src}"),
        Err(diags) => {
            assert_eq!(
                diags.len(),
                1,
                "want exactly one diagnostic, got {}: {:?}",
                diags.len(),
                diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            );
            (
                line_of(src, diags[0].primary.start as usize),
                diags[0].message.clone(),
            )
        }
    }
}

/// A trailing comma with nothing after it. The empty group is the SECOND
/// operand, and the refusal belongs to line 5, not line 1.
#[test]
fn trailing_empty_operand_names_its_own_line() {
    let src = "\tcpu 68000\n\tpadding off\n\tnop\n\tnop\n\tmove.l\t#$1234,\n\tnop\n";
    let (line, msg) = sole_refusal(src);
    assert_eq!(msg, "bad operand expression");
    assert_eq!(line, 5, "refusal must name the line the operand is on");
}

/// A leading comma. The empty group is the FIRST operand — the one whose
/// missing span the old fallback stood in for — and it too belongs to its line.
#[test]
fn leading_empty_operand_names_its_own_line() {
    let src = "\tcpu 68000\n\tpadding off\n\tnop\n\tmove.l\t,d0\n";
    let (line, msg) = sole_refusal(src);
    assert_eq!(msg, "bad operand expression");
    assert_eq!(line, 4, "refusal must name the line the operand is on");
}

/// The corpus shape: the empty operand is produced by macro expansion, so the
/// line it belongs to is the MACRO BODY's, not the invocation's and not the
/// root's first line. `s1disasm/Macros.asm(12)` is the real instance.
#[test]
fn empty_operand_from_a_macro_body_names_the_body_line() {
    let src = concat!(
        "\tcpu 68000\n",          // 1
        "\tpadding off\n",        // 2
        "mv:\tmacro loc,port\n",  // 3
        "\tmove.l\t#loc,port\n",  // 4
        "\tendm\n",               // 5
        "\tnop\n",                // 6
        "\tmv\t$1234\n",          // 7
    );
    let (line, msg) = sole_refusal(src);
    assert_eq!(msg, "bad operand expression");
    assert_eq!(line, 4, "refusal must name the macro body line");
}

/// A non-empty operand group still reports at its own first token, not at the
/// line span. Without this the fix could have replaced every operand refusal's
/// span with the line's and still passed the three tests above.
#[test]
fn a_non_empty_group_still_reports_at_the_operand() {
    // `d0)` — a stray close paren; the group has tokens, so it owns a span, and
    // that span is the operand's, three characters into the line rather than at
    // its start.
    let src = "\tcpu 68000\n\tpadding off\n\tnop\n\tmove.l\t#1,d0)\n";
    let (line, _msg) = sole_refusal(src);
    assert_eq!(line, 4);
    let Err(diags) = assemble(src, &Options::default()) else {
        panic!("expected a refusal");
    };
    let line_start = src.find("\tmove.l").expect("the instruction line") as u32;
    assert!(
        diags[0].primary.start > line_start,
        "a group that HAS tokens must report at the operand ({}), not at the \
         line start ({line_start})",
        diags[0].primary.start
    );
}
