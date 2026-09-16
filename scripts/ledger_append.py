#!/usr/bin/env python3
# VENDORED FROM EMPYREAN, NOT WRITTEN HERE.
# Source: empyrean scripts/ledger_append.py at 51a48d4, verified at adoption time
# as an ancestor of their origin/main. Cited by SHA so staleness is detectable
# rather than invisible; empyrean's copy governs on any disagreement, and a
# divergence between the two is a finding rather than a value to reconcile.
#
# ADOPTED 2026-09-16 as the PREVENTION half of this lane's open row
# LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE. scripts/ledger_gate.py is the
# DETECTION half and stays: this refuses to corrupt, that refuses to land if
# something else did.
#
# BOTH ARMS RE-VERIFIED HERE rather than taken from the author's report, since a
# peer's red-first is their custody and not this lane's:
#   arm 1, no trailing newline -> repaired, 3 parseable lines, BOTH prior records
#           intact ({"a":1} and {"b":2} recovered beside the new {"c":3});
#   arm 2, already-damaged tail -> exit 1, and md5 identical before and after,
#           which is the half the report asserted and did not show.
#
# THE RESIDUAL, NAMED RATHER THAN GLOSSED: by aeon's own removes-the-need-to-
# remember test this tool is first-best at the WRITE and still carries an
# obligation one layer out, because somebody has to remember to call it instead
# of hand-appending. That residual is bounded, not closed: a hand-append that
# corrupts is caught by ledger_gate.py at the next landing, before a push, and
# the damage is recoverable by raw_decode-splitting the welded line, which this
# lane has already done once. What is NOT recovered is the interval in between.
"""Append one record to a .jsonl ledger without being able to corrupt the previous one.

THE HAZARD (sigil, 2026-09-16, row LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE):
every lane appends to docs/lane-log.jsonl and docs/decisions.jsonl with
`open(p, "a").write(json.dumps(rec) + "\\n")`. That is correct ONLY if the file
already ends in a newline. When it does not, the new record is welded onto the
previous line and BOTH become unparseable -- and it destroys the PREVIOUS
session's record, in files LANE_LOG.md and DECISIONS.md both call append-only and
never-backfilled, which is to say unreconstructable.

Sigil built the DETECTION half (a gate that refuses a landing when a ledger line
does not parse or a file lacks its trailing newline). This is the PREVENTION half,
which by aeon's test is the first-best shape: a gate needs someone to run it, a
write that cannot corrupt needs nobody to remember anything.

    python3 scripts/ledger_append.py docs/lane-log.jsonl '<json object>'
    python3 scripts/ledger_append.py docs/lane-log.jsonl --file rec.json

Refuses (exit 1, nothing written) when the record is not a JSON object, or when
the file's last line does not parse -- a ledger that is ALREADY damaged must not
be appended to, because the append buries the damage under a good record.
"""
import json
import os
import sys


def append(path: str, record: dict) -> None:
    if not isinstance(record, dict):
        sys.exit("record must be a JSON object")
    if os.path.exists(path) and os.path.getsize(path) > 0:
        with open(path, "rb") as f:
            f.seek(-1, os.SEEK_END)
            needs_nl = f.read(1) != b"\n"
        # A damaged tail must stop the append, not be buried by it.
        with open(path, encoding="utf-8") as f:
            tail = [ln for ln in f.read().split("\n") if ln.strip()]
        if tail:
            try:
                json.loads(tail[-1])
            except Exception as e:
                sys.exit(f"REFUSING: {path} last line does not parse ({e}). "
                         "Repair it before appending; an append hides the damage.")
    else:
        needs_nl = False
    with open(path, "a", encoding="utf-8") as f:
        if needs_nl:
            f.write("\n")
            print(f"repaired: {path} lacked its trailing newline", file=sys.stderr)
        f.write(json.dumps(record, ensure_ascii=False) + "\n")


def main() -> None:
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    path = sys.argv[1]
    if sys.argv[2] == "--file":
        record = json.load(open(sys.argv[3], encoding="utf-8"))
    else:
        record = json.loads(sys.argv[2])
    append(path, record)


if __name__ == "__main__":
    main()
