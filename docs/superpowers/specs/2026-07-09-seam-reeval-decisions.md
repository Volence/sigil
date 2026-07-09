# Seam re-evaluation session — decision record (2026-07-09, overnight)

Fable's rulings for the ratified three-item re-evaluation session (see
`notes/2026-07-09-overnight-handoff-1-seam-reeval.md`). Volence was asleep; decisions made
autonomously per the standing directive, each with rationale. Implementation on worktree
branch `seam-reeval`, UNMERGED pending the morning checkpoint.

## Item B — bare cross-seam equ reads: RULING = option (ii), `extern("NAME")`

### The fact-check (constrains the decision; verified by recon + Fable spot-check)

**AS-side equs do NOT reach the linker's symbol table — hard no, by construction:**

- The only carrier for non-address symbols into the link is `Section::equ_syms`
  (`sigil-ir/src/lib.rs:215`), populated solely via `IrBuilder::add_equ_sym`
  (`builder.rs:167`). The ONLY caller in the workspace is the `.emp` lowering
  (`sigil-frontend-emp/src/lower/mod.rs:531`). `sigil-frontend-as` never calls it.
- AS `equ`/`=` land in `directive_equate` (`sigil-frontend-as/src/eval.rs:1905-1925`),
  which writes `self.env` (int) or `self.str_env` (string) — per-unit fold state only,
  never serialized into the emitted `Module`. Labels DO cross (`define_label`,
  eval.rs:1768 → `Section::labels` → link Pass 1 seeding, `sigil-link/src/lib.rs:59-75`).
- The linker's Pass 1b (`lib.rs:77-109`, R-T0.3) already seeds `equ_syms` into the same
  dup-checked `SymbolTable` before fixups — the hard half of the plumbing EXISTS; the AS
  frontend just never feeds it.
- Bonus correction to the T2-era account: the `label_ctx` bareword→label fallback never
  applied to builtin args at all — `bankid`/`winptr`/`ensure` are dispatched ahead of
  `bind_args` and evaluate args with plain `eval_expr` (`call.rs:53-95`,
  `builtins.rs:457,518`, `guards.rs:97`). The fallback fires only for user `comptime fn`
  args, value-call args, and label-value literal positions. `bankid("L")`'s quoted string
  sidesteps name resolution entirely — which is why the idiom worked.

### The ruling and rationale

**(ii) `extern("NAME")` — a raw link-symbol passthrough builtin — plus the AS-side equ
export it requires.** Rationale:

