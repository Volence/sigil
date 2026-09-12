#!/usr/bin/env python3
"""List every PLAIN label (a name not starting with `.`) written inside a
macro body, from AS source on stdin.

A body runs from a `NAME macro` / `NAME: macro` head to its `endm`, with
`rept`/`irp`/`irpc`/`while` blocks nested inside it counted so their `endm`
does not end the macro. A plain label is either `NAME:` at any indentation or
a bare `NAME` at column 0 (AS's column rule). Prints one row per hit and a
final count, so an empty result is a printed zero over a printed number of
bodies and body lines, not silence.
"""
import re, sys

KW_BLOCK = {"rept", "irp", "irpc", "while"}
HEAD = re.compile(r"^\s*([A-Za-z_][\w.]*)\s*:?\s+macro\b", re.I)
COLON = re.compile(r"^\s*([A-Za-z_][\w]*)\s*:(?!=)")
BARE0 = re.compile(r"^([A-Za-z_][\w]*)\b")
NOT_LABELS = {"if", "ifdef", "ifndef", "else", "elseif", "endif", "switch", "case",
              "elsecase", "endcase", "endm", "rept", "irp", "irpc", "while", "shift",
              "exitm", "set", "equ"}

depth = 0          # 0 = outside any macro; 1 = in a body; >1 = nested block
bodies = body_lines = hits = 0
cur = None
for n, raw in enumerate(sys.stdin, 1):
    line = raw.split(";", 1)[0].rstrip("\n")
    words = line.split()
    w0 = words[0].lower() if words else ""
    if depth == 0:
        m = HEAD.match(line)
        if m:
            depth, cur, bodies = 1, m.group(1), bodies + 1
        continue
    body_lines += 1
    if HEAD.match(line) or w0 in KW_BLOCK:
        depth += 1
        continue
    if w0 == "endm":
        depth -= 1
        if depth == 0:
            cur = None
        continue
    m = COLON.match(line)
    name = m.group(1) if m else None
    if name is None and line and not line[0].isspace():
        b = BARE0.match(line)
        if b and b.group(1).lower() not in NOT_LABELS:
            name = b.group(1)
    if name and name.lower() not in NOT_LABELS:
        hits += 1
        print(f"HIT line {n} in macro {cur}: {raw.rstrip()}")
print(f"BODYSCAN bodies={bodies} body_lines={body_lines} plain_labels={hits}")
