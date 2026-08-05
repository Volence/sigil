# 2026-08-05 — B′-4: the `--report` surface + the cc-precision hole

Branch pair: sigil `bprime-4` (from master `50382ddc`) / aeon `bprime-4` (from
master `77d5317`, **untouched — no aeon change was needed**). Merge state lives
in the campaign log, not here.

Two tasks, one parcel: (1) delta spec §5's report consolidation, (2) handoff §5's
cc-precision hole. They landed together because the report surface is what
MEASURED the corpus reality the cc fix is argued against.

---

## §0 — Headline

The parcel is byte-neutral ×7 and changes **zero corpus diagnostics**, exactly as
predicted. What it is actually worth is the two things the report surface found on
its first run:

1. **Six real `[call.slot-type-mismatch]` firings ship un-gated**, and
2. **Five real transitive clobber under-declarations ship in the DEBUG shape**,

both because every corpus gate walks a **define-free** corpus and the shipping
shapes are not define-free. Neither is fixed here (aeon adoption is HELD); both are
ledgered with their land-together constraint and a kill-list row.

The lens panel then added a third, in code rather than corpus: **`flag_check::invalid_edge`
never canonicalized the condition code it compares**, so `[call.result-invalid-path]`
silently never fired for the `hs`/`lo` spellings of a guard it fires for as `cc`/`cs`.
Fixed, by the helper this parcel introduced for the other half of the same class (§4.2).
It is also where the two lenses DISAGREED, and the disagreement was adjudicated own-run.

Byte-neutral ×7 · `refreeze --check` OK at chain 44 · strict **3183 / 0 / 4 = 3187**,
the branch's exact `#[test]` total · no aeon file touched.

---

## §1 — TASK 1: the report surface

### §1.1 — What was actually there

Verified own-run at the branch point: `crates/sigil-cli/src/bin/emp_contracts.rs`
was 211 lines, no `--report` flag existed anywhere, and **`emp_contracts` had zero
code consumers** — no test, no script, no `build.sh` reference. Every recorded
invocation in the notes is a human running it over `find <aeon> -name '*.emp'`.

T1's RAM report had already shipped, as **`sigil build --ram-report`** — a boolean
flag on the build subcommand, dispatching before `run_build_native`. Its consumers
were 3 lines in `crates/sigil-cli/tests/warn_tier_corpus.rs` and nothing else.

### §1.2 — The design: `--report <kind>`, one flag, closed vocabulary

```
sigil build --aeon <dir> [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean]
            [--report ram|contracts]
```

`ReportKind` is a two-variant enum with a parser that makes an unrecognised value a
usage error (exit 2), never a silent fallback. `BuildOpts.report: Option<ReportKind>`;
`None` builds.

`--ram-report` is **removed**, not aliased. The argument:

- One flag with a closed vocabulary is the surface that stays legible as the set of
  views grows — and the spec already names two more (derived-contract-as-annotation,
  the grant/bracket census). `--ram-report` / `--contracts-report` / `--annotations-report`
  / `--contexts-report` is an ad-hoc namespace.
- An alias would be a mirror with no kill condition — the class the kill list exists
  to prevent — for a flag with no external consumer.
- The cost of removal is 3 lines in one sigil test. There is no `build.sh` consumer,
  no aeon consumer, no documented user.

**FLAGGED FOR THE GATE:** the plan's *whichever-lands-second-conforms* rule, read
literally, says B′-4 conforms to T1 — i.e. `--contracts-report`. I read the rule's
PURPOSE (one report idiom, not two) and unified instead, which changes T1's shipped
spelling. The spec's own work order says `--report contracts`. If the overseer
prefers strict conformance, restoring `--ram-report` is one match arm.

### §1.3 — What consolidation actually buys: the inputs, not the packaging

`emp_contracts` took a hand-supplied file list and a hand-supplied `-D` list.
`--report contracts` takes neither: the module set comes from `Manifest::scan`, and
the comptime defines come from the **selected target's shipping `GameProfile`**, via
a new `BuildTarget::label_and_profile` that both reports now share.

That is not tidying. Measured, same tree, sonic4 plain:

| | `emp_contracts` (no `-D`) | `--report contracts` (profile) |
|---|---|---|
| dropped instructions | **1** (`DrawRings`) | **0** |
| `[call.slot-type-mismatch]` (G5) | **0** | **6** |
| `[context.*]` regions | **17** | **23** |
| everything else | identical | identical |

- The `DrawRings` drop was an artifact: without `MAX_RING_BUFFER` its
  `vram_art(VRAM_RING_PLACEHOLDER, …)` operand cannot lower, so the instruction fell
  out of the analysis buffer.
- The 6 slot firings are real (§1.5).
- The 6 extra context regions are `Section_RedrawPlanes` ×2, `VInt_Lag` ×2,
  `VInt_Level` ×2 — **exactly handoff §6's recorded honest gap** ("the corpus walk's
  no-`-D` shape does not cover the three widest brackets"). The report surface closes
  it structurally rather than by hand.

Per-shape census, all seven targets (own-run):

| shape | drops | firings | D1c | dead-save | slot | ctx regions | survives |
|---|---|---|---|---|---|---|---|
| sonic4 plain | 0 | 0 | 21 | 3 | **6** | 23 | 0 |
| sonic4 debug | 0 | **5** | 26 | 3 | **6** | 23 | 0 |
| demo plain | 0 | 0 | 21 | 3 | 0 | 20 | 0 |
| demo debug | 0 | **5** | 26 | 3 | 0 | 20 | 0 |
| config_a | 0 | **5** | 26 | 3 | **6** | 23 | 0 |
| config_b | 0 | 0 | 21 | 3 | 0 | 20 | 0 |
| lean | 0 | 0 | 21 | 3 | **6** | 23 | 0 |

