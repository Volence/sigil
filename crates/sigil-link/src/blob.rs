//! Placing a second address space into the ROM image the way `p2bin -z` does.
//!
//! A community disassembly assembles its Z80 sound driver at Z80 address 0 and
//! leaves it to a post-processor, `p2bin`, to compress it and put it in the gap
//! the 68000 code leaves for it. The instruction lives in the build script, in
//! p2bin's grammar: `-z=<address>,<format>,<constant>,<insert>` and `-p=<pad>`.
//! The AS route takes that instruction as written, so a build swaps one tool for
//! the other with no edit to the disassembly.
//!
//! Every rule below was measured by running the p2bin binary (md5
//! `4f2fff99c3347bafb93b12d5be1db754`) on probes, not read from its source; the
//! measurements are in `docs/superpowers/notes/2026-09-11-s1-driver-stage2.md`.
//!
//! p2bin reads asl's object file, a list of RECORDS (a CPU, a load address, the
//! bytes) in the order asl wrote them. This module derives the same list from
//! the linked sections: a [`Run`] is a stretch of bytes one section writes at
//! consecutive addresses, in program order. Then, as p2bin does:
//!
//! * the blob is the first Z80 run starting at `<address>`, joined by each run
//!   after it that is Z80 and starts where the blob so far ends;
//! * `after` stores it where the run before the blob ends, and reserves the gap
//!   up to the run after the blob (unbounded when nothing follows);
//! * `before` stores it where the run before the blob starts, over that run's
//!   own bytes, and reserves that run's length;
//! * a blob over 0x2000 bytes, or a stored stream larger than its reservation,
//!   is refused;
//! * every address no run writes holds the pad byte.
//!
//! `<constant>` is a name and nothing else: p2bin never reads its value, it
//! only names it in the overflow message, and so does this module.
//!
//! On top of p2bin, this refuses what p2bin does silently: an instruction that
//! names no second address space, a second address space left partly or wholly
//! unplaced, a stored stream some other run would overwrite, and a stream that
//! does not decompress, through sigil's own decompressor, to exactly the bytes
//! that were assembled.

use crate::LinkedImage;
use sigil_ir::map::MemoryMap;
use sigil_ir::{AddressSpace, Cpu, Fragment, Section};
use sigil_span::{Diagnostic, Level, Span};

/// Every format p2bin's `-z` names, in the order its help text lists them.
pub const P2BIN_FORMATS: [&str; 7] = [
    "uncompressed",
    "kosinski",
    "kosinski-optimised",
    "saxman",
    "saxman-bugged",
    "saxman-optimised",
    "kosinskiplus",
];

/// The largest blob p2bin accepts, in uncompressed bytes: 0x2000 is placed and
/// 0x2001 refused, whatever the format.
pub const MAX_BLOB: usize = 0x2000;

/// The formats this module implements: the ones the Sonic 1, Sonic 2 and Sonic 3
/// & Knuckles build scripts select.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobFormat {
    /// `uncompressed`: the bytes as assembled.
    Uncompressed,
    /// `kosinski`: Sega's own Kosinski compressor, "Kosinski (authentic)".
    Kosinski,
    /// `saxman-bugged`: Sega's own Saxman compressor, then one junk byte.
    SaxmanBugged,
}

impl BlobFormat {
    /// Every implemented format.
    pub const ALL: [BlobFormat; 3] = [BlobFormat::Uncompressed, BlobFormat::Kosinski, BlobFormat::SaxmanBugged];

