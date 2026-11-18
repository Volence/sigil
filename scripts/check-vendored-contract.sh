#!/usr/bin/env bash
# Is sigil's vendored empyrean.build contract (contract/build/) equal to empyrean's?
#
#   EMPYREAN_DIR=<an empyrean checkout or worktree> scripts/check-vendored-contract.sh
#
# Exit 0 = every file SOURCE pins equals the file in $EMPYREAN_DIR (schema, runner and every
# vector), with no extra file on either side. Exit 1 = a difference. Exit 2 = could not run.
# A MISSING EMPYREAN_DIR IS A FAILURE (exit 2, loud), never a skip: in the landing bar a gate
# that cannot compare is not green. The CI-safe half (SOURCE equals the files here, no other
# repo needed) is the test `vendored_schema_matches_source` in sigil-link.
set -uo pipefail
cd "$(dirname "$0")/.."

if [[ -z "${EMPYREAN_DIR:-}" ]]; then
    echo "check-vendored-contract: FAILED TO RUN: EMPYREAN_DIR is not set. This gate compares against empyrean and cannot skip." >&2
    echo "STEP_CONTRACT_VENDOR rc=2"; exit 2
fi
SRC="$EMPYREAN_DIR/contract/schema"
[[ -d "$SRC/build" && -f "$SRC/tests/validate_build_schema.py" ]] || {
    echo "check-vendored-contract: FAILED TO RUN: $SRC has no build/ contract (is EMPYREAN_DIR an empyrean tree on the contract branch?)" >&2
    echo "STEP_CONTRACT_VENDOR rc=2"; exit 2; }
[[ -f contract/build/SOURCE ]] || { echo "check-vendored-contract: FAILED TO RUN: contract/build/SOURCE missing" >&2; echo "STEP_CONTRACT_VENDOR rc=2"; exit 2; }

# Where a vendored path lives in empyrean (the runner is the one file that moves).
upstream() {
    if [[ "$1" == validate_build_schema.py ]]; then echo "$SRC/tests/validate_build_schema.py"; else echo "$SRC/build/$1"; fi
}

bad=0; n=0
while read -r h p; do
    [[ ${#h} -eq 64 && "$h" =~ ^[0-9a-f]+$ ]] || continue
    n=$((n+1))
    u=$(upstream "$p")
    if [[ ! -f "$u" ]]; then echo "MISSING upstream: $p"; bad=1; continue; fi
    uh=$(sha256sum "$u" | cut -d' ' -f1)
    if [[ "$uh" != "$h" ]]; then echo "DIFFERS: $p (SOURCE $h, empyrean $uh)"; bad=1; fi
done < contract/build/SOURCE
(( n > 0 )) || { echo "check-vendored-contract: FAILED TO RUN: SOURCE pins no files" >&2; echo "STEP_CONTRACT_VENDOR rc=2"; exit 2; }

# No unpinned file upstream: a new vector in empyrean that was never vendored.
while IFS= read -r f; do
    rel="${f#"$SRC/build/"}"
    grep -q "  $rel\$" contract/build/SOURCE || { echo "NOT VENDORED: $rel"; bad=1; }
done < <(find "$SRC/build" -type f | LC_ALL=C sort)

echo "check-vendored-contract: $n pinned files compared against $EMPYREAN_DIR"
if (( bad )); then echo "STEP_CONTRACT_VENDOR rc=1"; exit 1; fi
echo "STEP_CONTRACT_VENDOR rc=0"
