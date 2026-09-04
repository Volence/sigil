# `~~` is asl's logical NOT — a silent wrong answer that deleted 518 jump-table entries

`~~x` was lexed as two `Tilde` tokens and folded as `!!x`. That cancels. sigil
returned the operand unchanged, with no diagnostic, at every one of `s2disasm`'s
96 sites.

## Provenance

| | |
|---|---|
| corpora | `s2disasm` `e45ebf3` (entry `s2.asm`), `s1disasm` `f6ece657` (entry `sonic.asm`), both in detached worktrees, removed after the run |
| oracles | BOTH shipped `asl` builds — `s2disasm/build_tools/Linux-x86_64/asl` (flamewing fork) and `s1disasm/build_tools/Linux-x86_64/asl` (upstream), each `Macro Assembler 1.42 Beta [Bld 212]` |
| flags | `-xx -n -q -A -L -U -i .` on every invocation |
| sigil | before: master `b5e7714d`; after: `parcel/as-tilde-logical-not` tip, `--version` closure-revision `922b4379` |
| probes | `p1`…`p8` beside this note in `2026-09-03-tilde-tilde-probes/`, with `run.sh`, `diff_bytes.sh` and `mutate.sh` |

**The fork/upstream control is clean.** An operator's semantics could plausibly
differ between the two builds, so the whole semantics probe was run through both.
They agree on every byte column AND every error line, including the three error
cases below. Nothing in this note depends on which build you ask.

## The operator

`~~` is ONE greedy token, not two `~`. Every row is a byte column off a listing.

```text
  12/    100C : 01                  	dc.b	~~0
  13/    100D : 00                  	dc.b	~~1
  14/    100E : 00                  	dc.b	~~5
  15/    100F : 00                  	dc.b	~~-1
  16/    1010 : 00                  	dc.b	~~$FF
```

**`~~x` is `1` when `x` is zero and `0` otherwise.** `~` is untouched: it is still
one's complement (`~0` = `FFFFFFFF`, `~$0F` = `FFFFFFF0`).

**Maximal munch.** `~~~x` is `~~` then `~` — the logical NOT of the complement,
so zero for every operand but `-1`:

```text
  19/    1011 : 00                  	dc.b	~~~0
  20/    1012 : 00                  	dc.b	~~~1
  21/    1013 : 00                  	dc.b	~~~5
```

**Precedence: the atom tier, tighter than every binary operator.** One row per
tier, because no single row separates them:

```text
  38/    101E : 02                  	dc.b	~~0+1        ← (~~0)+1
  39/    101F : 01                  	dc.b	~~1+1
  45/    100D : 03                  	dc.b	~~0*3
  58/    1016 : 01                  	dc.b	~~0=1        ← (~~0)=1
  44/    100C : FF                  	dc.b	-~~0
```

**Result type: an integer, indistinguishable from a boolean.** asl's booleans and
integers interconvert freely in both directions, so there is no type layer to
model. `dc.b ~~0|2` is `03`, `dc.b (~~0)=(1=1)` is `01`, `dc.b ~~(1=1)` is `00`,
and `dc.l ~(1=1)` is `FFFFFFFE`.

**The connectives take plain integers, not booleans.** `dc.b 2||0,5||3,2&&1` is
`01 01 01` and `dc.b 0&&5` is `00`; `if 5` is `=>TRUE`. So `~~` is NOT a
coercion the corpus needs in order to use `||` — it is there for its own meaning.
Composition is ordinary: `dc.b ~~0||~~1` is `01`, `dc.b ~~1&&~~1` is `00`.

**One type rule, and it is the only one.** The operand must be an INTEGER:
`dc.b ~~0.0` is asl error #1134, *expected integer, but got floating point
number*. sigil's AS front end keeps floats out of `sigil_ir::Expr` entirely
(`eval_float` is a separate tree with no `~` at all), so this arrives already
matched, for a different reason than asl's.

### Three shapes asl refuses that sigil accepts

All three are corpus-unreachable — booked, not gated.

