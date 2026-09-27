#!/usr/bin/env bash
# compare.sh <sigil-output>: list every probe where the verdict (exit 0 or
# refused) differs from asl.out, or both assembled and the bytes differ.
# Refusal TEXT is not compared, only the verdict.
#
# Reproduce:  bash mkprobes.sh
#             bash run_asl.sh > asl.out
#             SIGIL=<sigil binary> bash run_sigil.sh > sigil-after.out
#             bash compare.sh sigil-after.out
cd "$(dirname "$0")" || exit 2
paste asl.out "$1" | awk -F'\t' '
    { split($1, a, " "); split($2, b, " ")
      if (a[2] != b[2] || (a[3] == "0" && a[4] != b[4])) { n++; print "  " $1; print "    sigil: " $2 } }
    END { printf "%d of %d probes disagree\n", n, NR }'
