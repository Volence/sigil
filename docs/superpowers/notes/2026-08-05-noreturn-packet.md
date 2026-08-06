# noreturn-tail model — packet (2026-08-05)

Executes the ruled spec `specs/2026-08-05-noreturn-tail-design.md` (amended on
master `ad670db4` after the first panel round: the trailing-local fall-off and
the `falls_into` composition rule). Branch `noreturn` in both worktrees, cut
from sigil `326809e5` / aeon `b9b1056`. No merge-state claims — the overseer
owns the queue.

## Numbers first

- **Byte bar: ALL SEVEN targets byte-identical.** Full-file CRC/size, fresh
  `capture_goldens.sh` order, matching the frozen goldens exactly: s4
  `c2d17ee3/411096` · s4.debug `6c296656/423480` · demo `4a09314e/91258` ·
  demo.debug `f3e5ed3e/93955` · config_a `4e34a38a/423871` · config_b
  `b8cce891/301132` · lean `b92cb485/379110`. Canonical s4/s4.debug rebuilt +
  re-cmp'd OK after the config clobbers. The adoption emits no bytes; every
  analysis change is byte-neutral.
- **Strict** (`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon b8> cargo test --workspace
  --release --no-fail-fast`, foreground, failures-first, re-run after the fix-up):
  **3386 passed / 0 failed / 4 ignored** across 310 suites. Base `#[test]` at the
  branch point = **3369**; the tree now holds **3390** (`git grep -c '#\[test\]'`),
  the +21 are this lane's new tests (15 model + 6 fix-up), all passing.
  `3386 + 4 = 3390` — nothing skipped. `refreeze --check: OK (chain len 47)`.
- **Warn tiers: UNCHANGED in every shape**, measured before (branch point) and
  after the fix-up: plain 19 · debug 18 · demo 19 · demodbg 18 · config_a 18 ·
  config_b 19 · lean 18, identical id breakdowns (module.path-mismatch,
  undeclared-fallthrough, out-unwritten, clobber-undeclared). **`[proc.ccr-
  advisory]` and `[noreturn.returns]` fire ZERO times in every shape** — the S1
  (falls_into) and M1 (trailing-local) refusals tripped NO corpus proc; only
  test-fixture behavior changed, per the panel's expectation.
- **repin: pins.rs / engine.inc / mixed_dac_rom.rs / repin_pins.rs / repin.toml
  UNTOUCHED** (git status clean of all five). No chain bump.
- **clippy** `--workspace --release --all-targets`: this lane's changed files are
  clean. Three PRE-EXISTING warnings surface in files this branch never touches
  (`mul_lower.rs:357` redundant-closure, `warn_tier_corpus.rs:168-169` doc-list);
  `git diff` against the branch point shows both files untouched, so they are not
  this lane's — left for the owning parcel.

## Commits (for the queue)

- sigil `0dce3719` — noreturn-tail model (code): `@noreturn` attr + check + set,
  cycle_budget consumer, AssertDesugar rail stamp, CCR advisory.
- sigil `ac6ef2bc` — ledger (5 rows) + packet (model round).
- sigil (fix-up, code) — M1 trailing-local + S2 falls_into composition + S1 tail
  refusal + S3 unified `transfer_target_sym`, +6 pins.
- sigil (fix-up, docs) — ledger (census numbers + 4 fix-up rows) + packet update.
- aeon `ce0eaee` — `@noreturn` on the 12 stubs + ErrorHandlerBlob + GameLoop +
  EntryPoint.
- aeon (fix-up) — `@noreturn` on `ReleaseFault`, `SndDrv_Init`/`_Idle`/`_Sample`,
  `Z80_Sound_Entry`, `Z80_IdleProgram`. 21 decls total.

## Land order — MEASURED, sigil-first

