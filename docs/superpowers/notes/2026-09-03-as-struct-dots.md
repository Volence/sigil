# `STRUCT … DOTS`, its two offset tables, and the RAM map that checks itself

Sonic 1's largest remaining row and Sonic 2's fourth. The count is the small
part: the struct's size decides where every RAM variable declared after the
instance lands, and Sonic 1's `_Variables.asm` **asserts its own layout twice**.
Both assertions now pass.

## Provenance

| | |
|---|---|
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098` |
| flags | `-xx -n -q -A -L -U -E -i .` — the corpus's own, `build_tools/lua/common.lua:773` |
| corpora | `s1disasm` `f6ece657` (entry `sonic.asm`), `s2disasm` `e45ebf3` (entry `s2.asm`), both in detached worktrees; S1 seeded with the gitignored `sound/dac/{pcm,dpcm}/generated/` **including the `.pcm`/`.dpcm` beside the `.inc`** |
| probes | committed beside this note in `2026-09-03-as-struct-probes/` — `q1`…`q19`, `s2structs`, plus `run.sh`, `cmp.sh`, `sweep.sh`, `mutate.sh`, `mutate_tests.sh`, `mkram.py` |

**S1 and S2 ship different `asl` builds behind one version string.** S1's is
upstream AS; S2's is the flamewing fork (md5 `0dee1f98…`). Every rule here was
measured against **S1's**, and `-U` is on every invocation.

## The semantics

### The declaration

`NAME struct [MODIFIER…]`, closed by `endstruct` — bare, or with the struct's
own name in the LABEL column (`SoundQueue ENDSTRUCT`, which is how three of the
six corpus sites are written), or by `ends` (`q8.asm`). The body runs its own
location counter from 0 and **emits nothing**; the outer PC is untouched.

`DOTS` makes the separator `.` instead of `_`, and it is a property of the
STRUCT, not of the site: a bare `A struct` yields `A_a` / `A_len`, and an
INSTANCE of it yields `j_u` while `j.u` is asl's `#1010` (`q8.asm`, `q10.asm`,
`q11.asm`). Recognised case-folded — Sonic 2 writes `struct dots` at one site
and `STRUCT DOTS` at three.

### Three member shapes, all of which the corpora write

- `[name:] ds.b|ds.w|ds.l <count>` — a reserve field.
- `name:` alone — a **marker**, which reserves nothing and binds the running
  offset. Sonic 1's `SMPS_RAM` has 21 of them and four are read by name.
- `[name:] <another struct's name>` — an **embed**. `SMPS_RAM` embeds
  `SMPS_Track` eighteen times and asl FLATTENS them, which is what makes
  `SMPS_RAM.v_music_dac_track.PlaybackControl` a name at all.

Members are bound as the body is WALKED, not at `endstruct`: `SMPS_RAM`'s last
field is `ds.b SMPS_RAM.v_1up_ram_end-SMPS_RAM.v_1up_ram`, reading two of its own
earlier markers back through the full dotted name
(`s1.sounddriver.ram.asm:108`).

### asl keeps TWO offset tables per struct and they DISAGREE

This is the finding of the row. A `b: ds.w 1` at an odd running offset under
`padding on` binds the **declaration-scope symbol** to the offset BEFORE its
alignment pad, and records the **struct element** — which is what an
instantiation reads — at the offset after. `q7.asm`,
`a ds.b 1 / b ds.w 1 / c ds.b 1 / d ds.l 1`:

```text
   9/ 1000 : 0000 0001 0004          dc.w S.a,S.b,S.c,S.d,S.len
      1006 : 0005 000A
  10/ 100A : (STRUCT)             inst:  S
  11/ 1014 : 0000 0002 0004          dc.w inst.a-inst,inst.b-inst,inst.c-inst,inst.d-inst
      101A : 0006
```

`S.b` is 1 and `inst.b - inst` is 2, from one declaration, in one run. The
listing shows the `<padding>` line at offset 1 and `b:` at 2, so the LISTING
agrees with the element table and the symbol does not.

Two consequences worth stating separately from the measurement. First: the
running offset is the same either way — `width * count` is even for every `ds.w`
and `ds.l`, so pad-before and pad-after land on the same total, which is why
`NAME.len` is not a discriminator and the pre-existing pad-after model was right
about every symbol it bound. Second: this looks like an asl bug rather than a
feature, and sigil reproduces it because the corpus reads both spellings.

### Placement

An instance is placed VERBATIM and is **never word-aligned**, even under
`padding on` and even when the struct leads with a `ds.w` — while a bare
`ds.w 1` at the same odd address does pad. `q9.asm` puts a word-leading struct
at `org $2001` and asl leaves it there. An embed is not re-aligned to the
parent's parity either, so an inner `ds.w` member can land at an ODD parent
offset (`q10.asm`: `S.n.r` = 3).

