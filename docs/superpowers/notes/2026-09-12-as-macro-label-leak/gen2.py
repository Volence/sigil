#!/usr/bin/env python3
"""Second batch: the edges a fix has to reproduce, not just the leaks."""
import os, sys

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
HEAD = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
TAIL = "\tdc.w\t$4444\n"
shapes = {}

def S(name, desc, body, extra=None):
    shapes[name] = (desc, HEAD + body + TAIL, extra or {})

# nameless, the visibility edges
S("n01_fwd_inside_own", "body `dc.w +` then its own `+`, invoked twice",
  "mac\tmacro\n\tdc.w\t+\t; REF\n+\tdc.w\t$2222\n\tendm\n\tmac\n\tmac\n")
S("n02_outer_fwd_to_inner_def", "outer body `dc.w +` then calls inner, which defines `+`",
  "inner\tmacro\n+\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tdc.w\t+\t; REF\n\tinner\n\tendm\n\touter\n+\tdc.w\t$3333\n")
S("n03_outer_back_to_closed_inner", "outer calls inner (defines `-`), then outer references `-`",
  "inner\tmacro\n-\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tdc.w\t-\t; REF\n\tendm\n\touter\n")
S("n04_inner_back_to_outer_def", "outer defines `-` then calls inner, which references `-`",
  "inner\tmacro\n\tdc.w\t-\t; REF\n\tendm\nouter\tmacro\n-\tdc.w\t$2222\n\tinner\n\tendm\n\touter\n\touter\n")
S("n05_inner_fwd_to_outer_def", "inner references `+`; outer calls inner then defines `+`",
  "inner\tmacro\n\tdc.w\t+\t; REF\n\tendm\nouter\tmacro\n\tinner\n+\tdc.w\t$2222\n\tendm\n\touter\n\touter\n")
S("n06_rept_fwd_in_iter", "`rept 2` body `dc.w +` then its own `+`",
  "\trept\t2\n\tdc.w\t+\t; REF\n+\tdc.w\t$2222\n\tendm\n")
S("n07_rept_back_in_iter", "`rept 2` body `-` then `dc.w -`",
  "\trept\t2\n-\tdc.w\t$2222\n\tdc.w\t-\t; REF\n\tendm\n")
S("n08_rept_reads_outer", "outer `-` before; `rept 2` body references `-`",
  "-\tdc.w\t$3333\n\trept\t2\n\tdc.w\t-\t; REF\n\tendm\n")
S("n09_label_on_rept_line", "the corpus shape: `-` on the `rept` line, `dbf d0,-` after",
  "\tmoveq\t#1,d0\n-\trept\t2\n\tnop\n\tendm\n\tdbf\td0,-\t; REF\n")
S("n10_other_macro_later", "`-` defined in one macro's body, referenced from a later different macro",
  "mdef\tmacro\n-\tdc.w\t$2222\n\tendm\nmref\tmacro\n\tdc.w\t-\t; REF\n\tendm\n\tmdef\n\tmref\n")
S("n11_ref_in_arg", "file-level `-`, then a macro whose body emits its argument, called with `-`",
  "-\tdc.w\t$3333\nmac\tmacro\tv\n\tdc.w\tv\t; REF\n\tendm\n\tmac\t-\n")
S("n12_ref_in_arg_body_def", "body defines `-` then emits its argument; called with `-` (whose `-`?)",
  "-\tdc.w\t$3333\nmac\tmacro\tv\n-\tdc.w\t$2222\n\tdc.w\tv\t; REF\n\tendm\n\tmac\t-\n")
S("n13_inner_fwd_past_closed_nested", "outer: `dc.w +`, calls inner (defines `+`), then defines its own `+`",
  "inner\tmacro\n+\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tdc.w\t+\t; REF\n\tinner\n+\tdc.w\t$3333\n\tendm\n\touter\n")
S("n14_while_fwd_in_iter", "`while` body `dc.w +` then its own `+`",
  "cnt\tset\t0\n\twhile\tcnt<2\n\tdc.w\t+\t; REF\n+\tdc.w\t$2222\ncnt\tset\tcnt+1\n\tendm\n")
