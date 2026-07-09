# Overnight handoff 1 of 2 — the seam re-evaluation session (3 items) + spec-integration pass

Written 2026-07-09 (Fable, post-T3-checkpoint) for the OVERNIGHT session. Volence is asleep;
he ratified this work tonight and wants it wrapped so the **68k engine-port campaign starts
tomorrow when he wakes**. Read `notes/2026-07-09-overnight-handoff-2-compression.md` next —
that's the second arc tonight; this one goes first (its outcomes feed the spec freeze).

## Session ground rules (Volence's explicit process directive, 2026-07-09)

- **Fable orchestrates: makes the design decisions and verifies work was done properly.**
- **Implementer/reviewer subagents use LESSER models to save tokens** — sonnet by default,
  haiku for purely mechanical steps. Reserve Fable-level reasoning for decisions, spec
  writing, and load-bearing verification (spot-check the critical claims yourself — with
  sonnet implementers the two-stage review discipline matters MORE, not less).
- Volence is unavailable: make decisions autonomously ([[user-defers-sigil-technical-calls]]),
  record every ruling with rationale. **NO functional merges to master without his morning
  checkpoint** — complete work on worktree branches, leave a checkpoint packet (the Plan-2
  overnight precedent). Docs/notes commits go straight to sigil master per house cadence.
  empyrean spec edits stay in its WORKING TREE, uncommitted (his cadence; it already holds
  the uncommitted #7/D2.25 + D2.26 passes — stack on them, do not commit).

## Where everything stands (post-checkpoint, all pushed)

- **T3 MERGED**: sigil master `365316d`, aeon master `a103e46`. The sound-data migration
  arc (DSM.8 T0–T3) is **DONE**. Post-merge validation: 1491/0 strict-gate on the merged
  masters. The forest-bg item is closed (aeon `b0e5a66`).
- Checkpoint packet + every T3 deviation: `notes/2026-07-09-sound-migration-t3-complete.md`
  + the plan's Execution notes (`plans/2026-07-09-sound-migration-t3-sfx.md`).
- Memory [[spec2-progress]] is current through the merge.
- **F1 flake watch item**: one non-reproducible full-workspace failure of the T1 mixed test
  during T3 review (judged environmental; 5 clean sequential runs after). If a workspace run
  fails ONCE tonight: re-run, and if clean, record the occurrence in the completion note
  (dump the resolved section LMAs if it recurs twice).

## The session: three agenda items (Volence ratified the bundle — "we're kind of debugging there")

### Item A — S2-D14(a)(d)(e) + 9d re-evaluation (spec-ledger work, Fable-role)

Re-open `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`'s deferred ledger and re-evaluate the named
dispositions against the FULL sound arc's evidence. The arc's data points: comptime defines
(T2 R1), imm32 deferral + its deliberately-narrow scope (T2 carry-forward #4),
`partial_fold` (T3 Task 5 — deferred fixup targets bake AS-env subterms; now observable AS
seam behavior), the bankid-label idiom ×3, int elements in pointer arrays (T3 P3 + the
range-check rule). Output: updated dispositions in the spec's working tree + a short
decision record in sigil `docs/superpowers/specs/` (committed).

### Item B — bare cross-seam equ reads: the language decision (Fable DECIDES tonight)

The gap: `.emp` exprs can't read an `.asm`-side equ by bare name (`ensure(x == SOME_ASM_EQU)`
→ comptime "unknown name"); only builtin call ARGS get the link-symbol fallback
(`label_ctx`, eval/call.rs — deliberate, T2 Deviation 2 traced it). Three tranches paid the
bankid-label workaround.

**Fact to establish FIRST (it constrains the whole decision):** do AS-side equs reach the
LINKER's symbol table in the mixed build at all, or only labels? (T2's probe showed the
COMPTIME failure; whether a deferred link-expr could see the equ is a different question.)
Read how `sigil-frontend-as` exports symbols → `sigil-link`'s `SymbolTable` (grep equ
handling in the AS frontend's module output + how `resolve_layout`/`link` seed the table;
the T0 "equ export" work and `ports.rs` probes are the map). If equs DON'T reach the
linker, options (i)/(ii) below need that plumbing too — scope accordingly.

**The options** (from the T3 completion note):
- (i) extend the bareword→link-symbol fallback to `ensure` condition operands (scoped: only
  where the expr already defers to link; a typo'd name stays loud at link as "never
  defined").
- (ii) an explicit `extern("NAME")` builtin — raw link-symbol value passthrough, no Genesis
  mask/shift (unlike bankid/winptr). Explicit-at-usage-site; tiny surface.
- (iii) ratify the bankid-label idiom as THE spelling and spec it (zero code; the idiom
  only works when a bank-aligned LABEL proxy exists — it's a lucky-structure workaround,
  which is why it keeps costing).

**Fable's standing recommendation going in: (ii)**, per the house explicit-beats-spooky
principle (DSM.7's own words), UNLESS the fact-check shows equs never reach the linker and
the plumbing is large — then (iii) with a spec'd idiom + a better diagnostic. Whoever runs
tonight: verify the fact, decide, record the ruling + rationale in the decision record,
implement behind TDD if (i)/(ii). Sonnet implementer; Fable reviews the diagnostic wording
and the typo-failure-mode tests personally (that's the risk surface).

### Item C — the "internal: … anchor label" diagnostic fix

`check_link_asserts` (crates/sigil-link/src/lib.rs ~:241) reports a `Fold::Poison` assert
condition as "…this is a compiler bug in the `here()`-relaxation fix, not a source error" —
but a cross-seam ensure compiled STANDALONE (no map, both operands external) is a LEGITIMATE
way to reach it (hit by mt_bank.emp T2, sfx_bank.emp T3, and every future port's standalone
check). Fix: distinguish the cases — if the unresolvable leaves are symbols simply absent
from the table, say THAT ("condition references symbol(s) `X`, `Y` not defined in this
link — expected when compiling a cross-seam module standalone; supply the map/harness
composition") and keep the internal-bug wording only for a genuinely-anchorless `here()`
shape (if distinguishable; if not, drop the bug claim entirely — never accuse the user's
source of being a compiler bug). TDD: pin the new message in a standalone-compile test
(sfx_bank.emp's exact shape) + keep the negative probes' message assertions green (they
assert on the ENSURE's own message for the resolvable-but-false case — different path,
should be untouched; verify). Item B's outcome may change what "expected standalone
failure" looks like — implement C AFTER B's decision.

### Deliverable order

1. Item B fact-check → **B ruling** (recorded) → 2. B implementation (if any) + C
   implementation, one worktree branch (`seam-reeval` off sigil master), TDD, sonnet
   implementers, two-stage reviews (sonnet reviewer + Fable spot-check on B's typo-mode and
   C's message tests). 3. Item A ledger pass + **the accumulated empyrean spec-integration
   pass** (T2 debt: defines incl. global-reserved-names, imm32 deferral, the ensure-spelling
   gap; T3 additions: partial_fold semantics, ptr-array int elements + range rule; B's
   ruling) — empyrean working tree, UNCOMMITTED. 4. Full nets (workspace + strict-gate) +
   completion note + memory update. Branch left checkpoint-ready, UNMERGED.

Then proceed to handoff 2 (compression builtins). After BOTH arcs: write the
**68k-engine-port campaign kickoff handoff** for tomorrow (the roadmap's next step is spec
FREEZE → campaign; the kickoff note should propose the freeze checklist + the first
migration targets so Volence can checkpoint-merge everything and start the port on waking).
