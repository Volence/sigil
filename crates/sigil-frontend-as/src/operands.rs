//! operands: comma-split + structural classification of operand groups.

use crate::expr::parse_expr;
use crate::token::{Punct, Tok, Token};
use sigil_backend_z80::z80::IndexReg;
use sigil_ir::Expr;
use sigil_span::{Diagnostic, Level, Span};

/// A structurally-classified operand. Register-vs-condition ambiguity is left in
/// `RegOrCond` for eval to resolve by mnemonic.
#[derive(Clone, Debug)]
pub enum OperandAtom {
    /// A bare word: a register (`a`, `hl`), a condition (`nz`), or `i`/`r`.
    RegOrCond(String),
    /// `(hl)` / `(bc)` / `(de)`.
    IndReg(String),
    /// `(ix+d)` / `(iy+d)`.
    Indexed { reg: IndexReg, disp: Expr },
    /// `(nn)` — absolute memory address.
    Mem(Expr),
    /// A bare expression: immediate, bit number, or a symbolic address.
    Value(Expr),
    /// `af'`.
    AfShadow,
    /// `#expr` — 68k explicit immediate marker.
    Imm(Expr),
    /// `-(An)` — 68k predecrement indirect.
    M68kPreDec(String),
    /// `(An)+` — 68k postincrement indirect.
    M68kPostInc(String),
    /// `(An)` — 68k address-register indirect (`reg` is `a0..a7`, unambiguous vs Z80).
    M68kInd(String),
    /// `(d16,An)` — 68k address-register indirect with displacement.
    M68kDisp { disp: Expr, an: String },
    /// `(d8,An,Xn.w|.l)` — 68k address-register indirect with index.
    M68kIdx { disp: Expr, an: String, xn: String, xlong: bool },
}

fn err(span: Span, msg: &str) -> Diagnostic {
    Diagnostic { level: Level::Error, message: msg.to_string(), primary: span }
}

/// Split `toks` on top-level commas and classify each group.
pub fn parse_operands(toks: &[Token]) -> Result<Vec<OperandAtom>, Diagnostic> {
    if toks.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for group in split_commas(toks) {
        out.push(classify(group)?);
    }
    Ok(out)
}

