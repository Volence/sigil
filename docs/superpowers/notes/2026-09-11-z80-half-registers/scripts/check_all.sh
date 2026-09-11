#!/usr/bin/env bash
# check_all.sh <sigil-binary> <label> : run check_sigil.py over every probe set
# and print each set's summary plus every row that is not MATCH or BOTH-REFUSE.
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
for set in m1 m2 m3 m4; do
    python3 "$S/check_sigil.py" "$S/probes/$set" "$1" "$2"
    /usr/bin/grep -v -E "^(BOTH-REFUSE|MATCH|#)" "$S/probes/check-$set-$2.txt" | cut -c1-220
done
echo CHECK_ALL_END