An UNLABELLED instantiation is asl's `#2040 structure name missing` and reserves
nothing at all (`q8.asm`: the PC does not move).

## Both corpora, before and after

Every class, summing with no remainder. Measured with the final committed binary
(md5 `a4a2467bc3261c02f06090a18212d762`).

### Sonic 1 — 887 → 368

| before | after | delta | class |
|---:|---:|---:|---|
| 497 | 1 | **−496** | unresolved symbol in operand |
| 25 | 5 | **−20** | unresolved long expression |
| 96 | 94 | **−2** | `X` is not a recognized 68000 mnemonic |
| 1 | 0 | **−1** | the corpus's own `error` self-check |
| 166 | 166 | 0 | bad word expression |
| 36 | 36 | 0 | bad operand expression |
| 18 | 18 | 0 | unexpected character |
| 18 | 18 | 0 | instruction needs an explicit size suffix |
| 8 | 8 | 0 | unresolved rept count |
| 6 | 6 | 0 | case needs a string literal |
| 6 | 6 | 0 | bad immediate expression |
| 4 | 4 | 0 | trailing tokens in operand |
| 2 | 2 | 0 | unsupported form: ccr is not a general EA |
| 2 | 2 | 0 | switch needs a string expression |
| 1 | 1 | 0 | unknown directive or mnemonic `purecode` |
| 1 | 1 | 0 | org target precedes the current phase base |
| **887** | **368** | **−519** | |

**No class rose.** Sites 609 → 124, **0 new**. Unresolved-symbol NAMES 86 → 1,
**0 new** — 85 of 86 resolved, and the survivor is `usp`, the 68000 user stack
pointer, which is instruction coverage and not this row.

The `−20 unresolved long expression` is the cascade the S1 baseline note booked
at `_inc/Special Stage Mappings & VRAM Pointers.asm(10)` and elsewhere: those
were the operands, and the operands were struct members.

### Sonic 2 — 9,317 → 8,918

| before | after | delta | class |
|---:|---:|---:|---|
| 3745 | 3376 | **−369** | unresolved symbol in operand |
| 219 | 187 | **−32** | `X` is not a recognized 68000 mnemonic |
| 0 | 1 | **+1** | struct member line this cannot read |
| 3 | 4 | **+1** | malformed number (hex needs a trailing `h`) |
| 2622 | 2622 | 0 | bad operand expression |
| 2307 | 2307 | 0 | expected mnemonic, directive, or label |
| 131 | 131 | 0 | bad word expression |
| 114 | 114 | 0 | absolute address needs an explicit width suffix |
| 58 | 58 | 0 | instruction needs an explicit size suffix |
| 39 | 39 | 0 | cannot include (gitignored generated sound data) |
| 30 | 30 | 0 | bad byte expression |
| 23 | 23 | 0 | `int()`: could not evaluate float expression |
| 11 | 11 | 0 | unexpected character |
| 6 | 6 | 0 | case needs a string literal |
| 3 | 3 | 0 | bad displacement expression |
| 2 | 2 | 0 | trailing tokens in operand |
| 2 | 2 | 0 | switch needs a string expression |
| 1 | 1 | 0 | unknown directive or mnemonic `purecode` |
| 1 | 1 | 0 | unsupported form: `sbc hl,bc` |
| **9317** | **8918** | **−399** | |

Sites 8588 → 8200, and **exactly one is new**: `s2.sounddriver.asm(159)`, which
carries BOTH rises. That line is the silent-half finding below. Unresolved-symbol
names 291 → 203, **0 new**.

## What the corpus's own RAM assertion says

`_Variables.asm` checks itself twice. Both fired before; neither fires now.

```text
BEFORE
  _Variables.asm(114): error: `SMPS_RAM` is not a recognized 68000 mnemonic
  _Variables.asm(430): error: v_chunk0collision needs to be at address $FFFFFF00 …
  _Variables.asm(486): error: `warning` is not a recognized 68000 mnemonic

AFTER
  (nothing)
```

- **Line 430** is the `if v_chunk0collision<>ramaddr($FFFFFF00)` guard on
  `FindNearestTile`. It passes.
- **Line 486** is the `elseif * < 0` arm of the whole-map size check at
  `v_ram_end`. It reached the `warning` because the map was SHORT; now neither
  arm is taken, so the total RAM declaration ends exactly where the disassembly
  says it must. (The `if * > 0` arm calls `fatal`, which sigil DOES implement —
  its absence is a measured fact, not an unimplemented directive:
  `grep -icE 'RAM variable declarations|too large by'` over the after-capture is 0.)

