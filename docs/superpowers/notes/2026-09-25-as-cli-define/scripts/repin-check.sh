#!/usr/bin/env bash
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d
cd "$W" || exit 1
echo "pwd=$(pwd) HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current)"
AEON_DIR="/home/volence/sonic_hacks/.aeon-cli-define" \
SIGIL_EMIT="/home/volence/sonic_hacks/.scratch/as-cli-define/target-prov/release/emit_sound_blob" \
CARGO_TARGET_DIR="/home/volence/sonic_hacks/.scratch/as-cli-define/target-prov" \
  cargo run --release -p sigil-harness --bin repin -- --check
echo "REPIN_END rc=$?"
