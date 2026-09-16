#!/usr/bin/env python3
# VENDORED FROM EMPYREAN, NOT WRITTEN HERE.
# Source: empyrean scripts/ledger_append.py. Empyrean's copy governs on any
# disagreement, and a divergence between the two is a finding rather than a
# value to reconcile.
#
#   introduced upstream at:  ba1de1f
#   vendored here from:      d631d18 (verified an ancestor of their origin/main)
#   content identity:        CRC32 9c6b6df6, 12253 bytes
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
# THE PIN HAS FIRED FOUR TIMES ON LEGITIMATE CHANGES, which is the only exercise
# that tests it without also reporting a fault: a docstring correction, the
# empty-time guard, the null and absent fixes, and the last-record fix below.
# Each reconciled after checking, not after being told.
#
# WHAT THIS REFUSES, RE-PROVED AT THESE BYTES rather than carried over, since
# red-first custody attaches to the BYTES that were proved and not to a file
# name. Every refusal md5-unchanged, every exit code read UNPIPED (zsh has no
# PIPESTATUS, so a piped read reports the downstream command's status and turns
# a refusal into a green that means nothing):
#   a file whose last line does not parse  -> exit 1, nothing written
#   a missing trailing newline             -> repaired, BOTH prior records intact
#   a time field empty, blank or null      -> exit 1
#   a time field absent                    -> exit 1 where the ledger evidences one
#
# THE REQUIRED-KEY RULE IS MONOTONE (`any`), AND THE THIRD MEMBER OF THAT FAMILY
# IS WHY THIS PARAGRAPH EXISTS. Evidence that a ledger keeps a time field cannot
# be erased by adding a record that lacks one. Two earlier rules could be:
#   ALL   this lane's first gate draft, reader-side. The candidate broke its own
#         file's unanimity and dropped the file out of the judged set; 469
#         records judged fell to 118 and the row still said ok.
#   LAST  this tool at 658502e, which read
#         `expected = [k for k in TIME_FIELDS if k in last_record]`. One bad
#         record written through the one-line bypass became the reference and
#         disabled the check for everything after it.
# ⚠ THIS LANE TOLD THE AUTHOR THEIR TOOL WAS UNAFFECTED AND NOT TO CHANGE IT,
# reasoning that a producer-side check evaluates the file before the candidate
# joins it, which is true and was not the mechanism. The source was on disk here
# and had been run three times; the function was never read. They checked anyway
# and found it. The lesson is not the wrong guess, it is that a DO-NOT-CHANGE-IT
# is a stronger act than a claim and this one was issued unmeasured.
#
# THE GUARD'S ACTIVATION IS STILL DATA-DERIVED. The tool has no schema for any
# ledger and will not invent one, so absent is refused only where the ledger's
# own records evidence a time field. MEASURED AT ADOPTION, and this is the part
# that goes stale, so re-measure rather than trusting it: a MIXED ledger now
# REFUSES (one record carrying a time field is enough), and only a ledger where
# NO record carries one is unconstrained. docs/lane-log.jsonl and
# docs/decisions.jsonl are both fully populated, so the guard is active on both.
#
# AND THE GUARANTEE IS NARROWER THAN "THE LEDGER IS SAFE": every refusal here is
# skipped by `open(p, "a").write(...)`, which is one line and is what a hurried
# seat reaches for. What this offers is that a record written THROUGH IT cannot
# corrupt the previous one. The reader-side row in scripts/ledger_gate.py is what
# judges entries however they arrived, and it is the half that cannot be walked
# around.
#
# ADOPTED as the PREVENTION half of this lane's open row
# LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE, whose closing condition is that
# hand-appending stops being something a seat can casually do, not that this file
# exists.
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

