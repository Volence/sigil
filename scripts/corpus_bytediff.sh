#!/usr/bin/env bash
# corpus_bytediff.sh — byte-diff probe: this worktree's sigil binary vs
# pristine master's sigil binary, run on the SAME source files (the
# worktree's copies of examples/*.emp and the two standing game
# invocations), building each tree's own binary first.
#
# Usage: scripts/corpus_bytediff.sh
#
# Exit status: 0 if every buildable file is byte-identical between the two
# binaries; nonzero if any file DIFFERS. SKIPPED files (master's binary
# fails to compile them) do not affect the exit status.
set -u

# This worktree (where this script lives).
WORKTREE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Pristine master checkout — the main checkout this worktree was created from.
MASTER_DIR="/home/volence/sonic_hacks/sigil"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "== building worktree sigil-cli =="
( cd "$WORKTREE_DIR" && cargo build -p sigil-cli ) || {
    echo "FATAL: worktree cargo build -p sigil-cli failed"
    exit 1
}
WORKTREE_BIN="$WORKTREE_DIR/target/debug/sigil"

echo "== building master sigil-cli =="
( cd "$MASTER_DIR" && cargo build -p sigil-cli ) || {
    echo "FATAL: master cargo build -p sigil-cli failed"
    exit 1
}
MASTER_BIN="$MASTER_DIR/target/debug/sigil"

any_diff=0

# verdict FILE_LABEL WORKTREE_EXIT MASTER_EXIT WORKTREE_OUT MASTER_OUT
verdict() {
    label="$1"
    wt_exit="$2"
    m_exit="$3"
    wt_out="$4"
    m_out="$5"

    if [ "$m_exit" -ne 0 ]; then
        echo "SKIPPED  $label (master's binary failed to compile it)"
        return
    fi
    if [ "$wt_exit" -ne 0 ]; then
        echo "DIFFERS  $label (worktree binary failed to compile it; master succeeded)"
        any_diff=1
        return
    fi
    if cmp -s "$wt_out" "$m_out"; then
        echo "IDENTICAL $label"
    else
        echo "DIFFERS  $label"
        any_diff=1
    fi
}

echo "== single-file examples (examples/*.emp) =="
for f in "$WORKTREE_DIR"/examples/*.emp; do
    base="$(basename "$f")"
    wt_out="$TMP_DIR/wt.$base.bin"
    m_out="$TMP_DIR/m.$base.bin"

    "$WORKTREE_BIN" emp "$f" -o "$wt_out" >"$TMP_DIR/wt.$base.log" 2>&1
    wt_exit=$?
    "$MASTER_BIN" emp "$f" -o "$m_out" >"$TMP_DIR/m.$base.log" 2>&1
    m_exit=$?

    verdict "$base" "$wt_exit" "$m_exit" "$wt_out" "$m_out"
done

echo "== game invocations (--root examples/game --prelude prelude) =="
GAME_ROOT="$WORKTREE_DIR/examples/game"
for f in \
    "$WORKTREE_DIR/examples/game/badniks/pitcher_plant.emp" \
    "$WORKTREE_DIR/examples/game/badniks/pitcher_plant_script.emp"
do
    base="$(basename "$f")"
    wt_out="$TMP_DIR/wt.$base.bin"
    m_out="$TMP_DIR/m.$base.bin"

    "$WORKTREE_BIN" emp "$f" --root "$GAME_ROOT" --prelude prelude -o "$wt_out" \
        >"$TMP_DIR/wt.$base.log" 2>&1
    wt_exit=$?
    "$MASTER_BIN" emp "$f" --root "$GAME_ROOT" --prelude prelude -o "$m_out" \
        >"$TMP_DIR/m.$base.log" 2>&1
    m_exit=$?

    verdict "$base" "$wt_exit" "$m_exit" "$wt_out" "$m_out"
done

if [ "$any_diff" -ne 0 ]; then
    echo "RESULT: DIFFERENCES FOUND"
    exit 1
fi
echo "RESULT: all identical (SKIPPED files, if any, excluded)"
exit 0
