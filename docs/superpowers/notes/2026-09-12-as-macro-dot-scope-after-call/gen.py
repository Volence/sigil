#!/usr/bin/env python3
"""Generate the AS-MACRO-DOT-SCOPE-AFTER-CALL shape matrix.

The question: does a plain label written in a macro body change the `.`-local
scope the CALLER sees after the expansion, and in every spelling, nesting and
loop form that can write one?

One shape per file and one suspect line per file, marked `; REF`, because an
error anywhere in an asl run poisons every value in it. Everything sits at
`org $100` behind a `$1111` filler word, so a bound label reads as a non-zero
address and a zero cannot pass for an answer. Each value the REF line can
reach is a distinct word ($5555 Base, $6666 Base.x, $7777 file Inner,
$8888 Inner.x, ...), so the bytes say WHICH candidate a reference bound to.

    gen.py <out-dir>
"""
import os, sys

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

HEAD = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
TAIL = "\tdc.w\t$4444\n"

shapes = {}


def S(name, desc, body):
    assert name not in shapes, name
    shapes[name] = (desc, HEAD + body + TAIL)


MAC = "mac\tmacro\nInner:\tdc.w\t$2222\n\tendm\n"
BASE = "Base:\tdc.w\t$5555\n"
BASE_X = "Base:\tdc.w\t$5555\n.x:\tdc.w\t$6666\n"
FILE_INNER_X = "Inner:\tdc.w\t$7777\n.x:\tdc.w\t$8888\n"

# ---- A. the scope after the call, one body label `Inner:` -------------------
S("a01_bind_read_inner", "body `Inner:`; after the call `.b := 2`; read `Inner.b`",
  BASE + MAC + "\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("a02_bind_read_base", "body `Inner:`; after the call `.b := 2`; read `Base.b`",
  BASE + MAC + "\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("a03_bind_read_dot", "body `Inner:`; after the call `.b := 2`; read `.b` (control: same scope both sides)",
  BASE + MAC + "\tmac\n.b\t:=\t2\n\tdc.w\t.b\t; REF\n")
S("a04_label_read_inner", "body `Inner:`; after the call `.y:`; read `Inner.y`",
  BASE + MAC + "\tmac\n.y:\tdc.w\t$6666\n\tdc.w\tInner.y\t; REF\n")
S("a05_label_read_base", "body `Inner:`; after the call `.y:`; read `Base.y`",
  BASE + MAC + "\tmac\n.y:\tdc.w\t$6666\n\tdc.w\tBase.y\t; REF\n")
S("a06_inner_itself", "body `Inner:`; read `Inner` after the call (control: body label stays in the body)",
  BASE + MAC + "\tmac\n\tdc.w\tInner\t; REF\n")
S("a07_twice", "macro called twice; after `.b := 2` read `Inner.b`",
  BASE + MAC + "\tmac\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("a08_no_base", "no caller label at all; after `.b := 2` read `Inner.b`",
  MAC + "\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")

# ---- S. the silent family: a `.x` whose meaning crosses the call ------------
S("s01_dot_before_read_after", "`.x:` under Base before the call; read `.x` after",
  BASE_X + MAC + "\tmac\n\tdc.w\t.x\t; REF\n")
S("s02_dot_before_read_after_file_inner_before",
  "file-level `Inner:`/`.x:` first; `.x:` under Base; call; read `.x` after",
  FILE_INNER_X + BASE_X + MAC + "\tmac\n\tdc.w\t.x\t; REF\n")
S("s03_dot_before_read_after_file_inner_after",
  "`.x:` under Base; call; read `.x` after; file-level `Inner:`/`.x:` last",
  BASE_X + MAC + "\tmac\n\tdc.w\t.x\t; REF\n\tdc.w\t$4444\n" + FILE_INNER_X)
S("s04_dot_before_read_base_qualified", "`.x:` under Base before the call; read `Base.x` after",
  BASE_X + MAC + "\tmac\n\tdc.w\tBase.x\t; REF\n")
