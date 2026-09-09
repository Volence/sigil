//! charset: AS's code page, the character-to-byte step.
//!
//! A source that spells its text in the game's own font rather than in ASCII
//! says so with `charset`, and every character it writes after that is
//! translated through a 256-entry table on its way to a byte. Sonic 1's level
//! select menu is written in the level-select font: `G` is `$17`, not `$47`,
//! and the 504 bytes between `LevelMenuText` and the `charset` reset are all
//! spelled that way.
//!
//! # What the code page is and is not
//!
//! It is exactly one step: the character-to-byte translation, and nothing
//! stacked on top of it. `dc.b "AB"` maps each character and emits the mapped
//! bytes; a string in an EXPRESSION maps each character and then packs the
//! MAPPED bytes big-endian (see `expr::string_to_int`). The packing is
//! unchanged by a `charset`; only what goes into it moves.
//!
//! The index into the table is the SOURCE character, so a mapping is applied
//! once and never chained: after `charset 'A','X',$11`, `dc.b "A"` is `$11`,
//! and `$11` does not then get looked up again.
//!
//! # Semantics, measured against asl 1.42 Beta Bld 212
//!
//! Every rule below is a live-probe reading from
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`. The probe sources and their listings are
//! quoted in `docs/superpowers/notes/2026-09-09-as-charset.md`; every value used
//! here was read out of a run that exited 0, because this asl build substitutes
//! stable-but-invented answers on shapes it declines.
//!
//! ## The forms
//!
//! ```text
//!   charset                     reset the whole page to the identity
//!   charset SRC, TGT            TGT integer: map[SRC] = TGT
//!   charset SRC, "str"          map[SRC+i] = the RAW byte of str[i]
//!   charset LO, HI, BASE        map[LO+i] = (BASE+i) mod 256, i in 0..=HI-LO
//! ```
//!
//! One operand, or four or more, is asl's `wrong number of operands`.
//!
//! ## Where the operands are evaluated
//!
//! `SRC`, `LO`, `HI` and `BASE` are ordinary integer expressions, which means a
//! character literal written there is ITSELF translated through the page that is
//! live at that moment. This is the one rule a reader is most likely to guess
//! wrong, and it is measured rather than assumed:
//!
//! ```text
//!       4/       0 :                     	charset 'A',$11
//!       5/       0 : 11                  	dc.b 'A'
//!       6/       1 :                     	charset 'A',$20
//!       7/       1 : 11                  	dc.b 'A'
//! ```
//!
//! The second `charset` does NOT move `A`. By the time it runs, `'A'` evaluates
//! to `$11`, so it remaps source index `$11` and leaves `$41` alone. Spelling
//! the same line with a raw index does move it (`charset $41,$20` -> `dc.b 'A'`
//! is `20`), which is the control proving the mechanism rather than the outcome.
//!
//! The one place a character is NOT translated is the STRING target of the
//! two-operand form: `charset $7A,$05` then `charset 'a',"z"` leaves `dc.b "a"`
//! reading `7A`, the raw `z`, not `05`.
//!
//! ## Range rules
//!
//! `SRC`/`LO`/`HI` and an integer `TGT`/`BASE` must all be `0..=255`; asl says
//! `range overflow` otherwise (`charset $100,$11` and `charset $41,$1FF` and
//! `charset $41,-1` all draw it) and applies no mapping. `LO > HI` is `range
//! underflow`, again with no mapping applied. Within an accepted range the
//! TARGET wraps: `charset $41,$43,$FE` gives `dc.b "ABC"` = `FE FF 00`.
//!
//! ## Scope, and it has none
//!
//! The page is one global, mutable, source-ordered piece of assembler state.
//! Measured, all four ways:
//!
//! - it reaches INTO an `include`, and a `charset` inside the include leaks back
//!   OUT to the includer;
//! - a `charset` inside a MACRO body leaks out of the expansion;
//! - `save`/`restore` do NOT bracket it (`charset $41,$11; save; charset
//!   $41,$44; restore` leaves `dc.b "A"` reading `44`);
//! - it IS reset to the identity at the start of every pass. A file that ends
//!   with two changed characters live and forces a second pass still reads `41`
//!   for a `dc.b "A"` on its first line.
//!
//! The per-pass reset needs no code here: `Asm` (and with it [`AsmState`]) is
//! rebuilt for each pass, so a fresh identity page comes for free. It is
//! asserted end to end anyway, in
//! `tests/as_charset.rs::every_pass_starts_from_the_identity_page`, because
//! "for free" is a property of a construction site that a later refactor can
//! move.
//!
//! [`AsmState`]: crate::state::AsmState

// REASON: the module doc above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. This one is an INNER attribute because the listing sits in
// the module's own `//!` doc and there is no item to hang it on; it is scoped to
// this module, not to the crate.
#![allow(clippy::tabs_in_doc_comments)]

/// AS's 256-entry code page: the character-to-byte translation table.
///
/// Cheap to clone (256 bytes, no allocation) and cheap to compare, which is what
/// lets it sit in [`crate::state::AsmState`] and be passed by reference into the
/// expression parser without the parser having to own assembler state.
#[derive(Clone, PartialEq, Eq)]
pub struct CodePage {
    map: [u8; 256],
}

impl Default for CodePage {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::fmt::Debug for CodePage {
    /// Print the CHANGED entries only, in asl's own accounting (`STANDARD (N
    /// changed characters)`). A 256-entry array in a panic message buries the
    /// one or two entries a failing test is about.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let changed: Vec<String> = (0..256usize)
            .filter(|&i| self.map[i] != i as u8)
            .map(|i| format!("{i:02X}->{:02X}", self.map[i]))
            .collect();
        write!(f, "CodePage({} changed: {})", changed.len(), changed.join(" "))
    }
}

impl CodePage {
    /// The page every assembly starts each pass on: every character maps to its
    /// own byte. asl reports this as `STANDARD (0 changed characters)`.
    pub fn identity() -> Self {
        let mut map = [0u8; 256];
        for (i, slot) in map.iter_mut().enumerate() {
            *slot = i as u8;
        }
        CodePage { map }
    }

    /// `true` iff nothing has been remapped. Test-only: the assembler never
    /// asks, and `#[cfg(test)]` rather than an `allow(dead_code)` so a future
    /// non-test caller is a compile error here instead of silent dead weight.
    #[cfg(test)]
    pub fn is_identity(&self) -> bool {
        (0..256usize).all(|i| self.map[i] == i as u8)
    }

    /// Translate one source character to its byte.
    ///
    /// The index is `c as u8`, the same truncation the identity page always
    /// performed, kept verbatim so a source with no `charset` in it emits
    /// exactly the bytes it did before this table existed.
    pub fn map_char(&self, c: char) -> u8 {
        self.map[c as u8 as usize]
    }

    /// Reset every entry: the bare `charset` directive.
    pub fn reset(&mut self) {
        *self = CodePage::identity();
    }

    /// `map[src] = tgt`. Both are already range-checked by the caller.
    pub fn set(&mut self, src: u8, tgt: u8) {
        self.map[src as usize] = tgt;
    }
}
