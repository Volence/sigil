#!/usr/bin/env python3
"""Generate the AS-MACRO-LABEL-LEAK shape matrix.

One shape per file: an error anywhere in an asl run poisons every value in it,
so each file carries exactly one suspect line, the reference marked `; REF`.
Everything is placed off address zero (`org $100`, then a $1111 filler word),
so a bound label reads as a non-zero address and cannot be confused with a
zero-filled unresolved fixup.
"""
import os, sys

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

HEAD = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
TAIL = "\tdc.w\t$4444\n"

shapes = {}

def S(name, desc, body, extra_files=None):
    shapes[name] = (desc, HEAD + body + TAIL, extra_files or {})

# ---- A. plain PC labels in a macro body, read from outside -----------------
S("a01_colon", "plain colon label in body, read after the expansion",
  "mac\tmacro\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLp\t; REF\n")
S("a02_bare_col0", "colon-less column-0 label on a data line in body, read after",
  "mac\tmacro\nLp\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLp\t; REF\n")
S("a03_bare_alone", "colon-less column-0 label alone on its line in body, read after",
  "mac\tmacro\nLp\n\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLp\t; REF\n")
S("a04_indented_colon", "indented colon label in body, read after",
  "mac\tmacro\n\tLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLp\t; REF\n")
S("a05_forward", "plain colon label in body, read BEFORE the expansion",
  "mac\tmacro\nLp:\tdc.w\t$2222\n\tendm\n\tdc.w\tLp\t; REF\n\tmac\n")
S("a06_later_other_macro", "label from mdef read inside a LATER expansion of a different macro",
  "mdef\tmacro\nLp:\tdc.w\t$2222\n\tendm\nmref\tmacro\n\tdc.w\tLp\t; REF\n\tendm\n\tmdef\n\tmref\n")
S("a07_earlier_other_macro", "label from mdef read inside an EARLIER expansion of a different macro",
  "mdef\tmacro\nLp:\tdc.w\t$2222\n\tendm\nmref\tmacro\n\tdc.w\tLp\t; REF\n\tendm\n\tmref\n\tmdef\n")
S("a08_defined_fn", "DEFINED() of a body label, asked after the expansion",
  "mac\tmacro\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tif\tdefined(Lp)\t; REF\n\tdc.w\t$AAAA\n\telse\n\tdc.w\t$BBBB\n\tendif\n")
S("a09_ifdef", "IFDEF of a body label, asked after the expansion",
  "mac\tmacro\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tifdef\tLp\t; REF\n\tdc.w\t$AAAA\n\telse\n\tdc.w\t$BBBB\n\tendif\n")
S("a10_dot_under_body_label", "body `Lp:` then `.x:`, read `Lp.x` after",
  "mac\tmacro\nLp:\tdc.w\t$2222\n.x:\tdc.w\t$3333\n\tendm\n\tmac\n\tdc.w\tLp.x\t; REF\n")
S("a11_scope_after_body_label_inner", "body `Inner:`; after the call `.b := 2`; read `Inner.b`",
  "Base:\tdc.w\t$5555\nmac\tmacro\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("a12_scope_after_body_label_base", "body `Inner:`; after the call `.b := 2`; read `Base.b`",
  "Base:\tdc.w\t$5555\nmac\tmacro\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("a13_twice_then_outside", "macro invoked twice, then read from outside",
  "mac\tmacro\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tmac\n\tdc.w\tLp\t; REF\n")

# ---- B. dotted labels -------------------------------------------------------
S("b01_dot_colon", "`.dl:` in body under a caller `Base:`, read `.dl` after",
  "Base:\tdc.w\t$5555\nmac\tmacro\n.dl:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t.dl\t; REF\n")
S("b02_dot_qualified", "`.dl:` in body under `Base:`, read `Base.dl` after",
  "Base:\tdc.w\t$5555\nmac\tmacro\n.dl:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tBase.dl\t; REF\n")
S("b03_dot_no_scope", "`.dl:` in body with no global label anywhere, read `.dl` after",
  "mac\tmacro\n.dl:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t.dl\t; REF\n")
