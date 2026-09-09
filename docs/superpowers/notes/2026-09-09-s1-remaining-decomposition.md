# What Sonic 1 still refuses to assemble: six root causes behind 46 diagnostics

2026-09-09, sigil overseer seat, measured in the main checkout before any parcel was dispatched.
**This is the sizing the queue row `S1-MISSING-DIRECTIVES` says must be re-derived at dispatch
rather than read off a note. Re-derive it again before quoting it later.** The count moves the
moment any of the six lands.

## Provenance, because a count with no instrument is not a measurement

| Instrument | Identity |
|---|---|
| sigil | `.target-s1/release/sigil`, `sigil 0.1.0 (bd7fce7b)`, md5 `ea2ea282285b89ad84b34801c9a20fcc`, 8191656 B, built 2026-09-09T07:12:27Z from master `bd7fce7b`, published (contained in `origin/master` at build time) |
| build | `CARGO_TARGET_DIR=.target-s1 cargo build --release --bin sigil`. The shared `target/release/sigil` was md5 `49ecc532e0b133ab0eab9447e071805c` before and after, so no lane's freeze was relinked. |
| corpus | `/home/volence/sonic_hacks/.scratch/s1-profile/s1disasm`, rev `f6ece657`, entry `sonic.asm`. Tracked paths clean; the 2 dirty paths the runner reports are untracked (`.aurora/`, `Test.hsproject`) and reach no assembly source. |
| generated includes | READY, 4 of 4 named paths present, derived from the corpus's own sources by `scripts/corpus-baseline.sh`, which refuses an unprepared tree. So this is a baseline and not an absent-generator artifact. |
| runner | `scripts/corpus-baseline.sh --label s1-bd7fce7b`, streams at `.scratch/s1-profile/baseline-bd7fce7b/` |

`sigil exit 1`, **46 diagnostics over a stream of 46 lines, all error level, 46 of 46 parsed**.

## The class table the runner prints, and why it is not the useful decomposition

```
error  18  unexpected character
error  11  `X` is not a recognized N mnemonic
error   6  bad immediate expression
error   6  case needs a string literal
error   2  switch needs a string expression
error   2  unresolved if condition
error   1  unknown directive or mnemonic `X`
```

Seven message classes. **They are not seven causes.** The mnemonic class splits into two unrelated
features, the switch and case classes are one feature, and the same missing word (`listing`) shows
up under two different messages because the 68000 and Z80 surfaces refuse it at different sites.
Sizing off the message classes would have produced seven parcels, two of them halves of each other.

## The six root causes, by enumeration rather than by count

| # | Root cause | rows | file(s) |
|---|---|---|---|
| A | `listing` and `page` unimplemented, on BOTH CPU surfaces | 3 | `MacroSetup.asm`, `sound/z80.asm` |
| B | `charset` unimplemented | 9 | `sonic.asm` |
| C | the `[count]value` duplicate-operand syntax in `dc` | 18 | `MacroSetup.asm(98)` |
| D | a multi-character string literal does not evaluate as an integer | 6 | `_incObj/82, 83 SBZ Eggman Cutscene and Crumbling Floor.asm` |
| E | `switch` / `case` accept only string expressions; AS also takes integers | 8 | `sound/_smps2asm_inc.asm` |
| F | `DEFINED(symbol)` unimplemented, so the `if` guarding it cannot decide | 2 | `sound/_smps2asm_inc.asm` |

3 + 9 + 18 + 6 + 8 + 2 = 46, and the set is written out so it cannot drift into a bare "six".
**Five distinct files.** Every one of C's 18 rows is the single line `MacroSetup.asm(98)`,
`\tdc.ATTRIBUTE\t[count]value` inside the `dcb` macro, so 18 rows is 18 expansions and not 18 sites.

Root-cause attributions worth stating as hypotheses rather than facts, since each was read off the
source line and not proved by a probe: D is `#"SW"` and `#"GO"` used as 16-bit command tags, which
AS packs big-endian; F's two rows are both `if (MOMPASS=1)&&(DEFINED(loc))` inside macro bodies, so
`MOMPASS` may or may not be implicated with `DEFINED`.

## The relayed figure this disagrees with

The hub relayed the owner's overnight directive with "46 remaining complaints, ten classes, seven
files, nine of them missing directives". The total agrees exactly. **The decomposition does not: the
runner reports seven message classes, this note reports six causes, and the population is five
files, not seven.** Recorded rather than smoothed over, because a figure taken from a row instead of
re-measured at the quoting date is a defect this lane has banked twice.

## The measurement's own blind spot, which is larger than the measurement

**46 is what the frontend REFUSED. It is not what is left to do.** A complaint count cannot see a
silent wrong answer and cannot see anything that fails at link, because a run that dies in the front
end never reaches layout, link or flatten. Sonic 1 does not assemble today; clearing all six causes
is necessary for that and is not sufficient for it, and **each fix will expose diagnostics that were
previously unreachable, which is the expected shape of progress rather than a regression.** The
discriminators that see what the count cannot, and which each parcel reports beside it: the
per-class decomposition in both directions, the emitted bytes for any construct newly accepted, and
whether the run reaches link at all.

D is the dangerous one by this test. If a string literal evaluates to the wrong integer, all six of
its diagnostics vanish, the count improves, and the ROM is wrong. Its parcel checks the emitted
bytes at all six sites against the reference assembler rather than the disappearance of the
complaint.

## How it was split into parcels

Wave 1, dispatched 2026-09-09T07:15Z, two agents in isolated worktrees with separate scratch
directories and separate cargo target directories:

- `S1-DIRECTIVES-LISTING-AND-DUP-OPERAND` = A + C, 21 rows. Both are surface acceptance work.
- `S1-EXPR-CHAR-LITERAL-AND-DEFINED` = D + F, 8 rows. Both are expression-layer work.

Wave 2, not dispatched: E (`switch`/`case` over integers) and B (`charset`). B is held back
deliberately: it remaps character encoding, so it composes with D, and the D parcel was asked to say
which way it left that seam rather than to implement B.

**The aeon byte gate is owed by both parcels and by neither agent.** `sigil-frontend-as` is on
aeon's shipping build path (their `build.sh` routes three residual `.asm` files through it), so a
change here can move aeon bytes. No agent was given an aeon tree: two agents building four shapes in
one reference tree are each other's concurrent writer, and this lane has already attributed its own
write to a peer once. The proof runs once, at landing, on the merged tree.
