#!/bin/bash
# headless harness legs (tools/leg_run.py, pinned oracle-aether spawned by the harness itself)
# on the four aeon ROMs, old vs new, then the BlastEm witness on each `run` leg result.
H=/home/volence/sonic_hacks/.walk-runs/harness; B=/home/volence/sonic_hacks/.walk-runs/bin
for side in old new; do
  T=$H/aeon-$side; cd $T
  export SIGIL_BUILD=$B/sigil-$side SIGIL_EMIT=$B/emit_sound_blob-$side
  for stem in s4 s4.debug; do
    for leg in idle run topspeed diagfall spindash stream; do
      timeout 900 python3 tools/leg_run.py --rom $T/$stem.bin --lst $T/$stem.lst --leg $leg --phase 0 --out $H/legs2-$side > $H/legs2-$side-$stem-$leg.log 2>&1
      echo "LEG $side $stem $leg rc=$?"
    done
    timeout 700 python3 tools/blastem_witness.py --rom $T/$stem.bin --lst $T/$stem.lst --result $H/legs2-$side/${stem}_run_p0.json > $H/blastem2-$side-$stem.log 2>&1
    echo "BLASTEM $side $stem rc=$?"
  done
  for stem in demo demo.debug; do
    timeout 900 python3 tools/leg_run.py --rom $T/$stem.bin --lst $T/$stem.lst --leg idle --phase 0 --out $H/legs2-$side > $H/legs2-$side-$stem-idle.log 2>&1
    echo "LEG $side $stem idle rc=$?"
  done
done
echo END
