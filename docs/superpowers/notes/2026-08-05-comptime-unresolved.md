# 2026-08-05 — the `[comptime.unresolved]` surface

Branch: sigil `comptime-unresolved`, built from master `de9d4ca2` and REBASED
onto master `41defe98` (chain 46) at merge prep — commits `148da3d0` (the
surface) + `152a354a` (last caller) + the lens-round commit carrying this
packet on top (its id is the branch tip; the pre-rebase ids
`82704ce0`/`8dc3a360`/`74e8f639` appear in §8's review notes).
**Sigil-only** — the aeon worktree served as the `AEON_DIR` corpus reference
and carries zero edits; at merge prep it was refreshed to aeon master `77f80c6`
(zero own commits, handoff-3 §9 step).

Work order: campaign-gap-ledger row ~2123 (`[bprime-4-gates lens B]`), the
define-free gate flip's residual hole.

---

## §0 — Headline

**Zero dropped instructions never proved a profile's defines reached the
analysis — now something does, and its first real outing caught shipping code
that was invisible to every analysis in every shape.**

A statement-`if` whose condition evaluates to `Value::Poison` discards BOTH arms
and contributes nothing to `dropped_instrs`. `ContractReport::comptime_unresolved`
now collects every name a comptime `if` condition referenced that the define set
failed to resolve — collected AT the condition's own evaluation, exposed on the
report, rendered by `--report contracts`, and **pinned EMPTY per shipped shape**
as an ERROR-tier corpus gate. "Add a new toggle" and "lose a toggle" both fail
loudly now, as a class.

The unplanned catch: `Camera_Update`'s landing-lock arm (`if
Game.CAMERA_JUMP_LOCK { }`, engine/level/camera.emp:328) — code that ASSEMBLES in
every sonic4-family ROM — never entered any corpus walk, because the gate is an
L1 interface member and every walk ran with `InterfaceEnv::empty()`. Fixed the
class way: the walk now binds each shape's own game's `implement`. §3.

Byte-neutral ×7 (all nine comparisons incl. canonical restore) ·
`refreeze --check` OK (tip `b-jumps`, chain len 44) · strict counts in §7 ·
aeon worktree clean.

---

## §1 — What the surface catches that the drop count cannot

The two are complementary classes, and the pair of assertions in the report test
now says so:

- a missing **VALUE** define drops the instruction that needed it
  (`MAX_RING_BUFFER` absent → `DrawRings` can't lower `vram_art(...)` → 1 drop) —
  the drop gate's class;
- a missing/misspelled **TOGGLE** define poisons the `if` condition and discards
  both arms whole — **zero drops, zero firings**, the guarded code absent from
  every downstream analysis (write set, closure, D1b/D1c, dead-save, §6). Only
  the unresolved surface sees it.

The ledger row's repro is executable, twice:

- `a_poisoned_statement_if_is_invisible_at_zero_drops_and_the_surface_names_it`
  (frontend, synthetic — the row's own `proc Foo` verbatim): no defines → zero
  drops, zero firings, surface = exactly `[("Foo", "NOPE_UNDEFINED")]`; then the
  SAME source with `-D NOPE_UNDEFINED=0` → surface empties and the hidden
  `moveq #1, d2` becomes a real closure firing. Both halves of the blind spot,
  end to end.
- `a_misspelled_toggle_in_a_condition_is_named_at_zero_drops_in_every_shape`
  (corpus-scale): doctors `Collected_ParkSlot`'s `if DEBUG == 1` dup-id scan to
  `if DEBUG_MISSPELLED == 1` and asserts, in ALL SEVEN shapes, that the surface
  names the proc+name while `dropped_instrs` stays 0. Because the misspelling
  resolves in no shape, this doubles as the surface's LIVENESS witness — no
  shape's silence can mean "measured nothing".
- `a_profile_that_loses_a_toggle_is_named_by_the_surface_in_every_shape`: strips
  `DEBUG` from each shape's own define set — the ledger row's exact
  "profile lost a define" failure mode — and demands the surface name it in all
  seven, still at zero drops.
- `a_poisoned_expression_if_inside_a_comptime_fn_lands_on_the_surface`: the
  expression-`if` sibling in the shape where it bites — a comptime Code-emitting
  fn (the emit-tool idiom) whose gate silently falls through to
  `Code.empty()`.

## §2 — The collection point, and why it cannot drift

Collection is AT the evaluation, not beside it:

- `eval_path`'s two `unknown name` → `Poison` fallthroughs (single-segment
  bareword, expr.rs; multi-segment path) push `(name, span)` onto an
  evaluator-local `unresolved_names` log — the exact program points where a
  lookup miss becomes `Poison`; the label-ctx fallback returns before either, so
  a deferred link symbol can never enter the log. Post-panel, the const/equ READ
  site also logs when the RESOLVED value is `Poison` (the memo side door — §8.3).