### §1.4 — Retire or keep `emp_contracts`: RETIRE

`git rm`'d, its rendering moved verbatim into `print_contract_report` beside
`print_ram_report` (data in the frontend, render in the CLI — T1's precedent, and the
reason `RamRegionRow` is documented as render-free).

The argument to retire:

- Zero code consumers; a hand-invoked driver.
- It duplicated file discovery and `-D` parsing, and did both worse than the
  manifest + profile path.
- Its `-D` handling is the direct cause of a live recorded honest gap and of the two
  blind spots in §1.5.
- Its one capability the new surface lacks — an ARBITRARY file list, including files
  outside an aeon tree — has no in-tree consumer and no recorded demand. If demand
  appears, `--report contracts` gains a file-list mode, demand-gated.

One behavioural tightening worth naming: `emp_contracts` printed
`warning: N parse error(s) — analyzing anyway` and continued. `--report contracts`
renders parse errors and exits 1, as `--report ram` does. A census over a corpus that
does not parse reports numbers nobody can act on.

**Kill-list:** the retirement itself creates and closes no mirror, so it gets no row.
But **row 103 is added**, and it is the real one — see §1.5.

### §1.5 — What the report found (NOT fixed here — aeon adoption is HELD)

**(a) Six `[call.slot-type-mismatch]` firings, invisible to the G5 gate.**

```
PState_Ground    ×2 |
PState_Spindash  ×2 |  calls Sound_PlaySFX  slot d0 expects SfxId but found an untyped value
Player_Animate      |
Player_Jump         |
```

Every site is `move.b #SFXID_*, d0` with no `as SfxId`, inside an
`if SOUND_DRIVER_ENABLED == 1 { }` block. `slot_type_corpus.rs` calls
`analyze_corpus` with **no defines**, so the block comptime-vanishes and
`retrofitted_corpus_has_zero_slot_mismatches` passes on code it never saw. The
engine's own sites (`sound_api.emp:384/387`, `animate.emp:228`, `game_debug.emp:110`)
all bless correctly — this is game-side drift, not a design hole. Fix = 6 `as SfxId`
blessings (byte-neutral, type-layer) **in aeon** + route the gate through the profile.

**(b) Five transitive clobber under-declarations in the DEBUG shape.**

```
Collected_ParkSlot                 direct     d2
EntityWindow_TrySpawnRing          direct     d5
EntityWindow_RescanRings           transitive d5
EntityWindow_ScanRingsRight        transitive d5
EntityWindow_PopulateSectionRings  transitive d5
```

Plain fires zero. `corpus_closure_residue_is_empty_the_error_gate` runs under
`GAME_CONFIG_DEFINES` only — **no `DEBUG`** — so the ERROR gate built to stop exactly
this class has never seen the debug arms. D1c independently agrees the values are
LIVE (`Collected_UpdateCenter holds d2 across clobber`,
`EntityWindow_{RescanY,Scan} holds d5`). Both sites save and restore by hand
(`entity_window.emp:368/380`, `:859/861`) without declaring the `preserves`, so the
closure charges the writes correctly and the CONTRACTS are what is wrong. Runtime
impact is nil today — ParkSlot's caller reads d2 only through `.w` ops, so its
half-width `move.w` restore happens to cover the use — but that is a coincidence
nothing checks, and `entity_window.emp:493`'s site comment ("preserves a0/d2-d5")
is false in the debug shape.

Both are ledgered; both are blocked on the same cross-repo constraint: **the corpus
fix and the gate flip must land together**, or the gates go red. That sequencing is
the overseer's call, which is why this parcel reports rather than acts.

### §1.6 — The gate for the new surface

`the_contracts_report_is_wired_and_carries_the_targets_defines`
(`crates/sigil-cli/tests/contract_closure_corpus.rs`) drives the real binary and
reads its real stdout. Every other gate in that file calls `analyze_corpus_with`
directly, so all of them stay green if `run_contract_report` is deleted.

Assertions: the three headers, `DEBUG=0`/`DEBUG=1` on the build-shape axis,
`MAX_RING_BUFFER=128`/`16` on the game→engine axis, and — the load-bearing one —
zero dropped instructions in all three shapes.

**ANTI-VACUITY PROVEN (mutant built and run):** changing the walk to
`analyze_corpus_with(&files, &[])` makes the test FAIL with
`sonic4 plain: the report dropped instructions, so its walk is missing defines:
-- dropped instructions (must be 0): 1 --`. Reverted; the test passes at HEAD.

---

## §2 — TASK 2: the cc-precision hole

### §2.1 — The hole, confirmed own-run

`closure::Contract::out_cond` was a bare `BTreeSet<String>`. The §4 relation read

```
bound.out_cond ⊆ target.out ∪ target.out_cond
```

— pure membership. A target declaring `out(a1 if ne)` therefore satisfied a bound
promising `out(a1 if eq)`, so a caller that tests `eq` and reads a1 reads a register
the target fills only on the opposite edge.

### §2.2 — What `Contract::out_cond` became

```rust
pub out_cond: BTreeMap<String, BTreeSet<String>>,   // register -> the ccs guarding it
```

Built by a new `ProcSig::cond_out_guards` (`ast.rs::cond_out_guards_of`), which
expands each `CondResult` through the SAME register-file expander `cond_out_regs_of`
uses. **The key set is therefore bit-for-bit the register set the field carried
before** — the widening adds information and removes none.

Deliberately NOT built from the pre-existing `ProcDecl::cond_out_pairs`, despite it
being the obvious `(reg, cc)` primitive: `cond_out_pairs` DROPS a register that also
carries an unconditional mention (correct for the gates that RELAX on
conditionality), while `contract_of_sig`'s `out` comes from `unconditional_outs`,
which SUBTRACTS every guarded register. Using pairs would have left a mixed-mention
register in neither half — a promise no relation term checks. The two accessors'
doc comments now name the distinction at both ends.

