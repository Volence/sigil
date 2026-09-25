#!/usr/bin/env bash
# aeon4.sh <tip> <base>: aeon's four shapes by both binaries, compared with each other
# and with the ROMs provisioning built into the reference tree.
set -u
TIP="$1"; BASE="$2"
A=/home/volence/sonic_hacks/.aeon-cli-define
O=/home/volence/sonic_hacks/.scratch/as-cli-define/aeon4
ID="python3 /home/volence/sonic_hacks/.scratch/as-cli-define/s3k/scripts/ident.py"
mkdir -p "$O"
echo "aeon: $A HEAD=$(git -C "$A" rev-parse HEAD) status-lines=$(git -C "$A" status --porcelain | wc -l)"
echo "tip:  $($TIP --version | sed -n 1p)"; echo "base: $($BASE --version | sed -n 1p)"
for shape in "s4::sonic4:" "s4.debug:--debug:sonic4:" "demo::demo:" "demo.debug:--debug:demo:"; do
  IFS=: read -r name dbg game _ <<< "$shape"
  for side in tip base; do
    bin="$TIP"; [ $side = base ] && bin="$BASE"
    rm -f "$O/$name-$side.bin"
    "$bin" build --aeon "$A" --game "$game" $dbg -o "$O/$name-$side.bin" > "$O/$name-$side.log" 2>&1
    echo "$name $side: exit $?"
    $ID "$O/$name-$side.bin"
  done
  $ID "$A/$name.bin"
  cmp "$O/$name-tip.bin" "$O/$name-base.bin" && echo "$name: tip == base"
  cmp "$O/$name-tip.bin" "$A/$name.bin" && echo "$name: tip == provisioned ROM"
done
echo AEON4_END
