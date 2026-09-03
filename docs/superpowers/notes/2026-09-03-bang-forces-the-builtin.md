# `!` forces the builtin, and a macro beats the builtin

2026-09-03 · `parcel/as-bang-builtin` · sigil `74f19b78` off master `9a08cfdc`

The corpus's largest single diagnostic site, `s2.macrosetup.asm:68` at 626, is
`!ds.ATTRIBUTE` re-entering the `ds` macro it sits inside. That characterisation
holds — it is the exact message and the exact count. What it does not say is that
the same misunderstanding of `!` had a second, silent half running the whole
corpus, and that a third divergence sits one `ds` away from the shipped ROM.

## What `!` means, with the listing rows

asl 1.42 Beta Bld 212, every probe under the corpus's own flags
(`-xx -n -q -A -L -U -i .`).

**A user macro BEATS the builtin of the same name — directive and mnemonic
alike — and `!` is the only escape.**

```text
      10/     100 : 11                  	dc.b	$11
      11/     101 : (MACRO)              	org	$200
      11/     101 : EE                          dc.b    $EE
      12/     102 : 22                  	dc.b	$22
      13/     103 : (MACRO)              	move.w	#1,d0
      13/     103 : DD                          dc.b    $DD
      14/     104 : 44                  	dc.b	$44
      15/     300 :                     	!org	$300
      16/     300 : 55                  	dc.b	$55
```

`org $200` advances by the macro's single byte instead of seeking to `$200`;
`move.w` emits `DD`; only the `!` line seeks.

**`!name` resolves in the builtin table and nowhere else.** It expands the
shadowed builtin ONCE — the `ds` macro whose own body is `!ds.ATTRIBUTE ALLARGS`
reserves and does not recurse:

```text
       4/     100 :                     ds macro
       5/     100 :                     	!ds.ATTRIBUTE ALLARGS
      10/     101 : (MACRO)              	ds.b	4
      10/     101 :                             !ds.b 4
      11/     105 : 33                  	dc.b	$33
```

`.ATTRIBUTE` carries the INVOCATION's suffix and `ALLARGS` its arguments, so
`ds.b 4` becomes `!ds.b 4` and `$101 → $105`. `ds.w 3` spans 6 and `ds.l 2`
spans 8, off the same probe.

**The bypass is not a fallback.** A `!` on a name that is only a user macro is an
error, not an invocation, and so is a `!` on a name that is nothing:

```text
      15/     10F : (MACRO)              	mym
      15/     10F : AA                          dc.b    $AA
> > > p3.asm(16):3: error #1200: unknown instruction
> > > MYM
      16/     110 :                     	!mym
      17/     110 : 44                  	dc.b	$44
```

`!frobnicate 1` is the same `#1200`.

**The `!` is a prefix of the name, not a separate word**, and it composes with a
colon label:

```text
       8/     101 :                     Lbl:	!ds.b	3
       9/     104 : 22                  	dc.b	$22
> > > p5.asm(10):3: error #1200: unknown instruction
> > >  ! ds.b	3		; bang separated by a space
      12/     106 : (MACRO)              	align	4
      12/     106 : EE                          dc.b    $EE
      14/     108 :                     	!align	4
```

**A macro invoked with NO attribute substitutes `ATTRIBUTE` with the empty
string**, and a bare `ds` reserves in the CPU's own granularity — word on the
68000, byte on the Z80, where `ds.w` is not an instruction at all:

```text
      11/     101 : (MACRO)              	ds	4
      11/     101 :                             !ds. 4
      12/     109 : 22                  	dc.b	$22
```

```text
       3/     100 : 11                  	db	11h        ; cpu z80
       4/     101 :                     	ds	4
       5/     105 : 33                  	db	33h
> > > p6.asm(6):2: error #1200: unknown instruction
> > > DS.W
```

## The diagnosis

Two mechanisms, one rule.

