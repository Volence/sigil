//! The Sigil linker: assign each section its LMA, compute label VMAs under
//! phase (VMA≠LMA), resolve every `Fixup` against the layout + symbol table,
//! materialize `Fill`/`Reserve`, and assemble the image.
//!
//! CPU-agnostic: consumes only `sigil-ir` types. Concrete backends are injected
//! upstream (the caller lowers instructions to `DataFragment`s first).

use sigil_ir::expr::Fold;
use sigil_ir::map::MemoryMap;
use sigil_ir::{
    Expr, Fixup, FixupKind, Fragment, LinkAssert, MsgPart, Section, SymbolTable, SymbolValue,
};
use sigil_span::{Diagnostic, Level, Span};

mod relax;
pub use relax::{asl_width_rule, resolve_layout, AbsWidth};

mod map_load;
pub use map_load::load_map;

mod listing;
pub use listing::{emit_listing, ListingSymbol};

/// One section's resolved bytes and where they load.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkedSection {
    pub name: String,
    pub lma: u32,
    pub bytes: Vec<u8>,
}

/// The result of a successful link: per-section resolved bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkedImage {
    pub sections: Vec<LinkedSection>,
}

impl LinkedImage {
    /// Look up a linked section by name.
    pub fn section(&self, name: &str) -> Option<&LinkedSection> {
        self.sections.iter().find(|s| s.name == name)
    }
}

/// Resolve `sections` into a `LinkedImage`, seeding the symbol table with
/// `stubs` (fixed external values, e.g. 68k leaf symbols in the harness).
/// Returns all diagnostics on failure.
///
/// A symbol name defined by two different sections is a hard `Error`
/// diagnostic (a real collision at full-ROM link). A section label resolving
/// against a `stubs` entry is legitimate and is not flagged.
pub fn link(sections: &[Section], stubs: &SymbolTable) -> Result<LinkedImage, Vec<Diagnostic>> {
    let mut diags: Vec<Diagnostic> = Vec::new();

    // Pass 1: build the symbol table — stubs first, then each section's labels
    // at their phased VMA (vma_origin + offset).
    let mut syms = stubs.clone();
    let mut defined_here: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for sec in sections {
        let origin = sec.vma_origin();
        for label in &sec.labels {
            if let Some(prev) = defined_here.insert(label.name.clone(), sec.name.clone()) {
                diags.push(diag(
                    format!(
                        "symbol `{}` redefined by section `{}` (already defined by section `{}`)",
                        label.name, sec.name, prev
                    ),
                    // TODO: null span — `Label` carries no span today. When labels
                    // gain a producer span, point this at the redefining label.
                    Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
                ));
            }
            syms.define(&label.name, SymbolValue::Int((origin + label.offset) as i64));
        }
    }

    // Pass 1b (R-T0.3): define each section's `equ_syms` — already folded to
    // `Expr::Int` by `resolve_layout` post-placement — as concrete
    // `SymbolValue::Int`, BEFORE any fixup is applied (Pass 2), so a cross-section
    // fixup can target an equ symbol. `resolve_layout` guarantees these exprs are
    // constant, so the fold below is trivial; the `Fold::Poison` arm is a
    // defensive internal error (an un-folded equ reaching link is a compiler bug,
    // not a source error). Duplicate names — equ-vs-equ and equ-vs-label — funnel
    // through the SAME `defined_here` dup-symbol channel as labels.
    for sec in sections {
        for eq in &sec.equ_syms {
            if let Some(prev) = defined_here.insert(eq.name.clone(), sec.name.clone()) {
                diags.push(diag(
                    format!(
                        "symbol `{}` redefined by section `{}` (already defined by section `{}`)",
                        eq.name, sec.name, prev
                    ),
                    eq.span,
                ));
                continue;
            }
            match eq.expr.fold(&|name| syms.resolve(name, None)) {
                Fold::Value(v) => syms.define(&eq.name, SymbolValue::Int(v)),
                Fold::Poison => diags.push(diag(
                    format!(
                        "internal: equ `{}` reached link() unfolded (resolve_layout must fold \
                         every equ to a constant before link)",
                        eq.name
                    ),
                    eq.span,
                )),
            }
        }
    }

    if diags.iter().any(|d| d.level == Level::Error) {
        return Err(diags);
    }

    // Pass 2: per section, copy image bytes and apply fixups.
    let mut linked = Vec::new();
    for sec in sections {
        let mut bytes = sec.image_bytes();
        let origin = sec.vma_origin();

        // Walk fragments to find each Data fragment's byte offset within the
        // section image, so fixup offsets and site VMAs are correct. This walk
        // is a write-cursor replay mirroring `Section::image_bytes`: `Org`
        // seeks the cursor (so a back-patched Data fragment's fixups, if any,
        // land at the right offset); Reserve leaves it untouched.
        let mut frag_img_off: u32 = 0; // offset within the image bytes (cursor)
        for frag in &sec.fragments {
            match frag {
                Fragment::Data(d) => {
                    for fx in &d.fixups {
                        // The WHOLE fixup (offset..offset+width) must fit within
                        // THIS fragment's own bytes; otherwise a multi-byte write
                        // would silently clobber the next fragment.
                        let width = fx.kind.byte_width() as usize;
                        if fx.offset as usize + width > d.bytes.len() {
                            diags.push(diag(
                                format!(
                                    "fixup at offset {} (width {}) exceeds fragment length {} in section {}",
                                    fx.offset,
                                    width,
                                    d.bytes.len(),
                                    sec.name
                                ),
                                d.span,
                            ));
                            continue;
                        }
                        let site_abs = frag_img_off + fx.offset; // offset within section image
                        let site_vma = origin + site_abs;
                        apply_fixup(&mut bytes, site_abs, site_vma, fx, &syms, sec.name.as_str(), d.span, &mut diags);
                    }
                    frag_img_off += d.bytes.len() as u32;
                }
                Fragment::Fill { count, .. } => frag_img_off += *count,
                Fragment::Reserve { .. } => {} // no image bytes
                Fragment::Org { target, .. } => frag_img_off = *target,
                Fragment::JmpJsrSym { .. } => {
                    unreachable!("JmpJsrSym must be lowered by resolve_layout before link")
                }
                Fragment::RelaxAbsSym { .. } => {
                    unreachable!("RelaxAbsSym must be lowered by resolve_layout before link")
                }
                Fragment::RelaxLadder { .. } => {
                    unreachable!("RelaxLadder must be lowered by resolve_layout before link")
                }
            }
        }

        linked.push(LinkedSection { name: sec.name.clone(), lma: sec.lma, bytes });
    }

    if diags.is_empty() {
        Ok(LinkedImage { sections: linked })
    } else {
        Err(diags)
    }
}

fn diag(message: String, span: Span) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary: span }
}

