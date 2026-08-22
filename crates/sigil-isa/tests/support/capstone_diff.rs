//! Shared machinery for the capstone differential gates: sigil's 68000
//! encoder/decoder against **Capstone**.
//!
//! Two gates consume this module — the opcode-space sweep
//! (`sigil-isa/tests/m68k_capstone_differential.rs`) and the emitted-stream
//! pass (`sigil-harness/tests/m68k_capstone_stream.rs`). It lives under
//! `tests/support/` (a subdirectory, so cargo does not build it as a test
//! target of its own) and the harness side reaches it with an explicit
//! `#[path]`, because the two gates must share ONE definition of what
//! "disagreement" means and one set of named exclusions. Two copies would
//! drift, and a drifted exclusion is a hole.
//!
//! # Why this exists
//!
//! Every other m68k gate in this repo is circular in one specific way: the
//! opcode sweep's oracle is sigil's own `encode`, so it proves *decoder ⊆
//! encoder* and is blind to a defect both halves share. The class is not
//! hypothetical — `TST`'s destination row is `DATA_ALTERABLE` on the MC68000,
//! and a row widened to `DATA` admits nine words the hardware traps while
//! round-tripping perfectly, because encoder and decoder read the same row.
//! This gate names all nine.
//!
//! Capstone is an independently written disassembler with no shared lineage
//! with sigil, which makes it the one non-circular 68000 oracle available.
//! `CS_MODE_M68K_000` is its MC68000 decode mode.
//!
//! # What is compared, and what is deliberately NOT
//!
//! Comparing capstone's operand TEXT to sigil's would measure spelling. Both
//! sides are instead normalised into the same abstract form
//! ([`Canon`]) and that form is compared:
//!
//! - **legality** — sigil decodes ⟹ capstone decodes. The reverse does NOT
//!   hold and is not asserted: sigil's decoder covers exactly the forms its
//!   encoder emits, so capstone decoding `exg`/`abcd`/`chk` where sigil
//!   declines is expected and carries no information.
//! - **length** — the consumed byte count.
//! - **family** — the mnemonic, in sigil's vocabulary, with the
//!   condition-parameterised families expanded (`scc`+`Eq` → `seq`) because
//!   capstone spells the condition into the mnemonic.
//! - **operation size** — only where capstone reports one. Capstone prints no
//!   suffix at all for `jmp`/`swap`/`trap`/`rts`/`dbcc`/`move …,sr`, so there
//!   is nothing to compare there; that is capstone declining to answer, not
//!   agreement.
//! - **operands** — an ordered list of canonical operand strings: EA mode,
//!   register number, displacement, absolute value, immediate value (masked to
//!   the operation width), register-list mask, and branch / PC-relative targets
//!   resolved to a byte offset from the start of the instruction.
//!
//! PC-relative operands need the byte offset of their own extension word to
//! resolve capstone's absolute target, and the decoder does not report it. It
//! is DERIVED here rather than tabulated: re-encode the instruction with that
//! one displacement perturbed and diff the bytes — the word that moves is the
//! extension word. See [`pc_ext_offset`].
//!
//! # The oracle is the subject too
//!
//! Capstone is not authoritative by fiat. Where the two disagree the question
//! is which one matches the MC68000 programmer's reference, and both answers
//! have been observed. The named exclusions below are the cases where capstone
//! is the one that is wrong; each carries its derivation, and each is written
//! as a predicate over a specific, counted word class so that it cannot
//! silently widen — the run prints every exclusion's hit count and FAILS if a
//! class matches nothing, which is what an exclusion outliving its cause looks
//! like.
//!
//! # Availability
//!
//! Capstone is a Python package. "capstone is not installed" must never read as
//! coverage, so a missing oracle prints a `skip:` line and, under
//! `SIGIL_STRICT_GATE=1`, panics instead — the repo-wide convention
//! (`sigil-harness::test_support::reference_tree`) applied to a tool rather
//! than a source tree.

// Each consumer uses a different subset of this module.
#![allow(dead_code)]

use sigil_isa::m68k::{encode, family_name, Cond, Instruction, Mnemonic, Operand, Size, Xn};
use sigil_isa::m68k_decode::{canonicalize, decode_one};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};

/// Padding length the Python side uses; both sides must feed capstone and
/// `decode_one` the same buffer or a length comparison is meaningless.
pub const PAD_LEN: usize = 14;

// ── the oracle process ──────────────────────────────────────────────────────

/// One capstone verdict for one input buffer.
#[derive(Debug, Clone)]
pub enum Cap {
    /// Capstone says this is not an MC68000 instruction.
    Reject,
    Ok { len: usize, mnem: String, ops: String },
}

/// Path to the dump helper. Walked up from the consuming crate's manifest
/// directory rather than joined at a fixed depth, so the module works from any
/// crate in the workspace and never depends on the process working directory.
fn helper() -> Option<std::path::PathBuf> {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|d| d.join("scripts/capstone_m68k_dump.py"))
        .find(|p| p.exists())
}

