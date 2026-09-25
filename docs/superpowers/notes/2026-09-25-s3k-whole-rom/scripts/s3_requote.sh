#!/usr/bin/env bash
# s3_requote.sh: size the S3-alone class. In a copy of the gen tree, rewrite the
# multi-character single-quoted dc.b operands sigil refused (s3.asm 394..420) to
# double quotes, show every edit on disk, prove the edit is neutral under asl + p2bin
# (same image as buildS3.lua's reference), then run sigil on it and compare.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
T=$S/trees/s3-requote
rm -rf "$T"; cp -a "$S/trees/sk-gen" "$T"
cd "$T" || exit 1
python3 - <<'PY'
import re
p = "s3.asm"
lines = open(p, encoding="latin-1").read().split("\n")
n = 0
for i in range(393, 421):          # 1-based 394..421
    l = lines[i]
    if re.match(r"\s*dc\.b\s", l):
        new = re.sub(r"(?<=[\s,])'([^']{2,})'(?=\s*(,|;|$))", r'"\1"', l)
        if new != l:
            print(f"s3.asm:{i+1}: {l.strip()}  ->  {new.strip()}")
            lines[i] = new; n += 1
open(p, "w", encoding="latin-1").write("\n".join(lines))
print(f"edits: {n}")
PY
echo "--- diff on disk"
diff "$S/trees/sk-gen/s3.asm" s3.asm
echo "--- asl + p2bin on the edited tree (buildS3.lua's line)"
rm -f s3.p s3.log
asl_run -xx -n -q -A -L -U -E -i . s3.asm; echo "asl rc=$?"
[ -f s3.log ] && { echo "s3.log:"; head -20 s3.log; } || echo "no s3.log"
"$ASLDIR/p2bin" -p=FF -z=0,uncompressed,Size_of_Snd_driver_guess,before -z=1300,uncompressed,Size_of_Snd_driver2_guess,before s3.p asl-edited.bin; echo "p2bin rc=$?"
cp asl-edited.bin "$S/flips/asl-s3-edited.bin"
python3 "$S/scripts/compare.py" "$S/flips/ref-s3-run2.bin" "$S/flips/asl-s3-edited.bin"; echo "edit-neutral compare rc=$?"
echo "--- sigil on the edited tree"
bash "$S/scripts/run_sigil.sh" s3-requote "$T" s3.asm -p=FF -z=0,uncompressed,Size_of_Snd_driver_guess,before -z=1300,uncompressed,Size_of_Snd_driver2_guess,before
if [ -f "$S/runs/s3-requote/image.bin" ]; then
  python3 "$S/scripts/compare.py" "$S/flips/ref-s3-run2.bin" "$S/runs/s3-requote/image.bin"; echo "compare rc=$?"
  python3 "$S/scripts/diffranges.py" "$S/flips/ref-s3-run2.bin" "$S/runs/s3-requote/image.bin"
fi
echo "--- sigil rows by message"
sed -E 's/^[^:]*\([0-9]+\)[^:]*:[0-9]+: //; s/\$[0-9A-F]+/$N/g' "$S/runs/s3-requote/rows" | sort | uniq -c | sort -rn | head
echo S3REQUOTE_END
