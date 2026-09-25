//! cli_define: the grammar of one asl `-D` argument, for [`Options::cli_defines`].
//!
//! asl reads `-D <list>` as a comma list of `NAME` or `NAME=VALUE` parts, and
//! defines each name as a SET variable before the first line of every pass.
//! What this module accepts and refuses is measured against the reference asl
//! (md5 `61e672562465725a8c102288a7da9098`); the probe table is in
//! `docs/superpowers/notes/2026-09-25-as-cli-define.md`:
//!
//! * a part with no `=`, or with nothing after it, defines the name as `1`;
//! * a comma list defines several names; one trailing comma is accepted, an
//!   empty part anywhere else is refused, and a space after a comma is part of
//!   the next name and so refuses it;
//! * a name starts with a letter or `_` and continues with letters, digits,
//!   `_` or `.`; anything else is refused;
//! * a value is an integer expression with no symbols in it, in the front
//!   end's own expression grammar, so `$10`, `%101`, `-1`, `~1`, `1+2`,
//!   `1<<4` and `(3)` fold exactly as they would in a source line. Whitespace
//!   around and inside the value is ignored, as asl ignores it;
//! * when a name is given twice, in one list or across repeated flags, the
//!   FIRST value stands, silently (asl: `-D FOO=1 -D FOO=2` reads `1`).
//!
//! Refused although asl accepts, each by its own message:
//!
//! * a quoted value, `"..."` or `'...'`: asl accepts both and the bytes it
//!   then emits for the symbol differ from run to run (a string `"A"` read
//!   back as `25` and then `64`, a character `'A'` as `C6`, `41`, `21`, `82`),
//!   so there is no answer to match;
//! * a name starting with `.`: asl defines it and then no source line can read
//!   it (`-D .FOO=1` with `dc.b .FOO` is `#1010 symbol undefined`), while
//!   sigil's `.FOO` at file level would read it;
//! * a floating-point value such as `1.5`: asl defines a float variable, which
//!   has no representation in [`Options::cli_defines`];
//! * the integer spellings asl's command line takes that sigil's source lexer
//!   does not (`0b101`, `@17`), and a hex literal past `i64` (`$FFFFFFFFFFFFFFFF`,
//!   `$10000000000000000`), which asl wraps or truncates. The lexer's own forms
//!   all work, `0FFh` included, and so does `0x`/`0X` hex, which the rest of
//!   sigil's `-D` flags already take.
//!
//! [`Options::cli_defines`]: crate::Options::cli_defines

use crate::charset::CodePage;
use crate::expr::{parse_expr, ExprCtx};
use crate::lexer::lex_line;
use crate::nameless::NamelessCounts;
use sigil_ir::backend::Cpu;
use sigil_ir::expr::Fold;
use sigil_span::SourceId;

/// Parse one `-D` argument into its `(name, value)` pairs, in the order
/// written. Duplicates are kept; [`first_wins`] applies asl's rule for them.
pub fn parse_define_arg(arg: &str) -> Result<Vec<(String, i64)>, String> {
    let body = arg.strip_suffix(',').unwrap_or(arg);
    if body.is_empty() {
        return Err(format!("-D expects NAME or NAME=VALUE, got '{arg}'"));
    }
    body.split(',').map(|part| parse_part(arg, part)).collect()
}

/// Keep the first value given for each name, in first-seen order: asl's rule
/// for a name defined twice on its command line.
pub fn first_wins(defines: &[(String, i64)]) -> Vec<(String, i64)> {
    let mut seen = std::collections::HashSet::new();
    defines.iter().filter(|(k, _)| seen.insert(k.as_str())).cloned().collect()
}

fn parse_part(arg: &str, part: &str) -> Result<(String, i64), String> {
    let (name, value) = match part.split_once('=') {
        Some((n, v)) => (n, Some(v)),
        None => (part, None),
    };
    if name.starts_with('.') {
        return Err(format!(
            "-D '{arg}': '{name}' starts with `.`, a local name no source line can read as this \
             define"
        ));
    }
    if !is_symbol_name(name) {
        return Err(if name.is_empty() {
            format!("-D '{arg}': an empty name (a list is NAME[=VALUE] parts separated by commas)")
        } else {
            format!(
                "-D '{arg}': '{name}' is not a symbol name (a letter or `_`, then letters, \
                 digits, `_` or `.`)"
            )
        });
    }
    let value = match value.map(str::trim) {
        None | Some("") => 1,
        Some(text) => eval_value(text).map_err(|why| format!("-D '{arg}': {name}={text}: {why}"))?,
    };
    Ok((name.to_string(), value))
}

fn is_symbol_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes.next().is_some_and(|c| c.is_ascii_alphabetic() || c == b'_' || c == b'.')
        && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
}

