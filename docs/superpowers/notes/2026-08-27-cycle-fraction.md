# CYCLE-FRACTION — how much of the shipped cartridge's code the cycle model can time

**2026-08-27.** Static measurement, no emulator (the model is static by construction —
`cycle_budget.rs` walks an evaluated `CodeBuf` with two cost tables and the shared `Cfg`).
Reference tree: `/home/volence/sonic_hacks/.sigil-ref-353aaa49`, a clean detached aeon
worktree at **353aaa49** — the aeon SHA the current sigil refreeze (`521956f9`,
`band-ceiling-16`, chain 172) pairs with. Wall clock for the whole measurement: **0.85 s
user / 1.0 s elapsed** per run.

This note discharges the gap-ledger row *"CYCLE-ASK's shape is settled before the work
starts"* (2026-08-26), whose kill condition was "the three figures exist and name the command
that produced them". They do; the command is in §5.

---

## 1. The headline

**Canonical shape = `sonic4 debug`** (the widest: every comptime arm on). Two denominators,
reported side by side because they disagree by a factor of seven and the disagreement is the
finding.

### D1 — instruction sites (every `CodeItem::Instr` the shape emits)

| | total | exact | ceiling only | cannot time |
|---|---|---|---|---|
| **68000** | 12 215 | 8 170 — **66.88 %** | 4 040 — **33.07 %** | 5 — **0.04 %** |
| **Z80** | 3 040 | 2 216 — **72.89 %** | 0 — **0.00 %** | 824 — **27.11 %** |
| **combined** | 15 255 | 10 386 — **68.08 %** | 4 040 — **26.48 %** | 829 — **5.43 %** |

### D2 — procs (worst bucket wins: a proc is only "exact" if *every* instruction in it is)

| | total | exact | ceiling only | cannot time |
|---|---|---|---|---|
| **68000** | 357 | 34 — **9.52 %** | 322 — **90.20 %** | 1 — **0.28 %** |
| **Z80** | 142 | 27 — **19.01 %** | 0 — **0.00 %** | 115 — **80.99 %** |
| **combined** | 499 | 61 — **12.22 %** | 322 — **64.53 %** | 116 — **23.25 %** |

### The divergence, foregrounded

D1 says the 68000 half is two-thirds exactly timeable. D2 says one proc in eleven is. **Both
are true and they answer different questions.** The reconciliation is one fact: 98 % of the
68000 ceiling bucket is the *linker-relaxation* ruling (a bare symbolic operand charged its
`abs.l` rung, see §2), and essentially every proc in the engine touches at least one RAM
variable by name. One relaxed operand in a 400-instruction proc moves that whole proc from
"exact" to "ceiling" under D2 while moving 0.25 % of D1.

So: **if you want to know how much of the instruction stream the tables can price, read D1.
If you want to know how many routines could carry `@cycles_exact`, read D2 — and D2 is
dominated by a width the linker has not chosen yet, not by ignorance of the machine.**

### Ceiling decomposition (68000 only; the Z80 table has no ceiling variant at all)

| cause | count | share of the ceiling bucket | share of all 68000 instructions |
|---|---|---|---|
| linker relaxation only | 3 961 | 98.04 % | 32.43 % |
| genuinely data-dependent | 79 | 1.96 % | 0.65 % |

The discriminator is a re-price with relaxation removed: every bare `Sym`/`SymOff` operand
replaced by the same address written `.l` (`AbsSym { long: true }` — the rung the ceiling
*already charges*, so the number cannot move), and an unsized conditional given the `.w`
width whose fall-through the ceiling already charges. Exact after that pin ⇒ relaxation only.
`jbra`/`jra`/`jbsr` are relaxation by construction (their four-rung ladder is priced in the
front end before the ISA table is consulted). See `ceiling_is_relaxation_only` in the tool.

**Only 0.65 % of shipped 68000 instructions are data-dependent** — `muls` (5), `divs` (2),
`divu` (2), `lsr`/`lsl`/`asl`/`asr`/`rol` with a register count (~50), `st`/`sf`/`sgt` on a
data register, `bclr`/`bset` on `Dn`, `dbeq`.

### Layer B — whole-proc reach (a different question again; see §3)