S("n15_irp_back_in_iter", "`irp` body `-` then `dc.w -`",
  "\tirp\tv,1,2\n-\tdc.w\tv\n\tdc.w\t-\t; REF\n\tendm\n")
S("n16_back_from_nested_child", "outer defines `-`; inner (called after) does `rept 1` whose body refs `-`",
  "inner\tmacro\n\trept\t1\n\tdc.w\t-\t; REF\n\tendm\n\tendm\nouter\tmacro\n-\tdc.w\t$2222\n\tinner\n\tendm\n\touter\n")
S("n17_fwd_child_ref_parent_def", "outer: calls inner (whose rept body refs `+`), then outer defines `+`",
  "inner\tmacro\n\trept\t1\n\tdc.w\t+\t; REF\n\tendm\n\tendm\nouter\tmacro\n\tinner\n+\tdc.w\t$2222\n\tendm\n\touter\n")
S("n18_slash_in_body_back_inside", "body `/` then `dc.w -` inside, invoked twice",
  "mac\tmacro\n/\tdc.w\t$2222\n\tdc.w\t-\t; REF\n\tendm\n\tmac\n\tmac\n")
S("n19_globalsymbols_fwd_from_outside", "`dc.w +` before; a {GLOBALSYMBOLS} body defines `+`",
  "mac\tmacro\t{GLOBALSYMBOLS}\n+\tdc.w\t$2222\n\tendm\n\tdc.w\t+\t; REF\n\tmac\n")
S("n20_if_in_body", "body `if 1` / `-` / `endif`; `-` after the expansion",
  "mac\tmacro\n\tif\t1\n-\tdc.w\t$2222\n\tendif\n\tendm\n\tmac\n\tdc.w\t-\t; REF\n")
S("n21_plusplus_in_body", "body `++`; `+++` referenced before the call, `+` after the call defined outside",
  "mac\tmacro\n++\tdc.w\t$2222\n\tendm\n\tdc.w\t+++\t; REF\n\tmac\n+\tdc.w\t$3333\n")
S("n22_fwd_outside_skips_body", "`dc.w ++` before; body defines `+`; outer `+` after",
  "mac\tmacro\n+\tdc.w\t$2222\n\tendm\n\tdc.w\t++\t; REF\n\tmac\n+\tdc.w\t$3333\n")
S("n23_back_outside_skips_body", "outer `-`; body `-`; after the call `dc.w --`",
  "-\tdc.w\t$3333\nmac\tmacro\n-\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\t--\t; REF\n")

# substituted names, the forward-inside edges
S("s01_brace_fwd_inside", "body `dc.w Lab{n}` then `Lab{n}:`, invoked with 3 and 4",
  "mac\tmacro\tn\n\tdc.w\tLab{n}\t; REF\nLab{n}:\tdc.w\t$2222\n\tendm\n\tmac\t3\n\tmac\t4\n")
S("s02_param_fwd_inside", "body `dc.w nm` then `nm:`, invoked with Foo and Bar",
  "mac\tmacro\tnm\n\tdc.w\tnm\t; REF\nnm:\tdc.w\t$2222\n\tendm\n\tmac\tFoo\n\tmac\tBar\n")
S("s03_intlabel_fwd_inside", "{INTLABEL} body `dc.w __LABEL___B` then `__LABEL___B:`",
  "mac\tmacro\t{INTLABEL}\n\tdc.w\t__LABEL___B\t; REF\n__LABEL___B:\tdc.w\t$2222\n\tendm\nAint:\tmac\nBint:\tmac\n")
S("s04_brace_global_fwd_inside", "file-level cnt; body `dc.w Lab{cnt}` then `Lab{cnt}:`",
  "cnt\t=\t7\nmac\tmacro\n\tdc.w\tLab{cnt}\t; REF\nLab{cnt}:\tdc.w\t$2222\n\tendm\n\tmac\n")
S("s05_brace_changes_in_body", "body sets `cnt` then defines `Lab{cnt}:`, reads `Lab5` inside",
  "mac\tmacro\ncnt\tset\t5\nLab{cnt}:\tdc.w\t$2222\n\tdc.w\tLab5\t; REF\n\tendm\n\tmac\n")