| source | asl | sigil |
|---|---|---|
| `~-1` / `~ -1` | error #1110, *expected one argument but got 0* | folds to `0` |
| `~~~~0` | error #1110, *expected one argument but got 2* | folds to `0` |
| `~ ~ 0` | error #1110, *…but got 2* | folds to `0` |

The mechanism is asl's operand splitter, and the caret spans tell you which:
for `~-1` the caret is the single `~` (the split happened at the binary minus,
leaving a bare `~` as the left operand — zero arguments); for the other two the
caret spans the whole operand (the split happened at the second tilde group,
leaving two). sigil's `parse_atom` recurses instead, which is why it is quiet.
`s2disasm` writes none of these shapes: zero `~-`, zero `~~~`, zero `~ ~`.

## Why this is code generation, not a wrong number

`s2disasm` spells "if this flag is OFF" as `if ~~FLAG`, and every flag it does
that to is 0:

| | |
|---|---|
| `s2.asm:27` | `fixBugs = 0` |
| `s2.asm:40` | `removeJmpTos = 0\|(gameRevision>=2)\|allOptimizations` = `0\|0\|0` |
| `s2.asm:49` | `useFullWaterTables = 0` |
| `s2.asm:68` | `FixMusicAndSFXDataBugs = fixBugs` |
| `s2.sounddriver.asm:8` | `FixDriverBugs = fixBugs` |
| `s2.sounddriver.asm:9` | `OptimiseDriver = 0` |

Reading `~~0` as `0` takes the **wrong arm of all 96**. The largest is
`jmpTosInternal`, whose entire body is `if ~~removeJmpTos` — so sigil was not
mis-assembling the jump tables, it was **deleting them**.

## The corpus population

99 occurrences over 96 lines, four files, all in `s2disasm`:

| file | lines | occurrences |
|---|---|---|
| `s2.sounddriver.asm` | 59 | 60 (`3253` writes `if (~~OptimiseDriver)&&(~~FixDriverBugs)`) |
| `s2.asm` | 34 | 34 |
| `s2.macrosetup.asm` | 2 | 4 (`245` chains three through `\|\|`) |
| `sound/music/9E - Credits.asm` | 1 | 1 |

**Zero in `s1disasm`. Zero in aeon.** Both re-counted here over the whole tree,
not just `.asm`.

## The brief's headline mechanism was right about the language and wrong about the corpus

The `_btst` macro at `s2.macrosetup.asm(245)` really does choose between `tst.b`
and `btst` (and `_beq`/`_bne` then choose `bpl`/`beq` and `bmi`/`bne`) on a
`~~`-valued predicate. But **no corpus site is affected**, and the reason is the
`||` chain:

```
last_btst_converted := ~~chkop("x",A) || ~~chkop("x",B) || ~~chkop("x",C)
```

`chkop(op,ref)` is 1 when `op` does NOT start with `ref`. All 81 `_btst` call
sites pass an operand matching exactly one of the three refs, so the correct
reading gives `1||0||0` = 1 and the broken reading gives `0||1||1` = 1. **Both
select `tst.b`.** The inversion only shows for an operand matching none of the
three, which the corpus never writes.

Measured, not reasoned: a probe carrying the six real operand shapes is
byte-identical to `asl` with the OLD binary (`3dea1e5d/4136` both sides). Add one
`_btst #4,status(a0)` and the old binary diverges.

## The byte sweep, and its proof of sensitivity

`diff_bytes.sh` assembles one source with `asl`+`p2bin` and with `sigil`, and
compares CRC32/size. It is worth nothing unless it can fail, so each row was run
with BOTH binaries:

| probe | with `sigil-BEFORE` | with `sigil-AFTER` |
|---|---|---|
| `p3` — the whole semantics grid + both corpus `if` shapes | **DIFFER** `469203ee/4171` vs `f9dbb8f6/4172` | **SAME** |
| `p4` — `_btst`/`_beq`/`_bne`, six real shapes + one unreachable | **DIFFER** | **SAME** |
| `p4a` — the six real shapes ALONE | SAME | SAME |
| `p5b` — `jmpTos`/`jmpTos0`, three corpus shapes | **DIFFER** `eb759384/4112` vs `77236b76/4148` | **SAME** |
| `p7` — `~~` in a macro BODY | **DIFFER** | **SAME** |
| `p8` — `~~` in a macro ARGUMENT, incl. a string embed | — | **SAME** |

