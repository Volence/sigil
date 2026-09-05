#!/usr/bin/env bash
# THE CENSUS. A grep cannot enumerate this population — the names come out of
# macro expansion, `\{}` interpolation and scope qualification, so they are not
# in the source text. The enumeration is the COMPILER: `SIGIL_CENSUS_EXPLABEL=1`
# makes it print one line per plain PC label defined at `expansion_depth > 0`,
# saying whether `scan_plain_labels` claimed the name (`scoped=yes`) or the
# fail-safe fallback left it global (`scoped=no`), plus a per-pass count of the
# expansion instances that got a non-empty namespace.
#
#   ./census.sh <sigil-binary> <aeon-dir> <out-dir>
#
# The counts, not the byte output, are the point — the ROMs are written under the
# out-dir so the reference tree is never touched. Put the out-dir OUTSIDE the
# sigil worktree if it will hold a second aeon tree; see poscontrol.sh.
set -uo pipefail
SIGIL="$1"; AEON="$2"; OUT="$3"
mkdir -p "$OUT"
export SIGIL_CENSUS_EXPLABEL=1
export AEON_DIR="$AEON"

# Split the two line kinds: the per-label rows, and the per-pass reachability
# witness. Counting them together is how a run reports "51 labels" for a tree
# that has none.
census_report() {
    local base="$1"
    grep '	depth=' "$base.err" > "$base.labels" || true
    cut -f2 "$base.labels" | sort -u > "$base.names" || true
    echo "  label definitions inside an expansion: $(wc -l < "$base.labels")"
    echo "  distinct names: $(grep -c . "$base.names" || true)"
    echo "  scoped=yes: $(grep -c 'scoped=yes' "$base.labels" || true)  scoped=no: $(grep -c 'scoped=no' "$base.labels" || true)"
    echo "  instances with a namespace, per pass: $(grep 'instances-with-labels' "$base.err" | sort -u | tr '\n' ' ')"
}

for shape in "sonic4:" "sonic4:--debug" "demo:" "demo:--debug"; do
    g="${shape%%:*}"; d="${shape##*:}"
    tag="$g${d:+-debug}"
    echo "### aeon $tag"
    # shellcheck disable=SC2086
    "$SIGIL" build --aeon "$AEON" --game "$g" $d -o "$OUT/$tag.bin" \
        >"$OUT/$tag.out" 2>"$OUT/$tag.err"
    echo "BUILD_EXIT=$?"
    census_report "$OUT/$tag"
done

for c in "s1:/home/volence/sonic_hacks/s1disasm/sonic.asm" \
         "s2:/home/volence/sonic_hacks/s2disasm/s2.asm"; do
    n="${c%%:*}"; root="${c#*:}"
    echo "### corpus $n"
    ( cd "$(dirname "$root")" && "$SIGIL" "$(basename "$root")" >"$OUT/$n.out" 2>"$OUT/$n.err" )
    echo "SIGIL_EXIT=$?"
    census_report "$OUT/$n"
done