S("s06_param_nested_arg", "outer passes its argument to inner as the label name; read outside",
  "inner\tmacro\tnm\nnm:\tdc.w\t$2222\n\tendm\nouter\tmacro\tq\n\tinner\tq\n\tdc.w\tq\t; REF\n\tendm\n\touter\tFoo\n")
S("s07_param_nested_outside", "outer passes its argument to inner as the label name; read at file level",
  "inner\tmacro\tnm\nnm:\tdc.w\t$2222\n\tendm\nouter\tmacro\tq\n\tinner\tq\n\tendm\n\touter\tFoo\n\tdc.w\tFoo\t; REF\n")
S("s08_param_value_binding", "argument-text name bound with `equ` in the body, read after (control)",
  "mac\tmacro\tnm\nnm\tequ\t$123\n\tendm\n\tmac\tFoo\n\tdc.w\tFoo\t; REF\n")
S("s09_param_label_dir", "argument-text name bound with `label *` in the body, read after (control)",
  "mac\tmacro\tnm\nnm\tlabel\t*\n\tendm\n\tmac\tFoo\n\tdc.w\tFoo\t; REF\n")

# enum edges
S("en1_inside", "`enum` member read inside the body (control)",
  "mac\tmacro\n\tenum\tEa=5,Eb\n\tdc.w\tEb\t; REF\n\tendm\n\tmac\n")
S("en2_twice", "`enum` in a body, macro invoked twice",
  "mac\tmacro\n\tenum\tEa=5,Eb\n\tendm\n\tmac\n\tmac\n\tdc.w\t$3333\n")
S("en3_file_enum_then_body_nextenum_inside", "file-level `enum`; body `nextenum Eb`, read inside",
  "\tenum\tEa=5\nmac\tmacro\n\tnextenum\tEb\n\tdc.w\tEb\t; REF\n\tendm\n\tmac\n")
S("en4_enum_counter_continues", "body `enum Ea=5`; after the call, file-level `nextenum Ez`, read Ez",
  "mac\tmacro\n\tenum\tEa=5\n\tendm\n\tmac\n\tnextenum\tEz\n\tdc.w\tEz\t; REF\n")

# include edges
S("in1_inside_after", "a label from a file included in a body, read later in the body",
  "mac\tmacro\n\tinclude\t\"in1_inc.inc\"\n\tdc.w\tLinc\t; REF\n\tendm\n\tmac\n\tmac\n",
  {"in1_inc.inc": "Linc:\tdc.w\t$2222\n"})
S("in2_inc_refs_own", "an included file that defines and references its own label, from a body",
  "mac\tmacro\n\tinclude\t\"in2_inc.inc\"\n\tendm\n\tmac\n\tmac\n",
  {"in2_inc.inc": "Linc:\tdc.w\t$2222\n\tdc.w\tLinc\t; REF\n"})
S("in3_inc_reads_caller_body", "a body label read from a file included later in the same body",
  "mac\tmacro\nLb:\tdc.w\t$2222\n\tinclude\t\"in3_inc.inc\"\n\tendm\n\tmac\n\tmac\n",
  {"in3_inc.inc": "\tdc.w\tLb\t; REF\n"})
S("in4_inc_fwd_inside", "a body reads, BEFORE the include, a label the included file defines",
  "mac\tmacro\n\tdc.w\tLinc\t; REF\n\tinclude\t\"in4_inc.inc\"\n\tendm\n\tmac\n\tmac\n",
  {"in4_inc.inc": "Linc:\tdc.w\t$2222\n"})
S("in5_file_include_control", "a file-level include's label, read after (control)",
  "\tinclude\t\"in5_inc.inc\"\n\tdc.w\tLinc\t; REF\n",
  {"in5_inc.inc": "Linc:\tdc.w\t$2222\n"})
S("in6_include_in_rept", "a label from a file included inside `rept 1`, read after",
  "\trept\t1\n\tinclude\t\"in6_inc.inc\"\n\tendm\n\tdc.w\tLinc\t; REF\n",
  {"in6_inc.inc": "Linc:\tdc.w\t$2222\n"})

