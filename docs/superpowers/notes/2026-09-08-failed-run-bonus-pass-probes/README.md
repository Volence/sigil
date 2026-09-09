# The failed-run bonus-pass measurement, reproduced

Three scripts, exactly as run for
`../2026-09-08-failed-run-bonus-pass-measurement.md`. They expect the corpora as
PRIVATE copies under `/home/volence/sonic_hacks/.scratch/bonus-pass/corpus/`
(`rsync -a --exclude .git` from each shared checkout) and two binaries in
`.../bin/` named `sigil-base` and `sigil-cut`. Paths are absolute and named at
the top of each file; change those two constants to re-point them.

- `run_roots.sh <binary> <tag>` -- runs one binary over all nine AS roots,
  capturing stdout, stderr and exit status per root.
- `diff_roots.sh` -- BOTH-direction exact-line set compare of the two capture
  sets, plus a per-class decomposition, so a class appearing for the first time
  is visible when the totals barely move. The classifier drops the
  `file(line):` prefix and blanks backtick-quoted identifiers and bare integers.
- `timecpu.py [reps]` -- interleaved CPU-time comparison. Wall clock on this
  machine moves by more than 4x with load, so the compared figure is child CPU
  from `getrusage(RUSAGE_CHILDREN)`; wall and `/proc/loadavg` are recorded beside
  every rep so the contamination stays visible.

The convergence census in the note came from a fourth build, the cut plus one
`SIGIL_PROBE_CONVERGE`-gated `eprintln` at the convergence site printing `pass`,
`poison.len()`, the error count, the carried-fatal count and `force_relocate`.
It is not kept here: it is four lines against a moving file, and re-adding it is
cheaper than maintaining a patch of it.