`p5b` is the one that names the damage: **36 bytes of jump table missing**, and
the fix restores them byte-exactly against `asl`.

## Both corpora, before and after

Same trees, same entry points, same binary path, master `b5e7714d` vs this
branch.

### Sonic 1 — the prediction, stated before the run, and the result

> S1 has zero `~~` sites, so the expected S1 movement is zero, in both the total
> and the sets.

**Held, and more strongly than the prediction claimed.** The two diagnostic
streams are identical BYTE FOR BYTE — 368 lines each, same order, same lines.
Not merely the same totals and the same class sets: `cmp` reports no difference
at all.

**And the S1 walk is not truncating, which is what would make that vacuous.** An
indented `zqpmark_tail_of_last_include` appended to the END of `s1.sounddriver.asm`
— the LAST include of `sonic.asm`, at line 5229 of 5237 — is reported, with the
patch shown landed (`git status` dirty) and restored from the committed baseline.
The same marker at COLUMN 0 is silent, and correctly so: a column-0 head under
`CPU Z80` is a label, which `asl` accepts too. The first attempt used column 0
and read as a truncated walk; it was the instrument, not the corpus.

### Sonic 2 — 8,932 → 9,539, `+607`, nothing lost

No class fell. No line disappeared (`comm` both directions: 607 added, **0**
removed). Four classes rose:

| before | after | delta | class |
|---|---|---|---|
| 0 | 518 | **+518** | `bad absolute address expression` |
| 131 | 215 | **+84** | `bad word expression` |
| 2622 | 2624 | +2 | `bad operand expression` |
| 2307 | 2309 | +2 | `expected mnemonic, directive, or label` |
| 3373 | 3374 | +1 | `unresolved symbol` |

**Every rise is code sigil had NEVER ASSEMBLED**, at five sites, and each
reconciles arithmetically:

- **518** at `s2.macrosetup.asm(304)` — `jmp (extractJmpToName("op")).l`, the body
  of `jmpTosInternal2`, reachable only through `if ~~removeJmpTos`. The count is
  the number of `jmpTo` entries actually assembled: 522 written, of which 7 sit
  inside `if gameRevision=0`/`else` at `s2.asm:52908` and `gameRevision` is 1, so
  515 ungated + 3 from the taken arm = **518**. The diagnostic itself is a
  PRE-EXISTING gap made visible: sigil cannot fold a user `function` returning a
  string into a `.l` absolute address.
- **84** at `s2.sounddriver.asm(1388)` — `zMakeFMFrequenciesOctave`, invoked for
  octaves 1–7 under `if ~~OptimiseDriver` (octave 0 is unconditional and was
  already diagnosed). 7 × 12 notes = **84**.
- **4** at `s2.asm(17578,17580,17582,17594)` — inside `if ~~fixBugs` at
  `s2.asm:17568`, the screen-shake fix block.
- **1** at `s2.asm(61565)` — inside `if ~~fixBugs` at `s2.asm:61564`.

518 + 84 + 4 + 1 = **607**, reconciled with no remainder.

**Half of my own prediction was wrong.** I predicted the sets would move in both
directions, because the `else` arms sigil had been taking would stop being
assembled. They moved in one: the `else` arms were diagnostic-free, so nothing
was lost.

### The honest framing

**Sonic 1 is the target corpus and this row does not touch it.** The S2 number
went UP. The value here is that the assembler was wrong — for any source using
the operator — and that a construct which chooses which instructions exist now
chooses correctly. The +607 is previously-invisible ground becoming visible, and
the two gaps it exposes (string-returning `function` in a `.l` address; the
float-heavy `zMakeFMFrequency` table) are real work now countable instead of
silently skipped.

## Verification

- **11 new `#[test]` rows** in `crates/sigil-frontend-as/src/eval.rs`, every
  expectation a byte column read off an asl listing.
