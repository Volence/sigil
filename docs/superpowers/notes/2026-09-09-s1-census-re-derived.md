# Sonic 1 re-measured: 17 diagnostics, two causes, and the count was never the thing to watch

2026-09-09T14:36Z, measurement parcel, branch `s1-census-2026-09-09`. **No sigil source
was changed.** Machine `up 2 days, 14:57`, load average 20.0 rising to 23.5 across the
run; every wall-clock figure below sits under that load.

This supersedes the census half of `2026-09-09-s1-remaining-decomposition.md`. That note
recorded **46 diagnostics, six causes, five files** at 07:21Z. **The present figure is 17
diagnostics, two causes, two files.** Everything in that note and in
`2026-09-04-s1-path-to-rom.md` was treated here as a claim to re-derive, and where the
re-derivation disagrees the disagreement is stated with the command.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | `.target-census/release/sigil`, `sigil 0.1.0 (c6179e8a)`, md5 `1b0357ce26db9ae79473244cc219aa01`, 8,230,608 B, built 2026-09-09T14:35:44Z from `c6179e8a`, which is `origin/master` |
| build | `CARGO_TARGET_DIR=<this worktree>/.target-census cargo build --release --bin sigil`. The shared `target/release/sigil` was md5 `49ecc532e0b133ab0eab9447e071805c` before the run and `49ecc532e0b133ab0eab9447e071805c` after, so no lane's md5-pinned freeze was relinked. |
| corpus | `/home/volence/sonic_hacks/.s1census-2026-09-09/s1disasm`, a **pristine detached worktree** of `/home/volence/sonic_hacks/s1disasm` at `f6ece657c1cf253404312137dfcb8ec15fa42318`. `git status --porcelain` reports **0 paths**, checked before the census and again after. Entry `sonic.asm`. |
| corpus, why not the live tree | the live checkout at the same revision reports 2 modified tracked paths (`artnem/GHZ Bridge.nem`, `artnem/Signpost.nem`) plus 2 untracked. It is not a reference and was not used. |
| generated includes | READY, 4 of 4 named paths present, written by the corpus's own `build.lua` generator half via `scripts/corpus-prepare.sh` (8 files). This is a baseline, not an absent-generator artifact. |
| oracle | `build_tools/Linux-x86_64/asl`, `Macro Assembler 1.42 Beta [Bld 212] (x86_64-unknown-linux)`, md5 `61e672562465725a8c102288a7da9098` |
| runner | `scripts/s1-census.sh`, committed with this note; streams under `/home/volence/sonic_hacks/.s1census-2026-09-09/out*/` |

**The oracle assembles this corpus cleanly.** `asl -xx -n -q -A -L -U -E -i . sonic.asm`
exits **0**, writes nothing to stderr, has **0** lines matching `error|warning` in its
stdout, and produces `sonic.p` at 529,364 bytes. So the corpus is sound and every one of
sigil's complaints below is sigil's own, not the corpus's.

## The census

`sigil exit 1`, **17 diagnostics over a stream of 17 lines, all error level, 17 of 17
parsed** as `file(line): level:` with no unclassified remainder.

```
error   9  `X` is not a recognized N mnemonic
error   6  case needs a string literal
error   2  switch needs a string expression
```

Per-file spread, and it is now only two files:

```
9  sonic.asm
8  sound/_smps2asm_inc.asm
```

### The two causes

| # | Root cause, in plain language | the construct | rows | file |
|---|---|---|---|---|
| B | sigil has no character-encoding table, so it cannot honour a source file that says "when you write text from here on, spell it in the game's font rather than in ASCII" | `charset lo,hi,base` (8 range forms) and bare `charset` (the reset) | 9 | `sonic.asm` 2616-2624, 2675 |
| E | sigil's multi-way selector compares only text, and this one selects on a number | `switch EXPR` / `case N` / `elsecase` / `endcase` over an integer | 8 | `sound/_smps2asm_inc.asm` 63, 64, 67, 88, 89, 92, 96, 113 |

9 + 8 = 17. The E rows are 2 `switch` heads and 6 `case` heads across **2** blocks;
`elsecase` and `endcase` draw no diagnostic. `Macros.asm(318) switch "btn"` is a STRING
switch and is accepted, which is the control showing the `switch` machinery itself works.

## The set diffs against the standing note, both directions

Line sets, sorted, diffed both ways (`scripts/corpus-baseline.sh --compare`):

```
lines only in the NEW run:  0
lines only in the OLD run: 12
```

