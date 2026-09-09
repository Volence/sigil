# Why an assembler reads the source more than once, and what we actually do

Written 2026-09-09 for the owner, who asked, verbatim: *"So I see complaints about how AS does
multiple pass throughs/builds too, is there any way to just avoid that? why can't it just build on
the first run?"*

He is right that it is a real community complaint. Empyrean `origin/main`,
`docs/2026-09-02-as-community-feedback.md`, read here at that revision: *"Assembler uses an
infinitely scaling build pass system, if you don't write the code in a particular way you'll likely
have to build in four passes"*, and their mapping table calls pass count **the most repeated
complaint, three respondents**.

## Two different things are both called a pass, and only one of them ran in the measurement

**1. Evaluation passes.** The front end reads the whole source and works out what every name means.
`crates/sigil-frontend-as/src/eval.rs:261`, `for pass in 0..PASS_CAP` with `PASS_CAP = 16`. It stops
when two consecutive passes produce an identical symbol table (`:330`, `pass > 0 && env == prev`),
and if it ever failed to settle within 16 it stops and says so by name rather than looping.

**2. The relaxation fixpoint.** Choosing how wide each jump instruction has to be.
`crates/sigil-link/src/relax.rs:648`, described in its own text as a *bounded, grow-only* fixpoint:
a jump's size can grow when the code around it grows, and can never shrink back, which is what makes
it provably terminate rather than oscillate.

**The Sonic 1 measurement was all of kind 1 and none of kind 2.** That build FAILS, so it exits
before the linker ever runs, and the relaxation loop is never entered. The three traversals he was
shown are three evaluation passes.

## Of the three, how many are ours to delete

- **Pass 1 is unavoidable.** Something has to read the file.
- **Pass 2 is the proof.** A name used before it is defined has an unknown value on pass 1, so the
  values pass 1 computed may be wrong. The second pass recomputes them and, if nothing changed, that
  is the evidence they were right. Two consecutive agreeing tables is the smallest honest stopping
  rule. **This is the floor and it is not a defect.**
- **Pass 3 is ours, and only on a failing build.** It exists to do two things an ordinary pass
  cannot: turn a still-unresolved call target into a deferred cross-repo reference for the linker
  rather than an error, and keep section-label references symbolic so the linker can place them. On
  a run that has already failed there is no linker, so its work is computed and dropped.
  **Cutting it saves 33 percent of the Sonic 1 run.**

## So how do we compare, in his terms

**On a build that succeeds, we take the same number of passes as AS on the same source: two.**
Measured: AS takes 2 on the Sonic 1 disassembly, 2 on Sonic 2, and 2 on any source with a forward
reference; it takes 1 only on source with no forward reference anywhere. Our floor is 2 always.

**The difference is not the count, it is the ceiling.** AS's is the "infinitely scaling" system the
community names: write the code a particular way and you pay four. Ours cannot exceed 16 and errors
by name rather than quietly costing more, and the jump-sizing loop can only ever grow, which is what
bounds it. **"AS needs more passes if you write it wrong; we need two, always" is the honest
summary.**

## Could it build on the first run

**Partly, and the part that cannot is a real trade rather than a limitation of effort.**

A forward reference used in a FIXED-SIZE place is not the problem. `dc.w Some_Label` occupies two
bytes whatever the label turns out to be, so a single pass can leave a hole and fill it at the end.
That is a patch, not an iteration.

The circularity is only in places where **the value decides the size**. A short branch is 2 bytes
and reaches 128 bytes away; a long one is 4 and reaches anywhere. To choose, you need the distance.
The distance depends on the sizes of everything in between. That is genuinely circular, and no
amount of cleverness reads it in one pass.

**There are exactly two honest ways out, and both cost something he can see:**

1. **Always use the widest form.** One pass, no iteration, and the ROM grows: every short branch
   becomes a long one. On a console with a fixed cartridge budget that is the expensive currency.
2. **Iterate, and pick the smallest that reaches.** What we do, and what the second pass buys. The
   cost is reading the source twice; the return is a smaller ROM.

So the second pass is not overhead. **It is what is paying for the size of the cartridge.** A
one-pass mode is buildable and would be honest to offer, but it should be offered as what it is: a
faster build in exchange for a larger ROM.

The third pass, on failing builds only, is the one with no return at all, and that is the one worth
cutting.