1. **Explicit-beats-spooky** (DSM.7's own words): explicit at the usage site, same
   spelling family as `bankid("…")`/`winptr("…")`. A typo'd name defers to link and dies
   loudly as an undefined symbol — and Item C's diagnostic fix (below) makes that failure
   name the missing symbol. B and C compose.
2. **Option (i) is dead on the facts**: its premise (extend the builtin-arg fallback) was
   inaccurate — there is no builtin-arg fallback to extend — and a bareword fallback in
   `ensure` operands would still need the same AS-export plumbing to see an equ, while
   adding spooky-resolution semantics to every nested expression position (`in_label_ctx`
   broadening hazard, `eval/mod.rs:377-393`). All hazard, no gain.
3. **Option (iii) keeps paying the tax**: the bankid-label idiom requires a bank-aligned
   LABEL proxy to exist (lucky structure), has cost three tranches, and the 68k engine-port
   campaign multiplies the exposure. Ratifying it would spec a workaround as a feature.
4. **The plumbing is moderate, not large** (the handoff's escape condition does not fire):
   Pass 1b, the EquSym carrier, dup checks, and equ folding all exist; the AS side adds one
   export site.

### Design (frozen)

- **`extern(name)`** (`.emp` builtin): `name` must evaluate to a comptime string; returns
  `Value::LinkExpr(Expr::Sym(name))` — raw value passthrough, NO Genesis mask/shift
  (unlike `bankid`/`winptr`). Works for any link-visible symbol (AS equs, AS labels, .emp
  equs/labels) — the table has no kind distinction and none is wanted. Diagnostics mirror
  the house builtin taxonomy (arity / non-string arg / empty name), all loud. In
  comptime-required positions it behaves as every LinkExpr does (refusal, not a wrong
  value).
- **AS equ export**: `directive_equate`'s int arm additionally exports
  `EquSym { name: qualified, expr: Expr::Int(final_value), span }` via the builder —
  exactly once per symbol with the final folded value (multi-pass discipline is the
  implementer's to verify against the crate's pass structure). String equs stay
  front-end-only (§7.4 unchanged). `set`/`:=` mutable symbols are NOT exported (no
  meaningful single value; also not part of the constants-read use case). Blanket export
  is intentional — collisions with `.emp`-side names surface as the existing loud
  "redefined" link error, which is the correct behavior for a genuinely double-defined
  constant. Contingency if the full corpus reveals a legitimate same-name-same-value dup
  pattern: permit identical-value equ re-definition in Pass 1b, recorded as a deviation.
- **Byte-identity invariant**: export adds symbols, never bytes — all existing ROM gates
  must stay byte-identical with zero allowlist changes.
- **Migration of existing files**: mt_bank.emp / sfx_bank.emp keep the bankid-label
  spelling tonight (aeon untouched); moving them to
  `ensure(bankid("…") == extern("SND_ENGINE_TABLE_BANK"))` is a follow-up ride-along for
  the next tranche touching those files. `extern` is THE spelling going forward; the
  spec-integration pass records it.

## Item C — the "internal: … anchor label" diagnostic: RULING = distinguish by re-walk

`check_link_asserts`'s `Fold::Poison` arm (`sigil-link/src/lib.rs:241-250`) claims
"compiler bug in the `here()`-relaxation fix" for ANY unresolvable assert condition — but a
cross-seam ensure compiled standalone (both operands external) reaches it legitimately
(mt_bank T2, sfx_bank T3, every future port's standalone check; and now every `extern`
typo).

**The cases ARE distinguishable** (recon-verified): `here()` anchor labels follow the
naming convention `__here$<module>$<n>` (`lower/mod.rs:567`), and `Expr` is a plain
walkable tree. Fix design:

- Add a small helper walking an assert condition's `Expr::Sym` leaves against the symbol
  table, collecting unresolved names.
- If ALL unresolved leaves are non-`__here$`-prefixed: user-facing message — name the
  symbols, state the cause ("not defined in this link — expected when compiling a
  cross-seam module standalone; supply the map/harness composition"), no bug claim.
- If any unresolved leaf IS a `__here$…` anchor: keep the internal-bug wording (that
  scenario is structurally unreachable today — `lower_item_guard` defines the anchor in
  the same pass that pushes the assert — so reaching it genuinely is a compiler bug).
- The existing test `link_assert_unresolved_cond_is_internal_contract_error`
  (`lib.rs:665-679`) pins the OLD message on what is actually a case-(a) shape
  (`Expr::Sym("Nope")`) — it gets intentionally rewritten to assert the new user-facing
  message. A new companion test pins the internal wording via a genuine `__here$…` leaf.
- New standalone-compile test at the sfx_bank.emp shape (cross-seam ensure, empty symbol
  table) pinning the new message end-to-end. The four negative probes (resolvable-but-
  false, `Fold::Value(0)` path) are untouched and must stay green:
  `link_assert_fails_when_cond_zero_and_renders_message`, `ports.rs::probe_b`,
  `mt_negative_probes.rs`, `sfx_negative_probes.rs`.

## Item A — ledger dispositions (full-arc re-evaluation, S2-D14(a)(d)(e) + 9d)

Evaluated against the COMPLETE sound arc (DAC T0/T1 + MT T2 + SFX T3, all merged) plus this
session's Item-B outcome. Spec working-tree edits made (uncommitted, stacked on the pending
#7/D2.25 + D2.26 passes): new changelog row **D2.27**, §4.5 pointer-array int elements,
§7.5 bidirectional-seam + `extern()` paragraph, §8.1 defines/imm32/partial_fold/standalone-
diagnostic contract text, §10 inventory entries, ledger-row annotations.

- **S2-D14(a) (packing linker): RE-AFFIRMED, unchanged gate.** Three tranches, two
  co-resident banks, zero packing demand — co-residency dictated the layout, so auto-packing
  had nothing to decide, and hand-pinned regions were REQUIRED for byte-identity anyway.
  Gate stays "3+ floating bank sections, or a real fit failure in a real port."
- **S2-D14(d) (Z80 bank-consumption idioms): resolution HELD through the hardest case.**
  T3's win-tab `dw sfx_winptr(Sfx_NN)` compound trees demanded `partial_fold` (deferred
  fixup targets bake AS-env-resolvable subterms) — an implementation refinement INSIDE the
  (d) contract ("driver `db`/`dw` defer cross-seam as value-fixup trees"), not a revision of
  it. Recorded as observable seam behavior in §8.1.
- **S2-D14(e) (bank:+vma: reject): RE-AFFIRMED.** No tranche produced a need; the
  wrong-latch trap stays unconstructible; still relaxable on demonstrated need.
- **9d (byte-command DSL): re-gate RE-AFFIRMED at arc end.** 34 stream embeds shipped
  (DAC 10, MT 6, SFX 18), zero hand-authoring demand; the one authored table (SfxTable)
  went hand-owned as a plain typed pointer array, not a DSL. Gate stays
  hand-authoring/source-diffing demand.
- **The ensure-spelling gap (T2/T3 carry-forward): CLOSED** by Item B's `extern()` + AS equ
  export (recorded under the S2-D14 row's umbrella + D2.27).
- **imm32 deferral's deliberately-narrow scope: RE-AFFIRMED as design, not debt.** An
  unresolved short immediate has no honest deferral story (imm8 can't carry an address;
  silent truncation is the enemy), and the one candidate consumer — Z80 imm8 bank
  constants — was eliminated by T3 R2's co-residency identity. Re-open only on a real
  cross-seam consumer that cannot be restructured.
- **Comptime defines (T2 R1): spec-integrated** (§8.1 + D2.27) including the
  global-reserved-names rule (`[defines.collision]` against ALL item kinds incl.
  procs/scripts) and the loud-not-silent unseeded-entry-point policy (seed-on-demand).
