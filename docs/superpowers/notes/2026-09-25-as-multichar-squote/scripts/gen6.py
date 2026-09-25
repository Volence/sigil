import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
M = "\tcpu 68000\n"
probes = {
 "t1": M+"\tdc.l 'A'+substr(\"BC\",0,1)+'D'\n",
 "t2": M+"\tdc.l 'A'+\"B\"+'C'\n",
 "t3": M+"X equ 'B'\n\tdc.l 'A'+X+'C'\n",
 "t4": M+"X equ 'A'+\"B\"+'C'\n\tdc.l X\n",
 "t5": M+"X equ ('AB')\n\tdc.w X\n",
 "t6": M+"X equ 'AB'\nY equ (X)\n\tdc.w Y\n",
 "t7": M+"X equ 'A'\n\tdc.w 'B'+X\n",
 "t8": M+"\tdc.l 'A'+1+'C'\n",
 "t9": M+"\tdc.l 'A'+('B')+'C'\n",
 "t10": M+"\tdc.l ''+'A'+''\n",
 "t11": M+"\tdc.w 'A' + 'B'\n",
 "t12": M+"\tdc.w  'AB' \n",
 "t13": M+"\tdc.w ('A'+'B')\n",
 "t14": M+"\tdc.w ('A')\n",
 "t15": M+"\tdc.w (('A')+'B')\n",
 "t16": M+"\tdc.w ('A'+('B'))\n",
 "t17": M+"\tdc.l lowstring('AB')+'C'\n",
 "t18": M+"\tdc.l 'A'+lowstring('BC')\n",
 "t19": M+"\tdc.l 'A'+lowstring('b')+'C'\n",
}
for k, v in probes.items():
    open(os.path.join(P, k + ".asm"), "w").write(v)
print(" ".join(probes))