/// Split on commas not nested inside parentheses.
fn split_commas(toks: &[Token]) -> Vec<&[Token]> {
    let mut groups = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::Punct(Punct::LParen) => depth += 1,
            Tok::Punct(Punct::RParen) => depth -= 1,
            Tok::Punct(Punct::Comma) if depth == 0 => {
                groups.push(&toks[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    groups.push(&toks[start..]);
    groups
}

fn classify(g: &[Token]) -> Result<OperandAtom, Diagnostic> {
    let span = g.first().map(|t| t.span).unwrap_or(Span { source: sigil_span::SourceId(0), start: 0, end: 0 });
    // `#expr` — 68k immediate marker.
    if let Some(Token { tok: Tok::Punct(Punct::Hash), .. }) = g.first() {
        let (e, rest) = parse_expr(&g[1..]).ok_or_else(|| err(span, "bad immediate expression"))?;
        if !rest.is_empty() {
            return Err(err(span, "trailing tokens in #immediate"));
        }
        return Ok(OperandAtom::Imm(e));
    }
    // af'
    if let [Token { tok: Tok::Ident(w), .. }] = g {
        if w == "af'" {
            return Ok(OperandAtom::AfShadow);
        }
        if is_reg_or_cond_word(w) {
            return Ok(OperandAtom::RegOrCond(w.clone()));
        }
    }
    // `-(An)` — 68k predecrement indirect. Exact 4-token shape only (`-`, `(`,
    // single ident, `)`) so an arithmetic `-(expr)` immediate (multi-token
    // inner) falls through to the ordinary expression parse below.
    if let [
        Token { tok: Tok::Punct(Punct::Minus), .. },
        Token { tok: Tok::Punct(Punct::LParen), .. },
        Token { tok: Tok::Ident(reg), .. },
        Token { tok: Tok::Punct(Punct::RParen), .. },
    ] = g
    {
        return Ok(OperandAtom::M68kPreDec(reg.clone()));
    }
    // `(An)+` — 68k postincrement indirect. Only recognized when the inner is
    // a single ident; anything else with this shape is a malformed operand.
    if g.len() >= 3 {
        if let (
            Some(Token { tok: Tok::Punct(Punct::LParen), .. }),
            Some(Token { tok: Tok::Punct(Punct::Plus), .. }),
            Some(Token { tok: Tok::Punct(Punct::RParen), .. }),
        ) = (g.first(), g.last(), g.get(g.len() - 2))
        {
            let inner = &g[1..g.len() - 2];
            if let [Token { tok: Tok::Ident(reg), .. }] = inner {
                return Ok(OperandAtom::M68kPostInc(reg.clone()));
            }
            return Err(err(span, "bad `(An)+` operand"));
        }
    }
    // Parenthesised: (reg) / (ix+d) / (nn) / (d,An) / (d,An,Xn).
    if let (Some(Token { tok: Tok::Punct(Punct::LParen), .. }), Some(Token { tok: Tok::Punct(Punct::RParen), .. })) =
        (g.first(), g.last())
    {
        let inner = &g[1..g.len() - 1];
        // (hl)/(bc)/(de), plus (sp) for `ex (sp),hl` (eval gates it by mnemonic).
        if let [Token { tok: Tok::Ident(w), .. }] = inner {
            if matches!(w.as_str(), "hl" | "bc" | "de" | "sp") {
                return Ok(OperandAtom::IndReg(w.clone()));
            }
        }
        // (An) — 68k address-register indirect. `a`+digit is unambiguously
        // 68k (Z80 has no such register names), so this is safe to recognize
        // structurally without a CPU flag.
        if let [Token { tok: Tok::Ident(w), .. }] = inner {
            if is_m68k_areg_name(w) {
                return Ok(OperandAtom::M68kInd(w.clone()));
            }
        }
        // (ix±d)/(iy±d)
        if let Some(Token { tok: Tok::Ident(reg), .. }) = inner.first() {
            if let Some(ir) = index_reg(reg) {
                let disp = parse_indexed_disp(&inner[1..], span)?;
                return Ok(OperandAtom::Indexed { reg: ir, disp });
            }
        }
        // (d,An) / (d,An,Xn) — split the inner on top-level commas.
        let inner_groups = split_commas(inner);
        if inner_groups.len() == 2 {
            let (disp, rest) = parse_expr(inner_groups[0]).ok_or_else(|| err(span, "bad displacement expression"))?;
            if !rest.is_empty() {
                return Err(err(span, "trailing tokens in displacement"));
            }
            let an = match inner_groups[1] {
                [Token { tok: Tok::Ident(w), .. }] => w.clone(),
                _ => return Err(err(span, "expected an address register in `(d,An)`")),
            };
            return Ok(OperandAtom::M68kDisp { disp, an });
        }
        if inner_groups.len() == 3 {
            let (disp, rest) = parse_expr(inner_groups[0]).ok_or_else(|| err(span, "bad displacement expression"))?;
            if !rest.is_empty() {
                return Err(err(span, "trailing tokens in displacement"));
            }
            let an = match inner_groups[1] {
                [Token { tok: Tok::Ident(w), .. }] => w.clone(),
                _ => return Err(err(span, "expected an address register in `(d,An,Xn)`")),
            };
            let (xn, xlong) = match inner_groups[2] {
                [Token { tok: Tok::Ident(w), .. }] => split_index_reg_size(w),
                _ => return Err(err(span, "expected an index register in `(d,An,Xn)`")),
            };
            return Ok(OperandAtom::M68kIdx { disp, an, xn, xlong });
        }
        // (nn) absolute
        let (e, rest) = parse_expr(inner).ok_or_else(|| err(span, "bad address expression"))?;
        if !rest.is_empty() {
            return Err(err(span, "trailing tokens in (address)"));
        }
        return Ok(OperandAtom::Mem(e));
    }
    // Bare expression.
    let (e, rest) = parse_expr(g).ok_or_else(|| err(span, "bad operand expression"))?;
    if !rest.is_empty() {
        return Err(err(span, "trailing tokens in operand"));
    }
    Ok(OperandAtom::Value(e))
}

/// Parse an index displacement: tokens after `ix`/`iy`, beginning with `+`/`-`.
fn parse_indexed_disp(rest: &[Token], span: Span) -> Result<Expr, Diagnostic> {
    match rest.first().map(|t| &t.tok) {
        Some(Tok::Punct(Punct::Plus)) => {
            let (e, tail) = parse_expr(&rest[1..]).ok_or_else(|| err(span, "bad +disp"))?;
            if !tail.is_empty() { return Err(err(span, "trailing tokens in disp")); }
            Ok(e)
        }
        Some(Tok::Punct(Punct::Minus)) => {
            let (e, tail) = parse_expr(&rest[1..]).ok_or_else(|| err(span, "bad -disp"))?;
            if !tail.is_empty() { return Err(err(span, "trailing tokens in disp")); }
            Ok(Expr::Unary { op: sigil_ir::expr::UnOp::Neg, operand: Box::new(e) })
        }
        _ => Err(err(span, "index operand needs `+`/`-` displacement")),
    }
}

fn index_reg(w: &str) -> Option<IndexReg> {
    match w {
        "ix" => Some(IndexReg::Ix),
        "iy" => Some(IndexReg::Iy),
        _ => None,
    }
}

/// `true` iff `w` is `a0`..`a7` — the 68k address-register spelling. Deliberately
/// excludes `sp`: that alias stays on the pre-existing Z80 `hl`/`bc`/`de`/`sp`
/// branch (see `classify`), so `(sp)` still parses as `IndReg("sp")`.
fn is_m68k_areg_name(w: &str) -> bool {
    w.strip_prefix('a').and_then(|d| d.parse::<u8>().ok()).is_some_and(|n| n <= 7)
}

/// Split a 68k index-register token (`d1`, `d1.w`, `a2.l`, ...) into its bare
/// register name and the index-size flag (`.w` → `false`, `.l` → `true`,
/// absent → `false`, matching asl's default). The lexer folds the `.w`/`.l`
/// suffix into the identifier itself (`.` is an identifier character), so
/// this is plain string surgery, not further tokenising.
fn split_index_reg_size(w: &str) -> (String, bool) {
    if let Some(base) = w.strip_suffix(".l") {
        (base.to_string(), true)
    } else if let Some(base) = w.strip_suffix(".w") {
        (base.to_string(), false)
    } else {
        (w.to_string(), false)
    }
}

/// The bare words eval may interpret as a register, pair, or condition.
fn is_reg_or_cond_word(w: &str) -> bool {
    matches!(
        w,
        "a" | "b" | "c" | "d" | "e" | "h" | "l"
            | "bc" | "de" | "hl" | "sp" | "af" | "ix" | "iy"
            | "nz" | "z" | "nc" | "po" | "pe" | "p" | "m"
            | "i" | "r"
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_operands, OperandAtom};
    use crate::lexer::lex_line;
    use sigil_ir::backend::Cpu;
    use sigil_ir::expr::Fold;
    use sigil_span::SourceId;

    fn atoms(src: &str) -> Vec<OperandAtom> {
        let toks = lex_line(src, Cpu::Z80, SourceId(0), 0).unwrap();
        parse_operands(&toks).unwrap()
    }

    #[test]
    fn word_and_paren_forms() {
        assert!(matches!(atoms("a").as_slice(), [OperandAtom::RegOrCond(w)] if w == "a"));
        assert!(matches!(atoms("(hl)").as_slice(), [OperandAtom::IndReg(w)] if w == "hl"));
        assert!(matches!(atoms("nz").as_slice(), [OperandAtom::RegOrCond(w)] if w == "nz"));
    }

    #[test]
    fn two_operands_reg_and_value() {
        let a = atoms("a,0FFh");
        assert_eq!(a.len(), 2);
        assert!(matches!(&a[0], OperandAtom::RegOrCond(w) if w == "a"));
        match &a[1] {
            OperandAtom::Value(e) => assert_eq!(e.fold(&|_| None), Fold::Value(0xFF)),
            _ => panic!("want Value"),
        }
    }

    #[test]
    fn indexed_with_symbolic_and_plus_one_disp() {
        // (ix+sc_flags)
        match &atoms("(ix+sc_flags)")[0] {
            OperandAtom::Indexed { reg, disp } => {
                assert!(matches!(reg, sigil_backend_z80::z80::IndexReg::Ix));
                assert_eq!(disp.fold(&|n| if n == "sc_flags" { Some(10) } else { None }), Fold::Value(10));
            }
            _ => panic!("want Indexed"),
        }
        // (ix+sc_mod_ptr+1)
        match &atoms("(ix+sc_mod_ptr+1)")[0] {
            OperandAtom::Indexed { disp, .. } => {
                assert_eq!(disp.fold(&|n| if n == "sc_mod_ptr" { Some(2) } else { None }), Fold::Value(3));
            }
            _ => panic!("want Indexed"),
        }
    }

    #[test]
    fn absolute_mem_vs_symbol_value() {
        assert!(matches!(&atoms("(SND_TEMPO_CUR)")[0], OperandAtom::Mem(_)));
        assert!(matches!(&atoms("SfxBlobWinTab")[0], OperandAtom::Value(_)));
    }

    fn atoms_68k(src: &str) -> Vec<OperandAtom> {
        let toks = lex_line(src, Cpu::M68000, SourceId(0), 0).unwrap();
        parse_operands(&toks).unwrap()
    }

    #[test]
    fn hash_immediate_marker_produces_imm_atom() {
        match &atoms_68k("#5")[0] {
            OperandAtom::Imm(e) => assert_eq!(e.fold(&|_| None), Fold::Value(5)),
            other => panic!("want Imm, got {other:?}"),
        }
        // Two operands: #imm then a bare register-ish word (still a Value here —
        // 68k register recognition is eval.rs's job, not operands.rs's).
        let a = atoms_68k("#$1234,d0");
        assert_eq!(a.len(), 2);
        match &a[0] {
            OperandAtom::Imm(e) => assert_eq!(e.fold(&|_| None), Fold::Value(0x1234)),
            other => panic!("want Imm, got {other:?}"),
        }
    }

    #[test]
    fn m68k_ind_form_parses() {
        match &atoms_68k("(a0)")[0] {
            OperandAtom::M68kInd(w) => assert_eq!(w, "a0"),
            other => panic!("want M68kInd, got {other:?}"),
        }
        // `(sp)` still parses down the pre-existing Z80 branch (eval.rs maps
        // it to `Ind(7)` separately).
        match &atoms_68k("(sp)")[0] {
            OperandAtom::IndReg(w) => assert_eq!(w, "sp"),
            other => panic!("want IndReg, got {other:?}"),
        }
    }

    #[test]
    fn m68k_postinc_form_parses() {
        match &atoms_68k("(a1)+")[0] {
            OperandAtom::M68kPostInc(w) => assert_eq!(w, "a1"),
            other => panic!("want M68kPostInc, got {other:?}"),
        }
    }

    #[test]
    fn m68k_predec_form_parses() {
        match &atoms_68k("-(a2)")[0] {
            OperandAtom::M68kPreDec(w) => assert_eq!(w, "a2"),
            other => panic!("want M68kPreDec, got {other:?}"),
        }
        // A genuine arithmetic negation of a parenthesised multi-token
        // expression is NOT a predecrement — falls through to Value.
        match &atoms_68k("-(2+3)")[0] {
            OperandAtom::Value(e) => assert_eq!(e.fold(&|_| None), Fold::Value(-5)),
            other => panic!("want Value, got {other:?}"),
        }
    }

    #[test]
    fn m68k_disp16_an_form_parses() {
        match &atoms_68k("(4,a0)")[0] {
            OperandAtom::M68kDisp { disp, an } => {
                assert_eq!(disp.fold(&|_| None), Fold::Value(4));
                assert_eq!(an, "a0");
            }
            other => panic!("want M68kDisp, got {other:?}"),
        }
    }

    #[test]
    fn m68k_disp8_an_xn_form_parses_word_and_long_index() {
        match &atoms_68k("(6,a0,d1.w)")[0] {
            OperandAtom::M68kIdx { disp, an, xn, xlong } => {
                assert_eq!(disp.fold(&|_| None), Fold::Value(6));
                assert_eq!(an, "a0");
                assert_eq!(xn, "d1");
                assert!(!xlong);
            }
            other => panic!("want M68kIdx, got {other:?}"),
        }
        match &atoms_68k("(8,a0,a2.l)")[0] {
            OperandAtom::M68kIdx { disp, an, xn, xlong } => {
                assert_eq!(disp.fold(&|_| None), Fold::Value(8));
                assert_eq!(an, "a0");
                assert_eq!(xn, "a2");
                assert!(*xlong);
            }
            other => panic!("want M68kIdx, got {other:?}"),
        }
    }
}
