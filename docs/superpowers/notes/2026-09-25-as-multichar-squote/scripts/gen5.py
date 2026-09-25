import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
M = "\tcpu 68000\n"
probes = {
 "f1": M+"X equ 'A'\n\tdc.w X+'B'\n",
 "f2": M+"X equ 'B'\n\tdc.w 'A'+X\n",
 "f3": M+"X equ 'A'\nY equ 'B'\n\tdc.w X+Y\n",
 "f4": M+"X equ 'AB'\n\tdc.w (X)\n",
 "f5": M+"X equ 'A'\nY equ X+'B'\n\tdc.w Y\n",
 "f6": M+"\tdc.w ('A')+'B'\n",
 "f7": M+"\tdc.w 'A'+('B')\n",
 "f8": M+"\tdc.l 'A'+'B'+'C'\n",
 "f9": M+"\tdc.l ('A'+'B')+'C'\n",
 "f10": M+"\tdc.w substr('AB',0,2)+''\n",
 "f11": M+"X equ 'AB'\n\tdc.w X\n\tdc.w X+''\n",
 "f12": M+"X set 'A'\n\tdc.w X\n\tdc.w X+'B'\n",
 "f13": M+"X equ 'AB'\n\tdc.w X+0\n",
 "f14": M+"\tdc.w 'AB'+0\n",
 "f15": M+"X equ \"A\"\n\tdc.w X+'B'\n",
 "f16": M+"\tdc.w (('A')+('B'))\n",
}
for k, v in probes.items():
    open(os.path.join(P, k + ".asm"), "w").write(v)
print(" ".join(probes))
