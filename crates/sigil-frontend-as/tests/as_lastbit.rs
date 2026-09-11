//! asl's `lastbit(x)` builtin: the index of the highest set bit.
//!
//! Sonic 2 sizes its end-of-ROM pad with `cnop -1,2<<lastbit(*-StartOfRom-1)`
//! (`s2.asm(91263)`), which expands through its own `cnop` and `org` macros
//! into two `if` conditions. sigil did not know the function, so both
//! conditions failed to evaluate and were refused.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .`, one construct
//! per probe (`l6_*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every
//! expected byte is from a run that exited 0; refusals are read for
//! accept-or-refuse only, and `lastbit(5.0)`, where asl aborts (`#10000 internal
//! error`, exit 3), has no answer to match.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | a 32-bit index (`lastbit(-1)` as 31, `lastbit($100000000)` refused or 0) | `lastbit_is_the_highest_set_bit_of_a_64_bit_integer` |
//! | no bit set read as 0 instead of -1 | `lastbit_is_the_highest_set_bit_of_a_64_bit_integer` |
//! | the LOWEST set bit (asl's `firstbit`) | `lastbit_is_the_highest_set_bit_of_a_64_bit_integer` |
//! | the name matched in one case only | `lastbit_works_wherever_an_integer_is_read` |
//! | the value right but not through Sonic 2's `cnop`/`org` macros | `sonic_2_s_end_of_rom_pad_assembles` |

use sigil_frontend_as::{assemble_root_located, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HEAD_Z80: &str = "\tcpu z80\n\torg 0\n";

fn assemble_with(head: &str, body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    let m = assemble_root_located(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .map_err(|e| vec![format!("{e:?}")])?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

fn check_all(head: &str, cases: &[(&str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| match assemble_with(head, body) {
            Ok(got) if got == *want => None,
            other => Some(format!("  {body:?}\n    asl   {want:02X?}\n    sigil {other:02X?}")),
        })
        .collect();
    assert!(wrong.is_empty(), "{} of {} cases differ from asl:\n{}", wrong.len(), cases.len(), wrong.join("\n"));
}

/// Assemble and return the bytes at `at..at+want.len()`: for the cases whose
/// listing starts past a reservation (`ds.b`, `org`), whose own bytes asl's
/// listing does not print.
fn bytes_at(body: &str, at: usize, len: usize) -> Result<Vec<u8>, Vec<String>> {
    assemble_with(HEAD, body).map(|b| b.get(at..at + len).map(<[u8]>::to_vec).unwrap_or_default())
}

#[test]
fn lastbit_is_the_highest_set_bit_of_a_64_bit_integer() {
    check_all(
        HEAD,
        &[
            ("\tdc.b lastbit(1)", &[0x00]),
            ("\tdc.b lastbit(5)", &[0x02]),
            ("\tdc.b lastbit($80)", &[0x07]),
            ("\tdc.b lastbit($FFFEB)", &[0x13]),
            ("\tdc.l 2<<lastbit($FFFEB)", &[0x00, 0x10, 0x00, 0x00]),
            ("\tdc.l lastbit(0)", &[0xFF, 0xFF, 0xFF, 0xFF]),
            ("\tdc.l lastbit(-1)", &[0x00, 0x00, 0x00, 0x3F]),
            ("\tdc.l lastbit(-2)", &[0x00, 0x00, 0x00, 0x3F]),
            ("\tdc.l lastbit($80000000)", &[0x00, 0x00, 0x00, 0x1F]),
            ("\tdc.l lastbit($7FFFFFFF)", &[0x00, 0x00, 0x00, 0x1E]),
            ("\tdc.l lastbit($100000000)", &[0x00, 0x00, 0x00, 0x20]),
            ("\tdc.b lastbit(lastbit($FFFEB))", &[0x04]),
            // An empty argument is 0, as `()` is: `FF` is -1.
            ("\tdc.b lastbit()", &[0xFF]),
        ],
    );
}

/// Any case, any integer context, a forward reference, and the Z80.
#[test]
fn lastbit_works_wherever_an_integer_is_read() {
    check_all(
        HEAD,
        &[
            ("\tdc.b LASTBIT(5)", &[0x02]),
            ("\tdc.b LastBit(9)", &[0x03]),
            ("X equ lastbit(5)\n\tdc.b X", &[0x02]),
            ("\tif lastbit(4)=2\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif", &[0x01]),
            ("\tmove.l #2<<lastbit($FFFEB),d0", &[0x20, 0x3C, 0x00, 0x10, 0x00, 0x00]),
            ("\tdc.b lastbit(Later)\nLater equ 5", &[0x02]),
        ],
    );
    check_all(HEAD_Z80, &[("\tdb lastbit(5)\n\tld a,lastbit(9)", &[0x02, 0x3E, 0x03])]);
    // A forward LABEL (at $41), a reservation sized by it, and the PC.
    assert_eq!(assemble_with(HEAD, "\tdc.b lastbit(Later)\n\tds.b $40\nLater:\tdc.b 1").map(|b| b[0]), Ok(0x06));
    assert_eq!(
        assemble_with(HEAD, "\tds.b lastbit(5)\n\tdc.b $EE").map(|b| (b.len(), b.last().copied())),
        Ok((3, Some(0xEE)))
    );
    assert_eq!(bytes_at("\torg $123\n\tdc.l 2<<lastbit(*-1)", 0x123, 4), Ok(vec![0x00, 0x00, 0x02, 0x00]));
}

/// Sonic 2's own `org` and `cnop` macros, driven by `lastbit` exactly as its
/// end-of-ROM pad is: a 3-byte image pads nothing, and a $123-byte one pads
/// to $1FF so that the `dc.b $00` after it ends the image at $200.
#[test]
fn sonic_2_s_end_of_rom_pad_assembles() {
    let macros = "org\tmacro address\n\tif address < *\n\terror \"too much\"\n\telseif address > *\n\t!org address\n\
                  \tendif\n\tendm\ncnop\tmacro offset,alignment\n\torg (*-1+(alignment)-((*-1+(-(offset)))#(alignment)))\n\tendm\n";
    check_all(
        HEAD,
        &[(&format!("{macros}Start:\n\tdc.b 1,2,3\n\tcnop -1,2<<lastbit(*-Start-1)\n\tdc.b $00"), &[0x01, 0x02, 0x03, 0x00])],
    );
    let big = format!("{macros}Start:\n\tds.b $123\n\tcnop -1,2<<lastbit(*-Start-1)\n\tdc.b $00\nEndR:\n\tdc.l EndR");
    assert_eq!(bytes_at(&big, 0x1FF, 5), Ok(vec![0x00, 0x00, 0x00, 0x02, 0x00]));
}

/// What asl refuses: two arguments (`#1490`), a string (`#1136`), an undefined
/// symbol (`#1010`); and a float, on which asl aborts.
#[test]
fn what_asl_refuses_is_refused() {
    for body in ["\tdc.b lastbit(1,2)", "\tdc.b lastbit(\"A\")", "\tdc.b lastbit(Nope)", "\tdc.b lastbit(5.0)"] {
        assert!(assemble_with(HEAD, body).is_err(), "asl refuses {body:?}, sigil assembled it");
    }
}
