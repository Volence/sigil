# M1.D T3 — jmp/jsr front-end width selection : live-asl probe matrix

**Tool:** `aeon/tools/asl` (asl 1.42 Beta Bld 212), `-cpu 68000 -q -L -U`, then `p2bin`.
**Date:** 2026-07-04. Re-runnable: `scratchpad/t3_probe.sh` (probe sources reproduced inline).

## Why this exists

T3 moves jmp/jsr abs.w/abs.l width selection out of the linker's `resolve_layout`
and into the front-end pass loop. The spec (§T3) mandated a probe of the one open
semantic before design freeze:

> what width does asl assume for a symbol *unknown in the current pass* (expected:
> the long/pessimistic form — verify), and does a width-depends-on-own-address
> construct oscillate or converge in asl.

**The probe REFUTES the spec's "expected pessimistic-long" hypothesis.** asl selects
the **LEAST fixpoint** — it assumes the **short/optimistic abs.w** form for an
unknown-this-pass forward symbol and grows to abs.l only when the *resolved* value
forces it. This is the same grow-only direction as the linker's `resolve_layout`,
and it makes the T3 front-end mechanism simpler than anticipated (below).

Per the process rule ("never trust a spec/doc claim over a probe"), the design
follows the probe, not the spec's stated expectation.

## Baseline width = the pinned `asl_width_rule` (jmp/jsr, all confirmed)

| probe | source (after `cpu 68000; padding off; org 0`) | listing | width |
|---|---|---|---|
| p1 jmp low     | `Low: equ $100` / `jmp Low`     | `4EF8 0100`      | abs.w |
| p2 jmp high    | `High: equ $10000` / `jmp High` | `4EF9 0001 0000` | abs.l |
| p3 jsr low     | `Low: equ $100` / `jsr Low`     | `4EB8 0100`      | abs.w |
| p4 jsr high    | `High: equ $10000` / `jsr High` | `4EB9 0001 0000` | abs.l |

`.l` opcode = `.w | 1` (jmp 4EF8/4EF9, jsr 4EB8/4EB9). Matches the M1.B pinned rule.

## Final width is resolved-VALUE-based (forward refs)

| probe | source | listing | note |
|---|---|---|---|
| p5 fwd low  | `jmp Fwd` / `nop` / `Fwd: rts`         | `4EF8 0006`      | Fwd=$0006 → abs.w |
| p6 fwd high | `jmp Fwd` / `ds.b $8100` / `Fwd: rts`  | `4EF9 0000 8106` | Fwd=$8106 → abs.l |

p5/p6 prove asl resolves the forward target's *final* address before choosing width.
p6's Fwd=$8106 (not $8104) shows the jmp itself is 6 bytes (abs.l) in the final
layout — the width and the address are mutually consistent at the fixpoint.

## Cascade converges to the LEAST fixpoint

| probe | source | listing |
|---|---|---|
| p7 | `Hi: equ $12FF00` / `jmp A` / `jmp Hi` / `nop` / `A: rts` | `jmp A`→`4EF8 000C`, `jmp Hi`→`4EF9 0012 FF00` |

`jmp A` stays abs.w with A resolved to $000C — i.e. A is positioned *after* the grown
`jmp Hi` (4+6+2 = A@$0C), yet asl still picks abs.w for it because $0C is low. Minimal
widths, value-based on the converged addresses.

## THE DECISIVE PROBE — least vs greatest fixpoint

A self-referential jmp whose target sits at the abs.w/abs.l boundary, positioned so
that **both** abs.w and abs.l are self-consistent fixpoints:

```
	cpu 68000
	padding off
	org $7FFA
	jmp T
T:	rts
```

- abs.w (4 bytes): `T = $7FFA + 4 = $7FFE` → `asl_width_rule($7FFE) = W` — consistent (LEAST).
- abs.l (6 bytes): `T = $7FFA + 6 = $8000` → `asl_width_rule($8000) = L` — consistent (GREATEST).

**asl emits `4EF8 7FFE` — abs.w, T=$7FFE. The LEAST fixpoint.** It did *not* pick the
greatest (abs.l/$8000). Therefore asl assumes the **short** form for the unknown
forward symbol and grows only when forced.

