#!/usr/bin/env bash
# Em-dash / en-dash scan over the lines THIS BRANCH ADDS, with a CANARY that
# MUST come back non-zero before any zero is believed.
#
# Two parcels on 2026-09-05 had a dash scan return 0 for every file AND 0 for a
# string containing a dash, so their clean sweep meant nothing. This runs the
# same pattern over a string that DOES contain both characters first and
# refuses to report unless that canary fires.
#
# ADDED LINES, not whole files, and that is the load-bearing choice. This
# repository's existing prose uses em dashes throughout (`eval.rs` alone holds
# 669 of them), so a whole-file scan reports a four-figure number that is all
# inherited and says nothing about what this branch wrote. `git diff <base>`
# with a leading `+` filter is the population the rule is actually about.
#
# Usage: dashscan.sh <base-rev>
set -u

PAT=$'—|–'   # em dash, en dash

canary=$'ok — and – here'
n=$(printf '%s\n' "$canary" | /usr/bin/grep -cE "$PAT" || true)
if [ "${n:-0}" -eq 0 ]; then
    echo "CANARY FAILED: the pattern does not match a string containing both dashes."
    echo "This scan can prove nothing. Fix the pattern before believing any zero."
    exit 2
fi
echo "canary: $n line(s) matched (must be >= 1) -- the scan can fire"

base="${1:?usage: dashscan.sh <base-rev>}"
# BRE, not ERE: `+` is a literal in a basic regex, and `\+` in an EXTENDED one
# is a stray escape that GNU grep warns about and then treats unpredictably.
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
git diff "$base" | /usr/bin/grep '^+' | /usr/bin/grep -v '^+++' > "$tmp" || true
total=$(wc -l < "$tmp")
hits=$(/usr/bin/grep -cE "$PAT" "$tmp" || true)

echo "added lines scanned: $total"
if [ "$total" -eq 0 ]; then
    echo "REFUSED: the population is EMPTY, so a zero here would examine nothing."
    exit 2
fi
if [ "${hits:-0}" -ne 0 ]; then
    echo "DASHES: $hits added line(s)"
    /usr/bin/grep -nE "$PAT" "$tmp" | head -40
    exit 1
fi
echo "clean: 0 dashes in $total added lines"
