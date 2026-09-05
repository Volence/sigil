//! A string builtin nested inside `substr`'s POSITION or LENGTH argument.
//!
//! `substr(s, pos, len)` takes two ordinary constant expressions for `pos` and
//! `len`, and those expressions may themselves call `strstr`/`strlen`/`val`.
//! Sonic 2's jump-table generator is built entirely out of that shape:
//!
//! ```text
//! extractJmpToName function name,val(substr(name, strstr(name, "_") + 1, strlen(name)))
//! ```
//!
//! The `substr` is consumed by an OUTER builtin (`val`), so the whole call is
//! taken by the top-level builtin scanner in one bite and its inner
//! `strstr`/`strlen` never reach that scanner. They were evaluated instead by
//! `fold_const`, which runs user `function` expansion and the expression parser
//! and nothing else, so a builtin head there parsed as a bare symbol followed by
//! a paren group and the fold returned `None`. The outer builtin then reported
//! `could not evaluate string builtin`, naming the wrong call.
//!
//! The same `substr` written at TOP level always worked, and that is the trap:
//! the linear scanner walks INTO an unrecognized head's argument list, so
//! `dc.b substr(s, strstr(s,"_")+1, 3)` has its `strstr` replaced by an `Int`
//! before `substr` is ever evaluated. Whether the nesting works depended on what
//! surrounded it.
//!
//! # Provenance of every expected value here
//!
//! Reference assembler `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `asl -q -A -L -U <file>` and
//! **checked for exit status 0** before any byte was read out of the listing: a
//! run carrying an error can leave a stale value in an unrelated line's byte
//! column, so a non-zero exit disqualifies the whole listing rather than just
//! the failing line. Probes and their verbatim listings are committed under
//! `docs/superpowers/notes/2026-09-05-as-jmptos-518-block-probes/`.
//!
//! # Why these particular strings
//!
//! The haystack is `"JmpTo_Foo"` and the extracted name is `Foo`, whose address
//! is `$1000` at the point it is taken. Three properties are deliberate:
//!
//! * `strstr("JmpTo_Foo", "_")` is **5**, not 0 and not 1, so an implementation
//!   that returned "found" as a boolean, or that returned the offset of the
//!   wrong occurrence, produces a different string.
//! * The `+ 1` past that underscore matters: `substr` at 5 is `"_Foo"` and at 6
//!   is `"Foo"`. An off-by-one is a different symbol name, not a different
//!   number, so it cannot fold to a near-miss value.
//! * The addresses are `$1000` and `$100C`, which differ in more than one hex
//!   digit and are not `0`, `1` or each other, so a fold that returned the
//!   wrong one of the two symbols, or returned zero, is visible in the bytes.

use sigil_frontend_as::{assemble, Options};

/// Every fixture here sits at `org $1000`, because an address of `0` is a value
/// a broken fold can produce by accident. `flatten` renders from address zero,
/// so the leading fill is dropped and what is compared is the code itself.
const ORG: usize = 0x1000;

fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default()).unwrap_or_else(|d| {
        panic!(
            "expected a successful assembly, got {:?}",
            d.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
    });
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    let image = sigil_link::flatten(&linked, 0x00);
    assert!(
        image.len() > ORG,
        "the fixture emitted nothing at $1000: {} bytes total",
        image.len()
    );
    image[ORG..].to_vec()
}

fn refusal(src: &str) -> Vec<String> {
    assemble(src, &Options::default())
        .err()
        .unwrap_or_else(|| panic!("expected a refusal, the source assembled"))
        .into_iter()
        .map(|d| d.message)
        .collect()
}

// A TAB in these quotations is the AS mnemonic COLUMN, and the column is
// load-bearing in this crate: an indented head is an instruction, a column-0
// head is a label. Spacing them to please a prose lint would falsify the
// quoted source and the quoted asl listing, so the lint is off HERE and the
// text stays verbatim.
#[allow(clippy::tabs_in_doc_comments)]
/// `strstr` inside `substr`'s POSITION argument, with the `substr` consumed by
/// an outer `val`. This is the corpus construct with the user `function`
/// removed, so it isolates the nesting from argument substitution.
///
/// asl (md5 above, exit 0):
///
/// ```text
///        4/    1000 : 0000 1000           	dc.l	val(substr("JmpTo_Foo", strstr("JmpTo_Foo","_")+1, 3))
/// ```
#[test]
fn strstr_in_the_position_argument_of_a_nested_substr() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "Foo:\n",
        "\tdc.l\tval(substr(\"JmpTo_Foo\", strstr(\"JmpTo_Foo\",\"_\")+1, 3))\n",
    );
    assert_eq!(bytes(src), vec![0x00, 0x00, 0x10, 0x00]);
}

