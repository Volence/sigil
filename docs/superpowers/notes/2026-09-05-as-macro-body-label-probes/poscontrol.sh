#!/usr/bin/env bash
# THE POSITIVE CONTROL. The census returned ZERO for all four aeon shapes, and a
# zero from an instrument that never fires is indistinguishable from a zero from
# a clean tree. So: inject a macro-body label into a COPY of the reference tree,
# show it on disk, and confirm the census names it — then confirm the unmutated
# tree is still zero.
#
#   ./poscontrol.sh <sigil-binary> <mutable-copy-of-aeon> <pristine-aeon> <out-dir>
#
# PUT THE COPY OUTSIDE THE SIGIL WORKTREE. `scripts_name_their_tree` and the
# drift harness scan the whole tree for a single `tools/suite_paths.py`, and a
# second aeon tree anywhere under it makes both refuse with COULD NOT MEASURE —
# a failure that looks nothing like its cause.
set -uo pipefail
SIGIL="$1"; MUT="$2"; REF="$3"; OUT="$4"
mkdir -p "$OUT"
ROOT="$MUT/games/sonic4/game_root.asm"

python3 - "$ROOT" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
anchor = "__Aeon_AS_Carrier:  equ 0\n"
assert s.count(anchor) == 1, f"anchor matched {s.count(anchor)} times, expected 1"
inject = (
    "SigilExpLabelCtl macro\n"
    "SigilCtlBody:\n"
    "    endm\n"
    "    SigilExpLabelCtl\n"
)
open(p, "w").write(s.replace(anchor, inject + anchor))
print("injected")
PY

echo "=== the mutation, shown on disk:"
grep -n 'SigilExpLabelCtl\|SigilCtlBody' "$ROOT"

echo "=== census over the MUTATED tree (MUST name SigilCtlBody):"
SIGIL_CENSUS_EXPLABEL=1 AEON_DIR="$MUT" "$SIGIL" build --aeon "$MUT" --game sonic4 \
    -o "$OUT/mut.bin" 2>&1 >/dev/null | grep 'CENSUS-EXPLABEL' | grep -v 'instances-with-labels=0'
echo "MUT_HITS=$(SIGIL_CENSUS_EXPLABEL=1 AEON_DIR="$MUT" "$SIGIL" build --aeon "$MUT" --game sonic4 -o "$OUT/mut.bin" 2>&1 >/dev/null | grep -c 'CENSUS-EXPLABEL	SigilCtlBody')"

echo "=== census over the PRISTINE tree (must be zero):"
SIGIL_CENSUS_EXPLABEL=1 AEON_DIR="$REF" "$SIGIL" build --aeon "$REF" --game sonic4 \
    -o "$OUT/ref.bin" 2>&1 >/dev/null | grep 'CENSUS-EXPLABEL' | grep -v 'instances-with-labels=0'
echo "REF_NAMED=$(SIGIL_CENSUS_EXPLABEL=1 AEON_DIR="$REF" "$SIGIL" build --aeon "$REF" --game sonic4 -o "$OUT/ref.bin" 2>&1 >/dev/null | grep -c '	depth=')"