The aeon adoption does not parse under old sigil: building aeon b8 (with the 21
`@noreturn` decls) through a sigil binary at the branch point `326809e5` fails
with the verbatim error **`[attr.unknown] `@noreturn` is not a known attribute
(expected one of: as_compat, allow, scaffolding, budget, cycles_exact)`** (one
per adopter). So **sigil must merge before aeon**. Once sigil is in, aeon builds
byte-identically (the seven CRCs above were captured with the b8 sigil over b8
aeon). There is no reverse coupling: old aeon builds under new sigil unchanged
(the byte bar's baseline was new-sigil/old-aeon).

## The three mechanisms

### 1 · `@noreturn` — a checked claim (spec design-point 1)

Rides the existing attrs channel: `@noreturn` is added to the parser's KNOWN set
and takes no arguments (`[attr.form]` on a stray arg). It attaches to `proc`
decls AND `extern proc` sigs — `ExternProcDecl` gained an `attrs` field (parallel
to `ProcDecl.attrs`), and `attach_item_attrs` now accepts `Item::ExternProc`.
`ProcDecl::is_noreturn()` / `ExternProcDecl::is_noreturn()` read it.

`check_noreturn` (lower/proc.rs) walks the body's `Cfg` edges (68k or Z80) and
fires `[noreturn.returns]` (ERROR, unsuppressible) on any `Edge::Return` OR
`Edge::FallOff`. FallOff is included deliberately: a body that runs off its end
"returns" into a successor, which is not leaving by a transfer or a loop, so the
checked claim would otherwise have a hole. Conditional returns are caught because
the returning instruction carries the `Return` edge whatever branch reaches it.
Negative probes both polarities: `noreturn_terminal_transfer_is_clean` (jbra
loop, clean), `noreturn_with_rts_errors`, `noreturn_with_conditional_rts_errors`,
`noreturn_fall_off_end_errors`, plus the form probe and the extern-proc parse.

### 2 · Authored divergence (spec design-point 2)

**Scope note the spec's ground-truth statement (§1(b)) required, and the residue
lane (b9) independently measured:** before this lane, ONLY the `assert` desugar
stamped `ItemAuthor::AssertDesugar`; `raise_error` and `raise_exception` stamped
ambient `User` on their rail items, so nothing marked the 5 raise_error + 14
raise_exception corpus rails as divergent. This lane fixes that at the stamping
site (`lower_raise_error`, eval/asm.rs), the same save/replace `lower_assert`
uses. All three rails now carry AssertDesugar end to end, terminal `jmp (pages)`
included — pinned per rail form (`the_raise_error_rail_is_desugar_authored…`,
`the_raise_exception_rail_is_desugar_authored…`, diag_desugar.rs). A hand-written
`jmp` to the same blob stays a plain `Defer` — authorship is the only
distinguisher, never the target symbol.

**Re-verified the widened AssertDesugar set against its existing consumer**
(`[proc.sr-undeclared]` exemption, lower/proc.rs:948-970): the raise tail's only
SR op is `move.w sr, -(sp)` — an SR **read** (source `sr`, dest `-(sp)`), whose
`ops.last()` is `PreDec(A7)`, NOT `Sr`. The exemption fires only on an SR
**destination** write (`ops.last() == Sr`), which the raise tail never has. So
exempting the rails changes nothing — confirmed by the measured zero warn-tier
delta in the DEBUG shapes (debug/config_a/demodbg all 18, no `proc.sr-undeclared`
in any). The `sr_writes` authorship census (corpus_contracts) likewise gains
nothing (no SR-dest write to record); the strict corpus gate is unchanged.

### 3 · Consumers

**cycle_budget** (spec §2.1): `charged_edges` gained a `divergent_terminal`
predicate — an AssertDesugar-authored unconditional transfer, OR a transfer whose
named target is `@noreturn` — and its `Defer` edge closes the path like a return
(charged its own cost, no successor). `[cycles.unbounded-transfer]`'s message now
points at `@noreturn`. **No corpus effect: the aeon corpus has ZERO `@budget` /
`@cycles_exact` procs** (grep-confirmed), so cycle_budget is entirely unit-
driven. Pins (spec §5 bar): `an_authored_rail_terminal_closes_the_budget_where_a
_hand_jmp_refuses` (the refuses-before / verifies-after), `a_tail_to_a_noreturn
_target_closes_the_path`, `a_conditional_branch_to_a_noreturn_target_closes
_only_the_taken_edge`. The b7 enum-dispatch lane also edits `charged_edges`
(a `targets()`-resolution arm); the divergent-terminal arm is a self-contained
`Edge::Defer if divergent_terminal` guard composed BEFORE the existing
computed/unbounded `Defer` arms and a matching close in the charging loop — it
reads only `author` / `ops` / the `@noreturn` set, so it composes with a
targets()-resolution change without ordering coupling.

**RESIDUE (reported, not papered over):** a REAL budgeted proc with a full assert
rail is still not measurable — the rail's inline message (`CodeItem::Inline`,
which carries NO author) trips `path_costs`'s blanket InlineData refusal before
the walk, and the rail's `jsr (handler)` trips OpaqueCall. Full-rail
measurability needs an author on `CodeItem::Inline` (or reachability-aware
refusals). Zero corpus consumers, so out of proportion this lane; the terminal-
transfer mechanism + unit pins are the §5 deliverable, and the b9 invoke-edge fix
depends on the same "a call inside a compiler-authored divergent rail owes the
return path nothing" principle (see §Dependencies).

