#!/usr/bin/env python3
import sys
P = [
    ("mul_vs_div",      "12", "/",  "2",  "*",  "3"),
    ("div_vs_mul",      "12", "*",  "2",  "/",  "3"),
    ("mod_vs_div",      "12", "#",  "5",  "/",  "2"),
    ("bitand_vs_mul_R", "3",  "*",  "2",  "&",  "5"),
    ("bitor_vs_mul_R",  "3",  "*",  "2",  "|",  "5"),
    ("bitxor_vs_mul_R", "3",  "*",  "2",  "!",  "5"),
    ("bitxor_vs_mul_L", "3",  "!",  "2",  "*",  "2"),
    ("add_vs_bitxor",   "1",  "+",  "3",  "!",  "2"),
    ("eq_vs_bitxor_R",  "1",  "=",  "3",  "!",  "2"),
    ("bitand_vs_shr",   "1",  "&",  "3",  ">>", "1"),
    ("bitor_vs_shl",    "1",  "|",  "3",  "<<", "1"),
    ("bitxor_vs_shl",   "1",  "!",  "3",  "<<", "1"),
    ("bitand_vs_bitor_R","1", "&",  "6",  "|",  "3"),
    ("bitor_vs_bitxor_R","3", "|",  "1",  "!",  "3"),
]
out = ["\tcpu\t68000", "\tpadding\toff", "\torg\t0"]
for (name, a, o1, b, o2, c) in P:
    out.append("; PROBE %s" % name)
    out.append("\tdc.b\t%s%s%s%s%s" % (a, o1, b, o2, c))
    out.append("\tdc.b\t(%s%s%s)%s%s" % (a, o1, b, o2, c))
    out.append("\tdc.b\t%s%s(%s%s%s)" % (a, o1, b, o2, c))
out.append("")
open(sys.argv[1], "w").write("\n".join(out) + "\n")