S("b04_dot_bare_col0", "colon-less column-0 `.dl` in body under `Base:`, read after",
  "Base:\tdc.w\t$5555\nmac\tmacro\n.dl\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t.dl\t; REF\n")
S("b05_dot_forward", "`.dl:` in body under `Base:`, read BEFORE the expansion",
  "Base:\tdc.w\t$5555\nmac\tmacro\n.dl:\tdc.w\t$2222\n\tendm\n\tdc.w\t.dl\t; REF\n\tmac\n")

# ---- C. nameless temporary labels ------------------------------------------
S("c01_minus_after", "column-1 `-` defined in body, `-` referenced after the expansion",
  "mac\tmacro\n-\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n")
S("c02_plus_before", "`+` referenced before the expansion, column-1 `+` defined in body",
  "mac\tmacro\n+\tdc.w\t$2222\n\tendm\n\tdc.w\t+\t; REF\n\tmac\n")
S("c03_plus_before_outer_after", "`+` before; body defines `+`; an OUTER `+` after (the mb.asm shape)",
  "mac\tmacro\n+\tdc.w\t$2222\n\tendm\n\tdc.w\t+\t; REF\n\tmac\n+\tdc.w\t$3333\n")
S("c04_slash_then_minus", "column-1 `/` in body, `-` referenced after",
  "mac\tmacro\n/\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n")
S("c05_plus_before_slash", "`+` referenced before; column-1 `/` in body",
  "mac\tmacro\n/\tdc.w\t$2222\n\tendm\n\tdc.w\t+\t; REF\n\tmac\n")
S("c06_inside_reads_outer_minus", "outer `-` before the call; the body references `-`",
  "-\tdc.w\t$3333\nmac\tmacro\n\tdc.w\t-\t; REF\n\tendm\n\tmac\n")
S("c07_inside_reads_outer_plus", "the body references `+`; outer `+` after the call",
  "mac\tmacro\n\tdc.w\t+\t; REF\n\tendm\n\tmac\n+\tdc.w\t$3333\n")
S("c08_minus_outer_before_body", "outer `-` before; body defines `-`; `-` referenced after",
  "-\tdc.w\t$3333\nmac\tmacro\n-\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n")
S("c09_own_minus_control", "body defines and references its own `-` (control)",
  "mac\tmacro\n-\tdc.w\t$2222\n\tdc.w\t-\t; REF\n\tendm\n\tmac\n\tmac\n")
S("c10_minus_rept_file", "column-1 `-` inside a file-level `rept 1` body, `-` after",
  "\trept\t1\n-\tdc.w\t$2222\n\tendm\n\tdc.w\t-\t; REF\n")
S("c11_minus_irp_file", "column-1 `-` inside a file-level `irp` body, `-` after",
  "\tirp\tv,5\n-\tdc.w\t$2222\n\tendm\n\tdc.w\t-\t; REF\n")
S("c12_minus_while_file", "column-1 `-` inside a file-level `while` body, `-` after",
  "cnt\tset\t0\n\twhile\tcnt<1\n-\tdc.w\t$2222\ncnt\tset\tcnt+1\n\tendm\n\tdc.w\t-\t; REF\n")
S("c13_minus_globalsymbols", "column-1 `-` in a {GLOBALSYMBOLS} body, `-` after",
  "mac\tmacro\t{GLOBALSYMBOLS}\n-\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n")
S("c14_plus_before_twice", "`+` before; the macro, which defines `+`, invoked twice",
  "mac\tmacro\n+\tdc.w\t$2222\n\tendm\n\tdc.w\t+\t; REF\n\tmac\n\tmac\n")
S("c15_minus_rept_in_macro", "column-1 `-` in a `rept` inside a macro body, `-` after the expansion",
  "mac\tmacro\n\trept\t1\n-\tdc.w\t$2222\n\tendm\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n")

