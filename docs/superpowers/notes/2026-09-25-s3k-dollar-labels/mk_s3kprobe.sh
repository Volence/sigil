#!/usr/bin/env bash
# mk_s3kprobe.sh: build probes/s3kdollar.asm from VERBATIM skdisasm 2fcd861c text:
# sonic3k.macros.asm 1-66 (ramaddr, vdpComm, the DMA macros, the dbf counters) and
# 93-103 (stopZ80/startZ80), then ten sonic3k.asm ranges holding all 93 `$$` lines
# of the 68000 code. Not verbatim: `cpu`/`supmode`/`org` at the top, the stand-in
# equates (s3k-standins.asm) for names defined elsewhere, and ONE inserted label,
# `BlueSpheresStartup:`, after the first range, because that range reaches it
# with a `bra.s` and a stand-in equate cannot sit inside a short branch's reach.
set -eu
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
T=$S/trees/sk-gen
O=$S/probes/s3kdollar.asm
{
  printf '\tcpu 68000\n\tsupmode on\n'
  sed -n '1,66p;93,103p' "$T/sonic3k.macros.asm"
  cat "$S/s3k-standins.asm"
  printf '\torg $2A6\n'
  sed -n '297,351p' "$T/sonic3k.asm"
  printf 'BlueSpheresStartup:\n\tnop\n'
  for r in 486,507 1010,1222 1313,1345 1747,1781 2003,2116 2269,2297 2298,2563 2667,2790; do
    sed -n "${r}p" "$T/sonic3k.asm"
  done
} > "$O"
echo "wrote $O: $(wc -l < "$O") lines, $(grep -c '\$\$' "$O") lines with \$\$, md5 $(md5sum < "$O" | cut -c1-32)"