/// Run the helper and parse its TSV. `Err` carries a human-readable reason the
/// oracle could not be consulted; the caller decides skip-vs-panic.
fn run_capstone(
    mode: &str,
    extra: &[String],
    stdin_lines: Option<String>,
) -> Result<Vec<(String, Cap)>, String> {
    let Some(path) = helper() else {
        return Err("dump helper scripts/capstone_m68k_dump.py not found".into());
    };
    let mut cmd = Command::new("python3");
    cmd.arg(&path).arg(mode).args(extra).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if stdin_lines.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = cmd.spawn().map_err(|e| format!("cannot spawn python3: {e}"))?;
    if let Some(text) = stdin_lines {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "capstone dump failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
    let mut recs = Vec::new();
    let mut banner = None;
    for line in text.lines() {
        if let Some(b) = line.strip_prefix('#') {
            banner = Some(b.to_string());
            continue;
        }
        let mut f = line.split('\t');
        let key = f.next().unwrap_or_default().to_string();
        match f.next() {
            Some("reject") => recs.push((key, Cap::Reject)),
            Some("ok") => {
                let len: usize = f.next().unwrap_or("0").parse().map_err(|_| "bad len")?;
                let mnem = f.next().unwrap_or_default().to_string();
                let ops = f.next().unwrap_or_default().to_string();
                recs.push((key, Cap::Ok { len, mnem, ops }));
            }
            other => return Err(format!("unparsable dump line {line:?} (verb {other:?})")),
        }
    }
    if banner.is_none() {
        return Err("dump produced no capstone banner".into());
    }
    println!("capstone oracle: {}", banner.unwrap());
    Ok(recs)
}

/// The availability guard. `None` means the oracle is absent and the caller
/// must print `skip:` — which `SIGIL_STRICT_GATE=1` has already turned into a
/// panic before returning.
pub fn capstone_or_skip(
    mode: &str,
    extra: &[String],
    stdin_lines: Option<String>,
) -> Option<Vec<(String, Cap)>> {
    match run_capstone(mode, extra, stdin_lines) {
        Ok(v) => Some(v),
        Err(why) => {
            assert!(
                std::env::var("SIGIL_STRICT_GATE").is_err(),
                "SIGIL_STRICT_GATE set but the capstone oracle is unavailable: {why}"
            );
            eprintln!("skip: capstone oracle unavailable ({why})");
            println!("skip: capstone oracle unavailable ({why})");
            None
        }
    }
}

// ── the canonical form both sides normalise into ────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct Canon {
    family: String,
    /// `None` = the side does not report a size for this form.
    size: Option<char>,
    ops: Vec<String>,
}

fn cc_name(c: Cond) -> &'static str {
    match c {
        Cond::T => "t", Cond::F => "f", Cond::Hi => "hi", Cond::Ls => "ls",
        Cond::Cc => "cc", Cond::Cs => "cs", Cond::Ne => "ne", Cond::Eq => "eq",
        Cond::Vc => "vc", Cond::Vs => "vs", Cond::Pl => "pl", Cond::Mi => "mi",
        Cond::Ge => "ge", Cond::Lt => "lt", Cond::Gt => "gt", Cond::Le => "le",
    }
}

/// Sigil's mnemonic in capstone's naming: the condition-parameterised families
/// spell their condition into the name, and the SR/CCR forms are plain
/// `move`/`andi`/`ori` there (the distinguishing operand carries it).
///
/// Derived from the `Mnemonic` enum by an exhaustive match on the
/// condition-bearing variants plus `family_name` for the rest, so a new family
/// inherits `family_name`'s compiler-forced labeling.
fn sigil_family(m: Mnemonic) -> String {
    match m {
        Mnemonic::Scc(c) => format!("s{}", cc_name(c)),
        Mnemonic::Bcc(c) => format!("b{}", cc_name(c)),
        Mnemonic::Dbcc(Cond::T) => "dbt".into(),
        Mnemonic::Dbcc(Cond::F) => "dbra".into(),
        Mnemonic::Dbcc(c) => format!("db{}", cc_name(c)),
        Mnemonic::MoveToSr | Mnemonic::MoveFromSr => "move".into(),
        Mnemonic::AndiCcr => "andi".into(),
        Mnemonic::OriCcr => "ori".into(),
        other => family_name(other).to_string(),
    }
}

fn size_char(s: Size) -> char {
    match s {
        Size::B => 'b',
        Size::W => 'w',
        Size::L => 'l',
        // The 8-bit branch displacement; capstone spells that form `.b`.
        Size::S => 'b',
    }
}

fn width_mask(s: Size) -> u32 {
    match s {
        Size::B => 0xFF,
        Size::W => 0xFFFF,
        Size::L | Size::S => 0xFFFF_FFFF,
    }
}

fn xn_str(xn: &Xn, long: bool) -> String {
    let w = if long { 'l' } else { 'w' };
    match xn {
        Xn::D(n) => format!("d{n}.{w}"),
        Xn::A(n) => format!("a{n}.{w}"),
    }
}

