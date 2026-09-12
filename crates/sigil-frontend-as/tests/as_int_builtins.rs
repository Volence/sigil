//! asl's integer-valued builtin family (`sgn`, `bitcnt`, `firstbit`, `bitpos`,
//! `toupper`, `tolower`, beside the older `lastbit`), and the empty argument
//! every numeric builtin reads as 0.
//!
//! Row AS-MISSING-BUILTINS. Every fixture here IS a probe file asl assembled:
//! source, listing and verdict are read from
//! `docs/superpowers/notes/2026-09-12-as-missing-builtins/probes/` at test
//! time, so the text sigil is tested on cannot drift from the text the oracle
//! answered, and the expected image is REBUILT FROM asl's LISTING rather than
//! copied out of it by hand. The oracle is asl 1.42 Beta Bld 212,
//! `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run through `asl_ref.sh`'s `asl_run`
//! with `-xx -n -q -A -L -U -i .`.
//!
//! [`builds`] first requires the probe's recorded `ASL_EXIT=0`: a run carrying
//! any error is not a source of values for the lines that did assemble, so a
//! listing is only read when the run was clean. [`refused`] requires the
//! recorded exit to be non-zero, and reads no byte from it.

use std::path::PathBuf;

use sigil_frontend_as::{assemble_root_located, Options};

fn probe_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-12-as-missing-builtins/probes")
}

fn read(name: &str, ext: &str) -> String {
    let path = probe_dir().join(format!("{name}.{ext}"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()))
}

/// asl's exit status for the probe, as `asl_run` recorded it.
fn asl_exit(name: &str) -> i32 {
    let out = read(name, "asl.out");
    let line = out
        .lines()
        .find_map(|l| l.strip_prefix("ASL_EXIT="))
        .unwrap_or_else(|| panic!("{name}.asl.out records no ASL_EXIT"));
    line.trim().parse().expect("ASL_EXIT is a number")
}

/// The image asl's listing shows: every `address : bytes` row, the unnumbered
/// continuation rows of a long line included, placed at its address. The byte
/// column is the text between ` : ` and the tab that starts the echoed source;
/// a row whose column is not hex (`(MACRO)`, `=$7`, `=>TRUE`) carries no
/// bytes. Reading stops at the symbol table.
fn listing_image(name: &str) -> Vec<u8> {
    let mut image = Vec::new();
    for line in read(name, "lst").lines() {
        if line.contains("Symbol Table") {
            break;
        }
        let Some((left, right)) = line.split_once(" : ") else { continue };
        let Some(addr) = left.split_whitespace().last().and_then(|a| usize::from_str_radix(a, 16).ok()) else {
            continue;
        };
        let column = right.split('\t').next().unwrap_or("").trim();
        if column.is_empty() || !column.chars().all(|c| c.is_ascii_hexdigit() || c == ' ') {
            continue;
        }
        let hex: String = column.chars().filter(|c| *c != ' ').collect();
        assert!(hex.len() % 2 == 0, "{name}.lst: odd byte column in {line:?}");
        for (k, pair) in hex.as_bytes().chunks(2).enumerate() {
            let byte = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
            if image.len() <= addr + k {
                image.resize(addr + k + 1, 0);
            }
            image[addr + k] = byte;
        }
    }
    image
}