**Nothing newly appeared. No class rose. No new class appeared.** The 46 to 17 move is a
pure subset shrink, which is the one shape that cannot be hiding a swap: a set that only
loses members cannot have gained a wrong one.

The 12 lines that went, at 29 counts, and the cause each belonged to:

| gone | counts | standing note's cause |
|---|---|---|
| `MacroSetup.asm(7)` `listing`, `(8)` `page`, `sound/z80.asm(11)` `listing` | 3 | A, `listing`/`page` on both CPU surfaces |
| `MacroSetup.asm(98)` unexpected character | 18 | C, the `[count]value` duplicate-operand syntax |
| `_incObj/82, 83 SBZ …asm` 133/163/185/255/293/312 bad immediate | 6 | D, a multi-character string literal as an integer |
| `sound/_smps2asm_inc.asm(238)`, `(282)` unresolved if condition | 2 | F, `DEFINED(symbol)` |

**Four of the standing note's six causes are closed, not one.** The brief said "at least
one"; the measurement says A, C, D and F are all gone and only B and E remain.

### The unresolved-symbol NAME sets, and why they are empty

```
population: 46 line(s) before, 17 after
before-only (0): []
after-only  (0): []
in both     (0)
```

**This emptiness is real and it is also uninformative, and the two are different claims.**
It is real: a canary of the same class was planted and found.

```
$ cat canary.asm
	cpu 68000
	dc.b NeverDefinedAnywhereCanary
	end
$ sigil canary.asm
canary.asm(2): error: unresolved symbol `NeverDefinedAnywhereCanary` for fixup in section sec0 at offset 0
  input reaching the matcher: 1 line(s)
  names found: ['NeverDefinedAnywhereCanary']
```

The instrument fires and the input arrived, so the zero is not a feed failure. It is
uninformative because **no run over this corpus reaches the stage that produces those
names**, so the name set is structurally empty on both sides and the diff that carried
information here was the LINE set. Reported rather than dropped, because a reader who
sees only "before-only 0, after-only 0" would read a strong clean where there is none.

## Does the run reach link? No, and not even with the front end stubbed clean

`crates/sigil-cli/src/main.rs:315` : the front end's `Err(failure)` arm calls
`process::exit(1)`. `Failure` carries no partial module, so with 17 diagnostics standing,
`resolve_layout`, `link`, `check_image_bounds` and `flatten` are all unreached. **Any
single diagnostic is as fatal as all 17**, which is why the count ranks nothing.

The interesting answer is what happens when the front end is made clean. `scripts/lib/s1_stub_bc.py`
stands in for B and E in a scratch COPY (never the pristine tree) and the run is measured
again. Stub E is byte-neutral by construction, AS's `switch` being an if/elseif chain;
**stub B is not byte-neutral** and no byte claim survives it.

```
stub E  sound/_smps2asm_inc.asm  12 line(s)   switch/case -> if/elseif/else
stub B  sonic.asm                 9 line(s)   charset commented out
Macros.asm identical, the string switch left alone
```

Result: **the front end goes completely clean, the run walks past it, and dies one stage
later with exactly one diagnostic.**

