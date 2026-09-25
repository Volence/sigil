#!/usr/bin/env bash
S=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-adae49c7bddc9a0a8
export CARGO_TARGET_DIR=$S/target AEON_DIR=/home/volence/sonic_hacks/.aeon-bcd-pcindex
export SIGIL_EMIT=$S/target/release/emit_sound_blob
cd "$W" || exit 9
echo "pwd=$(pwd) HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current) status=[$(git status --porcelain | wc -l) lines]"
echo "AEON_DIR=$AEON_DIR aeon HEAD $(git -C $AEON_DIR rev-parse HEAD)"
echo "== repin --check"
cargo run --release -p sigil-harness --bin repin -- --check 2>&1 | tail -3
echo "REPIN_RC=${PIPESTATUS[0]}"
echo "== workspace tests"
cargo test --release --workspace --no-fail-fast > "$S/logs/suite-raw.log" 2>&1
echo "SUITE_RC=$?"
grep -E '^test result' "$S/logs/suite-raw.log" | awk '{p+=$4; f+=$6; i+=$8; n++} END {print "result lines:", n, "passed:", p, "failed:", f, "ignored:", i}'
grep -E '^test .* FAILED$|^    [a-z_:0-9]+$' "$S/logs/suite-raw.log" | sort -u
grep -c 'as_bcd_pcindex' "$S/logs/suite-raw.log"
echo "== clippy"
cargo clippy --release --workspace --all-targets -- -D warnings > "$S/logs/clippy.log" 2>&1
echo "CLIPPY_RC=$?"
echo SUITE_END
