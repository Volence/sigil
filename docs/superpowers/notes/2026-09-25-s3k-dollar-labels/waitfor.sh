#!/usr/bin/env bash
# waitfor.sh <log> <marker> <pid>: print new rc/crc/error lines; exit on marker or pid death.
L="$1"; M="$2"; P="$3"; prev=0
while true; do
  n=$(wc -l < "$L")
  if [ "$n" != "$prev" ]; then
    tail -n +$((prev+1)) "$L" | grep -E 'rc=|crc32|rror|_END'
    prev=$n
  fi
  grep -q "$M" "$L" && exit 0
  kill -0 "$P" 2>/dev/null || { echo "process $P gone without $M"; exit 1; }
  sleep 5
done
