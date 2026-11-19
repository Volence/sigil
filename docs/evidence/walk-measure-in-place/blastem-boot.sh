#!/bin/bash
H=/home/volence/sonic_hacks/.walk-runs/harness
for side in old new; do for st in s4 s4.debug demo demo.debug; do
  timeout 400 python3 $H/blastem_boot.py $H/aeon-$side/$st.bin $H/aeon-$side/$st.lst 300 > $H/boot-$side-$st.json 2>$H/boot-$side-$st.err
  echo "BOOT $side $st rc=$?"
done; done
echo END
