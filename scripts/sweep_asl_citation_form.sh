#!/usr/bin/env bash
# Which notes cite the reference assembler by a string that CANNOT identify it?
#
# WHY THIS EXISTS. All four `asl` binaries in this workspace print the identical
# first banner line `Macro Assembler 1.42 Beta [Bld 212]`, so a note that cites its
# oracle by that line has named nothing: the rows it states cannot be attributed to
# a binary, and two binaries that disagree are indistinguishable in the record.
# (The second banner line separates three families, upstream x86-64, upstream i386
# and the flamewing fork, so only 0dee1f98 and aa6de52f print an identical banner
# in full; a family is still not a binary.) The identifying citation in this lane
# is an md5. See
# docs/superpowers/notes/asl-reference/README.md and the reconstruction at
# docs/superpowers/notes/2026-09-05-disp-or-call-probes/README.md, which is the
# repaired instance of exactly this defect.
#
# WHAT THIS TESTS, STATED SO ITS SILENCE IS NOT READ AS COVERAGE. It tests the
# literal banner text against the four digests, each in full or as its 8-hex
# prefix, less a reviewed list of files that only MENTION the banner (see
# MENTION_ONLY). It does NOT find a note that describes the oracle in prose ("the
# reference build", "the s2disasm copy") without either string, and it cannot tell
# whether a given note's claim actually DEPENDS on which binary produced it. A
# clean result here means "no note cites the banner without also citing a digest",
# never "every asl claim in the tree is attributable".
#
# TWO RULES, BOTH PRINTED. Until 2026-09-27 the script matched only the two full
# digests of the x86-64 builds and counted every file carrying the banner text.
# Dated records quote that figure, so the script still computes it and prints it
# as the "historical count", beside the corrected "count" and a list of exactly
# which files the correction moved and why.
#
# Exit 0 always: this reports a population, it is not a gate. Wiring it as a gate
# would need a red-first proof and a derived expectation, neither of which exists.
#
# TWO THINGS ABOUT THIS WORKSPACE THAT THIS SCRIPT DEPENDS ON.
#   * `grep` here is ugrep (7.8.4), not GNU grep. Everything below sticks to flags
#     the two agree on (-c, -l, -x, -F, -v and basic regex; the one extended
#     regex, IDENT, goes to `git grep -E`, not to grep). Do not reach for a GNU
#     extension without checking it against `grep --version` on the box.
#   * Run it BY PATH (`./scripts/sweep_asl_citation_form.sh`). Invoking it through
#     process substitution (`bash <(git show HEAD:...)`) makes `$0` a /dev/fd entry,
#     the `cd` below lands outside the repo, and every count comes back 0. The
#     positive control catches that and refuses to print the population, which is
#     the control doing its job rather than a finding about the tree.
set -uo pipefail
cd "$(dirname "$0")/.." || { echo "cannot reach the repo root from \$0=$0; run this by path, not through process substitution" >&2; exit 0; }
if [ ! -d docs/superpowers/notes ]; then
    echo "NOT IN THE SIGIL REPO ROOT (pwd=$(pwd)); every count below would be a vacuous zero." >&2
    echo "Run it as ./scripts/sweep_asl_citation_form.sh from anywhere in a sigil checkout." >&2
    exit 0
fi

BANNER='Bld 212\|1\.42 Beta'
SCOPE='docs/superpowers/notes/'

# THE HISTORICAL RULE (2026-09-16 to 2026-09-26): the two FULL x86-64 digests, and
# every file carrying the banner text counted. Kept and printed because dated
# records quote the figure it produced (22 at 946e83c9 and at 9212d6e0): QUEUE.md's
# STABILITY-RUNNER-MISSING-WHERE-CLAIMED section, docs/lane-log.jsonl at 2026-09-16
# and 2026-09-26, and the notes 2026-09-16-asl-citation-form-sweep.md and
# 2026-09-26-asl-banner-citation-dependence.md. Those records stay as written.
LEGACY_MD5S='61e672562465725a8c102288a7da9098\|0dee1f98e6480a4783d27ffd8b90896f'

# THE CURRENT RULE (since 2026-09-27). An identifying citation is any of the FOUR
# digests that run on this machine, in full OR as its 8-hex prefix, which is the
# form notes in this lane actually write (`61e67256...`). The leading class stops a
# prefix matching inside a longer unrelated hex run. Extended regex (git grep -E).
IDENT='(^|[^0-9a-f])(61e67256|0dee1f98|a8cd8b80|aa6de52f)'

