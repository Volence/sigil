#!/usr/bin/env bash
# Proves that `sigil --version`'s `tree:` word FOLLOWS the sources the binary is
# compiled from — end to end, through cargo, on this machine.
#
# ── WHY THIS EXISTS AND WHY IT IS NOT IN THE SUITE ──────────────────────────────
# The banner's tree word is what aeon's assembler-provenance gate keys on: a
# positive match on the trusted words {clean, clean-sources}, anything else read
# as suspect. The failure that gate cannot survive is a FALSE CLEAN, and the
# capture is a build script's, so it is only ever re-evaluated when cargo decides
# the script is stale. Before crates/sigil-cli/build.rs named the closure's own
# paths as rerun triggers, every trigger was revision-shaped — HEAD, refs,
# packed-refs, manifests — and none followed file CONTENT. Cargo tracks sources
# for COMPILATION, so an uncommitted edit to a closure source recompiled the
# crate and relinked the binary while the build script kept its previous answer.
# Reproduced on 2026-09-04: the binary printed the uncommitted edit back at the
# operator while `--version` said `tree: clean at capture — no uncommitted
# changes`, and aeon's gate PASSED.
#
# crates/sigil-cli/tests/version_provenance.rs asserts what cargo was TOLD — the
# trigger set, derived from the banner's own closure and this filesystem. It
# cannot assert that cargo ACTED, because proving that means editing a tracked
# source, rebuilding, and reading the banner back. That must never run inside a
# suite in a shared checkout: a killed run would leave someone else's tree
# modified. So it lives here, alone, and is run by hand — by whoever is about to
# land, merge, freeze or quote this banner, and after any change to build.rs's
# trigger set. It is on no timer, and saying so is the point: an unstated
# assumption about who runs a gate is how a gate stops being run.
#
# ── EXIT CODES ──────────────────────────────────────────────────────────────────
#   0  the tree word followed the source edit
#   1  it did NOT — the banner reported a clean tree over a binary linked from
#      uncommitted code, which is the defect
#   2  the gate COULD NOT RUN (it says why). Never read as a pass.
#
# ── WHAT THIS RUN MUST FAIL ─────────────────────────────────────────────────────
# Step 4. With an uncommitted edit to a closure source on disk and linked into
# the binary, `--version` must NOT report a clean tree. A run that reaches step 4
# and finds `clean` exits 1. A run that never reaches step 4 exits 2 and is not
# evidence of anything.
set -uo pipefail

