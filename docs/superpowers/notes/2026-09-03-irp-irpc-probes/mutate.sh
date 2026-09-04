#!/bin/bash
# Red-first proof.  Each mutation is applied to a COMMITTED baseline with sed,
# the applied patch is SHOWN from disk with `git diff`, the named tests are run,
# and the tree is restored with `git checkout`.  A mutation that comes back
# GREEN is reported as GREEN and chased, never banked.
set -u
cd /home/volence/sonic_hacks/.sigil-irpc
T=/home/volence/sonic_hacks/.sigil-irpc/.target-land
run() {
  local name=$1 file=$2 from=$3 to=$4; shift 4
  echo "════════════════════════════════════════════════════════════"
  echo "MUTATION: $name"
  git checkout -- "$file"
  python3 - "$file" "$from" "$to" <<'PY'
import sys
p,a,b = sys.argv[1],sys.argv[2],sys.argv[3]
s=open(p).read()
n=s.count(a)
assert n==1, f"mutation anchor matched {n} times, not 1 — NOT APPLIED"
open(p,'w').write(s.replace(a,b))
PY
  if [[ $? -ne 0 ]]; then echo "  !! MUTATION DID NOT APPLY — the proof would be vacuous"; return 2; fi
  echo "--- patch, read back from disk ---"
  git --no-pager diff --unified=1 -- "$file" | sed -n '4,40p'
  echo "--- test result ---"
  CARGO_TARGET_DIR=$T cargo test --release -p sigil-frontend-as --lib -- "$@" 2>&1 \
    | grep -E "^(test |test result|error\[|error:)" | tail -20
  git checkout -- "$file"
}
