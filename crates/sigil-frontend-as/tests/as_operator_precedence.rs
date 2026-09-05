//! AS operator precedence: the reference assembler's ladder, measured.
//!
//! Sigil placed `&&` and `||` LOOSER than the comparison operators, which is C's
//! convention and is not asl's. asl binds both logical operators TIGHTER than
//! every comparison, so `A=6&&C<>3` parses as `(A = (6 && C)) <> 3` and folds to
//! `1`, where sigil folded `(A=6) && (C<>3)` to `0`. Both assemblers exit 0 and
//! neither says a word: a silent wrong answer in an integer a `dc.b` emits.
//!
//! Measuring the pair in that row turned up four more tiers in the wrong place.
//! The full asl ladder, tightest first, is
//!
//!   `<<` `>>` / `&` / `|` / `!` / `*` `/` `#` / `+` `-` / `&&` / `||` /
//!   `=` `<>` `<` `>` `<=` `>=`
//!
//! so asl's bitwise and shift operators bind TIGHTER than multiplication, `!`
//! (bitwise xor) is its own tier looser than `|` rather than sharing `|`'s, and
//! the comparisons are the loosest tier rather than a middle one. Sigil had the
//! shifts below `+`, `!` sharing a tier with `|`, and the comparisons above the
//! logical operators: `1+1<<3` folded to 16 where asl folds 9, and `3!1|2`
//! folded to 2 where asl folds 0.
//!
//! Expectations derived from asl 1.42 Beta [Bld 212],
//! `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, from one `asl -L` listing that exited 0
//! with `0 errors` and `0 warnings`. THE DIGEST IS CITED BECAUSE THE BANNER
//! CANNOT IDENTIFY THE BINARY: `s2disasm`'s build prints the same banner and
//! substitutes a stale value for any operand it declines, so a listing from it
//! is not a source of expectations. The probes and the listing they came from
//! are committed under
//! `docs/superpowers/notes/2026-09-05-as-logical-precedence-probes/`.
//!
//! WHY THESE OPERANDS. Every row below was generated beside BOTH of its
//! candidate parenthesisations, `(a op1 b) op2 c` and `a op1 (b op2 c)`, in the
//! same listing, and a row is kept only where those two candidates emitted
//! DIFFERENT bytes. That is the property that lets a row refute a wrong ladder
//! instead of agreeing with two of them: `0`, `1` and symmetric operands make
//! the two parses coincide constantly. `A>4&&B<5` is the standing example and is
//! deliberately kept, because it folds to `1` under EITHER ladder: it is a
//! control, and reading it as evidence is the mistake this paragraph exists to
//! prevent. The probes rejected for that reason are named in the note.
//!
//! MUST FAIL on the C-style ladder sigil shipped, on any ladder that leaves the
//! shifts below `+`, on any that gives `!` the same tier as `|`, and on any that
//! puts the comparisons above `&&` or `||`.

use sigil_frontend_as::{assemble, Options};

/// The constants the last group of rows names, plus the 68000 selection every
/// probe assembled under.
const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n\
                    A\tequ 6\nB\tequ 2\nC\tequ 3\nK\tequ 3\nJ\tequ 7\n";

/// Assemble one `dc.b <expr>` and hand back the single byte it emitted.
///
/// A REFUSAL comes back as `Err` rather than a panic, because a wrong ladder can
/// make sigil decline a program asl assembles rather than merely fold it
/// differently, and that outcome belongs in the same report as a wrong byte:
/// `int(1.0+1<<3)` is exactly such a row. Panicking on it would have hidden the
/// other 88.
fn folded(expr: &str) -> Result<u8, String> {
    let src = format!("{HEAD}\tdc.b\t{expr}\n");
    let module = assemble(&src, &Options::default()).map_err(|f| {
        format!(
            "refused: {:?}",
            f.iter().map(|d| &d.message).collect::<Vec<_>>()
        )
    })?;
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new())
        .map_err(|_| "link failed".to_string())?;
    let bytes = sigil_link::flatten(&linked, 0x00);
    if bytes.len() != 1 {
        return Err(format!("emitted {} bytes, expected 1", bytes.len()));
    }
    Ok(bytes[0])
}

