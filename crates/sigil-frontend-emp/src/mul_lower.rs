//! `mul_const` / `mul_bounded` — cost-model multiply lowering (the 2026-08-03
//! design, the jbra/jbsr precedent generalized to multiplication).
//!
//! Two 68k-only mnemonic-position constructs:
//!
//! ```text
//! mul_const   dN, #n            // dN.l = u16(dN.w) × n            (n comptime, 0..=$FFFF)
//! mul_const   dN, #n, dS        // same, dS = scratch (may be clobbered)
//! mul_bounded dD, dS2, #M       // dD.l = u16(dD.w) × u16(dS2.w), src ≤ M
//! mul_bounded dD, dS2, #M, dS   // same, dS = scratch (enables the loop candidate)
//! ```
//!
//! **Two contracts, one per name — the suffix IS part of the name** (the
//! 2026-08-05 sized-variant ruling):
//!
//! - **bare (LONG)**: `dst.l = zero-extended(dst.w) × n`, all 32 result bits
//!   valid (k·x ≤ $FFFF·$FFFF < 2³², so a chain in `.l` ops after a zero-extend
//!   is exact). A caller who needs the upper word spells this form.
//! - **`.w` (WORD)**: `dst.w = (u16(dst.w) × n) mod 2^16`; the **upper word of
//!   `dst` is UNDEFINED** after the construct. The multiply is total mod 2^16 —
//!   whether the product FITS a word is the AUTHOR's range obligation (carried
//!   where the corpus already carries it, the module `ensure`), never checked
//!   here. Loosening the upper word is deliberate: an unchanged-upper promise
//!   would exclude `mulu.w` (it writes all 32 bits) from the word candidate set,
//!   condemning a many-set-bits multiplier to an unbounded chain. The word
//!   chains the corpus adopts happen to leave the upper word untouched; callers
//!   may not rely on it, and the unit oracle seeds garbage to prove the freedom.
//!
//! `.l` (or any other suffix) is REFUSED (`[mul.size]`): bare already IS the
//! long form, so a second spelling of one meaning is a reader tax. The word
//! candidate set differs from the long set in exactly one structural way — there
//! is NO zero-extend seed anywhere in it (that is the entire economic point:
//! the word chains price what the sites compute, and win back the 2–12 cycles a
//! zero-extend would cost them against `mulu`).
//!
//! **Expansion point (the soundness decision):** a construct item expands into
//! ORDINARY instructions at [`expand_buf`], called when an evaluated
//! [`CodeBuf`] completes — BEFORE any contract analysis walks the items. The
//! clobber/preserves/out machinery therefore sees the chosen lowering's real
//! register writes (a chain's `move.l dN, dS` scratch write is an ordinary
//! detected write), the flag walk sees real CC writers, and the cycle-budget
//! walk prices real instructions. A raw `mul_*` mnemonic surviving into an
//! analysis would report ZERO writes (`instr_written_regs` knows no such
//! mnemonic) — silently-clobbered-scratch, the exact failure mode the design
//! names — so no raw item may outlive this pass on a CPU-resolved buffer.
//! (A cpu-less `asm {}` template keeps its raw items; the proc that splices it
//! expands them at ITS buffer's completion, and the item-position lowering path
//! carries a safety net in `lower_code_buf`.)
//!
//! **Cost model:** every candidate is priced by [`crate::m68k_cycles::instr_cost`]
//! — the SAME classifier the cycle-budget walk consumes, backed by the
//! M68000UM-sourced `sigil_isa::m68k_cycles` table (B′-3b). No second multiply
//! cost table exists here (the `pad_to_cycles`-vs-`instr_cost` drift class is a
//! rejected shape); byte lengths for tie-breaks come from the ENCODER
//! ([`crate::lower::encoded_len_m68k`]), not a shadow table. A table change is
//! therefore a byte-changing event for adopters and rides the golden gates.
//!
//! **Determinism:** fixed candidate generators, fixed enumeration order, total
//! preference (fewest cycles, then fewest bytes, then earliest candidate —
//! `mulu` enumerates first, so equal-cost equal-size ties resolve to `mulu`,
//! the design's "then mulu (simpler)" rule). Same input, same sequence, forever.
//!
//! **`mul_bounded` costs** compare WORST CASES (the engine budgets worst
//! frames): `mulu.w src,dst` at its UM ceiling versus the repeated-add loop at
//! src = M. An absent bound is unexpressible (the `#M` operand is mandatory) —
//! the house refusal stance. The bound is TRUSTED in v1: the design's DEBUG
//! `assert src ≤ M` is deferred (gap-ledger row) — note a violating src still
//! computes the correct product under either lowering; only the cost ceiling
//! claim breaks.
//!
//! **CC contract:** condition codes are clobbered-undefined after the splice
//! (lowerings differ; a different `n` reshapes the tail). Analyses reason about
//! the ACTUAL emitted tail — sound per build — but authors must not rely on
//! flags surviving the construct; the meaning of the tail's flags is not part
//! of the contract and changes with `n`.

use crate::m68k_cycles::instr_cost;
use crate::value::{CodeBuf, CodeItem, CodeOperand, ItemAuthor, Reg, Width};
use sigil_backend_m68k::m68k_cycles::CycleCost;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level, Span};

/// Which of the two contracts a construct spelling selects (the suffix IS the
/// name): bare `mul_const`/`mul_bounded` is [`MulWidth::Long`] (32-bit result),
/// the `.w` spelling is [`MulWidth::Word`] (low word, upper undefined). Any
/// other suffix is refused before this ever binds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MulWidth {
    Long,
    Word,
}

/// Whether `base` is one of the two multiply-construct mnemonic-position words.
/// Reserved on BOTH CPUs (like `dc`/`jbra`: a comptime fn named `mul_const` is
/// unreachable at statement position), with the Z80 refusal at expansion.
pub(crate) fn is_mul_mnemonic(base: &str) -> bool {
    matches!(base, "mul_const" | "mul_bounded")
}

/// Expand every `mul_const`/`mul_bounded` item of `buf` into its chosen
/// lowering. `cpu == None` (a raw `asm {}` template) leaves the buffer
/// untouched — the CPU-resolved buffer that eventually splices or lowers it
/// expands the items then. On Z80 each construct item is a `[mul.non-68k]`
/// error and is dropped (the error fails the build; no silent bytes).
///
/// `module` + `counter` mint the loop candidate's label names
/// (`$mul${module}${c}$loop`): the module id spans files (the hygiene model's
/// own convention — `$m$asm0$wait` embeds it for the same reason) and the
/// caller threads the evaluator's monotonic instantiation counter within one,
/// so labels are unique program-wide.
pub(crate) fn expand_buf(
    buf: &mut CodeBuf,
    cpu: Option<Cpu>,
    module: &str,
    counter: &mut u32,
) -> Vec<Diagnostic> {
    let Some(cpu) = cpu else { return Vec::new() };
    if !buf.items.iter().any(
        |i| matches!(i, CodeItem::Instr { mnemonic, .. } if is_mul_mnemonic(mnemonic)),
    ) {
        return Vec::new();
    }
    let mut diags = Vec::new();
    let items = std::mem::take(&mut buf.items);
    for item in items {
        match item {
            CodeItem::Instr { ref mnemonic, size, ref ops, span, ref as_type, ref author, .. }
                if is_mul_mnemonic(mnemonic) =>
            {
                if cpu == Cpu::Z80 {
                    diags.push(non_68k_err(span, mnemonic));
                    continue;
                }
                // `as T` is dispatch-bound sugar for typed transfers; on a
                // multiply construct it means nothing — refuse rather than
                // silently discard a spelled annotation.
                if as_type.is_some() {
                    diags.push(err(
                        span,
                        format!("[mul.operands] `{mnemonic}` takes no `as` dispatch bound"),
                    ));
                    continue;
                }
                match expand_item(mnemonic, size, ops, span, module, counter, author) {
                    Ok(expanded) => buf.items.extend(expanded),
                    Err(d) => diags.push(d),
                }
            }
            other => buf.items.push(other),
        }
    }
    diags
}

/// The one spelling of the Z80 refusal — both expansion sites (the choke point
/// and the `lower_code_buf` safety net) emit THIS diagnostic, so the text
/// cannot drift by path.
pub(crate) fn non_68k_err(span: Span, mnemonic: &str) -> Diagnostic {
    err(
        span,
        format!(
            "[mul.non-68k] `{mnemonic}` is 68000-only in v1 (the Z80 variant is a \
             recorded ledger row awaiting a consumer)"
        ),
    )
}

