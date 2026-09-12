#!/usr/bin/env bash
# Generate the probes, run them through the base and M4 binaries, diff, summarize.
set -u
S=/home/volence/sonic_hacks/.scratch/second-space-pin
python3 "$S/gen_probes.py" > /dev/null || exit 2
rm -rf "$S/probe-out"
bash "$S/probes.sh" "$S/probe-out" "base=$S/sigil-base" "m4=$S/sigil-m4"
cat "$S/probe-out/BINARIES"
echo "== diff -r base m4:"
diff -r "$S/probe-out/base" "$S/probe-out/m4"
echo "diff_exit=$?"
echo "== per probe (base):"
for d in "$S"/probe-out/base/p*; do
    n=$(basename "$d")
    img=none
    [[ -f $d/out.bin ]] && img=$(stat -c %s "$d/out.bin")
    echo "$n exit=$(cat "$d/exit") errors=$(/usr/bin/grep -c ': error: ' "$d/stderr") image=$img"
done