**The offset the message names could not be read, and that does not matter here.**
`\{…}` interpolation is still not performed (a prior parcel's finding, unchanged
by this one), so line 430's text carries `\{signedToString(…)}` raw. The
directive firing at all was the signal; it stopping is the result. Where a
by-how-much figure was actually wanted, the byte sweep below supplies it
directly and for every symbol rather than for one.

## The silent half — what was done to look

A wrong struct size emits **wrong addresses, not complaints**. Counting
diagnostics cannot see any of it, and neither corpus reaches link, so there is no
ROM to compare. Five things were done instead.

**1. A 1,553-symbol byte sweep of the whole RAM map, and it is proven able to
fail.** `mkram.py` reads every label out of `_Variables.asm` and both struct
declarations — never out of an asl dump — and emits `dc.l` for each: every RAM
variable, every `SMPS_Track` and `SMPS_RAM` member, every embedded track's
members under `SMPS_RAM`, and the same layout again through the real
`v_snddriver_ram` instantiation. Assembled by both tools from the corpus's own
first 64 lines.

```text
asl abc4c314/6212   sigil abc4c314/6212
SWEEP: GREEN — 1553/1553 RAM symbols identical
```

This is the strongest available statement and it is stronger than the two
assertions: it says every address agrees, not that two of them do.

**2. Six mutations against that sweep, each applied to a committed baseline with
the patch read back FROM DISK, each restored.** Three RED — markers dropped, DOTS
ignored, and a positive control setting `ds.l` to 2 bytes wide. Each of the three
reddens by making the corpus's own line-430 assertion fire again, which is a
second witness that the oracle is live.

**3. Three of the six came back GREEN, and all three are real uncovered
ground.** Making the instance base word-aligned, re-aligning an embed to the
parent's parity, and taking the PRE-pad offset for the element table all leave
the sweep at 1553/1553. The reason is arithmetic, not luck, and it is worth
writing down: `v_snddriver_ram` lands at the even `$FFFFF000`, `SMPS_Track.len`
is an even `$30` so every embed starts even, and **neither struct has a `ds.w` or
`ds.l` member at an odd offset**, so the two tables never diverge in this corpus.
**The pre/post-pad split, instance non-alignment and embed non-re-alignment are
pinned by NOTHING in either corpus** — only by the asl probes and the unit tests
derived from them. Every one of the three is red against those.

**4. A GREEN mutation that was green against its own test, chased rather than
banked.** Rounding the instance base left
`a_struct_instance_is_never_word_aligned` passing, because the label is bound by
`define_label` BEFORE the instantiation runs — so the row read the PLACEMENT and
never the base the MEMBERS hang off. Two different facts wearing one name. Probe
`q14.asm` puts a word-leading instance at the odd `$1009` and reads `i1.w-i1` and
`i1.x-i1` back; the row now covers both and the mutation is red.

**5. A stale-binary trap, caught, and the runner fixed.** `mutate.sh` restored
the source and **not the binary**, so the `ds.l => 2` positive control stayed
installed at `.target-land/release/sigil` and every ad-hoc probe run afterwards
silently measured a mutated assembler. It produced a convincing false defect —
`ds.l` reading as 2 bytes wide — which was contradicted by the green RAM sweep,
and the contradiction is what exposed it. `sweep.sh` rebuilds every time, so no
sweep result was ever affected; every ad-hoc `cmp.sh` result between the control
and the fix was re-run. Both mutation runners now rebuild on restore.

### What the hunt found

**A silent one-byte struct size in Sonic 2, caught by bytes and by nothing
else.** `s2.sounddriver.asm(159)` declares `1upPlaying: ds.b 1` — asl accepts an
identifier that begins with a digit and sigil's lexer refuses it as a malformed
number. That refusal was being dropped by a bare `.ok()?` inside the struct
walker, and the line was skipped: `zVar.len` came out `$17` against asl's `$18`,
with **exit 0 on both sides, no diagnostic anywhere, and every member after it
one byte low**. Probe `q19.asm`, reduced to three fields, reproduces it exactly.

Fixed HALFWAY and deliberately: the lexer's own diagnostic is now surfaced, and
an unread member line says what it costs. **The leading-digit identifier itself
is an identifier-surface question and is NOT settled here** — it belongs to
whoever owns that rule, and guessing at it inside a struct parser is how a
narrow fix becomes a wide one. This is the source of both S2 rises, and it
converts a silent wrong answer into a loud one.

The other three S2 structs — `SoundQueue`, `HorizontalScrollBuffer`, `zTrack` —
are byte-exact against asl (`s2structs.asm`: `HorizontalScrollBuffer.len` `$400`,
`SoundQueue.len` `$5`, `zTrack.len` `$2A`, plus every member through both the
declaration and an instance).

### Booked, not fixed