say() { printf '%s\n' "$*"; }
refuse() { printf '\nCOULD NOT RUN: %s\n' "$1" >&2; shift; for l in "$@"; do printf '   %s\n' "$l" >&2; done; exit 2; }
fail() { printf '\nFAILED: %s\n' "$1" >&2; shift; for l in "$@"; do printf '   %s\n' "$l" >&2; done; exit 1; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$HERE" rev-parse --show-toplevel 2>/dev/null)" \
    || refuse "$HERE is not inside a git checkout, so there is no tree to read"
cd "$ROOT" || refuse "cannot enter $ROOT"

# NEVER $ROOT/target. That is a shared artifact other lanes pin by hash, and
# relinking it out from under them is a documented incident of its own. Never
# /tmp either: it is tmpfs on this machine and a cargo build there wedges the
# shell. `/.target-tree-state-gate/` is gitignored, mirroring `/.target-land/`,
# so this gate's own build cannot dirty the tree whose cleanliness it asserts.
GATE_TARGET="${TREE_STATE_GATE_TARGET:-$ROOT/.target-tree-state-gate}"

say "=== tree-state capture gate"
say "    root:   $ROOT"
say "    HEAD:   $(git rev-parse HEAD)"
say "    branch: $(git rev-parse --abbrev-ref HEAD)"
say "    target: $GATE_TARGET"
say

# ── 1. a clean baseline, because the restore must be a committed one ────────────
# The mutation below is undone with `git checkout --`, which on an already-dirty
# tree would discard work that was never this gate's to touch. So a dirty tree is
# a refusal, not something to work around.
DIRT="$(git status --porcelain=v1 --untracked-files=normal)"
if [[ -n "$DIRT" ]]; then
    refuse "this checkout has uncommitted changes, and this gate restores its mutation with \`git checkout --\`" \
        "Committing or stashing first is required: on a dirty tree the restore could discard" \
        "work this gate never had any business touching." \
        "" \
        "$(printf '%s\n' "$DIRT" | sed 's/^/     /')"
fi

build() {  # build -> echoes the banner, or returns non-zero
    CARGO_TARGET_DIR="$GATE_TARGET" cargo build --bin sigil >"$GATE_TARGET.buildlog" 2>&1 || return 1
    "$GATE_TARGET/debug/sigil" --version
}

tree_word() { sed -n 's/^  tree: *//p' <<<"$1" | head -1 | awk '{print $1}'; }
first_line() { head -1 <<<"$1"; }

mkdir -p "$(dirname "$GATE_TARGET")"

# ── 2. the clean capture ────────────────────────────────────────────────────────
say "--- 2. build on the clean tree"
BANNER_CLEAN="$(build)" || refuse "the assembler did not build from a clean $ROOT" \
    "$(tail -20 "$GATE_TARGET.buildlog" 2>/dev/null | sed 's/^/     /')"
WORD_CLEAN="$(tree_word "$BANNER_CLEAN")"
say "    tag:  $(first_line "$BANNER_CLEAN")"
say "    tree: $WORD_CLEAN"
[[ "$WORD_CLEAN" == "clean" ]] || refuse \
    "a clean checkout reported \`tree: $WORD_CLEAN\`, so this gate has no baseline to move away from" \
    "Nothing below would distinguish a working trigger set from a broken one."

# ── 3. mutate a closure source, derived from the banner's own list ──────────────
# Derived, never hardcoded: the file to edit must be one THIS binary says it is
# compiled from. A hardcoded path silently stops testing anything the day the
# closure changes shape, which is the same staleness this whole feature is about.
say
say "--- 3. pick a closure source to mutate, from the banner's own closure-paths"
CLOSURE_PATHS="$(sed -n 's/^  closure-paths: //p' <<<"$BANNER_CLEAN")"
[[ -n "$CLOSURE_PATHS" ]] || refuse "the binary reported no closure paths, so nothing can be chosen"
read -r -a PATHS <<< "$CLOSURE_PATHS"

VICTIM=""
for p in "${PATHS[@]}"; do
    [[ -d "$p" ]] || continue
    # `-print -quit`: one tracked .rs file is all this needs, and the first one
    # found under a declared source directory is a compile input by construction.
    cand="$(find "$p" -name '*.rs' -type f -print -quit 2>/dev/null)"
    [[ -n "$cand" ]] || continue
    git ls-files --error-unmatch "$cand" >/dev/null 2>&1 || continue
    VICTIM="$cand"
    break
done
[[ -n "$VICTIM" ]] || refuse \
    "no tracked .rs file was found under any of the ${#PATHS[@]} closure paths the binary reports" \
    "Either the closure no longer names a source directory, or this is not the checkout it came from."
say "    chose: $VICTIM  (one of ${#PATHS[@]} reported closure paths)"

# From here on the tree is dirty, so every exit restores it.
restore() {
    git checkout -- "$VICTIM" 2>/dev/null
    local left
    left="$(git status --porcelain=v1 -- "$VICTIM")"
    if [[ -n "$left" ]]; then
        printf '\nWARNING: could not restore %s — it is still modified:\n%s\n' "$VICTIM" "$left" >&2
    fi
}
trap restore EXIT

MARK="tree-state capture gate mutation $$ — if this line survives, the gate was killed; git checkout it"
printf '\n// %s\n' "$MARK" >> "$VICTIM"

say "    the mutation is on disk:"
git diff --stat -- "$VICTIM" | sed 's/^/      /'
grep -n "$MARK" "$VICTIM" | sed 's/^/      /'
STATUS_LINE="$(git status --porcelain=v1 -- "$VICTIM")"
say "    git status: ${STATUS_LINE:-<NOTHING — the mutation did not land>}"
[[ -n "$STATUS_LINE" ]] || refuse \
    "the mutation did not reach git's view of $VICTIM, so a green below would prove nothing" \
    "A proof whose mutation failed to apply runs the original file and prints ok."

HEAD_NOW="$(git rev-parse HEAD)"
[[ "$HEAD_NOW" == "$(git rev-parse HEAD)" ]] || refuse "HEAD moved mid-run"
say "    HEAD has NOT moved: $HEAD_NOW"

# ── 4. THE ASSERTION ───────────────────────────────────────────────────────────
say
say "--- 4. rebuild and ask the binary what tree it came from"
BANNER_DIRTY="$(build)" || refuse "the assembler did not rebuild after the mutation" \
    "$(tail -20 "$GATE_TARGET.buildlog" 2>/dev/null | sed 's/^/     /')"
WORD_DIRTY="$(tree_word "$BANNER_DIRTY")"
TAG_DIRTY="$(first_line "$BANNER_DIRTY")"
say "    tag:  $TAG_DIRTY"
say "    tree: $(sed -n 's/^  tree: *//p' <<<"$BANNER_DIRTY" | head -1)"

if [[ "$WORD_DIRTY" != dirty* ]]; then
    fail "the banner reports \`tree: $WORD_DIRTY\` over a binary linked from an uncommitted edit" \
        "" \
        "  mutated:  $VICTIM  (a path this same binary lists in closure-paths)" \
        "  git says: $STATUS_LINE" \
        "  banner:   $TAG_DIRTY" \
        "" \
        "This is the false clean. aeon's build.sh matches {clean, clean-sources} positively and" \
        "would PASS here, on a binary built from code that is in nobody's history." \
        "" \
        "The cause is the rerun-trigger set in crates/sigil-cli/build.rs: unless every existing" \
        "closure path is named as \`cargo:rerun-if-changed\`, the capture is keyed on the revision" \
        "moving and never on the content of the sources."
fi
# The tag on line one is the greppable half of the same fact, and a consumer that
# reads only that must not see a bare SHA here.
[[ "$TAG_DIRTY" == *-dirty\) ]] || fail \
    "the tree word says \`$WORD_DIRTY\` but the first line's tag does not carry \`-dirty\`: $TAG_DIRTY" \
    "The two are renderings of one fact and a reader may key on either."
say "    OK — the capture followed the edit"

# ── 5. and back, so the word is following the tree rather than latching ─────────
say
say "--- 5. restore and rebuild: the word must come back"
restore
trap - EXIT
LEFT="$(git status --porcelain=v1)"
[[ -z "$LEFT" ]] || refuse "the restore did not return the tree to its committed state:" \
    "$(printf '%s\n' "$LEFT" | sed 's/^/     /')"
BANNER_AGAIN="$(build)" || refuse "the assembler did not rebuild after the restore"
WORD_AGAIN="$(tree_word "$BANNER_AGAIN")"
say "    tag:  $(first_line "$BANNER_AGAIN")"
say "    tree: $WORD_AGAIN"
[[ "$WORD_AGAIN" == "clean" ]] || fail \
    "the tree is committed and clean again but the banner still says \`$WORD_AGAIN\`" \
    "A word that latches dirty is a warning that is always on, which is a warning nobody reads."

say
say "PASS — the tree word followed a closure-source edit and followed it back."
say "       (\`cannot have been built from uncommitted sources\` — not \`the output is identical\`.)"
exit 0
