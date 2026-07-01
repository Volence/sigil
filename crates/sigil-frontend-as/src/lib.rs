//! sigil-frontend-as: assembly-syntax frontend (parse_line / assemble_str).

use sigil_isa::z80::{Instruction, Reg8};
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

/// Parse a single register token (already lowercased) into a [`Reg8`].
fn parse_reg(tok: &str) -> Option<Reg8> {
    match tok {
        "b" => Some(Reg8::B),
        "c" => Some(Reg8::C),
        "d" => Some(Reg8::D),
        "e" => Some(Reg8::E),
        "h" => Some(Reg8::H),
        "l" => Some(Reg8::L),
        "(hl)" => Some(Reg8::Hl),
        "a" => Some(Reg8::A),
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
        "nop" => Ok(Some(Instruction::Nop)),
        "ld" => {
            let (dst_tok, src_tok) = split_operands(rest)?;
            let dst = parse_reg(&dst_tok)
                .ok_or_else(|| ParseError::BadOperand(dst_tok.clone()))?;
            if let Some(src) = parse_reg(&src_tok) {
                Ok(Some(Instruction::LdRegReg { dst, src }))
            } else if let Some(imm) = parse_int(&src_tok) {
                if !(0..=0xFF).contains(&imm) {
                    return Err(ParseError::BadOperand(src_tok));
                }
                Ok(Some(Instruction::LdRegImm { dst, imm: imm as u8 }))
            } else {
                Err(ParseError::BadOperand(src_tok))
            }
        }
        "add" => {
            let (dst_tok, src_tok) = split_operands(rest)?;
            if dst_tok != "a" {
                return Err(ParseError::BadOperand(dst_tok));
            }
            let src = parse_reg(&src_tok)
                .ok_or_else(|| ParseError::BadOperand(src_tok.clone()))?;
            Ok(Some(Instruction::AddAReg { src }))
        }
        "jp" => {
            let imm = parse_int(rest)
                .ok_or_else(|| ParseError::BadOperand(rest.to_string()))?;
            if !(0..=0xFFFF).contains(&imm) {
                return Err(ParseError::BadOperand(rest.to_string()));
            }
            Ok(Some(Instruction::JpImm { addr: imm as u16 }))
        }
        _ => Err(ParseError::UnknownMnemonic(mnemonic.to_string())),
    }
}

/// Assemble a multi-line Z80 source string into a flat machine-code image.
///
/// Each non-empty, non-comment line is parsed by [`parse_line`], encoded via
/// `sigil_isa::z80::encode`, and streamed through a [`ModuleBuilder`].  The
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
    use sigil_isa::z80::{Instruction, Reg8};

    #[test]
    fn parse_nop() {
        assert_eq!(parse_line("nop"), Ok(Some(Instruction::Nop)));
    }

    #[test]
    fn parse_ld_imm_forms() {
        assert_eq!(
            parse_line("ld a, 5"),
            Ok(Some(Instruction::LdRegImm { dst: Reg8::A, imm: 5 }))
        );
        assert_eq!(
            parse_line("ld c, 0x0A"),
            Ok(Some(Instruction::LdRegImm { dst: Reg8::C, imm: 10 }))
        );
        assert_eq!(
            parse_line("ld d, $FF"),
            Ok(Some(Instruction::LdRegImm { dst: Reg8::D, imm: 255 }))
        );
    }

    #[test]
    fn parse_ld_reg_reg() {
        assert_eq!(
            parse_line("ld b, c"),
            Ok(Some(Instruction::LdRegReg { dst: Reg8::B, src: Reg8::C }))
        );
    }

    #[test]
    fn parse_add_a_reg() {
        assert_eq!(
            parse_line("add a, b"),
            Ok(Some(Instruction::AddAReg { src: Reg8::B }))
        );
    }

    #[test]
    fn parse_jp_imm_forms() {
        assert_eq!(parse_line("jp $1234"), Ok(Some(Instruction::JpImm { addr: 0x1234 })));
        assert_eq!(parse_line("jp 0x00FF"), Ok(Some(Instruction::JpImm { addr: 0x00FF })));
        assert_eq!(parse_line("jp 65535"), Ok(Some(Instruction::JpImm { addr: 65535 })));
    }

    #[test]
    fn parse_comment_and_blank() {
        assert_eq!(parse_line(""), Ok(None));
        assert_eq!(parse_line("   "), Ok(None));
        assert_eq!(parse_line("; just a comment"), Ok(None));
        assert_eq!(parse_line("nop ; trailing comment"), Ok(Some(Instruction::Nop)));
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
}
