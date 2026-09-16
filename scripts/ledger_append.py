#!/usr/bin/env python3
# VENDORED FROM EMPYREAN, NOT WRITTEN HERE.
# Source: empyrean scripts/ledger_append.py. Empyrean's copy governs on any
# disagreement, and a divergence between the two is a finding rather than a
# value to reconcile.
#
#   introduced upstream at:  ba1de1f
#   vendored here from:      d6ce7a2 (verified an ancestor of their origin/main)
#   content identity:        CRC32 79f5187b, 5936 bytes
#
# CITED BY CONTENT, NOT ONLY BY SHA. An earlier header named 51a48d4 alone, the
# revision the hub announced, which is a true statement about a revision where
# the content existed and is NOT the commit that introduced the file:
# `git log --all -- scripts/ledger_append.py` in empyrean returns ba1de1f and
# never returns 51a48d4. The DIGEST is the part that cannot rot. A SHA stops
# resolving across a rewrite and says nothing about whether the bytes moved,
# while CRC32 plus size answers the only question a successor has, which is
# whether this copy still is what was verified. Recompute over empyrean's file
# with zlib.crc32 (IEEE, the campaign provenance standard); a decimal figure came
# from a foreign tool and does not compare.
#
# THIS PIN HAS FIRED TWICE ON LEGITIMATE CHANGES, which is the only exercise that
# tests the mechanism without also reporting a fault: a docstring correction
# (37e98d6b -> c36dd014) and then a real behaviour change, the empty-time guard
# (-> 79f5187b). Both reconciled after checking, not after being told.
#
# RE-VERIFIED AT THIS REVISION rather than carried over, since red-first custody
# attaches to the BYTES that were proved and not to a file name. Digest
# recomputed here. Arm 1, no trailing newline: repaired, BOTH prior records
# intact. Arm 2, already-damaged tail: exit 1, md5 identical before and after.
# Exit codes read UNPIPED: zsh has no PIPESTATUS, so a piped read reports the
# downstream command's status and turns a refusal into a green that means
# nothing.
#
# TWO THINGS THE TIME GUARD DOES NOT CATCH, MEASURED HERE AND REPORTED UPSTREAM.
# Recorded so this copy does not overclaim. This lane does NOT fork the behaviour
# to close them locally; empyrean owns the fix.
#   at: null    PASSES AND WRITES. The check is `key in record and
#               isinstance(record[key], str) and not record[key].strip()`, so a
#               JSON null is present, carries no time, and is not a str. That is
#               INSIDE the guard's own stated domain (present but empty), and it
#               is the author's own transit failure one language over: in shell a
#               dropped clock arrives as "", while in Python
#               `os.environ.get("NOW")` with NOW unset arrives as None, which is
#               the forgiving spelling a caller reaches for.
#   at absent   PASSES AND WRITES, by the author's stated boundary. Measured
#               here: scripts/ledger_gate.py does not catch it either. So it is a
#               hole in BOTH halves at once, which is the one thing a detection
#               plus prevention pair is supposed to make impossible.
#
# ADOPTED as the PREVENTION half of this lane's open row
# LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE. scripts/ledger_gate.py is the
# DETECTION half and stays: this refuses to corrupt, that refuses to land if
# something else did.
#
# THE RESIDUAL: by aeon's removes-the-need-to-remember test this is first-best at
# the WRITE and still carries an obligation one layer out, because somebody has
# to remember to call it instead of hand-appending. Bounded, not closed. The row
# closes when hand-appending is no longer something a seat can casually do, not
# when this file merely exists.
"""Append one record to a .jsonl ledger without being able to corrupt the previous one.

THE HAZARD (sigil, 2026-09-16, row LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE):
every lane appends to docs/lane-log.jsonl and docs/decisions.jsonl with
`open(p, "a").write(json.dumps(rec) + "\\n")`. That is correct ONLY if the file
already ends in a newline. When it does not, the new record is welded onto the
previous line and BOTH become unparseable -- and it destroys the PREVIOUS
session's record, in files LANE_LOG.md and DECISIONS.md both call append-only and
never-backfilled, which is to say unreconstructable.

Sigil built the DETECTION half (a gate that refuses a landing when a ledger line
does not parse or a file lacks its trailing newline). This is the PREVENTION half.

CORRECTED 2026-09-16 ON SIGIL'S PUSHBACK, and the correction is the point. This
docstring said "a gate needs someone to run it, a write that cannot corrupt needs
nobody to remember anything." That is true of the WRITE and false of the REACH:
somebody still has to choose this tool over a hand-append, which is aeon's
removes-the-need-to-remember test applied one layer out. So the pair -- this tool
plus sigil's ledger_gate.py -- is detection PLUS prevention, genuinely better than
either, and it is NOT closure. The residual is bounded, not deleted: a hand-append
that corrupts is caught by the gate at the next landing before a push, and a welded
line is recoverable by raw_decode-splitting; what is not recovered is the interval
in between. THE CLOSING CONDITION IS SIGIL'S AND IT GOVERNS: this closes when
hand-appending is no longer something a seat can casually do, not when the tool
merely exists.

ADOPTING THIS IS NOT A DOCS-ONLY COMMIT (sigil, on its first live adoption). It
puts a new file in scripts/, and a docs-only exemption measured against one
population says nothing about that one: sigil's clause rested on no Rust target
reading docs/, and its repo turned out to have a tree-walking gate policing what
scripts may contain. Run your own tree-walking gates rather than reasoning that a
small utility cannot matter.

    python3 scripts/ledger_append.py docs/lane-log.jsonl '<json object>'
    python3 scripts/ledger_append.py docs/lane-log.jsonl --file rec.json

Refuses (exit 1, nothing written) when the record is not a JSON object, or when
the file's last line does not parse -- a ledger that is ALREADY damaged must not
be appended to, because the append buries the damage under a good record.
"""
import json
import os
import sys


