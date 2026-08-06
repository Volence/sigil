# noreturn-tail model — packet (2026-08-05)

Executes the ruled spec `specs/2026-08-05-noreturn-tail-design.md`. Branch
`noreturn` in both worktrees, cut from sigil `326809e5` / aeon `b9b1056`. NOT
merged, NOT pushed — the overseer merges after a lens panel.

## Numbers first

- **Byte bar: ALL SEVEN targets byte-identical.** Full-file CRC/size, fresh
  `capture_goldens.sh` order, matching the frozen goldens exactly: s4
  `c2d17ee3/411096` · s4.debug `6c296656/423480` · demo `4a09314e/91258` ·
  demo.debug `f3e5ed3e/93955` · config_a `4e34a38a/423871` · config_b
  `b8cce891/301132` · lean `b92cb485/379110`. Canonical s4/s4.debug rebuilt +
  re-cmp'd OK after the config clobbers. The adoption emits no bytes; every
  analysis change is byte-neutral.
- **Strict** (`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon b8> cargo test --workspace
  --release --no-fail-fast`, foreground, failures-first): **3380 passed / 0
  failed / 4 ignored** across 310 suites. Base `#[test]` at the branch point =
  **3369**; the tree now holds **3384** (`git grep -c '#\[test\]'`), the +15 are
  this lane's new tests, all passing. `3380 + 4 = 3384` — nothing skipped. The
  four ignored are the standing set.
- **Warn tiers: UNCHANGED in every shape**, measured before (branch point) and
  after: plain 19 · debug 18 · demo 19 · demodbg 18 · config_a 18 · config_b 19
  · lean 18, identical id breakdowns (module.path-mismatch, undeclared-
  fallthrough, out-unwritten, clobber-undeclared). **`[proc.ccr-advisory]` fires
  ZERO times in every shape** — see §CCR below.
- **repin: pins.rs / engine.inc / mixed_dac_rom.rs / repin_pins.rs / repin.toml
  UNTOUCHED** (git status clean of all five). No chain bump.
- **clippy** `--workspace --release --all-targets`: clean on the Rust side (exit
  0 under `-D warnings`; the only cc-crate warnings are the pre-existing vendored
  `sigil-clownlzss-sys` enigma.h C++ warnings, not this lane's).

## Commits (for the queue)

- sigil `0dce3719` — noreturn-tail model (code): `@noreturn` attr + check + set,
  cycle_budget consumer, AssertDesugar rail stamp, CCR advisory. 9 files.
- sigil (branch tip, this commit) — ledger (5 rows) + this packet.
- aeon `ce0eaee` — `@noreturn` on the 12 stubs + ErrorHandlerBlob + GameLoop +
  EntryPoint. 3 files.

## Land order — MEASURED, sigil-first

The aeon adoption does not parse under old sigil: building aeon b8 (with the 15
`@noreturn` decls) through a sigil binary at the branch point `326809e5` fails
with **`[attr.unknown] @noreturn is not a known attribute` × 15** (one per
adopter). So **sigil must merge before aeon**. Once sigil is in, aeon builds
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

## Ledger (spec §5)

- `[t25 error_handler panel A1]` (the noreturn ask) → **CLOSED**.
- `[sr-split, 2026-08-05]` (the advisory enablement) → **advisory ENABLED**, all
  four items, zero firings measured.
- `[bprime-0b lens C, 2026-08-04]` (@noreturn) → **@noreturn shipped, row stays
  OPEN**: noreturn was NOT the sole missing piece — the out-cond transitive
  CHARGE is an out_verify SURVIVES-verifier change (call-target code, scoped out
  §4/§6); now unblocked.
- `[bprime-2, 2026-08-04]` (stack delta) → **census recorded, charge OFF**: the
  corpus push-then-tail population is dominated by push + `jbra .local` (intra-
  proc, `Edge::Follow`, never a `Defer` exit); the genuine push-then-external-
  tail set cannot be soundly enumerated without running the charge, and
  preserves.rs's Defer arm is byte-frozen this lane. Unblocked on the divergence
  distinction; enabling is a dedicated preserves.rs parcel.
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
