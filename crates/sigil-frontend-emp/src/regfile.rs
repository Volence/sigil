//! The register-file seam (rung-2 §2): the contract-reglist vocabulary is
//! CPU-parametric, ISA-free. The reglist STRUCTURE is identical across CPUs (a
//! set of register-unit names, expanded from pair sugar, complemented against a
//! universe); only two things differ — the name→unit map and the universe.
//!
//! This module owns that difference so the Z80 contract checker
//! ([`crate::corpus_contracts`]'s Z80 sibling pass) and the genericized
//! `preserves` proof read ONE register vocabulary. The 68k register file is the
//! `d0..a7` universe (an index unit alone — no register halves); the Z80 file is
//! the finer `{a,f,b,c,d,e,h,l}` 8-bit halves plus the index/pair units
//! `{ix,iy}` (§1.1). A pair name in a Z80 reglist EXPANDS to its halves
//! (`bc→{b,c}`, `af→{a,f}`); `ix`/`iy` are units (no half-split).
//!
//! The 68k PRODUCTION contract path (`lower/proc.rs`'s `reglist_expand`, with its
//! ordinal ranges + `sr` composition and `[proc.*-invalid]` diagnostics) is
//! unchanged and frozen — this seam is the Z80 recognizer plus the CPU-parametric
//! `expand_reglist` the rung-2 checker and tests consume.

use std::collections::BTreeSet;

/// Which CPU's register vocabulary a reglist is read against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegFile {
    /// 68k: `d0`..`d7`, `a0`..`a7` (`sp`→`a7`). No register halves; ordinal
    /// ranges are the movem form (handled by the frozen 68k production path, not
    /// this seam's `expand_reglist`).
    M68k,
    /// Z80: the 8-bit halves `{a,f,b,c,d,e,h,l}` plus the index units `{ix,iy}`.
    /// A pair name expands to its halves; reglists enumerate (no range form).
    Z80,
}

impl RegFile {
    /// The register UNITS a reglist name denotes, or `None` when the name is not
    /// in this CPU's register vocabulary (the caller reports
    /// `[contract.unknown-register]`). A Z80 PAIR expands to its two halves; a
    /// half, an index, or a 68k register denotes itself.
    fn units_of(self, name: &str) -> Option<Vec<&'static str>> {
        match self {
            RegFile::M68k => M68K_UNITS.iter().find(|u| **u == name).map(|u| vec![*u]),
            RegFile::Z80 => match name {
                "bc" => Some(vec!["b", "c"]),
                "de" => Some(vec!["d", "e"]),
                "hl" => Some(vec!["h", "l"]),
                "af" => Some(vec!["a", "f"]),
                _ => Z80_UNITS.iter().find(|u| **u == name).map(|u| vec![*u]),
            },
        }
    }

    /// Whether this CPU's reglists carry an ordinal `lo-hi` RANGE form. 68k does
    /// (the movem mask); Z80 does not — its registers are not ordinal, so a Z80
    /// range is a loud reason, never a silent expansion (§2.1).
    fn allows_range(self) -> bool {
        matches!(self, RegFile::M68k)
    }

    /// The register UNIVERSE this CPU's contracts range over — the set a
    /// preserves-only bound clobbers the complement of, and the ⊤ a clobber-all
    /// callee reaches. 68k: `d0..d7`+`a0..a6` (`a7`/sp is stack discipline). Z80:
    /// the 8-bit halves + `ix`/`iy` (`sp` is the a7 analog, likewise excluded).
    pub fn universe(self) -> BTreeSet<String> {
        match self {
            RegFile::M68k => (0..8)
                .map(|n| format!("d{n}"))
                .chain((0..7).map(|n| format!("a{n}")))
                .collect(),
            RegFile::Z80 => Z80_UNITS.iter().map(|u| u.to_string()).collect(),
        }
    }
}

/// The 68k register units (`sp` canonicalizes to `a7`).
const M68K_UNITS: &[&str] = &[
    "d0", "d1", "d2", "d3", "d4", "d5", "d6", "d7", "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7",
];

/// The Z80 register universe: 8-bit halves + index units (`sp` excluded — stack
/// discipline, the a7 analog; `i`/`r` are rung-1-only and never appear in a
/// contract reglist).
const Z80_UNITS: &[&str] = &["a", "f", "b", "c", "d", "e", "h", "l", "ix", "iy"];

/// Expand + validate a contract reglist under register file `rf`, calling
/// `on_error` with a human `[contract.*]` reason for each ill-formed segment.
/// Each segment is a single register (a Z80 pair composes to its halves) or —
/// on 68k only — an inclusive `lo-hi` range. A name outside `rf`'s vocabulary is
/// `[contract.unknown-register]`; a range under a CPU that has none (Z80) is a
/// no-range-form reason. Returns the expanded UNIT set (canonical names).
pub fn expand_reglist(
    segs: &[(String, Option<String>)],
    rf: RegFile,
    mut on_error: impl FnMut(String),
) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for (lo, hi) in segs {
        match hi {
            None => match rf.units_of(lo) {
                Some(units) => set.extend(units.into_iter().map(str::to_string)),
                None => on_error(format!(
                    "[contract.unknown-register] `{lo}` is not a register of this CPU"
                )),
            },
            Some(h) => {
                if !rf.allows_range() {
                    on_error(format!(
                        "[contract.reglist-range] `{lo}-{h}`: Z80 reglists enumerate — there is no \
                         ordinal range form (a pair name expands to its halves instead)"
                    ));
                    continue;
                }
                // 68k ordinal range (the movem mask). The frozen production path
                // owns the corpus 68k ranges; this seam supports them so the
                // recognizer is genuinely CPU-parametric.
                let (Some(lb), Some(hb)) = (m68k_ordinal(lo), m68k_ordinal(h)) else {
                    on_error(format!(
                        "[contract.unknown-register] `{lo}-{h}` has a non-register endpoint"
                    ));
                    continue;
                };
                if hb < lb {
                    on_error(format!(
                        "[contract.reglist-range] the range `{lo}-{h}` is reversed"
                    ));
                    continue;
                }
                for b in lb..=hb {
                    set.insert(m68k_ordinal_name(b));
                }
            }
        }
    }
    set
}

/// A 68k register spelling to its movem ordinal (`d0`..`d7` = 0..7, `a0`..`a7`
/// = 8..15, `sp`→`a7`), or `None` if not a 68k register.
fn m68k_ordinal(name: &str) -> Option<u8> {
    let canon = if name == "sp" { "a7" } else { name };
    M68K_UNITS.iter().position(|u| *u == canon).map(|p| p as u8)
}

/// The inverse of [`m68k_ordinal`].
fn m68k_ordinal_name(bit: u8) -> String {
    if bit < 8 {
        format!("d{bit}")
    } else {
        format!("a{}", bit - 8)
    }
}
