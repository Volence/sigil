# CLOBBER-PAYOFF-MEASURE: what the over-declared clobber sets cost in the shipped ROM

Parcel note. Branch `measure/clobber-payoff`, off master `ea8c64fa`. **This parcel moves no
shipped code.** It answers d-26's size-S question — *what does the over-declaration actually cost
across the shipped ROM, in bytes and cycles* — measured at the CONSUMING end, and it re-verifies
the blocker the row was ordered behind.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | `.target-parcel/release/sigil` in this worktree, md5 `bea7edad80a040cf2f94811b9f0067a1`, `sigil 0.1.0` at branch tip off master `ea8c64fa` |
| reference tree | `/home/volence/sonic_hacks/.aeon-sigil-ref`, aeon `ec640bcf`, 0 dirty paths, READ-ONLY (no build was run in it) |
| corpus (step zero) | PRIVATE clone of `s1disasm` at `f6ece657` in `/home/volence/sonic_hacks/.scratch/clobber/s1disasm`, 0 dirty paths, PREPARED by this tree's own `scripts/corpus-prepare.sh` (8 generated files written, generator exit 0) |
| asl | the corpus's own `build_tools/Linux-x86_64/asl` |
| producer + consumer census | `crates/sigil-harness/examples/clobber_payoff.rs`, written for this parcel and committed with it. Not `crates/*/src/`; no shipped code was altered |
| cycle figures | `sigil_frontend_emp::m68k_cycles::instr_cost` over `sigil_isa::m68k_cycles` — the same table the `@budget` path walk charges. STATIC per-execution costs; no wall clock is quoted anywhere in this note and no emulator was run |
| upper-bound cross-check | `/home/volence/sonic_hacks/.scratch/clobber/textscan.py`, a source-text scan, deliberately generous (see below) |

## Step zero: the blocker DID clear, and the sweep's evidence still did not show it

The queue row was ordered to run only after `AS-S1-DRIVER-SIZE-60X`. The 2026-09-08 sweep recorded
that blocker as landed at `ee136317` on the evidence that `ee136317` is an ancestor of master. That
is evidence the commit exists, not that it closed the blocker. Measured instead:

```
$ sigil sonic.asm                     # prepared s1disasm f6ece657
Uncompressed driver size: 1BBDh bytes.
rc=1, 50 diagnostics
$ ./build_tools/Linux-x86_64/asl -xx -n -q -A -U -E -i . sonic.asm
Uncompressed driver size: 1BC6h bytes.
rc=0
```

**The 60x defect is gone.** The recorded symptom was `73DFDh` against asl's `1BC6h` (a factor of
67) plus a `fatal` at `sound/z80.asm(229)` that truncated the pass so nine of twelve message sites
were never reached. Today sigil reports `1BBDh`, the `fatal` does not fire, and the run reaches
line 231's `message` — the driver-size line is on stdout above. The record's own caveat holds: this
is off a PREPARED tree, and the number moves on a bare one.

**A residual remains and it is a DIFFERENT defect.** sigil is 9 bytes short of asl. Those 9 bytes
are not org accounting; they are four Z80 instructions sigil fails to encode, all of them a
user-defined `function` macro call in an operand:

```
sound/z80.asm(51)   ld  a,zmake68kBank(SegaPCM)&1     -> "trailing tokens in operand"   (2 bytes)
sound/z80.asm(55)   ld  a,zmake68kBank(SegaPCM)>>1    -> "trailing tokens in operand"   (2 bytes)
sound/z80.asm(188)  ld  de,zmake68kPtr(SegaPCM)       -> "trailing tokens in operand"   (3 bytes)
sound/z80.asm(197)  ld  b,pcmLoopCounter(16000)       -> "trailing tokens in operand"   (2 bytes)
```

2 + 2 + 3 + 2 = 9. Proved rather than asserted: replacing exactly those four operands with literals
in the private corpus and re-running gives **`Uncompressed driver size: 1BC6h bytes.`**, byte-equal
to asl, with the four diagnostics gone and every other diagnostic unchanged. The corpus file was
restored to baseline afterwards (0 dirty paths). A first attempt that only PARENTHESISED the calls
changed the error text and not the size, which is what said the defect is the function call and not
the trailing operator.

So the row is unblocked and the measurement below is legitimate. The correct queue text is that
`ee136317` closed the 60x and the truncation; the reported size is still wrong by 9 bytes for an
unrelated reason, which belongs to Z80 operand parsing and not to `org`.

## The mechanism, before any count: a declaration is not code

`clobbers(...)` emits nothing. It is read by the contract analyses, and every caller-side analysis
reads the closure's **`effective`** set, not the declaration:

```
effective(P) = localWrites(P) ∪ ⋃ effective(callees) ∪ indirect bounds − verifiedPreserves(P)
```

