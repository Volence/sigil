# 2026-08-05 — `Edge::Return` / `Edge::FallOff`: the CFG builder decides once

Status: Merge state lives in the campaign log, not here. Branch `edge-split` off sigil `ea7b1c36`, **zero commits in aeon** — a
sigil-only parcel. Worktrees `sigil/.worktrees/b4` + `aeon/.worktrees/b4`
(aeon at `77d5317`, read-only reference).

Four commits, and the split is only the first of them:

| | | |
|---|---|---|
| `56a0f1c6` | the split — one variant becomes two, decided at construction | byte- and diagnostic-neutral |
| `fba2146e` | **FINDING 1**: Z80 `call cc` was still classified as a conditional branch | behaviour change, corpus-neutral |
| `8513ecbc` | the lens panel's documentation / naming / test dispositions | byte- and diagnostic-neutral |
| `1a52e010` | **FINDING 2**: a Z80 jump to a label that CLOSES the proc read as a transfer out | behaviour change, corpus-neutral |

The two behaviour changes are isolated in their own commits, each with its own
probe and its own corpus-census proof, so a reviewer can read the mechanical
refactor without them.

**Read §0, §5 and §8 if you are short of time.** §5 carries TWO live soundness
defects this work turned up in the substrate it was refactoring — one of them
found by the panel, both ERROR-tier, both silent.

---

## §0 — THE HEADLINE

`Edge::Abandon` meant *either* "this path RETURNED from the proc" *or* "control
ran off the end of the body". Five analyses consume these edges and each one
needs a policy about the difference, so each re-derived it by reading the exit
instruction's mnemonic back off the item list — with a CPU it had to supply
itself. **That re-derivation shipped a soundness defect in three consecutive
parcels, in both polarities**, and its sharpest instance was a pair of literally
equal values: `flag_check.rs:359` returned `vec![Edge::Abandon, Edge::Abandon]`
for a `ret cc`, one of which returned and one of which did not.

The variant is now two variants, chosen where the edge is CONSTRUCTED. The
builder is the only place that holds all three facts the classification needs —
the mnemonic, the CPU's terminator set, and which side of a conditional
terminator is being emitted — and it is now the only place that uses them.

What that buys, concretely:

- An end-of-body `ret cc` presents `[Return, FallOff]`. There is no positional
  rule and no mnemonic test left for a consumer to get wrong, because the two
  edges are no longer the same value.
- A consumer cannot pick the wrong CPU's return table, because there is no table
  left for it to pick — WHICH BUILDER produced the edge already settled it.
- `is_return_mnemonic` is now a BUILDER's classifier plus exactly one other
  caller — the only one with no CFG to ask: `z80_cycles::span_cost` walks a
  straight-line item slice. (`56a0f1c6`'s message says "exactly one caller",
  which overcounts the deletion: the function has three call sites, two of them
  inside `Cfg::z80_edges`. Lens A caught it; the code doc is correct.)
- Two consumers' compensating logic is DELETED rather than migrated: the cycle
  budget's `this_edge_returns` positional flag (B′-3a's fix) and the stack
  checker's mnemonic re-read (B′-2's fix). Both were correct; both are now
  unnecessary, which is the difference between fixing a bug and removing the
  place it could occur.
- **There is now ONE Z80 edge builder, not two.** `z80_preserves` carried a
  private duplicate, 79 lines with its own copy of the return table and a
  character-identical `branch_sym`. Finding 2 made the two agree on their last
  divergent input; the duplicate is deleted and the Z80 proof calls the shared
  builder. There is no longer a pair to keep in sync.

**Byte-neutral ×7. Zero diagnostic change across all seven shapes** (§7.5 —
own-run census, diffed line-for-line against the branch point, re-run at final
HEAD after both findings landed).

---

## §1 — The design

```rust
pub(crate) enum Edge {
    Follow(usize),  // stays in this proc, at item index .0
    Return,         // executed a return instruction for THIS CPU
    FallOff,        // ran past the last instruction; no return executed
    Defer,          // left for a symbol local analysis cannot read
}
```

TWO builders classify, one per CPU, and each is total over its own terminator
set. (It was three when the split landed; `1a52e010` deleted the duplicate — §5.2.)

| builder | `Return` | `FallOff` | `Defer` |
|---|---|---|---|
| `Cfg::edges` (68k) | `rts`/`rte`/`rtr`/`rtd` | no next instruction | a transfer to a non-local symbol |
| `Cfg::z80_edges` | `ret`/`reti`/`retn`, incl. the taken edge of a `ret cc` | no next instruction, **or** a `jp`/`jr` to a label that CLOSES the body | a transfer to a non-local symbol, or a computed `jp (hl)` |

One builder change beyond the mechanical rename is worth naming. `Cfg::z80_edges`
guarded its conditional-return arm on `mnem == "ret"`; it now guards on
`is_return_mnemonic(mnem, Cpu::Z80)`. That closes the ONE input on which old and
new would otherwise have disagreed: a `reti cc`-shaped item fell through to the
plain arm, produced a single `Abandon`, and the old mnemonic-keyed test in
`cycle_budget` closed it as a return anyway. Post-split without the widening it
would have been a `FallOff` and bailed. **Neither `reti cc` nor `retn cc` exists
in the Z80 ISA, so the input is unreachable** — the widening is not a behaviour
fix, it is what makes the builder's classification total over its own table,
which is the property the whole parcel is for.

