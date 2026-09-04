#!/bin/bash
# Red-first proof for the `~~` gates. Each mutation is applied to a COMMITTED
# baseline, READ BACK FROM DISK so the patch is shown to have landed, run, and
# then restored with `git checkout --` from that same committed baseline.
# A mutation that fails to apply would run the original file and print ok --
# indistinguishable from a clean restore -- so the read-back is the proof.
set -u
W=/home/volence/sonic_hacks/.wt-tilde
export CARGO_TARGET_DIR=/home/volence/sonic_hacks/.tilde-target
cd "$W" || exit 1

run() {
  local name=$1 file=$2 old=$3 new=$4 grepfor=$5
  echo "======================================================================"
  echo "MUTATION: $name   ($file)"
  git checkout -- "$file"
  python3 - "$file" "$old" "$new" <<'PY'
import sys
p,old,new=sys.argv[1],sys.argv[2],sys.argv[3]
s=open(p).read()
if old not in s:
    print("!! MUTATION DID NOT APPLY: anchor absent"); sys.exit(3)
open(p,'w').write(s.replace(old,new,1))
PY
  if [[ $? -ne 0 ]]; then echo "ABORT: mutation did not apply"; git checkout -- "$file"; return 1; fi
  echo "--- applied patch, read back from disk: ---"
  git diff --unified=0 -- "$file" | grep -E '^[-+][^-+]' | sed 's/^/    /'
  if ! git diff --quiet -- "$file"; then echo "    (file is dirty: patch landed)"; else echo "!! FILE CLEAN — patch did NOT land"; return 1; fi
  echo "--- cargo test -p sigil-frontend-as --lib $grepfor ---"
  cargo test --release -p sigil-frontend-as --lib "$grepfor" 2>&1 | grep -E '^test result:|^error(\[|:)|FAILED|panicked at' | head -6
  git checkout -- "$file"
  git diff --quiet -- "$file" && echo "restored: clean" || echo "!! RESTORE FAILED"
}

run "lexer: un-munch ~~ back into two Tilde" \
  crates/sigil-frontend-as/src/lexer.rs \
  "        Some((b'~', b'~')) => return Some((TildeTilde, 2))," \
  "" \
  ""

run "fold: LogNot as bitwise complement (the original defect)" \
  crates/sigil-ir/src/expr.rs \
  "UnOp::LogNot => Fold::Value(i64::from(v == 0))," \
  "UnOp::LogNot => Fold::Value(!v)," \
  ""

run "fold: LogNot inverted (v != 0)" \
  crates/sigil-ir/src/expr.rs \
  "UnOp::LogNot => Fold::Value(i64::from(v == 0))," \
  "UnOp::LogNot => Fold::Value(i64::from(v != 0))," \
  ""

run "expand: render ~~ back as a single ~" \
  crates/sigil-frontend-as/src/expand.rs \
  '        Punct::TildeTilde => "~~",' \
  '        Punct::TildeTilde => "~",' \
  ""

run "parser: LogNot takes a full binary expression, not an atom" \
  crates/sigil-frontend-as/src/expr.rs \
  "        Tok::Punct(Punct::TildeTilde) => {
            let (inner, r) = parse_atom(rest, depth)?;" \
  "        Tok::Punct(Punct::TildeTilde) => {
            let (inner, r) = parse_bp(rest, 0, depth)?;" \
  ""

echo "======================================================================"
echo "final tree state:"; git status --short