Enumerating the CONSUMERS of `declared_clobbers` (the population, not the producer) gives exactly
three: the extern-leaf seed in `closure.rs:264`, the `allowed` set of the under-declaration firing
check in `closure.rs:503`, and the cond-out survives claims in `corpus_contracts.rs:1013-1046`.
Only the first turns a declaration into something a caller can see, and **this corpus has zero
`extern proc` declarations** (`clobber_payoff` counts 0 over 198 files; the one `git grep` hit for
`extern proc` is a comment saying why there is none; positive control, 84 files carry `pub proc`).

So an over-declared register on an ordinary proc reaches no caller-side lint at all. It cannot
cause sigil to ask for a save. The only remaining channel is HUMAN: an author reads the callee's
declaration and writes a `movem` by hand. That channel is real, it is what the ruling describes,
and the instrument that finds its output is sigil's own `[proc.dead-save]` walk, which fires
exactly when a verified save/restore pair brackets calls that all preserve the register per
`effective`. Every save an over-declaration could have motivated is in that list, because an
over-declared register is by definition absent from `effective`.

## The answer, at the consuming end

**In the shipped (non-DEBUG) shapes the cost is ZERO bytes and ZERO cycles.** Not "small": no
instruction in the shipped image is there because a contract over-declares.

`[proc.dead-save]`, per shape, over the whole corpus:

| shape | firings | what they are |
|---|---|---|
| sonic4 plain | **1** | `TestChurnObj_Main` saves `a0` around `AllocDynamic` |
| demo plain / config_b / lean | **1** | the same one |
| sonic4 debug / demo debug / config_a | **10** | that one, plus 9 registers of `Canopy_Persist`'s movem around `Canopy_Fire` |

The single shipped firing is **not** over-declaration. `AllocDynamic` declares
`clobbers(d0/a1) out(a1 if eq) preserves(a0)`: its contract already promises `a0` survives, and the
author saved it anyway. Priced for completeness, deleting it would recover **6 bytes**
(`move.l a0,-(sp)`, `movea.l (sp),a0`, `movea.l (sp)+,a0`, one opcode word each) and **24 cycles**
per expiry-path execution, 36 with the peek — and `TestChurnObj_Main` is a soak-test object entered
by hand, so its dynamic count on an ordinary play frame is zero. Whether that save should go is the
engine lane's call and it is not this row's business.

The nine debug-shape firings are one deliberate `movem.l d0-d7/a0-a6,-(sp)` in `Canopy_Persist`,
whose own comment says it preserves every register on purpose so the enclosing
`Draw_TileRow_FromCache` contract does not widen for a probe. They cost **0 bytes in the shipped
ROM** (the body is `if DEBUG == 1`), and shrinking a movem never recovers a byte in any shape: the
encoding is opcode + mask regardless of population. Per register it is worth 8 cycles on the store
and 8 on the load, measured: `movem.l` of 15 registers costs 128 / 132 cycles and of 14 costs
120 / 124.

### The upper bound that says the zero is not blindness

The dead-save walk is conservative in ways that would hide cost, so the same question was asked a
second way, from the source text, pairing pushes to pops crudely and ignoring comptime gating —
every bias chosen so it OVER-counts. It reports, for `sonic4 plain`, 90 over-declaring callees, 93
call sites to them, and **14** (site, register) pairs where the caller has an over-declared register
on the stack across the call. All 14 were then read by hand, and all 14 fall:

| pair | verdict |
|---|---|
| `TileCache_DecompressBlock` -> `S4LZ_DecompressDict`, `a3` | the "over-declaration" is an ANALYSIS ARTIFACT (see below). `S4LZ_Decompress`, the fall-through successor, opens `movea.l a1, a3`. The save is required |
| `TileCache_FillRow` -> `TileCache_DecompressBlock`, `d2` | not a call-protection save: `move.w d2, -(sp)` is a documented loop-invariant frame slot read back as `0(sp)`, and `d2` is the CALLER's own scratch (`decompose_block` rewrites it) |
| `Tile_Cache_Fill` -> `TileCache_HSlide`, `d0` | same shape: `move.w d0, -(sp) // [sp] = desired_left`, spilled because the caller loads `d0` with HSlide's own argument two lines later |
| `Debug_LabCycleHotkey` -> `Debug_PresetReadout_Blank`, `d1` (x2) | DEBUG-only on both ends, zero bytes in the shipped ROM — and in the DEBUG shape, where the code exists, `d1` IS clobbered and the save is needed |
| `Section_UpdateColumns` -> `Canopy_Probe`, 9 registers | same: `Canopy_Probe`'s body is `if DEBUG == 1`, so in the plain shape it writes nothing and its whole declared set reads over-declared. In the debug shape the residue is `a2/d7`, 0 bytes and 32 cycles per call |

`Debug_LabCycleHotkey` is worth quoting because it is the ONLY place in the corpus where an author
states the mechanism d-26 posits, in their own words: *"The call clobbers d0-d4/a0-a2, so the
sub-index rides the stack across it."* The declaration really did buy that `move.w d1, -(sp)`. It
is inside `if DEBUG == 1`, and in the shape that ships it the clobber is real.