`dispatch` matched the keyword table BEFORE the macro table, so any macro whose
name is a directive keyword never ran. Exactly two of the corpus's 187 macro
names collide with one of the 25 keywords `dispatch` handles — `org` and
`align` — and `s2.macrosetup.asm` defines both: `org` forward-only and
padding-counting, `align` as `cnop 0,n` through that same `org`. Running asl's
builtins there is running a different program, with no diagnostic. Aeon's one
`.asm` macro (`_kdebug`) collides with nothing.

`!` was stripped and re-dispatched, which is correct only while nothing shadows
a builtin — the case the escape exists for is the case it got wrong.

Both are the same rule read from the two ends: the macro wins, and `!` is how
you say you meant the builtin. `exec_one` records the `!` instead of dropping it
and routes to `dispatch_builtin`; `dispatch_resolved` consults the macro table
first, and the forced path skips it. The mnemonic side was already right —
`lower_instruction` sits at the bottom of the match — and so was the
`.ATTRIBUTE`-suffix side, which is why `ds.b` reached the macro at all and
recursed.

## `AS-ATTRIBUTE-BANG-ADDI` does not share this cause

Its 40 diagnostics at `macrosetup:224`/`:227` are not about `!` or about
`.ATTRIBUTE`. All 40 are `unresolved symbol` — the line already reached the
builtin `addi`/`subi` correctly and failed on its OPERAND. Sixteen distinct
names, and every one of them resolves, directly or through a single `=` alias,
to an `objoff_XX` defined by one of the three `enum` lines at
`s2.constants.asm:133-135` — a directive sigil does not implement (31 of its own
diagnostics). The row is `enum` fallout reported at a macro body's span; it is
unmoved by this parcel and belongs to `enum`'s.

## The silent half, and how it was hunted

Four things were done, and the negative results are stated as results.

1. **A byte-level probe that reaches LINK**, comparing `sigil x.asm -o x.bin`
   against `asl` + `p2bin` on the corpus's own macro-shadowed `ds`. It found a
   divergence — see below.
2. **The precedence probe.** Writing a macro that shadows `align` and asserting
   asl's `EE` is what turned up the `org`/`align` mechanism above. This was the
   large one, and it was found by a test that failed, not by reading code.
3. **Every `ds` site in the corpus enumerated and classified.** 695 of them:
   639 inside `phase ramaddr($FFFF0000)` in `s2.constants.asm`, 56 inside the
   `zTrack`/`zVar` `STRUCT` blocks in `s2.sounddriver.asm`. All address-only,
   none in an emitting context. Aeon's three residual `.asm` files contain no
   `ds` at all — the grep is the fact.
4. **Bare `ds` (no attribute).** asl reserves in CPU granularity; sigil errors
   with `` `ds.ATTRIBUTE` is not a recognized 68000 mnemonic``, because
   `ATTRIBUTE` is not substituted on an unsuffixed invocation. LOUD, not silent,
   and zero corpus reach. Booked, not fixed.

### The divergence probe 1 found, and why it is not in this parcel

`Reserve` advances the VMA and places no image byte, which is right for RAM and
wrong the moment a byte follows a reservation inside ONE section: asl reserves by
leaving a GAP and `p2bin` FILLS that gap. Same source, one section:

```text
  asl:   11 00 00 00 00 22 00 00 00 00 00 00 33 00 00 00 00 00 00 00 00 44 00 00 00 16
  sigil: 11 22 33 44 00 00 00 16
```

Both agree the trailing label is `$16`; only asl's image has a byte there. Exit
0, no diagnostic, twenty bytes short — the shape a diagnostic count cannot see.

The fix is one rule in three image walks (`Section::image_bytes`, `link`'s
fixup-offset replay, `image_final_size`): the write cursor advances, the image
grows only where something writes. That is both halves of p2bin at once — the gap
fills, a trailing reservation is trimmed, a pure-reserve section places nothing —
and it was implemented, measured, and is byte-for-byte asl+p2bin on all four
probe shapes with all four Aeon ROMs still identical.

