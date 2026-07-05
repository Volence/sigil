# M1.D T2 — bare-symbol absolute EA + `END` directive : live-asl probe matrix

**Tool:** `aeon/tools/asl` (asl 1.42 Beta Bld 212), `-cpu 68000 -q -L -U`, then `p2bin`.
**Date:** 2026-07-04. Re-runnable: probe sources reproduced inline below.

## Why this exists

Post-T1 recon = 7 diagnostics: **6 EA sites over 3 symbols** + **1 `END`**.
The 6 EA sites are all bare-symbol absolute source EAs in
`test/ojz_scroll_test.asm` (pulled in by `games/sonic4/main.asm:415`):

```
:20   lea BGND_Palette, a0          (BGND_Palette ×1)
:27   lea OJZ_Palette, a0           (OJZ_Palette ×1)
:36   lea OJZ_Act1_Descriptor, a0   (OJZ_Act1_Descriptor ×4: :36,:47,:98,:117)
```

`lea (Palette_Buffer).w, a1` (explicit `.w`) already assembles (M1.C T5b); only
the **bare-symbol** form (no `.w`/`.l` suffix) is the gap — the front-end
rejects it in `convert_one_atom_m68k` (`eval.rs:2584`, "out of scope for T5").
The `END` at `main.asm:446` falls through to m68k instruction lowering →
"`END` is not a recognized 68000 mnemonic".

## Absolute EA width = the pinned `asl_width_rule` (all confirmed)

asl selects abs.w iff addr ∈ `[0,$7FFF] ∪ [$FF8000,$FFFFFF]`, else abs.l —
the **same rule** already pinned + boundary-swept for jmp/jsr in
`sigil-link/relax.rs:29` (M1.B). So a bare-symbol absolute EA is a
**width-variable instruction** (4 or 6 bytes), same class as jmp/jsr.

| probe | source (after `cpu 68000; padding off; org 0`) | bytes | width |
|---|---|---|---|
| lea_low     | `Low: equ $100` / `lea Low,a0` / `rts`        | `41F8 0100`      | abs.w |
| lea_high    | `High: equ $10000` / `lea High,a0` / `rts`    | `41F9 00010000`  | abs.l |
| lea_fwd_low | `lea Target,a0` / `rts` / `Target: dc.w 0`    | `41F8 0006`      | abs.w (forward ref fully resolved) |
| lea_8000    | `S: equ $8000` / `lea S,a0`                   | `41F9 00008000`  | abs.l ($8000 ∉ low range) |
| lea_ff8000  | `S: equ $FF8000` / `lea S,a0`                 | `41F8 8000`      | abs.w (sign-extends to $FF8000) |
| move_low    | `L: equ $100` / `move.w L,d0`                 | `3038 0100`      | abs.w (rule is EA-general, not lea-specific) |
| move_high   | `H: equ $10000` / `move.w H,d0`               | `3039 00010000`  | abs.l |

Boundaries match `relax.rs` `asl_width_rule` exactly: `$7FFF`→W, `$8000`→L,
`$FF8000`→W. `lea_fwd_low` proves asl fully resolves the address before
choosing width (forward ref → abs.w because the resolved value is low).

## `END` directive = emission no-op (both forms)

| probe | source | bytes | meaning |
|---|---|---|---|
| end_bare | `nop` / `END`         | `4E71` | `END` alone emits nothing |
| end_arg  | `Start: nop` / `END Start` | `4E71` | `END <sym>` (entry-point arg) also emits nothing |

The listing shows `END`/`END Start` occupying zero bytes (`5/ 2 :   END Start`,
no machine code column). Aeon's only use is the bare `END` at `main.asm:446`.
Handler = no-op (do not evaluate/require the optional argument for bytes).

## Design decision (settled)

**Fold + width-select in the front-end now** (not a deferred resolve_layout
fragment): in `convert_one_atom_m68k`, the bare-symbol / bare-expression
absolute-EA path folds the address from the current-pass env, picks abs.w/abs.l
via `asl_width_rule`, and emits `M68kOperand::AbsW`/`AbsL`. The instruction's
Data fragment carries the true encoded length, so the cursor advances correctly
and the existing multi-pass fixpoint converges. Rationale:

- **asl-faithful**: matches every probe row above.
- **Reuses the one pinned rule** rather than adding parallel linker machinery
  that T3 would immediately replace (T3 moves *all* width selection —
  jmp/jsr included — into the front-end pass loop; this is that mechanism for
  the absolute-EA class, a stepping-stone, not throwaway).
- **Unknown-this-pass symbol → pessimistic abs.l** (matches asl's
  forward-symbol width guess; keeps the fixpoint shrink-only during
  convergence — never grow-after-fold, which is the F2 hazard direction).
- The 6 real sites all target high-address level data (`OJZ_Palette` etc. > $8000)
  → unconditional abs.l → no per-pass width flip. Byte-exact **value** (F2's
  stale-address concern) remains a T3/T4 matter; T2's bar is recon-0 + goldens.

**Shared width rule**: relocate `asl_width_rule` + `AbsWidth` from `sigil-link`
into `sigil-ir` (the common upstream crate both the front-end and the linker
depend on); `sigil-link` re-exports them so its code and the M1.B boundary-sweep
tests are unchanged. The front-end cannot depend on `sigil-link` (one-way crate
graph); a single shared definition avoids a drift-prone copy and is exactly what
T3 needs in both places.

## Implementation checklist (semantics → code)

1. `sigil-ir`: define `AbsWidth` + `asl_width_rule` (moved from `relax.rs`);
   `sigil-link` re-exports (keep `relax.rs` boundary-sweep tests green).
2. `convert_one_atom_m68k`: replace the two "out of scope" errors
   (`Value(Expr::Sym)` fall-through + `Value(_)`) with a shared
   `abs_ea_from_expr` helper: qualify → fold over u32 → `asl_width_rule` →
   `AbsW`/`AbsL` (unresolved-this-pass → abs.l).
3. `dispatch`: `"end" | "END" => {}` no-op arm (exact-case, like `BINCLUDE`;
   does not collide with the `endif`/`endm`/`endr`/`endcase` block closers,
   which are handled in block scanning, not dispatch).
4. Snippet goldens (real asl): `t2_lea_abs_w` (low sym → abs.w),
   `t2_lea_abs_l` (high sym → abs.l), `t2_lea_abs_boundary` ($FF8000 → abs.w),
   `t2_move_abs_w`/`t2_move_abs_l` (EA-general), `t2_end_noop`. `gen_snippet_vectors`
   must churn only these new blocks (non-circularity invariant).
5. Acceptance: recon (`m1c_full`) reaches **0 diagnostics** → arms `m1c_rom` (T4).
