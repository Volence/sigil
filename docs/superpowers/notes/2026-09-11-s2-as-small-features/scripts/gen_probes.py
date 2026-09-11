#!/usr/bin/env python3
"""Write the probe sources, one construct per file, into probes/."""
import os

D = "/home/volence/sonic_hacks/.scratch/s2-as-small-features/probes"
os.makedirs(D, exist_ok=True)

M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
Z = "\tcpu z80\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
ZEND = "\tdb 0EEh\n\tend\n"

P = {}

# ---- feature 1: empty () -------------------------------------------------
e1 = {
    "e1_db_or": "\tdc.b $10|()|$02\n",
    "e1_db_bare": "\tdc.b ()\n",
    "e1_dw": "\tdc.w ()\n",
    "e1_dl": "\tdc.l ()\n",
    "e1_db_space": "\tdc.b ( )\n",
    "e1_db_nested": "\tdc.b (())\n",
    "e1_db_neg": "\tdc.b -()\n",
    "e1_db_not": "\tdc.b ~()\n",
    "e1_db_lognot": "\tdc.b ~~()\n",
    "e1_db_add_after": "\tdc.b ()+1\n",
    "e1_db_add_before": "\tdc.b 1+()\n",
    "e1_db_mul": "\tdc.b 5*()\n",
    "e1_db_shl": "\tdc.b ()<<1\n",
    "e1_db_cmp": "\tdc.b ()=0\n",
    "e1_db_div": "\tdc.b 4/()\n",
    "e1_imm": "\tmove.w #(),d0\n",
    "e1_imm_or": "\tmove.w #1|(),d0\n",
    "e1_equ": "X equ ()\n\tdc.b X\n",
    "e1_set": "X set ()\n\tdc.b X\n",
    "e1_coloneq": "X := ()\n\tdc.b X\n",
    "e1_if": "\tif ()\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif\n",
    "e1_if_or": "\tif ()|1\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif\n",
    "e1_ds": "\tds.b ()\n",
    "e1_rept": "\trept ()\n\tdc.b 1\n\tendm\n",
    "e1_disp": "\tmove.w ()(a0),d0\n",
    "e1_indirect": "\tmove.w (),d0\n",
    "e1_absw": "\tmove.w ().w,d0\n",
    "e1_func_call": "f function x,x+1\n\tdc.b f()\n",
    "e1_func_paren": "f function x,x+1\n\tdc.b f(())\n",
    "e1_macro_omitted": "m macro a,b\n\tdc.b (a)|(b)\n\tendm\n\tm 5\n",
    "e1_macro_omitted_bare": "m macro a,b\n\tdc.b b\n\tendm\n\tm 5\n",
    "e1_db_empty_item": "\tdc.b 1,,2\n",
    "e1_db_trailing_comma": "\tdc.b 1,\n",
}
for k, v in e1.items():
    P[k] = M + v + END
e1z = {
    "e1z_db_or": "\tdb 10h|()|02h\n",
    "e1z_db_bare": "\tdb ()\n",
    "e1z_dw": "\tdw ()\n",
    "e1z_ld_a": "\tld a,()\n",
    "e1z_ld_a_or": "\tld a,1|()\n",
    "e1z_ld_hl": "\tld hl,()\n",
    "e1z_ld_a_paren_or": "\tld a,(1|())\n",
    "e1z_macro_omitted": "m macro a,b\n\tdb (a)|(b)\n\tendm\n\tm 5\n",
}
for k, v in e1z.items():
    P[k] = Z + v + ZEND