/// Build the post-relaxation symbol table (D-H.6): `stubs` plus every section's
/// labels at their phased VMA (`vma_origin + offset`) — IDENTICAL to `link()`'s
/// Pass-1 table. `sections` must already be `resolve_layout`-resolved (label
/// offsets shifted to their final layout), so the values the deferred link
/// assertions fold against are the SAME addresses `link()` resolved fixups
/// against. (`link()` rebuilds this internally rather than exporting it, keeping
/// its signature stable; the contract D-H.6 fixes is identical VALUES, which this
/// shared computation guarantees.)
fn build_symbol_table(sections: &[Section], stubs: &SymbolTable) -> SymbolTable {
    let mut syms = stubs.clone();
    for sec in sections {
        let origin = sec.vma_origin();
        for label in &sec.labels {
            syms.define(&label.name, SymbolValue::Int((origin + label.offset) as i64));
        }
    }
    // Task B3 (seam re-eval): also seed `equ_syms` — mirrors `link()`'s own
    // Pass 1b (R-T0.3). Without this, a `check_link_asserts` caller (the
    // deferred `ensure`/`ensure_fatal` path, D-H.4) could not resolve a
    // condition naming a symbol defined ONLY by an `equ` (no label) —
    // e.g. `ensure(extern("SOME_EQ") == N, ...)` — even though `link()`
    // resolves the identical symbol fine via its own separate table build.
    // `sections` here is ALWAYS `resolve_layout`'s output (every caller's
    // contract — see this fn's doc + every call site), so every `equ_syms`
    // entry's `expr` is already folded to `Expr::Int`; `.fold` with a
    // by-then-partially-seeded lookup still handles an equ that references
    // an EARLIER equ (equ-referencing-equ chains), exactly like `link()`.
    //
    // DELIBERATE DIVERGENCE from Pass 1b's error handling: Pass 1b raises a
    // loud "internal: equ ... reached link() unfolded" diagnostic on a Poison
    // fold; this function has no diagnostics channel, so the same broken
    // invariant is a `debug_assert!` here and a silent skip in release (the
    // symbol then stays undefined, and the assert's own Poison-condition arm
    // reports it — never a silent wrong value).
    for sec in sections {
        for eq in &sec.equ_syms {
            let fold = eq.expr.fold(&|name| syms.resolve(name, None));
            debug_assert!(
                matches!(fold, Fold::Value(_)),
                "internal: equ `{}` reached build_symbol_table() unfolded (resolve_layout must \
                 fold every equ to a constant before link)",
                eq.name
            );
            if let Fold::Value(v) = fold {
                syms.define(&eq.name, SymbolValue::Int(v));
            }
        }
    }
    syms
}

/// Evaluate a program's deferred link-time assertions (D-H.4/D-H.6) against the
/// post-`resolve_layout` symbol table, returning ONE `Error` diagnostic per
/// FAILING assert (ALL failures collected, never first-failure). `resolved` is
/// the `resolve_layout` output (final label offsets); `stubs` seeds the same
/// external symbols `link()` saw.
///
/// Per assert: its `cond` folds against the table. `0` → the build FAILS with the
/// rendered message (a lazy `{expr}` message part is folded here to its final
/// value — so `"overran at {here()}"` reports the REAL post-relaxation address).
/// Nonzero → the assert passes (no diagnostic). [`Fold::Poison`](Fold::Poison) in
/// the CONDITION — an unresolved symbol, which cannot happen if the anchor was
/// defined — is an internal-contract error naming the assert's span (never a
/// silent pass). `ensure` and `ensure_fatal` are identical in effect at link
/// (D-H.7): both are an `Error` that fails the build; `fatal` only colors wording.
pub fn check_link_asserts(
    resolved: &[Section],
    stubs: &SymbolTable,
    asserts: &[LinkAssert],
) -> Vec<Diagnostic> {
    if asserts.is_empty() {
        return Vec::new();
    }
    let syms = build_symbol_table(resolved, stubs);
    let lookup = |name: &str| syms.resolve(name, None);
    let mut out = Vec::new();
    for a in asserts {
        match a.cond.fold(&lookup) {
            // Nonzero → the guard holds; silent.
            Fold::Value(v) if v != 0 => {}
            // Zero → report at the assert's own severity (Error fails the
            // build; the [layout.odd-item] data check is Warning-tier).
            Fold::Value(_) => {
                out.push(Diagnostic {
                    level: a.level,
                    message: render_assert_message(&a.message, &lookup),
                    primary: a.span,
                });
            }
            // A Poison fold has two very different legitimate causes, so name
            // which one before diagnosing (Item C, seam re-eval): most of the
            // time this is a cross-seam `ensure`/`extern`/`bankid` condition
            // compiled STANDALONE (mt_bank.emp, sfx_bank.emp — no map/harness,
            // so the external symbol simply isn't in this link) or a plain
            // `extern()` typo — a SOURCE-level miss, not a compiler bug. Only an
            // unresolved `__here$...` anchor leaf (D-H.8's anonymous here()
            // anchor) is structurally unreachable today and stays an
            // internal-contract error.
            Fold::Poison => {
                let missing = unresolved_sym_leaves(&a.cond, &lookup);
                let message = if missing.iter().any(|n| n.starts_with("__here$")) {
                    "internal: deferred link assertion has an unresolvable condition \
                     (an anchor label was never defined) — this is a compiler bug in the \
                     `here()`-relaxation fix, not a source error"
                        .to_string()
                } else if missing.is_empty() {
                    // Poison from a non-symbol source (e.g. division/modulo by
                    // zero) reaching this far is still never a silent pass.
                    "link assertion condition is unresolvable at link time".to_string()
                } else {
                    let names: Vec<String> = missing.iter().map(|n| format!("`{n}`")).collect();
                    format!(
                        "link assertion condition references symbol(s) {} not defined in this \
                         link — expected when compiling a cross-seam module standalone; supply \
                         the map/harness composition that defines them",
                        names.join(", ")
                    )
                };
                out.push(Diagnostic { level: Level::Error, message, primary: a.span });
            }
        }
    }
    out
}

/// Walk `expr`'s `Sym` leaves against `lookup`, collecting the names that do
/// NOT resolve — deduplicated, in first-seen (stable, left-to-right) order.
/// Local to `sigil-link` rather than a general `sigil-ir::expr` utility: it is
/// tied to the diagnostic-reporting concern of "which names caused this
/// Poison", not a general property of `Expr` (the folder itself only needs
/// yes/no per-leaf — see `Expr::fold`); `sigil-link` already has one sibling
/// of this exact shape (`relax.rs::first_unresolved_sym`, first-only), so this
/// stays next to that convention rather than promoting either into `sigil-ir`.
fn unresolved_sym_leaves(expr: &Expr, lookup: &dyn Fn(&str) -> Option<i64>) -> Vec<String> {
    let mut out = Vec::new();
    collect_unresolved_sym_leaves(expr, lookup, &mut out);
    out
}

fn collect_unresolved_sym_leaves(expr: &Expr, lookup: &dyn Fn(&str) -> Option<i64>, out: &mut Vec<String>) {
    match expr {
        Expr::Int(_) => {}
        Expr::Sym(name) => {
            if lookup(name).is_none() && !out.iter().any(|n| n == name) {
                out.push(name.clone());
            }
        }
        Expr::Unary { operand, .. } => collect_unresolved_sym_leaves(operand, lookup, out),
        Expr::Binary { lhs, rhs, .. } => {
            collect_unresolved_sym_leaves(lhs, lookup, out);
            collect_unresolved_sym_leaves(rhs, lookup, out);
        }
    }
}

