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

**Suite** — `cargo test --workspace --no-fail-fast`, `AEON_DIR=/home/volence/sonic_hacks/.aeon-as-fold`
(detached at aeon `4f5ad5a1`), sigil `b1f0cc06` on `parcel/as-align-in-phase`,
started 17:39:17 finished 17:52:43: **4319 passed, 0 failed, 2 ignored, runner
exit 0.** Nothing redded that needed adjudicating. The one test that encoded the
wrong rule was rewritten before this run; a run at the fix-only commit reddened
only it.

(A first run of the same command reported 371 failures and was DISCARDED, not
adjudicated: `AEON_DIR` was unset and every one was the d-18 refusal to measure
against an unnamed reference tree. A second was discarded for being compiled
across an edit.)

**Aeon's four shapes — BYTE-IDENTICAL, and the CRCs are a consequence, not the
argument.** Each ROM was deleted before its build so a failed build could not
present a stale artifact as an identical CRC (the first attempt did exactly that:
two shapes exited 1 for want of `SIGIL_EMIT` and their untouched files compared
equal). All four then built exit 0 from nothing:

| shape | CRC32 | size | vs frozen baseline |
|---|---|---:|---|
| `s4.bin` | `14ee2440` | 719700 | identical, 0 differing bytes |
| `s4.debug.bin` | `142294b3` | 737683 | identical, 0 differing bytes |
| `demo.bin` | `0c456778` | 96474 | identical, 0 differing bytes |
| `demo.debug.bin` | `2e603d53` | 101339 | identical, 0 differing bytes |

**Why it is byte-neutral, measured rather than assumed.** Aeon reaches the rule
at exactly two kinds of site and neither lands on a disagreeing address:

- The three `.asm` sources (`games/*/game_root.asm`, `engine/debug/debugger.asm`)
  contain **no `phase` at all**, so `directive_align` always took its non-phase
  branch — which was already the correct rule. `debugger.asm`'s 21 `!align`s are
  all `align 2` at ROM addresses.
- `regions.rs`'s single `@align(256)` (`Player_Pos_Ring`) sits at a RAM cursor
  whose low byte is `$1A` (plain) and `$50` (debug) — and for `n = 256` the old
  rule and the true one differ **only** when the low byte is exactly `$01`
  (510 of 65536 RAM addresses differ across all `n`; 1 in 256 for `n = 256`).

The overshoot itself is live and shipped, so this was never a dead rule:

```
s4.lst        2324/FFFFBA18 : Player_Bound_Bottom:     (u16, cursor -> $FFFFBA1A)
              2325/FFFFBC00 : Player_Pos_Ring:         naive round-up: $FFFFBB00
s4.debug.lst  2765/FFFFE34E : Player_Bound_Bottom:     (u16, cursor -> $FFFFE350)
              2766/FFFFE500 : Player_Pos_Ring:         naive round-up: $FFFFE400
```

`demo` has no `@align` and no `Player_Pos_Ring` at all.

**What this did NOT do:** nothing under `crates/sigil-harness/golden/`,
`src/pins.rs` or `repin.toml` was touched. There was no byte movement to
reconcile, and re-freezing is not this parcel's act in any case.

## Left open

- `parcel/as-reserve-materialise` (`68386152`) is unblocked. It conflicts with
  master in 5 hunks of `crates/sigil-frontend-as/src/eval.rs`, all comment
  reflow, and **the same 5 hunks conflict against plain master** — this parcel
  neither adds nor removes any. Its blocked expectation is now three tests, and
  the branch's own p2bin witness (258 bytes ending `b1 00` at `$100`) is exactly
  what the phased-ROM one should assert once a reservation materialises. Landing
  it is a separate parcel.
- `.emp`'s top-level `align N` item (`emit_align_pad`) still rounds up plainly.
  Correct for every ROM section, which is all aeon has; a `vars`-side use would
  need the shared rule, and its link-time congruence assert would hold either way
  (asl's result is always a multiple of `n`).
- Two asl corners left unmodelled on purpose, recorded at `asl_align_pad`: `n`
  truncated to a `Word`, and a PC that leaves the 32-bit space.
