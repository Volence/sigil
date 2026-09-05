#!/usr/bin/env bash
# THE REFUSAL CONTROL THAT COULD NOT BE BUILT, and its own refutation, kept
# together because the attempt is the finding.
#
# The stronger positive control would be: inject a macro-body label REFERENCED
# FROM OUTSIDE into a copy of the reference tree and watch the aeon build refuse
# it BY NAME. It cannot be built. Aeon's AS residual root is declared to emit
# nothing, and ANY PC label in it opens a section, so the build stops at
# `[layout.undeclared-alignment]` before symbol resolution is reached. Two
# injections were tried — a `dc.w` read and a byte-free `equ` read — and both
# produced that same red, which is not the red under test.
#
# Step 2 is what turns that from a guess into a measurement: a BARE FILE-LEVEL
# label, no macro anywhere, nothing this parcel touches, produces the identical
# diagnostic. So the refusal path of the macro-body-label rule is unreachable in
# the aeon build BY CONSTRUCTION, not merely absent from the source — and a red
# from step 1 could never have been read as evidence for the rule.
#
# The refusal evidence therefore lives entirely in
# `crates/sigil-frontend-as/tests/as_macro_body_label.rs` and its mutation gate.
# The aeon-side control that CAN be built is `poscontrol.sh`, which proves the
# census instrument fires on a definition.
#
#   ./poscontrol-refuse.sh <sigil-binary> <mutable-copy-of-aeon> <pristine-aeon> <out-dir>
#
# PUT THE COPY OUTSIDE THE SIGIL WORKTREE — see poscontrol.sh.
set -uo pipefail
SIGIL="$1"; MUT="$2"; REF="$3"; OUT="$4"
mkdir -p "$OUT"
ROOT="$MUT/games/sonic4/game_root.asm"

inject() {  # $1 = the lines to splice in above the carrier equ
    python3 - "$ROOT" "$1" <<'PY'
import sys
p, text = sys.argv[1], sys.argv[2]
s = open(p).read()
anchor = "__Aeon_AS_Carrier:  equ 0\n"
assert s.count(anchor) == 1, f"anchor matched {s.count(anchor)} times, expected 1"
open(p, "w").write(s.replace(anchor, text + anchor))
print("injected")
PY
}

restore() { git -C "$MUT" checkout -- games/sonic4/game_root.asm 2>/dev/null \
    || cp "$REF/games/sonic4/game_root.asm" "$ROOT"; }

echo "################ STEP 1 — the control as it would be written"
restore
# Invoked TWICE, so this is also the shape the pre-parcel binary refused for a
# DIFFERENT reason (`symbol redefined by section` from the linker). The read is
# an `equ` rather than a `dc.w` so that it emits no bytes — the `dc.w` version
# was tried first and its red was the emits-nothing contract, not the symbol.
inject 'SigilRefCtl macro
SigilCtlInner:
    endm
    SigilRefCtl
    SigilRefCtl
SigilCtlRef:  equ SigilCtlInner
'
grep -n 'SigilRefCtl\|SigilCtlInner\|SigilCtlRef' "$ROOT"
AEON_DIR="$MUT" "$SIGIL" build --aeon "$MUT" --game sonic4 -o "$OUT/mut.bin" 2>&1 \
    | grep -iE 'error' | head -3
echo "BUILD_EXIT=$(AEON_DIR="$MUT" "$SIGIL" build --aeon "$MUT" --game sonic4 >/dev/null 2>&1; echo $?)"

echo "################ STEP 2 — WHY that red is not evidence"
echo "A bare FILE-LEVEL label, no macro anywhere. If this reds the same way, the"
echo "step-1 red is about PC labels in the residual and not about this rule."
restore
inject 'SigilPlainCtl:
'
grep -n 'SigilPlainCtl' "$ROOT"
AEON_DIR="$MUT" "$SIGIL" build --aeon "$MUT" --game sonic4 -o "$OUT/plain.bin" 2>&1 \
    | grep -iE 'error' | head -3
echo "BUILD_EXIT=$(AEON_DIR="$MUT" "$SIGIL" build --aeon "$MUT" --game sonic4 >/dev/null 2>&1; echo $?)"

echo "################ STEP 3 — the unmutated tree (must be green)"
restore
AEON_DIR="$REF" "$SIGIL" build --aeon "$REF" --game sonic4 -o "$OUT/ref.bin" 2>&1 \
    | grep -E 'built|error' | head -3
echo "BUILD_EXIT=$(AEON_DIR="$REF" "$SIGIL" build --aeon "$REF" --game sonic4 >/dev/null 2>&1; echo $?)"
