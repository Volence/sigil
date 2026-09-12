#!/usr/bin/env python3
"""The committed evidence set. Every probe whose run must exit 0 may hold many
constructs (a clean run makes every line a value source); every refusal is one
construct per file, so its verdict is attributable. Tables are copied from
round 2. Written into probes4/."""
import os, shutil

S = "/home/volence/sonic_hacks/.scratch/as-missing-builtins"
D = f"{S}/probes4"
os.makedirs(D, exist_ok=True)
for f in os.listdir(D):
    os.remove(os.path.join(D, f))

M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
Z = "\tcpu z80\n\torg 0\n"
ZEND = "\tdb 0EEh\n\tend\n"
P = {}

# fn -> (argument, a nested call that stays in the accepted domain, mixed-case spelling)
CTX = {
    "sgn": ("-5", "sgn(sgn(-5))", "Sgn"),
    "bitcnt": ("7", "bitcnt(bitcnt(7))", "BitCnt"),
    "firstbit": ("12", "firstbit(firstbit(12))", "FirstBit"),
    "bitpos": ("8", "bitpos(bitpos(4))", "BitPos"),
    "toupper": ("97", "toupper(toupper(97))", "ToUpper"),
    "tolower": ("65", "tolower(tolower(65))", "ToLower"),
}
for f, (a, nested, mixed) in CTX.items():
    # asl refuses bitpos of a forward reference (#1540 on pass 1, where the
    # symbol is still 0), so bitpos's forward cases are refusal files below.
    fwd_label = "" if f == "bitpos" else f"\tdc.b {f}(FwdL)\n\tds.b $3F\nFwdL:\n"
    fwd_equ = "" if f == "bitpos" else f"\tdc.b {f}(Later)\n"
    later = "" if f == "bitpos" else f"Later equ {a}\n"
    empty = "" if f == "bitpos" else f"\tdc.b {f}()\n\tdc.b {f}( )\n\tdc.b {f}(())\n"
    body = (
        fwd_label
        + f"\tdc.b {f}({a})\n"
        f"\tdc.b {f.upper()}({a})\n"
        f"\tdc.b {mixed}({a})\n"
        f"\tdc.b {f}( {a} )\n"
        f"\tdc.b {f}(({a}))\n"
        + fwd_equ
        + f"X equ {f}({a})\n\tdc.b X\n"
        f"Y set {f}({a})\n\tdc.b Y\n"
        f"\tif {f}({a})={f}({a})+1\n\tdc.b 2\n\telse\n\tdc.b 1\n\tendif\n"
        f"\tmove.l #{f}({a}),d0\n"
        f"\tmove.w #{f}({a})<<2,d1\n"
        f"\tdc.b {f}({a})+1\n"
        f"\tdc.b {nested}\n"
        f"\tdc.b \"\\{{{f}({a})}}\"\n"
        + empty
        + later
    )
    P[f"ctx_{f}"] = M + body + END
    P[f"ctx_{f}_z80"] = Z + f"\tdb {f}({a})\n\tld a,{f}({a})\n\tld hl,{f}({a})\n" + ZEND
P["ref_bitpos_fwd"] = M + "\tdc.b bitpos(Later)\nLater equ 8\n" + END
P["ref_bitpos_fwdlabel"] = M + "\tdc.b bitpos(FwdL)\n\tds.b $3F\nFwdL:\n" + END
# sgn also takes a float, and answers an integer for it
P["ctx_sgn_float"] = M + ("\tdc.l sgn(-2.5)\n\tdc.b sgn(2.5)\n\tmove.l #sgn(-0.5),d0\n"
                          "F set 2.5\n\tdc.l sgn(F)\n\tdc.l sgn(0.1-0.2)\n\tdc.l sgn(sqrt(2))\n"
                          "\tdc.l INT(sgn(-2.5))\n\tdc.l sgn(-0.0)\n") + END
# the empty argument on the builtins that predate this family: everything asl
# accepts, in one clean file (each new builtin's empty call is in its ctx file)
EMPTY_OK = ["\tdc.l int()", "\tdc.l abs()", "\tdc.b abs()", "\tdc.l lastbit()",
            "\tdc.l abs( )", "\tdc.l abs(())", "\tmove.l #abs(),d0", "X equ abs()\n\tdc.l X", "\tdc.l abs()+5",
            "\tdc.b int( )", "\tif abs()=0\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif"]
