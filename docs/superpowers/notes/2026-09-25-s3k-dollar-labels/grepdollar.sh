#!/usr/bin/env bash
# grepdollar.sh: count files and lines with `$$` in each tree's assembly sources
# (comment text included, which only over-counts), with skdisasm as the positive control.
A=/home/volence/sonic_hacks/.aeon-s3k-codepage
echo "aeon HEAD: $(git -C "$A" rev-parse HEAD)"
echo "aeon status lines: $(git -C "$A" status --short | wc -l)"
for T in "$A" /home/volence/sonic_hacks/.scratch/s3k-dollar-labels/trees/sk-gen "$@"; do
  n=$(find "$T" \( -name '*.asm' -o -name '*.inc' -o -name '*.s' -o -name '*.emp' -o -name '*.z80' \) -not -path '*/.git/*' | wc -l)
  hits=$(find "$T" \( -name '*.asm' -o -name '*.inc' -o -name '*.s' -o -name '*.emp' -o -name '*.z80' \) -not -path '*/.git/*' -print0 | xargs -0 grep -c '\$\$' | grep -v ':0$')
  echo "== $T: $n source files; files with \$\$:"
  echo "${hits:-  (none)}"
done
echo GREP_END