# ---- feature 2: digit-led struct member ---------------------------------
P["d2_member_dots"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.1upPlaying\n\tdc.b zV.B\n\tdc.b zV.len\n") + END
P["d2_member_nodots"] = M + (
    "zV STRUCT\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV_1upPlaying\n\tdc.b zV_B\n\tdc.b zV_len\n") + END
P["d2_member_expr"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 2\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.1upPlaying+1\n\tdc.b 2*zV.1upPlaying\n") + END
P["d2_member_undef"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.1foo\n") + END
P["d2_member_alldigit"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t2:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.len\n") + END
P["d2_member_hexh"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t12h:\tds.b 1\n\tB:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.B\n\tdc.b zV.len\n") + END
P["d2_member_nocolon"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n1upPlaying\tds.b 1\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.1upPlaying\n\tdc.b zV.B\n\tdc.b zV.len\n") + END
P["d2_member_marker"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1up:\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b zV.1up\n\tdc.b zV.B\n\tdc.b zV.len\n") + END
P["d2_member_bare_ref"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b 1upPlaying\n") + END
P["d2_instance"] = M + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdc.b $11\nInst:\tzV\n\tdc.l Inst.1upPlaying\n") + END
P["d2_label"] = M + "\tdc.b $11\n1up:\tdc.b $22\n" + END
P["d2_label_nocolon"] = M + "\tdc.b $11\n1up\tdc.b $22\n" + END
P["d2_label_equ"] = M + "1up equ 5\n" + END
P["d2_label_set"] = M + "1up = 5\n" + END
P["d2_label_dotted"] = M + "\tdc.b $11\nFoo.1up:\tdc.b $22\n\tdc.b Foo.1up\n" + END
P["d2z_member"] = Z + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n"
    "\tld a,(zV.1upPlaying)\n\tld a,(ix+zV.1upPlaying)\n") + ZEND
P["d2z_instance"] = Z + (
    "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n"
    "\tdb 11h\nzAbs:\tzV\n\tld a,(zAbs.1upPlaying)\n") + ZEND

# ---- feature 3: shift/rotate with no size -------------------------------
SH = ["asl", "asr", "lsl", "lsr", "rol", "ror", "roxl", "roxr"]
EAS = ["$1A(a0)", "(a1)", "(a2)+", "-(a3)", "$10(a4,d0.w)", "$1234.w", "$12345678.l", "$18(a0)"]
P["s3_mem_nosize"] = M + "".join(f"\t{m}\t{ea}\n" for m, ea in zip(SH, EAS)) + END
P["s3_mem_w"] = M + "".join(f"\t{m}.w\t{ea}\n" for m, ea in zip(SH, EAS)) + END
P["s3_mem_nosize_rot"] = M + "".join(f"\t{m}\t{ea}\n" for m, ea in zip(SH, EAS[1:] + EAS[:1])) + END
P["s3_reg_imm_nosize"] = M + "".join(f"\t{m}\t#{i+1},d{i}\n" for i, m in enumerate(SH)) + END
P["s3_reg_imm_w"] = M + "".join(f"\t{m}.w\t#{i+1},d{i}\n" for i, m in enumerate(SH)) + END
P["s3_reg_reg_nosize"] = M + "".join(f"\t{m}\td{(i+1)%8},d{i}\n" for i, m in enumerate(SH)) + END
P["s3_reg_reg_w"] = M + "".join(f"\t{m}.w\td{(i+1)%8},d{i}\n" for i, m in enumerate(SH)) + END
for m in SH:
    P[f"s3_reg_single_{m}"] = M + f"\t{m}\td3\n" + END
    P[f"s3_reg_single_w_{m}"] = M + f"\t{m}.w\td3\n" + END
    P[f"s3_mem_b_{m}"] = M + f"\t{m}.b\t(a0)\n" + END
    P[f"s3_mem_l_{m}"] = M + f"\t{m}.l\t(a0)\n" + END
P["s3_mem_count"] = M + "\tasl\t#2,(a0)\n" + END
P["s3_upper"] = M + "\tASL\t$1A(a0)\n\tRoXr\t(a1)\n" + END

# ---- feature 4: pushv / popv -------------------------------------------
P["p4_basic"] = M + "Ver := 2\n\tdc.b Ver\n\tpushv ,Ver\nVer := 4\n\tdc.b Ver\n\tpopv ,Ver\n\tdc.b Ver\n" + END
P["p4_nested"] = M + ("Ver := 2\n\tpushv ,Ver\nVer := 4\n\tpushv ,Ver\nVer := 6\n\tdc.b Ver\n"
                      "\tpopv ,Ver\n\tdc.b Ver\n\tpopv ,Ver\n\tdc.b Ver\n") + END
P["p4_multi_same"] = M + ("A := 1\nB := 2\n\tpushv ,A,B\nA := 3\nB := 4\n\tpopv ,A,B\n\tdc.b A,B\n") + END
P["p4_multi_rev"] = M + ("A := 1\nB := 2\n\tpushv ,A,B\nA := 3\nB := 4\n\tpopv ,B,A\n\tdc.b A,B\n") + END
P["p4_multi_split"] = M + ("A := 1\nB := 2\n\tpushv ,A,B\nA := 3\nB := 4\n\tpopv ,A\n\tdc.b A,B\n\tpopv ,A\n\tdc.b A,B\n") + END
P["p4_named"] = M + ("A := 1\n\tpushv s1,A\nA := 2\n\tpushv s2,A\nA := 3\n\tpopv s1,A\n\tdc.b A\n\tpopv s2,A\n\tdc.b A\n") + END
P["p4_named_vs_default"] = M + ("A := 1\n\tpushv ,A\nA := 2\n\tpushv s1,A\nA := 3\n\tpopv ,A\n\tdc.b A\n\tpopv s1,A\n\tdc.b A\n") + END
P["p4_pop_empty"] = M + "A := 1\n\tpopv ,A\n\tdc.b A\n" + END
P["p4_leftover"] = M + "A := 1\n\tpushv ,A\n\tdc.b A\n" + END
P["p4_undef"] = M + "\tpushv ,Nope\n\tpopv ,Nope\n" + END
P["p4_label"] = M + "Lbl:\tdc.b 1\n\tpushv ,Lbl\n\tpopv ,Lbl\n" + END
P["p4_equ"] = M + "E equ 5\n\tpushv ,E\n\tpopv ,E\n\tdc.b E\n" + END
P["p4_pop_into_new"] = M + "A := 7\n\tpushv ,A\n\tpopv ,B\n\tdc.b A,B\n" + END
P["p4_set"] = M + "Ver set 2\n\tpushv ,Ver\nVer set 4\n\tdc.b Ver\n\tpopv ,Ver\n\tdc.b Ver\n" + END
P["p4_upper"] = M + "Ver := 2\n\tPUSHV ,Ver\nVer := 4\n\tdc.b Ver\n\tPOPV ,Ver\n\tdc.b Ver\n" + END
P["p4_string"] = M + "S := \"ab\"\n\tpushv ,S\nS := \"cd\"\n\tdc.b S\n\tpopv ,S\n\tdc.b S\n" + END
P["p4_nocomma"] = M + "Ver := 2\n\tpushv Ver\n\tpopv Ver\n\tdc.b Ver\n" + END
P["p4_multipass"] = M + ("Ver := 2\n\tpushv ,Ver\nVer := 4\n\tbra.w Fwd\n\tdc.b Ver\n\tpopv ,Ver\n"
                         "Fwd:\tdc.b Ver\n") + END
P["p4_pop_twice"] = M + "A := 1\n\tpushv ,A\n\tpopv ,A\n\tpopv ,A\n\tdc.b A\n" + END
P["p4_float"] = M + "F := 1.5\n\tpushv ,F\nF := 2.5\n\tpopv ,F\n\tdc.l int(F*2)\n" + END
P["p4_empty_list"] = M + "\tpushv ,\n" + END
P["p4_z80"] = Z + "Ver := 2\n\tpushv ,Ver\nVer := 4\n\tdb Ver\n\tpopv ,Ver\n\tdb Ver\n" + ZEND

# ---- feature 5: macro argument text ------------------------------------
PAL = "pal\tmacro path\n\tdc.b \"path\"\n\tendm\n"
for k, arg in {
    "m5_1bin": "Special Stage 1.bin",
    "m5_2p": "Special Stage 1 2p.bin",
    "m5_1up": "1up",
    "m5_hexh": "12h",
    "m5_dollar": "$10",
    "m5_leading_zero": "007",
    "m5_percent": "%101",
    "m5_0x": "0x41",
    "m5_float": "1.5e3",
    "m5_words": "1st 2nd 3rd 4th.bin",
    "m5_digitdot": "2.bin",
    "m5_hexish": "0FFh 1F 2G",
    "m5_char": "'A'",
    "m5_spaces": "a  1   2p",
    "m5_expr": "1+2",
    "m5_neg": "-1",
}.items():
    P[k] = M + PAL + f"\tpal {arg}\n" + END
P["m5_label_compose"] = M + "lbl\tmacro n\nLab_n:\tdc.b 1\n\tendm\n\tlbl 2p\n\tdc.l Lab_2p\n" + END
P["m5_value_2p"] = M + "val\tmacro n\n\tdc.b n\n\tendm\n\tval 2p\n" + END
P["m5_value_hexh"] = M + "val\tmacro n\n\tdc.b n\n\tendm\n\tval 12h\n" + END
P["m5_value_dollar"] = M + "val\tmacro n\n\tdc.b n\n\tendm\n\tval $10\n" + END
P["m5_value_zero"] = M + "val\tmacro n\n\tdc.b n\n\tendm\n\tval 007\n" + END
P["m5z_2p"] = Z + "pal\tmacro path\n\tdb \"path\"\n\tendm\n\tpal Special Stage 1 2p.bin\n" + ZEND
P["m5z_hex"] = Z + "pal\tmacro path\n\tdb \"path\"\n\tendm\n\tpal 0101b 1Fh 0FFh\n" + ZEND
P["m5_interp"] = M + "pal\tmacro path\n\tmessage \"[path]\"\n\tendm\n\tpal Special Stage 1 2p.bin\n" + END

# ---- feature 6: lastbit -------------------------------------------------
P["l6_values"] = M + ("\tdc.b lastbit(1)\n\tdc.b lastbit(5)\n\tdc.b lastbit($80)\n\tdc.b lastbit($FFFEB)\n"
                      "\tdc.l 2<<lastbit($FFFEB)\n") + END
P["l6_zero"] = M + "\tdc.l lastbit(0)\n" + END
P["l6_neg"] = M + "\tdc.l lastbit(-1)\n" + END
P["l6_neg2"] = M + "\tdc.l lastbit(-2)\n" + END
P["l6_high"] = M + "\tdc.l lastbit($80000000)\n\tdc.l lastbit($7FFFFFFF)\n" + END
P["l6_64"] = M + "\tdc.l lastbit($100000000)\n" + END
P["l6_case"] = M + "\tdc.b LASTBIT(5)\n\tdc.b LastBit(9)\n" + END
P["l6_fwd"] = M + "\tdc.b lastbit(Later)\nLater equ 5\n" + END
P["l6_fwd_label"] = M + "\tdc.b lastbit(Later)\n\tds.b $40\nLater:\tdc.b 1\n" + END
P["l6_star"] = M + "\torg $123\n\tdc.l 2<<lastbit(*-1)\n" + END
P["l6_equ"] = M + "X equ lastbit(5)\n\tdc.b X\n" + END
P["l6_if"] = M + "\tif lastbit(4)=2\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif\n" + END
P["l6_noarg"] = M + "\tdc.b lastbit()\n" + END
P["l6_twoarg"] = M + "\tdc.b lastbit(1,2)\n" + END
P["l6_float"] = M + "\tdc.b lastbit(5.0)\n" + END
P["l6_string"] = M + "\tdc.b lastbit(\"A\")\n" + END
P["l6_undef"] = M + "\tdc.b lastbit(Nope)\n" + END
P["l6_firstbit"] = M + "\tdc.b firstbit(12)\n" + END
P["l6_bitcnt"] = M + "\tdc.b bitcnt(7)\n" + END
P["l6_imm"] = M + "\tmove.l #2<<lastbit($FFFEB),d0\n" + END
P["l6_ds"] = M + "\tds.b lastbit(5)\n" + END
P["l6_nested"] = M + "\tdc.b lastbit(lastbit($FFFEB))\n" + END
P["l6z_db"] = Z + "\tdb lastbit(5)\n\tld a,lastbit(9)\n" + ZEND
CNOP = ("org\tmacro address\n\tif address < *\n\terror \"too much\"\n\telseif address > *\n\t!org address\n"
        "\tendif\n\tendm\ncnop\tmacro offset,alignment\n\torg (*-1+(alignment)-((*-1+(-(offset)))#(alignment)))\n\tendm\n")
P["l6_cnop"] = M + CNOP + "Start:\n\tdc.b 1,2,3\n\tcnop -1,2<<lastbit(*-Start-1)\n\tdc.b $00\n" + END
P["l6_cnop_big"] = M + CNOP + "Start:\n\tds.b $123\n\tcnop -1,2<<lastbit(*-Start-1)\n\tdc.b $00\nEndR:\n\tdc.l EndR\n" + END

# ---- feature 7: shared --------------------------------------------------
P["s7_basic"] = M + "\tdc.w $1234\nFoo:\tmove.w #$0F64,d7\n\tshared Foo\n" + END
P["s7_undef"] = M + "\tshared Nope\n" + END
P["s7_multi"] = M + "A:\tdc.b 1\nB:\tdc.b 2\n\tshared A,B\n" + END
P["s7_noarg"] = M + "\tshared\n" + END
P["s7_twice"] = M + "A:\tdc.b 1\n\tshared A\n\tshared A\n" + END
P["s7_set"] = M + "V := 3\n\tshared V\n" + END
P["s7_fwd"] = M + "\tshared Later\nLater:\tdc.b 1\n" + END
P["s7_expr"] = M + "A:\tdc.b 1\n\tshared A+1\n" + END
P["s7_upper"] = M + "A:\tdc.b 1\n\tSHARED A\n" + END
P["s7z"] = Z + "A:\tdb 1\n\tshared A\n" + ZEND

for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as f:
        f.write(v)
print(len(P), "probes")