/// Byte offset of the extension word carrying `ops[idx]`'s displacement,
/// derived by perturbing that displacement and diffing the re-encode: the word
/// that moves IS the extension word. Capstone resolves PC-relative operands to
/// an absolute target, so without this offset there is nothing to compare
/// against.
///
/// `None` when the derivation cannot run (a re-encode that fails, or bytes that
/// do not move). The caller renders that as `pc:?`, which matches nothing — a
/// failed derivation surfaces as a disagreement, never as a pass.
fn pc_ext_offset(inst: &Instruction, idx: usize) -> Option<usize> {
    let mut alt = inst.clone();
    match &mut alt.ops[idx] {
        Operand::Pcd16(d) => *d ^= 0x5A5A_u16 as i16,
        Operand::Pcd8Xn { d, .. } => *d ^= 0x5A_u8 as i8,
        _ => return None,
    }
    let a = encode(inst).ok()?;
    let b = encode(&alt).ok()?;
    if a.len() != b.len() {
        return None;
    }
    let first = a.iter().zip(&b).position(|(x, y)| x != y)?;
    Some(first & !1)
}

/// Render one sigil operand into the shared canonical form. `inst` must already
/// be [`canonicalize`]d — its size is what immediates compare modulo.
fn sigil_op(inst: &Instruction, idx: usize) -> String {
    match &inst.ops[idx] {
        Operand::Dn(n) => format!("d{n}"),
        Operand::An(n) => format!("a{n}"),
        Operand::Ind(n) => format!("(a{n})"),
        Operand::PostInc(n) => format!("(a{n})+"),
        Operand::PreDec(n) => format!("-(a{n})"),
        Operand::Disp16An(d, n) => format!("{d}(a{n})"),
        Operand::Disp8AnXn { d, an, xn, long } => {
            format!("{d}(a{an},{})", xn_str(xn, *long))
        }
        Operand::Pcd8Xn { d, xn, long } => match pc_ext_offset(inst, idx) {
            Some(off) => format!(
                "pcx:{:08X}({})",
                (off as i64 + *d as i64) as u32,
                xn_str(xn, *long)
            ),
            None => format!("pcx:?({})", xn_str(xn, *long)),
        },
        Operand::AbsW(v) => format!("abs.w:{:04X}", *v as u16),
        Operand::AbsL(v) => format!("abs.l:{:08X}", *v as u32),
        Operand::Pcd16(d) => match pc_ext_offset(inst, idx) {
            Some(off) => format!("pc:{:08X}", (off as i64 + *d as i64) as u32),
            None => "pc:?".to_string(),
        },
        Operand::Imm(v) => format!("#{:08X}", (*v as u32) & imm_field_mask(inst.mnemonic, inst.size)),
        Operand::RegList(m) => format!("regs:{m:04X}"),
        // The opcode word always comes first, so a branch displacement is
        // measured from offset 2 with no derivation needed.
        Operand::Disp(d) => format!("br:{:08X}", (2i64 + *d as i64) as u32),
        Operand::Ccr => "ccr".into(),
        Operand::Sr => "sr".into(),
    }
}

fn sigil_canon(inst: &Instruction) -> Canon {
    let c = canonicalize(inst);
    let ops = (0..c.ops.len()).map(|i| sigil_op(&c, i)).collect();
    Canon { family: sigil_family(c.mnemonic), size: Some(size_char(c.size)), ops }
}

// ── parsing capstone's rendering ────────────────────────────────────────────

/// Split an `op_str` on top-level `, ` — capstone puts a comma INSIDE the
/// indexed forms (`$20(a0, d0.w)`), so a naive split would shear them.
fn split_ops(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '(' => { depth += 1; cur.push(ch) }
            ')' => { depth -= 1; cur.push(ch) }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
                if chars.peek() == Some(&' ') { chars.next(); }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() { out.push(cur.trim().to_string()); }
    out
}

fn parse_hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s, 16).ok()
}

fn reg_num(s: &str) -> Option<(char, u8)> {
    let b = s.as_bytes();
    if b.len() == 2 && (b[0] == b'd' || b[0] == b'a') && b[1].is_ascii_digit() {
        Some((b[0] as char, b[1] - b'0'))
    } else {
        None
    }
}

/// Expand capstone's register-list spelling (`d0-d7/a0-a6`) into the canonical
/// bit0=D0..bit15=A7 mask sigil's `RegList` uses.
fn parse_reglist(s: &str) -> Option<u16> {
    let bit = |r: &str| -> Option<u32> {
        let (k, n) = reg_num(r)?;
        Some(if k == 'd' { n as u32 } else { 8 + n as u32 })
    };
    let mut mask = 0u16;
    for part in s.split('/') {
        if let Some((lo, hi)) = part.split_once('-') {
            let (a, b) = (bit(lo)?, bit(hi)?);
            if a > b { return None; }
            for i in a..=b { mask |= 1 << i; }
        } else {
            mask |= 1 << bit(part)?;
        }
    }
    Some(mask)
}

