#!/usr/bin/env bash
# RED-FIRST for `crates/sigil-frontend-as/tests/as_include_repeat.rs`.
#
#   ./mutations.sh <worktree-root> <cargo-target-dir>
#
# Each mutation breaks the rule in ONE way and the script says, per mutation,
# which tests MUST go red. Three things this checks that a naive red-first does
# not:
#
#   * IT PROVES THE PATCH LANDED. A `sed` that matches nothing leaves the
#     original file in place, the suite runs the UNMUTATED code and prints `ok`,
#     and that is indistinguishable from a clean restore. Every mutation here
#     diffs the file against HEAD and REFUSES if the diff is empty.
#   * APPLIED-AND-STILL-GREEN IS A DEFECT, NOT A PASS. If the patch landed and
#     the suite is green, the test does not cover the mutation; the script says
#     so in those words rather than counting it.
#   * THE RESTORE COMES FROM THE COMMITTED BASELINE (`git checkout -- <path>`),
#     not from a saved copy this script made, so a crash mid-run cannot leave a
#     mutated tree that looks original.
set -uo pipefail
ROOT="${1:?usage: mutations.sh <worktree-root> <cargo-target-dir>}"
TGT="${2:?usage: mutations.sh <worktree-root> <cargo-target-dir>}"
EVAL_RS="crates/sigil-frontend-as/src/eval.rs"
LIB_RS="crates/sigil-frontend-as/src/lib.rs"
cd "$ROOT" || exit 2

restore() { git checkout -- "$EVAL_RS" "$LIB_RS"; }
trap restore EXIT

mutate() {
    local name="$1" file="$2" must="$3"; shift 3
    echo
    echo "########## MUTATION: $name"
    echo "MUST FAIL: $must"
    "$@"
    if git diff --quiet -- "$file"; then
        echo "RESULT: VACUOUS — the edit did not land ($file unchanged vs HEAD)."
        echo "        A suite run now would exercise the ORIGINAL code and print ok,"
        echo "        which is indistinguishable from a clean restore. Not a pass."
        restore
        return
    fi
    echo "--- the patch, as it stands on disk ---"
    git diff --unified=1 -- "$file" | sed -n '5,40p'
    echo "--- suite ---"
    CARGO_TARGET_DIR="$TGT" cargo test -p sigil-frontend-as --test as_include_repeat 2>&1 \
        | grep -E '^(test |error|test result)' | sed 's/^/    /'
    local rc=${PIPESTATUS[0]}
    echo "CARGO_EXIT=$rc"
    if [[ $rc -eq 0 ]]; then
        echo "RESULT: APPLIED AND STILL GREEN — this is a RUNNER DEFECT, not a pass."
        echo "        The mutation is live and no fixture noticed it."
    else
        echo "RESULT: RED, as required."
    fi
    restore
}

# m1 — put the old DAG guard back: skip an include whose canonical path was
# already executed. This is the exact code that was removed.
mutate "m1 restore the visited-set DAG guard" "$EVAL_RS" \
  "a_header_included_twice_assembles_twice, a_diamond_assembles_the_shared_file_twice, three_spellings_of_one_path_are_three_inclusions, a_re_included_header_re_runs_its_directives, a_self_including_file_is_refused_for_depth, mutual_recursion_between_two_files_is_refused_for_depth" \
  perl -0pi -e 's/(        self\.include_census\.seen_total \+= 1;\n)/$1        {\n            let canon = path.canonicalize().unwrap_or_else(|_| path.clone());\n            if !self.include_census.seen.insert(canon) { return; }\n        }\n/' "$EVAL_RS"

# m2 — off by one the wrong way: refuse one level EARLIER than asl.
mutate "m2 bound is INCLUDE_NEST_MAX-1" "$LIB_RS" \
  "the_bound_constant_matches_what_asl_measured, and nesting_is_clean_at_the_bound_and_refused_one_level_past (its CLEAN half: a 199-deep chain must now be refused). NOTE: the first version of the test file built its chains FROM this constant and stayed green under this mutation — a fixture whose input derives from the value under test cannot disagree with it. The 199/200 are now written out in the test file as asl own numbers." \
  perl -0pi -e 's/pub const INCLUDE_NEST_MAX: u32 = 199;/pub const INCLUDE_NEST_MAX: u32 = 198;/' "$LIB_RS"