`preserves.rs` gained a small `ExitKind { Return, FallOff, Defer }` — the
projection of `Edge` onto "ways of leaving the body" — because its
`StackObserver` seam reports exits to two consumers with DIFFERENT policies, and
that seam previously passed a bare `is_return: bool` that meant "not a Defer".

### Why the wrong answer is now unrepresentable

The claim is narrow and it is about the type, not about care:

1. A consumer holding an `Edge` cannot ask "was this a return?" and get a wrong
   answer, because the question is answered by the variant it is already
   matching on. Every consumer's match is exhaustive — there is no `_ =>`
   catch-all anywhere in the six sites — so a fourth variant would fail to
   compile at each of them rather than being silently swallowed.
2. A consumer cannot supply the wrong CPU, because it supplies none. `Cpu`
   selects the BUILDER (`context::edges_for`, `flag_check::abandons_flag`), and
   the builder's terminator set is fixed by which function it is.
3. The two edges of an end-of-body `ret cc` are structurally distinct values, so
   no rule that maps a mnemonic (or a position) to "returns" can close both.

It does NOT claim: that a consumer chose the right POLICY (that is still a
judgement, stated as an arm and reviewed as one — and §3 names three consumers
that declined to state one), nor that `Edge::Defer`'s own conflation is resolved
(§4).

---

## §2 — Per-consumer table: behaviour preserved, and the arm that proves it

Every arm derived from `git show ea7b1c36:<path>` and re-derived from the working
tree; the enumeration for the two non-trivial ones follows the table.

| # | consumer | old | new | verdict |
|---|---|---|---|---|
| 1 | `flag_check::abandons_flag` | `Abandon => return true` | `Return \| FallOff => return true` | **PRESERVED.** The check never distinguished them: a return hands the flag to a caller that did not ask, a fall-off drops it. Both abandon. |
| 2 | `cycle_budget::charged_edges` | `Abandon if returns && (!two_way \|\| i==0)` closes the path, else bail | `Return` closes, `FallOff \| Defer` bail | **PRESERVED** over every `z80_edges` arm — enumeration below. `returns` and `this_edge_returns` deleted. |
| 3 | `preserves::BalanceObserver` | `is_return_mnemonic(m, M68000) \|\| charge_fall_off_end`, reached only on `Abandon` | `Return => true`, `FallOff => charge_fall_off_end`, `Defer => false` | **PRESERVED** — enumeration below. |
| 4 | `preserves::PreserveObserver` via `ReturnScope::checks` | `AllReturns => is_return` (`Abandon` true, `Defer` false) | `AllReturns => exit.ends_this_body()` (`Return`/`FallOff` true, `Defer` false) | **PRESERVED.** `Sites` was and is index-keyed, indifferent to kind. |
| 5 | `out_verify` exit arm | `Abandon => check_return(..)` | `Return \| FallOff => check_return(..)` | **PRESERVED.** The `Defer` arm — which re-derives `is_uncond_tail` from the mnemonic — is untouched (§4). |
| 6 | `out_verify::not_cc_exit_sites` | `matches!(e, Abandon \| Defer)` | `matches!(e, Return \| FallOff \| Defer)` | **PRESERVED.** "Any edge that ends the proc" is the same set. |
| 7 | `context::check_regions` | `Defer if is_call => {}` then `Abandon \| Defer => Escape` | `Return \| FallOff \| Defer => Escape`; the `is_call` arm **deleted** | **PRESERVED, contingent on §5.** After the `call cc` fix no call mnemonic can produce a `Defer` under either builder, so the excepting arm had nothing left to except. |
| 8 | `z80_preserves` walk | `Abandon => checkpoint` | `Return \| FallOff => checkpoint` | **PRESERVED.** Its `Defer` arm (tail-callee oracle) is untouched. Its private duplicate builder was deleted later, in `1a52e010`; the arm itself is unchanged. |
| 9 | `z80_cycles::span_cost` | `is_return_mnemonic(m, Z80)` | **unchanged** | **UNTOUCHED and correct.** It walks a raw item slice with no CFG, so the mnemonic test is the only thing it can ask. It is the function's only caller OUTSIDE an edge builder; the other two are `Cfg::z80_edges`' own return arms. |

### §2.1 — `cycle_budget::charged_edges`, every `z80_edges` arm

| arm | old edges | old `returns` | old outcome | new edges | new outcome |
|---|---|---|---|---|---|
| `ret`/`reti`/`retn` | `[Abandon]` | true | i=0 closes | `[Return]` | closes |
| `ret cc`, fall-through exists | `[Abandon, Follow]` | true | i=0 closes, i=1 follows | `[Return, Follow]` | same |
| **`ret cc` at end of body** | `[Abandon, Abandon]` | true | i=0 closes; i=1 `(!two_way \|\| i==0)` false ⇒ **bail** | `[Return, FallOff]` | i=0 closes, i=1 bails |
| `jp`/`jr` local | `[Follow]` | false | follows | `[Follow]` | same |
| `jp`/`jr` external | `[Defer]` | false | bail | `[Defer]` | bail |
| `jp cc`/`jr cc` | `[Follow\|Defer, Follow\|Abandon]` | false | `Abandon` ⇒ bail | `[…, Follow\|FallOff]` | bail |
| `djnz` | `[Follow?, Follow\|Abandon]` | false | `Abandon` ⇒ bail | `[…, FallOff]` | bail |
| `call` / `call cc` | — | — | refused as `OpaqueCall` before edges are consulted | — | same |
| anything else | `[Follow]` / `[Abandon]` | false | `Abandon` ⇒ bail | `[Follow]` / `[FallOff]` | bail |