/// Normalise one capstone operand token. `mask` is the immediate's stored-field
/// mask taken from SIGIL's canonicalised instruction — the two sides must mask
/// identically or a `#-1` at `.w` would look like a disagreement.
///
/// Returns `None` for a token this parser does not model; the caller reports
/// that as an unparsed-token disagreement rather than silently passing.
fn cap_op(tok: &str, mask: u32) -> Option<String> {
    // Registers and the special registers.
    if let Some((k, n)) = reg_num(tok) {
        return Some(format!("{k}{n}"));
    }
    if tok == "sr" { return Some("sr".into()); }
    if tok == "ccr" { return Some("ccr".into()); }

    // Immediate.
    if let Some(rest) = tok.strip_prefix("#$") {
        return parse_hex(rest).map(|v| format!("#{:08X}", v & mask));
    }

    // Register list (contains `-` or `/`, or is a bare register handled above).
    if tok.contains('/') || (tok.contains('-') && !tok.starts_with('-')) {
        return parse_reglist(tok).map(|m| format!("regs:{m:04X}"));
    }

    // -(An)
    if let Some(inner) = tok.strip_prefix("-(").and_then(|t| t.strip_suffix(')')) {
        if let Some((k, n)) = reg_num(inner) {
            if k == 'a' { return Some(format!("-(a{n})")); }
        }
        return None;
    }

    // (An), (An)+, (An,Xn), (PC,Xn) — the no-displacement bracket forms.
    let (body, post_inc) = match tok.strip_suffix('+') {
        Some(t) => (t, true),
        None => (tok, false),
    };

    // Anything with a leading displacement: [-]$hex( … )
    let (disp, bracket) = if let Some(open) = body.find('(') {
        let head = &body[..open];
        let tail = &body[open..];
        if head.is_empty() {
            (None, tail)
        } else {
            let (neg, hx) = match head.strip_prefix('-') {
                Some(h) => (true, h),
                None => (false, head),
            };
            let v = parse_hex(hx.strip_prefix('$')?)? as i64;
            (Some(if neg { -v } else { v }), tail)
        }
    } else {
        // Absolute / branch target / bare value.
        if let Some(v) = body.strip_suffix(".w").and_then(|t| t.strip_prefix('$')).and_then(parse_hex) {
            return Some(format!("abs.w:{:04X}", v as u16));
        }
        if let Some(v) = body.strip_suffix(".l").and_then(|t| t.strip_prefix('$')).and_then(parse_hex) {
            return Some(format!("abs.l:{v:08X}"));
        }
        if let Some(v) = body.strip_prefix('$').and_then(parse_hex) {
            return Some(format!("br:{v:08X}"));
        }
        return None;
    };

    // Capstone is not consistent about the space after the inner comma
    // (`(a0, d0.w)` but `(pc,d0.w)`), so split on the comma and trim.
    let inner = bracket.strip_prefix('(')?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    match parts.as_slice() {
        // `(An)` / `(An)+` / `$d(An)`, and the base-less `$d(pc)`.
        ["pc"] => {
            if post_inc { return None; }
            Some(format!("pc:{:08X}", disp.unwrap_or(0) as u32))
        }
        [base] => {
            let (k, n) = reg_num(base)?;
            if k != 'a' { return None; }
            match (disp, post_inc) {
                (None, true) => Some(format!("(a{n})+")),
                (None, false) => Some(format!("(a{n})")),
                (Some(d), false) => Some(format!("{d}(a{n})")),
                (Some(_), true) => None,
            }
        }
        [base, index] => {
            if post_inc { return None; }
            let idx = index.to_string();
            let d = disp.unwrap_or(0);
            // Base 0 on the capstone side, so the printed value of a
            // PC-relative form IS the target offset, not a displacement.
            if *base == "pc" {
                Some(format!("pcx:{:08X}({idx})", d as u32))
            } else {
                let (k, n) = reg_num(base)?;
                if k != 'a' { return None; }
                Some(format!("{d}(a{n},{idx})"))
            }
        }
        _ => None,
    }
}

fn cap_canon(mnem: &str, ops: &str, inst: &Instruction) -> Option<Canon> {
    let (family, sz) = match mnem.split_once('.') {
        Some((f, s)) if s.len() == 1 => (f.to_string(), s.chars().next()),
        _ => (mnem.to_string(), None),
    };
    let c = canonicalize(inst);
    let mask = imm_field_mask(c.mnemonic, c.size);
    let mut list = Vec::new();
    for tok in split_ops(ops) {
        list.push(cap_op(&tok, mask)?);
    }
    Some(Canon { family, size: sz, ops: list })
}
// ── rendering equivalences (spelling, not semantics) ────────────────────────