    /// The name p2bin gives the format.
    pub fn name(self) -> &'static str {
        match self {
            BlobFormat::Uncompressed => "uncompressed",
            BlobFormat::Kosinski => "kosinski",
            BlobFormat::SaxmanBugged => "saxman-bugged",
        }
    }

    fn from_name(name: &str) -> Option<BlobFormat> {
        BlobFormat::ALL.into_iter().find(|f| f.name() == name)
    }

    /// The bytes p2bin stores for `data`.
    pub fn compress(self, data: &[u8]) -> Vec<u8> {
        match self {
            BlobFormat::Uncompressed => data.to_vec(),
            BlobFormat::Kosinski => sigil_clownlzss_sys::accurate::compress_kosinski_authentic(data),
            BlobFormat::SaxmanBugged => sigil_clownlzss_sys::accurate::compress_saxman_bugged(data),
        }
    }

    /// What a reader of the stored bytes gets back, through sigil's own
    /// decompressors. Saxman is read as Sonic 2's decompressor reads it: the
    /// stored length counts the junk byte, which is never used, and a match
    /// whose source starts before the output begins is zeros throughout.
    pub fn decompress(self, stored: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            BlobFormat::Uncompressed => Ok(stored.to_vec()),
            BlobFormat::Kosinski => sigil_clownlzss_sys::decompress_kosinski(stored).map_err(|e| e.to_string()),
            BlobFormat::SaxmanBugged => {
                let used = stored.len().saturating_sub(1);
                sigil_clownlzss_sys::decompress_saxman_no_header(stored, used).map_err(|e| e.to_string())
            }
        }
    }
}

/// Where p2bin puts the stored stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobInsert {
    /// Over the start of the run before the blob, which reserves the space.
    Before,
    /// Where the run before the blob ends, in the gap up to the run after it.
    After,
}

/// One `-z` instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobInstruction {
    /// The argument as given, for messages.
    pub text: String,
    /// The Z80 address the blob starts at.
    pub address: u32,
    pub format: BlobFormat,
    /// The name the overflow message tells the reader to raise.
    pub constant: String,
    pub insert: BlobInsert,
}

/// A number as p2bin's `sscanf("%X")` reads it: leading white space, an
/// optional sign, an optional `0x`, then hexadecimal digits up to the first
/// character that is not one. Returns the sign and the saturated magnitude, or
/// `None` when no digit is found.
fn scan_hex(text: &str) -> Option<(bool, u64)> {
    let mut s = text.trim_start_matches([' ', '\t', '\n', '\x0B', '\x0C', '\r']);
    let negative = s.starts_with('-');
    if negative || s.starts_with('+') {
        s = &s[1..];
    }
    if s.len() >= 2 && (s.starts_with("0x") || s.starts_with("0X")) {
        s = &s[2..];
        if !s.starts_with(|c: char| c.is_ascii_hexdigit()) {
            return None;
        }
    }
    let digits: &str = &s[..s.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(s.len())];
    if digits.is_empty() {
        return None;
    }
    let mut value: u64 = 0;
    for c in digits.chars() {
        value = value.saturating_mul(16).saturating_add(u64::from(c.to_digit(16).unwrap_or(0)));
    }
    Some((negative, value))
}

/// Parse a `-p=<pad>` argument. p2bin prints its complaint and goes on with the
/// default pad; this refuses instead, so a typo cannot build a ROM.
pub fn parse_pad(arg: &str) -> Result<u8, String> {
    let Some(value) = arg.strip_prefix("-p=") else {
        return Err(format!("`{arg}`: the pad byte is written `-p=<hex>`, as p2bin spells it, for example `-p=FF`"));
    };
    match scan_hex(value) {
        Some((negative, v)) if !negative || v == 0 => match u8::try_from(v) {
            Ok(b) => Ok(b),
            Err(_) => Err(format!("`{arg}`: the pad byte must be FF or lower")),
        },
        Some(_) => Err(format!("`{arg}`: the pad byte must be FF or lower")),
        None => Err(format!("`{arg}`: `{value}` is not a hexadecimal number")),
    }
}