Coverage of the walk, so the 1 has a denominator: 582 procs scanned, **51** contain both a save and
a call (the population a dead save can live in), and **0** contain `link`/`unlk`, which is
`find_dead_saves`' hard bail. Its other bails (an unmodeled `sp` write, a join at differing stack
depth) are not quantified here and are the residual blind spot.

## The producer count, re-derived, and two ways it lies

The row records 76 of 387 procs, 62 on the debug shape. Re-derived at aeon `ec640bcf`: the corpus
now holds **577 procs, 569 declaring clobbers**, and `declared \ effective` is non-empty for

| shape | over-declaring procs | (proc, register) pairs | write nothing in this shape | declare `falls_into` | residue |
|---|---|---|---|---|---|
| sonic4 plain | 90 | 328 | 19 | 11 | **60** |
| sonic4 debug | 71 | 218 | 5 | 11 | **55** |
| demo plain | 101 | 363 | 23 | 11 | 67 |
| demo debug | 82 | 253 | 9 | 11 | 62 |
| config_a | 71 | 218 | 5 | 11 | 55 |
| config_b | 91 | 329 | 20 | 11 | 60 |
| lean | 90 | 328 | 19 | 11 | 60 |

The direction (plain over-declares MORE than debug) reproduces the row's 76 > 62. The absolute
figures do not, and should not: the corpus grew from 387 to 577 procs.

Two of the three columns are not contract looseness at all:

1. **A comptime-empty body.** 19 of the 90 write nothing in the plain shape because their body is
   `if DEBUG == 1`, so the whole declared set reads as over-declared while describing the other
   shape. Tightening any of them would be wrong in the shape that runs them.
2. **`falls_into` is invisible to the closure.** The node builder in
   `corpus_contracts.rs:2206-2231` collects `direct_callees` from CALL and TAIL MNEMONICS in the
   evaluated `CodeBuf` only; a declared `falls_into SUCC` contributes no edge, so the successor's
   writes never enter the falling proc's `effective`. Measured, not merely read:
   `S4LZ_DecompressDict` declares `d0-d3/a0-a4` and the walk reports it never writes 8 of the 9,
   leaving `{a4}` — while `S4LZ_Decompress`, the thing it falls into, declares and performs
   `d0-d3/a1-a3`. 11 procs are in this class in every shape.

**This is the near-miss worth foregrounding.** Had the L fix started on the unnumbered producer
count, `S4LZ_DecompressDict`'s `a3` was on the list to be deleted, and with it the caller's
correct save.

The gap is not only cosmetic. `effective` is what `[proc.clobber-undeclared]` fires on
(`effective \ allowed`), so a proc that falls into a heavy successor and UNDER-declares would not
fire either. That is the destructive direction and it wants its own row.

## What a fix would buy, and it is not a fix to the declarations

Described, not taken — the declarations are aeon's, 89 of their files against 3 of ours.

Correcting the 60-proc residue would recover **0 bytes and 0 cycles** in the shipped ROM. There is
no call site paying for it. What it would buy is what a contract is for: an over-declared register
is a licence nobody is using, and the day the proc starts writing it, nothing fires — the
`allowed` set already covers it. That is a soundness argument for tightening, not a size or speed
one, and it should be argued on its own terms rather than on a byte count that does not exist.

If the item is kept, the cheap and honest first move is not in the declarations at all: teach the
closure the `falls_into` edge. It removes 11 false rows from the producer census, and it closes a
missed-firing direction in the shipping clobber lint. It is sigil's own code, so it does not
serialize behind the byte chain.

## What in the brief and in this tree turned out to be wrong

1. **The sweep's blocker claim was right in its conclusion and wrong in its evidence.** `ee136317`
   did close the 60x and the truncation; the sweep proved only that the commit is an ancestor of
   master. The row is runnable, and it was runnable for a reason nobody had checked.
2. **The blocker did not close COMPLETELY.** sigil still reports the driver 9 bytes short. Anyone
   reading "landed" as "sigil now agrees with asl on that line" would be wrong.
3. **`dead_save_corpus.rs`'s own doc comment is stale.** It says "The live worklist is now 1 firing
   (TestChurnObj_Main/A0) in every shape". It is 10 in `sonic4 debug`, `demo debug` and `config_a`.
   The test asserts `widest >= 1`, so a nine-firing widening never disturbed it — a tolerant floor
   that cannot notice growth is the same shape as a count-free gate, in the other direction.
4. **d-26's framing that the cost "lands on every call site" does not hold in this corpus.** The
   declaration reaches no caller-side analysis, because `effective` is computed and there are no
   extern leaves. The only channel is a human reading the contract, and it has one instance, in
   debug-only code, where the clobber is real in the shape that ships it.
