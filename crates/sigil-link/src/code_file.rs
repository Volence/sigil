//! Writing the linked program as asl's object file (`.p`), for the stock `p2bin`.
//!
//! When sigil stands in for `asl` in a community disassembly's build script, the
//! script hands the object file to `p2bin` (md5 `4f2fff99c3347bafb93b12d5be1db754`),
//! which places every record at its address, pads the gaps and stores each `-z`
//! blob. This module writes that file; it does not place or compress anything.
//!
//! The format, as asl 1.42 Bld 212 (md5 `61e672562465725a8c102288a7da9098`) writes
//! it and `p2bin` reads it: the magic `89 14`; then records, each `81`, the CPU
//! (`01` for the 68000, `51` for the Z80), the segment (`01`, code), the
//! granularity (`01`), the load address as a little-endian `u32`, the length as a
//! little-endian `u16`, and the bytes; then `00` and the creator text, which runs
//! to the end of the file.
//!
//! Two properties of the record list carry meaning to `p2bin`, and both were
//! measured by re-encoding asl's own object files for all three disassemblies
//! (`docs/superpowers/notes/2026-09-27-as-replacement-state.md`, section 3):
//!
//! * **File order.** `p2bin` finds a `-z` blob and the record it is placed
//!   against by walking the records in the order they were written, so the
//!   records keep program order. Sorting them by CPU or address changes the ROM.
//! * **The record just before each Z80 record.** `-z ...,after` stores the blob
//!   where that record ends, and `-z ...,before` stores it over that record, from
//!   its start, reserving its length. Coalescing that record with the one before
//!   it changes where a `before` blob lands.
//!
//! The records are therefore the runs [`crate::flatten_placing`] models as
//! `p2bin`'s records (a stretch of bytes one section writes at consecutive
//! addresses, in program order), which is the model that reproduces the stock
//! ROM of every disassembly build on sigil's direct route. A run longer than a
//! record can hold is split into consecutive records from its start, which is
//! the direction asl fills its own records in: asl starts a new record when the
//! next statement's bytes would take the current one past `0xFFFF`
//! (`s2disasm` and `skdisasm` show records of `0xFF00`, `0xFFFC`, `0xFFFE` and
//! `0xFFFF` bytes, each continued by the next).
//!
//! Splitting changes no address, and it changes nothing an `after` blob reads,
//! which is where the record before it ENDS. It can change what a `before` blob
//! reads when the run before the blob is longer than one record: asl's last
//! record there starts at a statement boundary sigil does not track, and this
//! writer's starts at a multiple of `0xFFFF` bytes into the run. That shape
//! stores the blob over code rather than over the filler every corpus puts
//! there, so it is a broken source under either tool; it is named here, and in
//! `docs/superpowers/notes/2026-09-27-as-dropin-asl-contract.md`, as the one
//! place the record boundaries are sigil's own.

use crate::blob::runs;
use crate::LinkedImage;
use sigil_ir::{Cpu, Section};

/// The two bytes every asl object file starts with.
pub const CODE_FILE_MAGIC: [u8; 2] = [0x89, 0x14];

/// The most bytes one record can hold: its length is a `u16`.
pub const MAX_RECORD: usize = 0xFFFF;

/// asl's header byte for a processor, as its object files carry it.
pub fn asl_cpu_id(cpu: Cpu) -> u8 {
    match cpu {
        Cpu::M68000 => 0x01,
        Cpu::Z80 => 0x51,
    }
}

/// One record of an object file: which processor the bytes are for, where they
/// load, and the bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeRecord {
    /// asl's processor byte, see [`asl_cpu_id`].
    pub cpu: u8,
    /// The load address of the first byte.
    pub start: u32,
    pub bytes: Vec<u8>,
}

/// The records of the linked program, in program order, split so that none
/// holds more than [`MAX_RECORD`] bytes.
///
/// `resolved` is `resolve_layout`'s output and `linked` is `link`'s image of it,
/// one linked section per resolved section in the same order, exactly as
/// [`crate::flatten_placing`] takes them.
pub fn code_file_records(resolved: &[Section], linked: &LinkedImage) -> Vec<CodeRecord> {
    assert_eq!(resolved.len(), linked.sections.len(), "one linked section per resolved section");
    let mut out = Vec::new();
    for r in runs(resolved) {
        let bytes = &linked.sections[r.sec].bytes[r.offset as usize..(r.offset + (r.end - r.start)) as usize];
        for (i, chunk) in bytes.chunks(MAX_RECORD).enumerate() {
            out.push(CodeRecord {
                cpu: asl_cpu_id(r.cpu),
                start: r.start + (i * MAX_RECORD) as u32,
                bytes: chunk.to_vec(),
            });
        }
    }
    out
}

