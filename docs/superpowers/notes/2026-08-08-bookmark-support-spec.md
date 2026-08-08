# Bookmark support: typed computed `jmp`, `irq_frame.pc`, `@continuation` — spec (lane-bookmark, 2026-08-08)

Bookmark asks 3, 4, 5 from `aeon/docs/superpowers/2026-08-06-bookmark-implementation-sketch.md`
§6. Sigil-side only; aeon lands nothing until these exist. Companion to the
`@resumable` note (`2026-08-08-resumable-spec.md`, asks 1-2) and the `inout` facet
note — same discipline: **extend the existing contract machinery, do not build a
parallel checker.** Land order (smallest first): ask 5, then ask 3, then ask 4.

---

## Ask 5 — typed computed `jmp (aN) as Type`

SEMANTICS. The `as ContractType` dispatch bound on a computed TAIL (`jmp (aN)`)
means exactly what it means on a computed CALL (`jsr (aN) as Type`, shipped —
game_loop.emp:42, vblank.emp:45): the bound names the contract every installable
target of this indirect transfer satisfies, so the transitive-clobber closure
charges the bound's write surface into the transferring proc's effective set
instead of going ⊤. An unbounded `jmp (aN)` stays ⊤ (the load-bearing fact — an
un-annotated computed transfer can land anywhere).

**D5.1 — no new lowering was needed; the machinery was already mnemonic-generic.**
The dispatch-bound path never keyed on `jsr`:

- `parser::instr` attaches the trailing `as Name` to `InstrLine.dispatch_bound`
  after ANY instruction's operands (parser.rs ~2963) — `jmp (a1) as ObjRoutine`
  already parsed onto the instruction.
- `eval::asm::lower_instr_to_item` copies `dispatch_bound` into the CodeItem's
  `as_type` for every non-`movem`/non-`dc` instruction (asm.rs ~1281), so the
  bound survives lowering on a `jmp` exactly as on a `jsr`.
- `corpus_contracts::is_indirect_call` / `collect_indirect_sites` gate on
  `CALL_MNEMONICS ∪ TAIL_MNEMONICS` (both include the tail set), so a bounded
  `jmp (aN)` is collected as an indirect site with its bound, and
  `closure.rs` charges every indirect site uniformly (bounded → union the bound's
  clobbers; unbounded → `set_top`).

So the "add the spelling" work is: **prove the parity holds and pin it against a
future regression that silently drops the tail half.** `tests/typed_jmp.rs` is
that proof — parse, byte-neutrality (`as` is pure metadata), bounded-charges-only-
the-bound, under-declaration-fires-transitively, unbounded-goes-⊤ — each mirroring
its shipped `jsr` sibling in `corpus_contracts.rs`
(`bounded_indirect_is_not_top` / `unbounded_indirect_fires_unbounded`). No
production code changed; no golden/pin bytes move (a bound emits nothing).

**D5.2 — composes with `@resumable` for free.** A resumable decoder's terminal
continuation exit is `jmp (a3)`; the typed spelling `jmp (a3) as Cont` must pass
the `@resumable` stackless scan the untyped form already passes. It does, with no
change: `resumable::scan_stack_ops` reads the instruction's `mnemonic` and `ops`
only — `jmp` is neither a call nor a return mnemonic, and `Ind(a3)` (a3 ≠ a7) is
not a stack operand. The `as_type` is a separate CodeItem field the scan never
reads. Pinned by `resumable_terminal_typed_jmp_passes_the_stackless_scan`. This is
exactly what the resumable note's D1 anticipated ("the typed `jmp (aN) as Type`
spelling is bookmark ask 5 … until then the plain `jmp (a3)` is accepted").

---

## Ask 3 — the sanctioned stacked-frame accessor `irq_frame.pc`

