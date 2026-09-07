//! `dc.w`, `dc.l` and the Z80 `dw` REFUSE a value outside their width, exactly
//! as `dc.b` already does, instead of casting it and emitting the low bytes.
//!
//! ## What was silently wrong
//!
//! ```text
//!     dc.w    $12345
//! ```
//!
//! assembled to `23 45`, exit 0, no diagnostic: `v as u16` dropped the high
//! nibble. `dc.l $100000000` assembled to `00 00 00 00` the same way. A `dc.b`
//! on the same value has range-checked and refused for as long as it has
//! existed, so the three wider directives were the only data emitters that
//! could turn a wrong constant into plausible bytes without a word said.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) accepts a word in `-32768..=65535` and a long in
//! `-2147483648..=4294967295`, and refuses outside that with
//!
//! ```text
//!     w_bad1.asm(3): error #1320: range overflow
//!      dc.w $12345
//! ```
//!
//! exit 2, on every one of `dc.w $12345`, `dc.w -32769`, `dc.w $10000`,
//! `dc.l $100000000`, `dc.l -$80000001`, a Z80 `dw 74565`, a Z80 `dw -32769`,
//! and `L: dc.w L` for a label at `$20000`. The accepted side, one clean run,
//! exit 0:
//!
//! ```text
//!     dc.w -1 / $FFFF / -32768 / 65535   ->  FFFF FFFF 8000 FFFF
//!     dc.l -1 / $FFFFFFFF / -$80000000   ->  FFFFFFFF FFFFFFFF 80000000
//!     dc.w $7FFF                         ->  7FFF
//!     (cpu z80) dw -1 / 65535 / -32768   ->  FFFF FFFF 0080
//! ```

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).expect("flatten"))
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        l.rsplit_once('(')
                            .and_then(|(_, n)| n.trim_end_matches(')').parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

/// Refused at `line`, with a message naming the value and the window. The bytes
/// are printed on the failing side so the pre-fix truncation is visible.
fn assert_range_refused(body: &str, line: u32, names: &[&str]) {
    match assemble(body) {
        Ok(bytes) => {
            // The tail, not the whole image: an `org` probe carries kilobytes of
            // padding ahead of the word under test.
            let tail = &bytes[bytes.len().saturating_sub(8)..];
            panic!(
                "assembled with exit 0 to {} byte(s) ending {tail:02X?} instead of refusing the out-of-range value",
                bytes.len()
            )
        }
        Err(diags) => {
            let hit = diags
                .iter()
                .find(|(l, m)| *l == line && names.iter().all(|n| m.contains(n)));
            assert!(
                hit.is_some(),
                "expected a diagnostic at line {line} naming {names:?}; got {diags:?}"
            );
        }
    }
}

const M68K: &str = "\tcpu 68000\n\tpadding off\n";
const WORD: &str = "-32768..=65535";
const LONG: &str = "-2147483648..=4294967295";

#[test]
fn dc_w_above_the_unsigned_ceiling_is_refused_not_truncated() {
    // Pre-fix: `23 45`.
    assert_range_refused(&format!("{M68K}\tdc.w $12345\n"), 3, &["74565", WORD]);
}

#[test]
fn dc_w_at_65536_is_refused() {
    assert_range_refused(&format!("{M68K}\tdc.w $10000\n"), 3, &["65536", WORD]);
}

#[test]
fn dc_w_below_the_signed_floor_is_refused() {
    assert_range_refused(&format!("{M68K}\tdc.w -32769\n"), 3, &["-32769", WORD]);
}

#[test]
fn dc_w_in_range_signed_and_unsigned_values_assemble_as_asl() {
    let src = format!("{M68K}\tdc.w -1\n\tdc.w $FFFF\n\tdc.w -32768\n\tdc.w 65535\n\tdc.w $7FFF\n");
    assert_eq!(
        assemble(&src).expect("assemble"),
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x00, 0xFF, 0xFF, 0x7F, 0xFF]
    );
}

#[test]
fn dc_w_of_a_label_beyond_the_word_window_is_refused() {
    // asl: `w_lbl.asm(4): error #1320: range overflow` on `L: dc.w L`.
    assert_range_refused(&format!("{M68K}\torg $20000\nL:\tdc.w L\n"), 4, &["131072", WORD]);
}

#[test]
fn dc_l_above_the_unsigned_ceiling_is_refused_not_truncated() {
    // Pre-fix: `00 00 00 00`.
    assert_range_refused(&format!("{M68K}\tdc.l $100000000\n"), 3, &["4294967296", LONG]);
}

#[test]
fn dc_l_below_the_signed_floor_is_refused() {
    assert_range_refused(&format!("{M68K}\tdc.l -$80000001\n"), 3, &["-2147483649", LONG]);
}

#[test]
fn dc_l_in_range_signed_and_unsigned_values_assemble_as_asl() {
    let src = format!("{M68K}\tdc.l -1\n\tdc.l $FFFFFFFF\n\tdc.l -$80000000\n");
    assert_eq!(
        assemble(&src).expect("assemble"),
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x00, 0x00, 0x00]
    );
}

#[test]
fn z80_dw_above_the_unsigned_ceiling_is_refused_not_truncated() {
    // Pre-fix: `45 23` (little-endian low word of 74565).
    assert_range_refused("\tcpu z80\n\tdw 74565\n", 2, &["74565", WORD]);
}

#[test]
fn z80_dw_below_the_signed_floor_is_refused() {
    assert_range_refused("\tcpu z80\n\tdw -32769\n", 2, &["-32769", WORD]);
}

#[test]
fn z80_dw_in_range_values_assemble_as_asl() {
    assert_eq!(
        assemble("\tcpu z80\n\tdw -1\n\tdw 65535\n\tdw -32768\n").expect("assemble"),
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x80]
    );
}