/// Expand ONE construct item into its chosen lowering — the pure decision
/// function (also the `lower_code_buf` safety net for item-position streams no
/// evaluator completed). Deterministic: same mnemonic/operands, same items —
/// up to the minted loop-label names, which carry `module` + `counter`.
///
/// `author` is the CONSTRUCT ITEM's authorship, propagated onto every emitted
/// instruction (via [`crate::value::reauthor_user_items`], the splice-boundary
/// helper). The §2 invariant — authorship never exempts, it REDIRECTS — rules
/// out a compiler author here: the operand registers are the AUTHOR's choice,
/// so the expansion's writes are that author's obligations exactly as the
/// construct line's were (a `User` line's scratch clobber stays charged to the
/// proc — the clobber-lint pins depend on it; a `Splice`-carried line keeps
/// its template for the `-> Code` contract parcel). The compiler chose only
/// the SPELLING, and the spelling carries no obligation the containing proc's
/// contracts do not already own.
pub(crate) fn expand_item(
    mnemonic: &str,
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    module: &str,
    counter: &mut u32,
    author: &ItemAuthor,
) -> Result<Vec<CodeItem>, Diagnostic> {
    let width = match size {
        None => MulWidth::Long,
        Some(Width::W) => MulWidth::Word,
        Some(other) => {
            let sfx = width_suffix(other);
            // `.l` would be a DUPLICATE of the long form (bare already means it);
            // `.b`/`.s` would be a NEW, unratified narrow contract — different
            // refusals for different reasons.
            let tail = if other == Width::L {
                "there is no `.l` — bare already IS the long form, so a second name \
                 for one meaning is a reader tax"
            } else {
                "there is no `.b`/`.s` — a byte- or short-width multiply would be an \
                 unratified THIRD contract, not a duplicate spelling (no consumer yet)"
            };
            return Err(err(
                span,
                format!(
                    "[mul.size] `{mnemonic}.{sfx}` — bare `{mnemonic}` IS the LONG form \
                     (dst.l = u16(dst.w) × n, all 32 bits) and `.w` is the WORD form \
                     (dst.w = the low word of the product; the upper word is undefined); \
                     {tail}"
                ),
            ));
        }
    };
    let mut items = match mnemonic {
        "mul_const" => expand_mul_const(ops, span, width)?,
        "mul_bounded" => expand_mul_bounded(ops, span, module, counter, width)?,
        _ => unreachable!("guarded by is_mul_mnemonic"),
    };
    // Generators stamp `User`; the splice-boundary helper lifts that to the
    // construct item's own authorship (a no-op when it IS `User`).
    crate::value::reauthor_user_items(&mut items, author);
    Ok(items)
}

// ---- mul_const ----------------------------------------------------------

fn expand_mul_const(
    ops: &[CodeOperand],
    span: Span,
    width: MulWidth,
) -> Result<Vec<CodeItem>, Diagnostic> {
    let (dst, n, scratch) = match ops {
        [d, CodeOperand::Imm(n)] => (data_reg(d, "dst", span)?, *n, None),
        [d, CodeOperand::Imm(n), s] => (
            data_reg(d, "dst", span)?,
            *n,
            Some(data_reg(s, "scratch", span)?),
        ),
        _ => {
            return Err(err(
                span,
                "[mul.operands] `mul_const` takes `dN, #const` or `dN, #const, dScratch`",
            ));
        }
    };
    if !(0..=0xFFFF).contains(&n) {
        return Err(err(
            span,
            format!(
                "[mul.const-range] `mul_const` multiplier must fit u16 (0..=$FFFF), got {n} — \
                 16×16→32 is the m68k multiply domain (a 32-bit multiplier is a deferred, \
                 rarer problem)"
            ),
        ));
    }
    let n = n as u32;
    if scratch == Some(dst) {
        return Err(err(
            span,
            "[mul.scratch-aliases] `mul_const` scratch must differ from dst (the chain \
             reads the original value from scratch after dst is reshaped)",
        ));
    }
    let candidates = mul_const_candidates(dst, n, scratch, span, width);
    Ok(choose(candidates))
}

/// The fixed candidate set for `mul_const`, in enumeration (tie-preference)
/// order — routed by the two contracts. Every LONG candidate computes exactly
/// dst.l = zx(dst.w) × n; every WORD candidate computes exactly
/// dst.w = (u16(dst.w) × n) mod 2^16 (upper word free).
fn mul_const_candidates(
    dst: Reg,
    n: u32,
    scratch: Option<Reg>,
    span: Span,
    width: MulWidth,
) -> Vec<Vec<CodeItem>> {
    match width {
        MulWidth::Long => mul_const_candidates_long(dst, n, scratch, span),
        MulWidth::Word => mul_const_candidates_word(dst, n, scratch, span),
    }
}

/// The LONG candidate set (32-bit result, zero-extend seeded).
fn mul_const_candidates_long(
    dst: Reg,
    n: u32,
    scratch: Option<Reg>,
    span: Span,
) -> Vec<Vec<CodeItem>> {
    let mut cands: Vec<Vec<CodeItem>> = Vec::new();
    // 1 — `mulu.w #n, dst`: always legal, needs no zero-extend (MULU reads only
    // dst.w and writes the full 32-bit product). Enumerates FIRST so equal
    // (cycles, bytes) ties resolve to it.
    cands.push(vec![instr("mulu", Some(Width::W), vec![imm(n), reg(dst)], span)]);
    // 2 — degenerates.
    if n == 0 {
        cands.push(vec![instr("moveq", None, vec![imm(0), reg(dst)], span)]);
    }
    if n == 1 {
        // In-place zero-extend: swap / clr.w / swap (no scratch demanded).
        cands.push(zx_in_place(dst, span));
    }
    // 3 — power of two: in-place zero-extend, then shift (scratch-free).
    if n >= 2 && n.is_power_of_two() {
        let mut items = zx_in_place(dst, span);
        items.extend(shift_run(n.trailing_zeros(), dst, span));
        cands.push(items);
    }
    // 4 — general chains (need the original value twice → scratch-gated;
    // design: no hidden register allocation).
    if let Some(s) = scratch {
        if n.count_ones() >= 2 {
            // 4a — left-to-right binary: seed both registers with zx(x), then
            // double per bit below the MSB, adding x on set bits. Doubling runs
            // are run-coded: a single double is `add.l dst,dst` (8 cycles, vs
            // `lsl.l #1`'s 10), a run of k ≥ 2 is `lsl.l` in chunks of ≤ 8.
            let mut items = seed_pair(dst, s, span);
            let msb = 31 - n.leading_zeros();
            let mut pending: u32 = 0;
            for bit in (0..msb).rev() {
                pending += 1;
                if n & (1 << bit) != 0 {
                    items.extend(shift_run(pending, dst, span));
                    pending = 0;
                    items.push(instr("add", Some(Width::L), vec![reg(s), reg(dst)], span));
                }
            }
            items.extend(shift_run(pending, dst, span));
            cands.push(items);
            // 4b — the ±1 factored form n = (2^a − 1) · 2^b (×63 = x·64 − x):
            // ((x << a) − x) << b. (The (2^a + 1)·2^b family is already the
            // LTR chain's own shape, so only the subtract form adds anything.)
            let b = n.trailing_zeros();
            let m = n >> b;
            if (m + 1).is_power_of_two() && m >= 3 {
                let a = (m + 1).trailing_zeros();
                let mut items = seed_pair(dst, s, span);
                items.extend(shift_run(a, dst, span));
                items.push(instr("sub", Some(Width::L), vec![reg(s), reg(dst)], span));
                items.extend(shift_run(b, dst, span));
                cands.push(items);
            }
        }
    }
    cands
}