for fn in ["sin", "cos", "tan", "atan", "asin", "acos", "sinh", "cosh", "tanh", "asinh", "atanh", "sqrt", "exp"]:
    EMPTY_OK.append(f"\tdc.l INT({fn}())")
P["empty"] = M + "\n".join(EMPTY_OK) + "\n" + END
P["empty_z80"] = Z + "\tdb abs()\n\tld a,lastbit()\n\tdb int()\n" + ZEND
for name, line in [("log", "\tdc.l INT(log())"), ("ln", "\tdc.l INT(ln())"), ("acosh", "\tdc.l INT(acosh())"),
                   ("bitpos", "\tdc.l bitpos()"), ("sin_bare", "\tdc.l sin()")]:
    P[f"ref_empty_{name}"] = M + line + "\n" + END

# refusals, one per file
for f in ["sgn", "bitcnt", "firstbit", "bitpos", "toupper", "tolower"]:
    if f != "sgn":
        P[f"ref_{f}_float"] = M + f"\tdc.l {f}(5.0)\n" + END
        P[f"ref_{f}_floatsym"] = M + f"F set 2.5\n\tdc.l {f}(F)\n" + END
    P[f"ref_{f}_string"] = M + f"\tdc.l {f}(\"A\")\n" + END
    P[f"ref_{f}_twoarg"] = M + f"\tdc.l {f}(1,2)\n" + END
    P[f"ref_{f}_undef"] = M + f"\tdc.l {f}(Nope)\n" + END
    P[f"ref_{f}_char"] = M + f"\tdc.l {f}('a')\n" + END
for a, tag in [("0", "0"), ("6", "6"), ("-1", "m1"), ("-$7FFFFFFFFFFFFFFF-1", "min"), ("$FFFEB", "fffeb")]:
    P[f"ref_bitpos_{tag}"] = M + f"\tdc.l bitpos({a})\n" + END
for f in ["toupper", "tolower"]:
    P[f"ref_{f}_256"] = M + f"\tdc.b {f}(256)\n" + END
    P[f"ref_{f}_m1"] = M + f"\tdc.b {f}(-1)\n" + END
# the pre-existing builtins on a character constant, for the open row
for f in ["lastbit", "abs", "int"]:
    P[f"ref_{f}_char"] = M + f"\tdc.l {f}('a')\n" + END
P["ref_lastbit_charsym"] = M + "Q equ 'a'\n\tdc.l lastbit(Q)\n" + END

# names: a user function, symbol or label spelled like a builtin
NAMES = ["sgn", "bitcnt", "firstbit", "bitpos", "toupper", "tolower", "lastbit", "abs", "int"]
P["names_userfn"] = M + "".join(f"{f} function x,x+100\n" for f in NAMES) + "".join(f"\tdc.b {f}(1)\n" for f in NAMES) + END
P["names_userfn_upper"] = M + "".join(f"{f.upper()} function x,x+100\n" for f in NAMES) + "".join(f"\tdc.b {f}(8)\n" for f in NAMES) + END
P["names_symbol"] = M + "".join(f"{f} equ 7\n" for f in NAMES) + "".join(f"\tdc.b {f},{f}(8)\n" for f in NAMES) + END

# Sonic 1's signedToString (s1disasm MacroSetup.asm(221)), and interpolation
SIG = 'signedToString function number,substr("-",0,-sgn(number))+"$\\{abs(number)}"\n'
P["signed"] = M + SIG + ('\tdc.b signedToString(-5)\n\tdc.b signedToString(5)\n\tdc.b signedToString(0)\n'
                         '\tdc.b signedToString(-$123)\n\tdc.b "\\{sgn(-5)}"\n\tdc.b "\\{bitcnt(-1)}"\n') + END

for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as fh:
        fh.write(v)
for t in ["t_firstbit", "t_bitcnt", "t_lastbit", "t_sgn", "t_abs", "t_bitpos", "t_toupper", "t_tolower", "t_sgn_float"]:
    shutil.copy(f"{S}/probes2/{t}.asm", f"{D}/{t}.asm")
print(len(P), "probes + 9 tables")