SEMANTICS. Inside an interrupt handler, `irq_frame.pc` is a memory operand naming
the stacked interrupted-PC longword — the word an interrupt may read (range-check)
or rewrite (the supervisor-bookmark redirect). It lowers to `(disp, sp)` where the
displacement is DERIVED by the toolchain — `movem_save_bytes + 2` — instead of the
hand-maintained `62(sp)` magic S3K used (the +2 is the 68k group-1 exception
frame's SR word, which sits between the saved registers and the PC). If the
handler's `movem` save set ever changes, the offset re-derives; a hand offset
would silently rot.

**Surface.** A reserved two-segment operand `irq_frame.pc`, recognized in
`eval::asm::map_plain` before the ordinary `Item.field` / `Owner.label`
resolution. `move.l irq_frame.pc, d0` reads it; `move.l #New, irq_frame.pc` (or
`move.l d0, irq_frame.pc`) rewrites it — both are ordinary moves against the
resolved `(disp, sp)`. `irq_frame` is a reserved prefix; only `.pc` is a field.

**D3.1 — the offset is derived at eval time from a tracked full-save `movem`.** The
evaluator records the byte size of the most recent `movem.<sz> <list>, -(sp)`
(`irq_frame_save_bytes`, set in `lower_instr_to_item`'s movem branch) and clears it
on a `movem.<sz> (sp)+, <list>` restore (the exception frame is gone once the
registers are popped). Bodies lower in source order, so at an `irq_frame.pc` operand
this reflects every movem BEFORE it. The displacement is `save_bytes + 2`. Deriving
from the ACTUAL save set (not a hard-wired d0-a6 = 62) means a partial save
(`movem.l d0-d3,-(sp)` = 16 B yields `18(sp)`) is correct too — pinned by
`displacement_tracks_the_save_set`, which also asserts it is NOT 62.

**D3.2 — the two validity rules are the ask's nuance (b), enforced at the operand.**
- HANDLER CONTEXT — the enclosing proc must carry a `grants(...)` clause (a
  trust-root entered by hardware / a dispatcher; the checker looks the current proc
  up via `enclosing_owner` then `procs[name].grants`). An ordinary proc has no
  hardware-pushed exception frame to address: `[irqframe.not-handler]`.
- PRIOR FULL-SAVE MOVEM — without a recorded save there is no frame to derive from:
  `[irqframe.no-save]` (also fired when the accessor sits AFTER the restore — the
  anchor is cleared). Any field but `.pc` is `[irqframe.unknown-field]`. On the two
  validity errors the accessor still returns a best-effort `(62,sp)` operand, so
  exactly one clear diagnostic surfaces (no secondary "instruction dropped").

**D3.3 — nuance (a): a PC rewrite still satisfies `preserves(d0-a6)`, via an
authored exemption, NOT a broadened stack model.** A `d(sp)` STORE normally bails
the preserves slice's stack model (`preserves::sp_hazard` — a store COULD alias a
saved-register slot), which is why even a hand-written `move.l #x, 62(sp)` between a
movem save/restore reports `[proc.preserves-unverifiable]` today (measured). But the
`irq_frame.pc` store provably addresses the frame ABOVE the saves, so it cannot
alias a tracked slot. Rather than teach the shared (byte-frozen) stack model a
general depth-aware rule, the resolved accessor's instruction is authored
`ItemAuthor::IrqFrame` (a transient per-instruction flag set in the accessor,
consumed when the CodeItem is built), and `preserves::transfer` exempts an
`IrqFrame`-authored line from the sp-hazard bail — it then takes the normal
handling, where a store to memory writes no register and the movem round-trip proof
is untouched. Pinned by `a_pc_rewrite_preserves_the_registers` (no `[preserves`, no
`[stack.`, no `[proc.clobber`). This mirrors how `AssertDesugar`/`Context` authors
already license compiler-involved lines the checkers special-case; it is provably
corpus-neutral (the author only overrides `User`, never a splice/context author).

**D3.4 — byte- and behavior-neutral over the existing corpus.** No corpus proc uses
`irq_frame.*`, so `map_plain`'s interception, the movem tracking, and the preserves
exemption are all inert everywhere else. The new `ItemAuthor::IrqFrame` variant is
matched only by `is_irq_frame_access` (every other `ItemAuthor` site is
`matches!`/construction — no exhaustive break). Full `sigil-frontend-emp` suite
green (119 groups).