# m3 — remove the bound entirely, the "just delete the guard" reading of this
# parcel. p2/p3 then recurse until the native stack goes.
mutate "m3 no depth bound at all" "$EVAL_RS" \
  "a_self_including_file_is_refused_for_depth, mutual_recursion_between_two_files_is_refused_for_depth, nesting_is_clean_at_the_bound_and_refused_one_level_past (the refused half). A stack overflow aborts the test process, which is red — and is exactly the crash-instead-of-a-verdict this bound exists to prevent." \
  perl -0pi -e 's/if self\.include_depth >= crate::INCLUDE_NEST_MAX \{/if false \{/' "$EVAL_RS"

# m4 — refuse but do NOT abort: unwind one level and carry on.
mutate "m4 depth refusal returns instead of aborting" "$EVAL_RS" \
  "the_depth_refusal_terminates_the_assembly — measured at 200 diagnostics under this mutation against 1 with the abort. NOTE: the first version of that fixture put an UNDEFINED SYMBOL after the include and stayed green, because undefined names are promoted at the end of the converged pass and a run already holding an error never gets there; the trailing line must be an error the front end raises immediately." \
  perl -0pi -e 's/(            self\.err\(span, crate::INCLUDE_NEST_TOO_DEEP\);\n)(?:.*?\n)*?(            self\.aborted = true;\n)/$1/' "$EVAL_RS"

# m5 — do not restore the depth on the way out, so depth only ever climbs.
# A sequence of sibling includes then trips the bound as if it were nested.
mutate "m5 include_depth is never decremented" "$EVAL_RS" \
  "sibling_includes_do_not_accumulate_depth — 250 includes IN SEQUENCE, which asl assembles clean because its bound is on how many are OPEN AT ONCE. NOTE: every NESTED fixture in the file stays green under this mutation (the diamond reaches depth 2, the cycles are refused anyway, the 199-chain is at the bound either way), so before that test existed the whole decrement was unpinned." \
  perl -0pi -e 's/                self\.exec\(&lines\);\n                self\.include_depth -= 1;\n/                self.exec(&lines);\n/' "$EVAL_RS"

# m6 — the census counter lies: never count a repeat. The engagement number is
# load-bearing evidence in the parcel report ("an empty diff beside repeats=0"
# means nothing unless `repeats` would have been non-zero had there been
# repeats), so it needs a check of its own.
#
# Its runner is NOT cargo: nothing in `as_include_repeat.rs` reads the census,
# because the census is an instrument and not a rule, and wiring a unit test to
# an eprintln would be testing the printer. `census_selfcheck.sh` calibrates it
# end to end against four probe sources whose repeat counts can be read straight
# off the file. This mutation is here to show that check has teeth.
echo
echo "########## MUTATION: m6 census never counts a repeat"
echo "MUST FAIL: census_selfcheck.sh, on all four rows (they expect 1, 1, 2, 2 and would all read 0)."
perl -0pi -e 's/                self\.include_census\.repeats \+= 1;/                let _ = 0;/' "$EVAL_RS"
if git diff --quiet -- "$EVAL_RS"; then
    echo "RESULT: VACUOUS — the edit did not land."
else
    git diff --unified=1 -- "$EVAL_RS" | sed -n '5,20p'
    CARGO_TARGET_DIR="$TGT" cargo build --release -p sigil-cli 2>&1 | grep -E '^error' | head -5
    MUT_BIN="$(mktemp -u /home/volence/sonic_hacks/.parcel-include-scratch/sigil-m6.XXXXXX)"
    cp "$TGT/release/sigil" "$MUT_BIN"
    docs/superpowers/notes/2026-09-05-as-include-repeat-probes/census_selfcheck.sh "$MUT_BIN"
    src="$?"
    echo "SELFCHECK_EXIT under mutation=$src"
    if [[ $src -eq 0 ]]; then
        echo "RESULT: APPLIED AND STILL GREEN — RUNNER DEFECT, not a pass."
    else
        echo "RESULT: RED, as required."
    fi
    rm -f "$MUT_BIN"
fi
restore
# The release binary in the target dir is now the MUTATED one. Rebuild it from
# the restored source, or the next thing to pick it up gets m6's assembler.
CARGO_TARGET_DIR="$TGT" cargo build --release -p sigil-cli 2>&1 | grep -E '^error' | head -5
echo "release binary rebuilt from restored source"

echo
echo "########## restored"
git status --short -- "$EVAL_RS" "$LIB_RS"
