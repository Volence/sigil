#!/usr/bin/env bash
# byte_corpus.sh BASE NEW : every committed .asm in the sigil repo, assembled by
# both binaries from the file's own directory. Per file: exit status, image
# crc32+size, and the diagnostic (level, message) multiset with locations
# stripped. Byte neutrality is the image column; the diagnostic column must
# differ in locations only.
BASE="$1"; NEW="$2"
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312
OUT="$S/bytecorpus"; rm -rf "$OUT"; mkdir -p "$OUT"
echo "base md5: $(md5sum "$BASE" | cut -d' ' -f1)   new md5: $(md5sum "$NEW" | cut -d' ' -f1)"
cd "$W" || exit 1
git ls-files '*.asm' > "$OUT/files.txt"
python3 - "$BASE" "$NEW" "$W" "$OUT" <<'PY'
import subprocess, sys, os, zlib, re, collections
base, new, root, out = sys.argv[1:5]
files = open(os.path.join(out, "files.txt")).read().split()
ROW = re.compile(r"^(?:(?P<loc>.*?): )?(?P<level>error|warning|note): (?P<msg>.*)$")
def run(binp, f, tag):
    d = os.path.join(root, os.path.dirname(f))
    img = os.path.join(out, tag + ".bin")
    if os.path.exists(img): os.remove(img)
    try:
        p = subprocess.run([binp, os.path.basename(f), "-o", img], cwd=d, capture_output=True, timeout=120)
    except subprocess.TimeoutExpired:
        return ("timeout", None, [], b"")
    im = open(img, "rb").read() if os.path.exists(img) else None
    rows = []
    for line in p.stderr.decode("utf-8", "replace").splitlines():
        m = ROW.match(line)
        if m: rows.append((m["level"], m["msg"]))
    return (p.returncode, im, rows, p.stderr)
tot = collections.Counter()
lines = []
for f in files:
    b = run(base, f, "b"); n = run(new, f, "n")
    tot["files"] += 1
    if b[0] != n[0]: tot["exit_differs"] += 1; lines.append(f"EXIT DIFF {f}: {b[0]} -> {n[0]}")
    if b[1] is not None or n[1] is not None:
        tot["images"] += 1
        if b[1] == n[1]:
            tot["image_identical"] += 1
            lines.append(f"IMAGE SAME {f} size={len(n[1])} crc32={zlib.crc32(n[1]):08x}")
        else:
            tot["image_differs"] += 1
            lines.append(f"IMAGE DIFF {f}")
    if collections.Counter(b[2]) != collections.Counter(n[2]):
        tot["message_multiset_differs"] += 1; lines.append(f"MESSAGES DIFF {f}")
    elif b[3] != n[3]:
        tot["locations_only_differ"] += 1; lines.append(f"LOCATIONS MOVED {f}")
    else:
        tot["stderr_identical"] += 1
open(os.path.join(out, "report.txt"), "w").write("\n".join(lines) + "\n")
print(dict(tot))
PY