**CCR advisory** — see below.

## CCR advisory (`[proc.ccr-advisory]`, spec §2.2, the bare-sr four-item list)

Bare `preserves(sr)` claims both halves; `check_preserves_sr` proves only the
mask round-trip. The advisory runs the SAME `ccr_bracket_refusal` walk the
explicit-`sr.ccr` ERROR uses, at WARN tier, over bare-`sr` procs — **gated on the
mask proof PASSING** (snapshot `diags.len()` around `check_preserves_sr`), so it
is CCR-SPECIFIC: a proc that already failed the round-trip is not double-nagged.
That gate IS the fixture-audit answer to the lower_proc unbalanced-SR fixtures
(they fail the mask proof, so the advisory never reaches them — no double-fire).

All four enabling items:
- **(i) divergence awareness** — the walk SKIPS `AssertDesugar` items wholesale
  (the rail's frame-sim `move.w sr, -(sp)` no longer reads as a nested save, its
  `jmp (pages)` no longer as a `Leaves`); a User transfer to a `@noreturn` target
  is divergent, not a leave.
- **(ii) local-label awareness** — a `jbra .local` is intra-proc flow, decided by
  the shared `Cfg` (`is_local_label`), not a second matcher; only a real EXTERNAL
  non-`@noreturn` tail is a `Leaves`.
- **(iii) new warn id + DEBUG baselines** — MEASURED: **ZERO firings in every
  shape**, plain and DEBUG. The 3 corpus bare-`sr` adopters (Sound_PostByte,
  Sound_Init, BG_Init) all return `None`: BG_Init's `movem`/`movea` outside the
  pair are cc-inert; Sound_Init's DEBUG `raise_error` inside its bracket is now
  skipped (the exact FP the sr-split lane could not clear). The advisory now
  machine-checks the CCR half of bare-`sr` at warn tier — and zero firings is the
  proof the bare-sr flip was complete.
- **(iv) fixture audit + detectors** — 4 new detector tests in lower_proc.rs
  (`ccr_advisory_names_post_restore_flag_traffic`, `…_silent_when_whole_body
  _bracketed`, `…_silent_on_local_jump`, `…_silent_on_debug_rail_inside_bracket`).
  **Finding, reported per spec:** diag_desugar's hand-written `golden`
  transliteration procs declare bare `preserves(sr)` and their frame-sim
  `move.w sr, -(sp)` — being `User`-authored there — reads as a nested bracket
  save, so the advisory (correctly) fired on them. They are flipped to
  `preserves(sr.mask)` (the half a linear walk can prove), while their
  `construct` twins KEEP bare `sr` (the desugar's traffic is emission-site-
  proven). The twins stay byte-identical (contracts emit nothing) — this
  asymmetry IS the model, documented in a comment on the fixture.

## Panel fix-up round (spec amended `ad670db4`)

