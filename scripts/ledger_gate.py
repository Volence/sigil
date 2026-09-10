#!/usr/bin/env python3
"""THE LEDGER GATE. Three assertions over `docs/*.jsonl`, at three different strengths.

WHY THIS EXISTS. `tools/decisions_reader_audit.py` was committed, correct, and had no
caller anywhere in the tree. On 2026-08-30 a hand run of it reported 3 of 16 decision
lines unrenderable on the owner's console; on 2026-09-09 the same tool over the same
file reported 12 of 36. The file doubled and the unrenderable share went 19% -> 33%
with nobody noticing, because nothing ran it. AN UNWIRED AUDIT DOES NOT STOP THE NUMBER
GROWING, IT STOPS ANYONE NOTICING THAT IT GREW. This script is the caller.

THE THREE ASSERTIONS, and why they are deliberately not the same strength:

(1) EVERY LINE OF EVERY `docs/*.jsonl` PARSES AS JSON, AND EVERY FILE ENDS IN A NEWLINE.
    HARD. Green when written and expected to stay green. This is the incident in
    docs/OVERSEER-ROW-HISTORY.md's LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE row: a
    file whose last line carried no newline took an append that CONCATENATED the new
    object onto the previous one and produced a single unparseable line. It destroyed
    the PREVIOUS session's record as well as mangling the new one, it was silent, and
    both contracts call these files append-only and never-backfilled, so the damage is
    the kind that cannot be honestly reconstructed. That row asks for exactly the first
    half of this: "a suite-side check that every .jsonl in docs/ parses line-by-line".
    The trailing-newline half is the same defect caught one step EARLIER — before a
    record is destroyed rather than after — and its remedy (append one newline byte)
    adds no record, so it is legal on an append-only file.

(2) CONSOLE-RENDERABILITY IS A RATCHET, NEVER A HARD GATE. It fails on GROWTH past the
    pin below and on nothing else. A hard gate here would be RED ON ARRIVAL against
    correct, ratified history — three of the current rejects (d-14, d-15, d-16) are an
    already-listed non-repair under DECISIONS.md rule 8f — and the remedy a reasonable
    person reaches for when a gate is red against correct history is WEAKENING THE
    GATE. A ratchet asks for the one thing that is actually in the author's hands: do
    not add a thirteenth.

(3) ID-UNIQUENESS IS REPORTED AND IS NOT A GATE. `d-18` appears three times and `d-25`
    twice. Contract rule 8e (empyrean 2d8ab09) rules that id-uniqueness wins narrowly
    and that re-iding the LATER line is the single sanctioned in-place edit to an
    append-only file; oracle's tools/land.sh G2b refuses every landing on this shape.
    That re-id is a separate decision and a separate parcel, and wiring the assertion
    before making it would hand this repo an unlandable master. So this half prints and
    returns nothing to the exit code. It is labelled REPORTING ONLY on every line it
    writes, because a bar reported beside a green verdict is a bar that gets landed
    over, and the only defence against that is saying which one it is out loud.

UNMEASURABLE IS NOT GREEN, AND EMPTY IS NOT CLEAN. Every "nothing found" answer below
is checked against an instrument that could have answered otherwise: the file set must
be non-empty, the ledger must have lines, and the audit's own result count must equal a
line count this script derives independently. An audit silently returning zero results
is the failure mode a wired-but-vacuous check has, and it exits 2 here rather than
printing "0 rejected".

EXIT CODES
  0  every assertion holds
  1  an assertion FAILED (unparseable JSON, a missing trailing newline, or the ratchet
     moved backwards). The caller must treat this as a red run.
  2  the gate COULD NOT MEASURE. Never a count, never green.

WHERE THE VERDICT GOES. scripts/landing-run.sh runs this inside its own command span,
writes this exit code to the log as `LEDGER_EXIT=`, and that variable sits in the SAME
verdict condition as CARGO_EXIT, CLIPPY_EXIT and the skip count -- see the block marked
`(9) THE LEDGER GATE` there and the `RESULT FAILED` condition it names. The two halves
are three hundred lines apart in that file, which is the shape that makes a correct
collect-then-decide gate read as decorative, so each half names the other by marker.
"""