- **`X: ds.w 1` at an odd PC is not padded by sigil.** asl gives the label the
  POST-padding address (`q4.asm`: `y` at `$2001` lists as `$2002`), and a LONE
  label on its own line also takes the address after the NEXT line's padding
  (`q12.asm`: `n-m` is 4 where the listing reads 3). sigil pads neither.
  Pre-existing — `directive_ds` never calls `pad_word_align` and this parcel did
  not touch it — and unreachable in Sonic 1's RAM map, which the byte sweep
  proves rather than assumes. Falsifiers: `q9.asm`, `q12.asm`.
- **`endstruct <name>` with the name in the OPERAND column** renames the size
  symbol to that name: `endstruct C` defines `C` = the size and leaves `C.len`
  undefined. sigil defines `C.len` normally. Corpus-unreachable (all six sites
  are bare or label-column). Probe `q8.asm`.

  **CORRECTED 2026-09-05, and the correction is the whole second half of the
  row.** This entry used to continue "yet asl then resolves `C.len` to the
  current PC and exits 0, which is its own silent-wrong-answer". Both halves are
  wrong. asl exits **2** on `q8.asm`, and the `100A` that `dc.w C.a,C.len`
  printed is the PASS-1 PLACEHOLDER for an unresolved symbol, not a resolution.
  `q8.asm` carries an unrelated `#2040 structure name missing` on its last line,
  which stopped asl's pass loop, and the pass that judges forward references
  never ran. Delete only that one line and asl says

  ```text
  q8n.asm(15):11: error #1010: symbol undefined
  C.len
  ```

  and prints **no byte column at all** for that line. asl is LOUD about `C.len`;
  there is no silent wrong answer here, only a suppressed diagnostic. See
  `2026-09-05-asl-pass-loop-swallows-diagnostics.md`.
- **A struct body under `CPU Z80` cannot use `ds.b`** at all in asl (`q13.asm`);
  both corpora route it through their own `ds` MACRO, whose Z80 arm emits `db 0`
  bytes. `parse_struct_member` reads the width off the WRITTEN token instead of
  expanding, which gives the same offsets on the 68000 path sigil stays on for
  S2's `cpu z80undoc`. Not equivalent in general; not exercised.
- **The separator when a DOTS struct embeds a non-DOTS one.** The join uses the
  OUTER struct's separator. Both corpora are uniform, so this was not measured.

## Verification

- **8 new `#[test]` rows** in `crates/sigil-frontend-as/src/eval.rs`, every
  expectation a byte column read off an asl listing.
- **Seven red-first mutations against those rows** (`mutate_tests.sh`), each
  applied to a committed baseline with the patch read back from disk, each
  actually red, each restored: instance member base aligned; instance PLACEMENT
  aligned; embed re-aligned; element table taking the pre-pad offset; markers
  dropped; DOTS ignored; `ds.l` two bytes wide.
- **Six mutations against the corpus byte sweep** (`mutate.sh`) — three red,
  three green and accounted for above.
- **Aeon byte-neutrality**: all four artifacts deleted first, each shape rebuilt
  in its own invocation under `SIGIL_VERSION_STRICT=1` against
  `.aeon-eval-ref` at `4f5ad5a1`. **All four byte-identical**:
  `s4 14ee2440/719700`, `s4.debug 142294b3/737683`, `demo 0c456778/96474`,
  `demo.debug 2e603d53/101339`. Aeon writes no AS `struct` — `grep -rniE
  '^\s*\S+\s+struct\b' --include='*.asm' --include='*.inc'` is 0 — so
  byte-neutrality was the expectation and not a surprise.
- `cargo clippy --release --workspace --all-targets -- -D warnings` — **exit 0**.
- **The landing run.** `scripts/landing-run.sh --baseline 4348 --aeon
  ~/sonic_hacks/.aeon-eval-ref`, stamped `pwd` `/home/volence/sonic_hacks/.sigil-struct`,
  HEAD `0d0306b1`, branch `parcel/as-struct-dots`, reference `.aeon-eval-ref` @
  `4f5ad5a1`: **378 suites / 4,348 passed / 0 failed / 2 ignored**, `CARGO_EXIT=0`,
  GREEN. All eight new rows appear in that log by name. The stamp reads DIRTY
  because `.probe/` and this note were untracked; `crates/` was clean, and the
  two commits after `0d0306b1` are documentation only, so the green covers every
  line of code on the branch.
  Log: `.probe/landing-branch.log`.
- **The master baseline, derived rather than taken.** Same wrapper in a detached
  worktree of `e45bc305`, its own target directory: **378 suites / 4,340 passed /
  0 failed / 2 ignored**, `CARGO_EXIT=0`. The two runs were sequential, never
  concurrent. 4,340 + 8 new `#[test]` rows = 4,348. It reconciles exactly.
  Log: `.probe/landing-master.log`.