The three-lens panel adjudicated; all items TAKEN in one consolidated round.

- **M1 (Lens B, real 68k soundness hole)** — an unconditional transfer to a
  TRAILING local label (a `.end:` that closes the body) classifies as
  `Edge::Defer` on the 68k `Cfg::edges`, and both new checks accepted the lie: a
  `@noreturn` proc falling off via `bra .out` passed `[noreturn.returns]`, and a
  `preserves(sr.ccr)`/bare-`sr` proc whose `jbra .end` reads as intra-proc flow
  was ACCEPTED where it was refused. **Ruled NARROW fix, taken:** `check_noreturn`
  treats a `Defer` to a trailing local label (`is_local_label(t) &&
  label_index(t).is_none()`) as a `FallOff`; the CCR walk gates intra-proc on
  `cfg.label_index(t).is_some()` instead of `is_local_label`. `Cfg::edges` is NOT
  touched — the 68k builder unification is a ledgered own-parcel (cross-analysis
  blast radius). Both counterexamples pinned verbatim
  (`noreturn_trailing_local_transfer_is_a_fall_off`,
  `ccr_trailing_local_transfer_is_a_leave`).
- **S1** — `check_ccr_advisory` now reuses the ERROR check's `sr_tail_refusal`
  (factored): a bare-`sr` proc with a declared `falls_into` or a non-terminator
  ending refuses BEFORE walking, so it is not silently green
  (`ccr_advisory_fires_on_a_falls_into_bare_sr_proc`).
- **S2 (spec-amended composition)** — `[noreturn.returns]` accepts a `FallOff`
  IFF the proc's `falls_into` names a symbol that is itself `@noreturn`; refused
  otherwise. Both polarities pinned
  (`noreturn_falls_into_noreturn_successor_composes`,
  `noreturn_falls_into_returning_successor_is_refused`). SndDrv_Init is the first
  live corpus adopter (empty init body + `falls_into SndDrv_Idle`, itself
  `@noreturn`).
- **S3** — the `@noreturn` / computed-target extraction is unified in one
  `flag_check::transfer_target_sym` (`Sym | SymOff | AbsSym`), replacing the
  `Sym`-only `branch_target` at both consumers, so `jmp (Diverge).l` (an
  `AbsSym`) matches a `@noreturn` symbol
  (`a_tail_via_abs_long_to_a_noreturn_target_closes_the_path`). The cycle_budget
  authored-terminal arm is conjoined with `names_a_target` so it is
  order-independent with the coming b7 `targets()` arm by construction.

**Adoptions added (Lens C census, byte-neutral, all pass `[noreturn.returns]`
as-written):** `ReleaseFault` (release_fault.emp — the missed 68k sibling, ends
`.halt: bra .halt`); Z80 `SndDrv_Idle`/`SndDrv_Sample`/`Z80_Sound_Entry`
(z80_sound_driver.emp) + `Z80_IdleProgram` (z80_init.emp); and `SndDrv_Init` (the
S2 live adopter). 21 `@noreturn` decls total.

**INFO edges recorded to the ledger, not code:** a data-only `@noreturn` body
(`ErrorHandlerBlob`) passes vacuously — an accepted-lie edge for a literally-
empty proc, load-bearing and by design; `jsr`/`bsr` to a `@noreturn` target
stays `[cycles.opaque-call]` (conservative, no consumer); the `@noreturn` set is
per-module (a cross-module divergent tail needs `extern proc @noreturn` re-decl).

**seam1** — the synthesized `extern proc` stubs (`seam1.rs`) now carry `@noreturn`
across the seam (filtering it OUT of `@budget`/`@scaffolding`, which are
body-only), so a stub states the same divergence the body proves.

**Fixture dispositions** — `diag_desugar` `golden`↔`construct` (§iv above);
`diag_assert_vector.rs:147` declares bare `preserves(sr)` on a hand-written raise
transliteration and its test asserts only ERROR-freedom, so the warn-tier
advisory (if it fires on the `User`-authored frame-sim) is orthogonal — no change
needed, disposition recorded.