| | procs | could carry `@budget(cycles:)` | could carry `@cycles_exact` | declare one today |
|---|---|---|---|---|
| **68000** | 357 | 63 — **17.6 %** | 12 — **3.4 %** | **1** |
| **Z80** | 142 | 10 — **7.0 %** | 10 — **7.0 %** | **0** |

Why the refusals (first finding per proc, canonical shape):

| 68000 | | Z80 | |
|---|---|---|---|
| `cycles.unbounded-transfer` | 95 | `cycles.unbounded-transfer` | 63 |
| `cycles.unbounded-loop` | 89 | `cycles.unbounded-loop` | 33 |
| `cycles.inline-data` | 67 | `cycles.unknown-op` | 24 |
| `cycles.opaque-call` | 42 | `cycles.opaque-call` | 12 |
| `cycles.computed-transfer` | 1 | | |

The single 68000 `computed-transfer` is `Player_SetState`; the one declared budget in the
whole corpus is `Process_DMA_Critical` (`engine/system/dma_queue.emp:313`,
`@budget(cycles: 670)`), which gets through a computed `jmp` only because it carries a
`targets(...)` clause.

### All seven shapes (robustness, not a headline)

Summed over `sonic4 plain/debug`, `demo plain/debug`, `config_a`, `config_b`, `lean` — these
are **proc-instances, not distinct procs**; a module in six shapes is counted six times.

| | D1 total | exact | ceiling | unmodeled |
|---|---|---|---|---|
| 68000 | 70 731 | 46 925 (66.34 %) | 23 771 (33.61 %) | 35 (0.05 %) |
| Z80 | 12 089 | 8 801 (72.80 %) | 0 (0.00 %) | 3 288 (27.20 %) |
| combined | 82 820 | 55 726 (67.29 %) | 23 771 (28.70 %) | 3 323 (4.01 %) |

Per-shape D1 exact share moves between 65.11 % and 67.04 % on the 68000 and is 72.89 %/73.37 %
on the Z80 wherever the sound driver ships. The stability is not an instrument artefact: the
shapes share most of the engine, and the two demo shapes (a different game, no sound) land in
the same band by a different module set.

---

## 2. Bucket definitions

The model's real structure is **four-valued per instruction**, not three — a conditional's
cost is a *pair keyed by outcome*, which is neither "one exact number" nor "a ceiling". The
mapping used here, and the reason for it:

**EXACT** — the table states a per-execution count it claims is the machine's.
* 68000 `CycleCost::Fixed { exact: true }`.
* 68000 `CycleCost::Branch { taken, not_taken, exact: true }` — **counted as exact.** Both
  numbers are exact; the pair is outcome-keyed, and `cycle_budget`'s walk routes `taken` to
  the branch edge and `not_taken` to the fall-through. The model is not uncertain about this
  instruction; it knows both costs and which is which. A reader who wants to bucket branches
  separately can: the sub-counts are reported (canonical shape, 68000: 7 929 fixed-exact +
  241 branch-exact; Z80: 1 865 fixed + 351 split).
* Z80 `Cost::Fixed(_)` and `Cost::Split { taken, not_taken }` — same reasoning.

**CEILING** — the table states an upper bound and says so.
* 68000 `Fixed { exact: false }` / `Branch { exact: false }`. Two causes, decomposed above:
  the linker-relaxation ruling (a bare symbolic operand or an unsized branch, charged its
  dearest rung), and genuine data dependence (`mulu`'s bit pattern, a shift count read from a
  register, `Scc`'s condition, `DBcc`'s two fall-through reasons).
* **The Z80 has no ceiling bucket and cannot have one:** `z80_cycles::Cost` carries no
  exactness flag. Every form it prices, it prices exactly; every form it is unsure of, it
  refuses. This is a structural asymmetry between the two halves, not a measurement result.

**CANNOT TIME** — the table refuses; there is no default cost.
* 68000 `CycleCost::Unmodeled`, Z80 `Cost::Unknown`.

**Where an instruction whose cost depends on operand VALUES lands:** in CEILING when the
model prices it as a maximum (the 68000 case), in CANNOT TIME when the model does not price
it at all. There is no third behaviour — no arm of either table returns a value-blind guess.
The one place a value is read and priced *exactly* is `mulu #imm, Dn`, where the UM's
`38 + 2n` has `n` = the popcount of a compile-time-known source; the all-ones case reproduces
the 70-cycle ceiling row, so the exact form and the maximum cannot drift apart.