```
error 0 -> 1  sections `X` [0x0, 0x2CA) and `X` [0x0, 0x1BC6) overlap in the image (colliding pins)  <== APPEARED
sonic.asm(81): error: sections `sec0` [0x0, 0x2CA) and `sec0#2` [0x0, 0x1BC6) overlap in the image (colliding pins)
```

That message is emitted at `crates/sigil-link/src/relax.rs:321`, inside `resolve_layout`
(`relax.rs:648`), which `main.rs` calls **before** `sigil_link::link`. So even with the
whole front end stubbed silent, **the run reaches LAYOUT and still does not reach LINK.**

The collision is the Z80 sub-assembly's phased block: `sound/z80.asm(9) !org 0` inside
`save`/`restore` puts the driver at LMA 0, where the 68000 vector table already is.
`0x1BC6` is the driver's real size, confirmed independently by both assemblers printing
the same line on stdout: `Uncompressed driver size: 1BC6h bytes.`

**`1` is a floor, not a population.** The overlap scan `return`s on the first colliding
pair (`relax.rs:314-327`), so it reports one and stops. How many sections actually collide
is unmeasured and is not measurable without changing sigil, which this parcel may not do.

### A probe that refuted itself, reported because it teaches something

A second stub level additionally commented out `!org 0`, expecting to see what stands
behind the overlap. It produced **11 diagnostics, all of them the stub's own fault**: 10
`operand N out of range` in `sound/z80.asm` with values like 470702 (`$72E6E`, a 68000 ROM
address) and the corpus's own guard firing, `The driver is too big; the maximum size it
can take is 1FFCh. It currently takes 74A42h bytes.` Without the `!org`, every Z80 label
is a 68000 address and nothing in the driver fits a 16-bit operand. **`!org 0` is
load-bearing and cannot be stubbed past**, so the depth beyond the overlap is not
measurable from here, and the 11 must not be read as a population. This is why the
committed script's depth probe stops at B and E.

## What each cause is worth, measured where it could be measured

### B, `charset`: 9 diagnostics and 504 ROM bytes. Both measured.

The oracle's listing gives the extent directly rather than by inference:

```
2615/  359E :                     LevelMenuText:
2626/  359E : 1722 1515 1EFF      		dc.b "GREEN HILL ZONE  STAGE 1"
2675/  3796 :                     	charset	; reset charset to default
```

`$3796 - $359E = $1F8 = **504 bytes**`, and this re-derives the 2026-09-04 note's 504
figure by a different route (listing addresses rather than a ROM diff), so that figure
stands. The byte column shows the feature is not cosmetic: `G` is `$17`, not `$47`.

The cross-corpus figure (82 `charset` sites in s2disasm) is **inherited from the
2026-09-04 note and was NOT re-derived here.** Flagged as such; do not quote it as this
parcel's measurement.

### E, integer `switch`/`case`: 8 diagnostics, and 670 uses of the symbols it binds.

The oracle resolves both blocks to `case 1` and gives the required bindings:

```
(2) 65/ 745DC : =$1..$6      enum     fTone_01=$01,fTone_02,fTone_03,fTone_04,fTone_05,fTone_06
(2) 66/ 745DC : =$7..$9      nextenum fTone_07,fTone_08,fTone_09
(2) 90/ 745DC : =$81..$83    enum     dKick=$81,dSnare,dTimpani
(2) 91/ 745DC : =$88..$8B    enum     dHiTimpani=$88,dMidTimpani,dLowTimpani,dVLowTimpani
```

Uses of those 16 names **outside the file that defines them: 670, across 22 files**
(`grep -rohwE … --exclude=_smps2asm_inc.asm | wc -l`). Every one is an operand in music or
DAC data.

**"670 uses" is a measurement; "670 ROM bytes" is an estimate** and is flagged as one: the
uses are `dc.b`-family operands so one byte each is the expected mapping, but no byte diff
was run to confirm it and this parcel produced no ROM.

### The Z80 phased-block overlap: 1 diagnostic, floor. Bytes unknown.

Cannot be sized until B and E land, because until then it is unreachable. What is known:
it is `resolve_layout`, it is `sound/z80.asm(9) !org 0` under `save`, and the `!org` cannot
be removed.

## Recommended fix order

**B and E are independent** (different files, different subsystems, no shared code path),
so they can go in parallel. If they go in series, **E first**, and the reason is not size:

1. **E, integer `switch`/`case`.** 8 rows, 1 file, 2 blocks. Put it first because it is the
   one whose wrong implementation this corpus **cannot detect** (see below), so it needs
   attention while attention is on it.
2. **B, `charset`.** 9 rows, 1 file, 504 measured bytes, and the better cross-corpus value
   on the inherited S2 figure.
3. **The Z80 phased-block overlap.** Not a queue item yet; it is what B and E are standing
   in front of. Size it after they land, by re-running `scripts/s1-census.sh`.

Do **not** size step 3 off "1 diagnostic". That 1 is a first-hit report from a
short-circuiting scan.

## Which of these can fail SILENTLY, and this is the headline

The standing note flagged **D** as the dangerous one. D is closed. **The crown has moved to
E, and E's hazard is stronger than D's ever was, because the trap is already latent in the
code that would be edited.**

`eval.rs::exec_switch` collects arm heads, and a `case` whose argument is not a string
becomes `Some(idx, None)`, the **same shape as `elsecase`**. The selection loop then reads
`None => true`, "default, taken if reached". So an integer `case` head is currently treated
as the default arm, and **the FIRST one wins.** Proven, not read off the source:

```
$ cat switcharm.asm            $ asl … switcharm.asm (V = 2)
	cpu 68000                     3/ 0 : =$2       switch V
V = 2                             4/ 0 : =>FALSE      case 1
	switch V                      6/ 0 : =>TRUE       case 2
		case 1                    7/ 0 : 22               dc.b $22
Sel = 1
		case 2                  $ sigil switcharm.asm
Sel = 2                         ARM TAKEN: Sel=1
		elsecase                switcharm.asm(3): error: switch needs a string expression