from __future__ import annotations

import argparse
import collections
import importlib.util
import json
import sys
from pathlib import Path

# THE RATCHET PIN. Derived, not copied: measured by running
#   python3 tools/decisions_reader_audit.py docs/decisions.jsonl
# in this tree at sigil f48a19cd on 2026-09-09, which reported
#   lines: 36 total / 24 parse / 12 rejected
#
# RAISING THIS NUMBER IS A DECISION, NOT A FIX. It means a new decision entry the owner
# cannot see on his console was accepted into the ledger; the entry is what should
# change. Rule 8f allows a repair only when a gate is red and the repair clears it, and
# decisions.jsonl is append-only, so a rejected line that has already landed is listed
# and left -- which is precisely why this is a ratchet and not a target of zero.
#
# LOWERING IT IS FREE AND WANTED. The gate says so itself when it measures fewer.
PIN = 12

# The ledger the ratchet is about. Assertion (1) covers every `docs/*.jsonl`; assertion
# (2) is Dominion's decision-card reader and reads only this one.
LEDGER_NAME = "decisions.jsonl"

# Machine-readable prefix. scripts/landing-run.sh lifts every line carrying it out of
# the ledger span and reprints it in the verdict block, so the number this gate measured
# is visible in the verdict a merge reads and not only in the log body.
TAG = "LEDGER:"


class Unmeasurable(Exception):
    """The gate could not measure its subject, so no count of any kind is honest."""


def emit(text: str = "") -> None:
    print(f"{TAG} {text}" if text else TAG, flush=True)


def load_audit_module(repo: Path):
    """Import tools/decisions_reader_audit.py by path.

    Imported rather than shelled out to and screen-scraped: this gate needs the result
    per line, and re-parsing the tool's prose report would make a change to its wording
    look like a change to the ledger. A missing or unimportable tool is UNMEASURABLE --
    the one thing it must not become is a green run with no audit in it.
    """
    path = repo / "tools" / "decisions_reader_audit.py"
    if not path.is_file():
        raise Unmeasurable(
            f"the audit tool is not at {path}. The renderability assertion has no "
            f"implementation to run, and a run without it is not a run with nothing to "
            f"report."
        )
    # A GATE MUST NOT DIRTY THE TREE IT MEASURES. Importing by path writes
    # `tools/__pycache__/` beside the tool, that directory is not in .gitignore, and a
    # landing run stamps its log with whether the checkout is DIRTY. A gate whose own
    # execution can flip that stamp is a gate that changes the answer by being run.
    # Measured here: the first hand run left the directory behind.
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("decisions_reader_audit", path)
    if spec is None or spec.loader is None:
        raise Unmeasurable(f"cannot load {path} as a python module")
    module = importlib.util.module_from_spec(spec)
    # Registered BEFORE exec_module: the tool declares @dataclass classes, and
    # dataclasses resolves a class's module out of sys.modules while the module body is
    # still executing. Without this line the import fails with an AttributeError that
    # says nothing about the real cause -- measured here on the first run.
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # noqa: BLE001 - any import failure is unmeasurable
        del sys.modules[spec.name]
        raise Unmeasurable(f"{path} failed to import: {exc!r}") from None
    for name in ("audit", "Unmeasurable"):
        if not hasattr(module, name):
            raise Unmeasurable(
                f"{path} has no `{name}`; this gate was written against a different "
                f"version of that tool and cannot honestly claim to have run it"
            )
    return module


def content_lines(path: Path) -> list[tuple[int, str]]:
    """Every non-blank line of a file, with its 1-based number.

    This is the gate's OWN count, derived here and not taken from the audit tool. It is
    what makes a vacuous audit detectable: see `check_renderability`.
    """
    text = path.read_text(encoding="utf-8")
    return [(n, raw) for n, raw in enumerate(text.split("\n"), start=1) if raw.strip()]


