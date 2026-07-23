#!/usr/bin/env python3
"""Bump gate-resume orgs by a fixed delta, then self-verify against repin's
authoritative new-value set. Gate orgs = `org` followed by >=2 spaces then $HEX.
The lone `org $10000` (ObjCodeBase, single space) is a fixed boundary and skipped."""
import re, sys

AEON = "/home/volence/sonic_hacks/aeon-p25"
FILES = [f"{AEON}/engine/engine.inc", f"{AEON}/games/sonic4/main.asm"]
REPIN = "/tmp/claude-1000/-home-volence-sonic-hacks-sigil/c2fca342-448e-4a05-8810-21b4cc1a650f/scratchpad/c3_full.txt"
DELTA = int(sys.argv[1], 0) if len(sys.argv) > 1 else 2

# authoritative new values from repin's printed block
auth = []
inblk = False
for line in open(REPIN):
    if "gate resume orgs" in line:
        inblk = True
    if inblk:
        m = re.search(r'^\s+org\s{2,}\$([0-9A-Fa-f]+)', line)
        if m:
            auth.append(int(m.group(1), 16))
auth_set = sorted(auth)

# bump gate orgs in the files
GATE = re.compile(r'^(\s+org\s{2,}\$)([0-9A-Fa-f]+)(.*)$')
produced = []
newtext = {}
for path in FILES:
    out = []
    for line in open(path):
        m = GATE.match(line)
        if m:
            old = int(m.group(2), 16)
            new = old + DELTA
            produced.append(new)
            out.append(f"{m.group(1)}{new:X}{m.group(3)}\n")
        else:
            out.append(line)
    newtext[path] = "".join(out)

produced_set = sorted(produced)
if produced_set != auth_set:
    print("MISMATCH — bumped set != repin authoritative set")
    print("  in-file-bumped not in repin:", [hex(x) for x in sorted(set(produced_set)-set(auth_set))])
    print("  repin not in in-file-bumped:", [hex(x) for x in sorted(set(auth_set)-set(produced_set))])
    print(f"  counts: bumped={len(produced_set)} repin={len(auth_set)}")
    sys.exit(1)

for path, txt in newtext.items():
    open(path, "w").write(txt)
print(f"OK — {len(produced_set)} gate orgs bumped by {DELTA:+#x}, all match repin authoritative set")
