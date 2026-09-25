import os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
HEAD = "\tcpu 68000\n\tpadding off\n\torg 0\n"
CS3 = "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n"
bodies = [
    "\tcharset 'a',$11\n\tdc.b \"ab\"+1,$EE",
    f"{CS3}\tdc.b 'AB'+1",
    f"{CS3}\tcharset $23,$77\n\tdc.b 'AB'+1",
    "\tcharset $43,$77\n\tdc.b 'AB'+1",
    "\tcharset 'A',$11\n\tcharset $11,$55\n\tdc.b 'AB'+1",
    "\tcharset 'A',$51\n\tdc.b 'AB'+1",
    "\tcharset 'A',$11\n\tdc.b ('AB'+1)=\"AC\",('AB'+1)=\"\\x11C\"",
    "\tcharset 'A',$11\n\tdc.b lowstring('AB'+1)",
    "\tcharset 'a',$11\n\tdc.b \"ab\",$EE",
]
names = []
for i, b in enumerate(bodies):
    n = f"tp{i}"
    open(os.path.join(P, n + ".asm"), "w").write(f"{HEAD}{b}\n\tend\n")
    names.append(n)
print(" ".join(names))