# ---------------------------------------------------------------------------------------
# (1) HARD: every line parses, every file ends in a newline.
# ---------------------------------------------------------------------------------------
def check_json(files: list[Path]) -> bool:
    total = 0
    bad: list[str] = []
    no_newline: list[str] = []

    for path in files:
        try:
            raw = path.read_bytes()
        except OSError as exc:
            raise Unmeasurable(f"cannot read {path}: {exc}") from None
        if raw and not raw.endswith(b"\n"):
            no_newline.append(path.name)
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise Unmeasurable(f"{path} is not valid UTF-8: {exc}") from None
        for number, line in enumerate(text.split("\n"), start=1):
            if not line.strip():
                continue
            total += 1
            try:
                json.loads(line)
            except ValueError as exc:
                bad.append(f"{path.name}:{number}  {exc}")

    ok = not bad and not no_newline
    emit(
        f"json      {len(files)} file(s), {total} line(s), {len(bad)} unparseable, "
        f"{len(no_newline)} missing a trailing newline  -- HARD, "
        f"{'ok' if ok else 'FAILED'}"
    )
    for entry in bad:
        emit(f"  unparseable  {entry}")
    for name in no_newline:
        emit(f"  no trailing newline  {name}")
    if not ok:
        emit("  A line that does not parse is a record that no reader can read, and these")
        emit("  files are append-only and never backfilled, so it cannot be reconstructed")
        emit("  later. A file with no trailing newline is the SAME defect one step earlier:")
        emit("  the next append concatenates onto the last record and destroys it too.")
        emit("  Fix a missing newline with `printf '\\n' >> docs/<file>.jsonl` -- that adds")
        emit("  a byte, not a record, so it is legal on an append-only file.")
    return ok


# ---------------------------------------------------------------------------------------
# (2) RATCHET: console-renderability may shrink, never grow.
# ---------------------------------------------------------------------------------------
def check_renderability(repo: Path, ledger: Path, pin: int, pin_origin: str) -> bool:
    module = load_audit_module(repo)
    emit(f"pin       {pin} ({pin_origin})")

    try:
        results = module.audit(str(ledger))
    except module.Unmeasurable as exc:
        raise Unmeasurable(f"the audit tool could not measure {ledger}: {exc}") from None

    # THE VACUITY CONTROL. An audit that silently returns nothing would otherwise report
    # "0 rejected" and read as the cleanest possible ledger. The line count is derived
    # here, from the file, by code that shares nothing with the audit but the filename;
    # a disagreement between the two is UNMEASURABLE, never a count.
    expected = content_lines(ledger)
    if not expected:
        raise Unmeasurable(
            f"{ledger} has no content lines. A ledger with nothing in it is not a ledger "
            f"with nothing wrong; there is no renderability figure to state."
        )
    if len(results) != len(expected):
        raise Unmeasurable(
            f"the audit returned {len(results)} result(s) for {ledger}, which has "
            f"{len(expected)} content line(s). The audit did not measure this file: a "
            f"count taken from it now would be a number about something else."
        )

    rejected = [r for r in results if not r.ok]
    count = len(rejected)
    distance = count - pin
    ok = count <= pin

    emit(
        f"render    {ledger.name}  {len(results)} line(s) / {len(results) - count} render "
        f"/ {count} rejected   pin {pin}, distance {distance:+d}  -- RATCHET, "
        f"{'ok' if ok else 'FAILED'}"
    )
    for result in rejected:
        shown = result.id if result.id is not None else "(no id)"
        emit(f"  rejected  line {result.line}  {shown}  -- {'; '.join(result.reasons)}")

    if not ok:
        emit(
            f"  THE RATCHET MOVED BACKWARDS: {distance} more line(s) than the pin of {pin} "
            f"are invisible"
        )
        emit("  on the owner's console. Every id above is a decision he cannot see. This is")
        emit("  a ratchet, so it can only be red because something was ADDED -- fix the new")
        emit("  entry, do not raise PIN in scripts/ledger_gate.py. Raising it accepts an")
        emit("  unreadable decision permanently, because the file is append-only.")
    elif distance < 0:
        emit(
            f"  THE RATCHET CAN TIGHTEN: {-distance} fewer than the pin. Lower PIN in "
            f"scripts/ledger_gate.py"
        )
        emit(f"  to {count} so the ground that was won is held.")
    return ok


