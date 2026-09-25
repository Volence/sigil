#!/usr/bin/env bash
# redfirst_base.sh: run this parcel's tests against the BASE implementation (120be609
# archive) with only the tests copied in; they must fail, naming the lexer refusal.
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
LOG=$S/logs/redfirst-base.log
python3 "$S/redfirst_base.py"
cd "$S/base-src" || exit 9
{
  echo "pwd=$(pwd) tree=git archive 120be609 (tar md5 $(md5sum < "$S/base-120be609.tar" | cut -c1-32)) plus the tests only"
  echo "eval.rs identical to base archive: $(tar -xOf "$S/base-120be609.tar" crates/sigil-frontend-as/src/eval.rs | cmp -s - crates/sigil-frontend-as/src/eval.rs && echo yes || echo NO)"
  touch crates/sigil-frontend-as/src/lexer.rs
  CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-frontend-as --test as_dollar_labels --no-fail-fast 2>&1
  CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-frontend-as --lib a_temporary_symbol 2>&1
} > "$LOG"
echo REDFIRST_END >> "$LOG"
grep -E '^test |test result|REDFIRST_END|identical' "$LOG"
