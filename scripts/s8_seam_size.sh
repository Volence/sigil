#!/usr/bin/env bash
# Size the deferred `sigil-harness` crate split (campaign-gap-ledger, lens sweep S8).
#
# WHY THIS EXISTS. S8's cost is the size of the move, and that size was written
# down as a bare integer in prose: "~9,300 LOC" in August 2026, re-derived as
# "~15,800" a month later, with nothing in between able to notice the drift. The
# figure moves every time anyone adds a line to the harness, so it is a
# MEASUREMENT and belongs in a command, not in a sentence. Run this instead of
# quoting a number.
#
# WHAT IT COUNTS, exactly: physical lines (`wc -l`, blanks and comments included)
# of the `crates/sigil-harness/src/*.rs` modules, grouped into the three seams the
# ledger row names. Modules the seam plan does NOT name are listed separately
# rather than silently folded in or silently dropped: the plan predates them, and
# that unassigned remainder is the part of the crate the split has no home for
# yet. `src/bin` and `tests/` are reported as context, not as part of the move.
#
# WHAT IT IS NOT. Not a gate. Nothing asserts a number here, because a number
# that grows with ordinary work would go red on correct code. It is a report; it
# exits nonzero only when it CANNOT measure (a named module has moved out from
# under the seam table), because a size report that quietly counts eight of nine
# modules is worse than no report.
#
# Exit: 0 measured / 2 could not measure.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$HERE/../crates/sigil-harness/src"

# The seam table, transcribed from the S8 ledger row. Editing the plan means
# editing this table, and the run then re-prices the plan you actually have.
SEAM_BUILD=(native map_placement contract_baseline seam1 seam2)
SEAM_PINS=(pins)
SEAM_TESTKIT=(test_support repin provenance)

missing=()
total=0

seam_total() {
    local label="$1"; shift
    local sum=0 mod path lines
    printf '%s\n' "$label"
    for mod in "$@"; do
        path="$SRC/$mod.rs"
        if [[ ! -f "$path" ]]; then
            printf '  %-22s MISSING  (%s)\n' "$mod" "$path"
            missing+=("$mod")
            continue
        fi
        lines=$(wc -l < "$path")
        printf '  %-22s %6d\n' "$mod" "$lines"
        sum=$((sum + lines))
    done
    printf '  %-22s %6d\n\n' "-- seam total" "$sum"
    total=$((total + sum))
}

printf 'S8 seam sizing, measured %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'tree: %s\n' "$(cd "$HERE/.." && git rev-parse --short HEAD 2>/dev/null || echo '(not a git tree)')"
printf 'counting: physical lines of crates/sigil-harness/src/<module>.rs\n\n'

seam_total 'sigil-build   <- ' "${SEAM_BUILD[@]}"
seam_total 'sigil-pins    <- ' "${SEAM_PINS[@]}"
seam_total 'sigil-testkit <- ' "${SEAM_TESTKIT[@]}"

printf 'THE MOVE: %d lines across %d modules.\n\n' \
    "$total" "$((${#SEAM_BUILD[@]} + ${#SEAM_PINS[@]} + ${#SEAM_TESTKIT[@]}))"

# Everything in src/ the seam table does not claim. Not part of the move as
# planned, and the number worth watching: it is the crate the split would leave
# behind, and the plan has never been re-cut against it.
named=" ${SEAM_BUILD[*]} ${SEAM_PINS[*]} ${SEAM_TESTKIT[*]} lib "
unassigned_total=0
printf 'UNASSIGNED by the seam plan (stays in sigil-harness, or wants a fourth seam):\n'
for path in "$SRC"/*.rs; do
    mod="$(basename "$path" .rs)"
    [[ "$named" == *" $mod "* ]] && continue
    lines=$(wc -l < "$path")
    printf '  %-22s %6d\n' "$mod" "$lines"
    unassigned_total=$((unassigned_total + lines))
done
lib_lines=$(wc -l < "$SRC/lib.rs" 2>/dev/null || echo 0)
printf '  %-22s %6d  (the module tree itself; rewritten by any split)\n' "lib" "$lib_lines"
printf '  %-22s %6d\n\n' "-- unassigned total" "$((unassigned_total + lib_lines))"

printf 'CONTEXT (not part of the move):\n'
printf '  %-22s %6d\n' "src/*.rs whole crate" "$(cat "$SRC"/*.rs | wc -l)"
printf '  %-22s %6d\n' "src/bin/*.rs" "$(cat "$SRC"/bin/*.rs 2>/dev/null | wc -l)"
printf '  %-22s %6d\n' "tests/*.rs" "$(cat "$HERE/../crates/sigil-harness/tests"/*.rs 2>/dev/null | wc -l)"

if (( ${#missing[@]} )); then
    printf '\nCOULD NOT MEASURE: %d module(s) named by the seam table are absent: %s\n' \
        "${#missing[@]}" "${missing[*]}" >&2
    printf 'The seam plan and the crate have diverged. Re-cut the table above before\n' >&2
    printf 'trusting any figure this run printed.\n' >&2
    exit 2
fi
exit 0