/// Four places where capstone spells the SAME fact differently rather than
/// asserting a different one. These are folded into the canonical form before
/// comparison — they are not exclusions, because nothing is being excused: both
/// sides carry the same value and only the notation differs. Each rule is
/// written so it can only fire when the two values already agree.
///
/// 1. **Empty register list.** `movem` with mask `$0000` has no registers to
///    name, and capstone falls back to printing the raw mask word as an
///    immediate (`movem.w #$0, (a0)`). Rewritten to `regs:0000` ONLY when the
///    sigil side has a `RegList` in that same slot AND the numeric values
///    agree, so a genuine mask disagreement still fires.
/// 2. **Single-register list.** A mask with exactly one bit set prints as the
///    bare register (`movem.l a3, -(a7)`), which is indistinguishable by shape
///    from a register operand. Rewritten ONLY when sigil has a `RegList` in
///    that slot, that mask has exactly one bit set, and the bit is the one this
///    register name denotes.
/// 3. **`illegal`.** The instruction is the fixed word `$4AFC` with no
///    operands; capstone prints that word as an immediate operand
///    (`illegal #$4afc`). Dropped ONLY for the `illegal` family, and only when
///    the printed immediate IS `$4AFC`.
/// 4. **`moveq` immediate width.** The `moveq` data field is a stored SIGNED
///    BYTE that the processor sign-extends to long. Sigil's decoder reports the
///    sign-extended value (`Imm(-128)`), capstone prints the stored byte
///    (`#$80`). Both describe the same 8-bit field, so the comparison is over
///    that field — the same "immediates compare modulo the field width" rule
///    the existing round-trip relation (Rule I) uses. Applied on BOTH sides via
///    [`imm_field_mask`], never to one side only.
fn fold_rendering(sigil: &Canon, cap: &mut Canon) {
    if cap.family == "illegal" && sigil.ops.is_empty() && cap.ops == ["#00004AFC"] {
        cap.ops.clear();
    }
    for (i, s) in sigil.ops.iter().enumerate() {
        let Some(mask_hex) = s.strip_prefix("regs:") else { continue };
        let Some(mask) = u32::from_str_radix(mask_hex, 16).ok() else { continue };
        let Some(c) = cap.ops.get(i) else { continue };
        // Rule 1: the raw mask word printed as an immediate.
        if let Some(imm) = c.strip_prefix('#') {
            if u32::from_str_radix(imm, 16).ok() == Some(mask) {
                cap.ops[i] = s.clone();
            }
            continue;
        }
        // Rule 2: a one-bit mask printed as the bare register it names.
        if mask.count_ones() == 1 {
            if let Some((kind, n)) = reg_num(c) {
                let bit = if kind == 'd' { n as u32 } else { 8 + n as u32 };
                if mask.trailing_zeros() == bit {
                    cap.ops[i] = s.clone();
                }
            }
        }
    }
}

/// The width of the immediate's STORED field, which is what the two sides
/// compare modulo. Equal to the operation width except for `moveq`, whose data
/// is a signed byte the processor sign-extends to long.
fn imm_field_mask(m: Mnemonic, size: Size) -> u32 {
    match m {
        Mnemonic::Moveq => 0xFF,
        _ => width_mask(size),
    }
}

// ── named exclusions ────────────────────────────────────────────────────────

/// Everything one disagreement offers a predicate, so an exclusion can be
/// written against the actual values rather than against a word range.
struct Ctx<'a> {
    word: u16,
    kind: &'static str,
    inst: &'a Instruction,
    /// `None` when capstone rejected the word.
    cap: Option<&'a Cap>,
    sigil: &'a Canon,
    /// `None` when capstone rejected the word or its rendering did not parse.
    other: Option<&'a Canon>,
    /// Bytes sigil consumed.
    used: usize,
}

/// One named, derived carve-out: a place where the two disagree and the
/// MC68000 programmer's reference says CAPSTONE is the wrong side.
///
/// Two properties keep a carve-out from quietly becoming a hole. `matches` is
/// written against the values of the specific disagreement, not against a word
/// range, so it cannot swallow a different defect in the same word. And
/// `sweep_words` states, from the encoding, exactly how many opcode words the
/// class contains — the sweep asserts that count exactly, so the class can
/// neither grow nor vanish without failing the gate.
pub struct Exclusion {
    name: &'static str,
    derivation: &'static str,
    matches: fn(&Ctx) -> bool,
    sweep_words: usize,
}

/// `6xFF` — a branch whose 8-bit displacement field is `$FF`.
///
/// MC68000 (M68000 8-/16-/32-Bit Microprocessor User's Manual, BRA/BSR/Bcc):
/// the 8-bit displacement is used as written, and the ONLY escape is `$00`,
/// which selects the following 16-bit displacement word. The `$FF` escape to a
/// 32-bit displacement word is an MC68020 addition and does not exist on the
/// MC68000. So `6xFF` on an MC68000 is a plain 2-byte branch with displacement
/// −1 (a branch to an odd address, which faults at execution time — but the
/// INSTRUCTION is well-formed, which is what a decoder judges). Capstone,
/// including in `CS_MODE_M68K_000`, applies the 68020 escape and then refuses
/// the long form, so it answers `dc.w`.
///
/// Class size: the condition field (bits 11–8) is free and the displacement
/// byte is fixed, so the class is `0x60FF..=0x6FFF` step `0x0100` — **16
/// words**, covering `bra`, `bsr` and the 14 conditional branches.
fn excl_branch_ff(c: &Ctx) -> bool {
    c.kind == "legality" && (c.word & 0xF0FF) == 0x60FF
}

