#!/usr/bin/env python3
"""Generate ladder.asm: for every probe emit the BARE expression plus both
fully parenthesised candidates, so the listing itself shows whether the probe
can distinguish the two parses. A probe whose two candidates print the same
byte is confounded and is reported as such rather than read as evidence."""
import sys

# (id, left_operand, op1, mid_operand, op2, right_operand)
# bare  = "a op1 b op2 c"
# LEFT  = "(a op1 b) op2 c"   -> op1 binds tighter
# RIGHT = "a op1 (b op2 c)"   -> op2 binds tighter
P = [
    # --- && against every other tier -------------------------------------
    ("andand_vs_bitand_L",  "1", "&&", "12", "&",  "3"),
    ("andand_vs_bitand_R",  "6", "&",  "4",  "&&", "2"),
    ("andand_vs_bitor_L",   "0", "&&", "8",  "|",  "4"),
    ("andand_vs_bitor_R",   "4", "|",  "0",  "&&", "8"),
    ("andand_vs_bitxor_L",  "0", "&&", "8",  "!",  "4"),
    ("andand_vs_bitxor_R",  "4", "!",  "0",  "&&", "8"),
    ("andand_vs_shl_L",     "1", "&&", "1",  "<<", "3"),
    ("andand_vs_shl_R",     "1", "<<", "3",  "&&", "0"),
    ("andand_vs_add_L",     "1", "&&", "2",  "+",  "3"),
    ("andand_vs_add_R",     "3", "+",  "0",  "&&", "0"),
    ("andand_vs_mul_L",     "2", "&&", "3",  "*",  "4"),
    ("andand_vs_mul_R",     "3", "*",  "1",  "&&", "0"),
    ("andand_vs_eq_L",      "1", "&&", "2",  "=",  "2"),
    ("andand_vs_eq_R",      "2", "=",  "2",  "&&", "1"),
    ("andand_vs_ne_R",      "7", "<>", "3",  "&&", "0"),
    ("andand_vs_lt_R",      "1", "<",  "5",  "&&", "0"),
    # --- || against every other tier -------------------------------------
    ("oror_vs_andand_L",    "1", "||", "0",  "&&", "0"),
    ("oror_vs_andand_R",    "0", "&&", "0",  "||", "1"),
    ("oror_vs_bitand_L",    "0", "||", "12", "&",  "3"),
    ("oror_vs_bitand_R",    "6", "&",  "4",  "||", "0"),
    ("oror_vs_bitor_L",     "0", "||", "8",  "|",  "4"),
    ("oror_vs_bitor_R",     "8", "|",  "0",  "||", "4"),
    ("oror_vs_shl_L",       "1", "||", "1",  "<<", "3"),
    ("oror_vs_add_L",       "1", "||", "2",  "+",  "3"),
    ("oror_vs_eq_L",        "1", "||", "2",  "=",  "2"),
    ("oror_vs_eq_R",        "2", "=",  "2",  "||", "0"),
    # --- the non-logical region, to prove the rest of the ladder ---------
    ("bitand_vs_bitor",     "1", "|",  "2",  "&",  "2"),
    ("bitxor_vs_bitand",    "1", "!",  "3",  "&",  "2"),
    ("bitxor_vs_bitor",     "3", "!",  "1",  "|",  "2"),
    ("bitand_vs_shl",       "1", "&",  "3",  "<<", "1"),
    ("shl_vs_add",          "1", "+",  "1",  "<<", "3"),
    ("shl_vs_mul",          "2", "<<", "1",  "*",  "3"),
    ("eq_vs_add",           "4", "=",  "1",  "+",  "1"),
    ("eq_vs_bitand",        "6", "&",  "2",  "=",  "2"),
    ("eq_vs_shl",           "6", "<<", "1",  "=",  "12"),
    ("lt_vs_eq",            "1", "<",  "2",  "=",  "1"),
    ("mul_vs_add",          "6", "+",  "2",  "*",  "3"),
]

out = ["\tcpu\t68000", "\tpadding\toff", "\torg\t0"]
for (name, a, o1, b, o2, c) in P:
    out.append("; PROBE %s" % name)
    out.append("\tdc.b\t%s%s%s%s%s" % (a, o1, b, o2, c))
    out.append("\tdc.b\t(%s%s%s)%s%s" % (a, o1, b, o2, c))
    out.append("\tdc.b\t%s%s(%s%s%s)" % (a, o1, b, o2, c))
out.append("")
open(sys.argv[1], "w").write("\n".join(out) + "\n")
print("probes:", len(P), "lines:", len(out))
