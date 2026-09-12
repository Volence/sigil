#!/usr/bin/env python3
"""Read a landing-run log's VERDICT, and treat an ABSENT verdict as RED.

Run this instead of reading the log by eye, and never accept a landing on the
harness's reported exit code.

WHY THIS EXISTS. Oracle measured three distinct causes, in three consecutive sessions,
of the harness reporting a landing as "completed (exit code 0)" when it was nothing of
the kind:

  1. a nested `&` inside a backgrounded call, so the tracked process was the outer
     shell: reported 0 while the log stopped mid-run;
  2. a chained command where the reported code belonged to a trailing `grep`, which
     announced a RED landing as exit 0;
  3. a WRONG PATH: `./land.sh` where the script is `tools/land.sh`. It died 127, ran
     zero gates, and the harness still reported 0.

The rule this lane first banked, *the log's own verdict line is the only truthful
artifact*, is sound against 1 and 2 and INSUFFICIENT against 3, because it assumes a
log with a verdict in it. **A script that never started leaves no verdict line at all,
and an absent verdict must read as RED rather than as missing information.** That is
the absence class: a command that failed and a command that found nothing produce the
same output, and only one of them leaves evidence.

So this checker is deliberately built so that every failure mode of ITS OWN subject
resolves to non-zero: missing file, empty file, no verdict line, truncated run, or a
verdict that is not GREEN. There is no input for which it is silent.

Exit 0 only for a log whose verdict line says GREEN. Exit 1 for an explicit non-green
verdict. Exit 2 for anything that cannot be read as a verdict at all, which includes
the never-ran case.
"""

import os
import re
import sys

VERDICT = re.compile(r"^\s*RESULT\s+(.+?)\s*$", re.M)
EXITS = ("CARGO_EXIT=", "CLIPPY_EXIT=", "LEDGER_EXIT=")


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: check_landing_log.py <log path>", file=sys.stderr)
        return 2
    path = sys.argv[1]

    if not os.path.exists(path):
        print(f"NO LOG at {path}.", file=sys.stderr)
        print("This is RED, not missing information: a run that never started leaves no log,", file=sys.stderr)
        print("and the harness reports exit 0 for a command that died on a wrong path.", file=sys.stderr)
        return 2

    text = open(path, encoding="utf-8", errors="replace").read()
    if not text.strip():
        print(f"EMPTY LOG at {path}. RED: the run produced no output at all.", file=sys.stderr)
        return 2

    found = VERDICT.findall(text)
    if not found:
        # The run started and died before writing its verdict. Say which gates it got
        # through, because that is what tells the operator where it stopped.
        reached = [e.rstrip("=") for e in EXITS if e in text]
        print(f"NO VERDICT LINE in {path}. RED.", file=sys.stderr)
        print(f"  log is {len(text)} bytes, so the run started and did not finish.", file=sys.stderr)
        print(f"  gates that reported: {', '.join(reached) if reached else 'none'}", file=sys.stderr)
        print("  An absent verdict is a failed landing, never an unmeasured one.", file=sys.stderr)
        return 2

    verdict = found[-1]
    print(f"VERDICT: RESULT {verdict}")
    for e in EXITS:
        for line in text.splitlines():
            if line.startswith(e):
                print(f"  {line}")
                break
        else:
            print(f"  {e.rstrip('=')} NOT REPORTED (the gate did not run)")

    if verdict.strip().upper().startswith("GREEN BUT UNRECONCILED"):
        print("\nNOT A PASS: green but unreconciled. The totals do not match the baseline.", file=sys.stderr)
        return 1
    if verdict.strip().upper().startswith("GREEN"):
        return 0
    print(f"\nRED: {verdict}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
