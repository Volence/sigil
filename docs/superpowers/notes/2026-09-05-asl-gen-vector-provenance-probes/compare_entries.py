#!/usr/bin/env python3
"""Per-entry cross-build comparison for `declined_operand_sweep.sh`.

Reads the snapshots that script minted and reports, PER ENTRY (not per file):

  DIFFERS   the reference build and the varying build disagree on this entry
  UNSTABLE  the varying build disagrees with ITSELF across its runs
  DRIFT     the reference mint disagrees with the COMMITTED file

Any entry in the first two classes carries an operand asl declined to value, and
its byte column is an artifact under EITHER build. An entry in neither class is
not thereby proven sound — this comparison sees only shapes the two builds fill
in differently (see the header of the shell script).

Usage: compare_entries.py <snapdir> <tag> <n_varying_runs> <rel-path-for-report>
"""
import os
import re
import sys


def parse_arrow(path):
    """`<snippet> => <hex>` per line; blank and `#` lines skipped."""
    out = {}
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if " => " not in line:
                continue
            snippet, hexs = line.split(" => ", 1)
            out[snippet.strip()] = hexs.strip()
    return out


def parse_blocks(path):
    """`=== name ===` ... `--- bytes ---` <hex> block format."""
    out, name, byts, inb = {}, None, None, False
    with open(path) as fh:
        for line in fh:
            m = re.match(r"^=== (.*) ===$", line.rstrip("\n"))
            if m:
                if name is not None:
                    out[name] = byts
                name, byts, inb = m.group(1), None, False
            elif line.strip() == "--- bytes ---":
                inb = True
            elif inb and byts is None:
                byts = line.strip()
    if name is not None:
        out[name] = byts
    return out


def main():
    snap, tag, n, rel = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    parse = parse_blocks if "snippets" in tag else parse_arrow

    committed = parse(os.path.join(snap, f"{tag}.committed"))
    ref = parse(os.path.join(snap, f"{tag}.ref"))
    var_runs = []
    for i in range(1, n + 1):
        p = os.path.join(snap, f"{tag}.var{i}")
        if os.path.exists(p):
            var_runs.append((i, parse(p)))

    if not var_runs:
        print("   UNMEASURABLE: no varying-build snapshot parsed — not a clean verdict")
        sys.exit(3)

    names = list(committed.keys())
    for extra in list(ref) + [k for _, b in var_runs for k in b]:
        if extra not in committed and extra not in names:
            names.append(extra)

    differs, unstable, drift = [], [], []
    for nm in names:
        r = ref.get(nm)
        vals = {}
        for i, b in var_runs:
            vals.setdefault(b.get(nm), []).append(i)
        if len(vals) > 1:
            unstable.append((nm, sorted(v for v in vals if v is not None)))
        elif r is not None and next(iter(vals)) != r:
            differs.append((nm, r, next(iter(vals))))
        if nm in committed and r != committed[nm]:
            drift.append((nm, committed[nm], r))

    print(f"   entries compared            : {len(names)}")
    print(f"   varying-build runs completed: {len(var_runs)} of {n}")
    print(f"   reference mint vs COMMITTED : {len(drift)} DRIFT")
    print(f"   cross-build DIFFERS         : {len(differs)}")
    print(f"   varying-build UNSTABLE      : {len(unstable)}")
    for nm, c, r in drift[:40]:
        print(f"     DRIFT    {nm}\n       committed: {c}\n       ref mint : {r}")
    for nm, r, v in differs[:40]:
        print(f"     DIFFERS  {nm}\n       reference: {r}\n       varying  : {v}")
    for nm, vals in unstable[:40]:
        print(f"     UNSTABLE {nm}  values={vals}")
    total = len(drift) + len(differs) + len(unstable)
    if total == 0:
        print(f"   VERDICT: no declined-operand entry found in {rel} BY THIS PARAMETER")
    else:
        print(f"   VERDICT: {total} flagged entries in {rel} — each is an artifact, not a measurement")


if __name__ == "__main__":
    main()
