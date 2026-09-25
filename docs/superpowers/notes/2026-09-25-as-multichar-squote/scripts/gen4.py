import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
M = "\tcpu 68000\n"
Z = "\tcpu z80\n"
probes = {
 "h1": M+"\tdc.w \"\\x99\\x41\"\n",
 "h2": M+"\tdc.l \"\\x99\\x41\"\n",
 "h3": Z+"\tdw \"\\x99\\x41\"\n",
 "h4": Z+"\tdw 'C\\x99\\x41'\n",
 "h5": M+"\tdc.w 'A\\x99\\x41'\n",
 "h6": Z+"\tdw \"\\x7f\\x80\"\n",
 "h7": M+"\tdc.l 'A\\x99\\x41\\x42\\x43'\n",
 "h8": Z+"\tdw 'AB'+1\n",
 "h9": Z+"\tdw 'A'+80h\n",
 "h10": M+"\tdc.w 'A'+$80\n",
 "h11": Z+"\tdb 'AB'+80h\n",
 "h12": Z+"\tcharset 'A',99h\n\tdw 'AB'+1\n",
 "h13": M+"\tdc.w \"A\",'BCD',$1234\n",
 "h14": M+"\tdc.b 'A'+$FF\n",
 "h15": M+"\tdc.w 'A'-$42+$80\n",
 "h16": M+"\tdc.w 'AB'+(-$4000)\n",
 "h17": Z+"\tdw 'AB'+(-4000h)\n",
 "h18": M+"\tdc.w [2]'ABC'\n",
 "h19": M+"\tdc.w ''+''\n",
 "h20": M+"\tdc.b ''+''\n",
 "h21": M+"\tdc.b ''+'',1\n",
}
for k, v in probes.items():
    open(os.path.join(P, k + ".asm"), "w").write(v)
print(" ".join(probes))
