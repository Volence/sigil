#!/usr/bin/env bash
# Red-first against the UNIT TESTS: apply one mutation to the committed
# baseline, show it landed from disk, run the suite, restore.
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
    print("MUTATION DID NOT APPLY: %d occurrences" % n); raise SystemExit(9)
open(p,'w').write(t.replace(old, new))
PY
rc=$?; [[ $rc -ne 0 ]] && { echo "ABORT: patch did not apply"; exit 9; }
echo "--- patched line, read back from disk:"
grep -nF -- "${new##*$'\n'}" "$F" | head -2
CARGO_TARGET_DIR=$W/.target-land cargo test --release --manifest-path "$W/Cargo.toml" \
  -p sigil-frontend-as --lib 2>&1 | grep -E '^test .*(FAILED|ok)$|test result:' | grep -vE '\.\.\. ok$'
# RESTORE THE BINARY TOO. Restoring only the source leaves the MUTATED
# assembler installed at .target-land/release/sigil, and every probe run
# afterwards silently measures it instead of the tree.
git -C "$W" checkout -- "$F"
CARGO_TARGET_DIR=$W/.target-land cargo build --release --manifest-path "$W/Cargo.toml" --bin sigil >/dev/null 2>&1
echo "--- restored: $(git -C "$W" status --porcelain -- "$F" | wc -l) dirty entries"
