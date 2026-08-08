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