**LANDED — see `2026-09-03-reserve-materialises.md`.** It was parked on
`parcel/as-reserve-materialise` behind the align question below; that question
is settled (`cb4521d9`, shared rule in `sigil-ir/src/align.rs`) and the fix is
rebased and merged. Seven expectations encoded the packed model, not five, and
none of them is the `align_inside_a_phase_advances_a_full_extra_block` named
below — the align parcel had already rewritten that test, under the name
`align_inside_a_phase_at_a_rom_address_is_a_plain_roundup`, so what remained was
its image expectation rather than its rule. Chasing the one mutation that stayed
green also turned up a missed-collision hole in the overlap check, now gated.

The question it was parked on, kept for the record:

> **`align` inside a `phase` does NOT advance a full extra block, per asl 1.42
> Bld 212 under `-xx -n -q -A -L -U -i .`.** Three probes, plain round-up of the
> LOGICAL address every time:
> ```text
>        4/    B000 :                     	ds.b	5
>        5/    B005 :                     	align	256
>        6/    B100 : B100                L:	dc.w	L
> ```
> ```text
>        4/    B000 : 0102 0304 05        	dc.b	1,2,3,4,5
>        5/    B005 :                     	align	256
>        6/    B100 : B100                L:	dc.w	L
> ```
> ```text
>        4/    B000 :                     	align	256
>        5/    B000 : B000                M:	dc.w	M
> ```
> `$B005 → $B100`, not `$B200`. `directive_align`'s recorded rule
> (`round_up(pos + n, n)`, from a 2026-07-08 probe) and the test asserting
> `[0xB2, 0x00]` are not reproduced here. This is a claim about someone else's
> measurement made from three probes of my own, on a rule that decides Aeon RAM
> addresses; it is FLAGGED, not changed. Whoever takes it should re-derive the
> original probe's shape before touching anything — the disagreement may be in
> the setup, not the rule.

`p2bin` in that probe: 258 bytes, `b1 00` at offset `$100` — so the phased
reservation IS materialised in the object file, which is the same finding from
the other side and is why the two questions are entangled.

**SETTLED 2026-09-03, in this reader's favour** — see
`2026-09-03-align-in-phase-contradiction.md` and the 31 committed listing rows in
`docs/superpowers/probes/2026-09-03-align/`. The three probes above are right and
the recorded rule was wrong, but not because July mismeasured: `align` rounds up
on the low 32 bits of the PC read as a SIGNED `i32` with C's truncating
remainder, so the regime is the SIGN of the address and not `phase` at all. July
probed aeon's `$FFFF….` game RAM and wrote its four rows down by their low half;
all four reproduce today as RAM addresses, under BOTH asl binaries on this
machine. `directive_align` and `regions.rs::align_to` now share one
`sigil_ir::asl_align_pad`, and the test asserting `[0xB2, 0x00]` is three tests
asserting what asl says on each side of the boundary. The `b1 00` at `$100`
above is the expectation this branch's own rewrite of that test should carry.

## Corpus decomposition

`sigil s2.asm` from `/home/volence/sonic_hacks/s2disasm`, master `9a08cfdc`
against branch `74f19b78`. **13,830 → 13,109, a fall of 721.**

| site | message | before | after |
|---|---|---:|---:|
| `s2.macrosetup.asm(68)` | macro `ds` expansion too deep | 626 | 0 |
| `s2.macros.asm(62)` | `warning` is not a recognized 68000 mnemonic | 30 | 0 |
| `s2.macros.asm(63)` | `exitm` is not a recognized 68000 mnemonic | 30 | 0 |
| `s2.asm`, 34 lines | operand -65536 out of range | 34 | 0 |
| `s2.macrosetup.asm(40)` | `org needs a constant expression` | 1 | 0 |