/// Render a deferred guard message (D-H.5) at link: `Text` parts verbatim, `Expr`
/// parts folded to their final integer (a `Poison` fold — an unresolved symbol in
/// a message subexpression — renders `<?>` rather than aborting the message).
fn render_assert_message(parts: &[MsgPart], lookup: &dyn Fn(&str) -> Option<i64>) -> String {
    let mut out = String::new();
    for p in parts {
        match p {
            MsgPart::Text(t) => out.push_str(t),
            MsgPart::Expr(e) => match e.fold(lookup) {
                Fold::Value(v) => out.push_str(&v.to_string()),
                Fold::Poison => out.push_str("<?>"),
            },
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn apply_fixup(
    bytes: &mut [u8],
    site_abs: u32,
    site_vma: u32,
    fx: &Fixup,
    syms: &SymbolTable,
    section: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    // Fold the target against the symbol table (global scope at link time; the
    // front-end will pre-qualify local names into fully-dotted `Sym`s in Plan 4).
    let value = match fx.target.fold(&|name| syms.resolve(name, None)) {
        Fold::Value(v) => v,
        Fold::Poison => {
            // Name the dangling leaves: a compound target (`Main - ObjCodeBase`,
            // the objroutine word) with one misspelled symbol should say WHICH
            // name dangled, same as the bare-Sym arm (tranche-6 probe finding).
            let lookup = |name: &str| syms.resolve(name, None);
            let missing = unresolved_sym_leaves(&fx.target, &lookup);
            let what = match &fx.target {
                Expr::Sym(name) => format!("symbol `{name}`"),
                _ if !missing.is_empty() => {
                    let names: Vec<String> = missing.iter().map(|n| format!("`{n}`")).collect();
                    format!("target expression (dangling symbol(s) {})", names.join(", "))
                }
                _ => "target expression".to_string(),
            };
            diags.push(diag(
                format!("unresolved {what} for fixup in section {section} at offset {site_abs}"),
                span,
            ));
            return;
        }
    };

    // The target's user-facing name for range/escape messages: a pc-relative
    // target that's out of reach is almost always cross-section/cross-seam,
    // so naming WHAT the code was reaching for pays more than the distance
    // alone (tranche-2 T1 review follow-up).
    let target_name = match &fx.target {
        Expr::Sym(name) => format!(" to `{name}`"),
        _ => String::new(),
    };

    match fx.kind {
        FixupKind::BankPtr16Le => {
            if (site_abs as usize) + 1 >= bytes.len() {
                diags.push(diag(
                    format!("BankPtr16Le fixup at offset {site_abs} would write past section end in section {section}"),
                    span,
                ));
                return;
            }
            let v = value as u16;
            let lo = (v & 0xFF) as u8;
            let hi = (v >> 8) as u8;
            bytes[site_abs as usize] = lo;
            bytes[(site_abs + 1) as usize] = hi;
        }
        FixupKind::BankPtr16Be => {
            // The 68k-section counterpart to BankPtr16Le: same windowed low-16
            // value, written big-endian (§7.2 / D-P4.7).
            if (site_abs as usize) + 1 >= bytes.len() {
                diags.push(diag(
                    format!("BankPtr16Be fixup at offset {site_abs} would write past section end in section {section}"),
                    span,
                ));
                return;
            }
            let v = value as u16;
            bytes[site_abs as usize] = (v >> 8) as u8;
            bytes[(site_abs + 1) as usize] = (v & 0xFF) as u8;
        }
        FixupKind::Z80JrRel8 => {
            if site_abs as usize >= bytes.len() {
                diags.push(diag(
                    format!("Z80JrRel8 fixup at offset {site_abs} would write past section end in section {section}"),
                    span,
                ));
                return;
            }
            // disp measured from the END of the 2-byte instruction. The opcode
            // is at site_abs-1; the instruction end VMA = (site_vma - 1) + 2.
            let inst_end_vma = (site_vma as i64 - 1) + 2;
            let disp = value - inst_end_vma;
            if !(-128..=127).contains(&disp) {
                diags.push(diag(
                    format!("jr/djnz displacement out of range ({disp}) in section {section}"),
                    span,
                ));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::Abs16Be => {
            // abs.w holds a sign-extended 16-bit address: the VMA must fit i16
            // (asl errors otherwise; matching that keeps us byte-exact).
            let v = value as i64;
            if !(-0x8000..=0x7FFF).contains(&v) && !(0xFF_8000..=0xFF_FFFF).contains(&(v & 0xFF_FFFF)) {
                diags.push(diag(
                    format!("value {v:#X} does not fit abs.w (16-bit sign-extended) in section {section}"),
                    span,
                ));
                return;
            }
            let w = (value & 0xFFFF) as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::ImmWord16Be => {
            // A `#imm16` word immediate holds the LOW 16 bits of a value whose
            // high 16 bits must be a consistent extension — all-zero (unsigned
            // value / objroutine offset in [0, 0xFFFF]) or all-one (sign-extended
            // RAM address like $FFFF9EDE). This is AS's word-immediate rule; it
            // is the UNION of Value16Be (rejects the address) and Abs16Be
            // (rejects the [0x8000, 0xFFFF] upper-unsigned half). Truncating to
            // u32 first canonicalizes both the sign-extended-i64 ($…FFFF9E8E)
            // and raw-u32 ($FFFF9E8E) representations of a RAM address.
            let hi = (value as u32) >> 16;
            if hi != 0 && hi != 0xFFFF {
                diags.push(diag(
                    format!(
                        "value {value:#X} does not fit a 16-bit immediate \
                         (high half neither zero- nor sign-extension) in section {section}"
                    ),
                    span,
                ));
                return;
            }
            let w = (value & 0xFFFF) as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::Abs32Be => {
            let w = value as u32;
            bytes[site_abs as usize] = (w >> 24) as u8;
            bytes[site_abs as usize + 1] = (w >> 16) as u8;
            bytes[site_abs as usize + 2] = (w >> 8) as u8;
            bytes[site_abs as usize + 3] = (w & 0xFF) as u8;
        }
        FixupKind::PcRel8 => {
            // disp measured from op+2; the disp byte sits at op+1 = site_vma.
            let disp = value - (site_vma as i64 + 1);
            // A 0x00 byte displacement is NOT a displacement on the 68000 — it is
            // the escape to the word form, so a `.s` branch to op+2 is unencodable.
            // Reject it loudly rather than silently writing the 0x00 word-form
            // escape (a desynced instruction). Reachable via an explicit `bra.s`
            // to the next instruction in .emp source; AS ports never take this
            // path (they resolve displacements before encoding, emitting no
            // PcRel8 fixups), so AS-port byte-exactness is untouched.
            if disp == 0 {
                diags.push(diag(
                    format!("bra.s/Bcc.s displacement{target_name} is 0 in section {section} — a 0x00 byte displacement is the 68000 word-form escape, not a branch to the next instruction (use .w, or pick a real target)"),
                    span,
                ));
                return;
            }
            if !(-128..=127).contains(&disp) {
                diags.push(diag(format!("bra.s/Bcc.s displacement{target_name} out of range ({disp}) in section {section}"), span));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::PcRelDisp16 => {
            // disp measured from the extension word's own VMA = site_vma.
            let disp = value - site_vma as i64;
            if !(-0x8000..=0x7FFF).contains(&disp) {
                diags.push(diag(format!("(d16,PC)/bra.w displacement{target_name} out of range ({disp}) in section {section}"), span));
                return;
            }
            let w = disp as i16 as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::PcRelDisp8 => {
            // `(d8,PC,Xn)`: the disp8 is the LOW byte of the brief extension
            // word, but the 68k PC reference is the extension word's own VMA —
            // one byte BEFORE this disp byte. So `disp = target - (site_vma - 1)`
            // (asl-verified: `move.w Tbl(pc,d0.w),d1` at $0, `Tbl` at $4 →
            // ext word `0002`, i.e. disp = 4 - (3 - 1) = 2). The fixup offset
            // points at the disp byte itself.
            let disp = value - (site_vma as i64 - 1);
            if !(-128..=127).contains(&disp) {
                diags.push(diag(format!("(d8,PC,Xn) displacement{target_name} out of range ({disp}) in section {section}"), span));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::HeaderChecksum => {
            diags.push(diag("HeaderChecksum is a post-image pass, not an in-fragment fixup".into(), span));
        }
        FixupKind::RelWord16Be => {
            // A self-relative signed word offset (`dc.w Target-Base`): `target`
            // is a symbol difference, so `value` is already the offset. Range i16.
            if !(-0x8000..=0x7FFF).contains(&value) {
                diags.push(diag(
                    format!("offset out of signed-word range ({value}) in section {section}"),
                    span,
                ));
                return;
            }
            let w = value as i16 as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        // General link-expr VALUE kinds (S2-D13f / R7m.4): write the folded
        // integer VERBATIM after an UNSIGNED-window range check
        // (`0 ≤ value < 2^(8·width)`). A fold outside that window — including a
        // negative value — is an Error naming the section, the folded value, and
        // the window. Endianness is the kind's own (68k=Be, Z80=Le); width 1 is
        // order-neutral. Deliberately NOT an address range check (no sign
        // extension, no masking) — that is the address kinds' job.
        FixupKind::Value8 => write_value(bytes, site_abs, value, 1, false, section, span, diags),
        FixupKind::Value16Be => write_value(bytes, site_abs, value, 2, false, section, span, diags),
        FixupKind::Value16Le => write_value(bytes, site_abs, value, 2, true, section, span, diags),
        FixupKind::Value32Be => write_value(bytes, site_abs, value, 4, false, section, span, diags),
        FixupKind::Value32Le => write_value(bytes, site_abs, value, 4, true, section, span, diags),
    }
}

/// Write a general link-expr VALUE (S2-D13f / R7m.4): range-check `value`
/// against the UNSIGNED window `0 ≤ value < 2^(8·width)`, then write its low
/// `width` bytes in the requested byte order (`little` = Z80). A fold outside
/// the window (including negative) is an Error naming the section, the folded
/// value, and the window — NOT a silent truncation or sign-extension.
#[allow(clippy::too_many_arguments)]
fn write_value(
    bytes: &mut [u8],
    site_abs: u32,
    value: i64,
    width: u8,
    little: bool,
    section: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    let w = width as usize;
    if (site_abs as usize) + w > bytes.len() {
        diags.push(diag(
            format!("value fixup at offset {site_abs} (width {w}) would write past section end in section {section}"),
            span,
        ));
        return;
    }
    // Unsigned window: `2^64` overflows i64, so bound width-8 separately — but a
    // value cell is never wider than 4, so `1 << (8·width)` is always in range.
    let limit: i128 = 1i128 << (8 * w as u32);
    let v = value as i128;
    if v < 0 || v >= limit {
        diags.push(diag(
            format!(
                "[value.out-of-range] link-expr value {v} does not fit an unsigned {}-bit cell (0..{}) in section {section}",
                8 * w,
                limit - 1
            ),
            span,
        ));
        return;
    }
    let u = value as u64;
    // Big-endian: high byte first. Little-endian: low byte first.
    for i in 0..w {
        let shift = if little { 8 * i } else { 8 * (w - 1 - i) };
        bytes[site_abs as usize + i] = (u >> shift) as u8;
    }
}

/// Materialize a full contiguous image: place each section's bytes at its LMA,
/// filling all gaps (and the head) with `fill`. Sections must not overlap.
///
/// EMPTY sections are skipped: a pure-`ds`/Reserve section (RAM variable
/// declarations phased to `$FFFF0000`+) reserves address space and defines
/// labels but emits NO ROM bytes — asl/p2bin write no binary records for it.
/// It carries a physical-counter LMA that can legitimately alias a real code
/// section's range (both start near physical 0), so it must contribute nothing
/// to, and never be range-checked against, the image.
pub fn flatten(image: &LinkedImage, fill: u8) -> Vec<u8> {
    let end = image
        .sections
        .iter()
        .filter(|s| !s.bytes.is_empty())
        .map(|s| s.lma as usize + s.bytes.len())
        .max()
        .unwrap_or(0);
    let mut out = vec![fill; end];
    for s in &image.sections {
        if s.bytes.is_empty() {
            continue;
        }
        let start = s.lma as usize;
        out[start..start + s.bytes.len()].copy_from_slice(&s.bytes);
    }
    out
}

/// Like `flatten`, but errors if any two sections' `[lma, lma+len)` ranges
/// overlap (a mis-assigned LMA map would otherwise silently clobber bytes).
/// Empty (zero-byte) sections are excluded — they place no bytes, so they can
/// neither clobber nor overlap (see `flatten`).
pub fn flatten_checked(image: &LinkedImage, fill: u8) -> Result<Vec<u8>, String> {
    let mut ranges: Vec<(usize, usize, &str)> = image
        .sections
        .iter()
        .filter(|s| !s.bytes.is_empty())
        .map(|s| (s.lma as usize, s.lma as usize + s.bytes.len(), s.name.as_str()))
        .collect();
    ranges.sort_by_key(|r| r.0);
    for w in ranges.windows(2) {
        if w[0].1 > w[1].0 {
            return Err(format!("sections `{}` and `{}` overlap in the image", w[0].2, w[1].2));
        }
    }
    Ok(flatten(image, fill))
}

/// The single-image ROM output (`p2bin` + `fixheader` replacement):
/// validate each section against the map, place bytes at LMA, gap-fill with the
/// map default, append NOTHING (the `convsym` no-op), then apply the header
/// checksum as the final pass. The ROM ends at the last section byte — no
/// power-of-two padding.
pub fn emit_rom(image: &LinkedImage, map: &MemoryMap) -> Result<Vec<u8>, String> {
    for s in &image.sections {
        map.validate_section(&s.name, s.lma, s.bytes.len() as u32)?;
    }
    let mut rom = flatten_checked(image, map.fill)?;
    // convsym no-op: append nothing.
    apply_header_checksum(&mut rom); // Task 6
    Ok(rom)
}

/// Sega header checksum: 16-bit big-endian additive word-sum over `[0x200, EOF)`,
/// written big-endian at `0x18E`. The genuinely-last byte-mutating pass. An odd
/// trailing byte is summed as the high half of a word (low half 0x00).
pub fn apply_header_checksum(rom: &mut [u8]) {
    if rom.len() < 0x200 {
        return;
    }
    let mut sum: u16 = 0;
    let mut i = 0x200;
    while i + 1 < rom.len() {
        sum = sum.wrapping_add(((rom[i] as u16) << 8) | rom[i + 1] as u16);
        i += 2;
    }
    if i < rom.len() {
        sum = sum.wrapping_add((rom[i] as u16) << 8);
    }
    rom[0x18E] = (sum >> 8) as u8;
    rom[0x18F] = (sum & 0xFF) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{Cpu, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SectionPlacement, SymbolTable, SymbolValue};
    use sigil_span::{SourceId, Span};

    // ---- deferred link-time assertions (D-H.4/D-H.6) --------------------------

    /// A section defining an anchor label `A` at offset `off` in a vma:$8000 section.
    fn anchor_section(off: u32) -> Section {
        Section {
            name: "s".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x8000),
            lma: 0,
            labels: vec![Label { name: "A".into(), offset: off }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0; (off + 2) as usize],
                fixups: vec![],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    #[test]
    fn link_assert_passes_when_cond_nonzero() {
        // A at $8004. `A <= $9000` → $8004 <= $9000 → 1 (pass): no diagnostic.
        let secs = [anchor_section(4)];
        let cond = Expr::Binary {
            op: sigil_ir::expr::BinOp::Le,
            lhs: Box::new(Expr::Sym("A".into())),
            rhs: Box::new(Expr::Int(0x9000)),
        };
        let a = LinkAssert { cond, message: vec![MsgPart::Text("over".into())], fatal: true,
            level: sigil_span::Level::Error, span: span() };
        assert!(check_link_asserts(&secs, &SymbolTable::new(), &[a]).is_empty());
    }

    #[test]
    fn link_assert_fails_when_cond_zero_and_renders_message() {
        // A at $8004. `A <= $8000` → false → 0 (fail). The message folds `{A}`.
        let secs = [anchor_section(4)];
        let cond = Expr::Binary {
            op: sigil_ir::expr::BinOp::Le,
            lhs: Box::new(Expr::Sym("A".into())),
            rhs: Box::new(Expr::Int(0x8000)),
        };
        let msg = vec![
            MsgPart::Text("overran at ".into()),
            MsgPart::Expr(Expr::Sym("A".into())),
        ];
        let a = LinkAssert { cond, message: msg, fatal: true,
            level: sigil_span::Level::Error, span: span() };
        let ds = check_link_asserts(&secs, &SymbolTable::new(), &[a]);
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].level, Level::Error);
        // $8004 = 32772 decimal — the REAL post-relaxation address.
        assert!(ds[0].message.contains("overran at 32772"), "got: {}", ds[0].message);
    }

    #[test]
    fn link_assert_collects_all_failures() {
        let secs = [anchor_section(4)];
        let fail = |rhs: i64| LinkAssert {
            cond: Expr::Binary {
                op: sigil_ir::expr::BinOp::Le,
                lhs: Box::new(Expr::Sym("A".into())),
                rhs: Box::new(Expr::Int(rhs)),
            },
            message: vec![MsgPart::Text("x".into())],
            fatal: false,
            level: sigil_span::Level::Error,
            span: span(),
        };
        // Two failing asserts ($8004 <= $10 and <= $20 both false) → both reported.
        let ds = check_link_asserts(&secs, &SymbolTable::new(), &[fail(0x10), fail(0x20)]);
        assert_eq!(ds.len(), 2);
    }

    /// Item C (seam re-eval): a cond naming an ordinary undefined symbol is the
    /// LEGITIMATE cross-seam-standalone-compile case (mt_bank.emp/sfx_bank.emp
    /// compiled alone, or an `extern()`/`bankid()` typo) — NOT a compiler bug.
    /// The message must name the missing symbol, explain the standalone-compile
    /// cause, and must NOT claim "compiler bug"/"internal".
    #[test]
    fn link_assert_unresolved_cond_names_missing_symbol_with_standalone_guidance() {
        let secs = [anchor_section(4)];
        let a = LinkAssert {
            cond: Expr::Sym("Nope".into()),
            message: vec![MsgPart::Text("x".into())],
            fatal: false,
            level: sigil_span::Level::Error,
            span: span(),
        };
        let ds = check_link_asserts(&secs, &SymbolTable::new(), &[a]);
        assert_eq!(ds.len(), 1);
        assert!(ds[0].message.contains("Nope"), "got: {}", ds[0].message);
        assert!(
            ds[0].message.contains("standalone"),
            "expected standalone-compile guidance, got: {}",
            ds[0].message
        );
        assert!(
            !ds[0].message.contains("compiler bug"),
            "must not accuse the user's source of a compiler bug, got: {}",
            ds[0].message
        );
        assert!(
            !ds[0].message.to_lowercase().contains("internal"),
            "must not use internal-contract wording for a legitimate standalone-compile miss, got: {}",
            ds[0].message
        );
    }

    /// Item C companion: an unresolved `__here$<module>$<n>`-style anchor leaf
    /// IS structurally unreachable today (the anchor is always defined by the
    /// lowerer before a deferred assert can reference it) — reaching it means a
    /// genuine compiler bug, so the internal-contract wording must still fire.
    #[test]
    fn link_assert_unresolved_here_anchor_leaf_is_still_internal_contract_error() {
        let secs = [anchor_section(4)];
        let a = LinkAssert {
            cond: Expr::Sym("__here$test$0".into()),
            message: vec![MsgPart::Text("x".into())],
            fatal: false,
            level: sigil_span::Level::Error,
            span: span(),
        };
        let ds = check_link_asserts(&secs, &SymbolTable::new(), &[a]);
        assert_eq!(ds.len(), 1);
        assert!(ds[0].message.contains("internal"), "got: {}", ds[0].message);
        assert!(ds[0].message.contains("compiler bug"), "got: {}", ds[0].message);
    }

    /// Item C: a mixed condition (one resolvable symbol + one missing) must name
    /// ONLY the missing symbol — not the resolvable one.
    #[test]
    fn link_assert_unresolved_cond_names_only_the_missing_symbol_in_a_mix() {
        let secs = [anchor_section(4)];
        let cond = Expr::Binary {
            op: sigil_ir::expr::BinOp::Eq,
            lhs: Box::new(Expr::Sym("A".into())), // resolvable (anchor_section defines it)
            rhs: Box::new(Expr::Sym("StillMissing".into())),
        };
        let a = LinkAssert { cond, message: vec![MsgPart::Text("x".into())], fatal: false,
            level: sigil_span::Level::Error, span: span() };
        let ds = check_link_asserts(&secs, &SymbolTable::new(), &[a]);
        assert_eq!(ds.len(), 1);
        assert!(ds[0].message.contains("StillMissing"), "got: {}", ds[0].message);
        assert!(!ds[0].message.contains("`A`"), "must not name the resolvable symbol, got: {}", ds[0].message);
    }

    /// A section carrying an `equ` (already folded to `Expr::Int` by
    /// `resolve_layout`, as `check_link_asserts`' caller always supplies —
    /// see `equ_link.rs`) at the given value, no labels.
    fn equ_only_section(name: &str, eq_name: &str, value: i64) -> Section {
        Section {
            name: name.into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![sigil_ir::EquSym {
                name: eq_name.into(),
                expr: Expr::Int(value),
                span: span(),
            }],
        }
    }

    /// Task B3 (seam re-eval, extern() e2e probe): a deferred `LinkAssert`
    /// condition naming a symbol defined ONLY by an `equ` (no label) must
    /// resolve through `check_link_asserts`, exactly like `link()`'s own
    /// Pass 1b already resolves equ-vs-label symbols uniformly. Before this
    /// fix, `build_symbol_table` (used ONLY by `check_link_asserts`) seeded
    /// labels but not `equ_syms`, so an `extern("EQ")`-style deferred
    /// condition hit `Fold::Poison` and the SAME "anchor label was never
    /// defined" internal-contract error `link_assert_unresolved_cond_is_
    /// internal_contract_error` pins for a genuinely undefined symbol —
    /// wrongly, for a symbol that IS defined, just via `equ` not a label.
    #[test]
    fn link_assert_cond_resolves_an_equ_defined_symbol_not_just_labels() {
        let secs = [equ_only_section("defs", "SND_PROBE_EQ", 0x0B)];
        let cond = Expr::Binary {
            op: sigil_ir::expr::BinOp::Eq,
            lhs: Box::new(Expr::Sym("SND_PROBE_EQ".into())),
            rhs: Box::new(Expr::Int(0x0B)),
        };
        let a = LinkAssert { cond, message: vec![MsgPart::Text("mismatch".into())], fatal: false,
            level: sigil_span::Level::Error, span: span() };
        assert_eq!(
            check_link_asserts(&secs, &SymbolTable::new(), &[a]),
            Vec::<Diagnostic>::new(),
            "an equ-defined symbol in a LinkAssert condition must resolve, not Poison"
        );
    }

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    // Region B: defines SfxBlobWinTab at VMA base $8000 + offset $45F = $845F.
    fn region_b() -> Section {
        let frags = vec![
            // 0x45F bytes of filler so the label lands at offset 0x45F.
            Fragment::Fill { value: 0xAA, count: 0x45F, span: span() },
            // The table's first bytes (content irrelevant to this test).
            Fragment::Data(DataFragment { bytes: vec![0x9A, 0xD6], fixups: vec![], span: span() }),
        ];
        Section {
            name: "regionB".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "SfxBlobWinTab".to_string(), offset: 0x45F }],
            fragments: frags,
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    // Region A: `ld de,SfxBlobWinTab` = 11 <lo> <hi>, fixup at offset 1.
    fn region_a() -> Section {
        Section {
            name: "regionA".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x0000),
            lma: 0x400,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x11, 0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::BankPtr16Le,
                    offset: 1,
                    target: Expr::Sym("SfxBlobWinTab".to_string()),
                }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    #[test]
    fn cross_region_fixup_resolves_to_phased_vma_little_endian() {
        let linked = link(&[region_a(), region_b()], &SymbolTable::new()).unwrap();
        let a = linked.section("regionA").unwrap();
        // 11 5F 84  — $845F little-endian.
        assert_eq!(a.bytes, vec![0x11, 0x5F, 0x84]);
        assert_eq!(a.lma, 0x400);
    }

    #[test]
    fn dw_bank_pointer_from_functions_emits_little_endian() {
        // dw sfx_winptr(Sfx_33) with Sfx_33 stubbed to 0x6569A:
        //   (Sfx_33 & 0x7FFF) | 0x8000 = 0xD69A  → LE 9A D6.
        let mut stubs = SymbolTable::new();
        stubs.define("Sfx_33", SymbolValue::Int(0x6569A));
        let winptr = Expr::Binary {
            op: sigil_ir::expr::BinOp::Or,
            lhs: Box::new(Expr::Binary {
                op: sigil_ir::expr::BinOp::And,
                lhs: Box::new(Expr::Sym("Sfx_33".to_string())),
                rhs: Box::new(Expr::Int(0x7FFF)),
            }),
            rhs: Box::new(Expr::Int(0x8000)),
        };
        let sec = Section {
            name: "tab".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 0, target: winptr }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[sec], &stubs).unwrap();
        assert_eq!(linked.section("tab").unwrap().bytes, vec![0x9A, 0xD6]);
    }

    #[test]
    fn z80_jr_rel8_in_range_resolves() {
        // A `jr` at VMA $8000 targeting VMA $8000 → disp = 0 - ... let target be site+2 → 0.
        // Fragment: [0x18, 0x00] with Z80JrRel8 fixup at offset 1 targeting VMA 0x8002.
        let sec = Section {
            name: "code".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "here".to_string(), offset: 2 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x18, 0x00],
                fixups: vec![Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target: Expr::Sym("here".to_string()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        // site VMA of the disp byte's instruction = 0x8000; target = 0x8002; disp = 0x8002 - (0x8000 + 2) = 0.
        assert_eq!(linked.section("code").unwrap().bytes, vec![0x18, 0x00]);
    }

    #[test]
    fn z80_jr_rel8_out_of_range_diagnoses() {
        // Target 0x9000 from site 0x8000 → disp = 0x9000 - 0x8002 = 0xFFE (>127).
        let sec = Section {
            name: "code".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "far".to_string(), offset: 0x1000 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x18, 0x00],
                fixups: vec![Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target: Expr::Sym("far".to_string()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("out of range")), "got: {:?}", err);
    }

    // ---- general link-expr VALUE cells (S2-D13f / R7m.4) ---------------------

    /// A `ValueN` fixup writes the FOLDED integer verbatim in its byte order,
    /// AFTER an unsigned-window range check. `A` at $8004; target `A + 2` folds
    /// to $8006. Value16Be (68k) → 80 06; Value16Le (Z80) → 06 80; Value8 →
    /// low byte only.
    fn value_section(cpu: Cpu, kind: FixupKind, width: usize) -> Section {
        // A at offset 4 in a vma:$8000 section: leading 4 bytes then the value hole.
        let bytes = vec![0u8; 4 + width];
        Section {
            name: "s".into(),
            cpu,
            vma_base: Some(0x8000),
            lma: 0,
            labels: vec![Label { name: "A".into(), offset: 4 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes,
                fixups: vec![Fixup {
                    kind,
                    offset: 4,
                    target: Expr::Binary {
                        op: sigil_ir::expr::BinOp::Add,
                        lhs: Box::new(Expr::Sym("A".into())),
                        rhs: Box::new(Expr::Int(2)),
                    },
                }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    #[test]
    fn value16_be_folds_big_endian() {
        let sec = value_section(Cpu::M68000, FixupKind::Value16Be, 2);
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(&linked.section("s").unwrap().bytes[4..6], &[0x80, 0x06]);
    }

    #[test]
    fn value16_le_folds_little_endian() {
        // The R7m.5 Z80 probe: same fold ($8006), written little-endian.
        let sec = value_section(Cpu::Z80, FixupKind::Value16Le, 2);
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(&linked.section("s").unwrap().bytes[4..6], &[0x06, 0x80]);
    }

    #[test]
    fn value8_writes_verbatim_in_window() {
        // A value8 cell requires the fold to fit 0..255 (unsigned window, NOT a
        // truncation). `A >> 15` — the bank-id idiom — at $8004 folds to 1.
        let mut sec = value_section(Cpu::M68000, FixupKind::Value8, 1);
        if let Fragment::Data(d) = &mut sec.fragments[0] {
            d.fixups[0].target = Expr::Binary {
                op: sigil_ir::expr::BinOp::Shr,
                lhs: Box::new(Expr::Sym("A".into())),
                rhs: Box::new(Expr::Int(15)),
            };
        }
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("s").unwrap().bytes[4], 0x01);
    }

    #[test]
    fn value16_overflow_is_range_error() {
        // A at $8004; target `A + $8000` folds to $10004 ≥ $10000 → out of the
        // unsigned 16-bit window. Error naming the value, not a silent truncation.
        let mut sec = value_section(Cpu::M68000, FixupKind::Value16Be, 2);
        if let Fragment::Data(d) = &mut sec.fragments[0] {
            d.fixups[0].target = Expr::Binary {
                op: sigil_ir::expr::BinOp::Add,
                lhs: Box::new(Expr::Sym("A".into())),
                rhs: Box::new(Expr::Int(0x8000)),
            };
        }
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("[value.out-of-range]")
                && d.message.contains("65540")
                && d.message.contains("16-bit")),
            "got: {err:?}"
        );
    }

    #[test]
    fn value8_negative_fold_is_range_error() {
        // A value that folds NEGATIVE ($8004 - $8010 = -12) does not fit the
        // unsigned window — an Error, never a two's-complement wrap.
        let mut sec = value_section(Cpu::M68000, FixupKind::Value8, 1);
        if let Fragment::Data(d) = &mut sec.fragments[0] {
            d.fixups[0].target = Expr::Binary {
                op: sigil_ir::expr::BinOp::Sub,
                lhs: Box::new(Expr::Sym("A".into())),
                rhs: Box::new(Expr::Int(0x8010)),
            };
        }
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("[value.out-of-range]")), "got: {err:?}");
    }

    #[test]
    fn link_reports_duplicate_section_symbol() {
        let mk = |name: &str| Section {
            name: name.into(),
            cpu: Cpu::M68000,
            vma_base: Some(0),
            lma: 0,
            labels: vec![Label { name: "Dup".into(), offset: 0 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x4E, 0x71],
                fixups: vec![],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[mk("a"), mk("b")], &SymbolTable::new()).unwrap_err();
        assert!(
            err.iter()
                .any(|d| d.message.contains("Dup") && d.message.to_lowercase().contains("redefin")),
            "expected a redefinition diagnostic for `Dup`, got: {:?}",
            err
        );
    }

    #[test]
    fn pcrel_disp16_measured_from_extension_word() {
        // bra.w at op VMA 0x1000: [0x60,0x00, hi,lo]. Disp word at offset 2 (VMA 0x1002).
        // target 0x1080 → disp = 0x1080 - 0x1002 = 0x7E.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
            labels: vec![Label { name: "t".into(), offset: 0x80 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00, 0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym("t".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x60, 0x00, 0x00, 0x7E]);
    }

    #[test]
    fn pcrel8_measured_from_op_plus_two() {
        // bra.s at op VMA 0x2000: [0x60, disp]. disp byte at offset 1 (VMA 0x2001).
        // target 0x2010 → disp = 0x2010 - (0x2001 + 1) = 0x0E.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "t".into(), offset: 0x10 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("t".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        assert_eq!(link(&[sec], &SymbolTable::new()).unwrap().section("c").unwrap().bytes, vec![0x60, 0x0E]);
    }

    #[test]
    fn pcrel8_disp_zero_is_rejected() {
        // An explicit `bra.s` to the NEXT instruction: op at VMA 0x2000, disp byte
        // at offset 1 (VMA 0x2001), target = op+2 = 0x2002 → disp = 0x2002 -
        // (0x2001 + 1) = 0. The 0x00 byte is the 68000 word-form escape, so this
        // must be a loud link error, NOT a silently-written 0x00.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "next".into(), offset: 2 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("next".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("displacement to `next` is 0") && d.message.contains("word-form escape")),
            "got: {:?}",
            err
        );
    }

    #[test]
    fn pcrel8_out_of_range_diagnoses() {
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "far".into(), offset: 0x200 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("far".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("out of range")), "got: {:?}", err);
    }

    #[test]
    fn pcrel_out_of_range_messages_name_the_target_symbol() {
        // Tranche-2 T1 review follow-up (small-opens bundle, landed tranche
        // 3): a cross-section/cross-seam pc-relative target is almost always
        // the thing that's out of range, so the message must NAME it — the
        // distance and section alone don't say WHAT the code was reaching
        // for. Pin all three 68k pc-relative kinds.
        for (kind, opcode_bytes, offset) in [
            (FixupKind::PcRel8, vec![0x60u8, 0x00], 1u32),
            (FixupKind::PcRelDisp16, vec![0x60, 0x00, 0x00, 0x00], 2),
            (FixupKind::PcRelDisp8, vec![0x30, 0x3B, 0x00, 0x00], 3),
        ] {
            let sec = Section {
                name: "c".to_string(),
                cpu: Cpu::M68000,
                vma_base: None,
                lma: 0x2000,
                labels: vec![],
                fragments: vec![Fragment::Data(DataFragment {
                    bytes: opcode_bytes,
                    fixups: vec![Fixup {
                        kind,
                        offset,
                        target: Expr::Sym("VeryFarAway".into()),
                    }],
                    span: span(),
                })],
                placement: SectionPlacement::Pinned,
                reserved_span: 0,
                group: None,
                bank: None,
                equ_syms: Vec::new(),
            };
            let mut stubs = SymbolTable::new();
            stubs.define("VeryFarAway", sigil_ir::SymbolValue::Int(0x80_0000));
            let err = link(&[sec], &stubs).unwrap_err();
            assert!(
                err.iter().any(|d| {
                    d.message.contains("out of range") && d.message.contains("VeryFarAway")
                }),
                "{kind:?}: the out-of-range message must name the target symbol, got: {err:?}"
            );
        }
    }

    #[test]
    fn rel_word_16_be_writes_symbol_difference() {
        // base at offset 0, target at offset 6; word[0] = target - base = 6.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
            labels: vec![
                Label { name: "Base".into(), offset: 0 },
                Label { name: "Tgt".into(), offset: 6 },
            ],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::RelWord16Be,
                    offset: 0,
                    target: Expr::Binary {
                        op: sigil_ir::expr::BinOp::Sub,
                        lhs: Box::new(Expr::Sym("Tgt".into())),
                        rhs: Box::new(Expr::Sym("Base".into())),
                    },
                }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x00, 0x06]);
    }

    #[test]
    fn rel_word_16_be_negative_offset_two_complement() {
        // target BEFORE base: Tgt at 0, Base at 4 → offset -4 = 0xFFFC.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
            labels: vec![
                Label { name: "Tgt".into(), offset: 0 },
                Label { name: "Base".into(), offset: 4 },
            ],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::RelWord16Be,
                    offset: 0,
                    target: Expr::Binary {
                        op: sigil_ir::expr::BinOp::Sub,
                        lhs: Box::new(Expr::Sym("Tgt".into())),
                        rhs: Box::new(Expr::Sym("Base".into())),
                    },
                }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0xFF, 0xFC]);
    }

    #[test]
    fn rel_word_16_be_overflow_diagnoses() {
        // Base at 0, target at 0x8000 → +32768 exceeds +0x7FFF → error.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
            labels: vec![
                Label { name: "Base".into(), offset: 0 },
                Label { name: "Far".into(), offset: 0x8000 },
            ],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::RelWord16Be,
                    offset: 0,
                    target: Expr::Binary {
                        op: sigil_ir::expr::BinOp::Sub,
                        lhs: Box::new(Expr::Sym("Far".into())),
                        rhs: Box::new(Expr::Sym("Base".into())),
                    },
                }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("signed-word range")), "got: {:?}", err);
    }

    #[test]
    fn unresolved_target_diagnoses() {
        let sec = region_a(); // references SfxBlobWinTab, which no section defines here.
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("unresolved")), "got: {:?}", err);
    }

    #[test]
    fn fixup_offset_past_fragment_diagnoses() {
        // Fragment is 2 bytes, but the fixup is at offset 5. Target is resolvable,
        // so the offset overrun is the ONLY error.
        let mut stubs = SymbolTable::new();
        stubs.define("Ok", SymbolValue::Int(0x1234));
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::BankPtr16Le,
                    offset: 5,
                    target: Expr::Sym("Ok".to_string()),
                }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[sec], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("exceeds fragment length")), "got: {:?}", err);
    }

    #[test]
    fn bankptr16le_at_fragment_boundary_diagnoses() {
        // Two Data fragments; a 2-byte BankPtr16Le at offset 1 of the FIRST
        // fragment ([0x00,0x00]) would write its high byte into the second
        // fragment ([0xCC,0xDD]). The width-aware check must catch this loudly,
        // and the second fragment's 0xCC must NOT be clobbered.
        let mut stubs = SymbolTable::new();
        stubs.define("Ptr", SymbolValue::Int(0xBEEF));
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment {
                    bytes: vec![0x00, 0x00],
                    fixups: vec![Fixup {
                        kind: FixupKind::BankPtr16Le,
                        offset: 1,
                        target: Expr::Sym("Ptr".to_string()),
                    }],
                    span: span(),
                }),
                Fragment::Data(DataFragment { bytes: vec![0xCC, 0xDD], fixups: vec![], span: span() }),
            ],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[sec], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("exceeds fragment length")), "got: {:?}", err);
    }

    #[test]
    fn abs32be_writes_big_endian_target_vma() {
        // A 4-byte data fragment; Abs32Be fixup at offset 0 targeting VMA 0x00123456.
        let mut stubs = SymbolTable::new();
        stubs.define("T", SymbolValue::Int(0x0012_3456));
        let sec = Section {
            name: "s".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("T".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[sec], &stubs).unwrap();
        assert_eq!(linked.section("s").unwrap().bytes, vec![0x00, 0x12, 0x34, 0x56]);
    }

    #[test]
    fn abs16be_writes_big_endian_and_rejects_overflow() {
        let mut stubs = SymbolTable::new();
        stubs.define("Ok", SymbolValue::Int(0x1234));
        stubs.define("Big", SymbolValue::Int(0x1_0000)); // does not fit abs.w sign-extension
        let ok = Section {
            name: "ok".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Sym("Ok".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        assert_eq!(link(&[ok], &stubs).unwrap().section("ok").unwrap().bytes, vec![0x12, 0x34]);

        let bad = Section {
            name: "bad".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Sym("Big".into()) }],
                span: span(),
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = link(&[bad], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("abs.w")), "got: {:?}", err);
    }

    #[test]
    fn imm_word16_be_accepts_the_full_word_immediate_union() {
        // A `.w` link immediate accepts the UNION of unsigned `[0, 0xFFFF]` and a
        // sign-extended RAM address — the two cases the single-window kinds each
        // reject: `Value16Be` rejects the address (negative), `Abs16Be` rejects
        // the `[0x8000, 0xFFFF]` upper-unsigned half (a valid objroutine offset).
        let mut stubs = SymbolTable::new();
        stubs.define("Upper", SymbolValue::Int(0xF800)); // objroutine offset, upper bank half
        stubs.define("Ram", SymbolValue::Int(0xFFFF_9EDE)); // sign-extended RAM address
        stubs.define("Small", SymbolValue::Int(0x0F7C)); // ordinary small offset
        stubs.define("Over", SymbolValue::Int(0x1_9EDE)); // genuine >16-bit value

        let write = |sym: &str| {
            let sec = Section {
                name: "s".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
                fragments: vec![Fragment::Data(DataFragment {
                    bytes: vec![0, 0],
                    fixups: vec![Fixup {
                        kind: FixupKind::ImmWord16Be, offset: 0, target: Expr::Sym(sym.into()),
                    }],
                    span: span(),
                })],
                placement: SectionPlacement::Pinned,
                reserved_span: 0, group: None, bank: None, equ_syms: Vec::new(),
            };
            link(&[sec], &stubs).map(|img| img.section("s").unwrap().bytes.clone())
        };

        // Upper-unsigned half: Abs16Be would REJECT this; ImmWord16Be accepts.
        assert_eq!(write("Upper").unwrap(), vec![0xF8, 0x00]);
        // Sign-extended RAM address: Value16Be would REJECT this; low word stored.
        assert_eq!(write("Ram").unwrap(), vec![0x9E, 0xDE]);
        assert_eq!(write("Small").unwrap(), vec![0x0F, 0x7C]);
        // Genuine >16-bit value: neither zero- nor sign-extension → loud error.
        let err = write("Over").unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("16-bit immediate")),
            "got: {:?}", err
        );
    }

    #[test]
    fn unresolved_names_the_symbol() {
        let sec = region_a(); // references SfxBlobWinTab, undefined here.
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("SfxBlobWinTab")), "got: {:?}", err);
    }

    #[test]
    fn flatten_places_sections_at_lma_with_gap_fill() {
        let a = Section {
            name: "a".to_string(),
            cpu: Cpu::Z80,
            vma_base: None,
            lma: 2,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment { bytes: vec![0xAA, 0xBB], fixups: vec![], span: span() })],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let linked = link(&[a], &SymbolTable::new()).unwrap();
        // Bytes at LMA 2..4; positions 0,1 gap-filled with 0x00.
        assert_eq!(flatten(&linked, 0x00), vec![0x00, 0x00, 0xAA, 0xBB]);
    }

    #[test]
    fn flatten_checked_errors_on_overlap() {
        // Two sections: lma 0 len 4 ([0,4)) and lma 2 len 4 ([2,6)) overlap.
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".to_string(), lma: 0, bytes: vec![0x11, 0x22, 0x33, 0x44] },
                LinkedSection { name: "b".to_string(), lma: 2, bytes: vec![0x55, 0x66, 0x77, 0x88] },
            ],
        };
        let err = flatten_checked(&img, 0x00).unwrap_err();
        assert!(err.contains("overlap"), "got: {err}");
    }

    #[test]
    fn flatten_checked_ok_when_disjoint() {
        // lma 0 len 2 ([0,2)) and lma 2 len 2 ([2,4)) are adjacent, not overlapping.
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".to_string(), lma: 0, bytes: vec![0xAA, 0xBB] },
                LinkedSection { name: "b".to_string(), lma: 2, bytes: vec![0xCC, 0xDD] },
            ],
        };
        assert_eq!(flatten_checked(&img, 0x00).unwrap(), vec![0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn emit_rom_places_sections_and_validates_regions() {
        use sigil_ir::map::{MemoryMap, Region, RegionKind};
        let map = MemoryMap::new(
            vec![Region { name: "rom".into(), lma_base: 0, size: 0x1_0000, kind: RegionKind::Rom, vma_base: None }],
            0x00,
        );
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".into(), lma: 2, bytes: vec![0xAA, 0xBB] },
                LinkedSection { name: "b".into(), lma: 6, bytes: vec![0xCC] },
            ],
        };
        // head 0,1 filled; bytes at 2..4; gap at 4,5; byte at 6. Terminus = 7 (no padding).
        assert_eq!(emit_rom(&img, &map).unwrap(), vec![0x00, 0x00, 0xAA, 0xBB, 0x00, 0x00, 0xCC]);
    }

    #[test]
    fn emit_rom_rejects_section_outside_region() {
        use sigil_ir::map::{MemoryMap, Region, RegionKind};
        let map = MemoryMap::new(
            vec![Region { name: "rom".into(), lma_base: 0, size: 4, kind: RegionKind::Rom, vma_base: None }],
            0x00,
        );
        let img = LinkedImage { sections: vec![LinkedSection { name: "a".into(), lma: 8, bytes: vec![1] }] };
        assert!(emit_rom(&img, &map).is_err());
    }

    #[test]
    fn header_checksum_is_be_wordsum_over_200_to_eof_at_18e() {
        // Build a >0x200-byte ROM; put known words after 0x200; assert the
        // checksum word at 0x18E equals the BE word-sum over [0x200, EOF).
        let mut rom = vec![0u8; 0x210];
        rom[0x200] = 0x12;
        rom[0x201] = 0x34; // word 0x1234
        rom[0x202] = 0x00;
        rom[0x203] = 0x01; // word 0x0001
        // remaining 0x204..0x210 are zero words → sum = 0x1235.
        apply_header_checksum(&mut rom);
        assert_eq!(rom[0x18E], 0x12);
        assert_eq!(rom[0x18F], 0x35);
    }

    #[test]
    fn header_checksum_handles_odd_trailing_byte() {
        // Odd length: last lone byte forms a word with a 0x00 low half (BE hi-byte).
        let mut rom = vec![0u8; 0x203];
        rom[0x200] = 0x00;
        rom[0x201] = 0x10; // word 0x0010
        rom[0x202] = 0x05; // lone byte → word 0x0500
        apply_header_checksum(&mut rom);
        assert_eq!(((rom[0x18E] as u16) << 8) | rom[0x18F] as u16, 0x0510);
    }
}