/// The WORD candidate set (low-word result, upper word free). Differs from the
/// long set in exactly one structural way: NO zero-extend seed anywhere. Word
/// ops (`lsl.w`/`add.w`/`sub.w`/`move.w`) leave dst's upper word untouched,
/// which is why a word chain beats `mulu.w` at the corpus's stride multipliers —
/// there is no zero-extend to pay for. The winner at those strides is the
/// left-to-right chain (5c); the corpus spells every one of them
/// `mul_const.w` and takes whatever this set prices cheapest.
fn mul_const_candidates_word(
    dst: Reg,
    n: u32,
    scratch: Option<Reg>,
    span: Span,
) -> Vec<Vec<CodeItem>> {
    let mut cands: Vec<Vec<CodeItem>> = Vec::new();
    // 1 — `mulu.w #n, dst`: always legal. It writes all 32 bits, licensed here
    // by the undefined-upper contract; enumerates FIRST so equal (cycles, bytes)
    // ties resolve to it.
    cands.push(vec![instr("mulu", Some(Width::W), vec![imm(n), reg(dst)], span)]);
    // 2 — n = 0 → `clr.w dst` (the low word is zero; upper left free).
    if n == 0 {
        cands.push(vec![instr("clr", Some(Width::W), vec![reg(dst)], span)]);
    }
    // 3 — n = 1 → ZERO instructions. The word result is already in dst.w; the
    // honest lowering of ×1 mod 2^16 is nothing.
    if n == 1 {
        cands.push(Vec::new());
    }
    // 4 — n = 2^k → `lsl.w` run(s) on dst directly (word shift, no seed).
    if n >= 2 && n.is_power_of_two() {
        cands.push(shift_run_word(n.trailing_zeros(), dst, span));
    }
    // 5 — scratch chains (need the original value twice → scratch-gated; a
    // chain needs ≥ 2 set bits, so 0/1/2^k never reach here — and 5b's
    // `n >> b` below is safe only under that guard).
    if let Some(s) = scratch {
        if n.count_ones() >= 2 {
            // 5b — the ±1 factored form n = (2^a − 1)·2^b (×63 = x·64 − x):
            // `move.w D,S / lsl.w #a,D / sub.w S,D / lsl.w #b,D`.
            let b = n.trailing_zeros();
            let m = n >> b;
            if (m + 1).is_power_of_two() && m >= 3 {
                let a = (m + 1).trailing_zeros();
                let mut items =
                    vec![instr("move", Some(Width::W), vec![reg(dst), reg(s)], span)];
                items.extend(shift_run_word(a, dst, span));
                items.push(instr("sub", Some(Width::W), vec![reg(s), reg(dst)], span));
                items.extend(shift_run_word(b, dst, span));
                cands.push(items);
            }
            // 5c — general left-to-right binary (word ops over the seed
            // `move.w D,S`; S holds the original throughout), a candidate at ≥ 2
            // set bits. At n with exactly TWO set bits it is the cheapest chain
            // this set generates (5b applies there only at n = 3·2^b, where 5c
            // wins by 2 cycles), which is why it is the lowering of the corpus's
            // stride multipliers: 32/32/34 cycles at ×66/×80/×160, against
            // `mulu.w`'s 46. That is the limit of the claim — at ≥ 3 set bits 5b
            // is cheaper wherever it applies with m ≥ 7, e.g. n = 63, where 5b
            // costs 26 against 5c's 64. At a two-power n = 2^a + 2^b it subsumes
            // the two-power sum
            // `move.w D,S / lsl.w #a,D / lsl.w #b,S / add.w S,D`: identical
            // bytes at b = 0 and strictly cheaper for every b ≥ 1, which
            // `word_two_power_arm_is_dominated_by_ltr` pins over the whole
            // 2-set-bit domain. The oracle executes this arm across the 2-bit
            // strides and the ≥ 3-bit multipliers (n = 11/19 in NS) with
            // garbage-upper seeds.
            if n.count_ones() >= 2 {
                let mut items =
                    vec![instr("move", Some(Width::W), vec![reg(dst), reg(s)], span)];
                let msb = 31 - n.leading_zeros();
                let mut pending: u32 = 0;
                for bit in (0..msb).rev() {
                    pending += 1;
                    if n & (1 << bit) != 0 {
                        items.extend(shift_run_word(pending, dst, span));
                        pending = 0;
                        items.push(instr("add", Some(Width::W), vec![reg(s), reg(dst)], span));
                    }
                }
                items.extend(shift_run_word(pending, dst, span));
                cands.push(items);
            }
        }
    }
    cands
}

// ---- mul_bounded --------------------------------------------------------

fn expand_mul_bounded(
    ops: &[CodeOperand],
    span: Span,
    module: &str,
    counter: &mut u32,
    width: MulWidth,
) -> Result<Vec<CodeItem>, Diagnostic> {
    // The accumulate runs at the result width: `.l` for the long contract, `.w`
    // for the word contract (upper word free). Everything else — the mulu.w
    // candidate, the loop structure, the worst-vs-worst comparison — is shared.
    let acc_width = match width {
        MulWidth::Long => Width::L,
        MulWidth::Word => Width::W,
    };
    let (dst, src, bound, scratch) = match ops {
        [d, s, CodeOperand::Imm(m)] => {
            (data_reg(d, "dst", span)?, data_reg(s, "src", span)?, *m, None)
        }
        [d, s, CodeOperand::Imm(m), sc] => (
            data_reg(d, "dst", span)?,
            data_reg(s, "src", span)?,
            *m,
            Some(data_reg(sc, "scratch", span)?),
        ),
        _ => {
            return Err(err(
                span,
                "[mul.operands] `mul_bounded` takes `dDst, dSrc, #bound` or \
                 `dDst, dSrc, #bound, dScratch` — the bound is mandatory (an \
                 unbounded operand's cost is undecidable: refusal, not a guess)",
            ));
        }
    };
    if !(0..=0xFFFF).contains(&bound) {
        return Err(err(
            span,
            format!(
                "[mul.const-range] `mul_bounded` bound must fit u16 (0..=$FFFF), got {bound}"
            ),
        ));
    }
    let bound = bound as u32;
    if scratch == Some(dst) || scratch == Some(src) {
        return Err(err(
            span,
            "[mul.scratch-aliases] `mul_bounded` scratch must differ from dst and src \
             (the loop candidate holds the multiplicand there while dst accumulates \
             and src counts down)",
        ));
    }

    // Candidate 1 — `mulu.w src, dst`: the UM ceiling (70, data-dependent).
    // Worst-vs-worst is the comparison (the engine budgets worst frames).
    let mulu_items = vec![instr("mulu", Some(Width::W), vec![reg(src), reg(dst)], span)];
    let mulu_cost = seq_worst_cycles(&mulu_items)
        .expect("mulu.w Dn,Dn is table-priced");

    // Candidate 2 — the repeated-add loop, worst case at src = bound. Needs a
    // scratch (three live values: multiplicand, counter, accumulator) and a
    // src distinct from dst (src IS the counter). src ends clobbered — the
    // construct's contract says src is undefined after the splice under EITHER
    // lowering, so the choice cannot change what callers may rely on.
    let loop_cand = (scratch.is_some() && dst != src).then(|| {
        let s = scratch.expect("gated on scratch");
        let c = *counter;
        *counter += 1;
        let l_loop = format!("$mul${module}${c}$loop");
        let l_done = format!("$mul${module}${c}$done");
        // The multiplicand holder `s` is seeded `move.w dst, s`. Only the LONG
        // contract also zeroes its upper word (`moveq #0, s`), because only its
        // `add.l s, dst` body reads all 32 bits; the word contract's
        // `add.w s, dst` reads `s`'s low word alone, so `s`'s upper word is
        // undefined throughout the word loop and never read.
        let mut setup = Vec::new();
        if width == MulWidth::Long {
            setup.push(instr("moveq", None, vec![imm(0), reg(s)], span));
        }
        setup.push(instr("move", Some(Width::W), vec![reg(dst), reg(s)], span));
        setup.push(instr("moveq", None, vec![imm(0), reg(dst)], span));
        setup.push(instr("subq", Some(Width::W), vec![imm(1), reg(src)], span));
        let add = instr("add", Some(acc_width), vec![reg(s), reg(dst)], span);
        // Worst-case cycles from the SAME cost seam as everything else.
        let setup_cost: u32 = setup.iter().map(item_worst_cycles).sum();
        let (bcs_taken, bcs_not_taken) = branch_cost("bcs", Some(Width::S));
        let (dbf_taken, dbf_not_taken) = branch_cost("dbf", None);
        let add_cost = item_worst_cycles(&add);
        let cycles = if bound == 0 {
            // src must be 0: subq borrows, bcs.s takes.
            setup_cost + bcs_taken
        } else {
            // src = bound: guard falls through; `add` runs bound times, dbf
            // loops back bound−1 times and exits once on counter expiry.
            setup_cost
                + bcs_not_taken
                + bound * add_cost
                + (bound - 1) * dbf_taken
                + dbf_not_taken
        };
        let mut items = setup;
        items.push(instr("bcs", Some(Width::S), vec![sym(&l_done)], span));
        items.push(CodeItem::Label { name: l_loop.clone(), export: false, span });
        items.push(add);
        items.push(instr("dbf", None, vec![reg(src), sym(&l_loop)], span));
        items.push(CodeItem::Label { name: l_done, export: false, span });
        (items, cycles)
    });

    // Worst-vs-worst comparison against mulu's flat 70 ceiling. The long loop
    // costs 28 + 18·M and wins through M = 2; the word loop, carrying no
    // upper-word seed, costs 24 + 14·M and wins through M = 3 (mulu at M ≥ 4).
    // Both boundaries are pinned by `*_bounded_semantics_and_boundary`.
    match loop_cand {
        Some((items, loop_cost)) if loop_cost < mulu_cost => Ok(items),
        _ => Ok(mulu_items),
    }
}

// ---- candidate selection ------------------------------------------------

/// Pick the winning straight-line candidate: fewest worst-case cycles, then
/// fewest encoded bytes, then earliest in enumeration order. Total and
/// deterministic; goldens depend on it.
fn choose(candidates: Vec<Vec<CodeItem>>) -> Vec<CodeItem> {
    let mut best: Option<(u32, u32, Vec<CodeItem>)> = None;
    for items in candidates {
        let Some(cycles) = seq_worst_cycles(&items) else {
            debug_assert!(false, "every generated candidate must be table-priced");
            continue;
        };
        let bytes = seq_bytes(&items);
        match &best {
            Some((bc, bb, _)) if (cycles, bytes) >= (*bc, *bb) => {}
            _ => best = Some((cycles, bytes, items)),
        }
    }
    best.expect("the mulu candidate always exists and always prices").2
}