S("s05_set_before_read_after", "`.a := 3` under Base before the call; read `.a` after",
  BASE + ".a\t:=\t3\n" + MAC + "\tmac\n\tdc.w\t.a\t; REF\n")
S("s06_set_before_read_after_file_inner",
  "file `Inner:` with `.a := 9`; Base `.a := 3`; call; read `.a` after",
  "Inner:\tdc.w\t$7777\n.a\t:=\t9\n" + BASE + ".a\t:=\t3\n" + MAC + "\tmac\n\tdc.w\t.a\t; REF\n")
S("s07_defined_after", "`.x:` under Base; call; `if defined(.x)` after picks a word",
  BASE_X + MAC + "\tmac\n\tif\tdefined(.x)\t; REF\n\tdc.w\t$AAAA\n\telse\n\tdc.w\t$BBBB\n\tendif\n")
S("s08_ifdef_after", "`.x:` under Base; call; `ifdef .x` after picks a word",
  BASE_X + MAC + "\tmac\n\tifdef\t.x\t; REF\n\tdc.w\t$AAAA\n\telse\n\tdc.w\t$BBBB\n\tendif\n")
S("s09_forward_ref_before_call", "`bra.s .x` before the call; `.x:` defined after it",
  BASE + MAC + "\tbra.s\t.x\t; REF\n\tmac\n.x:\tdc.w\t$6666\n")
S("s10_forward_ref_before_call_base_x_later",
  "`bra.s .x` before the call; `.x:` after it; then a second `Base2:`/`.x:` control",
  BASE + MAC + "\tbra.s\t.x\t; REF\n\tmac\n.x:\tdc.w\t$6666\nBase2:\tdc.w\t$9999\n.x:\tdc.w\t$9998\n")
S("s11_redefine_after_call_file_inner",
  "file `Inner:`/`.x:` first; Base; call; `.x:` again after the call (double definition?)",
  FILE_INNER_X + BASE + MAC + "\tmac\n.x:\tdc.w\t$6666\t; REF\n")
S("s12_redefine_after_call_base_x",
  "`.x:` under Base; call; `.x:` again after the call",
  BASE_X + MAC + "\tmac\n.x:\tdc.w\t$6667\t; REF\n")
S("s13_two_callers_same_local",
  "two routines, each calls mac then defines and reads `.lp` (collide as Inner.lp?)",
  "R1:\tdc.w\t$5555\n\tmac\n.lp:\tdc.w\t$6666\n\tdc.w\t.lp\nR2:\tdc.w\t$5556\n\tmac\n.lp:\tdc.w\t$6667\n\tdc.w\t.lp\t; REF\n"
  .replace("R1:", MAC + "R1:"))

# ---- C. controls: bodies that write no plain label ---------------------------
S("c01_no_label_read_base", "body with no label; after `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\n\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("c02_no_label_read_dot_before", "body with no label; `.x:` under Base before; read `.x` after",
  BASE_X + "mac\tmacro\n\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t.x\t; REF\n")
