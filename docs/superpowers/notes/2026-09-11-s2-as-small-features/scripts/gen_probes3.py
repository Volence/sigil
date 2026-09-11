#!/usr/bin/env python3
"""Round 3 probes: the real music_metadata shape with in-range values, keyword
argument spacing, and user-function empty arguments."""
import os

D = "/home/volence/sonic_hacks/.scratch/s2-as-small-features/probes"
M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
Z = "\tcpu z80\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
ZEND = "\tdb 0EEh\n\tend\n"
P = {}

P["e1z_music2"] = Z + (
    "z80_bank_size = 8000h\n"
    "getZ80BankOffset function label, label # z80_bank_size\n"
    "getZ80BankBase function label, label - getZ80BankOffset(label)\n"
    "withinSameZ80Bank function label1, label2, getZ80BankBase(label1) == getZ80BankBase(label2)\n"
    "MusFlag_SlowerOnPAL = 1 << 6\n"
    "music_metadata macro DATA,FLAGS\n"
    "\tdb\t(withinSameZ80Bank(DATA.pointer, MusicPoint2)<<7)|((~~DATA.is_compressed)<<5)|(FLAGS)|(getZ80BankOffset(DATA.pointer)/2)\n"
    "    endm\n"
    "Mus_EHZ.pointer = 10020h\nMus_EHZ.is_compressed = 1\n"
    "Mus_X.pointer = 8030h\nMus_X.is_compressed = 0\n"
    "MusicPoint2 = 10000h\n"
    "\tmusic_metadata Mus_EHZ\n"
    "\tmusic_metadata Mus_X,MusFlag_SlowerOnPAL\n") + ZEND

MM = "m\tmacro pa,pb,pc\n\tmessage \"(pa)(pb)(pc)[ALLARGS]\"\n\tdc.b ARGCOUNT\n\tendm\n"
for k, call in {
    "m5k_value_space": "\tm pb= 5",
    "m5k_name_space": "\tm pb =5",
    "m5k_both_space": "\tm aa, pb = 5 ",
    "m5k_le": "\tm 2<=3",
    "m5k_eq_in_paren": "\tm (pb=5)",
    "m5k_eq_in_str": "\tm \"pb=5\"",
}.items():
    P[k] = M + MM + call + "\n" + END

P["e1_func3_middle"] = M + "f function x,y,z,x+y+z\n\tdc.b f(1,,2)\n" + END
P["e1_func1_space"] = M + "f function x,x+1\n\tdc.b f( )\n" + END
P["e1_func2_both"] = M + "f function x,y,x+y\n\tdc.b f(,)\n" + END

for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as f:
        f.write(v)
print(" ".join(k + ".asm" for k in P))