/// Parse a `-z=<address>,<format>,<constant>,<insert>` argument. The fields are
/// split at the first three commas, so `<insert>` is the rest of the argument;
/// `<address>` is hexadecimal as p2bin reads it; `<constant>` may be empty.
pub fn parse_blob(arg: &str) -> Result<BlobInstruction, String> {
    let shape = "a blob instruction is written `-z=<address>,<format>,<constant>,<before|after>`, as p2bin spells it";
    let Some(body) = arg.strip_prefix("-z=") else {
        return Err(format!("`{arg}`: {shape}"));
    };
    let mut fields = body.splitn(4, ',');
    let (Some(address), Some(format), Some(constant), Some(insert)) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(format!("`{arg}`: {shape}"));
    };
    let address = match scan_hex(address) {
        Some((negative, v)) if !negative || v == 0 => u32::try_from(v)
            .map_err(|_| format!("`{arg}`: the address {address} is beyond the Z80's address space"))?,
        Some(_) => return Err(format!("`{arg}`: the address {address} is negative")),
        None => return Err(format!("`{arg}`: `{address}` is not a hexadecimal address; {shape}")),
    };
    let implemented = BlobFormat::ALL.map(BlobFormat::name).join(", ");
    let format = match BlobFormat::from_name(format) {
        Some(f) => f,
        None if P2BIN_FORMATS.contains(&format) => {
            return Err(format!(
                "`{arg}`: `{format}` is a p2bin format sigil does not implement; sigil implements {implemented}"
            ))
        }
        None => {
            return Err(format!(
                "`{arg}`: `{format}` is not a p2bin format; p2bin's are {}, and sigil implements {implemented}",
                P2BIN_FORMATS.join(", ")
            ))
        }
    };
    let insert = match insert {
        "before" => BlobInsert::Before,
        "after" => BlobInsert::After,
        other => return Err(format!("`{arg}`: the last field is `before` or `after`, not `{other}`")),
    };
    Ok(BlobInstruction { text: arg.to_string(), address, format, constant: constant.to_string(), insert })
}

/// A stretch of bytes one section writes at consecutive addresses: the unit
/// p2bin calls a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    /// Index of the section in program order.
    sec: usize,
    /// Offset of the run's first byte in the section's image bytes.
    offset: u32,
    /// Load address of the run's first byte.
    start: u32,
    /// Load address one past its last byte.
    end: u32,
    cpu: Cpu,
    space: AddressSpace,
}

/// Every run of `sections`, in program order. A reservation or a seek ends a
/// run; the next byte starts another, as the next record in asl's object file.
fn runs(sections: &[Section]) -> Vec<Run> {
    let mut out = Vec::new();
    for (si, sec) in sections.iter().enumerate() {
        let mut cursor: u32 = 0;
        let mut open: Option<Run> = None;
        for frag in &sec.fragments {
            let len = match frag {
                Fragment::Data(d) => d.bytes.len() as u32,
                Fragment::Fill { count, .. } => *count,
                Fragment::Reserve { count, .. } => {
                    cursor += *count;
                    continue;
                }
                Fragment::Org { target, .. } => {
                    cursor = *target;
                    continue;
                }
                // `resolve_layout` lowers these before anything is linked.
                Fragment::JmpJsrSym { .. } | Fragment::RelaxAbsSym { .. } | Fragment::RelaxLadder { .. } => 0,
            };
            if len == 0 {
                continue;
            }
            match open.as_mut() {
                Some(run) if run.offset + (run.end - run.start) == cursor => run.end += len,
                _ => {
                    if let Some(run) = open.take() {
                        out.push(run);
                    }
                    open = Some(Run {
                        sec: si,
                        offset: cursor,
                        start: sec.lma + cursor,
                        end: sec.lma + cursor + len,
                        cpu: sec.cpu,
                        space: sec.space,
                    });
                }
            }
            cursor += len;
        }
        if let Some(run) = open {
            out.push(run);
        }
    }
    out
}

