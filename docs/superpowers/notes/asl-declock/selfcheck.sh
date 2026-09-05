#!/usr/bin/env bash
# selfcheck.sh — prove `declock.sed` still has teeth.
#
#     ./selfcheck.sh          # exit 0 = every case as required
#
# ── WHY THIS EXISTS ──────────────────────────────────────────────────────────
# `declock.sed` blanks asl's clock stamps so that a repeated-run hash measures
# the assembler. The whole risk of doing that is OVER-STRIPPING: a filter that
# removes too much turns a stability check into one that CANNOT FAIL, which is a
# worse defect than the timer it was fixing, and it is invisible — every run
# agrees, forever, and the check reads green while measuring nothing.
#
# So the load-bearing case here is case 3: two streams that differ in CONTENT
# must still hash differently THROUGH the filter. Cases 1, 2, 5 and 6 fence the
# filter in from the other sides.
#
# ── THE FIXTURES ─────────────────────────────────────────────────────────────
# `fixtures/run1.lst` is a REAL listing, byte for byte, from
#
#     asl -xx -n -q -A -L -U -i . probe.asm
#
# with `s1disasm/build_tools/Linux-x86_64/asl` (md5 61e672562465725a8c102288a7da9098)
# on `fixtures/probe.asm`, which is committed beside it. Every other fixture is
# that file with one EXACT LITERAL substring replaced by another exact literal;
# none of them is generated from the filter's own regex, which is the trap where
# subject and expectation move together and the test cannot disagree with the
# code. The two long-duration spellings in `run3`/`run4` were MEASURED off real
# runs of this binary (100M and 220M `rept` iterations), not composed.
#
# `fixtures/run6_mentions.lst` is a second REAL listing, of a probe whose source
# comments contain the phrase `0.00 seconds assembly time`. Nothing about it is
# synthetic: asl echoed those comments into the listing body itself.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SED="$HERE/declock.sed"
FX="$HERE/fixtures"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass=0; fail=0
h()  { sed -E -f "$SED" "$1" | md5sum | cut -c1-12; }
raw(){ md5sum < "$1" | cut -c1-12; }

verdict() { # <case> <got> <want> <what-other-answer>
    local name="$1" got="$2" want="$3" other="$4"
    if [[ "$got" == "$want" ]]; then
        echo "PASS  $name"
        pass=$((pass+1))
    else
        echo "FAIL  $name  (got '$got', required '$want')"
        fail=$((fail+1))
    fi
    echo "      what other answer could this have given: $other"
}

# The three rules as they stood BEFORE this parcel, written out verbatim. They
# are the subject of the red-first controls in cases 1 and 2 — never an input to
# a fixture, so no expectation here moves when the filter does.
INHERITED_DATE='s#[0-9]{2}/[0-9]{2}/[0-9]{4}#DATE#g'
INHERITED_TIME='s#[0-9]{2}:[0-9]{2}:[0-9]{2}#TIME#g'
OLD_SECONDS='s#^[0-9]+\.[0-9]+ seconds assembly time#SECONDS seconds assembly time#'

echo "### declock.sed selfcheck"
echo "### filter $SED"
echo "### md5    $(md5sum "$SED" | cut -d' ' -f1)"
echo

# ── 0. the fixtures are what they claim to be ────────────────────────────────
# A fixture pair that does not actually differ agrees with every filter, so each
# pair's raw difference is established before anything is filtered.
echo "--- case 0: the fixture pairs really differ before filtering"
for f in run2_clock run3_minute run4_minutes run5_content; do
    if [[ "$(raw "$FX/run1.lst")" == "$(raw "$FX/$f.lst")" ]]; then
        echo "FAIL  run1 vs $f are byte-identical — the fixture is decoration"
        fail=$((fail+1))
    else
        echo "PASS  run1 vs $f differ before filtering"
        pass=$((pass+1))
    fi
done
echo "      what other answer could this have given: identical, which is how a"
echo "      pair that proves nothing looks — every later case would pass vacuously."
echo

# ── 1. a clock-only difference collapses ─────────────────────────────────────
# run2's clock has moved across the meridiem, which is the case the INHERITED
# time rule got wrong: it blanked `NN:NN:NN` and left `AM`/`PM` standing, so any
# stability batch straddling noon or midnight false-alarmed on four lines. The
# inherited rules are required to be RED here, so this case is a live red-first
# and not a restatement of the fix.
echo "--- case 1: run1 vs run2_clock (date, time and duration all moved; nothing else)"
diff <(sed -E -f "$SED" "$FX/run1.lst") <(sed -E -f "$SED" "$FX/run2_clock.lst") > "$WORK/d1" 2>&1
verdict "clock-only difference is invisible through the filter" \
    "$([[ -s $WORK/d1 ]] && echo differs || echo same)" "same" \
    "differs — which is the FALSE ALARM this filter exists to close: the assembler
      reported as disagreeing with itself when only the clock moved."
diff <(sed -E -e "$INHERITED_DATE" -e "$INHERITED_TIME" -e "$OLD_SECONDS" "$FX/run1.lst") \
     <(sed -E -e "$INHERITED_DATE" -e "$INHERITED_TIME" -e "$OLD_SECONDS" "$FX/run2_clock.lst") > "$WORK/d1o" 2>&1
