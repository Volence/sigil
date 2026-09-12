#!/usr/bin/env python3
"""Red-first mutation of the SUBJECT: make far_scratch_slot accept every slot, which is the
pre-fix cursor (0x70_0000 + k*0x10_0000, aliasing at k = 9). Touches native.rs only, and
only the admissibility test inside far_scratch_slot; the tests are left alone."""
import sys

p = sys.argv[1]
src = open(p).read()
old = "        let one_wrap = first >> 24 == last >> 24;\n        if one_wrap\n"
new = "        let one_wrap = first >> 24 == last >> 24;\n        if true || one_wrap\n"
assert src.count(old) == 1, "mutation anchor not found exactly once"
open(p, "w").write(src.replace(old, new))
print("mutated", p)