- **Five red-first mutations** (`mutate.sh`), each applied to a COMMITTED
  baseline with the patch read back from disk and the file proven dirty, each
  restored with `git checkout --` and re-verified clean: un-munch `~~` into two
  `Tilde`; fold `LogNot` as `!v` (the original defect); fold it inverted
  (`v != 0`); render `~~` back as one `~`; give `~~` a full binary operand
  instead of an atom.

### The mutation that came back GREEN, and what it uncovered

Breaking `punct_str`'s `TildeTilde` arm — the token→text renderer — left the
suite green, including the test named for the macro round trip. **Macro BODIES
in sigil are stored as raw text**, so a body never reaches the renderer.
`render_tokens` serves macro ARGUMENTS. The gate was therefore unreachable from
any test, and a wrong rendering would have shipped.

`logical_not_renders_back_through_a_macro_argument` closes it, with the string
embed as the sharpest column (`dc.b "[~~0]"` = `5B 7E 7E 30 5D` — a wrong
rendering becomes a literal byte). **That row is labelled corpus-UNEXERCISED in
its own doc comment**: all 96 `s2disasm` sites spell `~~` in a body or at top
level, and none passes it as a macro argument. It is a gate whose subject the
corpus cannot exhibit — not passing, unexercised — and it is committed saying so.

### The suite, both sides, derived rather than taken

Both runs `scripts/landing-run.sh --aeon /home/volence/sonic_hacks/.aeon-eval-ref`,
each in its OWN detached worktree with its OWN on-disk target dir, run
sequentially — never two full suites at once (the concurrency artefact the
`irp`/`irpc` note recorded).

| | tree | suites | passed | failed | ignored | CARGO_EXIT |
|---|---|---|---|---|---|---|
| master `b5e7714d` | `.wt-tilde-base` | 378 | **4,351** | 0 | 2 | 0 |
| this branch `064a2d78` | `.wt-tilde` | 378 | **4,362** | 0 | 2 | 0 |

**`+11`, and this parcel adds exactly 11 `#[test]` rows.** Reconciled.

The branch log carries all 11 by name; the master log carries **none** of them,
which is what says the two logs are of the two trees they claim. Zero `FAILED`
lines in either.

- **Clippy** `--release --workspace --all-targets -- -D warnings` — **exit 0**.
  The 19 `warning:` lines are all C compiler output from the `-sys` crates'
  build scripts; zero Rust warnings.
- **aeon** — four artifacts deleted first, then one shape per invocation under
  `SIGIL_VERSION_STRICT=1`, all exit 0. **Byte-identical**, as predicted:

  | shape | CRC32 / size |
  |---|---|
  | `s4.bin` | `14ee2440` / 719700 |
  | `s4.debug.bin` | `142294b3` / 737683 |
  | `demo.bin` | `0c456778` / 96474 |
  | `demo.debug.bin` | `2e603d53` / 101339 |

  No shape moved. Nothing in `golden/`, `pins.rs` or `repin.toml` was touched.
- Corpus figures re-derived with the FINAL committed binary: S2 9,539 and S1 368,
  each byte-identical to the run above.

## Booked, not done

- **`even` is not a recognized 68000 mnemonic in sigil.** Hit while building the
  `jmpTos` probe. Separate gap.
- **A trailing `align` at end of file emits padding sigil's `asl` does not.**
  `rts` + `align 4` differs by 2 bytes with the OLD binary AND the new one
  (`p6.asm`), so it is not this parcel's. It is why `p5b.asm` puts content after
  the last `align`.
- **A user `function` returning a string cannot be folded into a `.l` absolute
  address** (`jmp (extractJmpToName("op")).l`). This is the 518, and it is now
  the largest single S2 site.
- **`zMakeFMFrequency`** — `irp` over float literals through a user function
  using `roundFloatToInteger`/`INT`. 84 diagnostics, newly visible.
- **The three shapes asl refuses and sigil accepts** (`~-1`, `~~~~0`, `~ ~ 0`).
  Corpus-unreachable in both corpora, characterised above, not gated.
- **`~~` of a string.** `dc.b ~~"a"` is `00` under asl. Not probed far enough to
  say whether the string is coerced to an integer or tested for emptiness —
  `~~""` would separate them. Unreachable in both corpora.
