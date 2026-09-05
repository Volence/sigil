#!/usr/bin/env bash
# IS THE CHANGED CODE REACHED BY THE AEON BUILD? Two `panic!`s, in the two places
# that answer different halves of the question, each built and run against all
# four shapes.
#
#   ./reach.sh <worktree-root> <cargo-target-dir> <aeon-dir> <scratch-dir>
#
# C1 — panic at the TOP of `directive_include`. All four shapes MUST FAIL. If
#      they build, the directive is never reached and the four-shape identity is
#      about code that did not run.
# C2 — panic on the REPEAT path (an include whose canonical path was already
#      executed): the branch the old guard used to take. All four shapes MUST
#      STILL BUILD, because aeon's population of repeated includes is zero. A
#      shape that fails here would mean the census is lying.
#
# Reading the pair: C1 red and C2 green together say the plumbing runs (twice per
# build, once per game root) and the CHANGED BEHAVIOUR does not. That is the
# honest content of a four-shape CRC identity for this parcel, and it is why
# `poscontrol.sh` exists.
#
# The release binary in the target dir is MUTATED while this runs, and is rebuilt
# from restored source at the end.
set -uo pipefail
ROOT="$1"; TGT="$2"; AEON="$3"; SCRATCH="$4"
EVAL_RS="crates/sigil-frontend-as/src/eval.rs"
mkdir -p "$SCRATCH"
cd "$ROOT" || exit 2
restore() { git checkout -- "$EVAL_RS"; }
trap restore EXIT

shapes() {
    local bin="$1" tag="$2"
    for shape in "sonic4:" "sonic4:--debug" "demo:" "demo:--debug"; do
        local g="${shape%%:*}" d="${shape##*:}"
        local s="$g${d:+-debug}"
        # shellcheck disable=SC2086
        AEON_DIR="$AEON" "$bin" build --aeon "$AEON" --game "$g" $d \
            -o "$SCRATCH/$tag-$s.bin" > "$SCRATCH/$tag-$s.log" 2>&1
        printf '  %-14s BUILD_EXIT=%d %s\n' "$s" "$?" \
            "$(grep -c 'panicked' "$SCRATCH/$tag-$s.log" | sed 's/^/panics=/')"
    done
}

control() {
    local name="$1" must="$2"; shift 2
    echo
    echo "########## CONTROL: $name"
    echo "MUST: $must"
    "$@"
    if git diff --quiet -- "$EVAL_RS"; then
        echo "RESULT: VACUOUS — the panic did not land; this measures the ORIGINAL binary."
        restore
        return
    fi
    git diff --unified=0 -- "$EVAL_RS" | grep '^+' | grep -v '^+++' | sed 's/^/  patch: /'
    CARGO_TARGET_DIR="$TGT" cargo build --release -p sigil-cli 2>&1 | grep -E '^error' | head -5
    local bin="$SCRATCH/sigil-$name"
    cp "$TGT/release/sigil" "$bin"
    shapes "$bin" "$name"
    restore
}

control c1-top "ALL FOUR SHAPES FAIL — otherwise directive_include is never reached" \
  perl -0pi -e 's/(    fn directive_include\(&mut self, rest: &\[Token\], span: Span\) \{\n)/$1        panic!("C1: directive_include reached");\n/' "$EVAL_RS"

control c2-repeat "ALL FOUR SHAPES BUILD — aeon has no repeated include for this branch to take" \
  perl -0pi -e 's/(        self\.include_census\.seen_total \+= 1;\n)/$1        {\n            let canon = path.canonicalize().unwrap_or_else(|_| path.clone());\n            if !self.include_census.seen.insert(canon) { panic!("C2: repeated include reached"); }\n        }\n/' "$EVAL_RS"

echo
CARGO_TARGET_DIR="$TGT" cargo build --release -p sigil-cli 2>&1 | grep -E '^error' | head -5
echo "release binary rebuilt from restored source"
git status --short -- "$EVAL_RS"