TIME_FIELDS = ("at", "updatedAt", "since", "ts")


def check_timestamps(record: dict) -> None:
    """Refuse a record whose timestamp field is present but empty or blank.

    ADDED 2026-09-16, against this lane's own conduct, one command after the tool
    shipped. The suite already forbids typing a time from a session's own sense of
    it: a model has no clock and cannot feel time drifting, so every time field is
    taken from `date -u`. That rule was OBEYED here and the defect happened anyway.
    The shell ate it:

        NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ) python3 - "$NOW"   # "$NOW" is EMPTY

    An assignment used as a command prefix is not in scope for the expansion on that
    same line, so the clock was read correctly and then dropped in transit, and the
    record was written with at="". Nothing in the output said so.

    This is the harder half of the original class. The standing rule guards against a
    session INVENTING a number; it cannot see a correct number that never arrived. So
    the check is here rather than in anyone's memory: an empty timestamp is a stopped
    command, never a record that looks complete and is not. Set the variable on its own
    line, or pass $(date -u +%Y-%m-%dT%H:%M:%SZ) directly as the argument.

    Deliberately NOT validated: the format, or whether the time is plausible. This
    refuses a field that is present and empty -- the failure the shell actually causes.
    A record with no time field at all is a different shape and not this tool's call.
    """
    for key in TIME_FIELDS:
        if key in record and isinstance(record[key], str) and not record[key].strip():
            sys.exit(f"REFUSING: record has {key}=\"\" (empty). A time field was read "
                     "and lost in transit, most often by `VAR=$(date -u ...) python3 ... \"$VAR\"`, "
                     "where the assignment is not in scope for that same line. Set it on its "
                     "own line first, then pass it. Nothing was written.")


def append(path: str, record: dict) -> None:
    if not isinstance(record, dict):
        sys.exit("record must be a JSON object")
    check_timestamps(record)
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
