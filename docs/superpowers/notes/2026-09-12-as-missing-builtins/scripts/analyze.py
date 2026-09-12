#!/usr/bin/env python3
"""analyze.py [prefix...] : per probe key, asl's exit / diag / bytes of the
construct line in both variants (a_ bare, b_ with an accepted preamble), and the
sigil result. Reads the files probe.sh left in probes/ (sigil outputs are the
ones from the LAST probe.sh run)."""
import os, re, sys

D = "/home/volence/sonic_hacks/.scratch/as-missing-builtins/probes"
SKIP = re.compile(r"^\s*(cpu|padding|org|end)\b|move\.w #\$1234|dc\.w \$5678|dc\.b \$EE|^\s*(F set|X equ|Later equ|signedToString)")


def construct_line(asm):
    for n, l in enumerate(open(asm).read().split("\n"), 1):
        if l.strip() and not SKIP.search(l):
            return n
    return None


def asl(path):
    b = path[:-4]
    n = construct_line(path)
    out = open(b + ".asl.out", errors="replace").read()
    rc = re.search(r"ASL_EXIT=(\d+)", out).group(1)
    diag = (re.search(r"ASL_DIAG=(\w+)", out) or [None, "?"])[1]
    errs = ";".join(re.findall(r"(?:error|warning) #\d+: [^\n]*", out)[:2])
    rows = []
    if os.path.exists(b + ".lst"):
        for l in open(b + ".lst", errors="replace"):
            m = re.match(r"^ +(\d+)/ +([0-9A-F]+) : (.*)$", l.rstrip("\n"))
            if m and int(m.group(1)) == n:
                rows.append(m.group(3)[:20].strip())
    return rc, diag, " | ".join(r for r in rows if r) or "-", errs


def sigil(path):
    b = path[:-4]
    if not os.path.exists(b + ".sigil.out"):
        return "?", ""
    t = open(b + ".sigil.out", errors="replace").read()
    errs = [l for l in t.splitlines() if "error" in l]
    hexs = "".join(l.strip() for l in t.splitlines() if "error" not in l and "warning" not in l)
    return ("ERR " + errs[0].split(": error: ", 1)[-1][:60]) if errs else hexs[:40], t


def main():
    prefixes = sys.argv[1:] or [""]
    keys = sorted({f[2:-4] for f in os.listdir(D) if f.endswith(".asm")})
    for k in keys:
        if not any(k.startswith(p) for p in prefixes):
            continue
        a = asl(f"{D}/a_{k}.asm")
        b = asl(f"{D}/b_{k}.asm")
        agree = "same" if (a[0], a[2]) == (b[0], b[2]) else "DIFF"
        s = sigil(f"{D}/a_{k}.asm")[0]
        print(f"{k:30} asl exit={a[0]} {a[1][:4]} {agree} [{a[2]}]{' B=[' + b[2] + ']' if agree == 'DIFF' else ''} {a[3][:60]}")
        print(f"{'':30}   sigil {s}")


main()