/// One stored stream and where it goes.
struct Placement<'a> {
    instruction: &'a BlobInstruction,
    /// Index of the blob's first run, which is where p2bin writes it in file order.
    first_run: usize,
    at: u32,
    stored: Vec<u8>,
    span: Span,
}

/// A span no source map resolves, for a refusal about the command line itself.
const UNLOCATED: Span = Span { source: sigil_span::SourceId(u32::MAX), start: 0, end: 0 };

fn err(message: String, primary: Span) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary }
}

fn space_span(space: AddressSpace) -> Span {
    match space {
        AddressSpace::Foreign { entered_at, .. } => entered_at,
        AddressSpace::Image => UNLOCATED,
    }
}

/// Materialize the image as p2bin would from the same program: every image run
/// at its load address, each `-z` blob stored where its instruction says, and
/// `pad` everywhere nothing is written.
///
/// `resolved` is `resolve_layout`'s output and `linked` is `link`'s image of it,
/// one linked section per resolved section in the same order.
pub fn flatten_placing(
    resolved: &[Section],
    linked: &LinkedImage,
    blobs: &[BlobInstruction],
    pad: u8,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    assert_eq!(resolved.len(), linked.sections.len(), "one linked section per resolved section");
    let runs = runs(resolved);
    let bytes_of = |r: &Run| -> &[u8] {
        let b = &linked.sections[r.sec].bytes;
        &b[r.offset as usize..(r.offset + (r.end - r.start)) as usize]
    };
    let mut diags = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    let mut claimed = vec![false; runs.len()];

    // One instruction per address: p2bin gives no rule for two, so two are refused.
    let mut instructions: Vec<&BlobInstruction> = Vec::new();
    for z in blobs {
        match instructions.iter().find(|p| p.address == z.address) {
            Some(prior) => diags.push(err(
                format!("`{}` and `{}` both name the blob at Z80 address {:#X}", prior.text, z.text, z.address),
                UNLOCATED,
            )),
            None => instructions.push(z),
        }
    }
    let mut used = vec![false; instructions.len()];

    // In program order, as p2bin reads records: every Z80 run outside the image
    // that starts at an instruction's address begins a blob, however many there are.
    let mut next = 0;
    while next < runs.len() {
        let k = next;
        next += 1;
        let r = runs[k];
        if claimed[k] || r.cpu != Cpu::Z80 || !matches!(r.space, AddressSpace::Foreign { .. }) {
            continue;
        }
        let Some(zi) = instructions.iter().position(|z| z.address == r.start) else { continue };
        used[zi] = true;
        let z = instructions[zi];
        let space = runs[k].space;
        let span = space_span(space);
        let mut j = k;
        let mut blob: Vec<u8> = Vec::new();
        let mut end = z.address;
        let mut refused = false;
        while j < runs.len() && runs[j].cpu == Cpu::Z80 && runs[j].start == end {
            if runs[j].space != space {
                diags.push(err(
                    format!(
                        "`{}`: the Z80 code at [{:#X}, {:#X}) starts where the blob ends, so p2bin would join it to the blob, but it is not in the blob's address space",
                        z.text, runs[j].start, runs[j].end
                    ),
                    span,
                ));
                refused = true;
                break;
            }
            blob.extend_from_slice(bytes_of(&runs[j]));
            end = runs[j].end;
            claimed[j] = true;
            j += 1;
        }
        if refused {
            continue;
        }
        if let Some(rest) = runs.iter().enumerate().find(|(idx, r)| r.space == space && !(k..j).contains(idx)) {
            diags.push(err(
                format!(
                    "`{}`: the blob is the Z80 code at [{:#X}, {:#X}), and its address space also holds [{:#X}, {:#X}), which does not continue it; p2bin would compress only the first and write the rest at its Z80 address",
                    z.text, z.address, end, rest.1.start, rest.1.end
                ),
                span,
            ));
            continue;
        }
        if blob.len() > MAX_BLOB {
            diags.push(err(
                format!(
                    "`{}`: the blob [{:#X}, {:#X}) is {:#X} bytes, more than the {MAX_BLOB:#X} p2bin accepts",
                    z.text,
                    z.address,
                    end,
                    blob.len()
                ),
                span,
            ));
            continue;
        }
        // The record before the blob, or an empty one at 0 when there is none.
        let (prev_start, prev_end) = match k.checked_sub(1).map(|p| runs[p]) {
            Some(p) if p.space != AddressSpace::Image => {
                diags.push(err(
                    format!(
                        "`{}`: the code written just before the blob, at [{:#X}, {:#X}), is itself outside the image, so the blob has no ROM address to be placed against",
                        z.text, p.start, p.end
                    ),
                    span,
                ));
                continue;
            }
            Some(p) => (p.start, p.end),
            None => (0, 0),
        };
        let (at, reserved, where_) = match z.insert {
            BlobInsert::After => {
                let next = runs.get(j).copied();
                if let Some(n) = next.filter(|n| n.space != AddressSpace::Image) {
                    diags.push(err(
                        format!(
                            "`{}`: the code written just after the blob, at [{:#X}, {:#X}), is itself outside the image, so the gap reserved for the blob has no end",
                            z.text, n.start, n.end
                        ),
                        span,
                    ));
                    continue;
                }
                let reserved = next.map(|n| i64::from(n.start) - i64::from(prev_end));
                let where_ = match next {
                    Some(n) => format!("the gap [{prev_end:#X}, {:#X}) up to the code after it", n.start),
                    None => format!("the space from {prev_end:#X}, with nothing after it"),
                };
                (prev_end, reserved, where_)
            }
            BlobInsert::Before => (
                prev_start,
                Some(i64::from(prev_end) - i64::from(prev_start)),
                format!("[{prev_start:#X}, {prev_end:#X}), the code before it, which it overwrites"),
            ),
        };
        let stored = z.format.compress(&blob);
        if let Some(reserved) = reserved.filter(|&r| (stored.len() as i64) > r) {
            diags.push(err(
                format!(
                    "`{}`: the blob [{:#X}, {:#X}) is {:#X} bytes {}, and only {:#X} are reserved for it, at {where_}; set `{}` to at least ${:X}",
                    z.text,
                    z.address,
                    end,
                    stored.len(),
                    match z.format {
                        BlobFormat::Uncompressed => "stored uncompressed".to_string(),
                        f => format!("compressed as {}", f.name()),
                    },
                    reserved.max(0),
                    z.constant,
                    stored.len()
                ),
                span,
            ));
            continue;
        }
        match z.format.decompress(&stored) {
            Ok(back) if back == blob => {}
            Ok(back) => {
                let first = back.iter().zip(&blob).position(|(a, b)| a != b).unwrap_or(back.len().min(blob.len()));
                diags.push(err(
                    format!(
                        "`{}`: the stored {} stream does not decompress to the blob that was assembled (sigil's decompressor returns {:#X} bytes against {:#X}, first difference at offset {:#X}), so the game would load a different driver",
                        z.text,
                        z.format.name(),
                        back.len(),
                        blob.len(),
                        first
                    ),
                    span,
                ));
                continue;
            }
            Err(e) => {
                diags.push(err(
                    format!("`{}`: the stored {} stream does not decompress: {e}", z.text, z.format.name()),
                    span,
                ));
                continue;
            }
        }
        if let Err(message) = MemoryMap::mega_drive().validate_section(&z.text, at, stored.len() as u32) {
            diags.push(err(message, span));
            continue;
        }
        placements.push(Placement { instruction: z, first_run: k, at, stored, span });
    }

    // An instruction that began no blob names nothing p2bin would place. p2bin
    // ignores it; this refuses it, since it is a build script's mistake.
    for (zi, z) in instructions.iter().enumerate() {
        if used[zi] {
            continue;
        }
        let origins: Vec<String> = runs
            .iter()
            .filter(|r| matches!(r.space, AddressSpace::Foreign { .. }))
            .map(|r| format!("{:#X}", r.start))
            .collect();
        diags.push(err(
            format!(
                "`{}` names no second address space: no Z80 code assembled outside the image starts at {:#X} (code outside the image starts at: {})",
                z.text,
                z.address,
                if origins.is_empty() { "none".to_string() } else { origins.join(", ") }
            ),
            UNLOCATED,
        ));
    }

    // Every second address space is placed whole, or refused.
    for (idx, r) in runs.iter().enumerate() {
        if matches!(r.space, AddressSpace::Foreign { .. }) && !claimed[idx] {
            let already = diags.iter().any(|d| d.primary == space_span(r.space) && d.primary != UNLOCATED);
            if !already {
                diags.push(err(
                    format!(
                        "the Z80 code at [{:#X}, {:#X}) is outside the ROM image and no -z instruction places it",
                        r.start, r.end
                    ),
                    space_span(r.space),
                ));
            }
        }
    }

    // A stored stream no other write may touch. `before` overwrites the run in
    // front of it by design, and nothing else.
    for p in &placements {
        let (lo, hi) = (p.at, p.at + p.stored.len() as u32);
        for (idx, r) in runs.iter().enumerate() {
            if r.space != AddressSpace::Image || r.start >= hi || lo >= r.end {
                continue;
            }
            let own = p.instruction.insert == BlobInsert::Before && idx + 1 == p.first_run;
            if !own {
                diags.push(err(
                    format!(
                        "`{}`: the stored stream at [{lo:#X}, {hi:#X}) overlaps section `{}` at [{:#X}, {:#X})",
                        p.instruction.text, resolved[r.sec].name, r.start, r.end
                    ),
                    p.span,
                ));
            }
        }
    }
    for (a, p) in placements.iter().enumerate() {
        for q in &placements[a + 1..] {
            if p.at < q.at + q.stored.len() as u32 && q.at < p.at + p.stored.len() as u32 {
                diags.push(err(
                    format!("`{}` and `{}` store their streams over each other", p.instruction.text, q.instruction.text),
                    q.span,
                ));
            }
        }
    }
    if !diags.is_empty() {
        return Err(diags);
    }

    let image_end = runs.iter().filter(|r| r.space == AddressSpace::Image).map(|r| r.end).max().unwrap_or(0);
    let blob_end = placements.iter().map(|p| p.at + p.stored.len() as u32).max().unwrap_or(0);
    let mut out = vec![pad; image_end.max(blob_end) as usize];
    // In file order, as p2bin writes: a `before` stream lands on the run it
    // follows, so it is written after that run.
    let mut next_placement = 0;
    let mut ordered: Vec<&Placement> = placements.iter().collect();
    ordered.sort_by_key(|p| p.first_run);
    for (idx, r) in runs.iter().enumerate() {
        while next_placement < ordered.len() && ordered[next_placement].first_run <= idx {
            let p = ordered[next_placement];
            out[p.at as usize..p.at as usize + p.stored.len()].copy_from_slice(&p.stored);
            next_placement += 1;
        }
        if r.space == AddressSpace::Image {
            out[r.start as usize..r.end as usize].copy_from_slice(bytes_of(r));
        }
    }
    for p in &ordered[next_placement..] {
        out[p.at as usize..p.at as usize + p.stored.len()].copy_from_slice(&p.stored);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `-p` grammar, row by row as the p2bin binary answered each argument
    /// (probe `probe_rules.py`, section "-p grammar"): the pad it used, or the
    /// complaint it printed. Where p2bin complained and went on, sigil refuses.
    #[test]
    fn the_pad_grammar_is_p2bins() {
        let accepted = [("-p=FF", 0xFF), ("-p=ff", 0xFF), ("-p=0xFF", 0xFF), ("-p=0XFF", 0xFF), ("-p=FFx", 0xFF), ("-p=+7", 7), ("-p=7F", 0x7F), ("-p=0", 0), ("-p=-0", 0)];
        for (arg, want) in accepted {
            assert_eq!(parse_pad(arg), Ok(want), "{arg}");
        }
        // "Could not parse '-p' argument's padding value."
        for arg in ["-p=$FF", "-p=", "-p=0x", "-p=g", "-pFF"] {
            assert!(parse_pad(arg).is_err(), "{arg} parsed");
        }
        // "'-p' argument's padding value is too high (must be 0xFF or lower)."
        for arg in ["-p=-1", "-p=100", "-p=1FF"] {
            let e = parse_pad(arg).expect_err(arg);
            assert!(e.contains("FF or lower"), "{arg}: {e}");
        }
    }

    /// The `-z` grammar, row by row as the p2bin binary answered each argument
    /// (probe sections "-z grammar" and "-z address forms").
    #[test]
    fn the_blob_grammar_is_p2bins() {
        for (addr, want) in [("10", 0x10), ("0x10", 0x10), ("010", 0x10), ("10j", 0x10), ("+10", 0x10), (" 10", 0x10), ("10 ", 0x10), ("1300", 0x1300), ("00", 0), ("-0", 0)] {
            let z = parse_blob(&format!("-z={addr},kosinski,Guess,after")).expect(addr);
            assert_eq!(z.address, want, "{addr}");
        }
        let z = parse_blob("-z=0,uncompressed,,before").expect("an empty constant is accepted");
        assert_eq!((z.format, z.constant.as_str(), z.insert), (BlobFormat::Uncompressed, "", BlobInsert::Before));
        let z = parse_blob("-z=0,saxman-bugged,Size_of_Snd_driver_guess,after").expect("s2");
        assert_eq!(z.format, BlobFormat::SaxmanBugged);
        // "Could not parse '-z' argument's options."
        for arg in ["-z=0,uncompressed,Guess", "-z=,uncompressed,Guess,after", "-z=x,uncompressed,Guess,after", "-z=0x,uncompressed,Guess,after", "-z0,uncompressed,Guess,after"] {
            assert!(parse_blob(arg).is_err(), "{arg} parsed");
        }
        // "Unrecognised compression format (...)", case and white space included.
        for arg in ["-z=0,Uncompressed,Guess,after", "-z=0,,Guess,after", "-z=0, uncompressed,Guess,after", "-z=0,lzma,Guess,after", "-z=0,uncompressedx,Guess,after"] {
            assert!(parse_blob(arg).expect_err(arg).contains("not a p2bin format"), "{arg}");
        }
        // "Unrecognised type (...)": the type is the rest of the argument.
        for arg in ["-z=0,uncompressed,Guess,After", "-z=0,uncompressed,Guess,after,x", "-z=0,uncompressed,Guess,sideways", "-z=0,uncompressed,Guess,afterx"] {
            assert!(parse_blob(arg).expect_err(arg).contains("`before` or `after`"), "{arg}");
        }
    }

    /// The formats p2bin has and sigil does not are refused by name, with the
    /// list sigil does implement, rather than guessed at.
    #[test]
    fn an_unimplemented_p2bin_format_is_refused_by_name() {
        for name in ["kosinski-optimised", "saxman", "saxman-optimised", "kosinskiplus"] {
            let e = parse_blob(&format!("-z=0,{name},Guess,after")).expect_err(name);
            assert!(
                e.contains(&format!("`{name}` is a p2bin format sigil does not implement"))
                    && e.contains("uncompressed, kosinski, saxman-bugged"),
                "{name}: {e}"
            );
        }
        for f in BlobFormat::ALL {
            assert!(P2BIN_FORMATS.contains(&f.name()), "{} is not a p2bin name", f.name());
        }
    }
}
