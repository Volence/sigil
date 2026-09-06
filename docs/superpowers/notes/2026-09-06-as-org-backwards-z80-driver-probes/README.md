# Probe kit for `2026-09-06-as-org-backwards-z80-driver.md`

These are PROBES, not a gate. They insert `warning` lines into a prepared corpus
copy so both assemblers report the location counter either side of a backwards
`org`, and they carry no verdict of their own. Nothing here runs in a suite.

## Reproduce

Prepare each corpus in its own detached worktree first
(`scripts/corpus-prepare.sh`), keep an unprobed copy so the published count still
reproduces, and probe a COPY:

```
cp -a <prepared>/s1disasm <probe>/s1disasm
docs/.../probe_s1.py <probe>/s1disasm
docs/.../run_asl.sh   <probe>/s1disasm sonic.asm /tmp/asl_s1
( cd <probe>/s1disasm && SIGIL_WARNINGS=full sigil sonic.asm )
```

For skdisasm, `run_asl.sh` takes the corpus's own flag as a trailing argument:
`run_asl.sh <corpus> sonic3k.asm /tmp/asl_sk -D Sonic3_Complete=0`. Without it
asl exits 2 on `#1820 expression must be evaluatable in first pass`, and the
guard refuses the run rather than letting its listing be quoted.

## What each run must show

Stated before running, because a probe that cannot come out the other way proves
nothing. The org row is the one that must DIFFER between the two tools while the
rows above it agree in shape; if `PROBE-2` reads the same on both sides, the
mechanism in the note is refuted rather than confirmed.

At the sigil binary and corpus revisions the note pins:

| | asl | sigil |
|---|---|---|
| `PROBE-1` before `save` | `72E7C` | `72240` |
| `PROBE-2` after `!org 0` | `0` | `72240` |
| `PROBE-3` `$` at the size check | `1BC6` | `73DFD` |
| `PROBE-S2` after `!org 0` (s2disasm) | `0` | `0`, and **sigil raises nothing** |

Those absolute values move with the sigil revision, because they are built on top
of whatever upstream bytes sigil did or did not emit. Read the SHAPE of each row,
not the number.