- Every condition site — statement `AsmStmt::If` (eval/asm.rs), expression
  `eval_if`, and both loop heads (`eval_while`'s condition, `eval_for`'s
  iterable; a `Poison` head skips the body with the same silence, post-panel —
  §8.2) — watermarks the log, evaluates the condition, and DRAINS everything
  past the watermark onto `comptime_unresolved`. The harvest reads the same
  `eval_expr` call whose value selects the branch, so there is no second walk to
  disagree with the first. Draining (not copying) means a nested `if` inside a
  condition consumes its own entries — no double-attribution.
- Harvest is unconditional on the condition's value: a short-circuit can rescue
  the truth value after a miss, and the miss is still a name the define set
  failed to resolve. (It cannot false-positive: entries only enter the log at a
  genuine resolution miss, and any such miss inside a condition is a hard error
  in a real build — §5.)

Threading: `eval_proc_body_env` returns the set (5th element);
`corpus_contracts` aggregates it from both body-eval passes (68k PASS 2 and the
Z80 flag pass — a Z80 module's comptime conditions read the same defines),
attributed `(proc, name, span)`, sorted, exact-duplicates deduped.

**Honest boundary** (pinned in the expression-if test's doc, ledgered): a
condition evaluated UNDER a label-value context (an instruction immediate, a
data initializer, a call argument) resolves an unknown bareword as a deferred
link `Label`, not a miss — outside this surface's reach. The build rejects that
shape as a one-file type error, and every corpus statement-`if` evaluates
outside label ctx.

## §3 — The unplanned catch: interface-gated arms, and the per-shape bind

Running the new report over the real corpus surfaced ONE row, identical in all
seven shapes: `(Camera_Update, Game.CAMERA_JUMP_LOCK)`. That is ledger row
~2124's "gated on corpus constants, not defines" class — except this arm is
REACHED: sonic4 binds the lock `true`, the landing-lock block ships in five of
the seven ROMs, and the analysis had never read it. Its register writes were
invisible to the closure, D1c, dead-save — everything — at zero drops, and the
define-gate flip could not have seen it (it is not gated on a define).

The class fix, sigil-only and byte-neutral:

- `analyze_corpus_with_contracts(files, defines, &InterfaceEnv)` — the walk the
  shape gates now run; `analyze_corpus_with` keeps the empty env (synthetic
  tests, the define-free `corpus_report()` family — row 103 owns that flip, and
  a shape-free walk has no game to bind by).
- `bind_corpus_interfaces(files, defines, game_module_prefix)` — binds over
  engine + the SHAPE'S OWN game's modules (`bind` demands exactly one
  `implement` per interface; the corpus holds one per game; the prefix is
  derived from the profile's own `game_ram_module`, never a hand-kept list).
  Implement values reference imported consts (`const ENTRY_ID =
  GS_OJZ_SCROLL_TEST`), so binding evaluates with the PASS-1 corpus environment
  as ambient — `bind_with_ambient` / `eval_expr_in_file_ambient`, both exact
  zero-delta wrappers of the production path (`with_file(f)` IS
  `with_file_and_ambient(f, &[])`).
- Both shape-walking gate files AND `--report contracts` bind per shape, so the
  gates and the report stay one walk. Bind errors are as fatal as parse errors
  in both — a half-bound env silently poisons every member reference it missed.

**Measured blast radius of the arm entering the analysis: zero.** Every firing
family, D1c (21 plain / 26 debug), dead-saves (3), and the context censuses
(23/20) are unchanged in all seven shapes — the arm's contracts were already
correct; what changed is that this is now checked rather than assumed.

The proof it is real:
`an_interface_gated_arm_is_walked_under_each_shapes_own_game` plants an
undeclared `d5` write INSIDE the arm and demands the partition — fires in
exactly the five sonic4-family shapes, silent in the two demo shapes (whose game
binds `false` and compiles the arm away). The fires-here half proves the arm
genuinely enters the analysis buffer; the silent-there half proves each shape
binds ITS OWN game rather than one env under seven labels.

## §4 — The per-shape pinned-empty gate, and its non-vacuity

`corpus_comptime_conditions_all_resolve_the_error_gate` asserts
`comptime_unresolved` EMPTY for each of the seven shapes, with the shape label
and the offending rows in the failure message. Its non-vacuity is carried by the
probes of §1 (the misspelled-toggle probe fires in all seven — the liveness
half) and §3 (the interface probe partitions on the game axis), plus the
three-toggle probes of §6. `the_contracts_report_is_wired_and_carries_the_
targets_defines` additionally pins the RENDERED section
(`-- [comptime.unresolved] condition names (must be 0): 0 --`) through the real
binary for three shapes, so a pasted report carries the evidence.

## §5 — The build-tier ruling

**An unresolved name in a comptime condition is ALREADY a hard build failure at
build time, and no new build-tier diagnostic is added.** Defence, in the
two-stage doctrine's terms (B′-0b):

- The one-file fact — "this condition references a name the CURRENT build's
  define set does not resolve" — is provable from one file plus the `-D` set in
  hand, and the build already hard-fails it: the evaluator reports
  `unknown name` at the miss and `lower_module` surfaces every eval diagnostic.
  Pinned executable:
  `an_unresolved_condition_name_is_a_hard_error_in_the_build_path`.
- The trap in the naive answer was measured before ruling: is a module
  legitimately referencing a define the current shape doesn't set? **No — the
  house convention is that every shape defines all five toggles with explicit
  0/1 polarity** (all eight names, including the three game-config values, in
  every `emp_defines` block), so no legitimate unresolved reference exists in
  any shipped shape, and the loud build error is correct as-is.
- What NO single build can check is that every OTHER shipped shape's define set
  also reaches the analysis — a property of (corpus × profile matrix), not of
  one file. That is the merge-gate side, and the pinned-empty corpus gate is its
  correct home. The analysis walk discards eval diagnostics wholesale (by
  design — it re-evaluates per-file in a corpus ambient), which is exactly why
  the surface had to be a first-class report field rather than "read the
  diags".

No WARN-tier lint id was added — the surface is a report field + ERROR-tier
gate, so `warn_tier_corpus.rs`'s per-shape lint-id baselines are untouched.

## §6 — The three uncovered toggles, after this parcel

The flip packet left `CRASH_REPORT`, `SOUND_DEBUG_HOTKEYS`, `SOUND_DBG_MIRROR`
with "no proof they reach the analysis at all". Now, each measured and pinned:

| toggle | its condition sites | proof, per shape |
|---|---|---|
| `CRASH_REPORT` | `Vectors`' four `if DEBUG == 1 \|\| CRASH_REPORT == 1` fault-cell arms | `a_lost_crash_report_define_is_named_in_exactly_the_plain_shapes` — stripped, it surfaces in exactly the 4 DEBUG=0 shapes; the 3 debug shapes short-circuit before it evaluates. The partition is evaluation-order truth, pinned so a condition rewrite re-measures loudly. |
| `SOUND_DBG_MIRROR` | vblank.emp's `if DEBUG == 1 && SOUND_DRIVER_ENABLED == 1 && SOUND_DBG_MIRROR == 1` | `a_lost_mirror_define_is_named_in_exactly_the_debug_sound_on_shapes` — surfaces in exactly `sonic4 debug` + `config_a`. |
| `SOUND_DEBUG_HOTKEYS` | ONLY sonic4's `implement Game` comptime group (the hook bindings) — a BIND-time condition the proc walk never sees | `a_lost_hotkeys_define_fails_the_interface_bind_in_the_sonic4_shapes` — stripped, the bind errors naming the toggle in the 5 sonic4-family shapes (and `analyze_every_shape` asserts bind-error-free, so the same gate run fails); demo's implement never references it, so demo shapes stay clean — the honest residual: a demo-only corpus would carry no proof of this toggle. |

Evaluation-site collection means a toggle is covered exactly where some shape
EVALUATES it — the short-circuit partitions above are the honest shape of that
guarantee, pinned rather than papered over.

## §7 — Bars

- **Byte bar ×7**: `cmp` against the golden blobs in `capture_goldens.sh` order
  (`s4`, `s4.debug`, `demo`, `demo.debug`, `config_a`, `config_b`, `lean`),
  canonical `s4.bin`/`s4.debug.bin` rebuilt and re-compared afterwards. **All
  nine comparisons identical**, at the branch point (step-zero baseline) and
  after the parcel. This surface is analysis-only and the bytes agree.
- **`refreeze --check`**: `OK (tip 'b-jumps', chain len 44)` — baseline and
  final.
- **Strict**, `SIGIL_STRICT_GATE=1 AEON_DIR=<b3 aeon> cargo test --workspace
  --release`, full capture, failures-first. Baseline at `de9d4ca2`:
  **3248 / 0 / 4 = 3252**, exit 0. Final: see the counts block below (recorded
  after the convergence run).
- **Test delta accounted exactly**: `#[test]` totals 3252 (master `de9d4ca2`) →
  **3265** (branch), delta **+13**, every one named, no other file's count
  moved:
  | file | master → branch | added |
  |---|---|---|
  | `sigil-cli/tests/contract_closure_corpus.rs` | 11 → 19 | `corpus_comptime_conditions_all_resolve_the_error_gate`, `a_misspelled_toggle_in_a_condition_is_named_at_zero_drops_in_every_shape`, `a_profile_that_loses_a_toggle_is_named_by_the_surface_in_every_shape` (carries the Z80-lane witness, §8.2), `an_interface_gated_arm_is_walked_under_each_shapes_own_game`, `a_lost_crash_report_define_is_named_in_exactly_the_plain_shapes`, `a_lost_mirror_define_is_named_in_exactly_the_debug_sound_on_shapes`, `a_lost_hotkeys_define_fails_the_interface_bind_in_the_sonic4_shapes`, `an_env_free_walk_names_the_interface_member_it_cannot_resolve` (lens round) |
  | `sigil-frontend-emp/tests/corpus_contracts.rs` | 29 → 34 | `a_poisoned_statement_if_is_invisible_at_zero_drops_and_the_surface_names_it`, `a_poisoned_expression_if_inside_a_comptime_fn_lands_on_the_surface`, `an_unresolved_condition_name_is_a_hard_error_in_the_build_path`, `a_memoized_poison_const_read_in_a_condition_still_lands_on_the_surface` (lens round), `a_poisoned_loop_head_lands_on_the_surface` (lens round) |
- The mid-parcel convergence run (pre-panel, commits `82704ce0`+`8dc3a360`) was
  **3258 / 0 / 4 = 3262**, exit 0, catching the last un-updated caller of
  `eval_proc_body_env` (`tests/corpus_typeenv.rs`, three destructures —
  assertions unchanged).
- One prior-session trap re-confirmed en route: `build.sh` in a fresh worktree
  pair needs `SIGIL_BUILD` pointed at the lane's own sigil binary (its default
  resolves relative to the main checkout).

### Final strict counts (post-panel tree)

STRICT RESULT: 3261 passed / 0 failed / 4 ignored = 3265 == the branch's
`#[test]` total; exit 0. (`passed + ignored` arithmetic exact; the +13 are the
thirteen tests named above.)

### Merge-prep re-prove (rebased onto `41defe98`, aeon `77f80c6`)

Both masters moved during the lane's gate runs — sigil chain entries 45
`defect-batch-8` + 46 `objtest-gate` (golden CRCs moved: the object-test scene
no longer ships in release), aeon at the objtest-gate merge. Re-proven from
scratch at the new base:

- Rebase: CLEAN, zero conflicts (the ledger appends did not collide).
- Aeon worktree refreshed to `77f80c6` (zero own commits → hard reset per
  handoff-3 §9); both gitignored seeds verified present.
- Golden target list re-derived from `crates/sigil-harness/golden/` — still the
  same SEVEN.
- Byte bar ×7 + canonical restore: **all nine comparisons identical** against
  the NEW goldens.
- `refreeze --check`: **OK (tip `objtest-gate`, chain len 46)**.
- Test delta vs `41defe98`: master total 3252 (its two chain entries added zero
  `#[test]`s net), branch 3265, delta **+13** — per-file diff shows exactly the
  parcel's two files moved, nothing else.
- Full strict at the rebased tree: **3261 / 0 / 4 = 3265**, exit 0.

## §8 — Lens panel

Three fresh read-only lenses (A ceremony/style, B corpus-pattern with the
vacuity question pointed at it, C correctness/hazard) over
`git diff de9d4ca2..82704ce0`. No lens mutated the worktree.

### §8.1 — Lens A (ceremony/style): no blockers; 4 ACCEPTED, 5 nits

ACCEPTED and fixed in the follow-up commit:

- **History narration in two .rs doc-comments** (the misspelled-toggle probe's
  "pre-existing claim was false"; the interface probe's "Before the walk
  carried…") — rewritten to present-tense contract facts; the discovery story
  stays in the ledger row.
- **The dedup sort key omitted `span.source`** — rows equal in
  `(proc, name, start, end)` but differing in source could interleave, leaving
  true exact-duplicates non-adjacent for `dedup()` and making the "sorted
  (proc, name, span)" doc claim inexact. Key now includes `source`.
- **The game-prefix derivation existed in THREE copies** (CLI + both gate
  files) — the parcel's clearest single-source miss, now
  `GameProfile::game_module_prefix()` in `native.rs` (where the "RAM module's
  parent id names the game" contract is stated once), all three sites
  delegating.
- **Three strip-probes discarded bind diags unchecked** — a bind error under a
  stripped define set would surface as a confusing downstream assert; each now
  asserts the bind stayed clean.

Also taken (nits): the report row gains a leading `UNRESOLVED` tag (grep-uniform
with `DROPPED`/`COLLISION`/`HOLE` siblings); the build-tier test doc reframed
from ruling-narration to the fact it pins; the `eval_proc_body` doc's "always
0/empty" qualified to the clean-lowering path.

Lens A explicitly VERIFIED the two load-bearing claims it was pointed at: the
collection-cannot-drift argument (harvest inline at both condition-eval sites,
every exit path) and the two-sites-only claim (the third `unknown name` emitter,
asm.rs's `dc` register-name rejection, is a rejection of a RESOLVED value — not
a lookup miss — and is correctly outside the log).

### §8.2 — Lens B (corpus-pattern / vacuity): the pin cannot pass while measuring nothing — with three accepts

Lens B's verdict on the central question: **for the axis the surface claims, the
pinned-empty gate cannot pass vacuously**, proven link by link — every chain
link (miss site → log → harvest → 5th return → aggregation → report → gate) has
a committed probe that runs the IDENTICAL walk over a doctored corpus and
demands non-empty in every shape, so a dead link fails loudly. It also verified
the two short-circuit partitions against `eval_logical_with_lhs` (both probes
assert BOTH polarities per shape plus a straddle count, so eager evaluation or
operand reorder trips a named assert, never a silent inversion), the dedup
(full-Span equality; deterministic manifest order), and the structural fact
that `ast::Item` has no item-position comptime `if` — whole procs cannot vanish
behind a toggle.

ACCEPTED and fixed in the follow-up commit:

- **Loop heads were outside the surface while the gate's doc said "every
  comptime condition"** — `eval_while`'s condition and `eval_for`'s iterable
  poison-skip their bodies with the same silence as an `if` (zero corpus
  customers today: no `while` exists and every `for` bound resolves — but the
  claim and the mechanism now agree). Both heads harvest;
  `a_poisoned_loop_head_lands_on_the_surface` pins it. (Convergent with lens
  C's census.)
- **The Z80 lane's collection had no liveness witness** — reverting its one
  `extend` would have left every committed test green, the exact per-lane
  vacuity the parcel exists to kill. The lost-DEBUG probe now additionally
  demands a row attributed to `Sequencer_NextOpcode`, a `(cpu: z80)` proc with
  an in-body `if DEBUG == 1`.
- **The multi-segment miss site was untested, and it is the interface axis's
  own failure shape** —
  `an_env_free_walk_names_the_interface_member_it_cannot_resolve` now pins that
  a walk WITHOUT an interface env surfaces
  `(Camera_Update, Game.CAMERA_JUMP_LOCK)`: simultaneously the executable
  record of the pre-parcel blind spot and the tripwire for a half-bound env
  (a missing member falls through the same dotted fallthrough).

Convergent findings (already fixed under A/C): the bind-ambient game filter
(B's "watch" = C's accept) and the triplicated game-prefix derivation (= A's
single-source fix). Ledgered from B: comptime fn bodies INHERIT the caller's
`label_ctx`, so the first define-gated `if` inside a fn called from an
immediate/call-arg position enters the label-fallback blind zone silently —
today's operand-position fns are if-free, and the sentence rides the residuals
row. Declined with reason: harvesting under the step-budget `aborted` path
(fenced indirectly by every pinned census shifting; not worth code today).

### §8.3 — Lens C (correctness / hazard): ACCEPT, no blockers; 2 fixes taken

Lens C PROVED (not argued) production-path neutrality: `with_file(f)` already
delegated to `with_file_and_ambient(f, &[])` at the parent commit, both new
wrappers reduce to the old bodies with `&[]`, the harvest emits nothing and
changes no diagnostic count/text/order, and no `lower/` file is in the diff. It
also hand-verified the Camera_Update arm (every write inside `clobbers(d0-d4/a0)`
— "zero new firings" is because the contracts are correct AND the arm is now
visible, both halves proven: the planted-`d5` probe for visibility, the drop
gate for `PL_STATE_ADDR`'s extern-equ-sum lowering) and the harvest drain's
panic-freedom (marks are monotone lengths; nothing is ever discarded).

ACCEPTED and fixed in the follow-up commit:

- **The bind ambient was whole-corpus while the bind module set was
  game-filtered** — today's cross-game const collisions are same-valued and
  unreferenced by any implement value (lens C verified empirically), but the
  first divergent collision an implement value reads would resolve
  last-indexed-wins and select bindings the shipped ROM does not. The ambient
  now runs through the same `keep` filter as the module set — strictly more
  production-faithful. (The WALK's PASS-1 env stays whole-corpus by design: it
  analyzes both games in one pass.)
- **The memoized-Poison side door** — `resolve_const` memoizes a failed
  initializer as `Poison` and returns it WITHOUT re-entering `eval_path`, so a
  const first poisoned OUTSIDE any condition left a later `if FOO == 1` with
  nothing to harvest: the original blind spot resurrected through the memo. The
  const/equ READ site now logs the const's own name whenever the resolved value
  is `Poison` (over-reporting is the safe polarity for a pinned-empty surface);
  `a_memoized_poison_const_read_in_a_condition_still_lands_on_the_surface`
  pins both reads surfacing, the second served purely from the memo. Zero new
  rows on the real corpus (the pinned-empty gate re-run stays green).

LEDGERED (the "Poison is a silence budget" census, one row): the remaining
un-harvested `Poison => skip` arms — `eval_while`, `eval_match`, the
`with … if` bracket gate (whose failure mode is a silently smaller
`context_regions` census — a pinned number), the typed `eval_path` intercepts,
`resolve_data_value`'s memo (the data-item sibling), and seam1's doctored Z80
harness still evaluating with an empty env. DECLINED with reason: a
`OnceLock`-shared undoctored-walk report across the gate tests (~14 walks
saved) — the two gate binaries run in 1.5s/0.8s release; a doctoring-adjacent
cache is a footgun bought for seconds.

## §9 — Step-3 (language/tooling) vs step-5 (engine) findings

**Step 3 — the language finding is that Poison is a silence budget.** Three
constructs discard work on `Poison` without any surface: the statement-`if`
(closed here), the expression-`if` (closed here), and the implement-group
condition (`flatten_bindings`' `continue` — covered indirectly by the
bind-error assert, not by a first-class surface). The pattern to carry forward:
every `Poison => skip` arm in the evaluator is a candidate blind spot, and the
fix that scales is a named surface pinned empty, not a per-instance probe. The
label-ctx boundary (§2) is the residual ask: a condition evaluated in a
label-value position takes the Label fallback, and whether conditions should
SUSPEND label ctx (making the miss loud there too) is a design question
deliberately not answered inside this parcel — ledgered.

**Step 5 — the engine is fine, and the interesting fact is that it was fine
invisibly.** `Camera_Update`'s landing-lock arm carried correct contracts for
code no analysis had ever read — zero new firings when it entered the walk. The
engine change required: none. What moved is that "the analysis read the arm" is
now a checked property with a game-axis probe, instead of an assumption nobody
knew was being made.

**Neither bucket — the campaign-shaped observation.** The define-gate flip's
lens B predicted the next blind arm would be "gated on corpus CONSTANTS rather
than defines" and proposed config-matrix axes or per-arm probes. The actual next
instance (`Game.CAMERA_JUMP_LOCK`) was an INTERFACE member, and the fix was
neither of the predicted shapes — it was giving the walk the binding the build
already had. The general lesson: when the analysis walk and the build disagree
about what environment code evaluates under, every gate downstream of the walk
inherits the gap silently; the surface that catches the class is "what failed to
resolve", not an enumeration of resolvable things.

Packets carry no merge-state claims.
