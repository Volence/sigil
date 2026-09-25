#!/usr/bin/env bash
# mutate.sh: red-first proofs. Each mutation is applied to the committed tree,
# shown on disk with git diff, the two runners are run, the failing test names
# and first assertion text are quoted, and the file is restored from HEAD.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d
S=/home/volence/sonic_hacks/.scratch/as-cli-define
export CARGO_TARGET_DIR=$S/target
cd "$W" || exit 1
echo "pwd=$(pwd) HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current)"
if [ -n "$(git status --porcelain -- crates)" ]; then echo "tree not clean under crates, refusing"; exit 1; fi

mutate() {
  local name="$1" file="$2" old="$3" new="$4"
  echo
  echo "=================== $name"
  python3 - "$file" "$old" "$new" <<'EOF'
import sys
p,old,new=sys.argv[1:]
s=open(p).read()
assert s.count(old)==1, f"anchor not unique in {p}: {s.count(old)}"
open(p,'w').write(s.replace(old,new))
EOF
  echo "--- on disk:"
  git diff -- "$file"
  echo "--- runner 1: cargo test -p sigil-cli --test as_cli_define"
  cargo test --release -q -p sigil-cli --test as_cli_define 2>&1 | grep -A3 -E "panicked at" | grep -vE '^--$|RUST_BACKTRACE' | head -16; cargo test --release -q -p sigil-cli --test as_cli_define 2>&1 | grep "test result"
  echo "--- runner 2: cargo test -p sigil-frontend-as --lib cli_define"
  cargo test --release -q -p sigil-frontend-as --lib cli_define 2>&1 | grep -A3 -E "panicked at" | grep -vE '^--$|RUST_BACKTRACE' | head -10; cargo test --release -q -p sigil-frontend-as --lib cli_define 2>&1 | grep "test result"
  git checkout -q HEAD -- "$file"
  echo "--- restored: $(git status --porcelain -- "$file" | wc -l) changed path(s)"
}

mutate "M1 a bare -D defaults to 0" crates/sigil-frontend-as/src/cli_define.rs \
  'None | Some("") => 1,' 'None | Some("") => 0,'
mutate "M2 the define enters with no class, so = / equ / a label silently win" crates/sigil-frontend-as/src/eval.rs \
  '            self.sym_class.insert(name, SymClass::Var);
' ''
mutate "M3 bound on the first pass only, carried after" crates/sigil-frontend-as/src/eval.rs \
  '    asm.seed_cli_defines(&opts.cli_defines);' '    if mompass == FIRST_PASS { asm.seed_cli_defines(&opts.cli_defines); }'
mutate "M4 a repeated name keeps its LAST value" crates/sigil-frontend-as/src/cli_define.rs \
  'defines.iter().filter(|(k, _)| seen.insert(k.as_str()))' 'defines.iter().rev().filter(|(k, _)| seen.insert(k.as_str()))'
mutate "M5 bound in env only, not recorded as defined this pass" crates/sigil-frontend-as/src/eval.rs \
  "        for (name, value) in crate::cli_define::first_wins(defines) {
            self.define_sym(&name, SymbolValue::Int(value));" "        for (name, value) in crate::cli_define::first_wins(defines) {
            self.env.define(&name, SymbolValue::Int(value));"
mutate "M6 a leading-dot name accepted" crates/sigil-frontend-as/src/cli_define.rs \
  "    if name.starts_with('.') {" "    if name.starts_with('.') && false {"
echo
echo "final: $(git status --porcelain -- crates | wc -l) changed path(s) under crates"
echo MUTATE_END