Four booked rows closed: `AS-MACRO-BANG-SHADOW` (626),
`AS-WARNING-EXITM` (60), `AS-WORD-IMM-RAM-LABEL` (34),
`AS-CNOP-ORG-CONST` (1). The last three close because they were never divergences
of their own: `warning`/`exitm` sit in `clearRAM`'s `elseif startaddr==endaddr`
arm, which is no longer selected now that the RAM layout's addresses come from
the `org`/`align` MACROS; the 34 `.w` immediates name `$FFFF…` RAM labels whose
values those same macros place. **`warning` and `exitm` are still unimplemented**
— they are unreached, which is also what asl reports at those lines (nothing).

**ZERO classes rose**, so the accounting has nothing to explain. Distinct
unresolved symbols: 291 before, 291 after; newly unresolved `[]`, newly resolved
`[]` — sets compared both directions. Every changed site went to zero; no site
changed in the other direction.

## What stays open, with measured sizes

| row | size | what it is |
|---|---:|---|
| `sound/_smps2asm_inc.asm` | 3,520 | the largest FILE, unmoved, its own arc |
| `AS-MACRO-ARGCOUNT-IRP` | 272 | `s2.macros.asm:289` (`irpc`) |
| `AS-INSN2OP-DISP-OPERAND` | 162 | `insn2op`'s `1+y` arms |
| `AS-ENUM-DIRECTIVE` | 40 + 31 | 40 unresolved `objoff_XX` consumers at `macrosetup:224`/`:227` plus 31 at the three `enum` lines. **Renamed from `AS-ATTRIBUTE-BANG-ADDI`, which named the wrong cause.** |
| `AS-RESERVE-IMAGE-GAP` | 0 | the p2bin gap fill. LANDED — `2026-09-03-reserve-materialises.md`. Zero is the count it will always carry: both corpora exit in the front end before `link` runs, so a diagnostic count cannot see an image rule |
| `AS-ALIGN-IN-PHASE-ROUNDUP` | 0 | three probes say plain logical round-up, not `+n`. Flagged, not changed |
| `AS-DS-BARE-ATTRIBUTE` | 0 | `ATTRIBUTE` unsubstituted on an unsuffixed invocation, so bare `ds` errors. Loud; no corpus or Aeon reach |

## Verification

- Four tests, every expected value a listing row above:
  `bang_forces_the_builtin_past_a_macro_of_that_name`,
  `bang_never_falls_back_to_a_user_macro`,
  `bang_composes_with_a_label_and_binds_tightly`,
  `a_user_macro_shadows_the_builtin_of_the_same_name`.
- Red-first, three mutations at the shipped tip, each shown applied with
  `git diff` and restored from the committed baseline: reverting
  `dispatch_builtin` reddens 4, disabling the precedence check reddens 2,
  disabling the adjacency check reddens 1. Baseline and restored runs both green.
  **No gate here is pinned by construction; all three are pinned by a red.**
  One mutation was applied-and-still-green during the reserve work and chasing it
  found real uncovered ground — `link`'s fixup-offset walk is unreachable from AS
  source, because a same-section label is FOLDED by the front end and the line
  carries no fixup at all. That test is on the split branch, built at the IR
  level for exactly that reason.
- Landing run GREEN at `74f19b78`: 4,314 passed, 0 failed, 2 ignored, 376 suites,
  cargo exit 0, zero `skip:` and zero `ratchet:`, reconciling the 4,310 master
  baseline (measured in its own worktree and its own target directory) + 4. Each
  of the four test names appears exactly once in that log.
  `.sigil-bang-ds-target/landing-bang-builtin.log`.
- Clippy `--release --workspace --all-targets -- -D warnings`: exit 0.
- Aeon: all four artifacts deleted, each shape rebuilt in its own invocation
  under `SIGIL_VERSION_STRICT=1`, byte-identical to the frozen tip —
  s4 `14ee2440`/719700, s4.debug `142294b3`/737683, demo `0c456778`/96474,
  demo.debug `2e603d53`/101339. The trailing commit is doc-comment-only
  (filtered: 0 non-`///` lines) and the landing run was re-run on it anyway.
