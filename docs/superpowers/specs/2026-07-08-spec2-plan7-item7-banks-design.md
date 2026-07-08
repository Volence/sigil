# Design — Spec 2 · Plan 7 backlog #7: bank/window placement (T2-a) + the L-H.1 placement fix

Date: 2026-07-08 (Fable, design conversation with Volence). Status: **APPROVED by Volence,
2026-07-08** — scope, exhibit, and the section-attribute direction ratified in conversation;
technical calls delegated to Fable per standing practice and recorded here as D7.1–D7.7.
Inputs: research T2-a (aeon's highest-value unrepresentable idiom), ledger **L-H.1 /
S2-D13(e)** (cross-section origin staleness — this item's foundation), the shipped D2.23
link-time-value machinery (Value::LinkExpr, LinkAssert, lazily-folded messages), the shipped
`winptr` builtin (Plan 4, byte-proven), #4's `--map` region placement, and aeon's live sound
data (`games/sonic4/data/sound/dac_samples.asm`, `song_table.asm`, `main.asm:231-241` — the
18 straddle fatals and the twice-copied derivations).

## The one-paragraph thesis

The Z80 sees ROM through a 32KB window selected by a 9-bit latch; everything the sound
subsystem stores in ROM must therefore know its **bank id** (`(addr & $7F8000) >> 15`), its
**window pointer** (`(addr & $7FFF) | $8000`), and — critically — that it **never straddles
a 32KB boundary**. Aeon spells all three by hand: 18 `fatal` guards, two derivations copied
per call site, and `align $8000` boilerplate with comments begging editors not to remove it.
`.emp` should make the invariant a section property and the derivations checked builtins —
and because a bank id is a function of a **final** (post-relaxation) address, the item first
has to fix L-H.1 so section placement itself tells the truth. This is the flagged
prerequisite for the sound-subsystem migration.

## Decisions

- **D7.1 — scope: the bank property + the placement fix now; the packing linker DEFERRED.**
  (a) the `bank:` section attribute + `bankid()`/`winptr()` builtins + the inherent
  no-straddle invariant; (b) the L-H.1 final-size placement fix (a correctness prerequisite:
  a bank id computed from a stale baseline address is the silent-wrong-bytes class the
  here() fix exists to kill). (c) the ca65/rgbds-style **packing linker** (linker fits
  floating blobs into free bank space) is DEFERRED with a re-evaluation gate: **revisit when
  the sound migration starts** — the same demonstrated-need logic as jbcc/9d. Volence
  explicitly asked that (c) not be forgotten: it is recorded in the ledger below AND in
  S2-D13 when the spec integration lands.

- **D7.2 — banks are a SECTION PROPERTY, not a new construct.** Surface:

  ```
  section dac_bank (bank: $8000) {
      data Kick  = embed("dac/kick.bin")
      data Snare = embed("dac/snare.bin")
  }
  ```

  Rationale: the unit of the invariant is the co-resident GROUP (aeon packs nine drum
  samples into one bank so playback never re-latches), and the group/placement unit the
  language already has is `section` — a new `bank {}` construct would duplicate section
  semantics (placement, cpu, map interaction) to add one invariant, the drift-by-duplication
  failure mode. Semantics of `bank: N` (N a power of two, evaluated comptime int):
  the section's contents must not cross an N-boundary in LMA (ROM) space. **Placement bumps
  to the next boundary ONLY when the section would otherwise straddle** (aeon's
  always-`align $8000` wastes up to 32KB; the invariant is no-straddle, alignment is just
  one strategy). Contents larger than N are unsatisfiable → loud error in the §7.3 budget
  style ("over by K bytes"). Composes with `--map` regions (a bank section placed in a
  region satisfies BOTH the region budget and the boundary invariant) and with
  auto-sequential placement.

- **D7.3 — `bankid()` is a link-time value on the D2.23 machinery; NO new fixup-kind zoo.**
  `bankid(Label)` yields `(Sym(Label) & $7F8000) >> 15` as a `Value::LinkExpr` — the
  residual-expression trees the here() fix built, folded by the linker once addresses are
  final. Consequences, all inherited for free: `ensure` guards over bank ids defer to
  link-time assertions automatically; comptime-required contexts get the loud
  `[here.provisional]`-class refusal (spelled `[bank.provisional]` or reusing the existing
  code — implementation's call, message must steer); and **data-cell emission of link
  expressions — ledger S2-D13(f) — is UN-DEFERRED by this item** (its first real customer):
  a `Cell` carrying an `ir::Expr` + width, folded at link, range-checked on write. The
  alternative (a dedicated `BankId` fixup kind per derivation, the `winptr` pattern) is
  rejected: one general mechanism beats a per-idiom zoo, and the sound migration will surface
  more derived-address idioms. `winptr()` itself stays EXACTLY as shipped (byte-proven; no
  churn) — a later quality pass MAY re-express it over link-exprs if byte-diff-clean, not
  this item. The latch mask/shift constants (`$7F8000`/15) are the Genesis cartridge
  banking scheme; they live in ONE place (the builtin), not user code.

- **D7.4 — the L-H.1 fix: placement on FINAL sizes.** The invariant: **a section's placed
  base address derives from its predecessors' FINAL (post-relaxation) sizes.** Placement and
  relaxation are interdependent (moving a section changes cross-section branch distances),
  so they iterate to a joint fixpoint — termination by the same grow-only argument the
  relaxation ladder already uses. Explicitly pinned addresses (a `vma:` attribute, map
  region origins, org-style pins) stay pins. Any overlap that survives final placement is a
  loud link error — never silent image corruption (today's failure mode). This retroactively
  strengthens every label, `here()`, region budget, and guard — D2.23's "here() is exactly
  as accurate as a label" invariant becomes unconditionally true across sections. Bank
  boundary bumping (D7.2) happens IN this final-placement pass, so bank ids and the
  no-straddle assertion see true addresses by construction.

- **D7.5 — the no-straddle invariant is a generated link assertion.** For each `bank:`
  section the compiler emits the LinkAssert equivalent of
  `first_byte / N == last_byte / N` (evaluated post-placement), with a message naming the
  section, its final extent, and the boundary it crosses. Always-on — the point is that
  EVERY bank is checked, not just the ones someone remembered to guard (the T2-b lesson).

- **D7.6 — exhibit: the dac_samples shape (Volence-ratified).** A faithful `.emp` port of
  aeon's `dac_samples.asm` structure into the `examples/game` corpus: one `(bank: $8000)`
  section holding multiple PCM blobs (`embed`), the `SND_*_BANK`/`SND_*_PTR` constants via
  `bankid()`/`winptr()` emitted into a data table, lengths from the embedded values'
  comptime sizes, plus a deliberately-overflowing negative probe (straddle → loud). Layout
  byte-argued against the aeon original's scheme (alignment/padding differences per D7.2 are
  EXPECTED and documented, not silently absorbed — the exhibit argues equivalence of the
  derived VALUES, not byte-identity of padding). Acceptance pinned in a CLI test per house
  pattern. Programs not using `bank:` must remain byte-identical to master EXCEPT where the
  L-H.1 fix corrects a previously-silently-wrong overlap (any such divergence must be
  itemized and argued, here-fix precedent).

- **D7.7 — out of scope (this item).** The packing linker (D7.1c, gated); multi-latch /
  SRAM / mapper schemes beyond the single 9-bit cartridge latch; Z80-driver-side consumption
  patterns (the sound migration itself — though `bankid()` values spliced into Z80
  immediates should fall out of the general mechanism, verify with one probe, don't build
  bespoke support); `sizeof(data-item)` (still inherent via types/comptime values, D2.20).

## Watch-outs for the implementer (from the controller's context)

- The L-H.1 seam: `resolve_layout`/`place_sections` in sigil-link + the baked `next_lma`
  chain in `lower_module` (lower/mod.rs) — the fix likely lives at link (recompute bases
  from final lengths), NOT by threading new state through lowering. Verify the AS-frontend
  path (`sigil-frontend-as`) is placement-compatible — the s4.bin harness (m1d_rom etc.,
  currently 4 allowlisted reds for an UNRELATED aeon strlen drift) is the regression net;
  zero NEW failures ever.
- `section_attrs` (lower/mod.rs:548) is where `bank:` parses (cpu/vma precedent; unknown
  attrs currently diagnose — the attr must be threaded to the link layer with the section).
- The general link-expr data cell (D7.3) should reuse `Value::LinkExpr`'s lifting
  (eval/expr.rs) and the LinkAssert fold path — read the here-fix design
  (2026-07-08-spec2-plan7-here-relaxation-fix-design.md, D-H.1–D-H.9) FIRST.
- Rule-of-three note from #9 stands: the table-emit shape wants extraction next time that
  seam is touched — do it if this item touches it, don't detour otherwise.
- Process: worktree off master, strict TDD w/ recorded RED, commit-per-task, two-stage
  reviews on load-bearing tasks, whole-branch adversarial review + byte-diff probes vs
  master across examples/ + examples/game, controller verifies independently, NO merge
  without a Volence checkpoint.

## Ledger (this item's deferrals)

| id | item | gate |
|---|---|---|
| L7.1 | Packing linker (floating blobs auto-fitted into bank free space, ca65/rgbds-style) | **Re-evaluate when the sound migration starts** (Volence: "don't forget c") |
| L7.2 | Multi-latch / SRAM / mapper banking schemes | a demanding port |
| L7.3 | `winptr` re-expressed over general link-exprs (byte-diff-clean quality pass) | next quality tranche touching it |
| L7.4 | Z80-driver-side bank consumption idioms (latch-write sequences, re-bank protocols) | the sound migration itself |