## Ledger (spec §5)

- `[t25 error_handler panel A1]` (the noreturn ask) → **CLOSED**.
- `[sr-split, 2026-08-05]` (the advisory enablement) → **advisory ENABLED**, all
  four items, zero firings measured.
- `[bprime-0b lens C, 2026-08-04]` (@noreturn) → **@noreturn shipped, row stays
  OPEN**: noreturn was NOT the sole missing piece — the out-cond transitive
  CHARGE is an out_verify SURVIVES-verifier change (call-target code, scoped out
  §4/§6); now unblocked.
- `[bprime-2, 2026-08-04]` (stack delta) → **census recorded (EMPTY set), charge
  OFF**: the push-then-EXTERNAL-tail population is ZERO. Every corpus
  push-then-transfer is push + `jbra .local` (intra-proc, `Edge::Follow`, never a
  `Defer` exit): `section.emp:533/584/628/667`, `sprites.emp:249/:254`; `pea` in
  code = 0. No legitimate arg-passing tail to protect and no site the charge would
  fire on; left OFF because turning it on is a byte-frozen preserves.rs Defer-arm
  change. Unblocked on the divergence distinction; a future push-then-external-
  tail is adjudicated when it appears.
- `[bare-sr-flip lens C, 2026-08-05]` (preserves-through-tail) → **Z80 precedent
  cited** (spec §6): `z80_preserves`'s `Edge::Defer` arm already credits a tail
  transfer to a preserving callee via its `CalleePreserves` oracle; the 68k
  `preserves.rs` Defer arm has no such consult. Nothing more taken.

## Dependencies (for the follow-up micro-fixes)

The b9 residue lane's invoke-edge fix is BLOCKED on this model: the desugars'
abs-long `jsr (MDDBG__ErrorHandler).l` register as closure holes the moment
call-target recognition sees `AbsSym`, and the principled exclusion is "a call
inside a compiler-authored divergent rail owes the return path nothing." This
lane ships the authorship that makes that exclusion expressible (all three rails
now carry AssertDesugar) but does NOT change corpus_contracts call-target code
(briefed out). The follow-up can cite `0dce3719`.

## Step-3 (port retrospect) vs step-5 (engine optimize)

**Step-3 — what the language should make impossible.** The whole lane is one
step-3 finding cashed: a divergent tail was indistinguishable from a real tail
call because the language had no word for it. Now it does (`@noreturn` +
authored rails), and three analyses that were forced to guess (cycle_budget's
unbounded-transfer, the CCR walk's `Leaves`, and — pending — B′-0b's survives
charge and B′-2's stack charge) can each read the fact instead. The sharper
sub-finding: **`raise_error`/`raise_exception` were authored `User`** — the spec
and the residue lane both asserted all three rails carried AssertDesugar, and
only `assert` did. A construct that emits a divergent rail but stamps it as the
user's is a standing invitation for every author-reading analysis to mis-file it;
the fix is one stamping site, and now the invariant the codeitem-author §2 note
states ("authorship REDIRECTS the obligation") actually holds for all three.

**Step-5 — engine optimization.** None. The lane emits no bytes; the corpus
census is byte- and warn-identical. The adoption is metadata only.

## Neither bucket — the headline

**The advisory's value is a zero it PROVES, not a warning it emits.** Enabling
`[proc.ccr-advisory]` over bare-`sr` with divergence- and local-label-awareness
fires zero times across all seven shapes — and that zero is the deliverable: it
machine-confirms the sr-split flip left no dishonest bare-`sr` adopter, closes
the CCR-half gap row 2083 named, and does it without the DEBUG-rail false
positives that blocked the sr-split lane from turning it on. The `golden`-vs-
`construct` split the fixture audit forced is the same fact in miniature: two
byte-identical procs that HONESTLY declare different SR contracts, because one's
inner SR traffic is the compiler's (proven) and the other's is the author's
(unprovable by a linear walk) — which is exactly the distinction `@noreturn` and
the authored rails exist to draw.

## Kill-list

No twin, no scaffolding created; no row added or closed.
