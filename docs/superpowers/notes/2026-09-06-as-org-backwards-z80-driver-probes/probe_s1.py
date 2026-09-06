#!/usr/bin/env python3
"""Instrument a PREPARED s1disasm tree so both assemblers report `$` across the
Z80 driver's `!org 0`.

usage: probe_s1.py <corpus-dir>

The probes are INSERTED, never substituted. Nothing the original file supplies
is removed, so the defect under investigation is still present in the probed
tree: a reduction that replaced the org, or supplied a value for `$`, would test
a program in which the fault cannot occur.

Sigil's `message` directive evaluates its string and discards it, so `warning` is
the only author-diagnostic that reaches both assemblers' streams.

The three probes that sit under `CPU 68000` use `\\{*}` rather than `\\{$}`. Both
assemblers refuse `$` as an interpolated PC there, asl with `#1020 invalid symbol
name` and sigil by leaving the sequence verbatim, so `\\{$}` at those sites would
measure that gap instead of this one. `*` is the PC on both CPUs in both tools.

Run against a COPY of a prepared tree, never the baseline copy: the point of
keeping two is that the unprobed one still reproduces the published count.
"""
import io
import os
import sys


def read(p):
    with io.open(p, "r", encoding="latin-1", newline="") as f:
        return f.read().split("\n")


def write(p, lines):
    with io.open(p, "w", encoding="latin-1", newline="") as f:
        f.write("\n".join(lines))


def main():
    if len(sys.argv) != 2:
        sys.stderr.write("usage: probe_s1.py <corpus-dir>\n")
        return 2
    corpus = sys.argv[1]
    z80 = os.path.join(corpus, "sound", "z80.asm")
    snd = os.path.join(corpus, "s1.sounddriver.asm")
    for p in (z80, snd):
        if not os.path.isfile(p):
            sys.stderr.write("FATAL: no %s\n" % p)
            return 2

    z = read(z80)
    # Anchors, so a corpus revision that moved these lines refuses rather than
    # probing whatever now sits at the line number.
    assert z[7].strip() == "save", repr(z[7])
    assert z[8].strip().startswith("!org"), repr(z[8])
    assert z[9].strip() == "CPU Z80", repr(z[9])
    assert z[228].strip().startswith("fatal"), repr(z[228])

    out = []
    for i, line in enumerate(z):
        n = i + 1
        if n == 8:
            out.append('\twarning "PROBE-1 before-save $=\\{*}h"')
            out.append(line)
        elif n == 9:
            out.append(line)
            out.append('\twarning "PROBE-2 after-org $=\\{*}h"')
        elif n == 226:
            out.append('\twarning "PROBE-3 driver-end $=\\{$}h zDAC_Kick=\\{zDAC_Kick}h '
                       'zDAC_Timpani=\\{zDAC_Timpani}h DACDriver=\\{DACDriver}h"')
            out.append(line)
        else:
            out.append(line)
    write(z80, out)

    s = read(snd)
    assert s[2631].startswith("DACDriver:"), repr(s[2631])
    s.insert(2631, '\twarning "PROBE-0 before-include $=\\{*}h"')
    write(snd, s)

    print("probes applied to %s" % corpus)
    print("  sound/z80.asm         %d lines (was %d)" % (len(out), len(z)))
    print("  s1.sounddriver.asm    %d lines" % len(s))
    return 0


if __name__ == "__main__":
    sys.exit(main())
