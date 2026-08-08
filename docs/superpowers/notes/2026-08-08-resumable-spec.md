# The `@resumable` (stackless) proc attribute + exported extent symbol — spec (lane-resumable, 2026-08-08)

Bookmark asks 1 & 2 from `aeon/docs/superpowers/2026-08-06-bookmark-implementation-sketch.md` §6.
Sigil-side only; aeon lands nothing until these exist. Companion to the `inout`
facet note (`2026-08-08-inout-spec.md`) — same "extend the existing contract
machinery, do not build a parallel checker" discipline.

## Ask 1 — `@resumable`: the stackless contract

SEMANTICS. A `@resumable` proc is a supervisor-bookmarkable region: an interrupt
may snapshot its whole live state (registers + CCR), rewrite the stacked return
PC to a banking stub, and resume it later from any interrupt depth. Sound ONLY if
the body touches sp NOWHERE — no return address on the stack to dangle, no frame
half-built when preemption lands, all live state in registers. `@resumable` is the
CHECKED form of the decoder contract's "NO stack access, ALL state in registers"
comment.

FORBIDDEN (each build-fatal, `[resumable.stack-op]`, ERROR-tier, NEVER softened by
`@as_compat` — the whole VBlank-bookmark safety argument rests on this property, so
a faithful port cannot opt out of it):

- a CALL — `bsr` / `jsr` / `jbsr` — pushes a return address;
- `pea` pushes an address; `link` / `unlk` build / tear a frame;
- a RETURN — `rts` / `rte` / `rtr` / `rtd` — pops the return address off the stack;
- any operand naming sp/a7 — `-(sp)` push, `(sp)+` pop, `(sp)` / `d(sp)` / `Sym(sp)`
  (symbolic-displacement) / `(sp,Xn)` alias forms, sp as a PC-relative index
  (`Sym(pc,sp.size)`), a bare `sp` operand, or a `movem` register list containing a7.

PERMITTED (and must stay permitted): the terminal `jmp (aN)` (aN ≠ a7) continuation
exit — a computed transfer through an address register, no stack touch. The typed
`jmp (aN) as Type` spelling is bookmark ask 5 (a separate later ask); today the exit
rides the plain `jmp (aN)` form, which the scan simply does not flag. No weakening
of the stackless guarantee was needed to accept it — `Ind(aN)` for aN ≠ a7 is not a
stack operand, full stop.

REGISTER-STATE SET. Discharged through the EXISTING contract machinery, not a new
reglist argument on the attribute: the set is the proc's ordinary contract (params
∪ `clobbers` ∪ `out`), and "anything live outside the declared set is an error" is
exactly `[proc.clobber-undeclared]`. `@resumable` (a) REQUIRES a `clobbers(...)`
declaration (`[resumable.contract-required]` — without it there is no set to bound
liveness against) and (b) makes `check_clobbers` MANDATORY, running it even under
`@as_compat` (a resumable proc is a strict new contract, not a faithful port).

SCOPE GUARD. 68k only. `@resumable` on a Z80 proc is `[resumable.z80-unsupported]`
(the stack model + bookmark mechanism are 68k), matching the `inout` facet's Z80
scope guard.

## Ask 2 — exported extent symbols

For each `@resumable` proc, an exported extent label `Proc.__end` is emitted at the
byte immediately past the body, so a consumer compiles `[Proc, Proc.__end)` PC range
checks from toolchain symbols instead of a hand-maintained sentinel label.

## Measured decisions (lane-resumable, 2026-08-08)