// The tab below is the AS mnemonic column, quoted verbatim from an asl listing.
#[allow(clippy::tabs_in_doc_comments)]
/// `strlen` inside `substr`'s LENGTH argument. `strlen("JmpTo_Foo")` is 9, which
/// runs past the end of the tail and clamps: asl's answer is the whole tail, so
/// the extracted name is still `Foo`. A length that did NOT clamp would name a
/// symbol that does not exist and the assembly would fail rather than differ.
///
/// asl (md5 above, exit 0):
///
/// ```text
///        4/    1000 : 0000 1000           	dc.l	val(substr("JmpTo_Foo", 6, strlen("JmpTo_Foo")))
/// ```
#[test]
fn strlen_in_the_length_argument_of_a_nested_substr() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "Foo:\n",
        "\tdc.l\tval(substr(\"JmpTo_Foo\", 6, strlen(\"JmpTo_Foo\")))\n",
    );
    assert_eq!(bytes(src), vec![0x00, 0x00, 0x10, 0x00]);
}

// The tab below is the AS mnemonic column, quoted verbatim from an asl listing.
#[allow(clippy::tabs_in_doc_comments)]
/// Both at once, which is the exact body of `extractJmpToName`, and reached
/// through the user `function` so that argument substitution (which
/// parenthesises every substituted argument) is in the path too. The two
/// different names resolve to two different addresses, so a fix that resolved
/// every call to the same symbol fails here and could not fail on a
/// single-symbol probe.
///
/// asl (md5 above, exit 0):
///
/// ```text
///        7/    1000 : 0000 1000           	dc.l	extractJmpToName("JmpTo_Foo")
///        8/    1004 : 0000 100C           	dc.l	extractJmpToName("JmpTo_Bar")
/// ```
#[test]
fn the_corpus_function_body_through_a_user_function_call() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "extractJmpToName function name,",
        "val(substr(name, strstr(name, \"_\") + 1, strlen(name)))\n",
        "Foo:\n",
        "\tdc.l\t0,0,0\n",
        "Bar:\n",
        "\tdc.l\textractJmpToName(\"JmpTo_Foo\")\n",
        "\tdc.l\textractJmpToName(\"JmpTo_Bar\")\n",
    );
    assert_eq!(
        bytes(src),
        vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // the padding
            0x00, 0x00, 0x10, 0x00, // Foo
            0x00, 0x00, 0x10, 0x0C, // Bar
        ]
    );
}

/// The nesting is a way to COMPUTE a name, not a way to invent one. A name that
/// no label defines is still a refusal, and the message still names the builtin
/// that could not produce a value.
///
/// This is the negative half of the pair: without it, an implementation that
/// answered `0` for anything it could not resolve would pass every test above
/// that happened to want a value it could reach.
#[test]
fn a_computed_name_that_no_label_defines_is_still_refused() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "Foo:\n",
        "\tdc.l\tval(substr(\"JmpTo_Nope\", strstr(\"JmpTo_Nope\",\"_\")+1, 4))\n",
    );
    let msgs = refusal(src);
    assert!(
        msgs.iter().any(|m| m.contains("val()")),
        "expected the refusal to name `val()`, got {msgs:?}"
    );
}

// The tab below is the AS mnemonic column, quoted verbatim from an asl listing.
#[allow(clippy::tabs_in_doc_comments)]
/// The inertness half of the fix, and the reason it can be reasoned about
/// without running the byte gates: the new expansion is the IDENTITY on any
/// operand that does not spell one of the three builtin heads immediately
/// before a `(`. A `substr` whose position and length are ordinary arithmetic
/// over a symbol takes exactly the path it took before, and this test is here
/// so that a future rewrite of `fold_const` that perturbs that path is caught.
///
/// asl (md5 above, exit 0):
///
/// ```text
///        5/    1000 : 0000 1000           	dc.l	val(substr("JmpTo_Foo", Six, 3))
/// ```
#[test]
fn a_substr_with_plain_arithmetic_arguments_is_unchanged() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "Six:\tequ\t4+2\n",
        "Foo:\n",
        "\tdc.l\tval(substr(\"JmpTo_Foo\", Six, 3))\n",
    );
    assert_eq!(bytes(src), vec![0x00, 0x00, 0x10, 0x00]);
}
