#!/usr/bin/env python3
"""Pin asl's nameless counter arithmetic by reading its symbol table names."""
import os, sys
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
H = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
X = {
 "x1_file_plus_pp_plus": "+\tdc.w\t$2222\n++\tdc.w\t$3333\n+\tdc.w\t$5555\n",
 "x2_body_pp_then_file": "mac\tmacro\n++\tdc.w\t$2222\n\tendm\n\tmac\n+\tdc.w\t$3333\n+\tdc.w\t$5555\n",
 "x3_body_p_then_file": "mac\tmacro\n+\tdc.w\t$2222\n\tendm\n\tmac\n+\tdc.w\t$3333\n+\tdc.w\t$5555\n",
 "x4_body_pp_twice_then_file": "mac\tmacro\n++\tdc.w\t$2222\n\tendm\n\tmac\n\tmac\n+\tdc.w\t$3333\n",
 "x5_file_pp_then_plus": "++\tdc.w\t$2222\n+\tdc.w\t$3333\n",
 "x6_body_minus_then_file": "mac\tmacro\n-\tdc.w\t$2222\n\tendm\n\tmac\n-\tdc.w\t$3333\n",
 "x7_body_slash_then_file": "mac\tmacro\n/\tdc.w\t$2222\n\tendm\n\tmac\n+\tdc.w\t$3333\n-\tdc.w\t$5555\n",
 "x8_body_ppp_then_file": "mac\tmacro\n+++\tdc.w\t$2222\n\tendm\n\tmac\n+\tdc.w\t$3333\n",
 "x9_rept_pp_then_file": "\trept\t1\n++\tdc.w\t$2222\n\tendm\n+\tdc.w\t$3333\n",
}
for k, v in X.items():
    open(os.path.join(OUT, k + ".asm"), "w").write(H + v + "\tdc.w\t$4444\n")
print(len(X))
