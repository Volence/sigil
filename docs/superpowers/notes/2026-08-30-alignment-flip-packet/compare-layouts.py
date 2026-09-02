#!/usr/bin/env python3
"""compare-layouts.py <pre-dir> <post-dir> <sigil-root>

Per shape: every DECLARED head label's address in the pre and post listings, the delta,
how many moved, the largest move, and the histogram of (declared quantum, quantum a
residue-of-address reading of the shape's FROZEN table would have given) over the moved
sections. Reads: section_align.rs (the rows), golden/offcanonical_sizes/<shape>.txt
(the frozen provisional bases), and the two listings.
"""
import re, sys, zlib, pathlib, collections

pre, post, root = (pathlib.Path(a) for a in sys.argv[1:4])
rows = re.findall(r'^\s*d\("([^"]+)",\s*(0x[0-9A-Fa-f]+|\d+),', (root / "crates/sigil-harness/src/section_align.rs").read_text(), re.M)
declared = {label: int(q, 0) for label, q in rows}

def listing_addrs(p):
    out = {}
    for line in p.read_text(errors="replace").splitlines():
        m = re.match(r'\(\d+\)\s+\d+/([0-9A-Fa-f]+)\s*:\s+(\S+):\s*$', line)
        if m:
            out.setdefault(m.group(2), int(m.group(1), 16))
    return out

def frozen(shape):
    t = {}
    for line in (root / "crates/sigil-harness/golden/offcanonical_sizes" / f"{shape}.txt").read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"): continue
        name, addr = line.rsplit(" ", 1)
        t[name.strip()] = int(addr, 16)
    return t

def inferred(addr):
    for a in (16, 8, 4, 2):
        if addr % a == 0: return a
    return 1

def crc(p):
    d = p.read_bytes(); return f"{zlib.crc32(d)&0xffffffff:08x}/{len(d)}"

shapes = ["s4", "s4_debug", "demo", "demo_debug", "config_a", "config_b", "lean"]
for shape in shapes:
    a, b = listing_addrs(pre / f"{shape}.lst"), listing_addrs(post / f"{shape}.lst")
    fz = frozen(shape)
    print(f"\n## {shape}: pre {crc(pre / f'{shape}.bin')}  post {crc(post / f'{shape}.bin')}  size delta {len((post / f'{shape}.bin').read_bytes()) - len((pre / f'{shape}.bin').read_bytes()):+d}")
    print(f"| head label | pre | post | delta | declared | pin-residue quantum |")
    print(f"|---|---|---|---|---|---|")
    moved, hist, largest = 0, collections.Counter(), (0, None)
    for label in sorted(set(a) & set(declared), key=lambda l: a[l]):
        if label not in b:
            print(f"| {label} | {a[label]:#x} | (absent) | | {declared[label]} | |"); continue
        d = b[label] - a[label]
        inf = inferred(fz[label]) if label in fz else None
        flag = "" if d == 0 else " **moved**"
        if d != 0:
            moved += 1
            hist[(declared[label], inf)] += 1
            if abs(d) > abs(largest[0]): largest = (d, label)
        print(f"| {label} | {a[label]:#x} | {b[label]:#x} | {d:+d}{flag} | {declared[label]} | {inf if inf is not None else '(unpinned)'} |")
    print(f"\nsections moved: {moved} of {len(set(a) & set(declared))}; largest single move: {largest[0]:+d} ({largest[1]})")
    print("histogram over moved sections (declared quantum, pin-residue quantum) -> count:")
    for k, v in sorted(hist.items(), key=lambda kv: (kv[0][0], kv[0][1] or 0)):
        print(f"  declared {k[0]:>2} vs residue {k[1] if k[1] is not None else '(unpinned)':>10}: {v}")
    for probe in ("SoundTablesZ80_Head", "Sound_PlaySFX", "EndOfRom"):
        print(f"  {probe}: pre {a.get(probe, 0):#x} post {b.get(probe, 0):#x}")
