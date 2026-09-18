//! clownlzss's optimal parser against the streams p2bin's `-optimised` formats
//! wrote, and the identification that rests on it.
//!
//! p2bin offers Kosinski and Saxman twice: "authentic", which is Sega's own
//! greedy compressor (pinned next door in `accurate_vectors.rs`), and
//! "optimised". The claim this file measures is that "optimised" IS the
//! clownlzss library this crate vendors, and which of its entry points each
//! format is, so that `sigil-link`'s `BlobFormat` can route to them.
//!
//! Every expectation below was measured, not derived from the vendored sources:
//! each input was placed as a Z80 blob with `binclude`, assembled by the pinned
//! asl (md5 `61e672562465725a8c102288a7da9098`), and handed to p2bin (md5
//! `4f2fff99c3347bafb93b12d5be1db754`) with `-z=0,<format>,Guess,after`. The
//! stored length is the `comp_z80_size` p2bin writes into its header file; the
//! stream is that many bytes at the blob's ROM offset. The script is
//! `docs/superpowers/notes/2026-09-18-p2bin-optimised-compressors/gen_opt_vectors.py`
//! and its output is beside it.
//!
//! The inputs are the fourteen of `accurate_vectors.rs`, regenerated here by the
//! same deterministic generator with their CRC32s pinned, so a drift in the
//! generator reddens before any compressor is judged. That run also reprinted
//! the `kosinski`, `saxman` and `saxman-bugged` columns, and every one of the
//! forty-two matched the numbers `accurate_vectors.rs` already held: the same
//! p2bin through a second, independently written script.

use sigil_clownlzss_sys::{compress_kosinski, compress_saxman, decompress_kosinski, decompress_saxman_no_header};

/// The generator `gen_opt_vectors.py` feeds p2bin: literals from a small
/// alphabet, and on a third of the steps a copy of earlier output, byte by byte.
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

/// One input and what p2bin stored for it under each optimised format, as
/// (size, CRC32).
struct Vector {
    name: &'static str,
    seed: u64,
    len: usize,
    alphabet: u64,
    input_crc: u32,
    kosinski_optimised: (usize, u32),
    saxman_optimised: (usize, u32),
}

const VECTORS: &[Vector] = &[
    Vector { name: "one", seed: 1, len: 1, alphabet: 256, input_crc: 0xc0b506dd, kosinski_optimised: (6, 0x43b69185), saxman_optimised: (2, 0x4a75caee) },
    Vector { name: "two", seed: 2, len: 2, alphabet: 256, input_crc: 0x70819f5a, kosinski_optimised: (7, 0xbff9ca43), saxman_optimised: (3, 0xcc5feaee) },
    Vector { name: "small", seed: 3, len: 17, alphabet: 16, input_crc: 0x1e75c736, kosinski_optimised: (23, 0x2f35cce6), saxman_optimised: (20, 0x01bab608) },
    Vector { name: "odd_a", seed: 4, len: 300, alphabet: 7, input_crc: 0x55865362, kosinski_optimised: (67, 0xa98cf448), saxman_optimised: (79, 0xfc6f958c) },
    Vector { name: "odd_b", seed: 5, len: 301, alphabet: 7, input_crc: 0xe81b9948, kosinski_optimised: (81, 0x9e0cf231), saxman_optimised: (86, 0x914b82c7) },
    Vector { name: "odd_c", seed: 6, len: 1000, alphabet: 256, input_crc: 0x55fe8a6b, kosinski_optimised: (77, 0x83ba0b83), saxman_optimised: (162, 0xd7f8dca1) },
    Vector { name: "window", seed: 7, len: 5000, alphabet: 12, input_crc: 0xb6f239d7, kosinski_optimised: (497, 0xa31d44fc), saxman_optimised: (844, 0xff3e5fac) },
    Vector { name: "driver_sized", seed: 8, len: 7110, alphabet: 40, input_crc: 0xa3fe4671, kosinski_optimised: (754, 0xeaf77c28), saxman_optimised: (1236, 0xe8c28c91) },
    Vector { name: "zeros_run", seed: 9, len: 4100, alphabet: 1, input_crc: 0x2a96e5a8, kosinski_optimised: (59, 0x1b7301a1), saxman_optimised: (485, 0x48dfcaa2) },
    Vector { name: "limit", seed: 10, len: 0x2000, alphabet: 30, input_crc: 0xe922cecf, kosinski_optimised: (786, 0x5386cd98), saxman_optimised: (1393, 0x0965995d) },
    Vector { name: "p1", seed: 11, len: 100, alphabet: 3, input_crc: 0x44797df6, kosinski_optimised: (30, 0x21d1e0a8), saxman_optimised: (32, 0x6217c4e4) },
    Vector { name: "p2", seed: 12, len: 101, alphabet: 3, input_crc: 0x205d6fd1, kosinski_optimised: (34, 0xf3d02705), saxman_optimised: (34, 0xd7bf8692) },
    Vector { name: "p3", seed: 13, len: 102, alphabet: 3, input_crc: 0x81c2c99c, kosinski_optimised: (29, 0xbe13b263), saxman_optimised: (27, 0x36b725a1) },
    Vector { name: "p4", seed: 14, len: 103, alphabet: 3, input_crc: 0x994d6b2f, kosinski_optimised: (27, 0x94984125), saxman_optimised: (26, 0x70de5f32) },
];