# ---------------------------------------------------------------------------------------
# (3) REPORTING ONLY: id uniqueness. Never touches the exit code.
# ---------------------------------------------------------------------------------------
def report_ids(ledger: Path) -> None:
    counts: collections.Counter[str] = collections.Counter()
    unreadable = 0
    for _number, raw in content_lines(ledger):
        try:
            parsed = json.loads(raw)
        except ValueError:
            unreadable += 1
            continue
        if isinstance(parsed, dict):
            entry_id = parsed.get("id")
            if isinstance(entry_id, str) and entry_id.strip():
                counts[entry_id] += 1

    repeats = sorted((i, n) for i, n in counts.items() if n > 1)
    shown = ", ".join(f"{i} x{n}" for i, n in repeats) if repeats else "none"
    note = f", {unreadable} line(s) unreadable" if unreadable else ""
    emit(
        f"ids       {sum(counts.values())} id(s){note}, {len(repeats)} repeated: {shown}"
        f"  -- REPORTING ONLY, NOT A GATE"
    )
    if repeats:
        emit("  Contract rule 8e rules id-uniqueness wins narrowly and that re-iding the")
        emit("  LATER line is the single sanctioned in-place edit to an append-only file.")
        emit("  That re-id is its own decision and its own parcel. This line does NOT")
        emit("  affect this gate's exit code, deliberately: wiring it before the re-id is")
        emit("  made would leave master unlandable.")


def resolve_files(docs: Path) -> list[Path]:
    if not docs.is_dir():
        raise Unmeasurable(
            f"{docs} is not a directory, so the set of ledgers to check could not be "
            f"built. Nothing was measured."
        )
    files = sorted(p for p in docs.glob("*.jsonl") if p.is_file())
    # AN EMPTINESS IS NOT A FINDING WITHOUT AN INSTRUMENT THAT COULD HAVE RETURNED
    # NON-EMPTY. Zero files means the glob, the path, or the tree is wrong -- it does
    # not mean every ledger is clean, and rendering it as a pass is how a gate becomes
    # decorative without anyone editing it.
    if not files:
        raise Unmeasurable(
            f"no *.jsonl files under {docs}. A gate that checks nothing passes trivially, "
            f"so this is a refusal: either the path is wrong or the ledgers moved."
        )
    return files


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Assert what the owner's console can render out of docs/*.jsonl."
    )
    parser.add_argument(
        "--docs",
        default=None,
        help="the directory of ledgers (default: docs/ beside this script's repo root)",
    )
    parser.add_argument(
        "--repo",
        default=None,
        help="the checkout holding tools/decisions_reader_audit.py (default: derived)",
    )
    parser.add_argument(
        "--pin",
        type=int,
        default=None,
        help=(
            "override the ratchet pin. FOR TESTS. A landing run passes no --pin, so the "
            "pin a landing enforces is the PIN constant in this file and nothing a "
            "command line can move."
        ),
    )
    args = parser.parse_args(argv)

    repo = Path(args.repo).resolve() if args.repo else Path(__file__).resolve().parent.parent
    docs = Path(args.docs).resolve() if args.docs else repo / "docs"
    pin = PIN if args.pin is None else args.pin
    pin_origin = (
        "built-in constant PIN in scripts/ledger_gate.py"
        if args.pin is None
        else "--pin OVERRIDE on the command line, NOT the landing pin"
    )

    try:
        files = resolve_files(docs)
        emit(f"docs      {docs}")

        json_ok = check_json(files)

        ledger = docs / LEDGER_NAME
        if not ledger.is_file():
            raise Unmeasurable(
                f"{ledger} does not exist, so the renderability ratchet measured nothing. "
                f"The decisions ledger is the subject of this gate."
            )
        render_ok = check_renderability(repo, ledger, pin, pin_origin)

        report_ids(ledger)
    except Unmeasurable as exc:
        # Its own exit code and its own words, on stderr AND in the tagged stream, so a
        # log reader and a verdict parser both see it. Rendering this as 0 rejected would
        # read as the cleanest ledger this repo ever had.
        emit(f"UNMEASURABLE  {exc}")
        emit("result    UNMEASURABLE -- this is not a count and it is not green.")
        print(f"ledger-gate: UNMEASURABLE, {exc}", file=sys.stderr)
        return 2

    ok = json_ok and render_ok
    emit(f"result    {'ok' if ok else 'FAILED'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
