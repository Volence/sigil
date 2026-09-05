#!/usr/bin/env bash
# ASL-GEN-VECTORS-UNIDENTIFIED — the declined-operand enumeration, PARAMETER 2.
#
# `declined_operand_sweep.sh` enumerates by cross-build disagreement. That is a
# derived handle on the declined-operand set and it is blind to a shape both
# builds fill in identically. This sweep asks a question that does not depend on
# a second build at all:
#
#     DID ASL SAY ANYTHING ABOUT THIS LINE?
#
# For every entry of the three corpora it assembles the entry exactly as the
# generator does and reports the entry if asl exits non-zero OR writes anything
# on stderr OR its listing carries a warning/error count above zero.
#
# What this covers that parameter 1 does not: an operand asl WARNS about and then
# substitutes for, and an operand both builds decline the same way. What it does
# NOT cover: the silent class — `#f(<register>)`, exit 0, empty stderr, clean
# listing — which is precisely why there are two parameters and not one. The
# injected control below is a member of the silent class ON PURPOSE, so this
# sweep is expected to MISS it; `declined_operand_sweep.sh` is what catches it.
# A sweep that claimed to catch everything would be the thing to distrust.
#
# Nothing here writes to a golden: entries are read out of the committed files
# and assembled in scratch. There is no restore step because there is nothing to
# restore, which is itself worth stating — a sweep that cannot corrupt its
# subject is a better sweep.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
. "$REPO/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?

SCRATCH="${TMPDIR:-/tmp}/asl_diag_sweep.$$"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT

echo "# asl $ASL md5 $(md5sum "$ASL" | cut -d' ' -f1)"
echo

# Assemble one source. Echoes a one-line reason when asl had something to say,
# nothing when it was silent.
asl_complaint() {
    local src="$1"
    printf '%s' "$src" > "$SCRATCH/e.asm"
    rm -f "$SCRATCH/e.p" "$SCRATCH/e.lst"
    local err
    err="$(cd "$SCRATCH" && USEANSI=n "$ASL" -cpu 68000 -q -L -U \
        -olist "$SCRATCH/e.lst" -o "$SCRATCH/e.p" "$SCRATCH/e.asm" 2>&1 >/dev/null)"
    local rc=$?
    if [[ $rc -ne 0 ]]; then
        echo "exit $rc"
        return
    fi
    if [[ -n $err ]]; then
        echo "stderr: $(printf '%s' "$err" | head -1)"
        return
    fi
    # AS's listing footer counts warnings and errors; a non-zero count is a
    # complaint even when the exit status is 0.
    if [[ -f $SCRATCH/e.lst ]]; then
        local counts
        counts="$(grep -Ei '^ *[0-9]+ (warning|error)' "$SCRATCH/e.lst" \
                  | grep -Ev '^ *0 ' | head -1)"
        [[ -n $counts ]] && echo "listing: $(echo "$counts" | tr -s ' ')"
    fi
}

# ---- the `<snippet> => <hex>` corpora ---------------------------------------
sweep_arrow() {
    local rel="$1" pre="$2"
    echo "=== $rel"
    local n=0 flagged=0 snippet reason
    while IFS= read -r line; do
        [[ -z $line || $line == \#* ]] && continue
        snippet="${line%% => *}"
        n=$((n+1))
        reason="$(asl_complaint "$pre        $snippet"$'\n')"
        if [[ -n $reason ]]; then
            flagged=$((flagged+1))
            echo "   COMPLAINT  $snippet  -- $reason"
        fi
    done < "$REPO/$rel"
    echo "   $n entries assembled, $flagged with a complaint from asl"
}

# ---- the `=== name ===` corpus ----------------------------------------------
sweep_blocks() {
    local rel="$1"
    echo "=== $rel"
    local n=0 flagged=0
    # Split the committed file into one .asm per block, then assemble each.
    python3 - "$REPO/$rel" "$SCRATCH/blocks" <<'PY'
import os, re, sys
src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
name, asm, inb, seen = None, [], False, False
def flush():
    if name is not None:
        open(os.path.join(out, name + ".asm"), "w").write("".join(asm))
for line in open(src):
    m = re.match(r"^=== (.*) ===$", line.rstrip("\n"))
    if m:
        flush(); name, asm, inb, seen = m.group(1), [], False, True
    elif not seen:
        continue
    elif line.strip() == "--- bytes ---":
        inb = True
    elif not inb:
        asm.append(line)
flush()
PY
    local f nm reason
    for f in "$SCRATCH/blocks"/*.asm; do
        [[ -e $f ]] || continue
        nm="$(basename "$f" .asm)"
        n=$((n+1))
        reason="$(asl_complaint "$(cat "$f")")"
        if [[ -n $reason ]]; then
            flagged=$((flagged+1))
            echo "   COMPLAINT  $nm  -- $reason"
        fi
    done
    echo "   $n blocks assembled, $flagged with a complaint from asl"
}

# ---- the control -------------------------------------------------------------
# A shape asl refuses LOUDLY (range overflow, exit 2). If this sweep does not
# flag it, the sweep is blind and its zeroes mean nothing.
echo "=== CONTROL (must be flagged, or this sweep is blind)"
ctl="$(asl_complaint '        cpu 68000
        org 0
        move.w  #65536,d0
')"
if [[ -n $ctl ]]; then
    echo "   control FLAGGED -- $ctl"
else
    echo "   *** control NOT FLAGGED — this sweep is blind; its zeroes are meaningless ***"
    exit 4
fi
echo

sweep_arrow crates/sigil-isa/tests/m68k_golden_vectors.txt $'        cpu 68000\n        org 0\n'
echo
sweep_arrow crates/sigil-isa/tests/z80_golden_vectors.txt  $'        cpu z80\n        phase 0\n'
echo
sweep_blocks crates/sigil-frontend-as/tests/snippets_golden.txt
