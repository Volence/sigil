//! The authentic compressors against streams the `p2bin` binary wrote.
//!
//! Every expectation below was measured, not derived from the ported sources:
//! each input was placed as a Z80 blob with `binclude`, assembled by the pinned
//! asl (md5 `61e672562465725a8c102288a7da9098`), and handed to p2bin (md5
//! `4f2fff99c3347bafb93b12d5be1db754`) with `-z=0,<format>,Guess,after`. The
//! stored length is the `comp_z80_size` p2bin writes into its header file; the
//! stream is that many bytes at the blob's ROM offset. The script is
//! `docs/superpowers/notes/2026-09-11-s1-driver-stage2/gen_vectors.py`.
//!
//! Inputs are regenerated here by the same deterministic generator, and each
//! input's CRC32 is pinned too, so a drift in either generator reddens before
//! any compressor is judged.

use sigil_clownlzss_sys::accurate::{compress_kosinski_authentic, compress_saxman_authentic, compress_saxman_bugged};
use sigil_clownlzss_sys::{decompress_kosinski, decompress_saxman_no_header};

/// The generator `gen_vectors.py` feeds p2bin: literals from a small alphabet,
/// and on a third of the steps a copy of earlier output, byte by byte.
fn gen(seed: u64, n: usize, alphabet: u64) -> Vec<u8> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        state >> 33
    };
    let mut out: Vec<u8> = Vec::new();
    while out.len() < n {
        let r = next();
        if out.len() > 8 && r % 3 == 0 {
            let dist = 1 + (next() % (out.len().min(0x2100) as u64)) as usize;
            let length = if r % 7 == 0 { 2 + next() % 300 } else { 2 + next() % 40 };
            for _ in 0..length {
                out.push(out[out.len() - dist]);
            }
        } else {
            out.push((next() % alphabet) as u8);
        }
    }
    out.truncate(n);
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// One input and what p2bin stored for it under each format, as (size, CRC32).
struct Vector {
    name: &'static str,
    seed: u64,
    len: usize,
    alphabet: u64,
    input_crc: u32,
    kosinski: (usize, u32),
    saxman: (usize, u32),
    saxman_bugged: (usize, u32),
}

const VECTORS: &[Vector] = &[
    Vector { name: "one", seed: 1, len: 1, alphabet: 256, input_crc: 0xc0b506dd, kosinski: (16, 0x5b324c8a), saxman: (2, 0x4a75caee), saxman_bugged: (3, 0x95fa5538) },
    Vector { name: "two", seed: 2, len: 2, alphabet: 256, input_crc: 0x70819f5a, kosinski: (16, 0xf00aae39), saxman: (3, 0xcc5feaee), saxman_bugged: (4, 0x0418138f) },
    Vector { name: "small", seed: 3, len: 17, alphabet: 16, input_crc: 0x1e75c736, kosinski: (32, 0x8e8d83a1), saxman: (20, 0x01bab608), saxman_bugged: (21, 0xdcd8dd09) },
    Vector { name: "odd_a", seed: 4, len: 300, alphabet: 7, input_crc: 0x55865362, kosinski: (80, 0x24d2882b), saxman: (80, 0xe34780ec), saxman_bugged: (81, 0x7b5d065e) },
    Vector { name: "odd_b", seed: 5, len: 301, alphabet: 7, input_crc: 0xe81b9948, kosinski: (96, 0x108b0ab1), saxman: (86, 0x950d3d1c), saxman_bugged: (87, 0xc696beff) },
    Vector { name: "odd_c", seed: 6, len: 1000, alphabet: 256, input_crc: 0x55fe8a6b, kosinski: (80, 0x62bd057a), saxman: (163, 0xdd18ba79), saxman_bugged: (164, 0x6a625238) },
    Vector { name: "window", seed: 7, len: 5000, alphabet: 12, input_crc: 0xb6f239d7, kosinski: (528, 0xa813480a), saxman: (855, 0xfe61dede), saxman_bugged: (856, 0x22f31d17) },
    Vector { name: "driver_sized", seed: 8, len: 7110, alphabet: 40, input_crc: 0xa3fe4671, kosinski: (800, 0x2c307d91), saxman: (1256, 0x01f1afdb), saxman_bugged: (1257, 0xc302157e) },
    Vector { name: "zeros_run", seed: 9, len: 4100, alphabet: 1, input_crc: 0x2a96e5a8, kosinski: (64, 0x0ef8836a), saxman: (485, 0x1277e27e), saxman_bugged: (486, 0xf4c9a8c3) },
    Vector { name: "limit", seed: 10, len: 0x2000, alphabet: 30, input_crc: 0xe922cecf, kosinski: (832, 0x572df991), saxman: (1402, 0x5838738c), saxman_bugged: (1403, 0x365418f5) },
    Vector { name: "p1", seed: 11, len: 100, alphabet: 3, input_crc: 0x44797df6, kosinski: (32, 0xf5a45899), saxman: (31, 0xbdcf5925), saxman_bugged: (32, 0x08df9804) },
    Vector { name: "p2", seed: 12, len: 101, alphabet: 3, input_crc: 0x205d6fd1, kosinski: (48, 0x1e9c321f), saxman: (33, 0xa8ebe66e), saxman_bugged: (34, 0xe9c424a3) },
    Vector { name: "p3", seed: 13, len: 102, alphabet: 3, input_crc: 0x81c2c99c, kosinski: (32, 0x373f7ae7), saxman: (29, 0xac0229d2), saxman_bugged: (30, 0x2b1732cb) },
    Vector { name: "p4", seed: 14, len: 103, alphabet: 3, input_crc: 0x994d6b2f, kosinski: (32, 0x661230eb), saxman: (28, 0x6c8baba9), saxman_bugged: (29, 0x7d647f6a) },
];

