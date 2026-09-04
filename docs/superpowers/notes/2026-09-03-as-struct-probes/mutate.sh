#!/usr/bin/env bash
# Red-first: apply one mutation to the COMMITTED baseline, show it landed by
# reading the patched line back OFF DISK, run the sweep, restore, verify clean.
set -uo pipefail
W=/home/volence/sonic_hacks/.sigil-struct
F=$W/crates/sigil-frontend-as/src/eval.rs
name=$1; old=$2; new=$3
git -C "$W" checkout -- "$F"
echo "=================== MUTATION: $name"
python3 - "$F" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
t = open(p).read()
n = t.count(old)
if n != 1:
    print("MUTATION DID NOT APPLY: %d occurrences of the anchor" % n); raise SystemExit(9)
open(p,'w').write(t.replace(old, new))
PY
[[ $? -ne 0 ]] && { echo "ABORT: patch did not apply"; exit 9; }
echo "--- the patched line, read back from disk:"
grep -nF -- "$new" "$F" | head -3
"$W/.probe/as/sweep.sh"
# RESTORE THE BINARY TOO. Restoring only the source leaves the MUTATED
# assembler installed at .target-land/release/sigil, and every probe run
# afterwards silently measures it instead of the tree.
git -C "$W" checkout -- "$F"
CARGO_TARGET_DIR=$W/.target-land cargo build --release --manifest-path "$W/Cargo.toml" --bin sigil >/dev/null 2>&1
echo "--- restored; tree clean for that file: $(git -C "$W" status --porcelain -- "$F" | wc -l) entries"