/// `083C` — `btst #<data>,#<data>`.
///
/// MC68000 PRM, BTST: the destination effective-address field takes the data
/// addressing modes, and the immediate and PC-relative rows carry the footnote
/// that they are valid **for BTST only** — BTST reads a bit and writes nothing,
/// so it does not need an alterable destination the way BCHG/BCLR/BSET do.
/// `asl` implements exactly that differentiated rule: it assembles
/// `btst #1,#$ff` to `083C 0001 00FF` and REJECTS the same destination for
/// `bclr` ("addressing mode not allowed here"). Sigil agrees with asl; capstone
/// omits the immediate row from its BTST destination table and answers `dc.w`.
///
/// Class size: the static form fixes every bit — `0000 1000 0011 1100` — so the
/// class is exactly **1 word**.
fn excl_btst_immediate_destination(c: &Ctx) -> bool {
    c.kind == "legality" && c.word == 0x083C
}

/// `btst`/`bset`/`bclr` operation size.
///
/// MC68000 PRM, BTST/BCHG/BCLR/BSET — "Operand Size: Byte, Long": long when the
/// destination is a data register, byte when it is a memory location. The size
/// is a function of the DESTINATION, which is what sigil's `canonical_size`
/// implements.
///
/// Capstone answers a different function entirely — byte for everything except
/// the DYNAMIC form of `btst`, where it answers long. So it is wrong in both
/// directions: `btst d0,$1234.l` is byte-sized on the MC68000 and capstone says
/// long; `bset #1,d0` is long-sized and capstone says byte.
///
/// The predicate reproduces capstone's rule exactly and excuses only the size
/// that rule produces. Any other answer — say `.w`, or `.l` on a `bset` — is
/// not this quirk and still fails.
///
/// Class size, derived from the encoder's destination rows rather than
/// measured. BTST's destination row is DATA: `Dn`(8) `(An)`(8) `(An)+`(8)
/// `-(An)`(8) `(d16,An)`(8) `(d8,An,Xn)`(8) `(xxx).W` `(xxx).L` `(d16,PC)`
/// `(d8,PC,Xn)` `#<data>` = 53 forms. BSET/BCLR take DATA_ALTERABLE — the same
/// list without the three read-only rows = 50 forms.
/// - `btst` dynamic: sigil long only for a `Dn` destination, capstone always
///   long ⇒ the 45 non-`Dn` destinations disagree — but one of them,
///   `#<data>`, disagrees on LENGTH first (capstone reads a long immediate; see
///   [`excl_btst_dynamic_immediate_length`]) and never reaches the size
///   comparison, leaving 44 × 8 source registers = **352**.
/// - `btst` static: capstone always byte ⇒ disagreement on the 8 `Dn`
///   destinations = **8**.
/// - `bset`/`bclr`, both forms: capstone always byte ⇒ disagreement on the 8
///   `Dn` destinations, × 8 source registers for the dynamic form:
///   2 × (64 + 8) = **144**.
///
/// Total **504**.
fn excl_bit_op_size(c: &Ctx) -> bool {
    if c.kind != "size" || !matches!(c.inst.mnemonic, Mnemonic::Btst | Mnemonic::Bset | Mnemonic::Bclr) {
        return false;
    }
    let Some(other) = c.other else { return false };
    // Capstone's rule, reproduced: long for a dynamic `btst`, byte otherwise.
    let capstone_rule = match c.inst.mnemonic {
        Mnemonic::Btst if matches!(c.inst.ops.first(), Some(Operand::Dn(_))) => 'l',
        _ => 'b',
    };
    other.size == Some(capstone_rule)
}

/// `btst Dn,#<data>` — the same inverted size rule, costing a byte COUNT.
///
/// Because capstone believes this form is long-sized (see
/// [`excl_bit_op_size`]), it reads the immediate as a long and consumes two
/// extension words where the MC68000's byte-sized form has one. sigil consumes
/// 4 bytes; capstone reports 6. `asl` assembles `btst d0,#$ff` to `013C 00FF` —
/// 4 bytes — which is sigil's answer.
///
/// The predicate demands the exact +2 over-read, so a different length
/// disagreement on the same word is not excused.
///
/// Class size: `0000 rrr 1 00 111 100` — bits 11–9 free (the source register),
/// every other bit fixed — **8 words**.
fn excl_btst_dynamic_immediate_length(c: &Ctx) -> bool {
    if c.kind != "length" || (c.word & 0xF1FF) != 0x013C {
        return false;
    }
    matches!(c.cap, Some(Cap::Ok { len, .. }) if *len == c.used + 2)
}

