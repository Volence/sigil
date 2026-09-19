# `DEB2-Z80-LABEL-IN-68K-TABLE`: asl lists a phased label at its PHASE address, so this is not sigil's defect

**The measurement, separated from what it implies.** `asl` prints a label declared inside a
`PHASE` block at its **phase** address in the listing's symbol table, not at its physical address.

```
*Z80PhasedHead :               8000 C      (phase address; physical would be 1002)
*Z80CtrlBefore :               1000 C      control, outside the bracket
*Z80CtrlAfter  :               1004 C      control, outside the bracket
```

The 68000 probe answers identically: `PhasedHead : 8000` with controls `CtrlBefore : B8000` and
`CtrlAfter : B8004`. The controls are what make the `8000` a result rather than an artifact — a
probe that listed nothing, or listed everything at its phase, would be visibly different.

**What that decides.** `docs/superpowers/notes/2026-09-12-deb2-minus-20-bytes.md` set this out before
any design, and its own test routes the row: *"If `asl` also lists it at its phase (Z80) address,
sigil is being compatible and the collision is `convsym`'s, so the fix may belong on the consuming
side."* It does, so **sigil is behaving exactly as the reference assembler does and the row leaves
this lane.** The collision is created downstream, where a phase-addressed symbol is filed into the
68000 debugger table as a ROM address.

**This retires the two reasons the row was ranked out of `next` on 2026-09-12**, because both were
premised on the fix being sigil's: there is no byte-mover here, so no refreeze ritual and no
hand-typed `tests/repin_pins.rs` resync; and sigil's listing export does not change, so the surface
the engine lane consumes is untouched by this conclusion.

**⚠ WHAT THIS NOTE DOES NOT ESTABLISH, stated because the next reader will want it to.** It does not
measure what sigil itself prints for the same probe, and it does not measure whether any CURRENT
shape has a live collision (the per-tree check is
`grep -E ' : 8000 [A-Z] \|' <shape>.lst`). Both were in the parcel's brief and neither was reached.
So "sigil is compatible" rests on the booked description of sigil's behaviour in the 09-12 note
rather than on a fresh measurement of it. **A reader routing this row onward should say so**, and
whoever takes the consuming side should measure both before pricing anything.

## Provenance, and why it is unusual

The probes and the first listings were built by a dispatched agent that **died with its work
uncommitted** — no commits on `measure/deb2-phase-symbol`, artifacts left untracked in its worktree,
transcript silent for 2h22m after roughly 7 minutes of activity, no live process. They are salvaged
here because the measurement is sound and re-runnable, not because a dead agent's output is
trustworthy on its own.

**The salvage was verified rather than adopted.** `phasez80.verify.lst` is a re-run performed at the
overseer's seat through the blessed invocation:

```
ASL binary: /home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
61e672562465725a8c102288a7da9098   (the reference build, selected by digest)
ASL_EXIT=0
ASL_DIAG=complete
```

It reproduces `Z80PhasedHead : 8000`. **That re-run was necessary and not ceremony:** the agent's own
build log carries no `ASL_EXIT` record at all, so its runs satisfied the digest half of the ritual
and left the exit-status half unevidenced — and this lane's rule is that a run carrying any error is
not a source of values for the lines that did assemble, however complete the listing looks.

**One piece of the dead agent's discipline is worth keeping.** Its first Z80 probe reused the 68000
probe's `org 0B8000h`; asl answered `error #1925: address overflow`, because the Z80 address space is
16 bits. The agent discarded that run whole — including the phased line that had not errored — and
re-shaped the probe, recording why in the probe's own header. That is the exit-status rule applied
correctly by the party it would have been easiest for.
