//! sigil-frontend-as: assembly-syntax frontend (parse_line / assemble_str).
//!
//! Plan-1 coverage only (nop; ld r,r'/r,n; add a,r; jp nn), retargeted onto the
//! Plan-2 canonical `sigil-isa` operand/instruction model. The `sigil-isa` edge
//! and the whole front-end are rewritten in a later plan.

use sigil_isa::z80::{Instruction, Mnemonic, Operand, Reg8};
use sigil_ir::{assemble_to_image, ModuleBuilder, Streamer};
use sigil_span::{SourceId, Span};

/// An error produced while parsing or lowering assembly source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// An unrecognised mnemonic was encountered.
    UnknownMnemonic(String),
    /// An operand token could not be decoded.
    BadOperand(String),
    /// An ISA encoding error bubbled up from the encoder.
    Isa(sigil_isa::z80::IsaError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownMnemonic(m) => write!(f, "unknown mnemonic: {m}"),
            ParseError::BadOperand(s) => write!(f, "bad operand: {s}"),
            ParseError::Isa(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a register-or-`(hl)` operand token (already lowercased) into an
/// [`Operand`]. Pure registers become [`Operand::Reg`]; `(hl)` becomes
/// [`Operand::IndHl`].
fn parse_reg_operand(tok: &str) -> Option<Operand> {
    match tok {
        "b" => Some(Operand::Reg(Reg8::B)),
        "c" => Some(Operand::Reg(Reg8::C)),
        "d" => Some(Operand::Reg(Reg8::D)),
        "e" => Some(Operand::Reg(Reg8::E)),
        "h" => Some(Operand::Reg(Reg8::H)),
        "l" => Some(Operand::Reg(Reg8::L)),
        "a" => Some(Operand::Reg(Reg8::A)),
        "(hl)" => Some(Operand::IndHl),
        _ => None,
    }
}

/// Parse an integer literal: decimal, `$hex`, or `0x`-hex. Returns `None` on failure.
fn parse_int(tok: &str) -> Option<i64> {
    if let Some(hex) = tok.strip_prefix('$') {
        i64::from_str_radix(hex, 16).ok()
    } else if let Some(hex) = tok.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok()
    } else {
        tok.parse::<i64>().ok()
    }
}

/// Split `dst, src` operand text at the first comma, trimming both sides.
fn split_operands(rest: &str) -> Result<(String, String), ParseError> {
    match rest.split_once(',') {
        Some((a, b)) => Ok((
            a.trim().to_ascii_lowercase(),
            b.trim().to_ascii_lowercase(),
        )),
        None => Err(ParseError::BadOperand(rest.trim().to_string())),
    }
}

/// Parse one line of assembly source.
///
/// Returns `Ok(None)` for blank lines and comment-only lines.
/// Strips everything from the first `;` to end of line before parsing.
pub fn parse_line(line: &str) -> Result<Option<Instruction>, ParseError> {
    // Strip comments.
    let code = match line.split_once(';') {
        Some((before, _)) => before,
        None => line,
    };
    let code = code.trim();
    if code.is_empty() {
        return Ok(None);
    }

    // Split mnemonic from operands.
    let (mnemonic, rest) = match code.split_once(char::is_whitespace) {
        Some((m, r)) => (m, r.trim()),
        None => (code, ""),
    };

    match mnemonic.to_ascii_lowercase().as_str() {
        "nop" => Ok(Some(Instruction { mnemonic: Mnemonic::Nop, ops: vec![] })),
        "ld" => {
            let (dst_tok, src_tok) = split_operands(rest)?;
            let dst = parse_reg_operand(&dst_tok)
                .ok_or_else(|| ParseError::BadOperand(dst_tok.clone()))?;
            if let Some(src) = parse_reg_operand(&src_tok) {
                Ok(Some(Instruction { mnemonic: Mnemonic::Ld, ops: vec![dst, src] }))
            } else if let Some(imm) = parse_int(&src_tok) {
                if !(0..=0xFF).contains(&imm) {
                    return Err(ParseError::BadOperand(src_tok));
                }
                Ok(Some(Instruction {
                    mnemonic: Mnemonic::Ld,
                    ops: vec![dst, Operand::Imm8(imm as u8)],
                }))
            } else {
                Err(ParseError::BadOperand(src_tok))
            }
        }
        "add" => {
            let (dst_tok, src_tok) = split_operands(rest)?;
            if dst_tok != "a" {
                return Err(ParseError::BadOperand(dst_tok));
            }
            let src = parse_reg_operand(&src_tok)
                .ok_or_else(|| ParseError::BadOperand(src_tok.clone()))?;
            Ok(Some(Instruction {
                mnemonic: Mnemonic::Add,
                ops: vec![Operand::Reg(Reg8::A), src],
            }))
        }
        "jp" => {
            let imm = parse_int(rest)
                .ok_or_else(|| ParseError::BadOperand(rest.to_string()))?;
            if !(0..=0xFFFF).contains(&imm) {
                return Err(ParseError::BadOperand(rest.to_string()));
            }
            Ok(Some(Instruction {
                mnemonic: Mnemonic::Jp,
                ops: vec![Operand::Imm16(imm as u16)],
            }))
        }
        _ => Err(ParseError::UnknownMnemonic(mnemonic.to_string())),
    }
}

/// Assemble a multi-line Z80 source string into a flat machine-code image.
///
/// Each non-empty, non-comment line is parsed by [`parse_line`], encoded via
/// `sigil_isa::z80::encode`, and streamed through a [`ModuleBuilder`]. The
/// resulting [`sigil_ir::Module`] is flattened by [`assemble_to_image`].
pub fn assemble_str(src: &str) -> Result<Vec<u8>, ParseError> {
    let span = Span { source: SourceId(0), start: 0, end: 0 };
    let mut builder = ModuleBuilder::new(span);
    for line in src.lines() {
        if let Some(inst) = parse_line(line)? {
            let bytes = sigil_isa::z80::encode(&inst).map_err(ParseError::Isa)?;
            builder.emit_bytes(&bytes);
        }
    }
    Ok(assemble_to_image(&builder.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_isa::z80::{Instruction, Mnemonic, Operand, Reg8};

    fn ld(dst: Operand, src: Operand) -> Instruction {
        Instruction { mnemonic: Mnemonic::Ld, ops: vec![dst, src] }
    }

    #[test]
    fn parse_nop() {
        assert_eq!(
            parse_line("nop"),
            Ok(Some(Instruction { mnemonic: Mnemonic::Nop, ops: vec![] }))
        );
    }

    #[test]
    fn parse_ld_imm_forms() {
        assert_eq!(
            parse_line("ld a, 5"),
            Ok(Some(ld(Operand::Reg(Reg8::A), Operand::Imm8(5))))
        );
        assert_eq!(
            parse_line("ld c, 0x0A"),
            Ok(Some(ld(Operand::Reg(Reg8::C), Operand::Imm8(10))))
        );
        assert_eq!(
            parse_line("ld d, $FF"),
            Ok(Some(ld(Operand::Reg(Reg8::D), Operand::Imm8(255))))
        );
    }

    #[test]
    fn parse_ld_reg_reg() {
        assert_eq!(
            parse_line("ld b, c"),
            Ok(Some(ld(Operand::Reg(Reg8::B), Operand::Reg(Reg8::C))))
        );
    }

    #[test]
    fn parse_add_a_reg() {
        assert_eq!(
            parse_line("add a, b"),
            Ok(Some(Instruction {
                mnemonic: Mnemonic::Add,
                ops: vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::B)],
            }))
        );
    }

    #[test]
    fn parse_jp_imm_forms() {
        assert_eq!(
            parse_line("jp $1234"),
            Ok(Some(Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x1234)] }))
        );
        assert_eq!(
            parse_line("jp 0x00FF"),
            Ok(Some(Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x00FF)] }))
        );
        assert_eq!(
            parse_line("jp 65535"),
            Ok(Some(Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(65535)] }))
        );
    }

    #[test]
    fn parse_comment_and_blank() {
        assert_eq!(parse_line(""), Ok(None));
        assert_eq!(parse_line("   "), Ok(None));
        assert_eq!(parse_line("; just a comment"), Ok(None));
        assert_eq!(
            parse_line("nop ; trailing comment"),
            Ok(Some(Instruction { mnemonic: Mnemonic::Nop, ops: vec![] }))
        );
    }

    #[test]
    fn parse_unknown_mnemonic() {
        assert_eq!(
            parse_line("foo a, b"),
            Err(ParseError::UnknownMnemonic("foo".to_string()))
        );
    }

    #[test]
    fn assemble_golden_sample() {
        let src = "nop\nld a, 5\nld b, 10\nld b, c\nld a, a\nadd a, b\nadd a, a\njp $1234\njp 0x00FF\n";
        let expected = vec![
            0x00, 0x3E, 0x05, 0x06, 0x0A, 0x41, 0x7F, 0x80, 0x87, 0xC3, 0x34, 0x12, 0xC3, 0xFF,
            0x00,
        ];
        assert_eq!(assemble_str(src), Ok(expected));
    }

    // ── Negative / error-path tests ──────────────────────────────────────────

    /// `ld (hl), (hl)` parses successfully (dst=IndHl, src=IndHl) but that pairing
    /// is HALT (0x76), outside the driver ISA, so the encoder rejects it and
    /// assemble_str surfaces ParseError::Isa(IsaError::UnsupportedForm(_)).
    ///
    /// (Base-group `(HL)` forms like `ld a,(hl)` now assemble — see the
    /// `assemble_hl_operand_encodes` test — so this exercises a still-unsupported
    /// pairing to keep the ISA-error path covered.)
    #[test]
    fn assemble_hl_operand_surfaces_isa_error() {
        assert!(matches!(
            assemble_str("ld (hl), (hl)\n"),
            Err(ParseError::Isa(sigil_isa::z80::IsaError::UnsupportedForm(_)))
        ));
    }

    /// The base group encodes `(HL)` load/ALU forms, so the frontend now assembles
    /// `ld a,(hl)` (=7E) end-to-end instead of surfacing an ISA error.
    #[test]
    fn assemble_hl_operand_encodes() {
        assert_eq!(assemble_str("ld a, (hl)\n"), Ok(vec![0x7E]));
    }

    /// An 8-bit immediate that is out of the 0..=255 range must return
    /// ParseError::BadOperand.
    #[test]
    fn parse_imm8_out_of_range() {
        assert!(matches!(
            parse_line("ld a, 300"),
            Err(ParseError::BadOperand(_))
        ));
    }

    /// A 16-bit address that exceeds 0xFFFF must return ParseError::BadOperand.
    #[test]
    fn parse_jp_addr_out_of_range() {
        assert!(matches!(
            parse_line("jp 0x10000"),
            Err(ParseError::BadOperand(_))
        ));
    }

    /// A missing comma between operands must return ParseError::BadOperand
    /// (from split_operands failing to find a comma).
    #[test]
    fn parse_missing_comma() {
        assert!(matches!(
            parse_line("ld a 5"),
            Err(ParseError::BadOperand(_))
        ));
    }

    /// `add b, c` — the destination is not `a`, which is invalid for ADD.
    /// The parser returns ParseError::BadOperand for the non-`a` destination.
    #[test]
    fn parse_add_non_a_destination() {
        assert!(matches!(
            parse_line("add b, c"),
            Err(ParseError::BadOperand(_))
        ));
    }
}
