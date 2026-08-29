#!/bin/bash
# atomic_freeze.sh — the staged commit every golden freeze writes through.
#
# Sourced by capture_goldens.sh. A full capture runs longer than an agent can hold a
# foreground command open, so being killed part-way through one is an ordinary event
# rather than an exceptional one, and the write path is built for that.
#
# THE STAGING AREA is a dot-directory INSIDE the golden directory. A rename is atomic
# only within one filesystem, and the one directory guaranteed to share the goldens'
# filesystem is their own — the same reason `provenance::write_atomic` puts its
# temporary beside the ledger. Each captured ROM is copied there under its golden name;
# no committed blob is opened for writing until the whole set has been captured, and the
# set then moves into place with `mv`, which within one filesystem is rename(2).
#
# WHAT A KILL LEAVES BEHIND. Stated exactly, because the next reader will defend only
# against what this says:
#
#   * DURING CAPTURE OR STAGING — the multi-minute stretch — the committed set,
#     complete and untouched. No committed blob is open for writing at any point, so
#     none can be observed truncated.
#
#   * DURING THE COMMIT LOOP — milliseconds — no truncated blob, because a rename
#     either happened or did not. A MIXED set is still reachable: seven renames are
#     seven operations, not one. This window is NOT closed. It is made small, and it is
#     made loud: the staging area survives such a kill holding exactly the blobs that
#     did NOT land, and `freeze_open` refuses to start a fresh capture on top of one
#     rather than write a second capture over a mixture of two.
#
#   * Which of those two a leftover staging area came from is recorded by the
#     [`FREEZE_COMMIT_MARKER`] file, written before the first rename. Absent, the
#     committed set is provably the complete old one and the leftover is discardable
#     capture output. Present, the committed set may be a mixture and only a person can
#     say which blobs are wanted.
#
# That distinction rests on the order of syscalls within one process, which a kill
# respects. It is not a crash-consistency claim: a power loss may expose writes the
# kernel has not flushed in an order this file does not control.

# The staging directory's name within the golden directory, and the marker whose
# presence means the commit loop had begun.
FREEZE_STAGE_NAME=".staging"
FREEZE_COMMIT_MARKER=".committing"

# Set by freeze_open; empty means no staging area is held.
FREEZE_GOLDEN_DIR=""
FREEZE_STAGE_DIR=""

# freeze_open <golden_dir> — open a staging area for a fresh capture.
#
# Fails, before anything is built, if a previous run left one behind. That is the whole
# reason this runs first: a leftover found ten minutes later has already cost the build.
freeze_open() {
    local golden_dir="$1"
    if [[ ! -d "$golden_dir" ]]; then
        echo "ERROR: freeze_open: not a directory: $golden_dir" >&2
        return 1
    fi
    FREEZE_GOLDEN_DIR="$(cd "$golden_dir" && pwd)"
    local stage="$FREEZE_GOLDEN_DIR/$FREEZE_STAGE_NAME"
    if [[ -e "$stage" ]]; then
        if [[ -e "$stage/$FREEZE_COMMIT_MARKER" ]]; then
            echo "ERROR: a previous freeze was killed INSIDE its commit loop." >&2
            echo "       Staging area: $stage" >&2
            echo "       The committed goldens may be a MIXTURE of two captures: the blobs" >&2
            echo "       still staged below are the ones that did not land, so every other" >&2
            echo "       golden in the directory is from the newer capture." >&2
            local f
            for f in "$stage"/*; do
                [[ -e "$f" ]] && echo "         did not land: ${f##*/}" >&2
            done
            echo "       Restore the set you want (the committed blobs are tracked, so" >&2
            echo "       'git checkout -- <golden paths>' returns the whole old set), remove" >&2
            echo "       the staging area, then re-run." >&2
            FREEZE_GOLDEN_DIR=""
            return 1
        fi
        echo ">> discarding an abandoned capture at $stage" >&2
        echo "   (no commit had begun, so the committed goldens are the complete old set)" >&2
        rm -rf "$stage" || { FREEZE_GOLDEN_DIR=""; return 1; }
    fi
    mkdir -p "$stage" || { FREEZE_GOLDEN_DIR=""; return 1; }
    FREEZE_STAGE_DIR="$stage"
}

# freeze_stage <src> <golden_name> — put one captured artifact into the staging area.
freeze_stage() {
    local src="$1" name="$2"
    if [[ -z "$FREEZE_STAGE_DIR" ]]; then
        echo "ERROR: freeze_stage before freeze_open" >&2
        return 1
    fi
    if [[ "$name" == */* || "$name" == "." || "$name" == ".." || -z "$name" ]]; then
        echo "ERROR: freeze_stage: '$name' is not a golden filename" >&2
        return 1
    fi
    if [[ -d "$src" || ! -r "$src" ]]; then
        echo "ERROR: freeze_stage: not a readable file: $src" >&2
        return 1
    fi
    cp "$src" "$FREEZE_STAGE_DIR/$name"
}

# freeze_commit — move the whole staged set onto the committed goldens.
#
# The marker goes down before the first rename, so a kill anywhere in the loop is
# distinguishable from a kill during staging by anything that later finds the leftover.
freeze_commit() {
    if [[ -z "$FREEZE_STAGE_DIR" ]]; then
        echo "ERROR: freeze_commit before freeze_open" >&2
        return 1
    fi
    local staged=() f
    for f in "$FREEZE_STAGE_DIR"/*; do
        [[ -e "$f" ]] && staged+=("$f")
    done
    if [[ ${#staged[@]} -eq 0 ]]; then
        echo "ERROR: freeze_commit: nothing was staged" >&2
        return 1
    fi
    : > "$FREEZE_STAGE_DIR/$FREEZE_COMMIT_MARKER"
    for f in "${staged[@]}"; do
        mv -f "$f" "$FREEZE_GOLDEN_DIR/${f##*/}" || return 1
    done
    rm -f "$FREEZE_STAGE_DIR/$FREEZE_COMMIT_MARKER"
    rmdir "$FREEZE_STAGE_DIR" || {
        echo "ERROR: staging area not empty after commit: $FREEZE_STAGE_DIR" >&2
        return 1
    }
    FREEZE_STAGE_DIR=""
}

# freeze_abandon — drop a staging area whose capture never completed.
#
# Refuses once the commit loop has begun: past that point the staged blobs are the only
# copy of goldens the loop has not installed yet, and the leftover is the evidence that
# says the committed set may be mixed. Safe to call when nothing is held, which is what
# makes it usable from an EXIT trap.
freeze_abandon() {
    [[ -n "$FREEZE_STAGE_DIR" ]] || return 0
    if [[ -e "$FREEZE_STAGE_DIR/$FREEZE_COMMIT_MARKER" ]]; then
        echo "ERROR: not discarding $FREEZE_STAGE_DIR — the commit loop had begun and the" >&2
        echo "       staged blobs are the only copy of what it did not install." >&2
        return 1
    fi
    rm -rf "$FREEZE_STAGE_DIR"
    FREEZE_STAGE_DIR=""
}
