#!/usr/bin/env bash
# reach.sh: does aeon's source use the new mnemonics or PC-relative indexed/movem
# forms at all? Every grep is paired with the same grep on S3K as a positive control.
A=/home/volence/sonic_hacks/.aeon-bcd-pcindex
K=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/trees/sk-gen/sonic3k.asm
MN='^[^;]*\b(addx|subx|abcd|sbcd|negx|nbcd)\b'
PCX='^[^;]*\(pc, *(d[0-7]|a[0-7]|sp)'
MVPC='^[^;]*movem[^;]*pc'
echo "aeon .asm files:"; find $A -name '*.asm' -not -path '*/.git/*' | sed "s|$A/||"
echo "aeon .asm/.inc/.s count: $(find $A \( -name '*.asm' -o -name '*.inc' -o -name '*.s' \) -not -path '*/.git/*' | wc -l)"
echo "aeon .emp count: $(find $A -name '*.emp' -not -path '*/.git/*' | wc -l)"
for kind in asm emp; do
  echo "[$kind] new mnemonics: $(find $A -name "*.$kind" -not -path '*/.git/*' -exec grep -hiE "$MN" {} + | wc -l)"
  echo "[$kind] (pc,Xn): $(find $A -name "*.$kind" -not -path '*/.git/*' -exec grep -hiE "$PCX" {} + | wc -l)"
  echo "[$kind] movem ... pc: $(find $A -name "*.$kind" -not -path '*/.git/*' -exec grep -hiE "$MVPC" {} + | wc -l)"
done
echo "(pc,Xn) sites in aeon:"
find $A \( -name '*.asm' -o -name '*.emp' \) -not -path '*/.git/*' -exec grep -HniE "$PCX" {} + | sed "s|$A/||" | head -20
echo "control S3K: mnemonics $(grep -ciE "$MN" $K)  (pc,Xn) $(grep -ciE "$PCX" $K)  movem..pc $(grep -ciE "$MVPC" $K)"
