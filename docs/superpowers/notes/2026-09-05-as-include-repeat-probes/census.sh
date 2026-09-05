#!/usr/bin/env bash
# THE ENGAGEMENT COUNTER, over the whole population: four aeon shapes and both
# corpora, under `SIGIL_CENSUS_INCLUDE=1`.
#
#   ./census.sh <sigil-binary> <aeon-dir> <out-dir>
#
# Why this exists rather than a before/after diff of the diagnostic streams: an
# empty diff reads the same in two different worlds — the rule engaged on every
# site and every site agreed, or the rule was never reached at all. The counter
# separates them. `executed=N repeats=0` beside an empty diff says the population
# was walked and holds no repeated include; `executed=0` beside one says the
# tree does not exercise `include` and the identity attests nothing about the
# behaviour.
#
# The counts are the point, not the bytes; the ROMs are written under the out-dir
# so the reference tree is never touched. Keep the out-dir outside the sigil
# worktree and outside any cargo target dir.
set -uo pipefail
SIGIL="$1"; AEON="$2"; OUT="$3"
mkdir -p "$OUT"
export SIGIL_CENSUS_INCLUDE=1
export AEON_DIR="$AEON"

# `CENSUS-INCLUDE` is printed once per PASS, and the AS front end runs several to
# converge. Print every line so the reader sees the pass structure, and the total
# so a zero cannot hide inside it.
report() {
    local base="$1"
    grep '^CENSUS-INCLUDE' "$base.err" | sort | uniq -c | sed 's/^/  /' || true
    echo "  passes reporting: $(grep -c '^CENSUS-INCLUDE' "$base.err" || true)"
    echo "  max repeats in any pass: $(grep -o 'repeats=[0-9]*' "$base.err" | cut -d= -f2 | sort -n | tail -1)"
    echo "  max refused-too-deep in any pass: $(grep -o 'refused-too-deep=[0-9]*' "$base.err" | cut -d= -f2 | sort -n | tail -1)"
}

for shape in "sonic4:" "sonic4:--debug" "demo:" "demo:--debug"; do
    g="${shape%%:*}"; d="${shape##*:}"
    tag="$g${d:+-debug}"
    echo "### aeon $tag"
    # shellcheck disable=SC2086
    "$SIGIL" build --aeon "$AEON" --game "$g" $d -o "$OUT/$tag.bin" \
        >"$OUT/$tag.out" 2>"$OUT/$tag.err"
    echo "BUILD_EXIT=$?"
    report "$OUT/$tag"
done

for c in "s1:/home/volence/sonic_hacks/s1disasm:sonic.asm" \
         "s2:/home/volence/sonic_hacks/s2disasm:s2.asm"; do
    n="${c%%:*}"; rest="${c#*:}"; dir="${rest%%:*}"; root="${rest#*:}"
    echo "### corpus $n"
    ( cd "$dir" && "$SIGIL" "$root" -o "$OUT/$n.bin" >"$OUT/$n.out" 2>"$OUT/$n.err" )
    echo "SIGIL_EXIT=$?"
    report "$OUT/$n"
done
