import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
HEAD = "\tcpu 68000\n\tpadding off\n\torg 0\n"
names = []
def put(name, body, head=HEAD):
    open(os.path.join(P, name + ".asm"), "w").write(f"{head}{body}\n\tend\n")
    names.append(name)
i = 0
for w in ["dc.w", "dc.l", "dw"]:
    for expr in ["S2", "S2+1", "S1"]:
        put(f"tw{i}", f"S1 equ \"a\"\nS2 equ \"ab\"\n\t{w} {expr}")
        i += 1
put("tw9", "S2 equ \"ab\"\n\tdc.w S2-1")
put("tcs1", "\tcharset 'a',$11\n\tdc.b \"ab\"+1,$EE")
put("tcs2", "\tcharset 'a',$11\n\tdc.b \"ab\",$EE")
for j, line in enumerate(["\tdc.w \"AB\"", "\tdc.w \"AB\"+0", "\tdc.l \"ABCD\""]):
    put(f"tl{j}", line)
print(" ".join(names))
