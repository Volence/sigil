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
