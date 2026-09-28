#!/usr/bin/env python3
"""Read the VERDICT a landing run recorded in its own log, and treat an ABSENT verdict as RED.

Run this instead of reading the log by eye, and never accept a landing on the
harness's reported exit code.

WHICH FILE. The argument is the run's LOG: the file `scripts/landing-run.sh` stamps
first and names on its `log ->` line (`--log <path>`, or `<target>/landing-*.log`). A
run appends its verdict block to that file inside a `##### VERDICT SPAN`, with the
script's own exit code as `LANDING_EXIT=`, and this checker reads the RESULT line from
that span and nowhere else, beside the `CARGO_EXIT=`, `CLIPPY_EXIT=` and `LEDGER_EXIT=`
lines of the run record above it. A captured stdout is NOT the log: it has the RESULT
and none of the exit lines, and is refused here with the log's path named.

WHAT THIS CHECKS AND WHAT `--verdict-only` CHECKS. This reads the verdict the run
RECORDED. `scripts/landing-run.sh --verdict-only <log>` RECOMPUTES a verdict from the
same run record under the rules as they stand today, and writes nothing. On a log a run
wrote they give the same verdict; they can differ only when a verdict rule changed
between the run and the re-judge, and then the re-judge is the stricter reading.

A LOG WRITTEN BEFORE THE VERDICT SPAN EXISTED carries the exit lines and no verdict.
It is refused here (exit 2) with the command that judges it, `--verdict-only`, named: this
checker has nothing recorded to read, and it does not recompute.

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
resolves to non-zero: missing file, empty file, no verdict span, a span without its
RESULT or its exit code, more than one span, a truncated run, a GREEN that its own exit
lines contradict, or a verdict that is not GREEN. There is no input for which it is
silent.

Exit 0 only for a log whose recorded verdict says GREEN, whose `LANDING_EXIT` is 0 and
whose three gate exit lines are present and 0. Exit 1 for an explicit non-green verdict.
Exit 2 for anything that cannot be read as a recorded verdict at all, which includes the
never-ran case, the pre-span log and a captured stdout.
"""

import os
import re
import sys

SPAN_OPEN = "##### VERDICT SPAN,"
SPAN_CLOSE = "##### VERDICT SPAN ENDS"
VERDICT = re.compile(r"^\s*RESULT\s+(.+?)\s*$")
EXITS = ("CARGO_EXIT=", "CLIPPY_EXIT=", "LEDGER_EXIT=")
LOG_LINE = re.compile(r"^\s*log\s+(\S.*?)\s*$", re.M)


def last_value(lines, key):
    """The last `KEY=<n>` value, the one `landing-run.sh --verdict-only` reads too."""
    found = None
    for line in lines:
        if line.startswith(key):
            found = line[len(key):].strip()
    return found


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

    lines = text.splitlines()
    opens = [i for i, l in enumerate(lines) if l.startswith(SPAN_OPEN)]
    record = lines[: opens[0]] if opens else lines
    reached = [e.rstrip("=") for e in EXITS if last_value(record, e) is not None]

    if not opens:
        stray = [m for l in lines if (m := VERDICT.match(l))]
        if stray and not reached:
            named = LOG_LINE.search(text)
            print(f"NO VERDICT SPAN in {path}, and none of the exit lines. RED.", file=sys.stderr)
            print("  This file carries a RESULT line and no run record: it is a captured", file=sys.stderr)
            print("  stdout, not the log. The log is the file a run names on its `log` line", file=sys.stderr)
            print(f"  ({named.group(1) if named else 'not named in this file'}); check that.", file=sys.stderr)
            return 2
        if len(reached) == len(EXITS):
            print(f"NO RECORDED VERDICT in {path}. Not GREEN, and not a finding about the code.", file=sys.stderr)
            print("  The run finished (all three exit lines are here) and wrote no verdict span:", file=sys.stderr)
            print("  the log predates landing-run.sh writing its verdict into the log, or the run", file=sys.stderr)
            print("  was killed between its last gate and its verdict. This checker reads a", file=sys.stderr)
            print("  recorded verdict and does not recompute one. Judge this log with", file=sys.stderr)
            print(f"    scripts/landing-run.sh --verdict-only {path}", file=sys.stderr)
            return 2
        print(f"NO VERDICT LINE in {path}. RED.", file=sys.stderr)
        print(f"  log is {len(text)} characters, so the run started and did not finish.", file=sys.stderr)
        print(f"  gates that reported: {', '.join(reached) if reached else 'none'}", file=sys.stderr)
        print("  An absent verdict is a failed landing, never an unmeasured one.", file=sys.stderr)
        return 2

    if len(opens) > 1:
        print(f"{len(opens)} VERDICT SPANS in {path}. RED: a run writes exactly one, so this log", file=sys.stderr)
        print("  was written into by more than one run, and which verdict belongs to which", file=sys.stderr)
        print("  record cannot be read from it.", file=sys.stderr)
        return 2

    span = lines[opens[0] + 1:]
    if SPAN_CLOSE not in span:
        print(f"TRUNCATED VERDICT SPAN in {path}: no `{SPAN_CLOSE}` line. RED.", file=sys.stderr)
        return 2
    span = span[: span.index(SPAN_CLOSE)]
    landing_exit = last_value(span, "LANDING_EXIT=")
    verdicts = [m.group(1) for l in span if (m := VERDICT.match(l))]

    exits = {e.rstrip("="): last_value(record, e) for e in EXITS}
    if verdicts:
        print(f"VERDICT: RESULT {verdicts[-1]}")
    else:
        print("VERDICT: the span has no RESULT line")
    print(f"  LANDING_EXIT={landing_exit if landing_exit is not None else 'NOT RECORDED'}")
    for name, value in exits.items():
        if value is None:
            print(f"  {name} NOT REPORTED (the gate did not run)")
        else:
            print(f"  {name}={value}")

    if landing_exit is None or not landing_exit.isdigit():
        print("\nRED: the verdict span records no LANDING_EXIT, so the run's own status is unknown.", file=sys.stderr)
        return 2
    if not verdicts:
        print(f"\nRED: landing-run.sh exited {landing_exit} without a RESULT line. Read the span.", file=sys.stderr)
        return 2 if landing_exit == "2" else 1
    if len(verdicts) > 1:
        print(f"\nRED: the span carries {len(verdicts)} RESULT lines and a verdict has one.", file=sys.stderr)
        return 2

    verdict = verdicts[0].strip()
    if verdict.upper().startswith("GREEN BUT UNRECONCILED"):
        print("\nNOT A PASS: green but unreconciled. The totals do not match the baseline.", file=sys.stderr)
        return 1
    if verdict.upper().startswith("GREEN"):
        missing = [n for n, v in exits.items() if v is None]
        nonzero = [f"{n}={v}" for n, v in exits.items() if v is not None and v != "0"]
        if missing or nonzero or landing_exit != "0":
            print("\nINCONSISTENT, RED: the recorded verdict says GREEN and the run record", file=sys.stderr)
            print(f"  does not: missing {missing or 'none'}, nonzero {nonzero or 'none'},", file=sys.stderr)
            print(f"  LANDING_EXIT={landing_exit}. A green that its own exit lines contradict", file=sys.stderr)
            print("  is not a green. Re-judge with scripts/landing-run.sh --verdict-only.", file=sys.stderr)
            return 2
        return 0
    print(f"\nRED: {verdict}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