/// Assemble and link one probe. `Ok` is the image, `Err` the messages of
/// whichever stage refused it.
fn sigil_image(name: &str) -> Result<Vec<u8>, Vec<String>> {
    let path = probe_dir().join(format!("{name}.asm"));
    let m = assemble_root_located(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).map_err(|e| vec![format!("{e:?}")])?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

/// asl assembled the probe cleanly, and sigil's image is the one its listing
/// shows, byte for byte.
#[track_caller]
fn builds(name: &str) {
    assert_eq!(asl_exit(name), 0, "{name}: not a clean asl run, so its listing is no source of values");
    let want = listing_image(name);
    assert!(!want.is_empty(), "{name}: the listing shows no bytes");
    match sigil_image(name) {
        Ok(got) if got == want => {}
        Ok(got) => {
            let at = got.iter().zip(&want).position(|(g, w)| g != w).unwrap_or(got.len().min(want.len()));
            let end = |v: &[u8]| v[at.min(v.len())..(at + 8).min(v.len())].to_vec();
            panic!(
                "{name}: sigil's image differs from asl's listing at ${at:X} (asl {} bytes, sigil {}):\n  asl   {:02X?}\n  sigil {:02X?}",
                want.len(),
                got.len(),
                end(&want),
                end(&got)
            );
        }
        Err(m) => panic!("{name}: asl assembles it, sigil refused: {m:?}"),
    }
}

/// asl refused the probe, and so does sigil.
#[track_caller]
fn refused(name: &str) {
    assert_ne!(asl_exit(name), 0, "{name}: asl assembled it cleanly, so it is not a refusal");
    if let Ok(b) = sigil_image(name) {
        panic!("{name}: asl refuses it, sigil built {b:02X?}");
    }
}

/// An empty argument is the integer 0 for every numeric builtin, in every
/// integer context, on both CPUs. asl, exit 0, `empty.lst`:
///
/// ```text
///        4/       0 : 0000 0000           	dc.l int()
///        5/       4 : 0000 0000           	dc.l abs()
///        6/       8 : 00                  	dc.b abs()
///        7/       9 : FFFF FFFF           	dc.l lastbit()
///        8/       D : 0000 0000           	dc.l abs( )
///       10/      15 : 203C 0000 0000      	move.l #abs(),d0
///       13/      1F : 0000 0005           	dc.l abs()+5
///       21/      29 : 0000 0001           	dc.l INT(cos())
///       32/      55 : 0000 0001           	dc.l INT(exp())
/// ```
///
/// and `empty_z80.lst`: `3/ 0 : 00  db abs()`, `4/ 1 : 3E FF  ld a,lastbit()`.
#[test]
fn an_empty_argument_is_zero_for_every_builtin() {
    builds("empty");
    builds("empty_z80");
}

/// Where a builtin refuses 0 it refuses the empty call the same way, and an
/// empty FLOAT builtin is still a float: `INT(log())` and `INT(ln())` draw
/// `error #1870: function argument out of definition range`, `INT(acosh())`
/// `error #1880: floating point overflow`, and a bare `dc.l sin()` `error
/// #1133: expected integer or string, but got floating point number` (exit 2
/// each).
#[test]
fn an_empty_argument_is_refused_where_zero_is() {
    for name in ["ref_empty_log", "ref_empty_ln", "ref_empty_acosh", "ref_empty_sin_bare"] {
        refused(name);
    }
}

/// `firstbit` over its whole table: 1,862 values, every one from the same
/// clean run. It is NOT the lowest set bit on an odd value, which is what a
/// half-fix would write; `t_firstbit.lst`:
///
/// ```text
///        4/       0 : FFFF FFFF           	dc.l firstbit($0)
///        5/       4 : FFFF FFFF           	dc.l firstbit($1)
///        7/       C : 0000 0000           	dc.l firstbit($3)
///        9/      14 : 0000 0001           	dc.l firstbit($5)
///       16/      30 : 0000 0002           	dc.l firstbit($C)
///      357/     584 : 0000 0004           	dc.l firstbit($161)
///     1028/    1000 : 0000 0000           	dc.l firstbit(-$1)
///     1029/    1004 : 0000 0001           	dc.l firstbit(-$2)
///     1368/    1550 : 0000 001F           	dc.l firstbit($80000000)
///     1372/    1560 : 0000 0020           	dc.l firstbit($100000000)
///     1495/    174C : 0000 003F           	dc.l firstbit((-$7FFFFFFFFFFFFFFF-1))
/// ```
#[test]
fn firstbit_matches_asl_over_its_table() {
    builds("t_firstbit");
}

/// Every integer context, both CPUs: any case, a forward label and a forward
/// `equ`, `equ`/`set`, an `if`, two immediates, nesting, interpolation and the
/// empty argument. `ctx_firstbit.lst` and `ctx_firstbit_z80.lst`:
///
/// ```text
///        4/       0 : 06                  	dc.b firstbit(FwdL)
///        8/      41 : 02                  	dc.b FIRSTBIT(12)
///       12/      45 : 02                  	dc.b firstbit(Later)
///       23/      4F : 323C 0008           	move.w #firstbit(12)<<2,d1
///       25/      54 : 01                  	dc.b firstbit(firstbit(12))
///       26/      55 : 32                  	dc.b "\{firstbit(12)}"
///       27/      56 : FF                  	dc.b firstbit()
///        5/       3 : 21 02 00            	ld hl,firstbit(12)
/// ```
#[test]
fn firstbit_works_wherever_an_integer_is_read() {
    builds("ctx_firstbit");
    builds("ctx_firstbit_z80");
}

/// What asl refuses: a float (`firstbit(5.0)`, and a float `set` symbol) aborts
/// it (`error #10000: internal error`, exit 3); a string is `error #1136`, two
/// arguments `error #1490`, an undefined symbol `error #1010` (exit 2).
#[test]
fn what_asl_refuses_in_firstbit_is_refused() {
    for name in [
        "ref_firstbit_float",
        "ref_firstbit_floatsym",
        "ref_firstbit_string",
        "ref_firstbit_twoarg",
        "ref_firstbit_undef",
    ] {
        refused(name);
    }
}

/// `bitcnt` over its whole table. It counts all 64 bits of the two's-complement
/// value, so a negative argument counts its sign extension; `t_bitcnt.lst`:
///
/// ```text
///        4/       0 : 0000 0000           	dc.l bitcnt($0)
///       11/      1C : 0000 0003           	dc.l bitcnt($7)
///     1373/    1564 : 0000 0020           	dc.l bitcnt($FFFFFFFF)
///     1496/    1750 : 0000 003F           	dc.l bitcnt($7FFFFFFFFFFFFFFF)
///     1028/    1000 : 0000 0040           	dc.l bitcnt(-$1)
///     1029/    1004 : 0000 003F           	dc.l bitcnt(-$2)
///     1155/    11FC : 0000 0039           	dc.l bitcnt(-$80)
///     1495/    174C : 0000 0001           	dc.l bitcnt((-$7FFFFFFFFFFFFFFF-1))
/// ```
#[test]
fn bitcnt_matches_asl_over_its_table() {
    builds("t_bitcnt");
}

/// Every integer context, both CPUs. `ctx_bitcnt.lst` and `ctx_bitcnt_z80.lst`:
///
/// ```text
///        4/       0 : 01                  	dc.b bitcnt(FwdL)
///        8/      41 : 03                  	dc.b BITCNT(7)
///       12/      45 : 03                  	dc.b bitcnt(Later)
///       23/      4F : 323C 000C           	move.w #bitcnt(7)<<2,d1
///       25/      54 : 02                  	dc.b bitcnt(bitcnt(7))
///       26/      55 : 33                  	dc.b "\{bitcnt(7)}"
///       27/      56 : 00                  	dc.b bitcnt()
///        5/       3 : 21 03 00            	ld hl,bitcnt(7)
/// ```
#[test]
fn bitcnt_works_wherever_an_integer_is_read() {
    builds("ctx_bitcnt");
    builds("ctx_bitcnt_z80");
}

/// What asl refuses: a float aborts it (`error #10000`, exit 3); a string is
/// `error #1136`, two arguments `error #1490`, an undefined symbol
/// `error #1010` (exit 2).
#[test]
fn what_asl_refuses_in_bitcnt_is_refused() {
    for name in [
        "ref_bitcnt_float",
        "ref_bitcnt_floatsym",
        "ref_bitcnt_string",
        "ref_bitcnt_twoarg",
        "ref_bitcnt_undef",
    ] {
        refused(name);
    }
}

/// `bitpos` over every value it accepts, 2^0..2^62; `t_bitpos.lst`:
///
/// ```text
///        4/       0 : 0000 0000           	dc.l bitpos($1)
///        5/       4 : 0000 0001           	dc.l bitpos($2)
///       11/      1C : 0000 0007           	dc.l bitpos($80)
///       35/      7C : 0000 001F           	dc.l bitpos($80000000)
///       36/      80 : 0000 0020           	dc.l bitpos($100000000)
///       66/      F8 : 0000 003E           	dc.l bitpos($4000000000000000)
/// ```
#[test]
fn bitpos_matches_asl_over_every_value_it_accepts() {
    builds("t_bitpos");
}

/// Every integer context, both CPUs, except a forward reference, which asl
/// refuses (see the note). `ctx_bitpos.lst` and `ctx_bitpos_z80.lst`:
///
/// ```text
///        5/       1 : 03                  	dc.b BITPOS(8)
///       19/       E : 323C 000C           	move.w #bitpos(8)<<2,d1
///       21/      13 : 01                  	dc.b bitpos(bitpos(4))
///       22/      14 : 33                  	dc.b "\{bitpos(8)}"
///        5/       3 : 21 03 00            	ld hl,bitpos(8)
/// ```
#[test]
fn bitpos_works_wherever_an_integer_is_read() {
    builds("ctx_bitpos");
    builds("ctx_bitpos_z80");
}

/// Anything but one set bit in a positive value is `error #1540: not exactly
/// one bit set` (exit 2): `bitpos(0)`, `bitpos(6)`, `bitpos(-1)`,
/// `bitpos(-$7FFFFFFFFFFFFFFF-1)` (one bit, but the sign bit),
/// `bitpos($FFFEB)` and the empty call.
#[test]
fn bitpos_refuses_anything_but_one_positive_bit() {
    for name in [
        "ref_bitpos_0",
        "ref_bitpos_6",
        "ref_bitpos_m1",
        "ref_bitpos_min",
        "ref_bitpos_fffeb",
        "ref_empty_bitpos",
    ] {
        refused(name);
    }
}

/// What else asl refuses: a float aborts it (`error #10000`, exit 3); a string
/// is `error #1136`, two arguments `error #1490`; an undefined symbol is
/// `error #1540` here, not `#1010`, because pass 1 reads it as 0 (exit 2).
#[test]
fn what_asl_refuses_in_bitpos_is_refused() {
    for name in [
        "ref_bitpos_float",
        "ref_bitpos_floatsym",
        "ref_bitpos_string",
        "ref_bitpos_twoarg",
        "ref_bitpos_undef",
    ] {
        refused(name);
    }
}
