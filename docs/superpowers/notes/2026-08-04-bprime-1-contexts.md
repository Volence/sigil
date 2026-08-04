# 2026-08-04 — B′-1: generalized contexts (close packet)

Status: **BUILT, gate-green, UNMERGED.** Branch pair `bprime-1`; sigil off master
`6d332f5b`, aeon off master `b54f44d`. Two commits per repo (the construct, then
corpus adoption), sequenced so an adoption problem could never entangle the
construct.

Spec: `specs/2026-08-04-contract-delta-spec.md` §2 (the re-scope — B′-1 is an
EXTENSION of shipped machinery, not a new engine) over
`specs/2026-08-03-contract-unification-spec.md` §3.1–3.3 (the surface) and §8-P3
(corpus adoption).

## §0 — THE HEADLINE

**A comment became a compiler error.** `engine/level/bg.emp` carries this, written
by hand long before this parcel:

> Both counter ops are hoisted ABOVE the stop_z80 bracket on purpose:
> `.skip_tiles` sits past the matching `start_z80`, so a guard branch taken from
> inside the bracket would leave the Z80 halted for the rest of the level.

The shipped `[bus.*]` net **cannot** catch that, and not by oversight. Its MUST
lattice seeds proc entry `Unknown` (a caller may already hold the bus; not locally
provable), and at the join after `.skip_tiles` the held path meets the released
path, so the state falls to `Unknown` and the zero-false-positive stance fires
nothing at the eventual `rts`. That exact shape is now `[context.escape]`, an
error, proven on every path.

The gate has teeth on the real tree, not only in tests: mid-adoption the build
FAILED with `[context.entry-skip] in Sound_PlayMusic` (see §5.1 — it was a real
false positive in the first cut of the rule, and the rule was narrowed).
## §1 — What was verified OPEN before building

Verified own-run against the branch worktrees before a line was written:

| Claim | Verified how | Result |
|---|---|---|
| No `context` item exists | `git grep -n "Item::Context\|ContextDecl"` at `6d332f5b` | absent |
| No `with` statement exists | `git grep -n "AsmStmt::With\|at_kw(\"with\")"` | absent |
| No `requires`/`grants` clause exists | `git grep -n "requires\|grants" -- crates/sigil-frontend-emp/src/parser.rs` | absent |
| `[bus.*]` is the ONLY machine-state net, and it is inference-only | read `z80_bus.rs` in full: entry seed `Unknown`, private `BusState`, private `meet`, private `in_states` | confirmed |
| The `Cfg` is shared, not duplicated | `flag_check::Cfg` consumers: `preserves`, `out_verify`, `calls`, `branch_const`, `type_slice`, `z80_bus` | confirmed — one CFG |
| The `z80_bus` consumer list (the spec's "~10") | `grep -rn "stop_z80\|start_z80" --include='*.emp'` in aeon | **9 real consumers** + 1 comment-only mention (`release_fault.emp`). The spec's "~10" counts the comment. |
| The byte bar is SEVEN, not six | `ls crates/sigil-harness/golden/*.bin` → `s4 · s4.debug · demo · demo.debug · config_a · config_b · lean` | seven; the spec text's "×6" is stale |
| The chain tip | `refreeze --check` | `cheat-flag`, chain len 43 |
| The baseline is clean BEFORE any edit | full strict suite + all seven ROMs vs golden | 3095/0/4; all seven IDENTICAL |

## §2 — The design, and why it EXTENDS rather than replaces

### §2.1 — One CFG, one lattice, two tiers

`z80_bus.rs` shipped a private three-point MUST lattice (`BusState`), a private
`meet`, and a private worklist `in_states`, all over `flag_check::Cfg`. This
parcel moves exactly those three into `context.rs` as `Tri` / `tri_meet` /
`must_in_states` and makes `z80_bus` an instantiation:

```rust
type BusState = Tri;
const STOPPED: BusState = Tri::Held;
const RUNNING: BusState = Tri::NotHeld;
```

`must_in_states` gained two parameters the shared use demanded and nothing else:
a SEED LIST (not one seed) and a `follow` predicate that gates edge propagation.
The inference tier passes `&[(entry, seed)]` and `|_| true` — byte-for-byte its
old behaviour. The declared tier passes the region's first instruction with
`Tri::Held` and `|i| region.contains(i)`.

**Scope of that claim, corrected after the lens panel.** What is unified is the
two MACHINE-STATE tiers. The tree still holds four other worklists over the same
`Cfg` with their own lattices — `branch_const::in_states` (a line-for-line twin),
`type_slice::type_state_in`, two in `out_verify`, and `preserves`'. Making
`must_in_states` generic over the lattice and converting them is a real,
separately-scoped job; it is ledgered, not claimed here.

### §2.2 — What the region walk actually proves

A naive reading of "generalize the lattice" would run a per-context MUST analysis
over the whole proc and fire where the state disagrees. **That reproduces the
inference tier's blind spot exactly** — at the join after an escaping branch, Held
meets NotHeld, the lattice says `Unknown`, and under the zero-FP stance nothing
fires. The escaping branch is precisely the hazard the construct exists to catch.
So the declared tier scopes the walk TO THE REGION: seeded `Held` at the acquire,
propagating only inside, firing on the EDGE that leaves — before any join can wash
the fact out.

**Be precise about what that buys, because the word "lattice" oversells it** (the
panel's correction, adopted): inside a region the transfer is the IDENTITY, so
`NotHeld` and `Unknown` are unreachable there and the walk is breadth-first
REACHABILITY wearing the shared plumbing. The property checked is exactly:

> every instruction reachable from the acquire WITHIN the region has all its
> out-edges landing back inside the region.

That is reachability, not dominance — and the difference is load-bearing, not
pedantic: it is why entry-skip and the back-edge-into-the-acquire rule are
separate CHECKS rather than consequences, and a dominance proof would have caught
the back-edge hole (§9 B2) for free. The shared factoring earns its keep on the
INFERRED side, where the transfer is real and `Unknown` carries weight; here it
buys uniformity. The module header says all of this in those words so the next
reader does not assume path-sensitivity the check does not have.

### §2.3 — The marks

A `with` plants four zero-byte `CodeItem::ContextMark`s: `Enter` (before the
acquire), `AcquireEnd` (after it), `BodyEnd` (after the body, before the release),
`Exit` (after it). Each split is load-bearing for one proof, and the `AcquireEnd`
one was added BY the lens panel — §9 B1/B2 are both consequences of not having an
exact acquire range.

They ride the `CodeBuf` rather than a side table for two reasons that a side
table cannot give:

- a `Code` value is spliced by CONCATENATION (`buf.items.extend(inner.items)`),
  so a bracket inside a comptime template survives the splice with no offset
  bookkeeping — and so does a bracket nested inside ANOTHER module's template,
  which is the only way a cross-file same-context reacquire can occur;
- the shared `Cfg` indexes INSTRUCTIONS and steps over non-instruction items
  exactly as it already does for labels, so the marks cost every existing
  analysis nothing. Adding the variant required handling in **three** matches in
  the whole tree (`lower/code.rs` emit, `eval/builtins.rs` span, and the CLI
  report) — the measure of how well the item stream already tolerated it.

`BodyEnd` fences the release: without it the release's own fall-through out of the
region is indistinguishable from an escape. `AcquireEnd` fences the acquire:
without it "does this context take the bus" reads a toggle anywhere in the body
(so an outer bracket inherits an inner bus bracket), and a branch back INTO the
acquire — re-running it with no matching release — is indistinguishable from an
ordinary intra-region branch.
## §3 — The soundness argument, stated plainly

Three separate claims, each with its own evidence. They are stated apart because
they are true to different degrees and the packet should not blur them.

### §3.1 — The class becomes UNREPRESENTABLE (structural)

`stop_z80`/`start_z80` and `sr_masked` are **deleted**, not merely non-`pub`.
There is no spelling in the language for "acquire without a release": the release
is compiler-generated. What used to be a discipline enforced by a module header's
prose ("PAIRED USE ONLY") is now enforced by there being no other door.

Evidence: `grep -rn "stop_z80\|start_z80\|sr_masked" --include='*.emp'` over the
whole aeon tree returns ZERO code hits (two prose mentions remain, both rewritten
to name the contract rather than the retired template). Twin-scaffolding kill
rows 36 and 43 both close.

### §3.2 — The pairing check becomes TOTAL where brackets are adopted

`z80_bus`'s zero-false-positive stance means it fires only where the code itself
made the state definite. Its module header says so, and it costs a real class:
at a JOIN of a held path and a released path the state is `Unknown` and nothing
fires. That is exactly the shape of a branch out of a hand-written pair:

```
    stop_z80                    ; Stopped
    bne     .skip               ; --> leaves the pair with the bus HELD
    ...
    start_z80                   ; Running
.skip:                          ; meet(Stopped, Running) = Unknown
    rts                         ; Unknown -> nothing fires
```

Pinned as an ABSENCE by `the_inference_tier_cannot_see_a_branch_out_of_a_hand_written_pair`
(asserts the firing count is 0 — so if the inference tier ever grows teeth here,
the test fails and this claim must be re-derived rather than quietly inherited).

Inside a bracket the state is DECLARED, not inferred. The acquire is
compiler-generated; `[context.entry-skip]` proves no path reaches the body without
it; `[context.reacquire]` proves no path re-runs it. So the escape proof ranges
over EVERY edge out of the acquire+body with no `Unknown` to bail on, and the
shape above is `[context.escape]`. Pinned by
`escape_fires_on_a_branch_out_of_the_region` — the same source, one line
different. (§2.2 is precise about what the walk is: reachability inside the
region, with the three rules as separate checks.)

This is the concrete win that justifies the construct beyond ergonomics: not
"nicer syntax for a pair" but *a check that can be total where the inferred one
provably cannot*.

### §3.3 — The declared tier hands the inferred tier a seed it cannot compute

`requires(ctx)` / `grants(ctx)` make a proc's ENTRY state definite —
`[context.unsatisfied]` checks the requirement at every call site — so
`check_bus_state` takes `BusEntry::Held` instead of `Unknown` for a proc that
requires a bus-holding context. Pinned by the pair
`an_unpaired_toggle_at_proc_entry_is_invisible_without_a_declaration` (0 firings)
and `a_declared_bus_requirement_seeds_the_inference_tier` (double-stop +
released-at-return both fire on the SAME two bodies).

**Which context is bus-holding is read off the RESOLVED OPERAND** of what a
bracket splices, never off the context's spelling — the same discipline the net
already uses for `stop_z80` itself. A `granted` context splices nothing and never
identifies; pinned by `a_granted_context_does_not_seed_the_bus_net`.

**A polarity that had to be inverted, and was.** `[bus.stopped-at-return]` (E007)
says "a return reached with the bus provably Stopped leaves the Z80 dead". Under
a DECLARED held entry that is precisely the CONTRACT — the caller holds the bus
across the call — so keeping E007 there would fire on every correct adopter. It
is suppressed under `BusEntry::Held`, and its mirror
`[bus.released-at-return]` takes its place: a proc that returns with its caller's
hold freed. That class is reachable ONLY under a declaration (with an `Unknown`
entry there is no contract to break), which is why s4lint and the pre-context net
have no such check. Pinned in both directions —
`a_requiring_proc_that_returns_still_held_is_silent` is the no-false-positive
half.

**Honest census: `requires(z80_stopped)` has ZERO corpus adopters today.** No
aeon proc runs under a caller-held bus (every hold is self-contained inside one
proc). The §3.3 mechanism is therefore proven by tests, not by the corpus; what
the corpus DOES adopt is `requires(vblank)` (§5.3). Stated here rather than left
for a reader to discover.

## §4 — The surface

```
// ACQUIRED — the compiler owns the bracket
pub context z80_stopped {
    acquire = asm {
        move.w  #$0100, Z80_BUS_REQUEST
    .wait_z80:
        btst    #0, Z80_BUS_REQUEST
        bne     .wait_z80
    }
    release = asm { move.w #$0000, Z80_BUS_REQUEST }
}

// GRANTED — entered by hardware; a trust root, never inferred
pub context vblank { granted }
```

```
with z80_stopped {
    move.b  #1, SND_DMA_ACTIVE_SLOT
}

// comptime-gated: one body, two shapes
with z80_stopped if SOUND_DRIVER_ENABLED == 0 {
    ... the whole VDP/DMA pipeline ...
}

proc VBlank_Handler () ... grants(vblank) { ... }
proc Process_DMA_Critical () clobbers(a1/a5) requires(vblank) { ... }
```

### §4.1 — `with … if <cond>`: surface the spec did not name, and why

U-spec §8-P3 names vblank.emp's `SOUND_DRIVER_ENABLED`-conditional arm shape as
an adoption case and leaves the arms-inside-or-around choice to the porter,
"decided against bytes". Measured, **neither** works: the sound-OFF fence brackets
the WHOLE VDP/DMA pipeline, which the sound-ON build runs unbracketed. Bracket
inside the `if` duplicates the pipeline; `if` inside the bracket cannot remove the
acquire. Three sites have this shape (`VInt_Level`, `VInt_Lag`,
`Section_RedrawPlanes`).

`with <ctx> if <comptime cond> { }` resolves it in ~30 lines: a FALSE gate lowers
the body verbatim with no acquire, no release and no region — which is not a
loophole but the truth (the context genuinely is not held in that shape, so a
region there would be a lie the checker would then "prove"). Pinned by
`a_false_gate_lowers_the_body_bare` and `a_false_gate_still_lowers_nested_brackets`.

The alternative considered and rejected: declaring a SECOND, OFF-build-only
context whose acquire is comptime-conditional. It splits one hardware fact
(`the 68k holds the Z80 bus`) across two names, and the `[bus.*]` identification
would then have to treat a context as bus-holding in one shape and not another.

### §4.2 — A cross-module context must be SELF-CONTAINED

A context's `acquire`/`release` are evaluated at the USE site, in the CONSUMER's
scope — that is what makes per-site label hygiene and byte identity work. So an
imported context may only name link-resolved symbols and names the consumer
itself has. `engine.z80_bus` therefore spells its bracket inline instead of
calling a module-private `stop_z80()`; a same-module context may call whatever it
likes.

This was discovered during adoption and it CHANGED the plan: `stop_z80`/
`start_z80` could not merely go non-`pub` (an imported context clone could not
then resolve them), so they are DELETED and the context IS the definition —
strictly stronger than the spec asked for. **Ledgered:** a diagnostic that names
this rule at the context's DECL site (today the failure surfaces at the use site
as an ordinary unknown-name error).
## §5 — Corpus adoption (aeon), and what it discharged

15 `.emp` files. **Byte-neutral on all seven targets** (§7) — contracts and
brackets are metadata plus the identical bytes the manual pair emitted.

### §5.1 — `z80_stopped`: every consumer, and the templates deleted

The measured consumer list (own-run `grep`, not the spec's): `engine/z80_bus.emp`
(now the context), `engine/sound/sound_api.emp`, `engine/system/{vblank, boot,
controllers}.emp`, `engine/level/{section, bg, parallax}.emp`,
`engine/debug/sound_debug.emp`, `games/sonic4/test/ojz_scroll_test.emp`. That is
**9**, not ~10 — `engine/system/release_fault.emp` only MENTIONS `stop_z80` in a
comment ("NO stop_z80 — the busy-wait could hang forever"), rewritten to name the
contract.

`pub comptime fn stop_z80` / `start_z80` are **deleted** (kill row 36).

**The gate fired on the real tree during adoption.** The first cut of
`[context.entry-skip]` — "any branch from outside whose target lands inside the
region" — failed the build at `Sound_PlayMusic`:

```
.await_slot:
    with z80_stopped { move.b MUSIC_SLOT, d1 }
    tst.b   d1
    bne     .await_slot
```

A label targets the first instruction at or after it, so `.await_slot` resolves
INSIDE the region — but branching there re-takes the whole acquire, which is
correct. The rule narrowed to "inside the region AND not the region's first
instruction". Regression-pinned by
`a_loop_label_at_the_region_head_is_not_an_entry_skip`. Two corpus sites have
this shape (`Sound_Init.wait_alive`, `Sound_PlayMusic.await_slot`).

**That narrowing then opened a hole the lens panel found** — a branch back into
the acquire from INSIDE the region is the same edge shape and is NOT correct. The
two are told apart by where the edge STARTS, which the first rule threw away; see
§9 B2 for the `AcquireEnd` fence that recovers it.

### §5.2 — `ints_off`: `sr_masked` retired (kill row 43)

`engine/irq.emp`'s `sr_masked(code)` had 4 call sites, all of the form
`sr_masked(asm { {stop_z80()} … {start_z80()} })`. All four become NESTED
brackets (`with ints_off { with z80_stopped { … } }`) — the corpus's own witness
that nesting DIFFERENT contexts is legal and that the construct is not
bus-specific. `ojz_scroll_test.emp`'s hand-spelled SR pair around a bus hold
adopts the same nesting, making 5 `ints_off` regions. `sr_masked` is deleted.

The sites that stay hand-spelled are named in `engine/irq.emp`'s header with the
reason (a bracket is ONE lexical region with one entry and one exit): the
dma_queue 3-exit unwind, `Sound_Init`'s re-mask loop, and the long spans whose
masked region is a whole code block with its own internal structure
(`Section_RedrawPlanes`, `BG_Init`, `Sound_DrainSfxRing`). The STRUCTURAL
exclusion — a bracket with a CCR out-contract can never adopt, because the
full-SR restore overwrites CCR — is unchanged and still recorded.

This discharges the **t21 panel A1+B1 `z80_held(code)` / DMA-window bracket
demand row** (gap ledger ~:1550, demand 6). Its recorded OPEN QUESTION ("can
comptime `if DEFINE == 1 {}` arms appear INSIDE a `-> Code` fn body?") is
answered by not needing the answer: the construct is a statement bracket, not a
Code-taking fn, so the comptime `if` composes at statement position where it
already worked, and the wide OFF-build fence rides `with … if` (§4.1).

It also discharges the **t21 `sr_masked` code-argument validation ask**
(~:1551): "a Code argument containing an escaping branch skips the SR restore
(interrupts dead + leaked stack word)" was prose in a module header. It is now
`[context.escape]`, an error.

### §5.3 — `vblank`: the granted context, and where `requires` gets its teeth

`context vblank { granted }` in `engine/irq.emp`. `VBlank_Handler` — the hardware
entry point — declares `grants(vblank)`; nine procs declare `requires(vblank)`:
`VInt_Level`, `VInt_Lag`, `Enqueue_Dirty_Buffers`, `VInt_DrawLevel`,
`Process_DMA_Critical`, `Process_DMA_Important`, `Process_DMA_Deferrable`,
`Vscroll_Write`, `Read_Controllers`.

Each was verified own-run to have NO caller outside the two VBlank handlers
(`grep` over `*.emp` AND `*.asm`). `Flush_VDP_Shadow` is deliberately EXCLUDED —
`boot.emp` calls it too, so the claim would be false.

What this buys: calling `Process_DMA_Critical` from the main loop is now a
compile-time `[context.unsatisfied]`, not a timing bug found on hardware. It is
also the reason `requires` propagation is not vacuous on this corpus.

**Honest limits of the vblank adoption:**
- The chain is 2 deep (grant root → requiring proc), not 3. `VInt_Level` is
  reached from `VBlank_Handler` through an INDIRECT `jsr (a0) as VBlankHandler`,
  which the per-call-site check does not resolve (the same indirect hole the
  clobber closure has). Three-deep propagation is pinned by the unit test
  `requires_propagates_three_deep_from_a_grant_root`, not by the corpus.
- `grants` is an UNVERIFIED trust root by design (§3.2 of the U-spec): the
  assembler cannot check hardware dispatch. It is greppable and censused — and
  after the panel it no longer seeds the `[bus.*]` net (§9 B3).
- `grants` of an ACQUIRED context is now rejected outright, so the only grant the
  corpus can carry is of a `granted` one.

### §5.4 — The corpus census (own-run, `emp_contracts` over the real tree)

| shape | `with` regions | claims | discharged sites | bracket firings | unsatisfied | unknown ctx |
|---|---|---|---|---|---|---|
| `-D SOUND_DRIVER_ENABLED=1` | 23 | 10 | 12 | 0 | 0 | 0 |
| `-D SOUND_DRIVER_ENABLED=0` | 20 | 10 | 12 | 0 | 0 | 0 |
| no `-D` (the corpus gate's shape) | 17 | 10 | 12 | 0 | 0 | 0 |

Claims (1 grant + 9 requires) and discharged call sites are shape-independent — a
clause is a declaration, not code. `bus_contexts = {z80_stopped}` EXACTLY,
identified from what each bracket's ACQUIRE splices (the panel found this reading
the body too; see §9 B1).

The three shapes differ exactly by the comptime-gated arms; in the no-`-D` shape
`SOUND_DRIVER_ENABLED` does not resolve, so the three wide fences lower their
bodies bare and contribute no region. Those three are proven by the PER-FILE gate
in every shape the ×7 byte bar builds. Recorded in the gate's own comment so the
17 is not read as under-adoption.

### §5.5 — A silent under-approximation caught mid-parcel

After adoption the corpus census read **0 regions** while the build emitted them
correctly. Cause: `corpus_contracts::collect_env` (the whole-corpus type
environment PASS 1) did not carry `Item::Context`, so every `with` in the walk's
evaluator failed to resolve and lowered BARE — the walk's `CodeBuf`s were missing
every bracket's acquire and release.

This is invisible to the `dropped_instrs == 0` gate: nothing is *dropped*, the
statement simply lowers to less. Every downstream net (`[bus.*]`, the clobber
closure, dead-save, liveness) would have silently under-approximated. Fixed by
adding `Item::Context` to `collect_env` with the reason recorded at the arm.

A second instance of the same class, caught by the same measurement: `lower_with`
originally `return`ed on a Poison gate, DROPPING the whole body. Unlike an `if`
branch, a bracket's body is real code in every shape — only the acquire/release
are conditional — so it now lowers the body bare. The whole VBlank pipeline sits
inside one of those gates.

## §6 — Tests, and why each is non-vacuous

35 new tests: 33 in `crates/sigil-frontend-emp/tests/context_brackets.rs`, 2 in
`crates/sigil-cli/tests/contract_closure_corpus.rs`. NINE of the 33 were written
by the lens panel's findings and each pins a hazard a lens constructed (§9).

**The two ABSENCE tests are the ones to scrutinise**, since an
assert-zero passes trivially if the checker is broken. Both are paired with a
same-source positive (and the panel found three MORE bare-absence tests, all now
carrying an in-test positive — §9):

| absence test | its positive twin | why the pair cannot both be vacuous |
|---|---|---|
| `the_inference_tier_cannot_see_a_branch_out_of_a_hand_written_pair` (0 `[bus.*]`) | `escape_fires_on_a_branch_out_of_the_region` (1 `[context.escape]`) | the SAME program, one line different in spelling. If `check_contexts` returned empty, the positive fails. |
| `an_unpaired_toggle_at_proc_entry_is_invisible_without_a_declaration` (0) | `a_declared_bus_requirement_seeds_the_inference_tier` (1 double-stop + 1 released-at-return) | same two bodies, `requires(z80_stopped)` added. Also asserts `bus_contexts` is non-empty IN THE ABSENCE TEST, so the seed the positive exercises is provably reachable. |

Every "must not fire" test names a shape a naive implementation gets wrong:
- `a_branch_to_a_label_inside_the_region_is_not_an_escape` — a range test would reject every real bracket.
- `a_call_out_of_the_region_is_not_an_escape` — `Read_Controllers` calls a local sub-block from inside its bracket.
- `a_loop_label_at_the_region_head_is_not_an_entry_skip` — the false positive that failed the real build (§5.1).
- `a_requiring_proc_that_returns_still_held_is_silent` — the E007 polarity inversion (§3.3).
- `a_granted_context_does_not_seed_the_bus_net` — asserts a double-stop STILL fires there (1, not 0), so it distinguishes "no seed" from "no checking".
- `a_bracket_discharges_a_requirement_over_its_range_only` — the same callee inside and after the bracket; a bracket-wide discharge would make it vacuous.

`a_bracket_emits_the_same_bytes_as_the_manual_pair` links both spellings and
compares bytes, asserting the manual side is non-empty first — the unit-level
statement of the ×7 bar.

The two corpus gates carry FOUR anti-vacuity pins, because an assert-empty is only
as good as the set it ranged over:

1. the CLAIM census, exactly 10, with the grant ROOT named — delete the root and
   every requirement below it is discharged by nothing while the gate still reads
   empty;
2. the BRACKET census, exactly 17, with two named witnesses so an un-adoption that
   coincidentally preserves the count still fails;
3. the DISCHARGED census, non-empty — `context_unsatisfied` being empty proves
   nothing unless call sites were EXAMINED, and `call_target_sym` resolves a direct
   call only, so a refactor to indirect dispatch would have emptied the examined
   set silently (the panel's catch);
4. `bus_contexts` by EQUALITY, not `contains` — a `contains` passes with a wrong
   SET, which is exactly the bug §9 B1 was.
## §7 — Byte identity (own-run, SEVEN targets; re-proven at chain 44 for the merge)

The bar is SEVEN, derived from `crates/sigil-harness/golden/` in this worktree,
not from a remembered count: `s4.bin`, `s4.debug.bin`, `demo.bin`,
`demo.debug.bin`, `config_a.bin`, `config_b.bin`, `lean.bin`. (The delta spec's
"×6" is stale; the count went 4→6→7.)

Build order per `golden/capture_goldens.sh` — the four canonical via
`./build.sh <game>` one shape per invocation (`DEBUG=1` for debug), then
`--config-a -o s4.debug.bin`, `--config-b -o s4.bin`, `--lean -o s4.bin` via
`sigil build`; config_b and lean both clobber `s4.bin`, so canonical is rebuilt
after. Compared with `cmp` against the frozen golden blobs.

| run | chain | result |
|---|---|---|
| BASELINE (before any edit, both worktrees) | 43 | all seven IDENTICAL |
| after commit 1 (construct only, no `.emp` touched) | 43 | all seven IDENTICAL |
| after corpus adoption | 43 | all seven IDENTICAL |
| after the lens-panel fixes | 43 | all seven IDENTICAL |
| **MERGE REBASE onto chain-44 masters (overseer countersign)** | **44** | **all seven IDENTICAL** |

The merge rebase moved both repos to sigil `a846426a` / aeon `8bfe0ba` (the
engine session's `b-jumps` parcel, refreeze chain 44) and every one of the seven
was re-derived and re-compared there. Five of the seven CRCs moved between
chain 43 and 44 — which is the whole reason the table records a chain per row and
quotes no CRC literal.

The warn-tier tally was re-measured at chain 44 and is unchanged from master:
`30 warnings — module.path-mismatch 12, proc.sr-undeclared 8,
proc.undeclared-fallthrough 6, proc.out-unwritten 3, proc.clobber-undeclared 1`.
Deleting `sr_masked` and moving its `sr` push/pop into `ints_off`'s
acquire/release does not move `proc.sr-undeclared`, so the frozen lint-id set
needed no edit.

No CRC literal is quoted here: the goldens ARE the bar and `cmp` is the
comparison. `repin --check` → `pins.rs unchanged`. `refreeze --check` → OK, tip
`cheat-flag`, chain len 43 (unmoved — nothing to re-freeze on a byte-neutral
parcel).

The warn tier is unchanged: the firing lint-id SET per shape is the same five
ids, and the per-shape tallies are identical to baseline
(sonic4 plain 30 / debug 81 / demo plain 28 / demo debug 78 / config_a 81 /
config_b 30 / lean 30). `warn_tier_corpus.rs`'s frozen baseline is therefore
**untouched** — this parcel adds and removes no firing lint id.

## §8 — Strict suite

`AEON_DIR=<own worktree> SIGIL_EMIT=… SIGIL_BUILD=… cargo test --workspace --release`,
full capture to file, failures-first.

| | chain | passed | failed | ignored | result lines |
|---|---|---|---|---|---|
| master `6d332f5b` baseline (own run) | 43 | 3095 | 0 | 4 | 308 |
| branch `bprime-1` | 43 | **3130** | **0** | 4 | 309 |
| **branch, MERGE REBASE onto `a846426a` (overseer countersign)** | **44** | **3130** | **0** | **4** | 307 |

Delta **+35**, accounted for exactly: 33 in the new
`sigil-frontend-emp/tests/context_brackets.rs` + 2 new gates in
`sigil-cli/tests/contract_closure_corpus.rs`.

The countersign's RESULT-LINE count is 307, two below the chain-43 run, while
passed/failed/ignored are identical. That is master's own binary packaging moving
between chain 43 and 44, not a skipped binary here — the arithmetic below is the
check that settles it, and it balances on the chain-44 numbers exactly
(3130 + 4 = 3134 = the branch's own `#[test]` total). Recorded rather than
smoothed over: a result-line count that drops while the test count holds is
exactly the shape a silently-unbuilt test binary would also have, so it is worth
saying which of the two it was and how that was determined.

Cross-check that nothing is silently skipped:

```
git grep -c '^\s*#\[test\]' 6d332f5b -- 'crates/**/*.rs'  →  3099   (= 3095 + 4 ignored)
git grep -c '^\s*#\[test\]' <branch> -- 'crates/**/*.rs'  →  3134   (= 3130 + 4 ignored)
```

Both sides balance exactly.

`cargo clippy -D warnings` still fails on the pre-existing
`sigil-ir/src/symbols.rs:55` (not this parcel's). No NEW finding in any file this
parcel touches — the warnings reported inside `sigil-frontend-emp` were verified
present at `6d332f5b` by `git show HEAD:<file>` at the shifted line. (One NEW
clippy finding did appear in `context.rs` after the panel fixes — a collapsible
`if` inside a match arm — and was fixed rather than accepted.)

**A trap worth recording, hit once here.** A suite run that ABORTS partway leaves
the aeon tree's `s4.bin` clobbered by whichever off-canonical shape a test built
into it, and the next targeted port-test run then compares against that stale ROM
and reports a byte diff that looks exactly like a real regression. The recovery is
to rebuild canonical (the byte-bar script's own restore step) before believing any
port-test byte failure. This is the ledgered shared-tree stale-artifact class; it
cost one confused diagnosis.
## §9 — Lens panel adjudication

Three fresh read-only lenses over the finished diff. **They caught three
BLOCKERS, two of them live on the shipping corpus**, and one of them overturned a
claim this packet was about to make. Every finding is adjudicated below.

### FIXED — soundness (the blockers)

**B1 · `region_acquires_bus` scanned the acquire AND the body — `ints_off` was a
"bus context" on the real corpus today.** (Lens B #1 and Lens C #2, found
independently.) `items[region.enter..region.body_end]` spans both, and the corpus
nests `with ints_off { with z80_stopped { … } }` at four sites, so the OUTER
region's range contained the INNER acquire's bus request. Inert only because no
proc requires `ints_off` yet — and the first one written would have been analyzed
from a bogus `BusEntry::Held`, which SILENCES `[bus.vdp-write-unstopped]` (the
crash class, which fires only on a definite Running) for that whole proc, plus
inverts the return rule. A declared fact that is wrong is worse than none.

Fixed by planting a fourth mark, `ContextMarkKind::AcquireEnd`, so the acquire has
its own exact range; the scan is now `enter..acquire_end`. The gate that let it
through (`bus_contexts.contains("z80_stopped")`) is replaced by an EQUALITY pin,
and `an_outer_bracket_nesting_a_bus_bracket_is_not_a_bus_context` builds the exact
shape and asserts `ints_off` is absent.

**B2 · a back-edge from the body INTO the acquire re-ran it, unproven by all three
rules.** (Lens C #1.)

```
.spin:
    with ints_off {          // acquire = move.w sr,-(sp) ; move.w #$2700,sr
        tst.b   Flag
        bne     .spin        // back to the ACQUIRE; the release never runs
    }
```
`.spin` resolves to the region's first instruction, so escape saw a target inside
the region, entry-skip skipped it (source inside the region) and would have
excluded it anyway under the `t != entry` narrowing, and mark nesting saw one
region. For `ints_off` that is an unbounded `move.w sr,-(sp)` stack leak.

**This hole was introduced by the narrowing that fixed the legitimate spin-probe
(§5.1)** — the two shapes differ only in where the edge STARTS, and the first cut
of the rule threw that away. The `AcquireEnd` mark makes both expressible: an edge
into the acquire from OUTSIDE the acquire is `[context.reacquire]`; the acquire's
own `bne .wait_z80` spin starts inside it and is correct. Both directions pinned
(`a_back_edge_into_the_acquire_is_a_reacquire`,
`the_acquires_own_spin_is_not_a_reacquire`).

**B3 · `grants(<acquired context>)` was accepted and seeded the bus net from an
unverified root.** (Lens C #3.) `with <granted>` was rejected but the mirror was
not, so the obvious spelling of "this proc establishes the context" —
`grants(z80_stopped)` on a proc that then brackets — produced TWO false positives:
`[bus.double-stop]` on the compiler's own acquire and `[bus.released-at-return]`
on the `rts`. Separately, seeding a net that gates a crash class from an
explicitly UNVERIFIED trust root is the wrong direction of trust.

Both closed: `grants` of an acquired context is `[context.not-grantable]`
(error, per-file gate), and `BusEntry::Held` is now seeded from `requires` ONLY —
the half that has `[context.unsatisfied]` behind it at every call site.

### FIXED — soundness (should-fix)

- **Three AST walkers had no `With` arm** (Lens B #5, Lens C #4/#5):
  `walk_body_for_indirect` (an unbounded `jsr (a1)` inside a bracket contributed
  no ⊤ to the clobber closure — an under-approximation on a shipping ERROR gate),
  `collect_discarded` (a `@discards` inside a bracket stopped working), and two in
  `lower/script.rs`. All four arms added. Latent today; exactly the class the
  campaign's walkers exist to prevent.
- **`bsr External` inside a region was a false escape** (Lens B #7, Lens C #6):
  the shared CFG classifies `bsr` as a conditional branch on mnemonic shape
  (`b` + 3 chars), so its `Defer` read as a path out. A call RETURNS; `Defer` from
  a call mnemonic is no longer an escape. Pinned
  (`a_bsr_out_of_the_region_is_not_an_escape`) — the existing test pinned `jbsr`
  only and would have passed while the `bsr` spelling of the same intent failed.
- **entry-skip applied `branch_target` to every instruction** (Lens B #7b, Lens C
  #8): `branch_target` is "the last `Sym` operand", so `lea .inner(pc), a0` fired
  with a message about a branch that does not exist. Gated on a transfer
  predicate; pinned.
- **An `export` label inside a region was an unchecked entry point** (Lens C #7):
  it takes the stable `Owner.name` symbol, so any other proc can branch straight
  past the acquire, and a one-proc item scan cannot see it. Now fires entry-skip.
- **`[context.unknown]` on `requires`/`grants` was report-only** (Lens B #8) while
  the same id on `with` failed the build. Moved to the per-file gate. Doing so
  surfaced a second thing: the port harnesses lower ONE module standalone, with no
  resolver and no injection, so the gate now also accepts a name a
  `use m.{name}` brings in — an import the author wrote is not the typo the check
  exists to catch.
- **`context_firings` had no final sort** despite its doc claiming one (Lens B #6)
  — `proc_bufs` is in discovery order, not name order.

### FIXED — the claim that was wrong

**Lens C's item 4 overturned this packet's own framing.** The draft §2.2 said the
declared tier "instantiates the same lattice, scoped to the region", implying a
path-sensitive proof. Measured: inside a region the transfer is the IDENTITY, so
`NotHeld`/`Unknown` are unreachable and `must_in_states` there is breadth-first
REACHABILITY wearing the shared lattice's plumbing. The property actually checked
is:

> every instruction reachable from the acquire WITHIN the region has all its
> out-edges landing back inside the region.

That is reachability, not dominance — and the difference is load-bearing: it is
exactly WHY entry-skip and the back-edge rule have to be separate checks rather
than consequences. A dominance proof would have caught B2 for free. The module
header now says this in those words, and §2.2/§3.2 of this packet are rewritten
to match. The shared factoring earns its keep on the INFERRED side, where the
transfer is real and `Unknown` carries weight; on the declared side it buys
uniformity, not power. Ledgered.

**Relatedly, Lens B #2 caught a false claim in the same header**: "there is
exactly one worklist, one meet, and one CFG in the tree" is not true —
`branch_const`, `type_slice`, `out_verify` (×2) and `preserves` each still carry
their own worklist over the same `Cfg`. What B′-1 actually did is unify the two
MACHINE-STATE tiers. The sentence now says that, and names the residue. Making
`must_in_states` generic and converting `branch_const` (its line-for-line twin) is
ledgered, declined in-parcel as scope creep into byte-neutral code this parcel
does not otherwise touch.

### FIXED — anti-vacuity

- **The requirement gate had no DISCHARGED census** (Lens C #9). `context_unsatisfied`
  being empty means nothing unless call sites were examined, and `call_target_sym`
  resolves a DIRECT call only — a refactor to indirect dispatch would have emptied
  the examined set while both existing pins stayed green. `context_discharged` now
  records every satisfied site (corpus: **12**) and the gate asserts it non-empty.
- **Three tests were bare assert-absence** (Lens C #10):
  `a_call_out_of_the_region_is_not_an_escape`,
  `a_loop_label_at_the_region_head_is_not_an_entry_skip`,
  `a_requiring_proc_that_returns_still_held_is_silent` all passed against a checker
  returning `vec![]`. Each gained an in-test positive proving the region existed
  (or the claim was made) to be checked.
- **The bracket census was a bare count** (Lens B #12). Added named witnesses
  (`Read_Controllers`/`z80_stopped`, `Sound_PostByte`/`ints_off`) so an
  un-adoption that coincidentally preserves 17 still fails.
- **`bus_contexts` was pinned with `contains`** — replaced by equality (see B1).

### FIXED — consolidation and ceremony

- `regions_of` ran TWICE per proc in the corpus walk (Lens B #4): `check_contexts`
  split into `check_contexts` (per-file entry) and `check_regions` (takes
  already-recovered regions), so one mark scan feeds the census, the bus-context
  identification and the firings.
- `collect_context_kinds` cloned a `ContextKind` (two `Expr` trees) per context for
  a value never read (Lens A #10) → `collect_context_names` returning a `BTreeSet`.
- `tri_meet`/`edges_for` were `pub(crate)` with no outside caller (Lens A #21);
  `BusEntry`'s `Default` derive had no user (Lens A #22). Both narrowed.
- The `with … if` gate reused `[asm.if-not-comptime]`, an id naming a construct
  that did not fire (Lens A #16) → `[context.gate-not-comptime]`.
- `emp_contracts` printed `{:?}` kinds where every sibling row prints the literal
  diagnostic id, mis-padded one row, and left the claim census unheralded (Lens A
  #14/#19/#20). All three fixed.
- **Doc examples taught a pattern the parcel forbids** (Lens A #1/#2, BLOCKER-rank
  in that lens): `ast.rs` and `parser.rs` showed `acquire = stop_z80()` — a name
  this parcel deletes, and a cross-module shape §4.2 rules out. Rewritten to the
  real inline spelling, with the consumer-scope rule stated.
- Dead template names in live prose (Lens A #5/#6/#7/#18, #15): `z80_bus.rs`'s
  recognition paragraph and four firing docs, one test doc, and the two
  LOAD-BEARING aeon docs (`docs/specs/boot-ym-keyoff-race.md`, which `boot.emp`
  tells the reader to consult, and `ENGINE_ARCHITECTURE.md` §9.4).
- Change-history narration re-emitted in rewritten paragraphs (Lens A #8/#27):
  the loop-label test doc, `section.emp`'s "until this was fixed" and "now owns".
- `engine/irq.emp`'s hand-spelled-by-design list had lost three of its five rows
  in the rewrite, making it a false claim (Lens A #3); its `preserves(sr)` sentence
  was unqualified and false for 2 of 5 adopters (Lens A #4). Both corrected.
- Whitespace/wrap: the one stray blank line inside a `{` (Lens A #11), the
  duplicated Sound-OFF paragraph (#12), the orphan comment (#13), five ragged
  rewraps (#23), and the two verbatim-identical fence comments now
  cross-referenced "see VInt_Level" per the file's own convention (#24).
- `check_context_regions` renamed `check_context_brackets` — it reports all three
  classes, one of which is not a region property (Lens A #28).

### DECLINED, with reason

- **Move `Tri`/`must_in_states`/`edges_for` next to `Cfg` in `flag_check.rs`, and
  convert `abandons_flag`'s private CPU-selection closure** (Lens B #3). The
  tier-inverted import (`z80_bus` importing from `context`) is a real smell and the
  observation is correct. Declined here because the right home is a new
  `dataflow.rs` alongside the generic-lattice work (Lens B #2), and doing half of
  it now means touching it twice. Ledgered together.
- **Widen `ProcNode::direct_callees` to carry index+span** so the requirement check
  need not re-scan the CodeBuf for call sites (Lens B #11). One extra scan today;
  the widening touches the closure fixpoint's inputs, which is byte-neutral code
  this parcel has no other reason to disturb. Recorded as the right move IF the
  pattern recurs.
- **Walk `z80_proc_bufs` through `check_contexts` with `Cpu::Z80`** (Lens B #9).
  No Z80 module has a bracket, so it would add a loop over an empty set. Instead
  the 68k-only scope is now stated in the census's field doc (a bracket in a Z80
  module is proven by the per-file gate, which threads the module's real CPU) and
  the limit is ledgered.
- **`edges_for` computed twice per in-region instruction** (Lens B #10). Regions
  are ≤ 2 per proc and a handful of instructions each; folding the escape test into
  the transfer would make the transfer do two jobs. Cost measured as noise.
- **Stale mentions in `DEFERRED_WORK.md` and the crash-reporting plan** (part of
  Lens A #15). Those are dated RECORDS of decisions, not live guidance; the house
  treats a record's text as historical. The two LOAD-BEARING docs were fixed.
- **Lens A #25's re-indent policy split in `vblank.emp`** (comment columns squeezed
  on two lines to hold their absolute column, shifted elsewhere). Left as-is: the
  squeezed lines keep the `SND_DMA_ACTIVE_SLOT` comments aligned with their
  neighbours, which is the local convention those lines were written to.
## §10 — Step-3 (LANGUAGE) vs step-5 (ENGINE)

### Step 3 — LANGUAGE asks (the compiler owes the author)

1. **A context decl's self-containment should be diagnosed at the DECL, not the
   use.** §4.2's rule (a cross-module context's acquire/release evaluate in the
   consumer's scope) is discovered today as an ordinary unknown-name error at
   whichever consumer imports it first. A `pub context` whose halves name a
   non-`pub` same-module fn is statically detectable at the declaration.
   *(ledgered)*

2. **`requires` inference and promotion.** Propagation is checked but never
   INFERRED: a caller must spell the residue itself. U-spec §7's
   `--report contracts` (B′-4) should print each proc's DERIVED context
   requirement in paste-able annotation form, exactly as it will for register
   facets. The report surface for the grant/bracket census already exists in this
   parcel's `emp_contracts` output; B′-4 promotes it.

3. **An indirect call site cannot carry a context bound.** §5 of the U-spec gives
   `jsr (aN) as Type` a clobber bound; there is no way to say "every installable
   target of this dispatch requires `vblank`". That is why the vblank chain
   measures 2 deep rather than 3 (§5.3). The natural spelling is
   `requires(...)` inside a `type X = proc (...)` contract type. *(ledgered)*

4. **`with` cannot bracket a MULTI-EXIT region.** A bracket is one lexical region
   with one entry and one exit. The dma_queue 3-exit unwind and `Sound_Init`'s
   re-mask loop stay hand-spelled for that reason (named in
   `engine/irq.emp`'s header). An unwind-aware form — the question the t21 row
   left open for exactly this residue — is still open, now with a construct to
   extend rather than a blank page.

5. **`@allow` has no story for `[context.*]`.** §6's tier map says a context
   violation is error-tier with no `@as_compat` softening and no `@allow`. That
   is right for escape/entry-skip; `reacquire` is the one a future `reentrant`
   context property would relax (the U-spec names it). No demand today.

### Step 5 — ENGINE asks (the code owes itself)

1. **`Section_RedrawPlanes` owns its Z80 posture in both shapes and says so three
   times.** The header note, the open-bracket comment, and the close-bracket
   comment all restate "callers must NOT wrap it". With `z80_stopped` now a
   declared context, the honest spelling of that fact is a contract, not prose —
   but the language has no "MUST NOT be called under ctx" (an ANTI-requirement).
   Recorded as a language ask driven by an engine site; not built (one site).

2. **`Sound_Init`'s re-mask loop is the last SR bracket that cannot adopt.** It
   saves SR once outside the loop and re-masks each iteration. Restructuring it
   to a single `with ints_off` around the whole loop would change bytes (the mask
   write would leave the loop), so it is a byte-changing engine question, not a
   metadata one. Deferred to a parcel that owns its bytes.

3. **`Flush_VDP_Shadow` is called from both VBlank and boot** and so cannot claim
   `requires(vblank)`. If boot's use is really a one-shot init that could route
   through a different entry, the claim would become available and the vblank
   contract would cover the whole pipeline. An engine question, unmeasured here.

### Neither bucket — the headline

**A silent under-approximation can hide behind a gate that is watching the wrong
thing.** `dropped_instrs == 0` is the corpus's substrate gate, and it stayed
green while the contract walk lost every bracket's acquire and release (§5.5) —
because nothing was *dropped*, a statement simply lowered to less. The gate
measures instructions that failed to lower, not instructions that were never
constructed. Both instances this parcel hit were caught by a CENSUS (the region
count read 0 when the build emitted 23), not by any existing gate.

The general lesson for the next construct that lowers to a variable number of
instructions: **ship a census with it, and pin the census.** That is why both new
corpus gates carry an exact count rather than an assert-empty, and it is the
cheapest available defence against the class.
