#!/usr/bin/env python3
"""Round 3: a user FUNCTION or symbol spelled like a builtin, and the macro
parameter default forms around m5d_default_2p. One construct per file."""
import os

D = "/home/volence/sonic_hacks/.scratch/as-missing-builtins/probes3"
os.makedirs(D, exist_ok=True)
for f in os.listdir(D):
    os.remove(os.path.join(D, f))

M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
P = {}
for f in ["sgn", "bitcnt", "firstbit", "bitpos", "toupper", "tolower", "lastbit", "abs", "int"]:
    P[f"u_fn_{f}"] = f"{f} function x,x+100\n\tdc.b {f}(1)\n"
    P[f"u_fnU_{f}"] = f"{f.upper()} function x,x+100\n\tdc.b {f}(1)\n"
    P[f"u_sym_{f}"] = f"{f} equ 7\n\tdc.b {f}\n"
    P[f"u_symcall_{f}"] = f"{f} equ 7\n\tdc.b {f}(1)\n"
    P[f"u_label_{f}"] = f"{f}:\tdc.b {f}(8)\n"
# macro parameter defaults
P["m_default_2p"] = "m\tmacro pa=2p.bin\n\tdc.b \"pa\"\n\tendm\n\tm\n"
P["m_default_2p_given"] = "m\tmacro pa=2p.bin\n\tdc.b \"pa\"\n\tendm\n\tm xy\n"
P["m_default_2p_two"] = "m\tmacro pa=2p.bin,qq\n\tdc.b \"pa\",qq\n\tendm\n\tm ,5\n"
P["m_default_2p_second"] = "m\tmacro qq,pa=2p.bin\n\tdc.b qq,\"pa\"\n\tendm\n\tm 5\n"
P["m_default_digit"] = "m\tmacro pa=2p\n\tdc.b \"pa\"\n\tendm\n\tm\n"
P["m_default_1x"] = "m\tmacro pa=1x2\n\tdc.b \"pa\"\n\tendm\n\tm\n"
P["m_default_num"] = "m\tmacro pa=12\n\tdc.b pa\n\tendm\n\tm\n"
P["m_default_hexh"] = "m\tmacro pa=0Fh\n\tdc.b pa\n\tendm\n\tm\n"
P["m_default_space"] = "m\tmacro pa=2p bin\n\tdc.b \"pa\"\n\tendm\n\tm\n"
P["m_default_str"] = "m\tmacro pa=\"2p,x\"\n\tdc.b pa\n\tendm\n\tm\n"
P["m_default_amp"] = "m\tmacro pa=2p&q\n\tdc.b \"pa\"\n\tendm\n\tm\n"
P["m_body_2p"] = "m\tmacro pa\n\tdc.b \"pa\"\n\tendm\n\tm 2p.bin\n"
for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as fh:
        fh.write(M + v + END)
print(len(P), "probes")