The identical construction for an absolute EA (`org $7FFA; lea T,a0; T: rts`) gives
`41F8 7FFE` — abs.w, LEAST fixpoint too. So the same grow-only direction governs the
absolute-EA class (T2's `abs_ea_from_expr`), not just jmp/jsr.

## Consequence for the T3 mechanism (simpler than the spec anticipated)

Because asl is grow-only/least-fixpoint, the front-end does **not** need explicit
per-site width persistence across passes. Optimistic-abs.w start makes the existing
`env == prev` multi-pass loop *inherently* grow-only:

1. Unknown-this-pass target (Poison) → **abs.w** (smallest).
2. Earlier sites only ever grow (abs.w→abs.l), so every label address is
   monotone-nondecreasing across passes.
3. Monotone addresses → monotone widths → no width ever shrinks → no oscillation →
   the fixpoint reached is the LEAST one = asl's.

So T3's jmp/jsr lowering mirrors `abs_ea_from_expr` with **one change from the T2
template**: the Poison case picks **abs.w (optimistic)**, not abs.l (pessimistic).
The front-end emits a finished `Data` fragment (opcode + `Abs16Be`/`Abs32Be` fixup —
link resolves the value as today) and advances the cursor by the *true* width (4/6).

This also fixes T2's `abs_ea_from_expr`, whose Poison→abs.l (pessimistic/greatest)
is technically asl-unfaithful at a near-boundary EA (harmless for aeon — the 6 real
EA sites target high level-data, unconditional abs.l — but corrected for consistency
in T3, since both use the shared width machinery).

## $FF8000 non-monotone region is unreachable for aeon jmp/jsr

`asl_width_rule` is non-monotone at the $FF8000 sign-extension wrap (W below $8000,
L on [$8000,$FF7FFF], W again on [$FF8000,$FFFFFF]) — where asl 1.42's bidirectional
relaxer itself oscillates (see `sigil-link/relax.rs` grow-only caveat). Optimistic-
abs.w-no-persistence *could* oscillate there. It is unreachable in aeon:

All **22 distinct** bare `jmp`/`jsr` symbol targets in `games/sonic4` are ROM **code
labels** (AnimateSprite, PState_Roll, Sonic_LoadArt, Perform_DPLC, …) — none is a RAM
equate ≥$FF8000. RAM jumps use register-indirect `jmp (aN)` (an EA operand, a
different lowering path). So no bare jmp/jsr is self-referentially width-affected in
the wrap region. `PASS_CAP` backstops any pathological oscillation regardless — same
posture as the linker.

## Design decisions (settled)

- **Convergence direction:** grow-only via **optimistic abs.w start** (Poison→abs.w),
  advance cursor by true width. Probe-proven = asl's least fixpoint.
- **Fragment representation:** the front-end emits a **finished `Fragment::Data`**
  (opcode word + `Abs16Be`/`Abs32Be` fixup carrying the target expr; link resolves the
  value against the final symbol table, exactly as today) — NOT `Fragment::JmpJsrSym`.
  Mirrors `abs_ea_from_expr`; keeps link's tested, phase-aware fixup resolution;
  advancing by the true width makes `phys_base`/downstream LMAs correct by construction
  (closes the unflagged half of F2). `resolve_layout` sees no `JmpJsrSym` on the
  front-end path → identity (the "zero growth expected" verification, trivially held);
  it stays the live grow-only relaxer for hand-built IR (m1b_gate). The Org+JmpJsrSym
  guard can no longer fire on the front-end path — so the real object bank (`org
  $10000` + bare jmp/jsr + parallax `org` back-patch in one section) now assembles.
- **PASS_CAP:** raise (width growth consumes convergence passes); `env == prev` is the
  real convergence signal, PASS_CAP only backstops runaway.

## Snippet goldens to add (real asl, byte-affecting)

`t3_jmp_abs_w` (low→4EF8), `t3_jmp_abs_l` (high→4EF9), `t3_jsr_abs_w`/`t3_jsr_abs_l`,
`t3_jmp_fwd_low` (p5), `t3_jmp_fwd_high` (p6), `t3_jmp_boundary_selfref` (the decisive
`org $7FFA; jmp T; T:` → `4EF8 7FFE`). `gen_snippet_vectors` must churn only these.
Acceptance additionally: the two `stale_fold_repro.rs` `#[ignore]` tests flip green.