fn eval_value(text: &str) -> Result<i64, String> {
    if text.contains(['"', '\'']) {
        return Err("a quoted value is not supported (asl accepts one, and emits different \
                    bytes for it on every run)"
            .to_string());
    }
    if text.contains(';') {
        return Err("`;` is not part of an integer expression".to_string());
    }
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16)
            .map_err(|_| "not a 64-bit hexadecimal integer".to_string());
    }
    let not_int = || {
        "not an integer expression without symbols (decimal, $hex, %binary, 0x hex, and the \
         source's own operators; a symbol or a float is not supported)"
            .to_string()
    };
    let cs = CodePage::identity();
    let toks = lex_line(text, Cpu::M68000, &cs, SourceId(0), 0).map_err(|d| d.message)?;
    let ctx = ExprCtx { cs: &cs, nameless: NamelessCounts::default(), cpu: Cpu::M68000 };
    let expr = match parse_expr(&toks, &ctx) {
        Some((e, [])) => e,
        _ => return Err(not_int()),
    };
    match expr.fold(&|_| None) {
        Fold::Value(v) => Ok(v),
        Fold::Poison => Err(not_int()),
        Fold::Fault(f) => Err(f.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(arg: &str) -> Result<Vec<(String, i64)>, String> {
        parse_define_arg(arg)
    }
    fn pairs(v: &[(&str, i64)]) -> Vec<(String, i64)> {
        v.iter().map(|(k, x)| (k.to_string(), *x)).collect()
    }

    /// Every accepted row of the asl probe table, with asl's value.
    #[test]
    fn accepts_what_asl_accepts_with_asls_value() {
        assert_eq!(one("FOO=5"), Ok(pairs(&[("FOO", 5)])));
        assert_eq!(one("FOO"), Ok(pairs(&[("FOO", 1)])));
        assert_eq!(one("FOO="), Ok(pairs(&[("FOO", 1)])));
        assert_eq!(one("FOO=1,BAR=2"), Ok(pairs(&[("FOO", 1), ("BAR", 2)])));
        assert_eq!(one("FOO,BAR=2"), Ok(pairs(&[("FOO", 1), ("BAR", 2)])));
        assert_eq!(one("FOO=5,"), Ok(pairs(&[("FOO", 5)])));
        assert_eq!(one("FOO=$10"), Ok(pairs(&[("FOO", 0x10)])));
        assert_eq!(one("FOO=$ff"), Ok(pairs(&[("FOO", 0xFF)])));
        assert_eq!(one("FOO=0x10"), Ok(pairs(&[("FOO", 0x10)])));
        assert_eq!(one("FOO=0FFh"), Ok(pairs(&[("FOO", 0xFF)])));
        assert_eq!(one("FOO=%101"), Ok(pairs(&[("FOO", 5)])));
        assert_eq!(one("FOO=-1"), Ok(pairs(&[("FOO", -1)])));
        assert_eq!(one("FOO=~1"), Ok(pairs(&[("FOO", -2)])));
        assert_eq!(one("FOO=1+2"), Ok(pairs(&[("FOO", 3)])));
        assert_eq!(one("FOO=1 + 2"), Ok(pairs(&[("FOO", 3)])));
        assert_eq!(one("FOO=1<<4"), Ok(pairs(&[("FOO", 16)])));
        assert_eq!(one("FOO=(3)"), Ok(pairs(&[("FOO", 3)])));
        assert_eq!(one("FOO= 5"), Ok(pairs(&[("FOO", 5)])));
        assert_eq!(one("FOO=5 "), Ok(pairs(&[("FOO", 5)])));
        assert_eq!(one("FOO=1=1"), Ok(pairs(&[("FOO", 1)])));
        assert_eq!(one("FOO=$FFFFFFFF"), Ok(pairs(&[("FOO", 0xFFFF_FFFF)])));
        assert_eq!(one("FOO=$123456789"), Ok(pairs(&[("FOO", 0x1_2345_6789)])));
        assert_eq!(one("FOO.BAR=1"), Ok(pairs(&[("FOO.BAR", 1)])));
        assert_eq!(one("_FOO=1"), Ok(pairs(&[("_FOO", 1)])));
        assert_eq!(one("Sonic3_Complete=0"), Ok(pairs(&[("Sonic3_Complete", 0)])));
    }

    /// Every refused row of the asl probe table (asl: `Invalid option: -D`).
    #[test]
    fn refuses_what_asl_refuses() {
        for arg in [
            "", "1FOO=1", "=1", "FO-O=1", "FOO?=1", "@FOO=1", ",FOO=5", "FOO==5", "FOO=$",
            "FOO=5x", "FOO=1+", "FOO=ff", "FOO=1, BAR=2", "FOO =5", "FOO=1,,BAR=2", "FOO=BAR",
        ] {
            assert!(one(arg).is_err(), "`-D {arg}` must be refused, got {:?}", one(arg));
        }
    }

    /// asl accepts these and sigil refuses them, each for a stated reason.
    #[test]
    fn refuses_the_documented_gaps() {
        assert!(one("FOO=\"ab\"").unwrap_err().contains("quoted"));
        assert!(one("FOO='A'").unwrap_err().contains("quoted"));
        assert!(one(".FOO=1").unwrap_err().contains("starts with `.`"));
        assert!(one("FOO=1.5").is_err());
        assert!(one("FOO=0b101").is_err());
        assert!(one("FOO=@17").is_err());
        assert!(one("FOO=$FFFFFFFFFFFFFFFF").is_err());
        assert!(one("FOO=$10000000000000000").is_err());
    }

    #[test]
    fn a_repeated_name_keeps_its_first_value() {
        let mut all = one("FOO=1,FOO=2").unwrap();
        all.extend(one("FOO=3").unwrap());
        assert_eq!(first_wins(&all), pairs(&[("FOO", 1)]));
        let mut all = one("FOO").unwrap();
        all.extend(one("FOO=2").unwrap());
        assert_eq!(first_wins(&all), pairs(&[("FOO", 1)]));
    }
}
