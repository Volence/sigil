import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
M = "\tcpu 68000\n"
probes = {
 "cq1": M+"\tcharset 'B',$77\n\tcharset 'A','B'\n\tdc.b \"A\"\n",
 "cq2": M+"\tcharset $41,'BCD'\n\tdc.b \"ABC\"\n",
 "cq3": M+"\tcharset 'A',$99\n\tcharset 'a','c','A'\n\tdc.b \"abc\"\n",
 "cq4": M+"\tcharset 'C',$55\n\tcharset 'A','BC'\n\tdc.b \"AB\"\n",
 "cq5": M+"\tcharset 'B',$77\n\tcharset 'A',('B')\n\tdc.b \"A\"\n",
 "cq6": M+"\tcharset 'B',$77\n\tcharset 'A','B'+0\n\tdc.b \"A\"\n",
 "cq7": M+"\tcharset 'B',$77\n\tcharset 'B','A'\n\tdc.b \"AB\"\n",
}
for k, v in probes.items():
    open(os.path.join(P, k + ".asm"), "w").write(v)
print(" ".join(probes))