# ---- D. loop bodies at file level -------------------------------------------
S("d01_rept1", "plain label in a file-level `rept 1` body, read after",
  "\trept\t1\nLr:\tdc.w\t$2222\n\tendm\n\tdc.w\tLr\t; REF\n")
S("d02_rept2", "plain label in a file-level `rept 2` body, read after",
  "\trept\t2\nLr:\tdc.w\t$2222\n\tendm\n\tdc.w\tLr\t; REF\n")
S("d03_irp", "plain label in a file-level `irp` body, read after",
  "\tirp\tv,5\nLi:\tdc.w\t$2222\n\tendm\n\tdc.w\tLi\t; REF\n")
S("d04_irpc", "plain label in a file-level `irpc` body, read after",
  "\tirpc\tc,a\nLc:\tdc.w\t$2222\n\tendm\n\tdc.w\tLc\t; REF\n")
S("d05_while", "plain label in a file-level `while` body, read after",
  "cnt\tset\t0\n\twhile\tcnt<1\nLw:\tdc.w\t$2222\ncnt\tset\tcnt+1\n\tendm\n\tdc.w\tLw\t; REF\n")
S("d06_irp_var_name", "`irp` label spelled by the loop variable, read after",
  "\tirp\tv,Aa\nv:\tdc.w\t$2222\n\tendm\n\tdc.w\tAa\t; REF\n")
S("d07_rept_in_macro_read_in_body", "label in a `rept` inside a macro body, read later in that body",
  "mac\tmacro\n\trept\t1\nLr:\tdc.w\t$2222\n\tendm\n\tdc.w\tLr\t; REF\n\tendm\n\tmac\n")
S("d08_rept_in_macro_read_outside", "label in a `rept` inside a macro body, read after the expansion",
  "mac\tmacro\n\trept\t1\nLr:\tdc.w\t$2222\n\tendm\n\tendm\n\tmac\n\tdc.w\tLr\t; REF\n")
S("d09_irpc_var_name", "`irpc` label spelled from the loop variable by interpolation, read after",
  "\tirpc\tc,q\nL{\"c\"}:\tdc.w\t$2222\n\tendm\n\tdc.w\tLq\t; REF\n")

# ---- E. nested macros -------------------------------------------------------
S("e01_inner_label_outside", "inner macro's label, read at file level after the outer returns",
  "inner\tmacro\nLn:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tendm\n\touter\n\tdc.w\tLn\t; REF\n")
S("e02_outer_label_outside", "outer body label (outer also calls inner), read after",
  "inner\tmacro\n\tdc.w\t$3333\n\tendm\nouter\tmacro\nLo:\tdc.w\t$2222\n\tinner\n\tendm\n\touter\n\tdc.w\tLo\t; REF\n")
S("e03_macro_defined_in_macro", "a macro DEFINED inside a macro body, invoked at file level, its label read after",
  "outer\tmacro\ninn\tmacro\nLd:\tdc.w\t$2222\n\tendm\n\tendm\n\touter\n\tinn\n\tdc.w\tLd\t; REF\n")

# ---- F. MACRO statement options ---------------------------------------------
for opt in ["GLOBALSYMBOLS", "NOGLOBALSYMBOLS", "EXPAND", "NOEXPAND", "EXPIF", "NOEXPIF",
            "EXPMACRO", "NOEXPMACRO", "EXPORT", "NOEXPORT", "INTLABEL"]:
    S(f"f_{opt.lower()}", f"plain colon label in a {{{opt}}} body, read after",
      f"mac\tmacro\t{{{opt}}}\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLp\t; REF\n")
S("f_globalsymbols_twice", "{GLOBALSYMBOLS} body label, macro invoked twice (collision?)",
  "mac\tmacro\t{GLOBALSYMBOLS}\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tmac\n\tdc.w\tLp\t; REF\n")
S("f_globalsymbols_dot", "{GLOBALSYMBOLS} body `.dl:` under `Base:`, read `.dl` after",
  "Base:\tdc.w\t$5555\nmac\tmacro\t{GLOBALSYMBOLS}\n.dl:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t.dl\t; REF\n")
