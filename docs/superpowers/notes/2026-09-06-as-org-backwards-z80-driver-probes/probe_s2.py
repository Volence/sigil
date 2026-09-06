#!/usr/bin/env python3
"""Instrument a PREPARED s2disasm tree either side of `s2.sounddriver.asm`'s
`!org 0`, the site where sigil takes the OTHER branch of `directive_org`.

usage: probe_s2.py <corpus-dir>

The s1disasm site reaches `org` with a section open and is refused out loud. This
one reaches it with no section open, is accepted, and moves the location counter
to 0 with no diagnostic at all. The two arms are the same directive and the same
target, so this probe exists to keep the finding from being read as a property of
`org 0` rather than of the tree's arrangement at the moment it runs.

`\\{*}` for the same reason as probe_s1.py: this site sits under `CPU 68000`.
"""
import io
import os
import sys


def main():
    if len(sys.argv) != 2:
        sys.stderr.write("usage: probe_s2.py <corpus-dir>\n")
        return 2
    p = os.path.join(sys.argv[1], "s2.sounddriver.asm")
    if not os.path.isfile(p):
        sys.stderr.write("FATAL: no %s\n" % p)
        return 2
    with io.open(p, "r", encoding="latin-1", newline="") as f:
        lines = f.read().split("\n")
    assert lines[247].strip().startswith("!org 0"), repr(lines[247])
    lines.insert(248, '\twarning "PROBE-S2 after-org $=\\{*}h"')
    lines.insert(247, '\twarning "PROBE-S2 before-org $=\\{*}h"')
    with io.open(p, "w", encoding="latin-1", newline="") as f:
        f.write("\n".join(lines))
    print("probes applied to %s" % p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