Sel = 3                         switcharm.asm(4): error: case needs a string literal
	endcase                     switcharm.asm(6): error: case needs a string literal
	message "ARM TAKEN: Sel=\{Sel}"
```

asl takes `case 2` and emits `22`. sigil executes `case 1`. Today that is loud, because the
`switch` head is refused as well. **The failure mode is a fix that makes the refusals stop
without wiring the integer comparison into the arm test.** On Sonic 1, `SonicDriverVer = 1`
and `case 1` is the first arm of both blocks, so that fix would emit **zero diagnostics and
the correct 670 bytes.** The census would read 9, then 0, and every byte gate this corpus
can offer would pass. It is the maximally reassuring wrong answer.

**The E parcel therefore owes a NEGATIVE probe with the matching arm NOT first**, of
exactly the shape above, run against the oracle. Sonic 1 cannot be that probe.
`.s1census-2026-09-09/probe/switcharm.asm` and `switchint.asm` are the two ready-made ones.

**B has its own, weaker silent mode.** sigil's own comment at `expr.rs:73-83` names the
complete population a `charset` implementation must reach: `expr.rs::string_to_int` (a
string in an EXPRESSION) and `eval.rs::directive_db` (a string in a `dc` directive), "and
the two are the complete population". **Sonic 1 exercises only the second.** Inside the
charset region, 35 lines carry a quoted string and every one that is data is a `dc.b`; the
2 that are not are `warning` message text. So a fix that reaches only `directive_db`
produces the right 504 bytes here and is undetectable on this corpus. Secondly, the bare
`charset` at 2675 is a RESET; 0 `dc.b` string lines follow it in `sonic.asm`, but 45 exist
elsewhere in the corpus, so an unimplemented reset is invisible in the obvious place and
visible somewhere nobody is looking.

**The layout overlap is loud by nature** and is the one item here with no silent mode.

## Where the standing notes are wrong, and where this brief was

1. **`2026-09-09-s1-remaining-decomposition.md`: 46 is stale, and so is its shape.** 17 now,
   two causes not six, two files not five. The note said to re-derive rather than quote it;
   this is that re-derivation, and the note was right to say so.
2. **`2026-09-04-s1-path-to-rom.md`, booked item "the link diagnostic has no `file(line)`":
   CLOSED.** The canary above prints `canary.asm(2): error: unresolved symbol …`. That
   booked item can be struck.
3. **`2026-09-04`'s F11 has changed symptom, stage and location.** It was
   `sound/z80.asm(9): error: org target precedes the current phase base`, a front-end
   refusal at the offending line. It is now a LAYOUT diagnostic reported at
   `sonic.asm(81)`, a line reading `dc.l v_systemstack&$FFFFFF` that has nothing to do with
   it. Same root cause, later stage, worse location. That is the same span-attribution
   family as that note's still-open `sonic.asm(1)` item, and it is now the shape a reader
   will meet first.
4. **The brief said "at least one of those causes has been closed".** Four were: A, C, D
   and F.
5. **The brief said the number is "expected to RISE before it falls", and that is right
   about the mechanism and wrong about the granularity.** Across the four fixes measured
   here it did not rise once: 46 to 38 to 17, monotone, with **zero** newly-appearing lines
   at each step. The rise happens at a **STAGE boundary**, not per fix, and it happened
   exactly once and exactly there: going front-end-clean surfaced a class from
   `resolve_layout` that no front-end run could ever have shown. Stated per-fix, the
   expectation would have made a genuine monotone fall look suspicious.
6. **The brief said "one of the six was already flagged" as silently-failing.** True of D,
   which is closed. The flag now belongs to E, and E's version is worse: the wrong-arm
   behaviour is already implemented and already observable, and Sonic 1 is structurally
   unable to detect the wrong fix.
7. **The brief's instruction to diff the unresolved-symbol NAME sets was vacuous on this
   corpus** and would have been reported as a clean result by anyone who did not canary it.
   Both sides are structurally empty because nothing reaches link. The instruction is still
   right; it just did not bite here, and the reason it did not bite is itself the finding.

## Reproducing this

```
CARGO_TARGET_DIR=<somewhere on disk, NOT the shared checkout's target/> \
  cargo build --release --bin sigil
scripts/s1-census.sh --sigil <that binary> \
                     --work /home/volence/sonic_hacks/.s1census \
                     --rev f6ece657 \
                     --compare <a previous run's .err>
```

The script refuses a dirty corpus, refuses an unprepared one, refuses a stub that matched
zero lines, canaries the empty-set instrument, and prints the depth probe with the warning
attached. Read its output; do not quote this note's 17.
