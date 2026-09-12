#!/usr/bin/env bash
# Dump every probe's sections (front end, then after link-time placement) from the
# tree as it stands. The instrument test is copied in, run, and removed again.
# Usage: dump.sh <outfile>
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ab5e0036e93441809
S=/home/volence/sonic_hacks/.scratch/second-space-pin
export CARGO_TARGET_DIR=$S/target
cd "$W" || exit 2
T=crates/sigil-frontend-as/tests/zz_pin_probe_dump.rs
cp "$S/zz_pin_probe_dump.rs" "$T"
echo "HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current)"
echo "tracked status: [$(git status --porcelain --untracked-files=no | tr '\n' ' ')]"
/usr/bin/grep -n "MUTATION" crates/sigil-frontend-as/src/eval.rs || echo "no MUTATION marker in eval.rs"
PROBE_DIR=$S/probes DUMP_OUT=$1 cargo test --release -p sigil-frontend-as --test zz_pin_probe_dump 2>&1 | /usr/bin/grep -E "Compiling sigil-frontend-as|^test result|panicked|error"
rm -f "$T"
echo "instrument removed: $([[ -e $T ]] && echo NO || echo yes)"
