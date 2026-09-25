import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
M = "\tcpu 68000\n"
Z = "\tcpu z80\n"
ZCS = "\tcharset 'A',11h\n\tcharset 'B',22h\n\tcharset 'C',99h\n"
probes = {
 "cp1": M+"\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tdc.b 'AB'+1\n",
 "cp2": M+"\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.b 'AB'+2\n",
 "cp3": M+"\tcharset $23,$77\n\tdc.b 'AB'+1\n",
 "cp4": M+"\tcharset $43,$77\n\tdc.b 'AB'+1\n",
 "cp5": M+"\tcharset 'A',$11\n\tdc.b 'AB'+1\n",
 "cp6": M+"\tcharset 'B',$22\n\tdc.b 'AB'+1\n",
 "cp7": M+"\tcharset 'A',$51\n\tdc.b 'AB'+1\n",
 "cp8": M+"\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tdc.b \"AB\"+1\n",
 "zcs1": Z+ZCS+"\tdb 'CAB'\n",
 "zcs2": Z+ZCS+"\tdw 'CA'\n",
 "zcs3": Z+ZCS+"\tld hl,'BC'\n",
 "zcs4": Z+ZCS+"\tdw 'CAB'\n",
 "ff1": M+"\tdc.w 'A'+\"B\"\n",
 "ff2": M+"\tdc.w \"A\"+'B'\n",
 "ff3": M+"\tdc.w ''+'A'\n",
 "ff4": M+"\tdc.w 'AB'+''\n",
 "ff5": M+"\tdc.w 'ABC'+''\n",
 "ff6": M+"\tdc.w ('A'+'B')\n",
 "ff7": M+"X equ 'A'+'B'\n\tdc.w X\n",
 "ff8": M+"X equ 'AB'+1\n\tdc.w X\n",
 "ff9": M+"\tdc.l 'AB'+'CD'\n",
 "ff10": M+"\tdc.l 'AB'+'CDE'\n",
 "ff11": M+"\tdc.w \"A\"+\"B\"\n",
 "big7": M+"\tmove.l #'ABCDEF'|0,d0\n",
 "big8": M+"\tmove.l #'ABCDEFGHIJ'|0,d0\n",
 "big9": M+"\tmove.l #'XYZWVU'|0,d0\n",
 "zdb": Z+"\tdb 'A'+'B'\n\tdw 'A'+'B'\n",
 "mw3": M+"m macro a\n\tdc.w a+1\n\tendm\n\tm 'AB'\n",
 "subs": M+"X equ 'AB'\n\tdc.w substr(X,0,2)\n",
 "sym3": M+"X set 'A'\nX set X+'B'\n\tdc.w X\n",
}
for k, v in probes.items():
    open(os.path.join(P, k + ".asm"), "w").write(v)
print(" ".join(probes))
