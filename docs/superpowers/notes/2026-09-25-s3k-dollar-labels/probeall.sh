#!/usr/bin/env bash
# probeall.sh <bundle> [sigil]: split the bundle into probes/ and run probe.sh on each.
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
python3 "$S/split.py" "$1"
for n in $(grep '^### ' "$1" | cut -c5-); do
  SIGIL="${2:-$S/target/release/sigil}" bash "$S/probe.sh" "$n"
done
echo PROBEALL_END