fn check(format: &str, pick: fn(&Vector) -> (usize, u32), compress: fn(&[u8]) -> Vec<u8>) {
    let mut wrong = Vec::new();
    for v in VECTORS {
        let input = gen(v.seed, v.len, v.alphabet);
        let got = compress(&input);
        let want = pick(v);
        if (got.len(), crc32(&got)) != want {
            wrong.push(format!(
                "{}: sigil {} bytes crc {:08x}, p2bin {} bytes crc {:08x}",
                v.name,
                got.len(),
                crc32(&got),
                want.0,
                want.1
            ));
        }
    }
    assert!(wrong.is_empty(), "{format} differs from p2bin on {} of {} vectors:\n{}", wrong.len(), VECTORS.len(), wrong.join("\n"));
}

#[test]
fn the_generator_reproduces_the_inputs_p2bin_was_given() {
    for v in VECTORS {
        let input = gen(v.seed, v.len, v.alphabet);
        assert_eq!(input.len(), v.len, "{}", v.name);
        assert_eq!(crc32(&input), v.input_crc, "{}: the input is not the one p2bin compressed", v.name);
    }
}

#[test]
fn kosinski_authentic_matches_p2bin_on_every_vector() {
    check("kosinski", |v| v.kosinski, compress_kosinski_authentic);
}

#[test]
fn saxman_authentic_matches_p2bin_on_every_vector() {
    check("saxman", |v| v.saxman, compress_saxman_authentic);
}

#[test]
fn saxman_bugged_matches_p2bin_on_every_vector() {
    check("saxman-bugged", |v| v.saxman_bugged, compress_saxman_bugged);
}

/// The junk byte's two values are both measured, so both halves of the parity
/// rule are pinned by the vectors above and not by this module's reading of it.
#[test]
fn the_vectors_cover_both_parities_of_the_saxman_junk_byte() {
    let odd = VECTORS.iter().filter(|v| v.saxman.0 % 2 == 1).count();
    let even = VECTORS.iter().filter(|v| v.saxman.0 % 2 == 0).count();
    assert!(odd >= 2 && even >= 2, "odd {odd}, even {even}");
    for v in VECTORS {
        let got = compress_saxman_bugged(&gen(v.seed, v.len, v.alphabet));
        let want = if v.saxman.0 % 2 == 1 { 0x4E } else { 0x00 };
        assert_eq!(got.last(), Some(&want), "{}", v.name);
    }
}

/// p2bin refuses a blob over 0x2000 bytes, so it can never show the dummy
/// match Sega's compressor inserts past each 0xA000 input bytes. That path is
/// pinned against the accurate-kosinski compressor instead (the `kosinski-compress`
/// binary built from `programs/accurate-kosinski` at `45abe26a`, md5
/// `044a1bf2e26005d6880ae9d3b36016af`), whose output for this input contains
/// the dummy match `00 F0 01`.
#[test]
fn kosinski_authentic_matches_accurate_kosinski_past_the_0xa000_boundary() {
    let input = gen(15, 0xA100, 30);
    assert_eq!(crc32(&input), 0x9d02736a);
    let got = compress_kosinski_authentic(&input);
    assert_eq!((got.len(), crc32(&got)), (5360, 0x69daaf8e));
    assert!(got.windows(3).any(|w| w == [0x00, 0xF0, 0x01]));
}

#[test]
fn every_authentic_kosinski_stream_decompresses_to_its_input() {
    for v in VECTORS {
        let input = gen(v.seed, v.len, v.alphabet);
        let kos = compress_kosinski_authentic(&input);
        assert_eq!(decompress_kosinski(&kos).expect("kosinski decodes"), input, "{} kosinski", v.name);
    }
}

/// Sega's Saxman compressor (Okumura's) finds matches in a ring buffer that
/// starts zero-filled, so it can emit a match whose source begins in those
/// zeros and runs on into the output. Sonic 2's decompressor, and clownlzss's
/// after it, read a match whose source starts before the output as zeros
/// THROUGHOUT, so such a stream decodes to different bytes than went in.
///
/// Measured on these vectors' p2bin streams with a port of the game's own
/// decompressor (`DecompressSoundDriver`) and of Okumura's `Decode`: Okumura's
/// reproduces all fourteen inputs, the game's differs on exactly the five below,
/// and each of those five holds exactly one straddling match. This is why the
/// placement's round-trip check decodes the game's way: a stream the game would
/// misread is refused instead of shipped.
#[test]
fn an_authentic_saxman_stream_reads_back_wrong_exactly_when_a_match_straddles_the_start() {
    let straddling = ["odd_b", "p1", "p2", "p3", "p4"];
    let mut differ = Vec::new();
    for v in VECTORS {
        let input = gen(v.seed, v.len, v.alphabet);
        let sax = compress_saxman_authentic(&input);
        let back = decompress_saxman_no_header(&sax, sax.len()).expect("saxman decodes");
        assert_eq!(back.len(), input.len(), "{}: a straddling match is zeros, not missing bytes", v.name);
        if back != input {
            differ.push(v.name);
        }
    }
    assert_eq!(differ, straddling);
}