**Where memory wait states land: nowhere.** The model is explicit that it bounds *issued*
cycles, not elapsed time — bus contention (the Z80 losing the bus to a 68000 DMA, either CPU
stalling on a VDP-port FIFO) is a whole-machine fact and is not modelled at all. It is not in
any of the three buckets because it is not an instruction property. Every number here is
"nominal cycles at zero wait states, no arbitration loss, interrupts not counted."

---

## 3. Denominators, and why two

**D1 = instruction sites.** One count per `CodeItem::Instr` in a shipped proc's evaluated
`CodeBuf`. This is a *static* count — a site inside a loop counts once, and an unreachable
site still counts. Chosen because it is the unit the cost tables actually operate on, so the
fraction is literally "how many of the model's decisions were decisions".

**D2 = procs, worst bucket wins.** A proc is EXACT iff every instruction in it is exact;
CEILING iff no instruction is unmodeled and at least one is a ceiling; CANNOT TIME iff any
instruction is unmodeled. Procs with zero instruction sites are excluded from both (see §6).
Chosen because it is the unit an author declares a contract on — `@budget` and `@cycles_exact`
are proc attributes — so D2 is the fraction of the corpus a checker could be *asked about*.

**Not used, and why:**
* **Bytes of emitted code.** Attributing bytes to instructions needs the full lower→encode
  path (symbol resolution, relaxation), which is exactly the machinery this measurement runs
  *before*. A hand-rolled extension-word length model would be a second instrument with its
  own error, unvalidated. **Not measured — stated as a gap, not silently skipped.** Expect it
  to sit between D1 and D2 in spirit but nearer D1: the relaxed-operand instructions (`move`,
  `lea`, `jbsr`) are the *longer* ones, so a byte denominator would show a somewhat larger
  ceiling share than D1's 33 %.
* **Basic blocks.** `flag_check::Cfg` exposes per-instruction edges, not a block list; deriving
  blocks would be another instrument. Skipped for the same reason.

**The two denominators share no parameter.** D1 enumerates over instruction sites; D2
enumerates over declaration sites. Neither is derived from the other by rescaling, which is
why their disagreement is informative rather than arithmetic.

**Layer B is a third question, not a third denominator.** D1/D2 ask *can this instruction be
priced*. Layer B asks *can this proc's worst path be bounded*, which additionally needs a
finite, local, totally-modelled path set: no back edge, no call, no escape from the body, no
computed transfer without a `targets(...)` clause, no inline data in the code stream. A proc
can be 100 % exact under D2 and still refuse a budget because it contains a loop. Note the
direction is not one-way either: `Process_DMA_Critical` is **CANNOT TIME under D2** (its jump
table is padded with five unreachable `trap #0` fillers) and **budgetable under Layer B**,
because the walk prices only *reachable* instructions. That single proc is the entire 68000
"cannot time" bucket in every shape, and it is the one proc in the corpus that carries a
machine-checked budget today.

---

## 4. Corpus — what "shipped" means here

`native::shipped_shapes()` is the seven-shape table the byte gates enumerate over. **All seven
were measured**; nothing was capped.

For each shape the walk reads:
1. every module in that shape's placement registry (`profile.registry`), plus its
   `game_ram_module` and `manifest_module` — 47 to 100 modules depending on shape;
2. **plus the five resident Z80 sound modules** (`z80_sound_driver`, `sound_sequencer`,
   `sound_sfx`, `sound_fm`, `sound_psg`), for shapes with `sound_on`.

Point 2 is load-bearing and was a trap. The resident sound blob is linked by **seam 1**, not
placed by the emp registry, so a registry-only walk sees **zero Z80 code in every sound-on
shape** — the first run of this measurement reported exactly that, and the only Z80 code it
found was `Z80_IdleProgram` in the two *no-sound* shapes. Any future coverage claim over "the
shipped Z80 half" that reads `native::registry` alone is measuring nothing. The accessor
`seam1::resident_sound_modules(aeon, debug, with_banked_carriers)` was added for this walk.

Each module is evaluated under its own `-D` env: registry modules get
`native::shape_defines(profile, aeon)` plus that shape's bound L1 interface env
(`bind_corpus_interfaces`), resident sound modules get their per-module const seam plus
`DEBUG`. A define-free walk cannot see inside `if DEBUG == 1 { }` and would describe code the
ROM does not carry.