**D1 — the attribute carries NO reglist; the register set is the existing
contract.** The sketch phrases ask 1 as "declares the register state set (here
d0-d2/a0-a2 + ccr)", which reads like an attribute argument. Implemented instead as
a MARKER attribute (no args, like `@noreturn`), because the register set is already
declarable — and declared — via params/`clobbers`/`out`, and the ask's own next
clause ("anything live outside the declared set is an error per Sigil's existing
liveness/contract machinery — do not build a parallel checker") mandates routing
through `check_clobbers` rather than a new reglist checker. So `@resumable(d0)` is a
parse error (`[attr.form]`). This is the `inout` A2 move: discharge an obligation
through existing machinery rather than a new dataflow.

**D2 — the scan reads the evaluated/spliced `CodeBuf`, so stack ops that only
appear after evaluation are caught.** `scan_stack_ops` walks `buf.items` — the
`eval_proc_body` output, post-`with`-splice and post-comptime-template but
pre-backend-encoding — not AST source tokens. A `with <ctx> { }` bracket that
splices an acquire `move.l d7,-(sp)` is caught exactly like a literal push —
pinned by `tests/resumable.rs::a_with_bracket_that_emits_a_push_is_caught`. This
is why the check lives after eval and not at parse. (Wording precise: it is the
evaluated CodeBuf, not the backend-encoded byte stream.)

**D3 — `rts`-family is forbidden, beyond the sketch's enumerated list.** The sketch
enumerates `bsr/jsr/jbsr/pea/link/movem`; the implementation ALSO forbids the return
family (`rts`/`rte`/`rtr`/`rtd`), because a return READS the return address off the
stack and §1 clause 2 states the contract exits "`jmp (a3)` ... never touching sp".
A resumable body that ran `rts` would return to a word the bookmark mechanism
assumes is absent — the exact defect the contract exists to forbid. The Phase-2
draft body (plan Task 2) already exits `jmp (a3)`, so this rejects nothing real.

**D4 — the extent symbol rides the exported-LABEL path, no rename-table change.**
`Proc.__end` is emitted as an ordinary label at body-end. `canonicalize_name`'s
dotted-owner rule (the `export .name:` → `Owner.name` mechanism) module-qualifies it
to `<module>.Proc.__end` exactly as it does the proc's own exported labels — so a
consumer references it as `ZX0R_Decompress.__end` after `use`-ing the proc, with no
second `use` and no addition to the export index / rename map. The dot spelling
(`Proc.__end`, not a flat `Proc__end`) is deliberate: a flat suffix would be a fresh
top-level symbol ABSENT from the rename map and thus not cross-module-resolvable
without touching `resolve/imports.rs`; the dotted owner-local form resolves for free.
Pinned by `extent_symbol_resolves_from_another_module` (both the reference side and
the definition side canonicalize to the same string). The `.__end` owner-local name
is RESERVED: a body-defined `export .__end:` hygiene-resolves to the same
`Proc.__end` symbol and would silently mint a duplicate at a different offset, so it
is rejected (`[resumable.extent-reserved]`); a non-export `.__end:` mangles to
`$mod$Proc$__end` and does not collide (pinned both ways).

**D5 — byte- and symbol-neutral over the whole existing corpus.** No corpus proc is
`@resumable`, so neither feature emits anything anywhere but a resumable proc's own
module: the extent label is byte-free (labels emit no bytes) and appears only for
resumable procs, and the two new checks + the mandatory-`check_clobbers` widening are
gated on `is_resumable()`. The seven shipped shapes are unaffected.

## Closure-gate reconciliation (lane-resumable-closure, 2026-08-08)

**D6 — the terminal `jmp (aN)` continuation is a BOUNDED terminator in the
whole-corpus closure gate, not an unbounded ⊤ tail-transfer.** The first real
`@resumable` proc tripped `corpus_contracts.rs`: `collect_indirect_sites` modeled a
computed `jmp (aN)` as an unbounded indirect dispatch (`None` bound), so the proc's
effective clobber set became ⊤ and `[proc.clobber-undeclared]` fired unbounded
(`ZX0R_Decompress UNBOUNDED`) — even with the clean reference contract. The per-file
scan (D1-D5) and the closure gate were never reconciled; D5's byte-neutrality held
only because no corpus proc was `@resumable` yet.

Fix (extend, don't parallel): `collect_indirect_sites(body, is_resumable)` now skips
the site for a resumable proc's continuation exit — `is_resumable_continuation_exit`:
an UNTYPED (`dispatch_bound.is_none()`) indirect TAIL transfer (`jmp`/`bra`/`jbra`)
through a bare address register that is NOT a7. It is credited like a return/`@noreturn`
terminator (contributes no ⊤ site), because a resumable proc's register budget is
already pinned (mandatory `clobbers` + the stackless scan) and the exit leaves to a
caller-loaded continuation, not into unknown code. Keyed on the `@resumable` attribute,
never a proc name; the `@allow` census stays empty.

Three deliberate exclusions keep it sound and forward-compatible:
- **TAIL only** — an indirect CALL (`jsr (aN)`) in a resumable proc is already
  `[resumable.stack-op]` (it pushes), so it is not a shape to credit.
- **aN != a7** — `jmp (sp)` READS the stack (also `[resumable.stack-op]`); excluding
  it (and any unrecognizable/spliced operand) makes the credit fail-safe.
- **UNTYPED only** — a `jmp (aN) as Type` (ask 5) is bounded by the dispatch TYPE's
  effect through the EXISTING indirect-site machinery, which is stricter. Excluding
  the typed form lets that bound SUPERSEDE this credit cleanly the day ask 5 lands —
  no double-handling. Pinned by
  `resumable_typed_exit_is_bounded_by_the_type_not_the_attribute`.

Tests (`tests/corpus_contracts.rs`): `resumable_continuation_exit_is_not_unbounded`
(clean contract passes), `nonresumable_jmp_indirect_still_fires_unbounded` (the credit
does not leak to a non-resumable proc with the same shape),
`resumable_jmp_through_sp_is_not_credited` (a7 exclusion), and the typed-supersede test.

**D6a — a written param cursor (`(a0)+`) must be declared `out`/`clobbers` in a
`@resumable` proc.** The closure gate does NOT excuse a written PARAM the way
`check_clobbers` does (a param declares an input, not a licence to destroy it). So the
real ZX0R — whose `a0`/`a1` cursors ADVANCE past their buffers — must declare
`out(a0, a1)` (the draft header's "Out: a0/a1 past ends"), not merely list them as
params. This was masked before the D6 fix because a ⊤ effect subsumes every concrete
register; bounding the exit surfaced it. A consumer note, not a Sigil change.

## Deferred (bookmark asks not in this lane)

Asks 3 (sanctioned stacked-frame accessor `irq_frame.pc`), 4 (manufactured-frame
resume / `@continuation` proc form), 5 (typed computed `jmp (aN) as Type`), and 6
(module registration + byte-changing ritual) are OUT OF SCOPE here. Ask 5 in
particular is what would let the continuation exit be spelled `jmp (a3) as Cont`;
until then the plain `jmp (a3)` is accepted as the stackless exit (D1/scan).