/// The object file holding `records` in the order given, closed by `creator`.
///
/// Panics if a record holds more than [`MAX_RECORD`] bytes, which
/// [`code_file_records`] never produces.
pub fn encode_code_file(records: &[CodeRecord], creator: &str) -> Vec<u8> {
    let mut out = CODE_FILE_MAGIC.to_vec();
    for r in records {
        assert!(r.bytes.len() <= MAX_RECORD, "a record holds at most {MAX_RECORD:#X} bytes");
        out.extend_from_slice(&[0x81, r.cpu, 0x01, 0x01]);
        out.extend_from_slice(&r.start.to_le_bytes());
        out.extend_from_slice(&(r.bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&r.bytes);
    }
    out.push(0x00);
    out.extend_from_slice(creator.as_bytes());
    out
}

/// Read an object file back into its records and creator text.
///
/// Reads both record shapes asl writes: the `81` form above, and the older form
/// whose header byte is the processor itself, with segment and granularity 1.
/// An entry-point record (`80` and a `u32`) is skipped. Anything else, a segment
/// other than code or a granularity other than 1, is refused with the offset it
/// was found at, since this reader exists to check what a writer produced.
pub fn decode_code_file(bytes: &[u8]) -> Result<(Vec<CodeRecord>, String), String> {
    if bytes.get(..2) != Some(&CODE_FILE_MAGIC[..]) {
        return Err("not an asl object file: it does not start with 89 14".to_string());
    }
    let mut i = 2;
    let mut records = Vec::new();
    let take = |i: &mut usize, n: usize| -> Result<&[u8], String> {
        let s = bytes.get(*i..*i + n).ok_or_else(|| format!("truncated at offset {:#X}", *i))?;
        *i += n;
        Ok(s)
    };
    loop {
        let at = i;
        let header = *take(&mut i, 1)?.first().expect("one byte");
        let (cpu, seg, gran) = match header {
            0x00 => {
                let creator = String::from_utf8_lossy(&bytes[i..]).into_owned();
                return Ok((records, creator));
            }
            0x80 => {
                take(&mut i, 4)?;
                continue;
            }
            0x81 => {
                let h = take(&mut i, 3)?;
                (h[0], h[1], h[2])
            }
            cpu => (cpu, 1, 1),
        };
        if seg != 1 || gran != 1 {
            return Err(format!("record at offset {at:#X}: segment {seg} granularity {gran}, only code (1, 1) is read"));
        }
        let start = u32::from_le_bytes(take(&mut i, 4)?.try_into().expect("four bytes"));
        let len = u16::from_le_bytes(take(&mut i, 2)?.try_into().expect("two bytes")) as usize;
        records.push(CodeRecord { cpu, start, bytes: take(&mut i, len)?.to_vec() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_file_reads_back_record_for_record_in_order() {
        let records = vec![
            CodeRecord { cpu: 0x01, start: 0x10, bytes: vec![1, 2, 3] },
            CodeRecord { cpu: 0x51, start: 0, bytes: vec![0x3E, 0x01] },
            CodeRecord { cpu: 0x01, start: 0, bytes: vec![0xAA; MAX_RECORD] },
        ];
        let file = encode_code_file(&records, "sigil test");
        assert_eq!(&file[..6], &[0x89, 0x14, 0x81, 0x01, 0x01, 0x01]);
        assert_eq!(decode_code_file(&file), Ok((records, "sigil test".to_string())));
    }

    /// asl writes a 68000 record in the older shape too, its header byte the CPU
    /// itself, and an entry point as `80` and a `u32`; `p2bin` reads both.
    #[test]
    fn the_older_record_shape_and_an_entry_point_read() {
        let mut file = vec![0x89, 0x14, 0x01, 0x00, 0x02, 0x00, 0x00, 0x02, 0x00, 0x4E, 0x75];
        file.extend_from_slice(&[0x80, 0x00, 0x02, 0x00, 0x00]);
        file.extend_from_slice(b"\x00AS 1.42");
        let (records, creator) = decode_code_file(&file).unwrap();
        assert_eq!(records, vec![CodeRecord { cpu: 0x01, start: 0x200, bytes: vec![0x4E, 0x75] }]);
        assert_eq!(creator, "AS 1.42");
    }

    #[test]
    fn a_truncated_file_or_another_segment_is_refused() {
        assert!(decode_code_file(&[0x89, 0x14, 0x81, 0x01, 0x01, 0x01, 0, 0]).is_err());
        assert!(decode_code_file(&[0x89, 0x14, 0x81, 0x01, 0x02, 0x01, 0, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(decode_code_file(b"ELF").is_err());
    }

    #[test]
    #[should_panic(expected = "a record holds at most")]
    fn a_record_longer_than_the_format_holds_is_never_written() {
        encode_code_file(&[CodeRecord { cpu: 0x01, start: 0, bytes: vec![0; MAX_RECORD + 1] }], "x");
    }
}
