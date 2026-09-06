#!/usr/bin/env bash
# sweep.sh -- put the OLD and the NEW assembler to every source file in four
# trees and compare BYTES, EVERY DIAGNOSTIC and the EXIT CODE, per file.
#
#   ./sweep.sh <old sigil> <new sigil> [dir ...]
#
# A file neither tool assembles is still compared.  What is measured is that
# the two tools AGREE, not that either succeeds.
#
# ── THE START GUARD, and why the run refuses without it ──────────────────────
# A sweep reporting "N files, 0 differ" is exactly as consistent with having
# run ONE binary twice as it is with a null result.  Nothing downstream can
# tell those apart, and the second reads like work.  So before the loop this
# script assembles a WITNESS -- the exact divergent shape, `dc.l Parent.lq`
# after a `set` -- and REFUSES TO START unless the two binaries answer it
# differently: the old one must ACCEPT it (the symbol asl calls undefined) and
# the new one must REFUSE it.  That check is what makes a run of identical
# results mean something.
#
# The guard is written against the DEFECT, not against a version string or a
# digest: two binaries with different md5s can still be the same assembler.
#
# ── AND WHAT THE ARMS PRODUCED, which the guard does NOT answer ──────────────
# The guard proves the two binaries are different programs.  It says nothing
# about whether they were put to anything.  So the totals below also report
# how many files each arm ACCEPTED and how many BYTES it emitted, because two
# arms failing identically upstream are indistinguishable from two arms
# passing, and the counts are the only thing that tells them apart.
#
# ON THIS CORPUS THEY DO NOT PASS.  These trees are include FRAGMENTS: 1751 of
# 1753 are refused by BOTH arms and the 2 accepted emit ZERO bytes, so what
# the sweep compares here is 43,862 DIAGNOSTIC LINES, not an image.  That is a
# real comparison -- a diagnostic names the symbol it could not resolve, and
# moving a local to a different parent renames it -- but it is NOT a byte
# measurement, and reading it as one would be reading agreement as evidence.
# The byte half is the aeon ROM A/B, in the note beside this script.
set -uo pipefail
cd "$(dirname "$0")" || exit 2

OLD="${1:?usage: sweep.sh <old sigil> <new sigil> [dir ...]}"
NEW="${2:?usage: sweep.sh <old sigil> <new sigil> [dir ...]}"
shift 2
DIRS=("$@")
if [ ${#DIRS[@]} -eq 0 ]; then
    echo "FATAL: no trees named" >&2
    exit 2
fi

for b in "$OLD" "$NEW"; do
    [ -x "$b" ] || { echo "FATAL: no executable sigil at $b" >&2; exit 2; }
done
echo "OLD md5 $(md5sum "$OLD" | cut -d' ' -f1)   $OLD"
echo "NEW md5 $(md5sum "$NEW" | cut -d' ' -f1)   $NEW"

# ── the witness ──────────────────────────────────────────────────────────────
WIT=$(mktemp -d)
trap 'rm -rf "$WIT"' EXIT
cat > "$WIT/witness.asm" <<'EOF'
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
Var	set	5
.lq:
	nop
	dc.l	Parent.lq
	end
EOF
"$OLD" "$WIT/witness.asm" --hex >/dev/null 2>&1; old_rc=$?
"$NEW" "$WIT/witness.asm" --hex >/dev/null 2>&1; new_rc=$?
echo "witness: OLD rc=$old_rc  NEW rc=$new_rc  (dc.l Parent.lq after a \`set\`)"
if [ "$old_rc" -ne 0 ] || [ "$new_rc" -eq 0 ]; then
    echo "REFUSED TO START: the two binaries answer the witness the same way." >&2
    echo "  OLD must ACCEPT \`Parent.lq\` (rc 0) and NEW must REFUSE it (rc != 0)." >&2
    echo "  Without that, a sweep of identical results is indistinguishable from" >&2
    echo "  one binary compared with itself." >&2
    exit 2
fi
echo "witness: OLD accepts the symbol asl calls undefined, NEW refuses it -- two distinct tools"
echo

total=0
differ=0
accepted_old=0
accepted_new=0
bytes_old=0
bytes_new=0
diaglines=0
declare -a DIFFS=()
for dir in "${DIRS[@]}"; do
    if [ ! -d "$dir" ]; then
        echo "$dir  UNMEASURABLE: not a directory -- reported, not counted as zero"
        continue
    fi
    n=0
    d=0
    while IFS= read -r f; do
        n=$((n + 1))
        a_out=$("$OLD" "$f" --hex 2>&1); a_rc=$?
        b_out=$("$NEW" "$f" --hex 2>&1); b_rc=$?
        if [ "$a_rc" != "$b_rc" ] || [ "$a_out" != "$b_out" ]; then
            d=$((d + 1))
            DIFFS+=("$f  (rc $a_rc -> $b_rc)")
        fi
        if [ "$a_rc" -eq 0 ]; then
            accepted_old=$((accepted_old + 1))
            bytes_old=$((bytes_old + $(wc -w <<< "$a_out")))
        fi
        if [ "$b_rc" -eq 0 ]; then
            accepted_new=$((accepted_new + 1))
            bytes_new=$((bytes_new + $(wc -w <<< "$b_out")))
        else
            diaglines=$((diaglines + $(grep -c . <<< "$b_out")))
        fi
    done < <(find "$dir" -type f \( -name '*.asm' -o -name '*.inc' \) -not -path '*/.git/*' | sort)
    printf '%-58s files=%-6s differ=%s\n' "$dir" "$n" "$d"
    total=$((total + n))
    differ=$((differ + d))
done

echo
echo "TOTAL files=$total  identical=$((total - differ))  DIFFER=$differ"
echo
echo "WHAT THE ARMS PRODUCED -- read this before reading DIFFER=0 as a result:"
echo "  OLD accepted $accepted_old of $total, emitting $bytes_old bytes"
echo "  NEW accepted $accepted_new of $total, emitting $bytes_new bytes"
echo "  diagnostic lines compared on the refused files: $diaglines"
if [ "$bytes_new" -eq 0 ]; then
    echo "  ^ NO BYTES WERE COMPARED. Every agreement above is an agreement about"
    echo "    DIAGNOSTICS. This sweep did not measure an image; do not report it as"
    echo "    a byte-identity result."
fi
if [ "$differ" -gt 0 ]; then
    echo
    echo "THE FILES THAT DIFFER:"
    printf '  %s\n' "${DIFFS[@]}"
fi
