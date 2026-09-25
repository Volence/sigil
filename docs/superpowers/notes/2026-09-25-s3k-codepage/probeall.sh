#!/usr/bin/env bash
# probeall.sh <name>...: run probe.sh over each probe name.
for n in "$@"; do bash /home/volence/sonic_hacks/.scratch/s3k-codepage/probe.sh "$n"; done