The only way old and new could differ is a mnemonic in the Z80 return table
reaching a NON-return arm, and the builder's two return guards between them
consume every one (§1's widening is what makes that true for `reti cc`).

### §2.2 — `preserves::BalanceObserver`, every `Cfg::edges` arm that can abandon

`check_stack_balance` runs on 68k bodies only (`lower/proc.rs:222`,
`if ctx.cpu != Cpu::Z80`), so `Cfg::edges` is the right builder.

| arm | old edge | is the instruction a return mnemonic? | old `charges` | new edge | new `charges` |
|---|---|---|---|---|---|
| `RETURN_MNEMONICS` | `Abandon` | yes (it is the arm's condition) | `true` | `Return` | `true` |
| conditional branch, no fall-through | `Abandon` | no — a `bXX`/`dbXX` | `charge_fall_off_end` | `FallOff` | `charge_fall_off_end` |
| plain instruction, no fall-through | `Abandon` | no — the return arm returned first | `charge_fall_off_end` | `FallOff` | `charge_fall_off_end` |
| external transfer | `Defer` | — | not reached (old gated on `is_return`) | `Defer` | `false` |

The two old disjuncts of `is_return_mnemonic(..) || charge_fall_off_end` are
exactly the first two rows, and the arms are disjoint because the return-mnemonic
test is the FIRST thing `Cfg::edges` does. Nothing else can produce an `Abandon`
at a return instruction, and nothing that produces an end-of-body `Abandon` is at
a return instruction.

---

## §3 — What happened to `charge_fall_off_end`

**It survives, unchanged, as a genuinely separate axis — and the split is what
makes that legible.**

`check_stack_balance(items, charge_fall_off_end)` is called from
`lower/proc.rs:257` with `charge_fall_off_end = proc.falls_into.is_none()`. It is
a **per-proc DECLARATION**, not a property of the machine: a `falls_into` proc's
end is a continuation into its successor, whose own check covers the shared
frame.

The split subsumes the OTHER half of the old test — the
`is_return_mnemonic(mnemonic, Cpu::M68000)` disjunct, which was asking the item
list a question the edge already answered. What remains is the flag alone:

```rust
fn charges(&self, exit: ExitKind) -> bool {
    match exit {
        ExitKind::Return  => true,
        ExitKind::FallOff => self.charge_fall_off_end,
        ExitKind::Defer   => false,
    }
}
```

Read that as the tell it is: the edge states what the MACHINE did; the flag states
what THIS PROC DECLARED. They are orthogonal and always were — the old code just
had no way to say so, so it OR'd a machine fact against a declaration fact and
the two were indistinguishable in the result.

**A consequence the split makes visible, and which is NOT fixed here.**
`preserves`' `ReturnScope::AllReturns` charges a `FallOff` as an obligated exit
and has NO `charge_fall_off_end` equivalent. So a declared `falls_into` proc's
fall-off end is exempt from `[stack.unbalanced]` and NOT exempt from the
`preserves` entry-value obligation. The polarity is safe — charging more exits can
only make `preserves` say `NotPreserved`, a false POSITIVE — and it is
pre-existing, not introduced. Ledgered; see §9.

---

## §4 — Ruling on `Edge::Defer`: NOT split, and here is the argument

`Defer` covers three situations: an unconditional tail transfer out, a
conditional branch whose target is external, and a computed/unresolved transfer.
The brief asked for a ruling either way. **Declined for this parcel**, on three
grounds, the third of which is the load-bearing one:

1. **`Return`/`FallOff` are two facts about the MACHINE; `Defer`'s flavours are
   one fact about the ANALYSIS.** Whether control returns or runs off the end is
   decidable from the instruction stream. Whether a `Defer` is a real tail call
   or a divergent noreturn rail is not — it depends on the TARGET, which is
   exactly what "local analysis cannot read" means. A builder splitting on it
   would have to guess, which is the failure mode this parcel exists to remove.
   The right home for that distinction is the callee oracle, and both
   `preserves` and `z80_preserves` already consult one there.

2. **The distinction that IS builder-visible is a property of the terminator, not
   of the edge.** `out_verify.rs:326` re-derives `is_uncond_tail(mnem)` at the
   consumer to tell an unconditional tail transfer from a conditional branch's
   external taken edge — and yes, that is the identical defect class this parcel
   is about, in the same enum. It is a real finding and it is ledgered (§9). But
   see (3).

3. **`Defer` is currently the arm where a MISCLASSIFICATION lands, and both of
   this parcel's findings landed there.** A `call cc, External` produced a
   `Defer` it had no business producing (§5.1); a `jr .end` closing a body
   produced a `Defer` where the target is not external at all (§5.2). Splitting
   the variant while its population was still wrong would have minted a
   distinction over a set that had not settled. It has settled now — which is
   precisely why the successor parcel is worth doing and was not worth doing
   here.

**The successor parcel is named:** split `Defer` into `TailOut` (an
unconditional transfer out — a required return path from the caller's view) and
`BranchOut` (a conditional branch's external taken edge — not a local
counterexample), and delete `out_verify::is_uncond_tail` with it. The
prerequisite it was waiting on — one Z80 builder — landed in `1a52e010`.

---

## §5 — TWO FINDINGS, both ERROR-tier, both silent

Each is reported separately from the refactor, with its own commit and its own
probe, because each CHANGES behaviour. Both are corpus-neutral, and in both cases
that is a fact about what the corpus happens to contain, not about the compiler.

### §5.1 — B′-2's `bsr` bug was still live in the Z80 builder (`fba2146e`)

B′-2 took `bsr` out of the 68k conditional-branch arm: it is spelled like a
branch (leading `b`, three letters) and is not one, and giving it a taken edge
splices the callee's body into the caller's flow at the caller's stack state —
which charged a local helper's `rts` the caller's depth. The Z80 builder still
listed `call cc` beside `jp cc`/`jr cc`:

```rust
// flag_check.rs, before
if matches!(mnem, "jp" | "jr" | "call") && leads_cc { … taken edge … }
```

so `call nz, .helper` emitted `Follow(.helper)` — the identical splice — and
`call nz, External` emitted a `Defer` claiming the flag and register file left the
proc, which they do not.

**Exposure, measured.** All five corpus conditional-call sites target EXTERNAL
symbols (`sound_fm.emp:1067`, `sound_sequencer.emp:145,493,532,538`), so the
splice arm was never taken; and every consumer already discarded the spurious
`Defer` for its own reason — the context walk by an explicit `is_call` guard, the
flag walk by pruning `Defer`, the budget walk by refusing calls outright. So this
is corpus-neutral for exactly the reason B′-2's was: the bad arm exists and
nothing in the corpus walks it. **A single `call cc, .local_label` in a Z80 module
would have made it live**, in the same false direction as B′-2's.

Fixed: `call` leaves the conditional-branch arm and joins the plain arm — a call's
only successor is its fall-through, conditional or not.

**Second-order.** With no call able to produce a transfer-out edge under either
builder, `context.rs`'s `Edge::Defer if is_call => {}` arm has nothing left to
except, and the `is_call` binding with it. Both deleted. That is the outcome the
brief predicted a correct split would produce — a consumer's compensating arm
becoming redundant — and it is verified by probe, not by inspection: the probe
pins that `call cc` yields only its fall-through at end-of-body, mid-body, and to
a local target, and a second probe extends the invariant to `jsr`, `jbsr` and
`rst` (Lens B asked; they were unpinned).

### §5.2 — A Z80 jump to a label that CLOSES the proc read as a transfer OUT (`1a52e010`)

**Found by Lens C.** `Cfg::build` maps a label to the first instruction at or
after it, so a label that CLOSES a body has NO mapping — and by that map alone it
is indistinguishable from an external symbol. `Cfg::z80_edges` consulted only
that map:

```rust
// before
match branch_target(ops).and_then(|t| self.label_target.get(t)) {
    Some(&tgt) => vec![Edge::Follow(tgt)],
    None       => vec![Edge::Defer],       // <- `.done:` closing the proc lands here
}
```

so a `jr .done` whose `.done:` ends the proc became an `Edge::Defer`: a transfer
out to a callee that does not exist. Every obligation the path carried was then
discharged against nothing.

**Exposure.** `abandons_flag` PRUNES a `Defer` (the flag flows out of the proc, so
local analysis cannot judge it) and FIRES on a `FallOff`. So
`[call.flag-result-unused]` — ERROR tier, live for Z80 via
`corpus_contracts.rs:428` — went silent on a path that genuinely abandons the
flag. Lens C's exhibit:

```
0: call  FlagCallee        ; declares out(carry: ok)
1: jr    .done
2: .done:                  ; closes the proc — no instruction after it
```

`cycle_budget` and `context::check_regions` are NOT exposed (both treat `Defer`
and `FallOff` identically, bailing and firing `Escape` respectively).

**The sharpest artifact.** `Cfg::is_local_label` exists for exactly this, and its
own doc comment says so — *"a `jr cc, .end` whose `.end:` closes the proc must not
be read as an external tail transfer"* — and `Cfg::z80_edges`, ninety lines below,
never called it. Its only two callers were in `z80_preserves`, which got the
answer right.

Fixed by routing both `jp`/`jr` arms through one `z80_branch_edge` classifier.

**Second-order, and it is the bigger half.** That was the LAST input on which
`Cfg::z80_edges` and `z80_preserves`' private `z80_edges` disagreed — Lens C
diffed every arm and found the other six already identical, down to a
character-identical `branch_sym`/`branch_target` and two copies of the Z80 return
table. So the private builder is **deleted** (79 lines) and the Z80 `preserves`
proof calls `cfg.z80_edges`. There is one Z80 edge model now, which is the
strongest available form of "the builder decides once": the pair that could drift
no longer exists.

---

## §6 — The three regression probes

All four new tests live in `mod edge_model_tests` at the end of `flag_check.rs`
(the types are `pub(crate)`, so an integration test cannot reach them). Each is
stated over the EDGES, not over a consumer's output — the point is that the wrong
answer stopped being expressible, not that one more consumer was taught to
compensate.

| probe | pins | fails against the shape it names? |
|---|---|---|
| `a_tail_conditional_return_names_its_two_ends_apart` | B′-3a: a tail `ret cc`'s two edges are `[Return, FallOff]`, asserted as `assert_ne!(edges[0], edges[1])` | **YES — demonstrated against pre-split** |
| `a_bsr_calls_it_does_not_branch` | B′-2: `bsr .helper` yields only `Follow(next)` | YES against pre-B′-2 (that fix shipped; this pins it, and Lens C confirmed deleting the `mnem != "bsr"` guard breaks it) |
| `a_conditional_z80_call_calls_it_does_not_branch` | §5.1: `call cc` to local / to external / at end-of-body | **YES — it FAILED at HEAD before `fba2146e`** |
| `the_builder_owns_the_return_table_not_its_consumers` | B′-3a: `ret` is a `Return` to the Z80 builder and a `FallOff` to the 68k one | **YES — demonstrated against pre-split** |
| `a_jump_to_a_closing_label_falls_off_it_does_not_transfer_out` | §5.2: a `jr .done` closing a body, its `jr cc` twin, and the external contrast | **YES — it FAILED before `1a52e010`** |

Three more were added on the panel's asks (§8): `no_call_mnemonic_yields_a_transfer_out_edge`
(`jsr`/`jbsr`/`bsr`/`call`/`rst`), `the_z80_return_table_is_classified_whole`
(every return mnemonic, bare and guarded), and
`a_fall_off_end_is_charged_only_when_the_proc_declares_no_fallthrough` (the
`charge_fall_off_end` policy the edge deliberately does NOT carry — §3).

**One honest limit, named by Lens C.** `a_tail_conditional_return_names_its_two_ends_apart`
pins the MODEL, not the consumer that shipped B′-3a's bug: re-introducing a
mnemonic-keyed return test inside `charged_edges` would close both edges again and
this probe would still pass. The consumer half is covered by the pre-existing
`cycle_budget::a_tail_conditional_return_refuses_its_fall_through`, which asserts
`UnboundedTransfer` plus the `(15, 19)` measuring twin. The pair is adequate; the
probe alone is not, and it should not be read as if it were.

### Demonstration, own-run

The `assert_ne!` forms are expressible against the pre-split shape (they need
only `PartialEq` on `Edge`), so this is a real run, not a compile failure. Method:
`git checkout ea7b1c36 -- crates/sigil-frontend-emp/src/`, add `PartialEq`/`Debug`
to the single-variant `Edge`, paste the two probes, run:

```
test flag_check::presplit_demo::probe3_the_builder_owns_the_return_table_not_its_consumers ... FAILED
test flag_check::presplit_demo::probe1_a_tail_conditional_return_names_its_two_ends_apart ... FAILED
  left: [Abandon]      right: [Abandon]      (probe 3)
  left: Abandon        right: Abandon        (probe 1)
test result: FAILED. 0 passed; 2 failed
```

`left: Abandon / right: Abandon` IS the defect, printed. Tree restored to
`edge-split` immediately after; `git status` clean.

The `call cc` probe's failure at HEAD (before `fba2146e`) was:

```
left: [Follow(3), Follow(1)]
assertion failed: `call cc` calls conditionally; the callee is not a successor of the caller
```

`Follow(3)` is the local helper being spliced in.

---

## §7 — Bars

### §7.1 — Byte bar: SEVEN targets, `cmp`, capture order

Derived from `crates/sigil-harness/golden/` in this worktree, built in
`capture_goldens.sh` order (`config_a` → `s4.debug.bin`; `config_b` AND `lean` →
`s4.bin`; canonical rebuilt after). `AEON_DIR` = `aeon/.worktrees/b4` at
`77d5317`.

**BASELINE (before any edit)** and **POST-CHANGE** are identical:

```
OK   s4.bin          OK   demo.bin          OK   config_a.bin      OK   lean.bin
OK   s4.debug.bin    OK   demo.debug.bin    OK   config_b.bin
>> restoring canonical s4.bin + s4.debug.bin
OK   s4.bin          OK   s4.debug.bin
SEVEN-TARGET: ALL OK
```

Byte-neutral ×7. No target moved, no refreeze.

### §7.2 — `refreeze --check`

`refreeze --check: OK (tip 'b-jumps', chain len 44)` — at the branch point and at
final HEAD. No chain bump, no 5-site ripple (nothing under `pins.rs` / `engine.inc` /
`mixed_dac_rom.rs` / `repin_pins.rs` / `repin.toml` was touched).

### §7.3 — Strict suite

`SIGIL_STRICT_GATE=1 AEON_DIR=<b4 worktree> cargo test --workspace --release
--no-fail-fast`, full capture to file, failures-first, never piped through
`tail`/`head`.

| | branch point `ea7b1c36` | `edge-split` |
|---|---|---|
| **passed** | 3235 | **3243** |
| **failed** | 0 | **0** |
| **ignored** | 4 | **4** |
| result lines | 308 | 308 |

`3243 + 4 = 3247`, EXACTLY the branch's own `#[test]` total (§7.4) — nothing was
silently skipped. The baseline arithmetic checks the same way: `3235 + 4 = 3239`,
`ea7b1c36`'s own total. The `+8` passed is the eight new tests and nothing else.

The four ignored are the standing set, unchanged from the branch point:
`chained_resume_debug`, `chained_resume_plain`, `sigil_diff_reports_byte_identity`,
`secondary_pin_classes_match_the_hand_typed_baseline`.

Failures-first grep for `^test .* FAILED|^failures:|^error|panicked` over the full
log returns **nothing**.

The baseline run was taken at the branch point BEFORE any edit, per step zero,
along with the seven-target byte proof and `refreeze --check`.

### §7.4 — Test-delta arithmetic, every function named

`git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'`, per-file diffed
against the branch point `ea7b1c36`.

| file | ea7b1c36 | edge-split | Δ |
|---|---|---|---|
| `crates/sigil-frontend-emp/src/flag_check.rs` | 0 | 7 | **+7** |
| `crates/sigil-frontend-emp/src/preserves.rs` | 10 | 11 | **+1** |
| every other file | — | — | 0 |
| **total** | **3239** | **3247** | **+8** |

The eight, all named. In `flag_check::edge_model_tests` (a new `#[cfg(test)]`
module — the file had none before):

1. `a_tail_conditional_return_names_its_two_ends_apart`
2. `a_bsr_calls_it_does_not_branch`
3. `a_conditional_z80_call_calls_it_does_not_branch`
4. `a_jump_to_a_closing_label_falls_off_it_does_not_transfer_out`
5. `no_call_mnemonic_yields_a_transfer_out_edge`
6. `the_z80_return_table_is_classified_whole`
7. `the_builder_owns_the_return_table_not_its_consumers`

In `preserves::frame_tests` (an existing module):

8. `a_fall_off_end_is_charged_only_when_the_proc_declares_no_fallthrough`

No test was renamed, moved or deleted; the only per-file counts that changed are
those two, and both moved up.

### §7.5 — Diagnostic census: ZERO change, all seven shapes

The expected outcome of a refactor is byte-neutrality AND diagnostic-neutrality,
and the second is the one a byte bar cannot see. Method: build the same sigil
binary at `ea7b1c36` and at HEAD, run
`SIGIL_WARNINGS=full sigil build --aeon <b4> --native <shape> --report contracts`
for all seven shapes, `diff` the full output line-for-line.

```
IDENTICAL  plain      IDENTICAL  demo       IDENTICAL  cfga     IDENTICAL  lean
IDENTICAL  debug      IDENTICAL  demodbg    IDENTICAL  cfgb
```

Re-run at final HEAD, after both findings landed. That covers every counted
family the report renders: dropped instructions,
extern/proc collisions, unresolved callees, `[proc.clobber-undeclared]`,
flag-result firings, `[call.input-undefined]`, `[call.live-clobbered]` (21),
`[proc.out-cond-survives-unverifiable]`, dead-saves (3),
`[call.slot-type-mismatch]` (6), `[branch.condition-constant]`, `[bus.*]`, and
the whole `[context.*]` block (23 regions / 10 claims / 12 discharged sites / 0
bracket firings) — plus the 12 `module.path-mismatch` warnings, byte-identical
text and all.

No diagnostic appeared and none disappeared, so there is nothing to explain in
either direction.

---

## §8 — Lens panel

Three fresh read-only subagents over `git diff master...edge-split` — A
(ceremony/style/house rules), B (per-consumer behaviour preservation, briefed to
FALSIFY the preservation claim), C (soundness/hazard/residual ambiguity). None
reviewed its own work; the panel ran on the two-commit state and its dispositions
are commits `8513ecbc` and `1a52e010`.

**Verdict: no BLOCKER. One live ERROR-tier soundness defect found (C-1), fixed
with its own commit and probe. Two factual errors in this branch's own claims
caught and corrected. Three probes and one policy pin added.**

### Lens B — the falsification attempt

Briefed to break the "behaviour preserved" claim by enumerating every edge-list
shape each builder can produce. It derived the mapping independently and reported
it total and non-lossy: master's `Cfg::edges` had 3 `Abandon` sites → 1 `Return` +
2 `FallOff`; `Cfg::z80_edges` 6 → 2 + 4; `z80_preserves::z80_edges` 6 → 2 + 4.
Nothing became `Follow` or `Defer`, so `Abandon ≡ Return ∪ FallOff` exactly.

**All seven consumers PRESERVED**, matching §2 arm for arm. It independently
re-derived the `reti cc` case (§1) and the `context.rs` arm deletion (§5.1),
checking each of `jsr`/`jbsr`/`bsr`/`call`/`rst` against both builders rather than
taking the claim. It also verified the corpus's five conditional-call sites are
all external, all mid-body, none inside a `with` bracket — and noted the one
behaviour of commit `fba2146e` I had not stated: master would have fired
`[context.escape]` for a `call cc, .LocalLabelOutsideRegion` via the
`Follow(tgt) if !region.contains(succ)` arm, and that firing is gone. No corpus
site has that shape.

### Lens C — the soundness hunt

| # | finding | disposition |
|---|---|---|
| C-1 | **`Cfg::z80_edges` gives a `jr .end` closing a body an `Edge::Defer`, silencing `[call.flag-result-unused]` (ERROR tier).** The two Z80 builders' ONLY divergence, with `is_local_label`'s own doc naming the case and `z80_edges` never calling it. | **FIXED** — `1a52e010`, §5.2, probe `a_jump_to_a_closing_label_falls_off_it_does_not_transfer_out`, corpus census re-proven identical. Duplicate builder deleted as a consequence. |
| C-2 | `z80_preserves`' "one edge model" comment false — there were two. | FIXED; it is true now. |
| C-3 | Third copy of the Z80 return table in `z80_preserves::is_return`. | FIXED by the deletion. |
| C-4 | `out_verify::is_uncond_tail` re-derives from the mnemonic what a `Defer` split would state. | **LEDGERED**, and it is the successor parcel (§4). |
| C-5 | `invalid_edge`/`valid_edge` are wholesale 68k with no fence; `check_flag_unused` is already Z80-wired. | **LEDGERED.** Not the fourth recurrence today — it becomes one the moment `[call.result-invalid-path]` is wired for Z80. |
| C-6 | `z80_bus` holds a `Cfg` and consults its own private 68k return table; `Defer`/`FallOff` exits never reach `[bus.stopped-at-return]`. | **LEDGERED.** False negative. |
| C-7 | `ExitKind` faithful; `ends_this_body()` correctly has one call site, since `BalanceObserver` needs the three-arm form. | Confirmed, no action. |
| C-8 | **Three consumers collapse `Return`/`FallOff` with no `falls_into` policy** — `out_verify` (ERROR-tier false positive), `AllReturns`, `z80_preserves`. Pre-existing; the split is the moment each GAINED the variant to state a policy and three declined. | **LEDGERED**, and stated in §3. |
| C-9 | `postorder`'s `_ => None` catch-all would silently drop a future in-proc variant — a lost path makes a budget too LOW, the exact polarity B′-3a fixed. | FIXED, exhaustive. `not_cc_exit_sites` likewise inverted to ask whether the edge is a `Follow`. |
| C-10 | Probe 1 pins the model, not the consumer; the pre-existing `cycle_budget` twin covers the consumer half. Probe 3 weaker than its name. Missing: a differential test between the two builders. | Accepted and stated in §6. The differential test is MOOT — after C-1 there is one builder, which is the stronger form of the same guarantee. |

### Lens A — ceremony, style, house rules

| # | finding | disposition |
|---|---|---|
| A-1 | **`is_return_mnemonic` has THREE call sites, not one.** `56a0f1c6`'s message and my §0 both said "exactly one caller". | FIXED in the code doc; the correction is stated in §0 rather than hidden. |
| A-2 | The test module doc said "three regression probes" for four tests, and narrated parcel history. | FIXED. |
| A-3 | `PROBE n (B′-x)` tags and past-tense bug narration in five doc comments, referencing a variant (`Abandon`) that no longer exists. **House rule: comments state present-tense contract facts.** | FIXED — all five rewritten as the hazard in the abstract. |
| A-4 | Two `z80_edges` headers claimed every conditional form contributes both edges — false for `call cc` as of `fba2146e`. | FIXED in both. |
| A-5 | `charged_edges`' new doc said the returning side is "charged from the same list" — the `FallOff` arm `return bail(..)`, discarding `out`; the whole bound is refused. | FIXED. |
| A-6 | "made HERE and nowhere else" understated the builder count. | FIXED (and then made true by `1a52e010`). |
| A-7 | `z80_preserves`' `saw_return`/`bailed_reached_return` are set at a transfer out too; `preserves` already uses `saw_exit`/`bailed_reached_exit`. | FIXED, renamed for parity. |
| A-8 | Coverage gap: nothing pinned `BalanceObserver` routing `FallOff` through `charge_fall_off_end` — the one policy the split re-expressed. | FIXED — `a_fall_off_end_is_charged_only_when_the_proc_declares_no_fallthrough`. |
| A-9 | `ExitKind` derived `PartialEq`/`Eq`/`Debug` it does not use. | FIXED, trimmed to `Clone, Copy`. |
| A-10 | Residual history narration on `is_call_site` ("A NEW Z80 arm; … unchanged"), predating this branch. | FIXED while in the file. |
| A-11 | `is_call_site` and `is_call_mnemonic` are two classifiers for one question — the duplication this parcel's thesis argues against. | Noted; folded into the ISA-classifier ledger row (§9 item 5). |

**Two panel asks declined, with reasons.** Splitting the tail-`call cc` assertion
out of `a_conditional_z80_call_calls_it_does_not_branch` (A-20): it asserts the
same property — a call is not a transfer — for a third position, and the three
belong together. Trimming `Edge`'s `Copy` (A-9 half): the tests need `PartialEq`
and `Debug`, and `Copy` on a four-variant fieldless enum costs nothing and keeps
the `VecDeque<Edge>` consumer simple.

---

## §9 — Ledger and kill-list

**Closed by this parcel** (both were `— OPEN (the variant split)`):

- `[bprime-2 lenses B+C, 2026-08-04]` — "`Cfg::edges` uses ONE `Edge::Abandon`
  for a real return AND for control running off the end of a body". → CLOSED.
- `[bprime-3a lenses A+B+C, 2026-08-04]` — "`Edge::Abandon` conflating a RETURN
  with a fall-off-end cost this parcel a soundness blocker". → CLOSED.

**Opened by this parcel** — see the ledger for the full text:

1. **Three consumers collapse `Return` and `FallOff` with no `falls_into`
   policy** — `out_verify` (an ERROR-tier false positive on a `falls_into` proc),
   `ReturnScope::AllReturns`, and `z80_preserves`. Only `BalanceObserver` is
   parameterized. `charge_fall_off_end` wants to be a field of the WALK. (§3)
2. `out_verify::is_uncond_tail` re-derives from the mnemonic what a `Defer` split
   would state — the same defect class, in the same enum. (§4)
3. `z80_bus::check_bus_state` holds a `Cfg` and consults its own private 68k
   return table instead, so `Defer` and `FallOff` exits never reach
   `[bus.stopped-at-return]`. False negative.
4. `flag_check::{invalid_edge, valid_edge}` are wholesale 68k with no fence, and
   `check_flag_unused` is ALREADY wired for Z80. Not live — but it becomes
   B′-3a's defect silently the moment `[call.result-invalid-path]` is.
5. The mnemonic-SHAPE heuristic has now been excepted by name twice (`bsr`,
   `call`). It wants to be a per-CPU classifier in the ISA crate.
6. `CodeItem::Inline` remains invisible to `Cfg` — restated, since this parcel
   enumerated every builder arm and confirmed it.

**Kill-list:** this parcel creates no twin and no scaffolding, so it adds no row.
It closes none either.

---

## §10 — Step-3 (port retrospect) vs step-5 (engine optimize)

**Step-3 — what the LANGUAGE / the substrate should have made impossible:**

- The whole parcel is one step-3 finding cashed. A sum type whose variants do not
  distinguish the cases consumers must distinguish is not a modelling shortcut,
  it is a standing invitation for each consumer to invent its own answer — and
  five did, and three were wrong. The general form: **when a consumer re-reads
  the input a producer already parsed, the producer's output type is missing a
  case.** `charge_fall_off_end` is the counter-example that proves the rule has
  an edge — that flag is NOT a missing case, it is a genuinely separate axis, and
  the way to tell is that it comes from a DECLARATION and not from the
  instruction stream.
- **A distinction the type does not carry is only half the failure; the other
  half is a consumer that declines to state a policy it now can.** Three
  consumers (§3, ledger row 1) collapse `Return` and `FallOff` into one arm with
  no `falls_into` parameter, and one of them is an ERROR-tier false positive on a
  declared `falls_into` proc. The split gave all three the vocabulary; only one
  used it. The structural answer is that the declaration belongs to the WALK, not
  to one observer — a consumer should not be able to forget to ask.
- The `bsr` / `call cc` pair says the mnemonic-shape heuristic
  (`starts_with('b') && len() == 3`) is load-bearing in a place that cannot carry
  it. Both CPUs now have an explicit exception carved out of a spelling test. The
  structural answer is an ISA-level classifier (`is_call` / `is_return` /
  `is_branch` per CPU, in `sigil-ir::backend`) that the frontend consults instead
  of reading letters; neither builder should be pattern-matching on spellings,
  and `is_call_site` / `is_call_mnemonic` — two functions for one question —
  would collapse into it.
- **A doc comment that names a case the code does not handle is a bug report
  nobody filed.** `Cfg::is_local_label`'s doc described the closing-label case
  exactly, in the same file, ninety lines above the builder that ignored it
  (§5.2). Prose and code disagreed for as long as both existed, and only an
  arm-by-arm diff of two builders surfaced it.
- Three consecutive parcels found the root ambiguity by lens panel and each fixed
  the SYMPTOM within its own byte-freeze. That is the right local call every
  time, and it compounded into a defect that recurred in the opposite polarity
  one parcel later. The ledger row that said "expect it to recur" was correct
  twice, and the second recurrence cost an ERROR-tier bound that was too low.

**Step-5 — engine optimization opportunities:** none. This parcel emits no
bytes, and the corpus census is line-for-line identical, so there is nothing the
engine gained or lost. The `.emp` corpus was not touched (lane discipline: aeon
belongs to the other porter this lane).

**Neither bucket, and the headline of the three:**

- **Both findings are real soundness fixes that only an enumeration could have
  produced.** §5.1 (`call cc`) came from walking every arm of the builder I was
  editing; §5.2 (the closing label) came from a lens diffing two builders against
  each other. Neither is visible from the corpus, and both are corpus-dead by
  accident of what the corpus happens to contain. **"Corpus-neutral because
  nothing currently walks the bad arm" is a statement about the corpus, not about
  the compiler** — and it was the exact wording B′-2 used about its own `bsr`
  fix, whose Z80 twin was still live a parcel later.
- **A substrate with N consumers is only tested on the paths those N take.** This
  is now the fifth recorded instance (B′-2 §11 raised the general form; B′-3a's
  missing `ret` T-state was the third). The `context.rs` arm deleted in §5.1 is
  the inverse of the same coin: a consumer had built a compensator for a builder
  bug, and the compensator then kept the bug invisible to every other consumer.
- **The duplicate builder was the parcel's real prize and it was not in the
  brief.** Two hand-maintained Z80 edge builders, character-identical on six arms
  and silently divergent on the seventh — with the divergent one wrong in the
  ERROR-tier direction. It is gone. That, rather than the variant count, is what
  makes the Z80 half of "the builder decides once" true rather than aspirational.