/// Sum of worst-case cycle costs over straight-line items, from the one cost
/// seam ([`instr_cost`]). `None` if any item is off-table (a generator bug).
fn seq_worst_cycles(items: &[CodeItem]) -> Option<u32> {
    let mut total: u32 = 0;
    for item in items {
        match item {
            CodeItem::Instr { .. } => total += checked_item_cycles(item)?,
            CodeItem::Label { .. } => {}
            _ => return None,
        }
    }
    Some(total)
}

fn checked_item_cycles(item: &CodeItem) -> Option<u32> {
    let CodeItem::Instr { mnemonic, size, ops, .. } = item else { return None };
    match instr_cost(mnemonic, *size, ops) {
        CycleCost::Fixed { cycles, .. } => Some(u32::from(cycles)),
        // A branch inside a straight-line candidate is a generator bug.
        CycleCost::Branch { .. } | CycleCost::Unmodeled => None,
    }
}

/// Worst-case cycles of one instruction item; panics on off-table forms (the
/// loop generator emits only priced shapes — pinned by tests).
fn item_worst_cycles(item: &CodeItem) -> u32 {
    checked_item_cycles(item).expect("loop candidate instruction must be table-priced")
}

/// The (taken, not-taken) worst-case pair for a conditional transfer.
fn branch_cost(mnemonic: &str, size: Option<Width>) -> (u32, u32) {
    match instr_cost(mnemonic, size, &[]) {
        CycleCost::Branch { taken, not_taken, .. } => {
            (u32::from(taken), u32::from(not_taken))
        }
        other => panic!("`{mnemonic}` must price as a branch, got {other:?}"),
    }
}

/// Total encoded bytes of a straight-line candidate — the ENCODER's own answer
/// ([`crate::lower::encoded_len_m68k`]), used only as the cycle-tie arbiter.
/// An unencodable item (impossible for generated shapes) sorts last.
fn seq_bytes(items: &[CodeItem]) -> u32 {
    items
        .iter()
        .map(|item| match item {
            CodeItem::Instr { mnemonic, size, ops, .. } => {
                crate::lower::encoded_len_m68k(mnemonic, *size, ops)
                    .map_or(u32::MAX / 64, |n| n as u32)
            }
            _ => 0,
        })
        .sum()
}

// ---- shared sequence pieces ---------------------------------------------

/// In-place zero-extend of dN.w to dN.l: `swap / clr.w / swap` (12 cycles,
/// 6 bytes — beats `andi.l #$FFFF` at 16/6 and demands no scratch).
fn zx_in_place(d: Reg, span: Span) -> Vec<CodeItem> {
    vec![
        instr("swap", None, vec![reg(d)], span),
        instr("clr", Some(Width::W), vec![reg(d)], span),
        instr("swap", None, vec![reg(d)], span),
    ]
}

/// Seed dst AND scratch with zx(x): `moveq #0,S / move.w D,S / move.l S,D`
/// (12 cycles, 6 bytes — the cheapest way to hold the original twice).
fn seed_pair(dst: Reg, s: Reg, span: Span) -> Vec<CodeItem> {
    vec![
        instr("moveq", None, vec![imm(0), reg(s)], span),
        instr("move", Some(Width::W), vec![reg(dst), reg(s)], span),
        instr("move", Some(Width::L), vec![reg(s), reg(dst)], span),
    ]
}

/// ×2^k on a long register: nothing for k = 0; any REMAINING single double —
/// a run of 1, or the 1-bit remainder after `lsl.l` chunks of 8 (k = 9) — is
/// `add.l d,d` (8 cycles, vs `lsl.l #1`'s 10).
fn shift_run(mut k: u32, d: Reg, span: Span) -> Vec<CodeItem> {
    let mut items = Vec::new();
    while k > 0 {
        if k == 1 {
            items.push(instr("add", Some(Width::L), vec![reg(d), reg(d)], span));
            break;
        }
        let step = k.min(8);
        items.push(instr("lsl", Some(Width::L), vec![imm(step), reg(d)], span));
        k -= step;
    }
    items
}

/// ×2^k on a WORD register: `lsl.w` chunks of ≤ 8 (nothing for k = 0). Unlike
/// [`shift_run`], a k = 1 double emits a plain `lsl.w #1` rather than
/// `add.w d,d`.
///
/// That is a MISSED four cycles (`add.w d,d` costs 4, `lsl.w #1` costs 8), not a
/// contract: it was chosen to hold a byte-identity bar against hand-written
/// strides that no longer exist, and that bar is retired. Taking it is
/// byte-changing — it would move every chain whose trailing run is a single
/// shift, including the ×66 stride — so it belongs to a parcel that re-proves
/// the goldens, and is recorded in the gap ledger.
fn shift_run_word(mut k: u32, d: Reg, span: Span) -> Vec<CodeItem> {
    let mut items = Vec::new();
    while k > 0 {
        let step = k.min(8);
        items.push(instr("lsl", Some(Width::W), vec![imm(step), reg(d)], span));
        k -= step;
    }
    items
}

// ---- small constructors -------------------------------------------------

fn instr(mnemonic: &str, size: Option<Width>, ops: Vec<CodeOperand>, span: Span) -> CodeItem {
    CodeItem::Instr {
        mnemonic: mnemonic.to_string(),
        size,
        ops,
        span,
        as_type: None,
        targets: Vec::new(),
        // The placeholder the expansion's final `reauthor_user_items` pass
        // lifts to the construct item's own authorship.
        author: ItemAuthor::User,
    }
}

fn imm(v: u32) -> CodeOperand {
    CodeOperand::Imm(i128::from(v))
}

fn reg(r: Reg) -> CodeOperand {
    CodeOperand::Reg(r)
}

fn sym(s: &str) -> CodeOperand {
    CodeOperand::Sym(s.to_string())
}

fn err(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic { level: Level::Error, message: message.into(), primary: span }
}

/// The surface spelling of a refused size suffix, for the `[mul.size]` message
/// (`.w` is accepted and never reaches this).
fn width_suffix(w: Width) -> &'static str {
    match w {
        Width::B => "b",
        Width::W => "w",
        Width::L => "l",
        Width::S => "s",
    }
}