fn check(format: &str, pick: fn(&Vector) -> (usize, u32), compress: fn(&[u8]) -> Vec<u8>) {
    let mut wrong = Vec::new();
    for v in VECTORS {
        let got = compress(&gen(v.seed, v.len, v.alphabet));
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
fn clownlzss_kosinski_is_p2bins_kosinski_optimised() {
    check("kosinski-optimised", |v| v.kosinski_optimised, |d| compress_kosinski(d).expect("kosinski"));
}

#[test]
fn clownlzss_headerless_saxman_is_p2bins_saxman_optimised() {
    check("saxman-optimised", |v| v.saxman_optimised, |d| compress_saxman(d, false).expect("saxman"));
}

/// The positive control for the two tests above: the same compressor with the
/// other framing. `-z` writes the stored size into the header FILE p2bin is
/// given, so the stream carries no size of its own, and the header-framed
/// variant is exactly two bytes longer on every vector. If the comparison above
/// could not tell these apart it could not tell anything apart.
#[test]
fn the_header_framed_saxman_is_two_bytes_longer_on_every_vector_and_is_not_what_p2bin_stores() {
    for v in VECTORS {
        let input = gen(v.seed, v.len, v.alphabet);
        let framed = compress_saxman(&input, true).expect("saxman with header");
        assert_eq!(framed.len(), v.saxman_optimised.0 + 2, "{}", v.name);
        assert_ne!((framed.len(), crc32(&framed)), v.saxman_optimised, "{}", v.name);
    }
}

/// An optimised stream is the same CONTAINER as its authentic sibling: the
/// decompressor `sigil-link`'s round-trip check runs on a `kosinski` blob reads a
/// `kosinski-optimised` one, and likewise for Saxman. The placement refuses any
/// stream that fails this, so a wrong pairing would be a refused build, not a
/// wrong ROM; measuring it here says WHY it is not refused.
#[test]
fn every_optimised_stream_decompresses_to_its_input_through_the_authentic_decoder() {
    for v in VECTORS {
        let input = gen(v.seed, v.len, v.alphabet);
        let kos = compress_kosinski(&input).expect("kosinski");
        assert_eq!(decompress_kosinski(&kos).expect("kosinski decodes"), input, "{} kosinski-optimised", v.name);
        let sax = compress_saxman(&input, false).expect("saxman");
        let back = decompress_saxman_no_header(&sax, sax.len()).expect("saxman decodes");
        assert_eq!(back, input, "{} saxman-optimised", v.name);
    }
}

/// Sega's Saxman compressor emits a match whose source starts in the zeros its
/// ring buffer began with, on five of these fourteen inputs, and the game reads
/// such a match as zeros throughout: `accurate_vectors.rs`'s
/// `an_authentic_saxman_stream_reads_back_wrong_exactly_when_a_match_straddles_the_start`
/// pins exactly which five. clownlzss's parser never looks before the start, so
/// none of the fourteen straddles, which is why the test above holds on all of
/// them and not on nine. Stated as a measurement of the optimised compressor,
/// not as a licence to skip the round-trip check.
#[test]
fn no_optimised_saxman_stream_straddles_the_start_on_the_inputs_where_the_authentic_one_does() {
    for name in ["odd_b", "p1", "p2", "p3", "p4"] {
        let v = VECTORS.iter().find(|v| v.name == name).expect(name);
        let input = gen(v.seed, v.len, v.alphabet);
        let sax = compress_saxman(&input, false).expect("saxman");
        assert_eq!(decompress_saxman_no_header(&sax, sax.len()).expect("decodes"), input, "{name}");
    }
}
