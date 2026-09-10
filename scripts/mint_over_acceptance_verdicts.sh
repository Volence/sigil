#!/usr/bin/env bash
# Mint the committed asl verdict table for the over-acceptance probe corpus.
#
# MANUAL developer tool. The gate that reads the table
# (`crates/sigil-frontend-as/tests/as_over_acceptance.rs`) runs sigil live and
# reads asl's verdicts from the committed file, so the gate itself needs no asl
# and runs anywhere. Re-mint when a probe is added, changed or removed.
#
# ── WHAT THIS RECORDS, AND WHAT IT REFUSES TO RECORD ─────────────────────────
# Only VERDICTS: did asl exit zero, and if not, what did it say. NEVER BYTES.
# That is not an omission, it is the whole reason this tool is safe to point at
# a corpus of deliberately-failing files. `asl_ref.sh`'s header states the rule
# in its own words: a run carrying any error is not a source of values for the
# lines that DID assemble, because an error stops the pass loop and every
# forward reference is left at its unresolved pass-1 placeholder while the
# listing prints it looking complete. Most probes here are SUPPOSED to fail, so
# a nonzero exit is the subject of the measurement rather than a fault in it.
# Reading only the verdict is what makes that tension safe.
#
# For the same reason `asl_run`'s nonzero return is captured rather than
# propagated: `|| exit $?` is the correct discipline for a caller mining values,
# and the wrong one for a caller mining refusals.
#
# ── SELECTION ────────────────────────────────────────────────────────────────
# The reference build is selected by `asl_ref.sh`, by md5, never by path or
# banner. Seven asl paths on this machine execute under four digests and all of
# them print the same version string.
#
# Usage:  scripts/mint_over_acceptance_verdicts.sh
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PROBES="$REPO/crates/sigil-frontend-as/tests/over_acceptance/probes"
OUT="$REPO/crates/sigil-frontend-as/tests/over_acceptance/asl_verdicts.txt"
WORK="${OVER_ACCEPTANCE_WORK:-$REPO/.mint-work}"

. "$REPO/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?

[ -d "$PROBES" ] || { echo "FATAL: no probe directory at $PROBES" >&2; exit 2; }

rm -rf "$WORK"
mkdir -p "$WORK"

ASL_MD5="$(md5sum "$ASL" | cut -d' ' -f1)"
ASL_BANNER="$("$ASL" 2>&1 | head -2 | tr '\n' '|')"

tmp="$WORK/table"
: > "$tmp"

n=0
for p in "$PROBES"/*.asm; do
    name="$(basename "$p" .asm)"
    n=$((n + 1))
    d="$WORK/$name"
    mkdir -p "$d"
    cp "$p" "$d/probe.asm"
    (
        cd "$d" || exit 9
        asl_run -xx -n -q -A -L -U -i . probe.asm
    ) > "$d/stdout.txt" 2> "$d/stderr.txt"
    rc=$?

    # asl writes diagnostics to STDERR, not stdout, as
    #     > > > probe.asm(3): error #1320: range overflow
    # Measured, and the first version of this script had it wrong: reading
    # stdout returned no class for all 82 probes and the table rendered a clean
    # column of "no diagnostic", which is the vacuous zero this repo keeps
    # finding. `asl_run`'s own banner goes to stderr too and carries no
    # `error #NNNN:` shape, so it cannot be mistaken for a diagnostic here.
    #
    # The NUMBER is the class identifier and the text is its rendering, so both
    # are kept and the number is what the corpus is enumerated over. Taking the
    # text alone would make two distinct refusal classes that happen to share a
    # wording indistinguishable, and would move under a message-catalogue edit.
    msg="$(sed -n 's/^.*: \(error\|fatal error\) #\([0-9]\+\): \(.*\)$/#\2 \3/p' \
           "$d/stderr.txt" 2>/dev/null | head -1 | sed 's/[[:space:]]*$//')"

    diag="$(sed -n 's/^ASL_DIAG=//p' "$d/stderr.txt" | head -1 | cut -d' ' -f1)"

    if [ "$rc" -eq 0 ]; then
        verdict=accept
        msg=""
    else
        verdict=refuse
        # A nonzero exit with no numbered diagnostic anywhere is not a class we
        # can name, and rendering it as an empty message would put a blank in a
        # column readers scan for classes. Say so instead.
        # Real, and measured on `macro_recursive_call`: asl prints the macro
        # call chain as the location prefix, truncates that prefix at a fixed
        # width, and never reaches the message. The refusal is unambiguous and
        # the class is genuinely unnamed, so it is recorded as unnamed.
        [ -n "$msg" ] || msg="(exit $rc, refused with no numbered diagnostic)"
    fi

    printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$verdict" "$rc" "$diag" "$msg" >> "$tmp"
done

# EXTRACTOR CANARY. A broken diagnostic extractor cannot be told apart from a
# corpus asl happens to refuse without numbering, because both render the same
# empty class column, and the first version of this script was exactly that: it
# read stdout, asl writes to stderr, and all 82 rows came back classless while
# the table looked finished. A refusal corpus with no named class anywhere is
# therefore treated as an instrument failure and not as a measurement.
classed=$(awk -F'\t' '$2=="refuse" && $5 ~ /^#[0-9]/' "$tmp" | wc -l)
refused=$(awk -F'\t' '$2=="refuse"' "$tmp" | wc -l)
if [ "$refused" -gt 0 ] && [ "$classed" -eq 0 ]; then
    echo "FATAL: $refused probes were refused and NONE carried a numbered asl" >&2
    echo "  diagnostic. That is the extractor failing, not asl declining to" >&2
    echo "  number its errors. Nothing written." >&2
    exit 4
fi

{
    echo "# asl verdicts over the over-acceptance probe corpus. GENERATED by"
    echo "# scripts/mint_over_acceptance_verdicts.sh. Do not hand-edit."
    echo "#"
    echo "# VERDICTS ONLY, NEVER BYTES. Most probes here are supposed to fail, and a"
    echo "# failing asl run's byte column is an artifact: an error stops asl's pass"
    echo "# loop, so forward references stay at their pass-1 placeholder and the"
    echo "# listing prints them looking complete. Reading only accept-or-refuse and"
    echo "# the diagnostic text is what makes a corpus of failing files a safe"
    echo "# measurement. See docs/superpowers/notes/asl-reference/asl_ref.sh."
    echo "#"
    echo "# IDENTITY IS THE DIGEST. Seven asl paths on this machine execute under"
    echo "# four distinct digests and every one prints the banner below, so the"
    echo "# banner discriminates nothing and is human context only."
    echo "# asl-md5     $ASL_MD5"
    echo "# asl-banner  $ASL_BANNER"
    echo "#"
    echo "# diag is asl_diag_state's classification of the listing footer."
    echo "# INCOMPLETE means asl refused to start a later pass, so the diagnostics"
    echo "# that pass would have raised were never looked for: the message column"
    echo "# for such a row is the FIRST thing asl noticed, not the only thing wrong."
    echo "#"
    echo "# name<TAB>verdict<TAB>exit<TAB>diag<TAB>first error message"
    sort "$tmp"
} > "$OUT"

echo "minted $n verdicts to $OUT"
echo "  accept: $(cut -f2 "$tmp" | grep -c '^accept$')"
echo "  refuse: $(cut -f2 "$tmp" | grep -c '^refuse$')"
rm -rf "$WORK"
