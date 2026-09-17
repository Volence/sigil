#!/usr/bin/env python3
"""compare.py --sigil <path> [probe...]: assemble every probe here with the
REFERENCE asl and with sigil, and compare where each one says
`address is not properly aligned`.

For each probe it prints one row: the SET of source locations asl raised
`warning #180` at, the set sigil raised `[as.odd-address]` at, and whether the
two images are byte-identical. Locations are compared as SETS because asl
repeats the warning once per pass that evaluates a line (two for a file with a
forward reference) and twice per pass for some mnemonics; that multiplicity is
its pass loop's, not the source's. The raw counts are printed beside the sets
so nothing is hidden by the comparison.

A row is evidence only if asl exited 0 with its pass loop complete: the asl
half runs through `asl_run` from `../asl-reference/asl_ref.sh`, which selects
the build by md5 and reports both. A probe asl refuses is a FAILED row, never a
silent skip. Exit status is 0 only when every probe assembled clean under both
toolchains, every location set agrees and every image agrees.

Scratch: every file is written under SCRATCH (default
/home/volence/sonic_hacks/.scratch/asl-warn-parity/compare), never beside the
probes.
"""
import argparse
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
GUARD = os.path.join(HERE, "..", "asl-reference", "asl_ref.sh")
# Probes sigil refuses outright, for a reason that is not this warning: a
# processor it does not encode, or an instruction spelling it does not accept.
# Their asl rows are still evidence about asl's rule; there is simply no sigil
# warning to compare. Asserted in BOTH directions, and the refusal must name the
# stated text, so an entry cannot outlive the refusal it describes.
SIGIL_REFUSES = {
    "p10_cpu_68010": "unsupported processor `68010`",
    "p10_cpu_68020": "unsupported processor `68020`",
    "p10_cpu_68030": "unsupported processor `68030`",
    "p10_cpu_68040": "unsupported processor `68040`",
    "p10_cpu_68332": "unsupported processor `68332`",
    "p23c_oddpc_68020": "unsupported processor `68020`",
    "s01_sigil_refuses_unsuffixed": "instruction needs an explicit size suffix",
    "s02_sigil_refuses_chk": "`chk` is not a recognized 68000 mnemonic",
    "s03_sigil_refuses_nbcd": "`nbcd` is not a recognized 68000 mnemonic",
}

LOC = r"^(?:> > > )?([^\s(]+\(\d+\)(?: [^:]*?)?)(?::\d+)?: warning"


def asl_side(scratch, probe):
    script = (
        '. "%s" || exit $?\n'
        'rm -f "%s.p" "%s.lst" "%s.asl.bin"\n'
        'USEANSI=n asl_run -xx -n -q -A -L -U "%s.asm" || exit $?\n'
        '"$ASLDIR/p2bin" "%s.p" "%s.asl.bin" >/dev/null 2>&1\n'
        'echo "ASL_MD5=$ASL_REF_GOT"\n'
    ) % ((GUARD,) + (probe,) * 6)
    r = subprocess.run(["sh", "-c", script], cwd=scratch,
                       capture_output=True, text=True)
    out = r.stdout + r.stderr
    locs = [m.group(1) for line in out.splitlines()
            if "warning #180" in line
            for m in [re.match(LOC, line)] if m]
    other = sorted({c for c in re.findall(r"warning #(\d+)", out) if c != "180"})
    complete = "ASL_DIAG=complete" in out
    md5 = re.search(r"ASL_MD5=(\w+)", out)
    return r.returncode, complete, locs, other, md5.group(1) if md5 else None, out


def sigil_side(sigil, scratch, probe):
    r = subprocess.run([sigil, probe + ".asm", "-o", probe + ".sigil.bin"],
                       cwd=scratch, capture_output=True, text=True)
    out = r.stdout + r.stderr
    locs = [m.group(1) for line in out.splitlines()
            if "[as.odd-address]" in line
            for m in [re.match(LOC, line)] if m]
    other = sorted({line for line in out.splitlines()
                    if "warning:" in line and "[as.odd-address]" not in line})
    return r.returncode, locs, other, out


def read(path):
    try:
        return open(path, "rb").read()
    except OSError:
        return None


