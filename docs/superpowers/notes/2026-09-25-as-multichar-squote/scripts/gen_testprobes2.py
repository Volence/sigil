import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
HEAD = "\tcpu 68000\n\tpadding off\n\torg 0\n"
Z80_HEAD = "\tcpu z80\n\torg 0\n"
names = []
def put(name, body, head):
    open(os.path.join(P, name + ".asm"), "w").write(f"{head}{body}\n\tend\n")
    names.append(name)
for i, expr in enumerate(["S2", "S2+1", "S1"]):
    put(f"tz{i}", f"S1 equ \"a\"\nS2 equ \"ab\"\n\tdw {expr}", Z80_HEAD)
put("tz3", "S2 equ \"ab\"\n\tdw S2-1", Z80_HEAD)
put("tdw68", "\tdw 1", HEAD)
print(" ".join(names))
