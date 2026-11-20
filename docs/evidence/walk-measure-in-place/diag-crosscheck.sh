#!/bin/bash
# Cross-validate the proof's independent site model against sigil's own width_flip_report,
# printed by a SCRATCH diagnostic build of the OLD walk (master + an env-gated eprintln at
# convergence that measures every section in place and diffs it against the converged
# remeasure). Never committed as code; the patch is kept as evidence.
D=/home/volence/sonic_hacks/.walk-runs/diagx; BIN=/home/volence/sonic_hacks/.walk-runs/diagsrc/target/release/sigil
P=/home/volence/sonic_hacks/.walk-runs/proof
rm -rf $D/tree; cp -a /home/volence/sonic_hacks/.aeon-sigil-ref-99cb963d $D/tree
for row in "plain|" "debug|--debug" "demo|--game demo" "demo_debug|--game demo --debug" "config_a|--config-a" "config_b|--config-b" "lean|--lean" "stress_evict|--stress-evict" "stress_art|--stress-art"; do
  name=${row%%|*}; flags=${row#*|}
  WALK_DIAG=1 $BIN build --aeon $D/tree $flags -o $D/$name.bin --no-build-json 2> $D/$name.err >/dev/null || echo "$name diag build FAILED"
  python3 - "$D/$name.err" "$P/$name.proof.txt" "$name" <<'PY'
import re,sys
err=open(sys.argv[1]).read(); proof=open(sys.argv[2]).read(); name=sys.argv[3]
diag={}
for m in re.finditer(r"section `([^`]+)` \(`[^`]*`\) measures (0x[0-9a-f]+) then (0x[0-9a-f]+)", err):
    diag[m.group(1)]=int(m.group(2),16)-int(m.group(3),16)
mine={}
for m in re.finditer(r"site `([^`]+)` -(\d+) at", proof): mine[m.group(1)]=int(m.group(2))
for m in re.finditer(r"note `([^`]+)` \(before island `[^`]+`\) would have measured \+(\d+)", proof): mine[m.group(1)]=int(m.group(2))
same = diag==mine
print(f"DIAGX {name} sigil-flip-sections={len(diag)} bytes={sum(diag.values())} model-sections={len(mine)} bytes={sum(mine.values())} {'AGREE' if same else 'DISAGREE'}")
if not same:
    for k in sorted(set(diag)|set(mine)):
        if diag.get(k)!=mine.get(k): print("   ",k,"sigil",diag.get(k),"model",mine.get(k))
PY
done
rm -rf $D/tree $D/*.bin
echo END
