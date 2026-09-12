#!/usr/bin/env python3
"""Fourth batch: {GLOBALSYMBOLS} nested with plain macros, the shapes that
decide whether two transparency rules in the fix carry anything."""
import os, sys
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
HEAD = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
TAIL = "\tdc.w\t$4444\n"
shapes = {}
def S(name, desc, body):
    shapes[name] = (desc, HEAD + body + TAIL)

S("gs12_value_binding_in_global_in_plain", "plain outer (under Base) calls {GLOBALSYMBOLS} inner writing `.v := 7`; read `Base.v` after",
  "Base:\tdc.w\t$5555\ninner\tmacro\t{GLOBALSYMBOLS}\n.v\t:=\t7\n\tendm\nouter\tmacro\n\tinner\n\tendm\n\touter\n\tdc.w\tBase.v\t; REF\n")
S("gs13_dot_read_inside_global_in_plain", "plain outer calls {GLOBALSYMBOLS} inner writing `.dl:` and reading `.dl` itself",
  "Base:\tdc.w\t$5555\ninner\tmacro\t{GLOBALSYMBOLS}\n.dl:\tdc.w\t$2222\n\tdc.w\t.dl\t; REF\n\tendm\nouter\tmacro\n\tinner\n\tendm\n\touter\n\touter\n")
S("gs14_plain_value_binding_in_global", "{GLOBALSYMBOLS} outer (under Base) calls plain inner writing `.v := 7`; read `Base.v` after",
  "Base:\tdc.w\t$5555\ninner\tmacro\n.v\t:=\t7\n\tendm\nouter\tmacro\t{GLOBALSYMBOLS}\n\tinner\n\tendm\n\touter\n\tdc.w\tBase.v\t; REF\n")
S("gs15_plain_label_dir_in_global", "{GLOBALSYMBOLS} outer calls plain inner writing `Tbl label *`; after, `.c := 3`, read `Tbl.c`",
  "Base:\tdc.w\t$5555\ninner\tmacro\nTbl\tlabel\t*\n\tendm\nouter\tmacro\t{GLOBALSYMBOLS}\n\tinner\n\tendm\n\touter\n.c\t:=\t3\n\tdc.w\tTbl.c\t; REF\n")

for name, (desc, text) in shapes.items():
    with open(os.path.join(OUT, name + ".asm"), "w") as f:
        f.write("; " + desc + "\n" + text)
with open(os.path.join(OUT, "INDEX.tsv"), "w") as f:
    for name, (desc, _) in shapes.items():
        f.write(f"{name}\t{desc}\n")
print(len(shapes), "shapes")