**Non-vacuity witnesses, every shape:** 0 unresolved proc bodies, 0 dropped instructions, 0
unresolved comptime conditions. If any of those were non-zero the buckets would be describing
code that failed to lower rather than code that ships.

**Instrument-sensitivity control.** The run was repeated with the seam-1 banked `$8000`
carriers defined as comptime values (`SIGIL_CF_CARRIERS=1`) — the other of the two established
polarities in the tree. **Every number is byte-identical between the two runs** (`diff` shows
only the banner line). The carrier choice is not a confound.

---

## 5. Method and exact commands

The tool is `crates/sigil-harness/src/bin/cycle_fraction.rs` — a report binary, not a gate. It
never runs in the suite, changes no model behaviour, and exits 0 unless the corpus fails to
read.

```
# reference tree (already present; verified clean at the refreeze-paired SHA)
cd /home/volence/sonic_hacks/.sigil-ref-353aaa49 && git rev-parse HEAD && git status --short
# -> 353aaa49d37bd23ee629ef3c6c4096485f10d4bd, clean

cd <sigil worktree>
cargo build --release --bin cycle_fraction

AEON_DIR=/home/volence/sonic_hacks/.sigil-ref-353aaa49 ./target/release/cycle_fraction

# the instrument-sensitivity control
SIGIL_CF_CARRIERS=1 AEON_DIR=/home/volence/sonic_hacks/.sigil-ref-353aaa49 \
    ./target/release/cycle_fraction
```

What the tool does per proc, in order:

1. `eval_proc_body_env(file, name, params, body, span, counter, cpu, defines, ambient, iface)`
   → the evaluated `CodeBuf`. `cpu` comes from the `(cpu: z80)` attribute on the module or the
   enclosing `section` header.
2. **Layer A:** every `CodeItem::Instr` is priced by `m68k_cycles::instr_cost(mnemonic, size,
   ops)` or `z80_cycles::instr_cost(mnemonic, ops)` — the exact calls the compiler makes — and
   bucketed by the match in §2.
3. **Layer B:** `cycle_budget::check_cycle_budget(items, cpu, span, Some(u64::MAX), exact,
   noreturn)`. The unreachable ceiling makes `[cycles.over-budget]` impossible, so every
   finding returned is a *refusal*. Run once with `exact: false` (the `@budget` reach) and once
   with `exact: true`; for the second, `[cycles.path-mismatch]` is counted as a **verdict, not
   a refusal** — the model spoke, and said the paths differ.

Raw output of the run this note reports is reproducible with the command above; it is not
checked in (it is 250 lines of histogram and regenerates in one second).

---

## 6. Limits — what this measurement does not cover

* **The AS residual.** Three `.asm` files survive in the aeon tree:
  `engine/debug/debugger.asm` (the MD Debugger / crash-report island, ~806 lines, ~236
  instruction-position source lines before macro expansion) and the two `games/*/game_root.asm`
  shells (8 non-comment lines each, essentially `include` lines). `debugger.asm` is 68000 code
  that ships in six of the seven shapes and the `.emp` cycle model **never sees it at all** —
  it is not in any bucket. Order of magnitude: 2–5 % of the shipped 68000 instruction stream.
  This is the one respect in which "the real shipped cartridge's code" overstates the
  denominator.
* **Bytes were not measured.** See §3.
* **Data-emitting procs are excluded.** 15–25 procs per shape evaluate to a `CodeBuf` with zero
  instruction sites — a pure `dc.w` table (`DacSampleTable`, `SeqOpcodeTable`, the 22 vol-env
  tables in `sound_tables_z80.emp`), or a body whose every arm is comptime-gated off in that
  shape. They are excluded from both denominators. They contribute no timed code, but the count
  is reported per shape so the exclusion is visible.
* **Static, not dynamic.** Every figure is per *instruction site*, not per execution. A model
  that prices 67 % of sites exactly may cover much more or much less of the executed
  instruction stream. **This is the single most likely misreading of the headline** and nothing
  here supports a dynamic claim. A dynamic weighting would need an execution profile — that is
  the oracle lane's instrument, not this one, and it is TAGGED here rather than attempted.
* **`Cfg` reachability is the walk's, not the machine's.** Layer B refuses rather than guesses
  wherever it cannot follow control, so its 17.6 % is a floor on what is boundable, not a
  ceiling on what is bounded *correctly*.