verdict "  ...and the INHERITED rules do not (red-first for the meridiem)" \
    "$([[ -s $WORK/d1o ]] && echo differs || echo same)" "differs" \
    "same — which would mean the inherited rules already reached the meridiem and
      the extra rule was decoration. The residue they leave is exactly:"
sed -n '1,4p' "$WORK/d1o" | sed 's/^/        /'
echo

# ── 2. the durations asl prints past 60 seconds ──────────────────────────────
# This is the case the predecessor rule got wrong, so it is run through BOTH
# anchors and the old one is required to be RED. The old rule is written out
# here verbatim; it is the subject of the red-first, not a fixture input.
echo "--- case 2: run1 vs the two MEASURED long-duration forms"
for f in run3_minute run4_minutes; do
    diff <(sed -E -f "$SED" "$FX/run1.lst") <(sed -E -f "$SED" "$FX/$f.lst") > "$WORK/d2" 2>&1
    verdict "$f collapses under declock.sed" \
        "$([[ -s $WORK/d2 ]] && echo differs || echo same)" "same" \
        "differs — asl prints '1 minute, 17.08 seconds assembly time' past 60s, and
      an anchor that only accepts 'N.NN seconds' does not reach it."
    diff <(sed -E "$OLD_SECONDS" "$FX/run1.lst") <(sed -E "$OLD_SECONDS" "$FX/$f.lst") > "$WORK/d2o" 2>&1
    verdict "  ...and the PREDECESSOR anchor does not (red-first for the widening)" \
        "$([[ -s $WORK/d2o ]] && echo differs || echo same)" "differs" \
        "same — which would mean the old anchor already covered this form and the
      widening was decoration."
done
echo

# ── 3. THE GATE: a content difference must survive ───────────────────────────
echo "--- case 3: run1 vs run5_content (identical clock; one emitted byte differs)"
echo "    the whole difference between the two fixtures:"
diff "$FX/run1.lst" "$FX/run5_content.lst" | sed 's/^/      /'
diff <(sed -E -f "$SED" "$FX/run1.lst") <(sed -E -f "$SED" "$FX/run5_content.lst") > "$WORK/d3" 2>&1
verdict "a real content difference still shows through the filter" \
    "$([[ -s $WORK/d3 ]] && echo differs || echo same)" "differs" \
    "same — the filter would have eaten the evidence, and the stability check
      would be one that cannot fail: every run agrees, forever, measuring nothing."
echo

# ── 4. the strip actually lands, on disk ─────────────────────────────────────
echo "--- case 4: the duration line, on disk, before and after"
sed -E -f "$SED" "$FX/run1.lst" > "$WORK/run1.filtered"
echo "    before: $(grep -n '^0\.00 seconds assembly time$' "$FX/run1.lst" || echo '(ABSENT — fixture is wrong)')"
echo "    after : $(grep -n 'seconds assembly time' "$WORK/run1.filtered" || echo '(no such line)')"
before_has="$(grep -c '^0\.00 seconds assembly time$' "$FX/run1.lst")"
after_has="$(grep -c '^0\.00 seconds assembly time$' "$WORK/run1.filtered")"
after_blank="$(grep -c '^SECONDS seconds assembly time$' "$WORK/run1.filtered")"
verdict "the raw duration is present before and absent after" \
    "$before_has/$after_has/$after_blank" "1/0/1" \
    "1/1/0 — the rule never fired (a wrong path, or sed without -E), which looks
      exactly like a clean pass on a stream that happened not to tick."
echo

# ── 5. no over-strip: a MENTION of the phrase is not a stamp ─────────────────
echo "--- case 5: run6_mentions — asl echoed the phrase into the listing BODY"
sed -E -f "$SED" "$FX/run6_mentions.lst" > "$WORK/run6.filtered"
body_before="$(grep -c 'seconds assembly time' "$FX/run6_mentions.lst")"
body_after="$(grep -c 'seconds assembly time' "$WORK/run6.filtered")"
kept="$(grep -c '; 0\.00 seconds assembly time$' "$WORK/run6.filtered")"
kept2="$(grep -c '; 1 minute, 17\.08 seconds assembly time$' "$WORK/run6.filtered")"
echo "    lines mentioning the phrase: $body_before before, $body_after after"
verdict "both echoed source comments survive byte-identical" \
    "$kept/$kept2" "1/1" \
    "0/0 — an unanchored rule would have rewritten a comment, and a filter that
      edits listing BODY text can silently erase a real divergence."
echo

# ── 6. the filter is not a sink ──────────────────────────────────────────────
echo "--- case 6: content the filter must never touch"
b="$(grep -c '1122' "$WORK/run1.filtered")"
e="$(grep -c 'error #1000' "$WORK/run1.filtered")"
verdict "emitted bytes and the diagnostic survive" "$b/$e" "1/1" \
    "0/0 — a filter that dropped the byte column or the diagnostics would make
      every stream agree while measuring nothing at all."
echo

echo "### totals: PASS $pass  FAIL $fail"
[[ $fail -eq 0 ]] || exit 1