READ THIS BEFORE TRUSTING THIS FILE (aurora, 2026-09-16, and it is about this tool
rather than about a caller): A PRODUCER-SIDE GUARD ON A PATH ANYONE CAN WALK AROUND
IS A HABIT, NOT A GATE. Every refusal below is skipped entirely by
`open(p, "a").write(...)` in a heredoc, which is how both aurora and this lane
actually wrote entries earlier the same night -- aurora's got away with it because
it happened to be well formed, which is luck reported as a pass. So the guarantee
this file can honestly offer is: A RECORD WRITTEN THROUGH THIS TOOL CANNOT CORRUPT
THE PREVIOUS ONE. It is NOT: the ledger is safe. Only a reader-side check on the
committed file judges entries however they got there, and that is where the real
gate lives -- aurora landed one (lane-records-are-strict-jsonl) after finding its
own presence check could not tell `at: ""` from a good timestamp, since
`typeof '' === 'string'`.

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

    WIDENED TWICE ON SIGIL'S MEASUREMENT, 2026-09-16, and both were holes in the scope
    this docstring already CLAIMED rather than scope it declined.

    (1) `at: null` PASSED AND WROTE. The first guard read
    `isinstance(record[key], str) and not record[key].strip()`, so a JSON null -- present,
    carrying no time, not a str -- went straight through. The docstring said "present and
    empty", and null is present and empty of content, so the stated scope was simply not
    met. AND IT IS THE SAME DEFECT ONE LANGUAGE OVER: in shell a dropped clock arrives as
    "", in Python it arrives as None, from os.environ.get("NOW") with NOW unset -- the
    forgiving spelling a caller reaches for over the raising one. A guard built from the
    shape of the instance that produced it is a sampled domain, which is the failure this
    whole tool exists to stop, arriving on the FIX instead of on the claim.

    (2) AN ABSENT TIME FIELD WAS A HOLE IN BOTH HALVES AT ONCE. It passed here, and sigil
    measured that ledger_gate.py also exits 0 on a record with no `at` -- so detection and
    prevention had the SAME gap, which is the one thing a pair is supposed to make
    impossible, and neither side could have found it by looking at its own half. It also
    contradicted aurora's principle, which this lane had relayed two hours earlier: a
    missing field must be a NAMED failure, never an implicit pass, because "I could not
    look" and "I looked and it is empty" must not return the same answer. They did.

    THE BOUNDARY MOVED, AND HOW IT MOVED MATTERS. This tool appends to ANY .jsonl ledger
    and has no schema for any of them -- so it does not invent one. THE LEDGER ITSELF IS
    THE EVIDENCE: if the records already in the file carry a time field, a new record
    without one is refused; if they do not, it is allowed. The convention is read from the
    population rather than hardcoded, so lane-log.jsonl and decisions.jsonl are covered
    without breaking a ledger that legitimately has no time field. An empty or new file
    has no population and constrains nothing.

    Still deliberately NOT validated: the FORMAT, or whether the time is plausible. A
    wrong-but-present timestamp is a different failure with different evidence, and
    guessing at formats here would refuse records this tool has no standing to judge.
    """
    for key in TIME_FIELDS:
        if key not in record:
            continue
        value = record[key]
        if value is None:
            sys.exit(f"REFUSING: record has {key}=null. A time field was read and lost in "
                     "transit -- in Python most often os.environ.get(\"NOW\") with NOW unset, "
                     "which returns None instead of raising. Nothing was written.")
        if isinstance(value, str) and not value.strip():
            sys.exit(f"REFUSING: record has {key}=\"\" (empty). A time field was read "
                     "and lost in transit, most often by `VAR=$(date -u ...) python3 ... \"$VAR\"`, "
                     "where the assignment is not in scope for that same line. Set it on its "
                     "own line first, then pass it. Nothing was written.")


def check_conforms_to_ledger(record: dict, existing: list) -> None:
    """Refuse a record missing a time field that this ledger's own records carry.

    See check_timestamps' docstring, point (2): an absent field was a hole in BOTH this
    tool and sigil's gate simultaneously. The population is the authority -- no schema is
    invented here.

    ANY, NOT THE LAST RECORD, AND NOT ALL. Corrected 2026-09-16, measured against this
    tool after sigil reported the mirror-image defect in its own gate and said this tool
    was unaffected. It was affected, by a different mechanism, and only checking found it:

      * The first version sampled THE LAST RECORD ONLY. One record with no time field --
        landed through the bypass aurora proved anyone can walk (`open(p,"a").write(...)`,
        which skips every check in this file) -- became the sample, and the guard then let
        every subsequent record through. A CHECK WHOSE REFERENCE IS THE MOST RECENT WRITE
        IS DISABLED BY ONE BAD WRITE, silently, and the file looks normal afterwards.
      * ALL (unanimity) is what sigil tried and is SELF-DEFEATING READER-SIDE, for the
        symmetric reason: a gate reads the file AFTER the candidate is in it, so the one
        record with no time field breaks its own file's unanimity, drops the file out of
        the judged set, and the check goes green on the exact shape it exists to catch.
        Its judged population fell 469 -> 118 and the red would not fire.

    So: ANY record establishing the field is enough to require it. That is monotone --
    evidence of the convention cannot be erased by adding a bad record, which is the
    property both broken versions lacked. The same rule is sound on one side of the write
    and self-defeating on the other, decided entirely by WHICH SIDE IT RUNS ON, which is
    why it must not be copied across that seam by anyone who saw it work in one place.
    """
    fields = set()
    for rec in existing:
        if isinstance(rec, dict):
            fields.update(k for k in TIME_FIELDS if k in rec)
    if not fields:
        return
    missing = [k for k in TIME_FIELDS if k in fields and k not in record]
    if missing:
        sys.exit(f"REFUSING: record omits {', '.join(missing)}, which records already in "
                 "this ledger establish (ANY record is enough -- deliberately not 'every', "
                 "so one bad record cannot disable the check). An absent time field must be "
                 "a NAMED failure, not an implicit pass: 'I could not look' and 'I looked "
                 "and it is empty' must not give the same answer. Nothing was written.")


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
            existing = []
            for ln in tail:
                try:
                    existing.append(json.loads(ln))
                except Exception:
                    continue  # a damaged earlier line is not this check's subject
            check_conforms_to_ledger(record, existing)
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
