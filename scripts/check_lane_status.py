#!/usr/bin/env python3
"""Check docs/lane-status.json against EVERY size bound the contract states.

Run it before every write of the status file, and let it fail the write.

WHY THIS EXISTS, and it is not "because bounds are easy to forget". On 2026-09-12 this
lane breached the 240-character title bound, was corrected by the hub, added a write-time
assert for THAT bound, and breached the 120-character `focus` bound within the hour. The
assert covered the rule it had been corrected on rather than the rule set. Two hub
messages were spent on one lane's status file in one afternoon.

So the design constraint here is: **enumerate the bounds from the contract, never from
the last correction.** A checker that grows one bound per incident is a log of what
somebody already noticed, which is the shape it is replacing.

Bounds are `empyrean/contract/LANE_STATUS.md` rule 7 plus the `focus` field row. Read
them there rather than trusting this file, and when this file disagrees with the
contract, the contract wins and this file is the defect. The hub's own
`scripts/hub_check.py` (empyrean `f7d57b8`) checks the same set across all six lanes;
this is the local half so a breach fails at the author's end rather than arriving at
the hub's.

Exit 0 clean, 1 on any breach, 2 if the file cannot be read or parsed.
"""

import json
import os
import sys

# From empyrean contract/LANE_STATUS.md. Each carries the clause it comes from so a
# reader can check the number against its source rather than against this comment.
BOUNDS = {
    "focus_chars": (120, "the `focus` field row: one sentence, <=120 chars, written for the OWNER"),
    "title_chars": (240, "rule 7: a queue row's `title` is at most 240 characters"),
    "queue_rows": (20, "rule 7: `queue` holds at most 20 rows"),
    "file_bytes": (12 * 1024, "rule 7: the whole file is at most 12 KB"),
}


def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else "docs/lane-status.json"
    try:
        raw = open(path, "rb").read()
        doc = json.loads(raw)
    except FileNotFoundError:
        print(f"UNREADABLE: {path} does not exist", file=sys.stderr)
        return 2
    except json.JSONDecodeError as exc:
        print(f"UNPARSEABLE: {path}: {exc}", file=sys.stderr)
        return 2

    rows = doc.get("queue") or []
    measured = {
        "focus_chars": (len(doc.get("focus") or ""), "focus"),
        "queue_rows": (len(rows), "queue"),
        "file_bytes": (len(raw), os.path.basename(path)),
    }
    longest = max(((len(r.get("title") or ""), r.get("id", "?")) for r in rows), default=(0, "-"))
    measured["title_chars"] = (longest[0], f"longest title, {longest[1]}")

    breaches = []
    for key, (limit, clause) in BOUNDS.items():
        value, what = measured[key]
        state = "OVER" if value > limit else "ok"
        # A bound reached exactly is reported, because the next write breaches it and the
        # author is the only person standing there when that happens.
        if state == "ok" and value == limit:
            state = "AT LIMIT"
        print(f"{key:<13} {value:>6} / {limit:<6} {state:<9} ({what})")
        if value > limit:
            breaches.append(f"{key}: {value} > {limit} ({what}) - {clause}")

    if breaches:
        print("\nBREACH, do not write this file:", file=sys.stderr)
        for b in breaches:
            print("  " + b, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
