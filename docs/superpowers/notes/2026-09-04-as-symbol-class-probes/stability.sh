#!/usr/bin/env bash
# Oracle stability for the probes behind `../2026-09-04-as-symbol-class-tracking.md`
# and `../2026-09-05-as-duplicate-definition.md`: run each probe THREE times
# through `run.sh` and hash the whole stream. Three identical hashes per probe,
# or that shape does not carry a claim.
#
#     ./stability.sh                 # every m*.asm in this directory
#     ./stability.sh m12 m14 m20     # named stems only
#
# ── WHY THIS FILE EXISTS ─────────────────────────────────────────────────────
# `2026-09-05-as-duplicate-definition.md` opens with a stability claim — sixteen
# probes, three runs each, "one hash, 544d9b7ecf094dd1fa0dc71ccd63cce3, all
# three runs" — and names no runner. There was none committed, so the figure
# cannot be checked, and re-running the check by hand is exactly the situation
# that produces a new, differently-wrong filter each time: three plausible
# readings of "the whole diagnostic stream" (stderr alone, stderr plus exit
# status, and `run.sh`'s stdout+stderr with the listing) were each hashed here
# and none reproduces that value. The claim is not contradicted — it may well
# have been taken over a stream shape not tried — it is simply unverifiable as
# written, which is the thing a committed runner fixes.
#
# ── THE CLOCK ────────────────────────────────────────────────────────────────
# asl stamps the wall clock into its listing three times over, and `run.sh` cats
# the listing, so a bare hash of this stream changes every second — the page
# banner alone guarantees it. The blanking rules are `../asl-declock/declock.sed`,
# which carries each stamp's measured shape; `../asl-declock/selfcheck.sh` is
# the proof that the filtered stream still separates two runs differing in
# CONTENT, which is the property a stability check is worthless without.
#
# `m8.asm` is excluded: it needs command-line defines that `run.sh` does not
# pass, so its stream here would be a measurement of the wrong invocation.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
DECLOCK="$HERE/../asl-declock/declock.sed"
[[ -f $DECLOCK ]] || { echo "FATAL: no declock filter at $DECLOCK" >&2; exit 2; }

if [[ $# -gt 0 ]]; then
    stems=("$@")
else
    stems=()
    for f in "$HERE"/m*.asm; do
        b="$(basename "$f" .asm)"
        [[ $b == m8 ]] && continue
        stems+=("$b")
    done
fi

echo "# filter $DECLOCK md5 $(md5sum "$DECLOCK" | cut -d' ' -f1)"
unstable=0
for b in "${stems[@]}"; do
    printf '%-4s ' "$b"
    seen=""
    for _ in 1 2 3; do
        h="$("$HERE/run.sh" "$b.asm" 2>&1 | sed -E -f "$DECLOCK" | md5sum | cut -c1-12)"
        printf '%s ' "$h"
        seen="$seen $h"
    done
    if [[ $(echo "$seen" | tr ' ' '\n' | grep -c '[0-9a-f]') -eq 3 && \
          $(echo "$seen" | tr ' ' '\n' | sort -u | grep -c '[0-9a-f]') -eq 1 ]]; then
        echo "STABLE"
    else
        echo "UNSTABLE  <-- no claim may rest on this shape"
        unstable=$((unstable+1))
    fi
done
rm -f "$HERE"/m*.lst "$HERE"/m*.p
echo "# unstable shapes: $unstable"
[[ $unstable -eq 0 ]]