**Condition codes are canonicalized.** `ast::canonical_cc` folds `hs`→`cc` and
`lo`→`cs` — the documented 68000 aliases, the same raw-vs-canonical class
`canonical_contract_reg` exists for on the register side. Without it a target
guarding `hs` would be rejected against a bound spelling the same guard `cc`.

### §2.3 — The relation, and why equality

For each `(rN, ccs)` the bound promises conditionally:

1. `rN ∈ target.out` → satisfied, whatever the codes. **The strengthening escape:**
   unconditional production covers every edge.
2. `rN ∈ target.out_cond` with codes `have` → satisfied iff `ccs ⊆ have`. Coverage,
   not equality, so a target guarding MORE edges than demanded conforms; it
   degenerates to equality in the single-code case, which is every real shape.
3. otherwise → violation.

**Codes compare by canonical EQUALITY, not by implication.** `eq` implies `le` on
68k, so a target guarding `le` genuinely does satisfy a bound promising `eq`, and
this relation rejects that pair. Ruled deliberately:

- The rejection is the SAFE polarity. The author's remedy is to spell the bound's
  own code — the same escape §7.2's error tier is justified by. A wrong ACCEPT is a
  silent miscompile; a wrong REJECT is a compile error with an obvious fix.
- An implication lattice is a per-CPU flag-semantics table (16 codes on 68k, its own
  proof, its own tests) with **zero corpus demand**: three conditional-out declarers,
  all `if eq`, and no bound demanding one at all.
- House rule: over-claim is safe, never ship an unverifiable claim.

Ledgered with a demand gate: the first time a real target's code strictly implies a
real bound's and the author must weaken the bound to compile.

### §2.4 — THE §7.1 CLOBBER-LICENSING HALF IS UNTOUCHED

Explicitly confirmed. The licensing line is, before and after:

```rust
let writable: BTreeSet<&String> = bound.clobbers.iter().chain(&bound.out).collect();
```

`out_cond` is **not** a term and was not made one. Under §7.1 a cond-out register's
ABSENCE from `clobbers` IS the survives-claim, and `Contract` encodes that claim
purely by that absence; licensing a clobber off `out_cond` would erase it. That is
the bug B′-0c shipped. The existing pin
`a_conditional_out_does_not_license_the_target_to_clobber_it` still passes, and it is
the test that would catch a regression.

### §2.5 — Corpus reality, measured own-run

| | count | where |
|---|---|---|
| procs declaring a conditional out | **3** | `tile_cache.emp:134` `TileCache_FindStagedBlock`, `core.emp:123` `AllocDynamic`, `core.emp:182` `AllocEffect` |
| distinct condition codes used | **1** (`eq`) | all three |
| contract types (`type X = proc`) declaring one | **0** | 9 contract types exist; none |
| interface hooks declaring one | **0** | 2 hooks exist; neither |
| `extern proc` declaring one | **0** | 0 externs in the corpus |
| **bounds demanding a conditional out** | **0** | — |

The relation only fires against a BOUND, and no bound demands a conditional out. So
the hole is **latent with zero live instances**, and the fix changes **zero corpus
diagnostics**. That is the expected result, and the proof burden falls entirely on
unit tests.

### §2.6 — The three tests, and the mutant

All in `crates/sigil-frontend-emp/tests/contract_closure.rs`:

| role | test | asserts |
|---|---|---|
| **POSITIVE** | `a_conditional_bound_is_satisfied_by_a_target_guarding_the_same_cc` | bound `out(a1 if eq)` + target `out(a1 if eq)` → no violations |
| **NEGATIVE** | `a_conditional_bound_rejects_a_target_guarding_a_different_cc` | bound `out(a1 if eq)` + target `out(a1 if ne)` → violation naming `conditional output \`a1\`` and `` `eq` `` |
| **STRENGTHENING** | `unconditional_out_satisfies_a_conditional_bound_out` | bound `out(a1 if eq)` + target unconditional `out(a1)` → no violations |

Plus `a_target_guarding_more_ccs_than_the_bound_demands_satisfies_it` (the coverage
witness that justifies subset over equality), and two accessor pins in
`contract_grammar.rs`: `cond_out_guards_carry_each_registers_condition_codes` (keys
equal `cond_out_regs`; two clauses on one register carry both codes) and
`cond_out_guards_fold_the_cc_aliases` (`hs`→`cc`, `lo`→`cs`).

**THE NEGATIVE FAILS WITHOUT THE FIX — mutant built and run.** Keeping the map type
and neutering only the code comparison
(`let missing: Vec<&str> = Vec::new();`) reproduces the pre-fix relation
`bound.out_cond ⊆ target.out ∪ target.out_cond` exactly. Result:

```
test a_conditional_bound_is_satisfied_by_a_target_guarding_the_same_cc ... ok
test a_target_guarding_more_ccs_than_the_bound_demands_satisfies_it ... ok
test a_conditional_bound_rejects_a_target_guarding_a_different_cc ... FAILED
test result: FAILED. 29 passed; 1 failed
```

Reverted; 30/30 at HEAD.

---

## §3 — Bars

### §3.1 — Byte bar: SEVEN targets, `cmp`, capture order

Derived from `crates/sigil-harness/golden/` in this worktree, built in
`capture_goldens.sh` order (config_a → s4.debug.bin; config_b AND lean → s4.bin;
canonical rebuilt after), `AEON_DIR` = `aeon/.worktrees/b4` at `77d5317`.

**BASELINE (before any edit)** and **POST-CHANGE** are identical:

```
OK   s4.bin          OK   demo.bin          OK   config_a.bin      OK   lean.bin
OK   s4.debug.bin    OK   demo.debug.bin    OK   config_b.bin
>> restoring canonical s4.bin + s4.debug.bin
OK   s4.bin          OK   s4.debug.bin
SEVEN-TARGET: ALL OK
```

Byte-neutral ×7, as a checker + CLI parcel should be. No target moved, no refreeze.

### §3.2 — `refreeze --check`

`refreeze --check: OK (tip \`b-jumps\`, chain len 44)` — at the branch point and at
HEAD. No chain bump, no 5-site ripple (nothing under `pins.rs` / `engine.inc` /
`mixed_dac_rom.rs` / `repin_pins.rs` / `repin.toml` was touched).

### §3.3 — Strict suite

`SIGIL_STRICT_GATE=1 AEON_DIR=<b4 worktree> cargo test --workspace --release`, full
capture to file, failures-first, never piped through `tail`/`head`.

`SIGIL_STRICT_GATE=1 AEON_DIR=<b4 worktree> cargo test --workspace --release
--no-fail-fast`, full capture to file, failures-first, never piped through
`tail`/`head`.

| | |
|---|---|
| **passed** | **3183** |
| **failed** | **0** |
| **ignored** | **4** |
| result lines | 307 |

`3183 + 4 = 3187`, which is EXACTLY the branch's own `#[test]` total (§3.4) — so
nothing was silently skipped. The four ignored are the standing set
(`chained_resume_debug`, `chained_resume_plain`, `sigil_diff_reports_byte_identity`,
`secondary_pin_classes_match_the_hand_typed_baseline`), unchanged from master's four.

Failures-first grep for `^test .* FAILED|^failures:|^error` over the full log returns
**nothing**.

**A STALE-ARTIFACT TRAP WORTH REPORTING.** The first strict run failed 18 tests in
`sigil-clownlzss-sys --test byte_exact`, all `read level_select_2p.raw: No such file
or directory` — for fixtures that exist in this worktree. Cause: the WARM `target/`
this lane was handed had been seeded from `sigil/.worktrees/sr`, a worktree since
removed, so the test binary baked `CARGO_MANIFEST_DIR` pointing at a deleted path. It
is not a code defect and it is not visible from the failure text. **41 test binaries
carried the stale path.** Fixed by `find . -name '*.rs' -exec touch {} +` and a full
recompile. Read-only check of the B′-3 worktree: its binaries bake their OWN path, so
the trap was specific to this lane's seed. Anyone handed a warm `target/` should
verify it before trusting a suite result — cargo's staleness tracking cannot see it.

Clippy over the two crates this parcel touches reports 48 findings, **none of them on
any file in the diff** (`main.rs`, `closure.rs`, `ast.rs`, `resolve/contract.rs`,
`flag_check.rs` are all clean). They are the pre-existing toolchain drift handoff §3
item 7 assigns to its own solo lane.

### §3.4 — Test-delta arithmetic, every function named

`git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'`, per-file diffed. **Against
the BRANCH POINT `50382ddc`, not `master`** — see §3.5.

| file | 50382ddc | bprime-4 | Δ |
|---|---|---|---|
| `crates/sigil-cli/tests/contract_closure_corpus.rs` | 8 | 9 | **+1** |
| `crates/sigil-frontend-emp/tests/contract_closure.rs` | 27 | 29 | **+2** |
| `crates/sigil-frontend-emp/tests/contract_grammar.rs` | 29 | 31 | **+2** |
| `crates/sigil-frontend-emp/tests/game_contract.rs` | 21 | 23 | **+2** |
| **workspace `#[test]` total** | **3180** | **3187** | **+7** |

Eight functions added, one deleted, net +7. Named:

1. `the_contracts_report_is_wired_and_carries_the_targets_defines` — the report surface
2. `a_conditional_bound_rejects_a_target_guarding_a_different_cc` — **the NEGATIVE**
3. `a_target_guarding_more_ccs_than_the_bound_demands_satisfies_it` — the coverage witness
4. `cond_out_guards_carry_each_registers_condition_codes` — accessor pin
5. `cond_out_guards_fold_the_cc_aliases` — the alias fold, accessor level
6. `probe_hook_conditional_out_rejects_a_target_guarding_a_different_cc` — **the NEGATIVE, end to end** (panel A#9/C)
7. `hook_conditional_out_folds_the_cc_aliases` — **the alias fold, end to end** (panel A#9)
8. **DELETED:** `a_conditional_bound_is_satisfied_by_a_target_guarding_the_same_cc` — byte-identical
   inputs and assertion to `the_honest_alloc_dynamic_shape_conforms_to_a_matching_bound`
   four tests above it (panel A#10). That test's doc now names its dual role as the
   matching-condition control.

`unconditional_out_satisfies_a_conditional_bound_out` kept its name and gained an
explicit `eq` guard in its fixture; it is the strengthening witness. No other rename.

### §3.5 — MASTER MOVED DURING THIS PARCEL

The B′-3 lane merged while this ran: sigil master went `50382ddc` → **`cbd72ff1`**
("Merge bprime-3a: cycle budgets, the Z80 half", +52 tests in `cycle_budget.rs` ×2).
`bprime-4` is still based on `50382ddc` and was NOT rebased — merging is the overseer's
sequencing call and every parcel re-proves after its rebase (handoff §2 limit 3). All
numbers in this packet are against `50382ddc`. **The rebase is expected to be clean:**
B′-3 touches `sigil-isa` and `cycle_budget.rs`; this parcel touches `sigil-cli`
reporting, `closure.rs`, `ast.rs`, `flag_check.rs`, `resolve/`. Disjoint, as the lane
split intended.

---

## §4 — Lens panel

Three fresh read-only lenses over `git diff master...bprime-4` plus surroundings.
**A** ceremony/style/cold-reader, **B** corpus-pattern (both directions), **C**
correctness-hazard, pointed specifically at the §7.1 clobber-licensing invariant.

**The panel earned its keep again.** It cleared the charge it was aimed at, and then
found a live latent bug nobody was looking for, a warn-tier divergence a green test
was pinning, and two false doc claims this parcel had freshly written. It also
DISAGREED with itself once, which is why the disagreement was adjudicated own-run
rather than averaged.

### §4.1 — The primary charge: CLEARED

Lens C verified the §7.1 clobber-licensing half is intact: `writable` is byte-identical
across the diff (`bound.clobbers ∪ bound.out`, no `out_cond` term), every
`Contract::out_cond` reader in the workspace was grepped (two production sites, two test
helpers, nothing else), and the B′-0c mutant — adding `.chain(bound.out_cond.keys())`
— is caught by the existing `a_conditional_out_does_not_license_the_target_to_clobber_it`.
The invariant is not merely unbroken, it is test-defended.

### §4.2 — THE DISAGREEMENT, adjudicated own-run

Lens B reported a latent bug in `flag_check::invalid_edge`. Lens C, checking the same
question ("does canonicalizing only in the guards map create an inconsistency?"),
concluded there was none — "every downstream reader folds at point of use", naming
`flag_check.rs:615, 637` among them.

**Lens B is right; lens C checked the wrong two functions.** Verified own-run at
`flag_check.rs:396`:

```rust
fn invalid_edge(&self, call_idx: usize, cc: &str) -> Option<usize> {
    let neg = negate_cc(cc)?;                 // canonical: "hs" -> "cs"
    …
    if let Some(bc) = branch_cond(mnem) {     // canonical: "bhs" -> "cc"
        return if bc == cc { fall }           // "cc" == "hs"  -> FALSE
               else if bc == neg { taken }    // "cc" == "cs"  -> FALSE
               else { None };                 // bails
```

`negate_cc` folds aliases; `branch_cond` returns canonical; the raw declared `cc` was
compared against a canonical one and matched neither. A proc declaring `out(rN if hs)`
guarded by `bhs`/`bcc` therefore made the walk bail, and `[call.result-invalid-path]`
**silently never fired** — for exactly the guard spellings it fires for as `cc`/`cs`.
Half the spellings, one polarity.

The tell lens B spotted: the sibling `valid_edge` twelve lines above canonicalizes
explicitly, with the comment *"double-negate folds hs/lo aliases"*. Two functions with
the same job, one doing it and one not.

**FIXED** — both now call `canonical_cc` (and `valid_edge`'s double-negate idiom is
gone with it), and `cond_out_pairs_of` canonicalizes too, so every cc-carrying view of
a declaration agrees. False-negative polarity, and corpus-unreachable today (three
conditional-out declarers, all `if eq`), so nothing miscompiled — but it is precisely
the defect class this parcel exists to close, and this parcel's own new helper fixes
it in one line.

### §4.3 — Findings and dispositions

**FIXED IN PARCEL (16)**

| # | lens | finding | what was done |
|---|---|---|---|
| 1 | B2-2 | `invalid_edge` never canonicalized its cc — a live latent `[call.result-invalid-path]` false negative | §4.2; `canonical_cc` in `invalid_edge` + `valid_edge`, and in `cond_out_pairs_of` |
| 2 | B1-1, C7 | **`--report ram` showed a cleaner tree than the build** — it routed only `build_ram_report`'s diags, never the manifest scan's, so the 12 `[module.path-mismatch]` warnings were invisible from one report and visible from its sibling. `warn_tier_corpus.rs` PINNED the omission ("must print nothing at all"). Contradicted `notes/2026-08-04-warning-tier.md` rows 13/S3, both marked FIXED | `scan_or_exit` shared by both reports; the gate now asserts the tally instead of its absence |
| 3 | A19, B1-1 | the two reports hand-copied a 6-line prologue | one `scan_or_exit` + `profile_defines` + `print_report_header` |
| 4 | A1 | `print_contract_report`'s **freshly written** doc claimed a module count it does not print and "one section per firing family" when `[context.*]` is six lists in one blob | doc corrected; the `[context.*]` tail now labels its sub-lists (`regions:` / `claims:` / `firings:`) |
| 5 | A3 | `item-4 core` / `item-4 rider` — session-agenda numbers, unresolvable by a cold reader, in **shipped CLI output** (inherited from the dev driver, but the bar rose when the code moved) | renamed to `(§1)` / `(inference tier)`; the `[proc.clobber-undeclared]` family also got the label it lacked |
| 6 | B1-3, C8 | `ReportKind`'s doc claimed a report "can never disagree" with its ROM; the DEFINES are target-accurate, the MODULE SET is not | scope stated precisely in the doc; the narrowing is ledgered |
| 7 | C2 | the diff's new rationale asserted unconditional production is "strictly stronger, whatever the codes" — true for production, **false for the §7.1 survives half** | rationale scoped in `closure.rs` and in the coverage test's doc; the missing licensing term ledgered |
| 8 | A10 | `a_conditional_bound_is_satisfied_by_a_target_guarding_the_same_cc` had **byte-identical inputs and assertion** to a test four above it | deleted; the existing test's doc names its dual role as the control |
| 9 | A7 | `ProcSig::cond_out_regs` was left dead — the diff repointed its one caller and added a fourth accessor instead of replacing the third | deleted; its test uses go through `cond_out_guards().keys()` |
| 10 | A9 | the `hs`-vs-`cc` guarantee was claimed in `canonical_cc`'s doc but tested only at the accessor — no test drove it through `contract_of_sig` → `subcontract_violations` | `hook_conditional_out_folds_the_cc_aliases`, end to end through the real hook-binding path; plus `probe_hook_conditional_out_rejects_a_target_guarding_a_different_cc` for the mismatch |
| 11 | A4 | `cond_out_guards`' doc gave a positive design reason for what the parcel's own ledger row calls a §7.2 contradiction | doc now names the row and calls the keying PROVISIONAL (the forced-spelling site-comment rule) |
| 12 | A12 | `--report` silently ignored `-o` / `--emit-lst` | usage error, exit 2 |
| 13 | A13 | a repeated `--report` silently last-wins, against `ReportKind::parse`'s own stated principle | conflicting kinds are a usage error |
| 14 | A14 | unifying the flag did not unify the reports — only `contracts` printed its provenance | shared header; `--report ram` now carries `defines:` too, and `MAX_RING_BUFFER` sizes RAM regions |
| 15 | B1-2 | `run_build_native` still hand-rolled the same 5-arm target→label match, so a label edit could desync the build from the report | build label reads `label_and_profile().0` |
| 16 | B1-7, B1-8, B1-9, B1-10, A20 | stale `--ram-report` strings in `report_warnings`' doc and two `[ram.no-region]` messages; the new test's doc miscounted the define-free gates (7 of 8, not "hand-written"); missing `native::ensure_generated`; kill row 103 buried mid-table and naming only 3 of the 7 gate sites | all fixed |

**LEDGERED, NOT FIXED (9)** — each with its reason:

| lens | finding | why not now |
|---|---|---|
| C2 | **`target.out` / `target.out_cond` are unlicensed write channels.** Measured: bound `clobbers(d0) out(a1 if eq)` vs target `clobbers(d0) out(a1)` returns NO violation, though the target destroys a1 exactly where §7.1 says the bound promised it survives | **Not a regression** — master accepts the identical pairs; the missing term predates the cc comparison. And `unconditional_out_satisfies_a_conditional_bound_out` — the strengthening escape §7.2's error tier leans on — asserts one of the accepted pairs, so adding the term is a RULING on that escape, not a bug fix. Corpus-unreachable (no bound declares a conditional out) |
| C3 | `[proc.out-cond-invalid]` never runs on an `extern proc` or contract-type signature, and the cc just became load-bearing there | fail-safe polarity (loud, not silent), latent (0 externs, no signature declares one); the fix belongs with signature validation, not the relation |
| C4 | the mixed-mention row understated the polarity and missed the RANGE case | **row amended** rather than deferred: as a BOUND the demotion under-demands (unsound), as a TARGET it over-rejects (pessimism); `out(a0-a2, a1 if eq)` hits it identically |
| A/B L6, B1-5 | **the lost arbitrary-file-list census mode** — lens A calls it "the one substantive capability regression in the branch" | see §5 item 7: flagged for an explicit gate ruling, not a silent pass. Ledgered with `emp_census`'s kill condition attached |
| A15/16, L3/L4 | the report has no `path:line:col`, hardcoded column widths that demonstrably overflow, and no machine-readable form (the new gate asserts on a prose string) | all three dissolve together if `ContractReport` yields span-carrying rows rendered through the existing diagnostic path + `--report-format json`. That is a design parcel, not a fix |
| A6, L2, B1-11 | the four-accessor naming grid | the rename reaches `cond_out_pairs`' consumers in `lower/proc.rs` and `corpus_contracts.rs`; `ProcDecl::cond_out_regs` (already dead pre-parcel) dies with it |
| B2 tbl 2 | `canonical_cc` is a string-level patch over the existing `value::Cc`, which already has no `Hs`/`Lo` variants; four more inline alias folds remain | the two with a bug behind them were fixed; the other four are byte-neutral tidying across two more modules |
| B2-3 | the target→profile map and its five label strings are spelled six times; no `native::profile_for_shape` exists | the real hoist belongs in `sigil-harness`, not this parcel |
| B1-3 | scoping the contract report's MODULE SET via `profile.registry` | every pre-existing corpus gate walks `engine/` + `games/` whole; tightening the report first makes it disagree with all of them. Lands with kill row 103 |

**DECLINED (2)**

| lens | finding | reason |
|---|---|---|
| C6 | `m.file.clone()` per module in the report path | measured: the whole run is **180 ms** including scan, parse and the full closure/CFG/eval walk. No cheaper shape without changing `analyze_corpus_with`'s public signature to serve a developer-timescale report — the wrong direction, and it churns 6+ call sites. If that signature ever changes it should be for the borrow, not the speed |
| B1-6 | folding `emp_census` into `--report` | it emits TSV over an arbitrary file list — **precisely the capability `--report contracts` dropped**. Folding it now would break its documented pipeline use. Kill condition ledgered instead: it follows once `--report` grows a file-list input or a machine-readable mode |

### §4.4 — Non-vacuity: four mutants built and run

Every new cc test was proven to fail without the thing it tests.

| mutant | tests that went RED |
|---|---|
| drop the code comparison (`missing = Vec::new()`) — reproduces the pre-fix relation exactly | `a_conditional_bound_rejects_a_target_guarding_a_different_cc`, `probe_hook_conditional_out_rejects_a_target_guarding_a_different_cc` |
| drop `canonical_cc` from `cond_out_guards_of` | `cond_out_guards_fold_the_cc_aliases`, `hook_conditional_out_folds_the_cc_aliases` |
| `analyze_corpus_with(&files, &[])` in the report | `the_contracts_report_is_wired_and_carries_the_targets_defines` |

All reverted; the tree is green at HEAD.

---

## §5 — Honest gaps

1. **The two findings in §1.5 are REPORTED, not FIXED.** Aeon adoption is held by
   lane discipline, and both need the corpus fix and the gate flip in one change.
2. **`--report contracts` scans the WHOLE aeon tree**, so both games' modules enter
   one closure under one game's defines. That is what `emp_contracts` did (every
   recorded invocation was `find <aeon> -name '*.emp'`) and it is why the demo shape
   still reports 6 slot firings' worth of sonic4 modules as reachable. Narrowing to
   the target's reachable module set would be NEW analysis, which the spec forbids
   for this task. Not ledgered as a defect because the whole-tree walk is the
   deliberate census shape; named here so the next consumer does not assume
   per-target module scoping.
3. **The report has no machine-readable form.** It is aligned text, like
   `--report ram`. Nothing consumes either programmatically yet.
4. **The mixed-mention modelling inconsistency is ledgered, not fixed** —
   `out(a1, a1 if eq)` is conditional-only in `Contract` but unconditional under
   §7.2's reading via `cond_out_pairs`. Zero corpus instances; the correction moves
   a register between two live relation terms with nothing to prove against.
5. **`emp_census` (the sibling binary) was left alone.** It is a different analysis
   (the per-proc write-set table), not a `ContractReport` view, and folding it in
   would be a second design question this parcel did not measure demand for.
6. **The cc fix is unproven against a real bound**, because no real bound exists. Its
   whole proof is the tests in §2.6 — four of them now, two of which drive the real
   hook-binding path end to end, all four mutant-proven.

7. **THE ONE ITEM WANTING AN EXPLICIT GATE RULING: the lost file-list census mode.**
   Retiring `emp_contracts` cost the contract census its ability to run over an
   ARBITRARY set of `.emp` files with arbitrary `-D`. `--report contracts` requires a
   scannable aeon manifest and one of five fixed target profiles, so pointing the
   closure at a scratch file, a reduced repro, or a non-aeon tree is no longer possible
   with any shipped tool. Lens A called this "the one substantive capability regression
   in the branch", and it compounds the recorded `[bprime-0b lens C]` observation that
   a Sigil consumer outside the aeon tree is not backstopped. I judge fixed profiles
   the right DEFAULT and the regression acceptable — no in-tree consumer, no recorded
   demand, and the `-D` drift it removes caused two live blind spots. But it is a
   deliberate capability trade, not an oversight, so it should be accepted or reversed
   explicitly. Reversal is `--report contracts --files a.emp … -D X=1`, additive.

8. **`--report ram` now prints more than it did.** Fixing the warn-tier divergence
   means it emits the 12 `[module.path-mismatch]` warnings the build already emits.
   That is the correct behaviour and the reason the gate's assertion was inverted, but
   it IS a visible output change to a shipped surface, made on a panel finding rather
   than on a work order.

9. **The panel's own disagreement was resolved by me, not by a third lens.** Lenses B
   and C reached opposite conclusions on the `canonical_cc` consistency question; I
   adjudicated own-run at the source (§4.2) and B was right. A reviewer who wants the
   adjudication independently checked should read `flag_check.rs:396` against `:457`.

---

## §6 — Files touched

| file | what |
|---|---|
| `crates/sigil-cli/src/main.rs` | `ReportKind`, `--report <kind>`, `run_contract_report`, `print_contract_report`, `BuildTarget::label_and_profile`; `--ram-report` removed |
| `crates/sigil-cli/src/bin/emp_contracts.rs` | **DELETED** (211 lines) |
| `crates/sigil-cli/tests/warn_tier_corpus.rs` | 3 `--ram-report` → `--report ram` |
| `crates/sigil-cli/tests/contract_closure_corpus.rs` | +1 gate on the report surface |
| `crates/sigil-frontend-emp/src/ast.rs` | `canonical_cc`, `cond_out_guards_of`, `ProcSig::cond_out_guards` |
| `crates/sigil-frontend-emp/src/closure.rs` | `Contract::out_cond` → map; the out-satisfaction half of `subcontract_violations` |
| `crates/sigil-frontend-emp/src/resolve/contract.rs` | `contract_of_sig` builds `out_cond` from the guards map |
| `crates/sigil-frontend-emp/src/flag_check.rs` | `invalid_edge` + `valid_edge` canonicalize their cc (panel) |
| `crates/sigil-frontend-emp/src/resolve/mod.rs` | two `[ram.no-region]` messages named the removed flag |
| `crates/sigil-frontend-emp/tests/contract_closure.rs` | `contract_cond` takes `(reg, cc)` pairs; +3 tests, −1 duplicate |
| `crates/sigil-frontend-emp/tests/contract_grammar.rs` | +2 accessor pins |
| `crates/sigil-frontend-emp/tests/game_contract.rs` | +2 end-to-end cc pins (panel) |
| `docs/superpowers/notes/twin-scaffolding-kill-list.md` | row 103 (all 7 gate sites named, moved to the table end) |
| `docs/superpowers/notes/campaign-gap-ledger.md` | 12 rows + 1 amendment |

**No aeon file was touched.**

---

## §7 — What each pass added (step-3 vs step-5)

### Pass 1 — the report surface

**Step-3 findings (asks / reads-wrong / kill rows / ledger)**

- *Ceremony scan.* `emp_contracts`' 40 lines of hand-rolled file discovery and `-D`
  parsing were pure ceremony against a manifest and a profile that already exist.
  Outcome: deleted, not ported.
- *Escape-hatch census.* The hand-maintained define list is the recurring shape —
  `GAME_CONFIG_DEFINES` in `contract_closure_corpus.rs` plus three define-free
  `analyze_corpus` callers. Outcome: **kill-list row 103**, with the land-together
  constraint stated.
- *Comment-claim audit.* `entity_window.emp:493`'s "preserves a0/d2-d5" is FALSE in
  the debug shape. Outcome: ledgered (aeon-side).
- *Contract audit.* Five debug-shape contracts under-declare; six call sites hand a
  typed slot an untyped value. Outcome: two ledger rows, neither fixed.
- *Name audit / magic numbers / codename references.* Nothing. Two parcel tags
  ("B′-4") that had crept into doc comments were removed before commit.
- *Language/format ask.* None raised by this half. The report is plain `println!`
  against a struct; nothing in the language was strained.

**Step-5 findings (optimizations taken / not taken)**

- *Invariant ladder / counter audit / guard coverage.* **Not applicable** — no loop,
  no counter, no budget. The report is a one-shot corpus walk.
- *Hardware cross-check.* **Not applicable** — the CLI touches no hardware.
- *C1 (cycle/perf) is FLAGGED INACTIVE, with basis:* every line added runs on the
  host at developer timescale (`run_contract_report`, `print_contract_report`,
  `label_and_profile`, arg parsing). Zero emitted bytes changed on any of the seven
  targets, so there is no ROM-side cost to weigh. Reversible at the gate.
- *Not taken, logged:* `run_contract_report` clones every parsed module
  (`m.file.clone()`) to satisfy `analyze_corpus_with(&[ast::File])`. Threading a
  reference would mean changing the frontend's public analysis signature to serve a
  developer-timescale report — the wrong direction, and it would touch every
  `analyze_corpus` caller. Declined with reason; not a ledger row because it is a
  deliberate call, not a gap.
- *Silent-tradeoff comments.* The whole-tree module scope (§5 item 2) is the one
  accepted compromise; it is stated in `run_contract_report`'s doc comment and here.

### Pass 2 — the cc fix

**Step-3 findings**

- *Contract audit.* The hole is real and is precisely `Contract::out_cond`'s type.
  Outcome: fixed.
- *Comment-claim audit.* `subcontract_violations`' doc said "The condition CODES are
  not compared; a target guarding on a different cc than the bound is a hole recorded
  in the ledger." True when written, false now. Outcome: rewritten to state the
  relation and the equality-vs-implication ruling.
- *Noticing.* FOUR similarly-named accessors on one concept now exist
  (`cond_out_regs` / `cond_only_out_regs` / `cond_out_pairs` / `cond_out_guards`).
  Each has a distinct, documented consumer class, but the naming does not carry the
  distinction — you must read the doc. Raised for the panel; disposition in §4.
- *Domain-type scan.* A condition code is a closed 16-value vocabulary carried as a
  `String` throughout the frontend (`CondResult.cc`, `VALID_CCS`, `flag_check`). A
  `Cc` newtype would make `canonical_cc` a constructor and the alias fold
  unforgettable. Not built — it is a cross-cutting frontend change with no demand
  yet. Raised for the panel.
- *Ledger.* Two rows: the implication-order demand gate, the mixed-mention modelling
  inconsistency.

**Step-5 findings**

- *Invariant ladder.* The new loop is over `bound.out_cond`, whose corpus cardinality
  is 0 and whose worst realistic case is single digits. Nothing to hoist.
- *Guard-coverage audit.* Enumerated every `Contract::out_cond` reader in the
  workspace: one production construction site (`contract_of_sig`), one production
  relation site (`subcontract_violations`), one test helper. The clobber-licensing
  half reads `clobbers ∪ out` and was verified untouched (§2.4).
- *C2 (correctness-hazard) findings taken:* the alias fold (`hs`/`lo`) — without it
  the relation rejects two spellings of the same condition; and the decision to key
  the guards map on `cond_out_regs`' set rather than `cond_out_pairs`', which would
  have dropped a mixed-mention register out of every relation term.
- *C1 FLAGGED INACTIVE:* compile-time analysis only; zero ROM bytes changed.
- *No behaviour-affecting engine change*, so no oracle A/B is owed.

### Pass 3 — the dry circuit, and what the panel re-opened

My own pass came up empty at all three steps, so the panel was dispatched on that dry
claim. **It did not return nothing** — which is the point of the rule, and the reason
DRY is not self-declared. The circuit re-opened:

**Step-3 findings (panel-sourced)**

- *Comment-claim audit* — THREE freshly-written false claims: `print_contract_report`'s
  header description (A1), `ReportKind`'s "can never disagree" (B1-3/C8), and the
  strengthening rationale's "strictly stronger, whatever the codes" (C2). All three
  were written by me in this parcel, and a cold reader would have believed all three.
  This is the class the audit exists for and my own pass walked past every one.
- *Codename-reference audit* — `item-4 core` / `item-4 rider` rode into a shipped CLI
  surface from a dev driver (A3). The bar rose when the code moved and I did not
  re-apply it.
- *Contract audit* — the relation licenses only `target.clobbers` (C2); the cc validity
  check never runs on a signature (C3). Both ledgered.
- *Name audit* — four accessors over one concept, distinguished only by their docs
  (A6/L2/B1-11), and one of them left dead by my own repointing (A7). The dead one is
  cut; the grid is ledgered.
- *Language/format asks* — six, all ledgered: a `Cc` newtype (the strongest, and it
  would have made §4.2's bug unrepresentable); a naming convention for view families;
  reports as `Diagnostic`s rather than `println!`s; machine-readable output; a report
  scaffold; the file-list input.

**Step-5 findings (panel-sourced)**

- *Guard-coverage audit* — the finding of the round: `invalid_edge` vs `valid_edge`,
  two functions with the same job disagreeing on canonicalization (§4.2). **Taken.**
- *Counter/cache audit, applied to the warn tier* — two report surfaces, two policies,
  with a green test pinning the wrong one (B1-1/C7). **Taken.**
- *C1 (cycle/perf)* — lens C measured the whole report run at 180 ms and declined the
  clone with numbers. **Not taken, with the measurement.**
- *Silent-tradeoff comments* — the module-set scope is now stated at the site, not only
  in the packet.

**Step 4** — still empty: no construct adopted, built, asked for, or deleted.

A second panel round was not dispatched. One round per dry claim is the rule
(cost-bounded, not continuous), and every finding above is either fixed or ledgered
with its reason.