S("c03_dot_label_body_read_base", "body writes only `.q:`; after `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\n.q:\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("c04_label_directive_read_inner", "body `Inner label *`; after `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\nInner\tlabel\t*\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("c05_set_body_read_vv", "body `Vv set 5`; after `.b := 2` read `Vv.b`",
  BASE + "mac\tmacro\nVv\tset\t5\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tVv.b\t; REF\n")
S("c06_untaken_if_read_base", "body `if 0` / `Inner:` / `endif`; after `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\n\tif\t0\nInner:\tdc.w\t$2222\n\tendif\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("c07_exitm_before_label_read_base", "body `exitm` before `Inner:`; after `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\n\tdc.w\t$2222\n\texitm\nInner:\tdc.w\t$2223\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("c08_taken_if_read_inner", "body `if 1` / `Inner:` / `endif`; after `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\n\tif\t1\nInner:\tdc.w\t$2222\n\tendif\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")

# ---- T. two plain labels in one body ------------------------------------------
MAC2 = "mac\tmacro\nIn1:\tdc.w\t$2222\nIn2:\tdc.w\t$2223\n\tendm\n"
S("t01_two_read_second", "body `In1:` `In2:`; after `.b := 2` read `In2.b`",
  BASE + MAC2 + "\tmac\n.b\t:=\t2\n\tdc.w\tIn2.b\t; REF\n")
S("t02_two_read_first", "body `In1:` `In2:`; after `.b := 2` read `In1.b`",
  BASE + MAC2 + "\tmac\n.b\t:=\t2\n\tdc.w\tIn1.b\t; REF\n")
S("t03_two_read_base", "body `In1:` `In2:`; after `.b := 2` read `Base.b`",
  BASE + MAC2 + "\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")

# ---- N. nested expansions -----------------------------------------------------
INNER = "inner\tmacro\nInner:\tdc.w\t$2222\n\tendm\n"
S("n01_inner_writes_read_inner", "outer calls inner, inner writes `Inner:`; after outer `.b := 2` read `Inner.b`",
  BASE + INNER + "outer\tmacro\n\tinner\n\tendm\n\touter\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("n02_inner_writes_read_base", "outer calls inner, inner writes `Inner:`; after outer read `Base.b`",
  BASE + INNER + "outer\tmacro\n\tinner\n\tendm\n\touter\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("n03_outer_writes_then_inner_plain", "outer writes `Outer:` then calls a label-less inner; read `Outer.b`",
  BASE + "inner\tmacro\n\tdc.w\t$2222\n\tendm\nouter\tmacro\nOuter:\tdc.w\t$3333\n\tinner\n\tendm\n"
  "\touter\n.b\t:=\t2\n\tdc.w\tOuter.b\t; REF\n")
S("n04_inner_then_outer_label_read_outer2", "outer calls inner (writes `Inner:`) then writes `Outer2:`; read `Outer2.b`",
  BASE + INNER + "outer\tmacro\n\tinner\nOuter2:\tdc.w\t$3333\n\tendm\n\touter\n.b\t:=\t2\n\tdc.w\tOuter2.b\t; REF\n")
S("n05_inner_then_outer_label_read_inner", "outer calls inner (writes `Inner:`) then writes `Outer2:`; read `Inner.b`",
  BASE + INNER + "outer\tmacro\n\tinner\nOuter2:\tdc.w\t$3333\n\tendm\n\touter\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("n06_outer_binds_after_inner_read_inner", "outer calls inner, then `.v := 3` in the outer body; read `Inner.v` after",
  BASE + INNER + "outer\tmacro\n\tinner\n.v\t:=\t3\n\tendm\n\touter\n\tdc.w\tInner.v\t; REF\n")
S("n07_outer_binds_after_inner_read_base", "outer calls inner, then `.v := 3` in the outer body; read `Base.v` after",
  BASE + INNER + "outer\tmacro\n\tinner\n.v\t:=\t3\n\tendm\n\touter\n\tdc.w\tBase.v\t; REF\n")
S("n08_outer_label_after_inner_read_in_body", "outer calls inner, then `.y:` in the outer body read as `.y` there",
  BASE + INNER + "outer\tmacro\n\tinner\n.y:\tdc.w\t$3333\n\tdc.w\t.y\t; REF\n\tendm\n\touter\n")
S("n09_outer_label_after_inner_read_inner_y_outside", "outer calls inner, then `.y:` in the outer body; read `Inner.y` after outer",
  BASE + INNER + "outer\tmacro\n\tinner\n.y:\tdc.w\t$3333\n\tendm\n\touter\n\tdc.w\tInner.y\t; REF\n")
S("n10_outer_reads_caller_dot_after_inner", "`.x:` under Base; outer calls inner, then reads `.x` in its body",
  BASE_X + INNER + "outer\tmacro\n\tinner\n\tdc.w\t.x\t; REF\n\tendm\n\touter\n")
S("n11_outer_label_then_inner_then_dot_read_outer_y",
  "outer writes `Outer:`, calls inner, then `.y:`; the outer body reads `Outer.y`",
  BASE + INNER + "outer\tmacro\nOuter:\tdc.w\t$3333\n\tinner\n.y:\tdc.w\t$3334\n\tdc.w\tOuter.y\t; REF\n\tendm\n\touter\n")
S("n12_outer_label_then_inner_then_dot_read_inner_y",
  "outer writes `Outer:`, calls inner, then `.y:`; the outer body reads `Inner.y`",
  BASE + INNER + "outer\tmacro\nOuter:\tdc.w\t$3333\n\tinner\n.y:\tdc.w\t$3334\n\tdc.w\tInner.y\t; REF\n\tendm\n\touter\n")
S("n14_outer_dot_before_inner_read_after", "outer body `.lp:`, calls inner (writes `Inner:`), then reads `.lp` in its body",
  BASE + INNER + "outer\tmacro\n.lp:\tdc.w\t$3333\n\tinner\n\tdc.w\t.lp\t; REF\n\tendm\n\touter\n")
S("n15_outer_dot_before_inner_read_after_file_inner",
  "file `Inner:`/`.lp:`; outer body `.lp:`, calls inner (writes `Inner:`), then reads `.lp` in its body",
  "Inner:\tdc.w\t$7777\n.lp:\tdc.w\t$8888\n" + BASE + INNER
  + "outer\tmacro\n.lp:\tdc.w\t$3333\n\tinner\n\tdc.w\t.lp\t; REF\n\tendm\n\touter\n")
S("n16_inner_label_directive_outer_dot_read_qualified",
  "inner writes `Lx label *`; outer body then `.y:` and reads `Lx.y` in its body",
  BASE + "inner\tmacro\nLx\tlabel\t*\n\tendm\n"
  "outer\tmacro\n\tinner\n.y:\tdc.w\t$3333\n\tdc.w\tLx.y\t; REF\n\tendm\n\touter\n")
S("n17_outer_forward_dot_after_inner", "outer calls inner (writes `Inner:`), then `bra.s .y` forward to its own `.y:`",
  BASE + INNER + "outer\tmacro\n\tinner\n\tbra.s\t.y\t; REF\n\tdc.w\t$3333\n.y:\tdc.w\t$3334\n\tendm\n\touter\n")
S("n18_outer_dot_after_inner_twice", "outer (calls inner, then `.y:`) expanded twice; the second reads `.y` (no collision)",
  BASE + INNER + "outer\tmacro\n\tinner\n.y:\tdc.w\t$3333\n\tdc.w\t.y\n\tendm\n\touter\n\touter\n\tdc.w\t$3335\t; REF\n")
S("n19_outer_defined_own_dot_after_inner",
  "outer calls inner (writes `Inner:`), then `.y:`, then `if defined(.y)` in its body picks a word",
  BASE + INNER + "outer\tmacro\n\tinner\n.y:\tdc.w\t$3333\n\tif\tdefined(.y)\t; REF\n\tdc.w\t$AAAA\n\telse\n"
  "\tdc.w\t$BBBB\n\tendif\n\tendm\n\touter\n")
S("n20_outer_ifdef_own_dot_after_inner",
  "outer calls inner (writes `Inner:`), then `.y:`, then `ifdef .y` in its body picks a word",
  BASE + INNER + "outer\tmacro\n\tinner\n.y:\tdc.w\t$3333\n\tifdef\t.y\t; REF\n\tdc.w\t$AAAA\n\telse\n"
  "\tdc.w\t$BBBB\n\tendif\n\tendm\n\touter\n")
S("n13_three_deep","a calls b calls c, c writes `Deep:`; after a `.b := 2` read `Deep.b`",
  BASE + "mc\tmacro\nDeep:\tdc.w\t$2222\n\tendm\nmb\tmacro\n\tmc\n\tendm\nma\tmacro\n\tmb\n\tendm\n"
  "\tma\n.b\t:=\t2\n\tdc.w\tDeep.b\t; REF\n")

# ---- R. a body label inside a loop --------------------------------------------
S("r01_file_rept_read_inner", "file-level `rept 1` / `Inner:`; after `.b := 2` read `Inner.b`",
  BASE + "\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("r02_file_rept_read_base", "file-level `rept 1` / `Inner:`; after `.b := 2` read `Base.b`",
  BASE + "\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("r03_macro_rept_read_inner", "body `rept 1` / `Inner:`; after the call `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\n\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("r04_macro_rept_read_base", "body `rept 1` / `Inner:`; after the call `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\n\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("r05_file_irp_read_inner", "file-level `irp` / `Inner:`; after `.b := 2` read `Inner.b`",
  BASE + "\tirp\tv,1\nInner:\tdc.w\t$2222\n\tendm\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("r06_macro_irp_read_inner", "body `irp` / `Inner:`; after the call `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\n\tirp\tv,1\nInner:\tdc.w\t$2222\n\tendm\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("r07_macro_irp_read_base", "body `irp` / `Inner:`; after the call `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\n\tirp\tv,1\nInner:\tdc.w\t$2222\n\tendm\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("r08_macro_irp_var_label_read_last", "body `irp v,Aa,Bb` / `v:`; after the call `.b := 2` read `Bb.b`",
  BASE + "mac\tmacro\n\tirp\tv,Aa,Bb\nv:\tdc.w\t$2222\n\tendm\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBb.b\t; REF\n")
S("r09_macro_rept2_read_inner", "body `rept 2` / `Inner:`; after the call `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\n\trept\t2\nInner:\tdc.w\t$2222\n\tendr\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("r11_macro_rept_label_then_dot_read_outside",
  "body `rept 1` / `Inner:` / `endr`, then `.y:`; read `Inner.y` after the call",
  BASE + "mac\tmacro\n\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n.y:\tdc.w\t$2223\n\tendm\n\tmac\n\tdc.w\tInner.y\t; REF\n")
S("r12_macro_rept_label_then_dot_read_in_body",
  "body `rept 1` / `Inner:` / `endr`, then `.y:`; the body reads `.y`",
  BASE + "mac\tmacro\n\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n.y:\tdc.w\t$2223\n\tdc.w\t.y\t; REF\n\tendm\n\tmac\n")
S("r10_file_rept_dot_before_read_after","`.x:` under Base; file-level `rept 1` / `Inner:`; read `.x` after",
  BASE_X + "\trept\t1\nInner:\tdc.w\t$2222\n\tendr\n\tdc.w\t.x\t; REF\n")

# ---- L. spellings of the body label -------------------------------------------
for tag, spelled in [("colon_alone", "Inner:\n\tdc.w\t$2222\n"),
                     ("bare_alone", "Inner\n\tdc.w\t$2222\n"),
                     ("bare_data", "Inner\tdc.w\t$2222\n"),
                     ("indented_colon", "\tInner:\tdc.w\t$2222\n")]:
    for read in ["Inner", "Base"]:
        S(f"l_{tag}_read_{read.lower()}",
          f"body label spelled {tag}; after the call `.b := 2` read `{read}.b`",
          BASE + "mac\tmacro\n" + spelled + "\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\t" + read + ".b\t; REF\n")

# ---- G. the option twins --------------------------------------------------------
S("g01_globalsymbols_read_inner", "`{GLOBALSYMBOLS}` body `Inner:`; after `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\t{GLOBALSYMBOLS}\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("g02_globalsymbols_read_base", "`{GLOBALSYMBOLS}` body `Inner:`; after `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\t{GLOBALSYMBOLS}\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("g03_intlabel_label_read_tbl", "`{INTLABEL}` body `__LABEL__:`, called `Tbl mac`; after `.b := 2` read `Tbl.b`",
  BASE + "mac\tmacro\t{INTLABEL}\n__LABEL__:\tdc.w\t$2222\n\tendm\nTbl\tmac\n.b\t:=\t2\n\tdc.w\tTbl.b\t; REF\n")
S("g04_intlabel_label_read_base", "`{INTLABEL}` body `__LABEL__:`, called `Tbl mac`; after `.b := 2` read `Base.b`",
  BASE + "mac\tmacro\t{INTLABEL}\n__LABEL__:\tdc.w\t$2222\n\tendm\nTbl\tmac\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("g05_intlabel_inner_read_inner", "`{INTLABEL}` body `Inner:`, called `Tbl mac`; after `.b := 2` read `Inner.b`",
  BASE + "mac\tmacro\t{INTLABEL}\nInner:\tdc.w\t$2222\n\tendm\nTbl\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("g06_intlabel_inner_read_tbl", "`{INTLABEL}` body `Inner:`, called `Tbl mac`; after `.b := 2` read `Tbl.b`",
  BASE + "mac\tmacro\t{INTLABEL}\nInner:\tdc.w\t$2222\n\tendm\nTbl\tmac\n.b\t:=\t2\n\tdc.w\tTbl.b\t; REF\n")
S("g07_call_label_body_inner_read_inner", "plain macro called `Lbl mac`, body `Inner:`; after `.b := 2` read `Inner.b`",
  BASE + MAC + "Lbl\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("g08_call_label_body_inner_read_lbl", "plain macro called `Lbl mac`, body `Inner:`; after `.b := 2` read `Lbl.b`",
  BASE + MAC + "Lbl\tmac\n.b\t:=\t2\n\tdc.w\tLbl.b\t; REF\n")
S("g09_call_label_no_body_label_read_lbl", "plain macro called `Lbl mac`, no body label; after `.b := 2` read `Lbl.b` (control)",
  BASE + "mac\tmacro\n\tdc.w\t$2222\n\tendm\nLbl\tmac\n.b\t:=\t2\n\tdc.w\tLbl.b\t; REF\n")
S("g10_plain_outer_gs_inner_read_inner", "plain outer calls `{GLOBALSYMBOLS}` inner writing `Inner:`; read `Inner.b` after outer",
  BASE + "inner\tmacro\t{GLOBALSYMBOLS}\nInner:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tendm\n"
  "\touter\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("g11_gs_outer_plain_inner_read_inner", "`{GLOBALSYMBOLS}` outer calls plain inner writing `Inner:`; read `Inner.b` after outer",
  BASE + INNER + "outer\tmacro\t{GLOBALSYMBOLS}\n\tinner\n\tendm\n"
  "\touter\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("g12_plain_outer_gs_inner_read_base", "plain outer calls `{GLOBALSYMBOLS}` inner writing `Inner:`; read `Base.b` after outer",
  BASE + "inner\tmacro\t{GLOBALSYMBOLS}\nInner:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tendm\n"
  "\touter\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")
S("g13_gs_outer_plain_inner_read_base", "`{GLOBALSYMBOLS}` outer calls plain inner writing `Inner:`; read `Base.b` after outer",
  BASE + INNER + "outer\tmacro\t{GLOBALSYMBOLS}\n\tinner\n\tendm\n"
  "\touter\n.b\t:=\t2\n\tdc.w\tBase.b\t; REF\n")

# ---- V. inside the body, after its plain label -----------------------------------
S("v01_body_bind_read_inner", "body `Inner:` then `.v := 5`; read `Inner.v` after the call",
  BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n.v\t:=\t5\n\tendm\n\tmac\n\tdc.w\tInner.v\t; REF\n")
S("v02_body_bind_read_base", "body `Inner:` then `.v := 5`; read `Base.v` after the call",
  BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n.v\t:=\t5\n\tendm\n\tmac\n\tdc.w\tBase.v\t; REF\n")
S("v03_body_reads_caller_dot", "`.x:` under Base; body `Inner:` then reads `.x`",
  BASE_X + "mac\tmacro\nInner:\tdc.w\t$2222\n\tdc.w\t.x\t; REF\n\tendm\n\tmac\n")
S("v04_body_reads_caller_dot_file_inner", "file `Inner:`/`.x:`; `.x:` under Base; body `Inner:` then reads `.x`",
  FILE_INNER_X + BASE_X + "mac\tmacro\nInner:\tdc.w\t$2222\n\tdc.w\t.x\t; REF\n\tendm\n\tmac\n")
S("v05_body_bind_before_label_read_base", "body `.v := 5` then `Inner:`; read `Base.v` after the call",
  BASE + "mac\tmacro\n.v\t:=\t5\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tBase.v\t; REF\n")
S("v06_body_reads_caller_dot_before_label", "`.x:` under Base; body reads `.x` then writes `Inner:`",
  BASE_X + "mac\tmacro\n\tdc.w\t.x\t; REF\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n")
S("v08_body_dot_before_label_read_after", "body `.lp:`, then `Inner:`, then reads `.lp`",
  BASE + "mac\tmacro\n.lp:\tdc.w\t$2222\nInner:\tdc.w\t$2223\n\tdc.w\t.lp\t; REF\n\tendm\n\tmac\n")
S("v09_body_bind_after_label_read_in_body", "body `Inner:`, `.v := 5`, then reads `.v` in the body",
  BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n.v\t:=\t5\n\tdc.w\t.v\t; REF\n\tendm\n\tmac\n")
S("v10_body_bind_after_label_read_qualified_in_body", "body `Inner:`, `.v := 5`, then reads `Inner.v` in the body",
  BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n.v\t:=\t5\n\tdc.w\tInner.v\t; REF\n\tendm\n\tmac\n")
S("v11_outer_label_dot_inner_reads_dot", "outer body `Lp:` and `.x:`, then calls inner, which reads `.x`",
  BASE + "inner\tmacro\n\tdc.w\t.x\t; REF\n\tendm\n"
  "outer\tmacro\nLp:\tdc.w\t$3333\n.x:\tdc.w\t$3334\n\tinner\n\tendm\n\touter\n")
S("v12_outer_label_inner_reads_dot_forward", "outer body `Lp:`, calls inner, which reads `.x`; outer defines `.x:` after",
  BASE + "inner\tmacro\n\tdc.w\t.x\t; REF\n\tendm\n"
  "outer\tmacro\nLp:\tdc.w\t$3333\n\tinner\n.x:\tdc.w\t$3334\n\tendm\n\touter\n")
S("v13_body_label_reads_dot_file_inner_forward", "body `Inner:` reads `.x`; the file-level `Inner:`/`.x:` comes after the call",
  BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n\tdc.w\t.x\t; REF\n\tendm\n\tmac\n" + FILE_INNER_X)
S("v14_body_label_bind_dot_file_inner", "file `Inner:`/`.x:`; body `Inner:`, `.v := 5`, then reads `.x`",
  FILE_INNER_X + BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n.v\t:=\t5\n\tdc.w\t.x\t; REF\n\tendm\n\tmac\n")
S("v07_body_own_dot_after_label","body `Inner:` then `.q:` and `dc.w .q` (control)",
  BASE + "mac\tmacro\nInner:\tdc.w\t$2222\n.q:\tdc.w\t$2223\n\tdc.w\t.q\t; REF\n\tendm\n\tmac\n")

# ---- F. multi-pass --------------------------------------------------------------
S("f01_forced_passes_read_inner", "a11 shape with an unrelated forward reference forcing more passes; read `Inner.b`",
  "\tdc.w\tLater\n" + BASE + MAC + "\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\nLater:\tdc.w\t$9999\n")
S("f02_forced_passes_dot_forward", "`.x` read after the call, defined later in the same scope; forced passes",
  "\tdc.w\tLater\n" + BASE + MAC + "\tmac\n\tdc.w\t.x\t; REF\n.x:\tdc.w\t$6666\nLater:\tdc.w\t$9999\n")

for name, (desc, text) in shapes.items():
    with open(os.path.join(OUT, name + ".asm"), "w") as f:
        f.write("; " + desc + "\n" + text)
with open(os.path.join(OUT, "INDEX.tsv"), "w") as f:
    for name, (desc, _) in shapes.items():
        f.write(f"{name}\t{desc}\n")
print(len(shapes), "shapes")