/// PC-relative target when an extension word PRECEDES the displacement.
///
/// MC68000 PRM §2.6/§2.7: for `(d16,PC)` and `(d8,PC,Xn)` the PC value is "the
/// address of the extension word" — the displacement's OWN extension word, not
/// the first one. `asl` confirms it on both shapes: `btst #1,target(pc)` with
/// `target` at 6 assembles to `083A 0001 0002` (bit-number word at 2,
/// displacement word at 4, displacement 2 → 4+2 = 6), and
/// `movem.w target(pc),d0` with `target` at 8 assembles to
/// `4CBA 0001 0002` (mask word at 2, displacement word at 4, 4+2 = 6 … the
/// `dc.w` follows at 8 — the same base). Capstone resolves the target from the
/// address of the FIRST extension word, so whenever an extension word precedes
/// the displacement its answer is short by exactly that many bytes.
///
/// The predicate demands precisely that shortfall in precisely one operand
/// slot: same family, same size, same operand count, every other operand
/// identical, and the differing slot a PC-relative form on both sides whose
/// capstone target is sigil's minus `(ext_off − 2)`. Anything else about those
/// words still fails.
///
/// Class size: an extension word precedes a PC-relative displacement in exactly
/// two of sigil's emitted shapes, and PC-relative is read-only so only
/// source/tested operands can carry it.
/// - the STATIC bit ops put the bit-number word first, and their destination
///   row admits PC-relative for BTST only: `083A` (`(d16,PC)`) and `083B`
///   (`(d8,PC,Xn)`) — 2 words;
/// - `movem <ea>,reglist` puts the mask word first: `4CBA`/`4CBB` at `.w` and
///   `4CFA`/`4CFB` at `.l` — 4 words.
///
/// **6 words** total.
fn excl_pc_base_after_extension(c: &Ctx) -> bool {
    if c.kind != "operands" {
        return false;
    }
    let Some(other) = c.other else { return false };
    if other.family != c.sigil.family || other.ops.len() != c.sigil.ops.len() {
        return false;
    }
    let differing: Vec<usize> = (0..c.sigil.ops.len())
        .filter(|i| c.sigil.ops[*i] != other.ops[*i])
        .collect();
    let [i] = differing[..] else { return false };
    let (Some(s), Some(o)) = (pc_target(&c.sigil.ops[i]), pc_target(&other.ops[i])) else {
        return false;
    };
    // The shortfall must equal the bytes of extension word that precede the
    // displacement, and there must BE some — a zero shortfall is agreement.
    let Some(off) = pc_ext_offset(c.inst, i) else { return false };
    off > 2 && s.1 == o.1 && s.0.wrapping_sub(o.0) == (off as u32 - 2)
}

/// `(target, index-register suffix)` of a canonical PC-relative operand, or
/// `None` when the operand is not PC-relative.
fn pc_target(op: &str) -> Option<(u32, String)> {
    if let Some(rest) = op.strip_prefix("pc:") {
        return u32::from_str_radix(rest, 16).ok().map(|v| (v, String::new()));
    }
    let rest = op.strip_prefix("pcx:")?;
    let (hex, idx) = rest.split_once('(')?;
    u32::from_str_radix(hex, 16).ok().map(|v| (v, idx.to_string()))
}

pub fn exclusions() -> Vec<Exclusion> {
    vec![
        Exclusion {
            name: "branch-ff",
            derivation: "the $FF long-displacement escape is an MC68020 addition; capstone applies it in its 000 mode",
            matches: excl_branch_ff,
            sweep_words: 16,
        },
        Exclusion {
            name: "btst-immediate-destination",
            derivation: "MC68000 PRM allows the immediate destination row for BTST only; asl agrees, capstone omits it",
            matches: excl_btst_immediate_destination,
            sweep_words: 1,
        },
        Exclusion {
            name: "bit-op-size",
            derivation: "MC68000 PRM sizes btst/bset/bclr by the DESTINATION; capstone answers byte except for a dynamic btst",
            matches: excl_bit_op_size,
            sweep_words: 504,
        },
        Exclusion {
            name: "btst-dynamic-immediate-length",
            derivation: "capstone's inverted bit-op size makes it read a long immediate where the byte form has one word",
            matches: excl_btst_dynamic_immediate_length,
            sweep_words: 8,
        },
        Exclusion {
            name: "pc-base-after-extension",
            derivation: "the PC base is the displacement's OWN extension word (asl-confirmed); capstone uses the first one",
            matches: excl_pc_base_after_extension,
            sweep_words: 6,
        },
    ]
}

// ── the disagreement record ─────────────────────────────────────────────────

#[derive(Debug)]
pub struct Disagreement {
    key: String,
    kind: &'static str,
    detail: String,
    /// Sigil family, for grouping the inventory.
    family: String,
}