/// Every row of the golden listing: the expression, and the byte asl emitted.
const GOLDEN: &[(&str, u8)] = &[
    ("6&&3",                  0x01),
    ("4&&2",                  0x01),
    ("4||2",                  0x01),
    ("0&&5",                  0x00),
    ("0||0",                  0x00),
    ("2=2",                   0x01),
    ("1&&12&3",               0x00),
    ("6&4&&2",                0x01),
    ("0&&8|4",                0x00),
    ("4|0&&8",                0x01),
    ("0&&8!4",                0x00),
    ("4!0&&8",                0x01),
    ("1&&1<<3",               0x01),
    ("1<<3&&0",               0x00),
    ("1&&2+3",                0x01),
    ("3+0&&0",                0x00),
    ("2&&3*4",                0x01),
    ("1&&2=2",                0x00),
    ("2=2&&1",                0x00),
    ("7<>3&&0",               0x01),
    ("5>1&&0",                0x01),
    ("0<=1&&0",               0x01),
    ("0>=1&&0",               0x01),
    ("1||0&&0",               0x01),
    ("0&&0||1",               0x01),
    ("0||12&3",               0x00),
    ("6&4||0",                0x01),
    ("0||8|4",                0x01),
    ("8|0||4",                0x01),
    ("0||8!4",                0x01),
    ("1||1<<3",               0x01),
    ("1||2+3",                0x01),
    ("1||2=2",                0x00),
    ("2=2||0",                0x00),
    ("1|2&2",                 0x03),
    ("1&6|3",                 0x03),
    ("1!3&2",                 0x03),
    ("3!1|2",                 0x00),
    ("3|1!3",                 0x00),
    ("1&3<<1",                0x00),
    ("1&3>>1",                0x01),
    ("1|3<<1",                0x07),
    ("1!3<<1",                0x07),
    ("1&3+1",                 0x02),
    ("1+3&2",                 0x03),
    ("3|2+2",                 0x05),
    ("1+3|4",                 0x08),
    ("3!2+2",                 0x03),
    ("1+3!2",                 0x02),
    ("3&2*3",                 0x06),
    ("3*2&5",                 0x00),
    ("3*2|5",                 0x15),
    ("3*2!5",                 0x15),
    ("3!2*2",                 0x02),
    ("1+1<<3",                0x09),
    ("1+8>>2",                0x03),
    ("8-1<<2",                0x04),
    ("2<<1*3",                0x0C),
    ("12/2<<1",               0x03),
    ("12<<1/3",               0x08),
    ("12#5<<1",               0x02),
    ("8>>1<<2",               0x10),
    ("7#5*2",                 0x04),
    ("12/2*3",                0x12),
    ("12*2/3",                0x08),
    ("12#5/2",                0x01),
    ("6+2*3",                 0x0C),
    ("1<2=1",                 0x01),
    ("2=1<2",                 0x01),
    ("4=1+1",                 0x00),
    ("6&2=2",                 0x01),
    ("6<<1=12",               0x01),
    ("3!1=2",                 0x01),
    ("1=3!2",                 0x01),
    ("A=6&&C<>3",             0x01),
    ("A=6||C=3",              0x00),
    ("A&B=2",                 0x01),
    ("A|B=2",                 0x00),
    ("A>4&&B<5",              0x01),
    ("A<<1=12",               0x01),
    ("A+B*C",                 0x0C),
    ("(K*2)=6&&(J<>3)",       0x00),
    ("int(1&&2=2)",           0x00),
    ("int(1+1<<3)",           0x09),
    ("int(3!1|2)",            0x00),
    ("int(2=2&&1)",           0x00),
    ("int(1||2=2)",           0x00),
    ("int(1.0+1<<3)",         0x09),
    ("int(6.0/2*3)",          0x09),
];

/// Every golden row at once, reported as a set rather than on the first
/// mismatch: a ladder is a total order, so one misplaced tier moves many rows
/// and the FIRST failure is the least informative thing about it.
#[test]
fn the_asl_operator_ladder_is_reproduced_exactly() {
    let mut wrong = Vec::new();
    for (expr, want) in GOLDEN {
        match folded(expr) {
            Ok(got) if got == *want => {}
            Ok(got) => wrong.push(format!("  {expr:<20} asl={want:#04X} sigil={got:#04X}")),
            Err(why) => wrong.push(format!("  {expr:<20} asl={want:#04X} sigil {why}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} expressions fold differently from asl:\n{}",
        wrong.len(),
        GOLDEN.len(),
        wrong.join("\n")
    );
}