def mint():
    """Write `asl_expected.tsv`: for every probe, each distinct location asl
    raised `warning #180` at and how many times, or a `-` row for a probe
    where it raised none. The Rust test `as_odd_address.rs` reads this file, so
    its expectations are the instrument's output rather than a transcription.
    Refuses to write anything unless every probe assembled clean under the
    reference build."""
    scratch = os.path.join(os.environ.get(
        "SCRATCH", "/home/volence/sonic_hacks/.scratch/asl-warn-parity/compare"), "mint")
    os.makedirs(scratch, exist_ok=True)
    for f in os.listdir(HERE):
        if f.endswith((".asm", ".inc")):
            shutil.copy(os.path.join(HERE, f), scratch)
    probes = sorted(f[:-4] for f in os.listdir(HERE) if f.endswith(".asm"))
    rows, md5s = [], set()
    for p in probes:
        rc, complete, locs, _other, md5, out = asl_side(scratch, p)
        md5s.add(md5)
        if rc != 0 or not complete:
            print("REFUSED TO MINT: %s exit=%d complete=%s\n%s" % (p, rc, complete, out))
            return 1
        if not locs:
            rows.append("%s\t-\t0" % p)
        for loc in sorted(set(locs)):
            rows.append("%s\t%s\t%d" % (p, loc, locs.count(loc)))
    if md5s != {"61e672562465725a8c102288a7da9098"}:
        print("REFUSED TO MINT: asl digest(s) %s are not the reference build" % md5s)
        return 1
    with open(os.path.join(HERE, "asl_expected.tsv"), "w") as f:
        f.write("# Minted by compare.py --mint from the reference asl, md5 "
                "61e672562465725a8c102288a7da9098,\n"
                "# flags -xx -n -q -A -L -U, every probe exit 0 with its pass loop "
                "complete.\n"
                "# probe<TAB>location of `warning #180` (or -)<TAB>times asl printed it\n")
        f.write("\n".join(rows) + "\n")
    print("MINTED probes=%d rows=%d" % (len(probes), len(rows)))
    return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sigil")
    ap.add_argument("--mint", action="store_true",
                    help="run asl alone over every probe and rewrite asl_expected.tsv")
    ap.add_argument("probes", nargs="*")
    a = ap.parse_args()
    if a.mint:
        return mint()
    if not a.sigil:
        ap.error("--sigil is required unless --mint")
    scratch = os.environ.get(
        "SCRATCH", "/home/volence/sonic_hacks/.scratch/asl-warn-parity/compare")
    os.makedirs(scratch, exist_ok=True)
    for f in os.listdir(HERE):
        if f.endswith((".asm", ".inc")):
            shutil.copy(os.path.join(HERE, f), scratch)
    probes = a.probes or sorted(f[:-4] for f in os.listdir(HERE) if f.endswith(".asm"))
    bad = agreed = refused = 0
    md5s = set()
    unknown = sorted(set(SIGIL_REFUSES) - set(probes))
    if unknown and not a.probes:
        print("SIGIL_REFUSES names probes that do not exist: %s" % unknown)
        bad += len(unknown)
    print("probes: %d" % len(probes))
    for p in probes:
        rc, complete, alocs, aother, md5, aout = asl_side(scratch, p)
        md5s.add(md5)
        src, slocs, sother, sout = sigil_side(a.sigil, scratch, p)
        aimg = read(os.path.join(scratch, p + ".asl.bin"))
        simg = read(os.path.join(scratch, p + ".sigil.bin"))
        asl_clean = rc == 0 and complete
        if p in SIGIL_REFUSES:
            ok = asl_clean and src != 0 and SIGIL_REFUSES[p] in sout
            print("%-36s %s  asl=%d%s  sigil refuses: %s" % (
                p, "SIGIL-REFUSES" if ok else "DIFFER", len(alocs),
                "" if complete else "(INCOMPLETE)", SIGIL_REFUSES[p]))
            for loc in sorted(set(alocs)):
                print("    %-40s asl x%d" % (loc, alocs.count(loc)))
            if ok:
                refused += 1
            else:
                bad += 1
                print("    asl exit=%d complete=%s sigil exit=%d, refusal text %s" % (
                    rc, complete, src,
                    "present" if SIGIL_REFUSES[p] in sout else "ABSENT"))
            continue
        ok_run = asl_clean and src == 0
        ok_set = set(alocs) == set(slocs)
        ok_img = aimg is not None and aimg == simg
        verdict = "AGREE" if (ok_run and ok_set and ok_img) else "DIFFER"
        if verdict != "AGREE":
            bad += 1
        else:
            agreed += 1
        print("%-36s %s  asl=%d%s sigil=%d  image=%s" % (
            p, verdict, len(alocs), "" if complete else "(INCOMPLETE)",
            len(slocs), "same" if ok_img else "DIFFERENT"))
        for loc in sorted(set(alocs) | set(slocs)):
            print("    %-40s asl x%d  sigil x%d" % (loc, alocs.count(loc), slocs.count(loc)))
        if aother:
            print("    asl other warning codes: %s" % ", ".join(aother))
        for line in sother:
            print("    sigil other: %s" % line)
        if not ok_run:
            print("    asl exit=%d complete=%s sigil exit=%d" % (rc, complete, src))
            print("    " + aout.replace("\n", "\n    "))
            print("    " + sout.replace("\n", "\n    "))
    print("asl md5: %s" % ", ".join(sorted(str(m) for m in md5s)))
    if md5s != {"61e672562465725a8c102288a7da9098"}:
        print("asl digest is not the reference build")
        bad += 1
    print("COMPARE_END probes=%d agree=%d sigil-refuses=%d differ=%d"
          % (len(probes), agreed, refused, bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