* **A second, independent cycle model exists in the corpus and is not measured here.**
  `engine/effects/raster_dsl.emp` carries its own hand-written comptime cost model
  (`op_cost_cycles`, `fire_cost_cycles`, `RASTER_SCANLINE_CYC`) calibrated against
  emulator-measured fixtures, and it does **not** consult `m68k_cycles` at all. Two cycle
  models in one tree with no cross-check between them is a drift surface; flagged, not
  investigated.

---

## 7. Defects found in the model

Neither is a wrong number in the shipped corpus today. Both are recorded in
`campaign-gap-ledger.md` with kill conditions. **Neither was fixed here** — a fix belongs in its
own parcel with a red-first proof.

> **BOTH ARE FIXED as of `fix/cycle-model-soundness` (2026-08-27).** The two subsections below
> are kept as the DIAGNOSIS that led to the fix; read them as history, not as a live defect
> record, and do not re-file either from this text. What landed, and where the two accounts
> differ from what was actually measured:
>
> * **D-1** — `eval_cycles`/`eval_pad_to_cycles` now open with
>   `Evaluator::require_z80_for_timing`, raising `[cycles.wrong-cpu]` on a 68000 proc and on an
>   absent `ev.cpu`. It REFUSES rather than dispatching to `m68k_cycles`: that table answers a
>   weaker question (a third of the shipped 68000 stream is a ceiling — §1 — so an `==`
>   comparison against a sum of ceilings would assert a cost the machine need not have), and
>   `pad_to_cycles` has no 68000 pad unit to emit at all. **Two claims below re-derived wrong:**
>   there are TWO `pad_to_cycles(` sites in `z80_sound_driver.emp`, not one; and the
>   `dense: true` `jr` never reached the ROM — the 68000 mnemonic table rejected it downstream,
>   once per emitted pad unit. The `cycles()` and sparse-`pad_to_cycles` halves were exactly as
>   described: both built CLEAN on a 68000 proc with no diagnostic whatsoever.
> * **D-2** — `span_cost` accumulates and returns `TStates` (`u128`) with a plain `+`. Overflow
>   is UNREPRESENTABLE rather than detected: `MAX_SPAN_T_STATES` is computed in that same type
>   from `Cost::Fixed`'s `u16` payload and Rust's `isize::MAX` allocation cap, and const
>   arithmetic overflow is a compile error, so the width is checked by the compiler. There is
>   no limit left to refuse at.
>
> **Every measurement in §1–§6 is unaffected and was re-run on the fixed tree**: `cycle_fraction`
> reads `instr_cost`, never `span_cost` and never the builtins, and all seven shapes reproduce
> byte-for-byte (canonical 68000 12 215 / 8 170 / 4 040 / 5; Z80 3 040 / 2 216 / 0 / 824;
> all-shape 68000 70 731 / 46 925 / 23 771 / 35). No number in this note is stale.

### D-1 (latent, wrong-unit): `cycles(L1, L2)` and `pad_to_cycles(...)` have no CPU guard

`eval::builtins::eval_cycles` calls `z80_cycles::span_cost` **unconditionally**. The evaluator
knows the enclosing proc's CPU (`ev.cpu` is set by `eval_proc_body_env`) and `eval_cycles`
never reads it — there is no `cpu` reference anywhere in `builtins.rs`. `cycle_budget`, the
sibling consumer, dispatches on CPU properly; this one does not.

Consequence: in a **68000** proc, `ensure(cycles(.a, .b) == N)` prices the span with the Z80
T-state table. Most 68000 mnemonics are absent from that table, so the usual outcome is a loud
`[cycles.unknown-op]` bail — but with a *misleading* message ("add it to `z80_cycles`") on a
68000 proc. Worse, the two tables share two spellings that match on 68000-shaped operands:
`nop` (`Cost::Fixed(4)`) and `assume_some` (`Cost::Fixed(0)`). A 68000 timing pad made of
`nop`s therefore **succeeds and returns a number** — 4 per nop, which is coincidentally the
68000's own count but is by the model's own definition a *Z80 T-state at 3.58 MHz*, not a
68000 cycle at 7.67 MHz. `pad_to_cycles` then emits `nop`s against that number. Feed it a `jr`
(`dense: true`) and it would emit a Z80 instruction into a 68000 stream.

