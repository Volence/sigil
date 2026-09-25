#!/usr/bin/env bash
# red_base.sh: the committed new test file, run against the BASE implementation
# (git archive of 1e146771), own target dir. It must fail, naming the cases.
D=/home/volence/sonic_hacks/.scratch/as-squote/red-base
cd "$D" || exit 1
{
  echo "pwd=$(pwd) base tree = git archive 1e146771; test file = parcel HEAD's tests/as_single_quoted_string.rs"
  md5sum crates/sigil-frontend-as/tests/as_single_quoted_string.rs crates/sigil-frontend-as/src/lexer.rs
  CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/as-squote/target-red cargo test --release -p sigil-frontend-as --test as_single_quoted_string 2>&1
  echo "TEST_END rc=$?"
} > /home/volence/sonic_hacks/.scratch/as-squote/logs/red-base.log 2>&1