S("f_globalsymbols_inside", "{GLOBALSYMBOLS} body label read inside the body (control)",
  "mac\tmacro\t{GLOBALSYMBOLS}\nLp:\tdc.w\t$2222\n\tdc.w\tLp\t; REF\n\tendm\n\tmac\n")
S("f_globalsymbols_forward", "{GLOBALSYMBOLS} body label read BEFORE the expansion",
  "mac\tmacro\t{GLOBALSYMBOLS}\nLp:\tdc.w\t$2222\n\tendm\n\tdc.w\tLp\t; REF\n\tmac\n")
S("f_globalsymbols_nested_inner", "plain inner macro's label, inner called from a {GLOBALSYMBOLS} outer, read after",
  "inner\tmacro\nLn:\tdc.w\t$2222\n\tendm\nouter\tmacro\t{GLOBALSYMBOLS}\n\tinner\n\tendm\n\touter\n\tdc.w\tLn\t; REF\n")
S("f_globalsymbols_intlabel", "{INTLABEL,GLOBALSYMBOLS}: `__LABEL___Blocks:` read after",
  "mac\tmacro\t{INTLABEL},{GLOBALSYMBOLS}\n__LABEL___Blocks:\tdc.w\t$2222\n\tendm\nAint:\tmac\n\tdc.w\tAint_Blocks\t; REF\n")
S("f_globalsymbols_lower", "{globalsymbols} lower case under -U, read after",
  "mac\tmacro\t{globalsymbols}\nLp:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLp\t; REF\n")

# ---- G. composed names ------------------------------------------------------
S("g01_intlabel_suffix", "{INTLABEL} body `__LABEL___Blocks:`, read `Aint_Blocks` after",
  "mac\tmacro\t{INTLABEL}\n__LABEL___Blocks:\tdc.w\t$2222\n\tendm\nAint:\tmac\n\tdc.w\tAint_Blocks\t; REF\n")
S("g02_intlabel_bare", "{INTLABEL} body `__LABEL__:`, read `Aint` after",
  "mac\tmacro\t{INTLABEL}\n__LABEL__:\tdc.w\t$2222\n\tendm\nAint:\tmac\n\tdc.w\tAint\t; REF\n")
S("g03_intlabel_literal", "{INTLABEL} body `__LABEL__Plc:` (not substituted), read `__LABEL__Plc` after",
  "mac\tmacro\t{INTLABEL}\n__LABEL__Plc:\tdc.w\t$2222\n\tendm\nAint:\tmac\n\tdc.w\t__LABEL__Plc\t; REF\n")
S("g04_brace_global", "body `Lab{cnt}:` with a file-level `cnt`, read `Lab7` after",
  "cnt\t=\t7\nmac\tmacro\nLab{cnt}:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tLab7\t; REF\n")
S("g05_brace_param", "body `Lab{n}:` from parameter `n`, read `Lab3` after",
  "mac\tmacro\tn\nLab{n}:\tdc.w\t$2222\n\tendm\n\tmac\t3\n\tdc.w\tLab3\t; REF\n")
S("g06_param_name_colon", "label name arrives as argument text (`nm:`), read `Foo` after",
  "mac\tmacro\tnm\nnm:\tdc.w\t$2222\n\tendm\n\tmac\tFoo\n\tdc.w\tFoo\t; REF\n")
S("g07_param_name_bare", "label name arrives as argument text (column-0 `nm`), read `Foo` after",
  "mac\tmacro\tnm\nnm\tdc.w\t$2222\n\tendm\n\tmac\tFoo\n\tdc.w\tFoo\t; REF\n")
S("g08_param_name_dot", "dotted name arrives as argument text under `Base:`, read `.foo` after",
  "Base:\tdc.w\t$5555\nmac\tmacro\tnm\nnm:\tdc.w\t$2222\n\tendm\n\tmac\t.foo\n\tdc.w\t.foo\t; REF\n")