/// The operand must be a DATA register (mulu's destination and the shift ops
/// demand Dn; an address register refuses loudly).
fn data_reg(op: &CodeOperand, role: &str, span: Span) -> Result<Reg, Diagnostic> {
    match op {
        CodeOperand::Reg(r) => {
            let (is_addr, _) = crate::lower::reg_kind(*r);
            if is_addr {
                Err(err(
                    span,
                    format!(
                        "[mul.reg-class] {role} must be a data register (mulu and the \
                         shift/add chains operate on Dn), got `{r}`"
                    ),
                ))
            } else {
                Ok(*r)
            }
        }
        _ => Err(err(span, format!("[mul.operands] {role} must be a data register"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sp() -> Span {
        Span { source: sigil_span::SourceId(0), start: 7, end: 9 }
    }

    fn expand_const(ops: Vec<CodeOperand>) -> Result<Vec<CodeItem>, Diagnostic> {
        let mut c = 0;
        expand_item("mul_const", None, &ops, sp(), "m", &mut c, &ItemAuthor::User)
    }

    fn expand_bounded(ops: Vec<CodeOperand>) -> Result<Vec<CodeItem>, Diagnostic> {
        let mut c = 0;
        expand_item("mul_bounded", None, &ops, sp(), "m", &mut c, &ItemAuthor::User)
    }

    fn expand_const_w(ops: Vec<CodeOperand>) -> Result<Vec<CodeItem>, Diagnostic> {
        let mut c = 0;
        expand_item("mul_const", Some(Width::W), &ops, sp(), "m", &mut c, &ItemAuthor::User)
    }

    fn expand_bounded_w(ops: Vec<CodeOperand>) -> Result<Vec<CodeItem>, Diagnostic> {
        let mut c = 0;
        expand_item("mul_bounded", Some(Width::W), &ops, sp(), "m", &mut c, &ItemAuthor::User)
    }

    /// Render an item list as compact `mnemonic.size ops` strings for pins.
    fn shape(items: &[CodeItem]) -> Vec<String> {
        items
            .iter()
            .map(|i| match i {
                CodeItem::Instr { mnemonic, size, ops, .. } => {
                    let sz = match size {
                        Some(Width::B) => ".b",
                        Some(Width::W) => ".w",
                        Some(Width::L) => ".l",
                        Some(Width::S) => ".s",
                        None => "",
                    };
                    let ops: Vec<String> = ops
                        .iter()
                        .map(|o| match o {
                            CodeOperand::Imm(v) => format!("#{v}"),
                            CodeOperand::Reg(r) => r.to_string(),
                            CodeOperand::Sym(s) => s.clone(),
                            other => format!("{other:?}"),
                        })
                        .collect();
                    format!("{mnemonic}{sz} {}", ops.join(","))
                }
                CodeItem::Label { name, .. } => format!("{name}:"),
                other => format!("{other:?}"),
            })
            .collect()
    }

    /// Interpret an expanded sequence over concrete registers — the equivalence
    /// oracle. Covers EXACTLY the emitted vocabulary (an unknown mnemonic
    /// panics, keeping the oracle honest as generators grow). Carry is tracked
    /// only from `subq.w` (the loop guard's sole producer before `bcs`).
    fn run(items: &[CodeItem], regs: &mut BTreeMap<Reg, u32>) {
        let mut carry = false;
        let mut pc = 0usize;
        let label_at = |name: &str, items: &[CodeItem]| {
            items
                .iter()
                .position(
                    |i| matches!(i, CodeItem::Label { name: n, .. } if n == name),
                )
                .expect("branch target label exists")
        };
        let mut steps = 0u32;
        while pc < items.len() {
            steps += 1;
            assert!(steps < 1_000_000, "interpreter runaway");
            match &items[pc] {
                CodeItem::Label { .. } => {}
                CodeItem::Instr { mnemonic, size, ops, .. } => {
                    let get = |r: &CodeOperand, regs: &BTreeMap<Reg, u32>| match r {
                        CodeOperand::Reg(r) => *regs.get(r).unwrap_or(&0),
                        CodeOperand::Imm(v) => *v as u32,
                        _ => panic!("oracle: unsupported operand"),
                    };
                    let dst_reg = |r: &CodeOperand| match r {
                        CodeOperand::Reg(r) => *r,
                        _ => panic!("oracle: dest must be a register"),
                    };
                    match (mnemonic.as_str(), size) {
                        ("moveq", None) => {
                            let v = get(&ops[0], regs) as i8 as i32 as u32;
                            regs.insert(dst_reg(&ops[1]), v);
                        }
                        ("move", Some(Width::W)) => {
                            let d = dst_reg(&ops[1]);
                            let old = *regs.get(&d).unwrap_or(&0);
                            let v = get(&ops[0], regs) & 0xFFFF;
                            regs.insert(d, (old & 0xFFFF_0000) | v);
                        }
                        ("move", Some(Width::L)) => {
                            let v = get(&ops[0], regs);
                            regs.insert(dst_reg(&ops[1]), v);
                        }
                        ("swap", None) => {
                            let d = dst_reg(&ops[0]);
                            let v = regs[&d];
                            regs.insert(d, v.rotate_left(16));
                        }
                        ("clr", Some(Width::W)) => {
                            let d = dst_reg(&ops[0]);
                            let v = regs[&d];
                            regs.insert(d, v & 0xFFFF_0000);
                        }
                        ("lsl", Some(Width::L)) => {
                            let d = dst_reg(&ops[1]);
                            let k = get(&ops[0], regs);
                            regs.insert(d, regs[&d] << k);
                        }
                        ("add", Some(Width::L)) => {
                            let d = dst_reg(&ops[1]);
                            let v = regs[&d].wrapping_add(get(&ops[0], regs));
                            regs.insert(d, v);
                        }
                        ("sub", Some(Width::L)) => {
                            let d = dst_reg(&ops[1]);
                            let v = regs[&d].wrapping_sub(get(&ops[0], regs));
                            regs.insert(d, v);
                        }
                        // Word arithmetic: operate on the low word, PRESERVE the
                        // upper word — the property the word contract leaves free
                        // and the oracle proves a chain honors (while mulu does not).
                        ("lsl", Some(Width::W)) => {
                            let d = dst_reg(&ops[1]);
                            let k = get(&ops[0], regs);
                            let old = *regs.get(&d).unwrap_or(&0);
                            let low = ((old & 0xFFFF) << k) & 0xFFFF;
                            regs.insert(d, (old & 0xFFFF_0000) | low);
                        }
                        ("add", Some(Width::W)) => {
                            let d = dst_reg(&ops[1]);
                            let old = *regs.get(&d).unwrap_or(&0);
                            let v = get(&ops[0], regs) & 0xFFFF;
                            let low = (old & 0xFFFF).wrapping_add(v) & 0xFFFF;
                            regs.insert(d, (old & 0xFFFF_0000) | low);
                        }
                        ("sub", Some(Width::W)) => {
                            let d = dst_reg(&ops[1]);
                            let old = *regs.get(&d).unwrap_or(&0);
                            let v = get(&ops[0], regs) & 0xFFFF;
                            let low = (old & 0xFFFF).wrapping_sub(v) & 0xFFFF;
                            regs.insert(d, (old & 0xFFFF_0000) | low);
                        }
                        ("mulu", Some(Width::W)) => {
                            let d = dst_reg(&ops[1]);
                            let a = regs[&d] & 0xFFFF;
                            let b = get(&ops[0], regs) & 0xFFFF;
                            regs.insert(d, a * b);
                        }
                        ("subq", Some(Width::W)) => {
                            let d = dst_reg(&ops[1]);
                            let old = regs[&d];
                            let w = old & 0xFFFF;
                            let q = get(&ops[0], regs);
                            carry = w < q;
                            regs.insert(
                                d,
                                (old & 0xFFFF_0000) | (w.wrapping_sub(q) & 0xFFFF),
                            );
                        }
                        ("bcs", Some(Width::S)) => {
                            if carry {
                                let CodeOperand::Sym(t) = &ops[0] else {
                                    panic!("bcs target")
                                };
                                pc = label_at(t, items);
                                continue;
                            }
                        }
                        ("dbf", None) => {
                            let d = dst_reg(&ops[0]);
                            let old = regs[&d];
                            let w = (old & 0xFFFF).wrapping_sub(1) & 0xFFFF;
                            regs.insert(d, (old & 0xFFFF_0000) | w);
                            if w != 0xFFFF {
                                let CodeOperand::Sym(t) = &ops[1] else {
                                    panic!("dbf target")
                                };
                                pc = label_at(t, items);
                                continue;
                            }
                        }
                        other => panic!("oracle: unmodeled instruction {other:?}"),
                    }
                }
                other => panic!("oracle: unmodeled item {other:?}"),
            }
            pc += 1;
        }
    }

    const NS: &[u32] = &[
        0, 1, 2, 3, 5, 6, 7, 11, 19, 36, 40, 63, 64, 66, 80, 96, 160, 255, 256,
        512, 4096, 0x8000, 0x8001, 0xAAAA, 0xFFFF,
    ];
    const XS: &[u16] = &[0, 1, 2, 0x1234, 0x7FFF, 0x8000, 0xABCD, 0xFFFE, 0xFFFF];

    // The pinned result contract, proven by execution: every chosen lowering —
    // with and without scratch, upper-word garbage included — computes exactly
    // zx(x) × n over the sampled + boundary inputs.
    #[test]
    fn every_chosen_lowering_matches_mulu_semantics() {
        for &n in NS {
            for &with_scratch in &[false, true] {
                let mut ops = vec![reg(Reg::D0), imm(n)];
                if with_scratch {
                    ops.push(reg(Reg::D1));
                }
                let items = expand_const(ops).expect("expands");
                for &x in XS {
                    for &garbage in &[0u32, 0xDEAD_0000, 0xFFFF_0000] {
                        let mut regs = BTreeMap::new();
                        regs.insert(Reg::D0, garbage | u32::from(x));
                        regs.insert(Reg::D1, 0xBEEF_CAFE);
                        run(&items, &mut regs);
                        assert_eq!(
                            regs[&Reg::D0],
                            u32::from(x) * n,
                            "n={n} x={x:#x} scratch={with_scratch} shape={:?}",
                            shape(&items)
                        );
                    }
                }
            }
        }
    }

    // mul_bounded: both lowerings compute zx(dst.w) × zx(src.w) for every
    // src within the declared bound (loop for the tiny bounds, mulu above).
    #[test]
    fn mul_bounded_matches_mulu_semantics_within_bound() {
        for &m in &[0u32, 1, 2, 3, 16] {
            for &with_scratch in &[false, true] {
                let mut ops = vec![reg(Reg::D0), reg(Reg::D1), imm(m)];
                if with_scratch {
                    ops.push(reg(Reg::D2));
                }
                let items = expand_bounded(ops).expect("expands");
                for src in 0..=m.min(20) {
                    for &x in &[0u16, 1, 7, 0x8000, 0xFFFF] {
                        let mut regs = BTreeMap::new();
                        regs.insert(Reg::D0, 0xDEAD_0000 | u32::from(x));
                        regs.insert(Reg::D1, 0x5555_0000 | src);
                        regs.insert(Reg::D2, 0x0BAD_F00D);
                        run(&items, &mut regs);
                        assert_eq!(
                            regs[&Reg::D0],
                            u32::from(x) * src,
                            "M={m} src={src} x={x:#x} scratch={with_scratch}"
                        );
                    }
                }
            }
        }
    }

    // The cost decision, pinned per encoding class. The corpus strides all
    // resolve to mulu (the ×66 chain TIES mulu at 46 cycles and loses the byte
    // tie-break 12 vs 4 — the R-B verdict as a computed fact).
    #[test]
    fn corpus_strides_resolve_to_mulu() {
        for &n in &[36u32, 40, 66, 80, 160] {
            let items =
                expand_const(vec![reg(Reg::D0), imm(n), reg(Reg::D1)]).unwrap();
            assert_eq!(
                shape(&items),
                vec![format!("mulu.w #{n},d0")],
                "n={n} must lower to mulu (chain ties or loses)"
            );
        }
    }

    #[test]
    fn small_factors_win_as_chains() {
        // ×3 = double, add — 28 cycles vs mulu's 46.
        let items = expand_const(vec![reg(Reg::D0), imm(3), reg(Reg::D1)]).unwrap();
        assert_eq!(
            shape(&items),
            ["moveq #0,d1", "move.w d0,d1", "move.l d1,d0", "add.l d0,d0", "add.l d1,d0"]
        );
        // ×63 = (x·64 − x) — the subtract form (40 cycles) beats LTR (92) and
        // mulu (54).
        let items = expand_const(vec![reg(Reg::D0), imm(63), reg(Reg::D1)]).unwrap();
        assert_eq!(
            shape(&items),
            ["moveq #0,d1", "move.w d0,d1", "move.l d1,d0", "lsl.l #6,d0", "sub.l d1,d0"]
        );
    }

    #[test]
    fn degenerates_and_powers_of_two() {
        let z = expand_const(vec![reg(Reg::D0), imm(0)]).unwrap();
        assert_eq!(shape(&z), ["moveq #0,d0"]);
        let one = expand_const(vec![reg(Reg::D0), imm(1)]).unwrap();
        assert_eq!(shape(&one), ["swap d0", "clr.w d0", "swap d0"]);
        // ×2: zero-extend then add-self (20 cycles — add.l beats lsl.l #1).
        let two = expand_const(vec![reg(Reg::D0), imm(2)]).unwrap();
        assert_eq!(shape(&two), ["swap d0", "clr.w d0", "swap d0", "add.l d0,d0"]);
        // ×64: shift — scratch-free, 32 cycles vs mulu's 44.
        let p6 = expand_const(vec![reg(Reg::D0), imm(64)]).unwrap();
        assert_eq!(shape(&p6), ["swap d0", "clr.w d0", "swap d0", "lsl.l #6,d0"]);
    }

    // The chain-vs-mulu COST BOUNDARY for powers of two sits between 2^8 and
    // 2^9: ×256 = zx + lsl#8 (36 cycles) still beats mulu (44); ×512 needs a
    // second shift (46) and mulu wins.
    #[test]
    fn power_of_two_cost_boundary_flips_at_2_to_the_9() {
        let p8 = expand_const(vec![reg(Reg::D0), imm(256)]).unwrap();
        assert_eq!(shape(&p8), ["swap d0", "clr.w d0", "swap d0", "lsl.l #8,d0"]);
        let p9 = expand_const(vec![reg(Reg::D0), imm(512)]).unwrap();
        assert_eq!(shape(&p9), ["mulu.w #512,d0"]);
    }

    // Without scratch there is no general-chain candidate: a two-set-bit n
    // falls back to mulu SILENTLY (mulu is always legal — the design's rule;
    // the diagnostic surface is only for genuinely illegal inputs).
    #[test]
    fn missing_scratch_falls_back_to_mulu() {
        let items = expand_const(vec![reg(Reg::D0), imm(3)]).unwrap();
        assert_eq!(shape(&items), ["mulu.w #3,d0"]);
    }

    // mul_bounded's worst-vs-worst boundary: the loop (28 + 18·M) beats the
    // mulu ceiling (70) only through M = 2; M = 3 flips to mulu. No scratch or
    // aliased dst/src excludes the loop candidate entirely.
    #[test]
    fn mul_bounded_cost_boundary_and_candidate_gating() {
        let m2 = expand_bounded(vec![reg(Reg::D0), reg(Reg::D1), imm(2), reg(Reg::D2)])
            .unwrap();
        assert_eq!(
            shape(&m2),
            [
                "moveq #0,d2",
                "move.w d0,d2",
                "moveq #0,d0",
                "subq.w #1,d1",
                "bcs.s $mul$m$0$done",
                "$mul$m$0$loop:",
                "add.l d2,d0",
                "dbf d1,$mul$m$0$loop",
                "$mul$m$0$done:"
            ]
        );
        let m3 = expand_bounded(vec![reg(Reg::D0), reg(Reg::D1), imm(3), reg(Reg::D2)])
            .unwrap();
        assert_eq!(shape(&m3), ["mulu.w d1,d0"]);
        // No scratch → no loop candidate, even at M = 2.
        let no_s = expand_bounded(vec![reg(Reg::D0), reg(Reg::D1), imm(2)]).unwrap();
        assert_eq!(shape(&no_s), ["mulu.w d1,d0"]);
        // dst == src (a square) → the loop is structurally impossible; mulu.
        let sq = expand_bounded(vec![reg(Reg::D0), reg(Reg::D0), imm(2), reg(Reg::D2)])
            .unwrap();
        assert_eq!(shape(&sq), ["mulu.w d0,d0"]);
    }

    // Loop labels embed the module id (the hygiene convention): two modules at
    // the same counter value mint disjoint names, so cross-module use of the
    // loop lowering cannot collide at link.
    #[test]
    fn loop_labels_embed_the_module_id() {
        let ops = vec![reg(Reg::D0), reg(Reg::D1), imm(2), reg(Reg::D2)];
        let mut c = 0;
        let a = expand_item("mul_bounded", None, &ops, sp(), "alpha", &mut c, &ItemAuthor::User).unwrap();
        let mut c = 0;
        let b = expand_item("mul_bounded", None, &ops, sp(), "beta", &mut c, &ItemAuthor::User).unwrap();
        let labels = |items: &[CodeItem]| -> Vec<String> {
            items
                .iter()
                .filter_map(|i| match i {
                    CodeItem::Label { name, .. } => Some(name.clone()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(labels(&a), ["$mul$alpha$0$loop", "$mul$alpha$0$done"]);
        assert_eq!(labels(&b), ["$mul$beta$0$loop", "$mul$beta$0$done"]);
    }

    // Determinism: the same operands expand to the same items, every time.
    #[test]
    fn expansion_is_deterministic() {
        for &n in NS {
            let a = expand_const(vec![reg(Reg::D3), imm(n), reg(Reg::D5)]).unwrap();
            let b = expand_const(vec![reg(Reg::D3), imm(n), reg(Reg::D5)]).unwrap();
            assert_eq!(a, b, "n={n}");
        }
    }

    // Every generated straight-line candidate is priced EXACTLY by the one
    // cost seam — an inexact or off-table candidate would make the decision
    // depend on a ceiling, which the chain contract never needs.
    #[test]
    fn every_straight_line_candidate_prices_exactly() {
        for &n in NS {
            for width in [MulWidth::Long, MulWidth::Word] {
                for cand in mul_const_candidates(Reg::D0, n, Some(Reg::D1), sp(), width) {
                    for item in &cand {
                        let CodeItem::Instr { mnemonic, size, ops, .. } = item else {
                            continue;
                        };
                        match instr_cost(mnemonic, *size, ops) {
                            CycleCost::Fixed { exact: true, .. } => {}
                            other => panic!(
                                "candidate for n={n} width={width:?} has non-exact \
                                 cost {other:?}: {mnemonic}"
                            ),
                        }
                    }
                }
            }
        }
    }

    // Diagnostics: each refusal carries its tag.
    #[test]
    fn refusals_are_tagged() {
        let tag = |r: Result<Vec<CodeItem>, Diagnostic>| r.unwrap_err().message;
        assert!(
            tag(expand_const(vec![reg(Reg::D0), imm(0x10000)]))
                .contains("[mul.const-range]")
        );
        assert!(
            tag(expand_const(vec![reg(Reg::D0), CodeOperand::Imm(-1)]))
                .contains("[mul.const-range]")
        );
        assert!(
            tag(expand_const(vec![reg(Reg::D0), imm(66), reg(Reg::D0)]))
                .contains("[mul.scratch-aliases]")
        );
        assert!(
            tag(expand_const(vec![reg(Reg::A0), imm(66)])).contains("[mul.reg-class]")
        );
        assert!(tag(expand_const(vec![reg(Reg::D0)])).contains("[mul.operands]"));
        // `.w` is now a VALID second contract (the word form); `.l` (and any
        // other suffix) refuses — bare already IS the long form.
        let mut c = 0;
        let sized = expand_item(
            "mul_const",
            Some(Width::L),
            &[reg(Reg::D0), imm(66)],
            sp(),
            "m",
            &mut c,
            &ItemAuthor::User,
        );
        assert!(sized.unwrap_err().message.contains("[mul.size]"));
        // mul_bounded: the bound is mandatory; scratch may alias neither reg.
        assert!(
            tag(expand_bounded(vec![reg(Reg::D0), reg(Reg::D1)]))
                .contains("[mul.operands]")
        );
        assert!(
            tag(expand_bounded(vec![
                reg(Reg::D0),
                reg(Reg::D1),
                imm(4),
                reg(Reg::D1)
            ]))
            .contains("[mul.scratch-aliases]")
        );
    }

    // Z80: the construct refuses per-item at expand_buf, and a cpu-less
    // template keeps its raw items for the eventual CPU-resolved expansion.
    #[test]
    fn z80_refuses_and_cpuless_defers() {
        let mk = || CodeBuf {
            items: vec![CodeItem::Instr {
                mnemonic: "mul_const".into(),
                size: None,
                ops: vec![reg(Reg::D0), imm(66)],
                span: sp(),
                as_type: None,
                targets: Vec::new(),
                author: ItemAuthor::User,
            }],
        };
        let mut c = 0;
        let mut z = mk();
        let ds = expand_buf(&mut z, Some(Cpu::Z80), "m", &mut c);
        assert_eq!(ds.len(), 1);
        assert!(ds[0].message.contains("[mul.non-68k]"));
        assert!(z.items.is_empty(), "the refused item is dropped, never silent bytes");

        let mut raw = mk();
        let ds = expand_buf(&mut raw, None, "m", &mut c);
        assert!(ds.is_empty());
        assert_eq!(raw.items.len(), 1, "cpu-less template stays raw");

        let mut m68k = mk();
        let ds = expand_buf(&mut m68k, Some(Cpu::M68000), "m", &mut c);
        assert!(ds.is_empty());
        assert_eq!(shape(&m68k.items), ["mulu.w #66,d0"]);
    }

    // ---- the word contract (`.w`) ---------------------------------------

    // The WORD result contract, proven by execution: every chosen `.w` lowering
    // computes exactly (zx(x) × n) mod 2^16 in dst's LOW word, over the sampled +
    // boundary inputs with garbage upper words. NOTHING is asserted about the
    // upper word — that is the contract's freedom.
    #[test]
    fn word_lowering_matches_low_word_and_leaves_upper_free() {
        for &n in NS {
            for &with_scratch in &[false, true] {
                let mut ops = vec![reg(Reg::D0), imm(n)];
                if with_scratch {
                    ops.push(reg(Reg::D1));
                }
                let items = expand_const_w(ops).expect("expands");
                for &x in XS {
                    for &garbage in &[0u32, 0xDEAD_0000, 0xFFFF_0000] {
                        let mut regs = BTreeMap::new();
                        regs.insert(Reg::D0, garbage | u32::from(x));
                        regs.insert(Reg::D1, 0xBEEF_CAFE);
                        run(&items, &mut regs);
                        assert_eq!(
                            regs[&Reg::D0] & 0xFFFF,
                            u32::from(x).wrapping_mul(n) & 0xFFFF,
                            "n={n} x={x:#x} scratch={with_scratch} shape={:?}",
                            shape(&items)
                        );
                    }
                }
            }
        }
    }

    // The dedicated upper-word-freedom pin: a `.w` lowering that PICKS mulu
    // genuinely trashes the upper word, while a chosen chain leaves it untouched
    // — BOTH are contract-legal (the upper word is undefined), which is exactly
    // what makes the word candidate set free to include mulu.
    #[test]
    fn word_upper_is_free_mulu_trashes_chain_preserves() {
        // ×66 with scratch → the left-to-right chain (32 cy, beats `mulu.w`'s
        // 46), all word ops that leave dst's upper word untouched.
        let chain = expand_const_w(vec![reg(Reg::D0), imm(66), reg(Reg::D1)]).unwrap();
        assert_eq!(
            shape(&chain),
            ["move.w d0,d1", "lsl.w #5,d0", "add.w d1,d0", "lsl.w #1,d0"]
        );
        let mut regs = BTreeMap::new();
        regs.insert(Reg::D0, 0xDEAD_0002);
        regs.insert(Reg::D1, 0xBEEF_0000);
        run(&chain, &mut regs);
        assert_eq!(regs[&Reg::D0] & 0xFFFF, 2 * 66);
        assert_eq!(
            regs[&Reg::D0] & 0xFFFF_0000,
            0xDEAD_0000,
            "the word chain leaves dst's upper word untouched"
        );
        // A two-set-bit multiplier whose chain loses to mulu on cycles (×$8001:
        // the far-apart bits make the shift run dear) → mulu, which writes all
        // 32 bits.
        let mulu = expand_const_w(vec![reg(Reg::D0), imm(0x8001), reg(Reg::D1)]).unwrap();
        assert_eq!(shape(&mulu), ["mulu.w #32769,d0"]);
        let mut regs = BTreeMap::new();
        regs.insert(Reg::D0, 0xDEAD_0002);
        run(&mulu, &mut regs);
        assert_eq!(regs[&Reg::D0] & 0xFFFF, 2u32.wrapping_mul(0x8001) & 0xFFFF);
        assert_ne!(
            regs[&Reg::D0] & 0xFFFF_0000,
            0xDEAD_0000,
            "mulu writes the full product — the upper word is trashed (contract-legal)"
        );
    }

    // The corpus strides resolve to the left-to-right WORD chain (32/32/34 cycles
    // vs mulu's 46), the cheapest candidate at each; the chain uses only word ops
    // that leave dst's upper word free.
    #[test]
    fn word_corpus_strides_resolve_to_chains() {
        let c66 = expand_const_w(vec![reg(Reg::D0), imm(66), reg(Reg::D1)]).unwrap();
        assert_eq!(
            shape(&c66),
            ["move.w d0,d1", "lsl.w #5,d0", "add.w d1,d0", "lsl.w #1,d0"]
        );
        let c80 = expand_const_w(vec![reg(Reg::D0), imm(80), reg(Reg::D1)]).unwrap();
        assert_eq!(
            shape(&c80),
            ["move.w d0,d1", "lsl.w #2,d0", "add.w d1,d0", "lsl.w #4,d0"]
        );
        let c160 = expand_const_w(vec![reg(Reg::D0), imm(160), reg(Reg::D1)]).unwrap();
        assert_eq!(
            shape(&c160),
            ["move.w d0,d1", "lsl.w #2,d0", "add.w d1,d0", "lsl.w #5,d0"]
        );
        // Without scratch no chain is available → mulu (always legal).
        let c66_noscr = expand_const_w(vec![reg(Reg::D0), imm(66)]).unwrap();
        assert_eq!(shape(&c66_noscr), ["mulu.w #66,d0"]);
    }

    // The 5c gate, probed where it is actually DECIDABLE — at the CANDIDATE SET,
    // not at `choose()`'s output.
    //
    // What discriminates what, stated exactly, because the obvious framing does
    // not hold: a 1-bit multiplier cannot test 5c's own `>= 2` threshold at all.
    // Arm 5's OUTER guard already excludes 1-bit n, and even with both guards
    // removed 5c would lose to the plain shift arm (22 cy / 4 B against 18 / 2),
    // so `choose()` returns `lsl.w` no matter what 5c's gate says. The 1-bit
    // assertions below therefore pin arm-5 GATING IN GENERAL (a chain must never
    // be generated for a single set bit, so scratch is never touched even when
    // offered) — they are honest, non-vacuous, and NOT a probe of `>= 2`.
    //
    // The 2-bit assertions are what discriminate the changed gate: 5c must be
    // GENERATED at exactly 2 set bits and must WIN. Regress the gate to `>= 3`
    // and the LTR candidate disappears; at n = 80 no chain candidate is left at
    // all (5b's factored form does not apply — 80 = 5 · 2^4 and 5 + 1 is not a
    // power of two), so `mulu.w` wins by default and every corpus stride changes
    // bytes — these fail loudly.
    #[test]
    fn word_gate_boundary_generates_and_selects_the_ltr_arm() {
        let ltr80 = ["move.w d0,d1", "lsl.w #2,d0", "add.w d1,d0", "lsl.w #4,d0"];
        let cands: Vec<Vec<String>> =
            mul_const_candidates_word(Reg::D0, 80, Some(Reg::D1), sp())
                .iter()
                .map(|c| shape(c))
                .collect();
        assert!(
            cands.iter().any(|c| c.as_slice() == ltr80),
            "5c must be GENERATED at 2 set bits (a `>= 3` gate removes it): {cands:?}"
        );
        // ... and it is the one `choose()` takes.
        let two_bit = expand_const_w(vec![reg(Reg::D0), imm(80), reg(Reg::D1)]).unwrap();
        assert_eq!(shape(&two_bit), ltr80);

        // 1-bit: arm 5 generates nothing, so NO candidate mentions the scratch
        // register even though one was offered.
        let cands1: Vec<Vec<String>> =
            mul_const_candidates_word(Reg::D0, 64, Some(Reg::D1), sp())
                .iter()
                .map(|c| shape(c))
                .collect();
        assert!(
            cands1.iter().all(|c| !c.iter().any(|i| i.contains("d1"))),
            "a 1-bit multiplier generates no chain candidate at all: {cands1:?}"
        );
        let one_bit = expand_const_w(vec![reg(Reg::D0), imm(64), reg(Reg::D1)]).unwrap();
        assert_eq!(shape(&one_bit), ["lsl.w #6,d0"]);
    }

    // The two-power sum `move.w D,S / lsl.w #a,D / lsl.w #b,S / add.w S,D` is
    // DOMINATED BY 5c at every n = 2^a + 2^b, which is why the candidate set
    // does not carry it as its own arm: 5c is never dearer, and is byte-identical
    // whenever it is not cheaper. Both shapes are constructed HERE, by hand, so
    // this theorem is independent of what the generator offers — it holds for any
    // arm that factors a two-power n this way, whether or not the candidate set
    // carries one.
    //
    // Hand-building 5c is what makes the theorem arm-independent, and it is also
    // what would let the theorem outlive the generator: if `shift_run_word` ever
    // takes the open k = 1 `add.w d,d` row, the hand shape stops being what the
    // generator emits and this test would keep proving a fact about a 5c that no
    // longer exists — and stay green. The presence assertion in the loop closes
    // that: the hand shape must be IN the generated candidate set at every n.
    //
    // Stated as a direct 5a-vs-5c cost comparison rather than against `choose()`'s
    // winner on purpose: at some n (e.g. $2001) `mulu.w` beats BOTH chains, so
    // "the chosen shape is not the two-power one" would hold for a reason that has
    // nothing to do with dominance.
    //
    // The sweep is exhaustive over the claim's whole domain: every n in
    // `0..=$FFFF` with exactly two set bits IS every n the two-power form can
    // express.
    #[test]
    fn word_two_power_arm_is_dominated_by_ltr() {
        let mut equal = 0u32;
        let mut cheaper = 0u32;
        for n in 0u32..=0xFFFFu32 {
            if n.count_ones() != 2 {
                continue;
            }
            let a = 31 - n.leading_zeros();
            let b = n.trailing_zeros();
            // 5a — the two-power sum, built here rather than obtained from the
            // candidate set, which is what makes the theorem arm-independent.
            let mut five_a =
                vec![instr("move", Some(Width::W), vec![reg(Reg::D0), reg(Reg::D1)], sp())];
            five_a.extend(shift_run_word(a, Reg::D0, sp()));
            five_a.extend(shift_run_word(b, Reg::D1, sp()));
            five_a.push(instr("add", Some(Width::W), vec![reg(Reg::D1), reg(Reg::D0)], sp()));
            // 5c — the left-to-right chain, same construction as the generator.
            let mut five_c =
                vec![instr("move", Some(Width::W), vec![reg(Reg::D0), reg(Reg::D1)], sp())];
            let msb = 31 - n.leading_zeros();
            let mut pending: u32 = 0;
            for bit in (0..msb).rev() {
                pending += 1;
                if n & (1 << bit) != 0 {
                    five_c.extend(shift_run_word(pending, Reg::D0, sp()));
                    pending = 0;
                    five_c.push(instr("add", Some(Width::W), vec![reg(Reg::D1), reg(Reg::D0)], sp()));
                }
            }
            five_c.extend(shift_run_word(pending, Reg::D0, sp()));

            // Generator-dependence guard: the hand 5c must be a shape the word
            // candidate generator actually offers, or the dominance theorem
            // below is about a chain nothing emits.
            let cands = mul_const_candidates_word(Reg::D0, n, Some(Reg::D1), sp());
            assert!(
                cands.iter().any(|c| shape(c) == shape(&five_c)),
                "n={n}: the hand-built 5c is not in the generated candidate set {:?}",
                cands.iter().map(|c| shape(c)).collect::<Vec<_>>()
            );

            let ca = seq_worst_cycles(&five_a).expect("5a prices");
            let cc = seq_worst_cycles(&five_c).expect("5c prices");
            assert!(cc <= ca, "n={n}: 5c ({cc}) must never cost more than 5a ({ca})");
            if cc == ca {
                assert_eq!(
                    shape(&five_c),
                    shape(&five_a),
                    "n={n}: equal cost must mean an identical sequence"
                );
                equal += 1;
            } else {
                cheaper += 1;
            }
        }
        // Non-vacuity: both regimes are genuinely exercised.
        assert!(equal > 0 && cheaper > 0, "equal={equal} cheaper={cheaper}");
    }

    // Word degenerates: 0 → clr.w, 1 → NOTHING, 2^k → lsl.w (no zero-extend seed).
    #[test]
    fn word_degenerates_and_powers() {
        let z = expand_const_w(vec![reg(Reg::D0), imm(0)]).unwrap();
        assert_eq!(shape(&z), ["clr.w d0"]);
        let one = expand_const_w(vec![reg(Reg::D0), imm(1)]).unwrap();
        assert!(one.is_empty(), "×1 mod 2^16 is zero instructions");
        let p6 = expand_const_w(vec![reg(Reg::D0), imm(64)]).unwrap();
        assert_eq!(shape(&p6), ["lsl.w #6,d0"]);
        // ×512 = 2^9: two lsl.w chunks (≤ 8 each), still a pure word shift.
        let p9 = expand_const_w(vec![reg(Reg::D0), imm(512)]).unwrap();
        assert_eq!(shape(&p9), ["lsl.w #8,d0", "lsl.w #1,d0"]);
    }

    // mul_bounded.w: both lowerings compute (zx(dst.w) × src) mod 2^16 for every
    // in-bound src; the loop accumulates with add.w and wins through M = 3
    // (mulu at M ≥ 4), one M past the long form's boundary because it carries
    // no upper-word seed.
    #[test]
    fn word_bounded_semantics_and_boundary() {
        for &m in &[0u32, 1, 2, 3, 16] {
            for &with_scratch in &[false, true] {
                let mut ops = vec![reg(Reg::D0), reg(Reg::D1), imm(m)];
                if with_scratch {
                    ops.push(reg(Reg::D2));
                }
                let items = expand_bounded_w(ops).expect("expands");
                for src in 0..=m.min(20) {
                    for &x in &[0u16, 1, 7, 0x8000, 0xFFFF] {
                        let mut regs = BTreeMap::new();
                        regs.insert(Reg::D0, 0xDEAD_0000 | u32::from(x));
                        regs.insert(Reg::D1, 0x5555_0000 | src);
                        regs.insert(Reg::D2, 0x0BAD_F00D);
                        run(&items, &mut regs);
                        assert_eq!(
                            regs[&Reg::D0] & 0xFFFF,
                            u32::from(x).wrapping_mul(src) & 0xFFFF,
                            "M={m} src={src} x={x:#x} scratch={with_scratch}"
                        );
                    }
                }
            }
        }
        // At M = 3 the loop wins and accumulates with add.w; the word loop's
        // setup carries no `moveq #0` for the multiplicand holder (its upper
        // word is never read). At M = 4 mulu wins.
        let m3 = expand_bounded_w(vec![reg(Reg::D0), reg(Reg::D1), imm(3), reg(Reg::D2)])
            .unwrap();
        assert_eq!(
            shape(&m3),
            [
                "move.w d0,d2",
                "moveq #0,d0",
                "subq.w #1,d1",
                "bcs.s $mul$m$0$done",
                "$mul$m$0$loop:",
                "add.w d2,d0",
                "dbf d1,$mul$m$0$loop",
                "$mul$m$0$done:"
            ]
        );
        let m4 = expand_bounded_w(vec![reg(Reg::D0), reg(Reg::D1), imm(4), reg(Reg::D2)])
            .unwrap();
        assert_eq!(shape(&m4), ["mulu.w d1,d0"]);
    }

    // `.l` (and any non-`.w` suffix) refuses; `.w` is accepted.
    #[test]
    fn word_size_suffix_routing() {
        let mut c = 0;
        let long = expand_item(
            "mul_const",
            Some(Width::L),
            &[reg(Reg::D0), imm(66)],
            sp(),
            "m",
            &mut c,
            &ItemAuthor::User,
        );
        assert!(long.unwrap_err().message.contains("[mul.size]"));
        assert!(expand_const_w(vec![reg(Reg::D0), imm(66), reg(Reg::D1)]).is_ok());
    }
}
