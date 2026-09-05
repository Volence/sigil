#!/usr/bin/env bash
# RED-FIRST for `declock.sed` and for the stability runners that use it.
#
#     ./mutations.sh <worktree-root>
#
# Each mutation breaks the filter in ONE way and the script names, per mutation,
# what MUST go red. Three properties this checks that a naive red-first does not:
#
#   * IT PROVES THE PATCH LANDED. A `perl -0pi` that matches nothing leaves the
#     original file in place, the check runs the UNMUTATED filter and prints
#     PASS, and that is indistinguishable from a clean restore. Every mutation
#     here diffs the file against HEAD and refuses if the diff is empty.
#   * APPLIED-AND-STILL-GREEN IS A DEFECT, NOT A PASS. If the patch landed and
#     the check is green, the check does not cover the mutation; the script says
#     so in those words rather than counting it.
#   * THE RESTORE COMES FROM THE COMMITTED BASELINE (`git checkout -- <path>`),
#     not from a copy this script made, so a crash mid-run cannot leave a
#     mutated tree that looks original.
#
# Mutations m3 and m4 are the OVER-STRIPPING direction, which is the failure
# this whole directory exists to prevent: a filter that removes too much makes a
# stability check that cannot fail, reads green forever, and measures nothing.
set -uo pipefail
ROOT="${1:?usage: mutations.sh <worktree-root>}"
cd "$ROOT" || exit 2
SED_F="docs/superpowers/notes/asl-declock/declock.sed"
SC="docs/superpowers/notes/asl-declock/selfcheck.sh"
SYM_RUN="docs/superpowers/notes/2026-09-04-as-symbol-class-probes/run.sh"
SYM_STAB="docs/superpowers/notes/2026-09-04-as-symbol-class-probes/stability.sh"

restore() { git checkout -- "$SED_F" "$SYM_RUN" 2>/dev/null; }
trap restore EXIT

mutate() { # <name> <file> <must-fail> <checker> <cmd...>
    local name="$1" file="$2" must="$3" checker="$4"; shift 4
    echo
    echo "########## MUTATION: $name"
    echo "MUST FAIL: $must"
    "$@"
    if git diff --quiet -- "$file"; then
        echo "RESULT: VACUOUS — the edit did not land ($file unchanged vs HEAD)."
        echo "        A run now would exercise the ORIGINAL filter and print PASS,"
        echo "        which is indistinguishable from a clean restore. Not a pass."
        restore
        return
    fi
    echo "--- the patch, as it stands on disk ---"
    git diff --unified=0 -- "$file" | grep -E '^[-+][^-+]' | sed 's/^/    /'
    echo "--- $checker ---"
    "$checker" > /tmp/declock-mut.$$ 2>&1
    local rc=$?
    grep -E '^(FAIL|PASS  clock|PASS  a real|### totals|UNSTABLE|# unstable|m[0-9]+ )' /tmp/declock-mut.$$ \
        | sed 's/^/    /' | head -30
    rm -f /tmp/declock-mut.$$
    echo "CHECK_EXIT=$rc"
    if [[ $rc -eq 0 ]]; then
        echo "RESULT: APPLIED AND STILL GREEN — this is a RUNNER DEFECT, not a pass."
        echo "        The mutation is live and no case noticed it."
    else
        echo "RESULT: RED, as required."
    fi
    restore
}

# m1 — delete the duration rule. The clock reading that actually moves run to run
# is then back in the hashed stream.
mutate "m1 no duration rule at all" "$SED_F" \
  "selfcheck case 2 (both long forms), and case 4 (the line is still on disk after filtering)" \
  "$SC" \
  perl -0pi -e 's{^s\#\^\(\[0-9\]\+ hours.*\n}{}m' "$SED_F"

# m2 — put the inherited time rule back on its own: `NN:NN:NN` blanked, meridiem
# left standing. This is the defect this parcel found, restored.
mutate "m2 meridiem left standing (the inherited time rule)" "$SED_F" \
  "selfcheck case 1 — a clock-only pair straddling noon must otherwise read as an oracle divergence on four banner lines" \
  "$SC" \
  perl -0pi -e 's{^s\#\[0-9\]\{2\}:\[0-9\]\{2\}:\[0-9\]\{2\} \(AM\|PM\)\#TIME\#g\n}{}m' "$SED_F"

# m3 — THE OVER-STRIP. Drop any line mentioning the phrase, unanchored. It
# blanks the duration, and it also blanks a probe's own comment.
mutate "m3 over-strip: delete any line mentioning the phrase" "$SED_F" \
  "selfcheck case 5 — the two comments asl echoed into the listing body must survive; a filter that edits body text can erase a real divergence" \
  "$SC" \
  perl -0pi -e 's{^s\#\^\(\[0-9\]\+ hours.*$}{/seconds assembly time/d}m' "$SED_F"

# m4 — the pure form of the defect: a filter that deletes everything. Every
# stream then hashes the same, forever.
mutate "m4 the filter is a sink (delete every line)" "$SED_F" \
  "selfcheck case 3 (a real content difference must still show) and case 6 (emitted bytes and the diagnostic must survive)" \
  "$SC" \
  perl -0pi -e 's{^s\#\^\(\[0-9\]\+ hours.*$}{s#.*##}m' "$SED_F"

# m5 — not a filter mutation: prove the symbol-class runner's UNSTABLE arm is
# live at all. A stability runner that can only ever print STABLE is the same
# defect wearing different clothes, and no ordinary run distinguishes them.
#
# THE NONCE HAS TO ACTUALLY VARY. The first version of this mutation wrote
# `echo "nonce "` — perl ate the `$RANDOM` as one of its own variables — and the
# patch landed, changed every hash, and still reported STABLE, because a
# CONSTANT addition is not instability. It read as a runner defect and was a
# mutation defect: `\$RANDOM` is what reaches the file as shell text.
mutate "m5 the probe runner emits a nonce" "$SYM_RUN" \
  "$SYM_STAB — every probe must read UNSTABLE and the runner must exit non-zero" \
  "$SYM_STAB" \
  perl -0pi -e 's{(cat "\$base\.lst" 2>/dev/null\n)}{$1echo "nonce \$RANDOM"\n}' "$SYM_RUN"

echo
echo "########## restored"
git status --short -- "$SED_F" "$SYM_RUN"