/// Compare one buffer against capstone's verdict for it. `Ok(())` = agreement
/// or a named exclusion; `Err` = a disagreement to report.
pub fn compare(
    key: &str,
    word: u16,
    buf: &[u8],
    cap: &Cap,
    excl: &[Exclusion],
    hits: &mut [usize],
) -> Option<Disagreement> {
    let (inst, used) = decode_one(buf).ok()?;
    let sigil = sigil_canon(&inst);

    // Build capstone's canonical form up front so an exclusion can see it.
    let parsed = match cap {
        Cap::Reject => None,
        Cap::Ok { mnem, ops, .. } => cap_canon(mnem, ops, &inst).map(|mut c| {
            fold_rendering(&sigil, &mut c);
            c
        }),
    };

    let mut fire = |kind: &'static str, detail: String| -> Option<Disagreement> {
        let ctx = Ctx {
            word,
            kind,
            inst: &inst,
            cap: Some(cap),
            sigil: &sigil,
            other: parsed.as_ref(),
            used,
        };
        for (i, e) in excl.iter().enumerate() {
            if (e.matches)(&ctx) {
                hits[i] += 1;
                return None;
            }
        }
        Some(Disagreement { key: key.to_string(), kind, detail, family: sigil.family.clone() })
    };

    match cap {
        Cap::Reject => fire(
            "legality",
            format!(
                "sigil decodes {inst:?} from {:02X?} but capstone rejects the word",
                &buf[..used]
            ),
        ),
        Cap::Ok { len, mnem, ops } => {
            if *len != used {
                return fire(
                    "length",
                    format!("sigil {used} bytes, capstone {len} bytes (`{mnem} {ops}`) for {inst:?}"),
                );
            }
            let Some(other) = parsed.as_ref() else {
                return fire(
                    "unparsed",
                    format!("cannot normalise capstone `{mnem} {ops}` (sigil {inst:?})"),
                );
            };
            if other.family != sigil.family {
                return fire(
                    "family",
                    format!(
                        "sigil `{}` vs capstone `{}` ({inst:?} / `{mnem} {ops}`)",
                        sigil.family, other.family
                    ),
                );
            }
            if other.ops != sigil.ops {
                return fire(
                    "operands",
                    format!(
                        "sigil {:?} vs capstone {:?} ({inst:?} / `{mnem} {ops}`)",
                        sigil.ops, other.ops
                    ),
                );
            }
            if let (Some(a), Some(b)) = (sigil.size, other.size) {
                if a != b {
                    return fire(
                        "size",
                        format!("sigil `.{a}` vs capstone `.{b}` ({inst:?} / `{mnem} {ops}`)"),
                    );
                }
            }
            None
        }
    }
}

/// How hard a run holds the exclusions to their stated class sizes.
///
/// The distinction is not laxity, it is what each corpus can prove. Over the
/// zero-padded 65,536-word space every class size is a fixed property of the
/// encoding and is asserted exactly. Over a corpus that is a SUBSET of that
/// space — a nonzero-padded sweep, whose padding makes some words undecodable,
/// or the shapes' emitted stream, which contains whatever aeon happens to
/// compile — a class can legitimately be empty, and asserting a count there
/// would pin a measurement rather than a derivation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Accounting {
    /// Every exclusion must match its derived class size EXACTLY.
    ExactClassSizes,
    /// Print the hit counts; assert nothing about them.
    CountsAreInformational,
}

pub fn report(
    excl: &[Exclusion],
    hits: &[usize],
    bad: &[Disagreement],
    what: &str,
    mode: Accounting,
) {
    for (e, h) in excl.iter().zip(hits) {
        println!("  excluded `{}`: {h} hit(s) — {}", e.name, e.derivation);
    }

    if std::env::var("SIGIL_CAPSTONE_INVENTORY").is_ok() {
        let mut groups: BTreeMap<(&str, String), Vec<&Disagreement>> = BTreeMap::new();
        for d in bad {
            groups.entry((d.kind, d.family.clone())).or_default().push(d);
        }
        println!("--- inventory: {} disagreement(s) in {} class(es)", bad.len(), groups.len());
        for ((kind, fam), v) in &groups {
            println!("  [{kind}/{fam}] {} case(s); first 4:", v.len());
            for d in v.iter().take(4) {
                println!("      {}: {}", d.key, d.detail);
            }
        }
    }

    if mode == Accounting::ExactClassSizes {
        for (e, h) in excl.iter().zip(hits) {
            assert!(
                *h > 0,
                "exclusion `{}` matched nothing in the {what} — a carve-out that no longer \
                 covers anything is a hole waiting to widen; delete it",
                e.name
            );
            if e.sweep_words > 0 {
                assert_eq!(
                    *h, e.sweep_words,
                    "exclusion `{}` covered {h} words but its derivation says the class is {} \
                     words — the class moved and the derivation no longer describes it",
                    e.name, e.sweep_words
                );
            }
        }
    }

    if !bad.is_empty() {
        let shown: Vec<String> = bad
            .iter()
            .take(25)
            .map(|d| format!("  [{}] {}: {}", d.kind, d.key, d.detail))
            .collect();
        panic!(
            "{} capstone disagreement(s) in the {what} (first {} shown; \
             set SIGIL_CAPSTONE_INVENTORY=1 for the full grouped inventory):\n{}",
            bad.len(),
            shown.len(),
            shown.join("\n")
        );
    }
}
