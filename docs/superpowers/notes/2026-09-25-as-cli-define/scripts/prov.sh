#!/usr/bin/env bash
S=/home/volence/sonic_hacks/.scratch/as-cli-define
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d
cd "$W" || exit 1
echo "pwd=$(pwd) HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current)"
SIGIL_BIN=$S/target-base/release/sigil CARGO_TARGET_DIR=$S/target-prov "$W/scripts/provision-aeon-ref.sh" /home/volence/sonic_hacks/.aeon-cli-define
echo "PROVISION_END rc=$?"