# MENTION-ONLY: files whose banner text is a remark ABOUT the banner (typically
# "a log naming only this has identified nothing"), not a citation of an oracle.
# No grep can tell those apart, so this is a reviewed list, `path|reason`, one per
# line, and every entry is re-checked below: an entry that no longer carries the
# banner, or that now cites a digest, is stale and the counts are withheld.
MENTION_ONLY='
docs/superpowers/notes/2026-09-03-tilde-tilde-probes/README.md|quotes the banner to say a log naming only it has identified nothing; its runner defaults to the digest-pinned ref build
'

banner_files=$(git grep -l "$BANNER" -- "$SCOPE" | grep '\.md$' | sort)
legacy_md5_files=$(git grep -l "$LEGACY_MD5S" -- "$SCOPE" | grep '\.md$' | sort)
ident_files=$(git grep -l -E "$IDENT" -- "$SCOPE" | grep '\.md$' | sort)
mention_files=$(echo "$MENTION_ONLY" | grep . | cut -d'|' -f1 | sort)

echo "tree:   $(git rev-parse --short HEAD)  branch: $(git rev-parse --abbrev-ref HEAD)"
echo "scope:  $SCOPE (tracked .md only; git grep cannot see ignored files)"
echo
echo "cite the banner:                                $(echo "$banner_files" | grep -c . )"
echo "cite a full x86-64 md5 (historical rule):       $(echo "$legacy_md5_files" | grep -c . )"
echo "cite any of the four digests, full or 8-hex:    $(echo "$ident_files" | grep -c . )"
echo "reviewed as mention-only:                       $(echo "$mention_files" | grep -c . )"
echo

# POSITIVE CONTROLS. Each names a file that must land on a given side; a failure
# means the instrument is broken, not the tree, and the counts are withheld.
# NOT `... | grep -qx`: under `pipefail`, grep -q closes the pipe on its FIRST
# match, the writer dies of SIGPIPE, and the pipeline reports failure for a
# successful match. That inversion is banked in this lane and it broke this very
# script's first draft, where it turned a healthy instrument into a failed control.
in_set() { echo "$2" | grep -x -F "$1" >/dev/null; }
failed=0
# 1. The repaired instance cites a full md5, so both rules must see it.
control='docs/superpowers/notes/2026-09-05-disp-or-call-probes/README.md'
if in_set "$control" "$legacy_md5_files" && in_set "$control" "$ident_files"; then
    echo "positive control OK: the repaired instance cites an md5 (both rules)"
else
    echo "POSITIVE CONTROL FAILED: $control is not md5-citing under both rules."
    failed=1
fi
# 2. The prefix rule is live: this note names every build by 8-hex prefix only,
#    so the current rule must see it and the historical rule must not.
control='docs/superpowers/notes/2026-09-05-asl-silent-decline-regime.md'
if in_set "$control" "$ident_files" && ! in_set "$control" "$legacy_md5_files"; then
    echo "positive control OK: an 8-hex-prefix-only note is identifying under the current rule alone"
else
    echo "POSITIVE CONTROL FAILED: $control must be identifying by prefix and invisible to the historical rule."
    failed=1
fi
# 3. Every mention-only entry still carries the banner and still cites no digest.
for m in $mention_files; do
    if in_set "$m" "$banner_files" && ! in_set "$m" "$ident_files"; then
        echo "mention-only entry OK: $m"
    else
        echo "MENTION-ONLY ENTRY STALE: $m no longer carries the banner, or now cites a digest. Review the list."
        failed=1
    fi
done
if [ "$failed" != 0 ]; then
    echo "The counts would not mean what they say. Fix the instrument before reading them."
    exit 0
fi

legacy=$(comm -23 <(echo "$banner_files") <(echo "$legacy_md5_files"))
current=$(comm -23 <(comm -23 <(echo "$banner_files") <(echo "$ident_files")) <(echo "$mention_files"))
echo
echo "BANNER-ONLY (cites a string that identifies no binary, never a digest; mentions excluded):"
echo "$current" | grep . | sed 's/^/  /'
echo
echo "IN THE HISTORICAL COUNT, NOT IN THIS ONE, and why:"
comm -23 <(echo "$legacy" | grep .) <(echo "$current" | grep .) | while read -r f; do
    if in_set "$f" "$mention_files"; then
        why="mention-only: $(echo "$MENTION_ONLY" | grep -F "$f|" | cut -d'|' -f2)"
    elif in_set "$f" "$ident_files"; then
        why="cites a digest the historical rule could not see (8-hex prefix, or an i386 or aa6de52f digest)"
    else
        why="UNEXPLAINED: the two rules disagree for a reason this script does not know"
    fi
    echo "  $f"
    echo "    $why"
done
echo
echo "historical count (2026-09-16 rule: two full digests, mentions counted): $(echo "$legacy" | grep -c .)"
echo "count: $(echo "$current" | grep -c .)"
