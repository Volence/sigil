# `align` inside a `phase`: our rule contradicts asl on an identical source

2026-09-03 · established by the overseer from a dead agent's surviving probes ·
**not yet fixed, and the fix is a byte-mover**

Two agents died to a 529 within the same overload event. Neither committed. This
note exists because the finding below was established from what they left on
disk, and an unbanked finding is one rotation from gone.

## The contradiction

`crates/sigil-frontend-as/src/eval.rs`, `align_inside_a_phase_advances_a_full_extra_block`,
asserts `image(src) == [0xB2, 0x00]` — `L = $B200` — for this source:

```
cpu 68000
padding off
phase $B000
ds.b 5
align 256
L:	dc.w L
dephase
```

The probe `p1.asm` is that source character for character. asl 1.42 Beta Bld 212
under the corpus flags (`-xx -n -q -A -L -U -i .`) answers:

```
       4/    B000 :                     	ds.b	5
       5/    B005 :                     	align	256
       6/    B100 : B100                L:	dc.w	L
```

and its symbol table carries `L : B100 C`.

**`asl` says `$B100`. We say `$B200`. The setups are comparable — this is not a
probe-shape disagreement, which was the live alternative and is now closed.**

Two further probes agree that the rule is a plain round-up of the **logical**
(phased) address, with no extra block:

| probe | phase | before `align 256` | asl's answer |
|---|---|---|---|
| `p1` | `$B000` | `$B005` after `ds.b 5` | **`$B100`** |
| `p2` | `$B040` | `$B045` after `ds.b 5` | **`$B100`** |
| `p3` | `$B040` | `$B040`, no reserve | **`$B100`** |

## What our rule claims, and where it came from

The test's own comment: *"asl 1.42 Bld 212 (live-probed 2026-07-08): ALIGN inside
a `phase` (padding off) advances by `round_up(pos + n, n)` — ALWAYS at least a
full `n` beyond `pos`."* Same tool version, opposite answer. The July probe is not
reproducible from the note, so **what differed about it is unknown** — flags are
the first candidate, since `-U` was not yet standard practice here and a parcel's
whole premise was later found to be an artifact of omitting it.

## Why no gate here can see this

The comment says the rule *"places Aeon's `Player_Pos_Ring` one 256-block higher
than a naive align would"* — so it decides shipped addresses, and
`engine/debug/debugger.asm` is in aeon's build path with 21 `align` occurrences.

**Aeon's four shapes reproduce the frozen goldens exactly, and that is not
evidence.** The goldens were produced by this implementation. A wrong rule is
carried identically by both sides of every byte comparison and they agree
perfectly. This is the class the `×26` stride bug sat in: a byte gate proves twin
agreement, never correctness. `asl` is the only admissible oracle.

## Open, and why this note stops short of a fix

- **The general rule is not established, only this case.** A dying agent's last
  line proposed `trunc_div(pc + n - 1, n) * n` on a **signed** PC — C truncating
  division rounding toward zero, overshooting a block for negative addresses —
  and said it was about to try to break it. **That is an unfalsified candidate,
  not a finding.** All three probes above are positive addresses and do not
  discriminate it.
- **The fix is a byte-mover for aeon**, so it lands through the aeon overseer's
  lane, not this one. It must not be shaped to preserve the CRCs: matching bytes
  produced by the wrong rule is preserving the defect to satisfy a gate.
- The parked `parcel/as-reserve-materialise` (`68386152`) waits on this — one of
  the five expectations it correctly reddens is the very test above.
