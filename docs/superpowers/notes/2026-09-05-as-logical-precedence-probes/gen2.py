#!/usr/bin/env python3
import sys
P = [
    # shifts against the arithmetic tier (round 1 said `<<` beats `+`)
    ("shl_vs_div",        "12", "/",  "2",  "<<", "1"),
    ("div_vs_shl",        "12", "<<", "1",  "/",  "3"),
    ("shr_vs_add",        "1",  "+",  "8",  ">>", "2"),
    ("sub_vs_shl",        "8",  "-",  "1",  "<<", "2"),
    ("shl_vs_shr",        "8",  ">>", "1",  "<<", "2"),
    ("mod_vs_shl",        "12", "#",  "5",  "<<", "1"),
    ("mod_vs_mul",        "7",  "#",  "5",  "*",  "2"),
    # bitwise tier against the arithmetic tier
    ("bitand_vs_add",     "1",  "&",  "3",  "+",  "1"),
    ("add_vs_bitand",     "1",  "+",  "3",  "&",  "2"),
    ("bitor_vs_add",      "3",  "|",  "2",  "+",  "2"),
    ("bitxor_vs_add",     "3",  "!",  "2",  "+",  "2"),
    ("bitand_vs_mul",     "3",  "&",  "2",  "*",  "3"),
    ("add_vs_bitor",      "1",  "+",  "3",  "|",  "4"),
    # bitwise xor placement against comparisons and against ||
    ("eq_vs_bitxor",      "3",  "!",  "1",  "=",  "2"),
    ("oror_vs_bitxor_L",  "0",  "||", "8",  "!",  "4"),
    # the remaining comparison spellings against &&
    ("eq_vs_lt_R",        "2",  "=",  "1",  "<",  "2"),
    ("andand_vs_gt_R",    "5",  ">",  "1",  "&&", "0"),
    ("andand_vs_le_R",    "0",  "<=", "1",  "&&", "0"),
    ("andand_vs_ge_R",    "0",  ">=", "1",  "&&", "0"),
    ("andand_vs_lt_R2",   "0",  "<",  "1",  "&&", "0"),
    # && / || associativity, and && against the remaining bitwise spellings
    ("andand_assoc",      "1",  "&&", "0",  "&&", "1"),
    ("oror_assoc",        "0",  "||", "1",  "||", "0"),
]
out = ["\tcpu\t68000", "\tpadding\toff", "\torg\t0"]
for (name, a, o1, b, o2, c) in P:
    out.append("; PROBE %s" % name)
    out.append("\tdc.b\t%s%s%s%s%s" % (a, o1, b, o2, c))
    out.append("\tdc.b\t(%s%s%s)%s%s" % (a, o1, b, o2, c))
    out.append("\tdc.b\t%s%s(%s%s%s)" % (a, o1, b, o2, c))
out.append("")
open(sys.argv[1], "w").write("\n".join(out) + "\n")
print("probes:", len(P))