# globalsymbols model
S("gs1_global_inner_read_in_outer", "plain outer calls {GLOBALSYMBOLS} inner (defines Lg); outer reads Lg after",
  "inner\tmacro\t{GLOBALSYMBOLS}\nLg:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tdc.w\tLg\t; REF\n\tendm\n\touter\n")
S("gs2_global_inner_read_outside", "plain outer calls {GLOBALSYMBOLS} inner (defines Lg); read at file level",
  "inner\tmacro\t{GLOBALSYMBOLS}\nLg:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tendm\n\touter\n\tdc.w\tLg\t; REF\n")
S("gs3_global_inner_twice_in_outer", "plain outer invoked twice, each calls {GLOBALSYMBOLS} inner (defines Lg)",
  "inner\tmacro\t{GLOBALSYMBOLS}\nLg:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tendm\n\touter\n\touter\n\tdc.w\t$3333\n")
S("gs4_rept_in_global_body", "{GLOBALSYMBOLS} body `rept 1` with a label, read after",
  "mac\tmacro\t{GLOBALSYMBOLS}\n\trept\t1\nLr:\tdc.w\t$2222\n\tendm\n\tendm\n\tmac\n\tdc.w\tLr\t; REF\n")
S("gs5_global_body_scope_after", "{GLOBALSYMBOLS} body `Inner:`; after the call `.b := 2`; read `Inner.b`",
  "Base:\tdc.w\t$5555\nmac\tmacro\t{GLOBALSYMBOLS}\nInner:\tdc.w\t$2222\n\tendm\n\tmac\n.b\t:=\t2\n\tdc.w\tInner.b\t; REF\n")
S("gs6_global_body_dot_then_base", "{GLOBALSYMBOLS} body `.dl:`, read `Base.dl` after",
  "Base:\tdc.w\t$5555\nmac\tmacro\t{GLOBALSYMBOLS}\n.dl:\tdc.w\t$2222\n\tendm\n\tmac\n\tdc.w\tBase.dl\t; REF\n")
S("gs7_global_body_dot_twice", "{GLOBALSYMBOLS} body `.dl:`, invoked twice under `Base:`",
  "Base:\tdc.w\t$5555\nmac\tmacro\t{GLOBALSYMBOLS}\n.dl:\tdc.w\t$2222\n\tendm\n\tmac\n\tmac\n\tdc.w\t$3333\n")

# the `.x` under a body plain label
S("dx1_two_scopes_one_body", "body `La:` `.x:` `Lb:` `.x:`, read `.x` inside after each",
  "mac\tmacro\nLa:\tdc.w\t$2222\n.x:\tdc.w\t.x\t; REF\nLb:\tdc.w\t$3333\n.x:\tdc.w\t.x\n\tendm\n\tmac\n")
S("dx2_twice", "body `Lp:` `.x:`, macro invoked twice",
  "mac\tmacro\nLp:\tdc.w\t$2222\n.x:\tdc.w\t$3333\n\tendm\n\tmac\n\tmac\n\tdc.w\t$5555\n")
S("dx3_read_qualified_inside", "body `Lp:` `.x:` then `dc.w Lp.x` inside",
  "mac\tmacro\nLp:\tdc.w\t$2222\n.x:\tdc.w\t$3333\n\tdc.w\tLp.x\t; REF\n\tendm\n\tmac\n")

# irpc, repaired
S("d04b_irpc", "plain label in a file-level `irpc` body, read after",
  "\tirpc\tc,\"a\"\nLc:\tdc.w\t$2222\n\tendm\n\tdc.w\tLc\t; REF\n")
S("d09b_irpc_var_name", "`irpc` label spelled by the loop variable, read after",
  "\tirpc\tc,\"Qz\"\nc:\tdc.w\t$2222\n\tendm\n\tdc.w\tQ\t; REF\n")

for name, (desc, text, extra) in shapes.items():
    with open(os.path.join(OUT, name + ".asm"), "w") as f:
        f.write("; " + desc + "\n" + text)
    for en, et in extra.items():
        with open(os.path.join(OUT, en), "w") as f:
            f.write(et)
with open(os.path.join(OUT, "INDEX.tsv"), "w") as f:
    for name, (desc, _, _) in shapes.items():
        f.write(f"{name}\t{desc}\n")
print(len(shapes), "shapes")
