# Probe kit: what `org` means in asl

Four standalone probes and a runner, for
`docs/superpowers/notes/2026-09-06-as-org-backwards-fix.md`. They are PROBES, not
a gate: nothing here runs in a suite and none of them carries a verdict. The
committed regression that DOES gate is the `org_backwards_new_section` block in
`crates/sigil-frontend-as/tests/snippets_golden.txt`, whose bytes are minted from
the same asl by `gen_snippet_vectors`.

Each probe is standalone: no corpus, no prepared tree, no generated includes.

```
docs/superpowers/notes/2026-09-06-as-org-backwards-probes/run.sh \
    docs/superpowers/notes/2026-09-06-as-org-backwards-probes/p1_back.asm \
    ./target/release/sigil
```

## What each one asks, and what would refute the model

Written before running them. A probe whose two candidate answers are the same
proves nothing, so each row below says what the two models predict.

| probe | question | "org is a physical relocation, backward is illegal" | "org sets the PC absolutely" |
|---|---|---|---|
| `p1_back.asm` | emit at `$1000`, then `org $10` | a refusal, or a seek that leaves labels in `$1000` space | no diagnostic; `low = 10h` beside `start = 1000h`, and both byte runs in the image at their own addresses |
| `p2_saverestore.asm` | does `restore` put the PC back? | the PC returns to `1004h` | the PC stays where the org left it |
| `p4_pcsym.asm` | which token is the PC under each CPU | (not a model question, a fact the other probes depend on) | |
| `p5_overlap.asm` | a backward org onto ground a CLOSED region owns | | |

**The answers, on `asl` md5 `61e672562465725a8c102288a7da9098`.** `p1`: no
diagnostic, exit 0, `P1B pc=12h low=10h start=1000h`, and `p2bin` writes `AA BB`
at `$10` with `01 02 03 04` still at `$1000`. `p2`: `P2C` reads `11h`, the same
value `P2B` read, so `restore` does NOT carry the PC. Both refute the physical
model, and the second refutes it in a direction the first cannot see.

`p4` is the confounder check the other probes need. Under `cpu 68000` the PC is
`*` and `\{$}` raises `#1020 invalid symbol name`; under `cpu z80` it is exactly
the other way round, and `\{*}` raises `#1110 wrong number of operands` because
`*` is multiplication there. A probe that reads the PC with the wrong token for
the CPU in force measures that gap instead of the one it was written for.

`p5` is the one place the two assemblers still disagree AFTER the fix, and it is
recorded so the disagreement is not mistaken for a regression: asl lets a
backward org land on top of a region already emitted and `p2bin` resolves it
last-write-wins, while sigil reports `sections ... overlap in the image
(colliding pins)` and refuses. That is a property of a flat single-image linker
against AS's chunked `.p` file, not of this directive. See the note for what
depends on it.