Live exposure: **none**. All 24 builtin call sites are in `(cpu: z80)` modules —
`grep -rnE '[^_a-zA-Z]cycles\(\s*\.' engine games --include='*.emp'` returns 18 in
`engine/sound/z80_sound_driver.emp`, 3 in `sound_sequencer.emp`, 3 in `sound_fm.emp`, and
nothing else; the single `pad_to_cycles(` is in `z80_sound_driver.emp`. (Beware the loose
pattern `cycles(`: it also matches the user comptime fns `scene_axis3_vblank_cycles`,
`op_cost_cycles`, `fire_cost_cycles` in `scene_dsl.emp`/`raster_dsl.emp`, which are not this
builtin.) The defect is a missing guard, not an active miscount.

Kill: `eval_cycles`/`eval_pad_to_cycles` refuse on a non-Z80 `ev.cpu` with their own diagnostic
id, with a red test that a 68000 proc calling `cycles(.a, .b)` over two `nop`s fails to build
instead of returning 8.

### D-2 (latent, silent saturation): `z80_cycles::span_cost` accumulates in `u16`

```rust
let mut total: u16 = 0;
...
Cost::Fixed(n) => total = total.saturating_add(n),
```

A straight-line span costing more than 65 535 T-states silently returns 65 535 — a number
where the function should refuse. This is the one place in either table's consumers where an
arithmetic limit produces a value rather than a bail.

Headroom: the largest Z80 proc in the shipped corpus is **167 instruction sites**; at the
table's dearest form (19 T for `ld r,(ix+d)`) a whole-proc straight line bounds at ~3 173 T,
about 20× below saturation. The real spans (`FILL` = 195, `DRAIN` = 195, `DRAINING_TAIL` = 194)
are two orders of magnitude below it.

Kill: `span_cost` accumulates in `u32`/`u64` and returns a `CycleBail` (or the type makes
overflow unrepresentable) rather than saturating, with a red test over a synthetic span past
65 535 T.

### What was hunted and NOT found

The 68000 EXACT bucket was enumerated by mnemonic (60 distinct) and read against
M68000UM §8 looking for a data-dependent form priced as an equality. **None found.** The
table's structure is refusal-by-default (`_ => CycleCost::Unmodeled` as the final arm), every
data-dependent family is `at_most(...)`, and the one value-aware exact row (`mulu #imm`)
degenerates to its own ceiling at the all-ones input. `enumerated_succs` cannot return an empty
target set (`targets.is_empty() → None`), so the dispatch arm cannot produce a
`(u64::MAX, 0)` cost pair. `is_call_mnemonic` covers `jsr`/`bsr`/`jbsr` and `call`/`rst`
completely, so no call is charged its own cost and walked past.

---

## 8. Where I think the brief was wrong

* **"Three buckets" is the right report but not the model's shape.** Per instruction the model
  is four-valued: exact-fixed, exact-outcome-keyed-pair, ceiling, refusal. Folding the pair
  into "exact" is a *ruling*, and it is the one that matters most for the Z80 half (351 of its
  2 216 exact instructions are `Split` pairs). Both sub-counts are reported so the ruling is
  reversible by the reader.
* **"Cannot time at all" is two different things.** On the 68000 it means five unreachable
  `trap #0` pad bytes — the table covers the shipped instruction set essentially completely
  (99.96 %). On the Z80 it means the table is a deliberate **demand subset**: `push`, `pop`,
  `call nn`, `bit`, `ld` in most of its forms, `di`/`ei`/`im`/`ldir` are all absent by design,
  because the table was built for the DAC hot path and never grown. 27 % of shipped Z80
  instructions are unpriceable and that is a *scope decision*, not a modelling failure. A
  single "cannot time" number over both processors averages a rounding error together with a
  design boundary, which is why they are never combined here without the per-CPU split above.
* **The instruction/proc split is not the interesting axis; the Layer-A/Layer-B split is.**
  Both D1 and D2 measure the *tables*. Neither measures whether a bound can be *stated*, which
  is what a consumer actually wants and which comes out at 17.6 % / 7.0 %. If the oracle lane's
  waiting item is "can sigil tell us the cycle cost of this routine", the answer is Layer B's
  number, not D1's.
* **`native::shipped_shapes()` is the honest definition of shipped, but not a sufficient one
  for the Z80.** See §4 — the registry it exposes does not reach the sound blob.
