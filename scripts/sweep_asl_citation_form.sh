#!/usr/bin/env bash
# Which notes cite the reference assembler by a string that CANNOT identify it?
#
# WHY THIS EXISTS. All four `asl` binaries in this workspace print the identical
# version banner `Macro Assembler 1.42 Beta [Bld 212]`, so a note that cites its
# oracle by banner has named nothing: the rows it states cannot be attributed to a
# binary, and two binaries that disagree are indistinguishable in the record. The
# identifying citation in this lane is an md5. See
# docs/superpowers/notes/asl-reference/README.md and the reconstruction at
# docs/superpowers/notes/2026-09-05-disp-or-call-probes/README.md, which is the
# repaired instance of exactly this defect.
#
# WHAT THIS TESTS, STATED SO ITS SILENCE IS NOT READ AS COVERAGE. It tests ONE
# spelling: the literal banner text against the two known md5s. It does NOT find
# a note that describes the oracle in prose ("the reference build", "the s2disasm
# copy") without either string, and it cannot tell whether a given note's claim
# actually DEPENDS on which binary produced it. A clean result here means "no note
# cites the banner without also citing an md5", never "every asl claim in the tree
# is attributable".
#
# Exit 0 always: this reports a population, it is not a gate. Wiring it as a gate
# would need a red-first proof and a derived expectation, neither of which exists.
#
# TWO THINGS ABOUT THIS WORKSPACE THAT THIS SCRIPT DEPENDS ON.
#   * `grep` here is ugrep (7.8.4), not GNU grep. Everything below sticks to flags
#     the two agree on (-c, -l, -x, -v and basic regex). Do not reach for a GNU
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
MD5S='61e672562465725a8c102288a7da9098\|0dee1f98e6480a4783d27ffd8b90896f'
SCOPE='docs/superpowers/notes/'

banner_files=$(git grep -l "$BANNER" -- "$SCOPE" | grep '\.md$' | sort)
md5_files=$(git grep -l "$MD5S" -- "$SCOPE" | grep '\.md$' | sort)

echo "tree:   $(git rev-parse --short HEAD)  branch: $(git rev-parse --abbrev-ref HEAD)"
echo "scope:  $SCOPE (tracked .md only; git grep cannot see ignored files)"
echo
echo "cite the banner:            $(echo "$banner_files" | grep -c . )"
echo "cite an identifying md5:    $(echo "$md5_files" | grep -c . )"
echo

# POSITIVE CONTROL. If this returns nothing the instrument is broken, not the tree:
# this file is the repaired instance and must appear in the md5-citing set.
control='docs/superpowers/notes/2026-09-05-disp-or-call-probes/README.md'
# NOT `git grep ... | grep -qx`: under `pipefail`, grep -q closes the pipe on its
# FIRST match, git grep dies of SIGPIPE, and the pipeline reports failure for a
# successful match. That inversion is banked in this lane and it broke this very
# script's first draft, where it turned a healthy instrument into a failed control.
if echo "$md5_files" | grep -x "$control" >/dev/null; then
    echo "positive control OK: the repaired instance cites an md5"
else
    echo "POSITIVE CONTROL FAILED: $control does not cite an md5."
    echo "The zero below would be vacuous. Fix the instrument before reading it."
    exit 0
fi
echo
echo "BANNER-ONLY (cites a string that identifies no binary, never an md5):"
comm -23 <(echo "$banner_files") <(echo "$md5_files") | sed 's/^/  /'
echo
echo "count: $(comm -23 <(echo "$banner_files") <(echo "$md5_files") | grep -c .)"
