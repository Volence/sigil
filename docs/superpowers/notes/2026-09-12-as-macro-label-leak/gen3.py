#!/usr/bin/env python3
"""Third batch: corners the fix's design has to take a position on."""
import os, sys
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
HEAD = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
TAIL = "\tdc.w\t$4444\n"
shapes = {}
def S(name, desc, body, extra=None):
    shapes[name] = (desc, HEAD + body + TAIL, extra or {})

S("en5_enum_in_rept", "`enum` member written in a file-level `rept 1` body, read after",
  "\trept\t1\n\tenum\tEa=5,Eb\n\tendm\n\tdc.w\tEb\t; REF\n")
S("en6_enum_in_include_in_body", "`enum` member written in a file included from a body, read after",
  "mac\tmacro\n\tinclude\t\"en6_inc.inc\"\n\tendm\n\tmac\n\tdc.w\tEb\t; REF\n",
  {"en6_inc.inc": "\tenum\tEa=5,Eb\n"})
S("gs8_global_dot_in_plain_outer_read_in_outer", "outer (under Base) calls {GLOBALSYMBOLS} inner writing `.dl:`; outer reads `.dl`",
  "Base:\tdc.w\t$5555\ninner\tmacro\t{GLOBALSYMBOLS}\n.dl:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tdc.w\t.dl\t; REF\n\tendm\n\touter\n")
S("gs9_global_dot_in_plain_outer_read_outside", "outer (under Base) calls {GLOBALSYMBOLS} inner writing `.dl:`; read `Base.dl` after",
  "Base:\tdc.w\t$5555\ninner\tmacro\t{GLOBALSYMBOLS}\n.dl:\tdc.w\t$2222\n\tendm\nouter\tmacro\n\tinner\n\tendm\n\touter\n\tdc.w\tBase.dl\t; REF\n")
S("gs10_plain_in_global_outer", "{GLOBALSYMBOLS} outer calls plain inner writing `Ln:`; outer reads Ln after inner",
  "inner\tmacro\nLn:\tdc.w\t$2222\n\tendm\nouter\tmacro\t{GLOBALSYMBOLS}\n\tinner\n\tdc.w\tLn\t; REF\n\tendm\n\touter\n")
S("gs11_global_value_binding_dot", "{GLOBALSYMBOLS} body `.v := 7` under Base, read `Base.v` after",
  "Base:\tdc.w\t$5555\nmac\tmacro\t{GLOBALSYMBOLS}\n.v\t:=\t7\n\tendm\n\tmac\n\tdc.w\tBase.v\t; REF\n")
S("dx4_body_label_scope_after_rept", "file-level `rept 1` body `Lr:`; after the loop `.x:` and `dc.w .x`",
  "\trept\t1\nLr:\tdc.w\t$2222\n\tendm\n.x:\tdc.w\t$3333\n\tdc.w\t.x\t; REF\n")
S("dx5_body_label_dot_arg_to_nested", "body `Lp:` `.x:`, then passes `.x` to a nested macro that emits it",
  "emit\tmacro\tv\n\tdc.w\tv\t; REF\n\tendm\nmac\tmacro\nLp:\tdc.w\t$2222\n.x:\tdc.w\t$3333\n\temit\t.x\n\tendm\n\tmac\n")

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
