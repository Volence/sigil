# `align` is a truncating round-up on the SIGNED PC — the phase was never the rule

2026-09-03 · opened by the overseer from two dead agents' surviving probes ·
**settled and fixed on `parcel/as-align-in-phase`**

Two agents died to a 529 within the same overload event. Neither committed. This
note exists because the finding below was established from what they left on
disk, and an unbanked finding is one rotation from gone.

## The answer

`align n` in asl 1.42 Bld 212 does not round the address up. It computes the
aligned target on the **low 32 bits of the PC read as a signed `i32`**, with C's
truncating remainder, and advances by the unsigned 32-bit difference:

```
t32   = (int32) (pc + n - 1)
a32   = t32 - (t32 % n)          // C '%': the remainder takes the DIVIDEND's sign
delta = (uint32) (a32 - (int32) pc)
pc'   = pc + delta               // added to the WIDE pc, so it can exceed 32 bits
```

A **non-negative** low half — every ROM address — gets the plain round-up, and an
already-aligned PC does not move. A **negative** low half — every `$FFFF….` 68k
RAM address — rounds toward zero instead of down, so it usually lands one block
high and an already-aligned address advances a full `n`.

**`phase` has nothing to do with it.** `phase $B000` + `ds.b 5` + `align 256`
gives `$B100`, exactly as the unphased form does; `org $FFFFB000` + `align 256`
gives `$FFFFB100` with no phase in sight. The regime is the sign of the PC.

31 listing rows, the probe sources, and a runner are committed under
`docs/superpowers/probes/2026-09-03-align/` (`RESULTS.md` carries the table).
Every row is reproduced by `sigil_ir::asl_align_pad`, which is now the single
implementation both front-ends call.

## The July probe was right; only its condition was lost

`directive_align`'s doc comment recorded a 2026-07-08 live probe as a table of
four results — `$B000→$B100`, `$B005→$B200`, `$B026→$B200`, `$B100→$B200` — and
generalised them to "inside a phase, asl advances by `round_up(pos + n, n)`".

**All four rows reproduce today, on RAM addresses:**

```
$FFFFB000  n=256  -> FFFFB100        $FFFFB026  n=256  -> FFFFB200
$FFFFB005  n=256  -> FFFFB200        $FFFFB100  n=256  -> FFFFB200
```

July measured aeon's phased `$FFFF….` game-RAM block and wrote the addresses
down by their low half. What was lost between the probe and the comment is that
those were negative PCs. The rule was attributed to `disp != 0` instead, and the
regression test then transcribed it into a `phase $B000` source — a *positive*
PC, where it does not hold and where asl answers `$B100`.

The generalisation is wrong on both sides: `round_up(pos + n, n)` disagrees with
asl on **20 of the 31** measured rows, including 5 of the 11 RAM rows (it
overshoots `$FFFFB001` and `$FFFFB101`, and misses every non-power-of-two `n`).
It happened to be right on the four rows July took.

So this was never "we were simply wrong" — it is a **dropped condition**, the
charitable hypothesis, confirmed. The measurement was sound and is preserved in
the new tests.

Two hypotheses that were checked and are dead:

- **A different asl binary.** Both Linux-x86_64 builds in the workspace answer
  `$B100` on `p1`: `s2disasm` (flamewing, `x86_64-Linux`) and `s1disasm`/
  `skdisasm`/`sonic_hack` (upstream, `x86_64-unknown-linux`), both self-reporting
  1.42 Beta Bld 212. The binary is not the variable.
- **Probe-shape / flag drift.** The corpus flags reproduce the disagreement on a
  character-for-character identical source. Nothing about `-U` is involved.

## What changed

- `crates/sigil-ir/src/align.rs` — new. `asl_align_pad(pc, n)`, the one rule,
  with the 31 measured rows as its gate.
- `crates/sigil-frontend-as/src/eval.rs` — `directive_align` calls it. `disp`
  now decides only the KIND of pad (a phased RAM region reserves; a ROM section
  emits a real `$00` fill), not the arithmetic.
- `crates/sigil-frontend-emp/src/lower/regions.rs` — `align_to` calls it. It had
  carried a second copy of the rule, documented as mirroring the first; the two
  copies encoded a rule neither of them held.

`.emp`'s own top-level `align N` item (`emit_align_pad`, D2.29) is deliberately
NOT changed. It is a language feature with its own link-time congruence assert,
not an asl transliteration, and its sections are ROM where the two rules
coincide. Aeon has one `@align` (`Player_Pos_Ring`) and ten `align` items, all
`align 2` or `align $8000`.

Two asl behaviours remain unmodelled, both outside the corpus and both recorded
at the function: asl truncates `n` to a 16-bit `Word` (`align -256` acts as
`align $FF00`; `align 0` aborts asl with SIGFPE), and asl carries the PC wider
than 32 bits, so an align off the top of the space lands at `$1_0000_0000` where
sigil wraps to `$0000_0000`.

## Why no gate could see this

Aeon's four shapes reproduce the frozen goldens exactly, and that was never
evidence: the goldens were produced by this implementation, so a wrong rule is
carried identically by both sides of every byte comparison. This is the class the
`×26` stride bug sat in. `asl` is the only admissible oracle, and the CRCs below
are reported as a consequence, never as an argument.

What kept the defect invisible in practice is that aeon never exercises the two
cases where the wrong rule and the right one differ: its `.asm` sources carry no
`phase` at all (so `directive_align` always took its *correct* non-phase branch),
and `regions.rs`'s single `@align(256)` sits at a RAM cursor whose low byte is
not `$01` — the only offset at which the two rules disagree for `n = 256`.

## Measured impact

<!-- filled in below once the suite and the four builds have run -->
