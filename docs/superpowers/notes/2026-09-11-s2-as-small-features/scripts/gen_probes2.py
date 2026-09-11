#!/usr/bin/env python3
"""Round 2 probes: controls and the cases round 1 could not answer."""
import os

D = "/home/volence/sonic_hacks/.scratch/s2-as-small-features/probes"
M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
Z = "\tcpu z80\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
ZEND = "\tdb 0EEh\n\tend\n"
P = {}

# feature 1 controls
P["e1_disp0"] = M + "\tmove.w 0(a0),d0\n" + END
P["e1_paren0"] = M + "\tmove.w (0),d0\n" + END
P["e1_macro_omitted2"] = M + "m macro pa,pb\n\tdc.b (pa)|(pb)\n\tendm\n\tm 5\n" + END
P["e1_macro_omitted_bare2"] = M + "m macro pa,pb\n\tdc.b pb\n\tendm\n\tm 5\n" + END
P["e1z_macro_omitted2"] = Z + "m macro pa,pb\n\tdb (pa)|(pb)\n\tendm\n\tm 5\n" + ZEND
P["e1z_ld_a_paren0"] = Z + "\tld a,(0)\n" + ZEND
P["e1_func2_trailing"] = M + "f function x,y,x+y\n\tdc.b f(1,)\n" + END
P["e1_func2_leading"] = M + "f function x,y,x+y\n\tdc.b f(,1)\n" + END
P["e1_func2_parens"] = M + "f function x,y,x+y\n\tdc.b f((),1)\n" + END
P["e1_abs_empty"] = M + "\tdc.l abs()\n" + END
P["e1z_music"] = Z + (
    "z80_bank_size = 8000h\n"
    "getZ80BankOffset function label, label # z80_bank_size\n"
    "getZ80BankBase function label, label - getZ80BankOffset(label)\n"
    "withinSameZ80Bank function label1, label2, getZ80BankBase(label1) == getZ80BankBase(label2)\n"
    "MusFlag_SlowerOnPAL = 1 << 6\n"
    "music_metadata macro DATA,FLAGS\n"
    "\tdb\t(withinSameZ80Bank(DATA.pointer, MusicPoint2)<<7)|((~~DATA.is_compressed)<<5)|(FLAGS)|(getZ80BankOffset(DATA.pointer)/2)\n"
    "    endm\n"
    "Mus_EHZ.pointer = 12344h\nMus_EHZ.is_compressed = 1\n"
    "Mus_X.pointer = 9876h\nMus_X.is_compressed = 0\n"
    "MusicPoint2 = 10000h\n"
    "\tmusic_metadata Mus_EHZ\n"
    "\tmusic_metadata Mus_X,MusFlag_SlowerOnPAL\n") + ZEND

# feature 2: struct under 68000, used under Z80
S = ("zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n")
P["d2z_member2"] = "\tcpu 68000\n" + S + "\tcpu z80\n\torg 0\n\tld a,(zV.1upPlaying)\n\tld a,(ix+zV.1upPlaying)\n" + ZEND
P["d2_member_indented_nocolon"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying\tds.b 1\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.B\n\tdc.b zV.len\n") + END
P["d2_member_word"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1up:\tds.w 1\n\t2up:\tds.l 2\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.1up\n\tdc.b zV.2up\n\tdc.b zV.B\n\tdc.b zV.len\n") + END

# feature 4
P["p4_pop_into_equ_diff"] = M + "E equ 5\nA := 7\n\tpushv ,A\n\tpopv ,E\n\tdc.b E\n" + END
P["p4_pop_into_label_diff"] = M + "Lbl:\tdc.b 1\nA := 7\n\tpushv ,A\n\tpopv ,Lbl\n\tdc.b Lbl\n" + END
P["p4_push_label_pop_var"] = M + "\tdc.b 0\nLbl:\tdc.b 1\nA := 7\n\tpushv ,Lbl\n\tpopv ,A\n\tdc.b A\n" + END
P["p4_stack_case"] = M + "A := 1\n\tpushv S1,A\nA := 2\n\tpopv s1,A\n\tdc.b A\n" + END
P["p4_leftover_two"] = M + "A := 1\n\tpushv ,A\n\tpushv s1,A\n\tpushv s1,A\n\tdc.b A\n" + END
P["p4_in_macro"] = M + ("sv macro\n\tpushv ,Ver\nVer := 9\n\tdc.b Ver\n\tpopv ,Ver\n\tendm\n"
                        "Ver := 3\n\tsv\n\tdc.b Ver\n") + END

# feature 5: argument splitting
MM = "m\tmacro pa,pb,pc\n\tmessage \"(pa)(pb)(pc)[ALLARGS]\"\n\tdc.b ARGCOUNT\n\tendm\n"
for k, call in {
    "m5s_trim": "\tm   aa  ,  bb  ,cc   ; comment",
    "m5s_paren": "\tm (1,2),3",
    "m5s_bracket": "\tm [1,2],3",
    "m5s_eqeq": "\tm pb==5",
    "m5s_tab": "\tm a\t\tb",
    "m5s_empty": "\tm ,bb,",
    "m5s_spaceonly": "\tm aa,   ,cc",
    "m5s_at": "\tm a@b",
    "m5s_digits": "\tm 1up,2p.bin,3rd",
    "m5s_mixed": "\tm $10,007,%101",
}.items():
    P[k] = M + MM + call + "\n" + END
MQ = "m\tmacro pa,pb\n\tdc.b pa\n\tmessage \"(pb)\"\n\tdc.b ARGCOUNT\n\tendm\n"
P["m5s_quote"] = M + MQ + "\tm \"a,b\",c\n" + END
P["m5s_quote_semi"] = M + MQ + "\tm \"a;b\",c\n" + END
P["m5s_quote_ws"] = M + MQ + "\tm \" a \",c\n" + END
P["m5s_squote"] = M + "m\tmacro pa,pb\n\tdc.l pa\n\tmessage \"(pb)\"\n\tdc.b ARGCOUNT\n\tendm\n\tm 'a,b',c\n" + END
P["m5s_backslash"] = M + "pal\tmacro path\n\tdc.b \"<path>\"\n\tendm\n\tpal \\x41\n" + END
P["m5z_afshadow"] = Z + "m\tmacro pa,pb\n\tmessage \"(pa)(pb)\"\n\tdb ARGCOUNT\n\tendm\n\tm af',bb\n" + ZEND
P["m5_label_compose_ctl"] = M + "lbl\tmacro n\nLab_n:\tdc.b 1\n\tendm\n\tlbl 5\n\tdc.l Lab_5\n" + END
P["m5_label_compose_col0"] = M + "lbl\tmacro n\nLab_n:\tdc.b 1\n\tendm\n\tlbl 2p\n\tdc.l Lab_2p\n" + END
P["m5_label_compose_str"] = M + "lbl\tmacro n\n\tdc.b \"Lab_n\"\n\tendm\n\tlbl 2p\n" + END

# feature 6: where asl locates an error raised inside a macro expansion
CNOP = ("org\tmacro address\n\tif address < *\n\terror \"too much\"\n\telseif address > *\n\t!org address\n"
        "\tendif\n\tendm\ncnop\tmacro offset,alignment\n\torg (*-1+(alignment)-((*-1+(-(offset)))#(alignment)))\n\tendm\n")
P["l6_cnop_undef"] = M + CNOP + "Start:\n\tdc.b 1,2,3\n\tcnop -1,2<<lastbit(Nope)\n\tdc.b $00\n" + END

for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as f:
        f.write(v)
print(" ".join(k + ".asm" for k in P))
