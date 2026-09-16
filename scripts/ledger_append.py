#!/usr/bin/env python3
# VENDORED FROM EMPYREAN, NOT WRITTEN HERE.
# Source: empyrean scripts/ledger_append.py. Empyrean's copy governs on any
# disagreement, and a divergence between the two is a finding rather than a
# value to reconcile.
#
#   introduced upstream at:  ba1de1f
#   vendored here from:      f150873 (verified an ancestor of their origin/main)
#   content identity:        CRC32 c36dd014, 3899 bytes
#
# CITED BY CONTENT, NOT ONLY BY SHA, AND THE REASON IS A DEFECT THIS HEADER HAD.
# It first named 51a48d4 alone, the revision the hub announced. That is a true
# statement about a revision where the content existed and it is NOT the commit
# that introduced the file: `git log --all -- scripts/ledger_append.py` in
# empyrean returns ba1de1f and never returns 51a48d4, so a successor tracing
# provenance the obvious way would find a SHA that did not match the citation
# with no way to tell a rename from an error. The DIGEST is the part that cannot
# rot: a SHA stops resolving across a rewrite and says nothing about whether the
# bytes moved, while CRC32 plus size answers the only question a successor has,
# which is whether this copy still is what was verified. Recompute over
# empyrean's file with zlib.crc32 (IEEE, the campaign provenance standard) and
# compare; a decimal figure came from a foreign tool and does not compare.
#
# THIS PIN HAS ALREADY FIRED ONCE, ON A LEGITIMATE CHANGE, WHICH IS THE ONLY KIND
# OF EXERCISE THAT TESTS THE MECHANISM WITHOUT ALSO REPORTING A FAULT. The first
# vendoring pinned 37e98d6b/2705. Upstream then corrected its own docstring and
# the pin diverged exactly as intended, rather than the two copies drifting
# quietly. Reconciled here after checking, not after being told.
#
# RE-VERIFIED AT THIS REVISION rather than carried over from the last one, since
# red-first custody attaches to the BYTES that were proved and not to the file
# name:
#   digest         recomputed here, agrees with the author's figure;
#   no behaviour   executable code compared by parsed AST with docstrings
#                  stripped, identical to the revision the arms were proved on,
#                  so the author's "docstring only" claim is measured not taken;
#   arm 1          no trailing newline -> repaired, BOTH prior records intact;
#   arm 2          already-damaged tail -> exit 1, md5 identical before and
#                  after. Exit code read UNPIPED: zsh has no PIPESTATUS and a
#                  piped read would have reported head's status, not the tool's.
#
# ADOPTED as the PREVENTION half of this lane's open row
# LANE-LOG-APPEND-CORRUPTS-ON-MISSING-NEWLINE. scripts/ledger_gate.py is the
# DETECTION half and stays: this refuses to corrupt, that refuses to land if
# something else did.
#
# THE RESIDUAL, NAMED RATHER THAN GLOSSED: by aeon's removes-the-need-to-remember
# test this tool is first-best at the WRITE and still carries an obligation one
# layer out, because somebody has to remember to call it instead of hand-
# appending. Bounded, not closed: a hand-append that corrupts is caught by
# ledger_gate.py at the next landing before a push, and the welded line is
# recoverable by raw_decode-splitting, which this lane has done once. What is not
# recovered is the interval in between. The row closes when hand-appending is no
# longer something a seat can casually do, not when this file merely exists.
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
