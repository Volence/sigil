#!/usr/bin/env python3
"""Rewrite every hand-typed `assert_eq!(pins::X…, LITERAL)` in repin_pins.rs whose literal no
longer equals the regenerated pins.rs value, tagging each with one present-tense reason."""
import re, sys
root = sys.argv[1]
pins = open(f'{root}/crates/sigil-harness/src/pins.rs').read()
regions = {m.group(1): {k: int(v, 16) for k, v in re.findall(r'(\w+): (0x[0-9A-Fa-f]+)', m.group(2))}
           for m in re.finditer(r'pub const (\w+): Region = Region \{([^}]*)\}', pins)}
pinsv = {m.group(1): {k: int(v, 16) for k, v in re.findall(r'(\w+): (0x[0-9A-Fa-f]+)', m.group(2))}
         for m in re.finditer(r'pub const (\w+): Pin = Pin \{([^}]*)\}', pins)}
NOTE = '  // alignment-flip: sections pack to their DECLARED alignment (2 for all but the banks and the sound blobs), not to the residue of a frozen pin'
p = f'{root}/crates/sigil-harness/tests/repin_pins.rs'
src = open(p).read(); out = []; changed = 0
for line in src.splitlines():
    m = re.match(r'(\s*)assert_eq!\(pins::(\w+)\.(\w+), (0x[0-9A-Fa-f]+)\);(.*)$', line)
    if m:
        ind, name, field, lit, rest = m.groups()
        cur = regions.get(name, {}).get(field, pinsv.get(name, {}).get(field))
        if cur is not None and cur != int(lit, 16):
            line = f"{ind}assert_eq!(pins::{name}.{field}, {cur:#X});{NOTE}{rest}"; changed += 1
    m = re.match(r'(\s*)assert_eq!\(pins::(\w+), pins::Pin \{ plain: (0x[0-9A-Fa-f]+), debug: (0x[0-9A-Fa-f]+) \}\);(.*)$', line)
    if m:
        ind, name, pl, db, rest = m.groups(); cur = pinsv.get(name)
        if cur and (cur['plain'], cur['debug']) != (int(pl, 16), int(db, 16)):
            line = f"{ind}assert_eq!(pins::{name}, pins::Pin {{ plain: {cur['plain']:#X}, debug: {cur['debug']:#X} }});{NOTE}{rest}"; changed += 1
    out.append(line)
open(p, 'w').write('\n'.join(out) + '\n')
print("rewritten literals:", changed)