S("g09_param_name_twice", "argument-text label, macro invoked twice with the SAME name",
  "mac\tmacro\tnm\nnm:\tdc.w\t$2222\n\tendm\n\tmac\tFoo\n\tmac\tFoo\n\tdc.w\t$3333\n")
S("g10_brace_twice", "body `Lab{n}:` invoked twice with the same n",
  "mac\tmacro\tn\nLab{n}:\tdc.w\t$2222\n\tendm\n\tmac\t3\n\tmac\t3\n\tdc.w\t$3333\n")
S("g11_intlabel_suffix_twice", "{INTLABEL} `__LABEL___Blocks:` invoked twice under the SAME label? (control on collision)",
  "mac\tmacro\t{INTLABEL}\n__LABEL___Blocks:\tdc.w\t$2222\n\tendm\nAint:\tmac\nBint:\tmac\n\tdc.w\t$3333\n")
S("g12_param_name_inside", "argument-text label read INSIDE the body (control)",
  "mac\tmacro\tnm\nnm:\tdc.w\t$2222\n\tdc.w\tnm\t; REF\n\tendm\n\tmac\tFoo\n\tmac\tBar\n")
S("g13_brace_inside", "body `Lab{n}:` read inside the body as `Lab{n}` (control)",
  "mac\tmacro\tn\nLab{n}:\tdc.w\t$2222\n\tdc.w\tLab{n}\t; REF\n\tendm\n\tmac\t3\n\tmac\t4\n")
S("g14_allargs_name", "label name via ALLARGS (`ALLARGS:`), read after",
  "mac\tmacro\nALLARGS:\tdc.w\t$2222\n\tendm\n\tmac\tFoo\n\tdc.w\tFoo\t; REF\n")

# ---- H. value-binding and declaration forms (enum is the known open one) ----
S("h01_enum", "`enum` members declared in a body, read after",
  "mac\tmacro\n\tenum\tEa=5,Eb\n\tendm\n\tmac\n\tdc.w\tEb\t; REF\n")
S("h02_equ_control", "`equ` in body, read after (control: global)",
  "mac\tmacro\nEq\tequ\t$123\n\tendm\n\tmac\n\tdc.w\tEq\t; REF\n")
S("h03_label_dir_control", "`label *` in body, read after (control: global)",
  "mac\tmacro\nXl\tlabel\t*\n\tendm\n\tmac\n\tdc.w\tXl\t; REF\n")
S("h04_nextenum", "`nextenum` member declared in a body after a file-level `enum`, read after",
  "\tenum\tEa=5\nmac\tmacro\n\tnextenum\tEb\n\tendm\n\tmac\n\tdc.w\tEb\t; REF\n")

# ---- I. include inside a macro body -----------------------------------------
S("i01_include_in_body", "a label in a file INCLUDED from a macro body, read after",
  "mac\tmacro\n\tinclude\t\"i01_inc.inc\"\n\tendm\n\tmac\n\tdc.w\tLinc\t; REF\n",
  {"i01_inc.inc": "Linc:\tdc.w\t$2222\n"})
S("i02_include_in_body_twice", "the included label, macro invoked twice (collision?)",
  "mac\tmacro\n\tinclude\t\"i02_inc.inc\"\n\tendm\n\tmac\n\tmac\n\tdc.w\t$3333\n",
  {"i02_inc.inc": "Linc:\tdc.w\t$2222\n"})
S("i03_include_nameless", "a column-1 `-` in a file included from a macro body, `-` after",
  "mac\tmacro\n\tinclude\t\"i03_inc.inc\"\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n",
  {"i03_inc.inc": "-\tdc.w\t$2222\n"})

for name, (desc, text, extra) in shapes.items():
    with open(os.path.join(OUT, name + ".asm"), "w") as f:
        f.write("; " + desc + "\n" + text)
    for ename, etext in extra.items():
        with open(os.path.join(OUT, ename), "w") as f:
            f.write(etext)
with open(os.path.join(OUT, "INDEX.tsv"), "w") as f:
    for name, (desc, _, _) in shapes.items():
        f.write(f"{name}\t{desc}\n")
print(len(shapes), "shapes")
