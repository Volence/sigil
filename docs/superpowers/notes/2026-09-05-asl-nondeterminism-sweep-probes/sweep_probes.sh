#!/usr/bin/env bash
# ASL-ORACLE-NONDETERMINISM — sweep 1: every committed probe corpus.
#
# For each `.asm` under every `docs/superpowers/notes/*-probes/` directory, run
# asl N times, push the whole stream (stdout+stderr+listing, the run.sh shape)
# through the shared declock filter, and hash. N identical hashes = "not seen to
# vary in N runs"; any disagreement = UNSTABLE, reported with the run index at
# which it first differed from run 1.
#
# The corpus is COPIED into a scratch tree first: asl writes `.p`/`.lst` beside
# its input and those are not ignored in the repo.
#
# ASLDIR selects the assembler. Every probe runner in the tree is pinned to
# s1disasm's build; s2disasm ships a DIFFERENT binary with the same banner, on
# which several of these probes DO vary.
#
# ── THE POSITIVE CONTROL IS BUILT IN, AND IT IS THE POINT ────────────────────
# A sweep that has never been seen to fire is the artifact this parcel exists to
# eliminate, so re-run with
#
#     ASLDIR=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
#
# and the same corpus must come back with UNSTABLE rows — `s_fn_defined_reg`,
# `u_bare` and their siblings in `2026-09-05-asl-nondeterminism-sweep-probes`,
# plus `wimm.asm` and `wrange.asm` in `2026-09-04-as-end-probes`. That directory
# is itself swept (it matches `*-probes`), so the control travels with the sweep
# and cannot rot away from it. A run against that binary reporting zero UNSTABLE
# is a broken sweep, not a clean corpus.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"   # docs/superpowers/notes/<this dir>/ → repo root
ASLDIR="${ASLDIR:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64}"
DECLOCK="$REPO/docs/superpowers/notes/asl-declock/declock.sed"
N="${1:-5}"
# OUTSIDE the swept tree: this script's own directory matches `*-probes` and is
# therefore one of the corpora it sweeps, so a work dir under $HERE would be
# copied into itself.
WORK="${TMPDIR:-/tmp}/asl_sweep_probes.$$"
trap 'rm -rf "$WORK"' EXIT

[[ -f $DECLOCK ]] || { echo "FATAL: no declock filter at $DECLOCK" >&2; exit 2; }
[[ -x $ASLDIR/asl ]] || { echo "FATAL: no asl at $ASLDIR/asl" >&2; exit 2; }

echo "# asl     $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)"
echo "# declock $DECLOCK md5 $(md5sum "$DECLOCK" | cut -d' ' -f1)"
echo "# N=$N"

rm -rf "$WORK"; mkdir -p "$WORK"
cp -r "$REPO"/docs/superpowers/notes/*-probes "$WORK"/

# one asl invocation, whole stream, declocked
stream() {
    local dir="$1" f="$2" base="${2%.asm}"
    rm -f "$dir/$base.p" "$dir/$base.lst"
    ( cd "$dir" && AS_MSGPATH="$ASLDIR" timeout 60 "$ASLDIR/asl" -xx -n -q -A -L -U -i "$dir" "$f" 2>&1; echo "ASL_EXIT=$?" ) 2>/dev/null
    echo "=== LISTING ==="
    cat "$dir/$base.lst" 2>/dev/null
}

total=0; stable=0; unstable=0; timedout=0; crashed=0
declare -a UNSTABLE_ROWS=()
declare -a OVERSTRIP_ROWS=()

for dir in "$WORK"/*-probes; do
    dname="$(basename "$dir")"
    for path in "$dir"/*.asm; do
        [[ -e $path ]] || continue
        f="$(basename "$path")"
        total=$((total+1))
        first=""; firstraw=""; varied_at=""
        for i in $(seq 1 "$N"); do
            raw="$(stream "$dir" "$f")"
            h="$(printf '%s\n' "$raw" | sed -E -f "$DECLOCK" | md5sum | cut -c1-12)"
            if [[ $i -eq 1 ]]; then
                first="$h"; firstraw="$raw"
                # over-strip audit: which lines did the filter actually change?
                changed="$(diff <(printf '%s\n' "$raw") <(printf '%s\n' "$raw" | sed -E -f "$DECLOCK") | grep -c '^<' || true)"
                suspicious="$(printf '%s\n' "$raw" | grep -cE '[0-9]{2}/[0-9]{2}/[0-9]{4}|[0-9]{2}:[0-9]{2}:[0-9]{2}' || true)"
                banner="$(printf '%s\n' "$raw" | grep -cE 'Page [0-9]+ - [0-9]{2}/|\*DATE|\*TIME|assembly time' || true)"
                if [[ $changed -gt 0 && $suspicious -gt $banner ]]; then
                    OVERSTRIP_ROWS+=("$dname/$f changed=$changed datetime_shaped=$suspicious banner_shaped=$banner")
                fi
            elif [[ $h != "$first" && -z $varied_at ]]; then
                varied_at="$i"
            fi
        done
        if [[ -n $varied_at ]]; then
            unstable=$((unstable+1))
            UNSTABLE_ROWS+=("$dname/$f first_varied_at_run=$varied_at")
            printf 'UNSTABLE %-42s %-14s first varied at run %s\n' "$dname" "$f" "$varied_at"
        else
            stable=$((stable+1))
        fi
        rc="$(printf '%s' "$firstraw" | grep -oE 'ASL_EXIT=[0-9]+' | tail -1)"
        case "$rc" in
            ASL_EXIT=124) timedout=$((timedout+1))
                printf 'TIMEOUT  %-42s %-14s (60s, reported as a measurement)\n' "$dname" "$f" ;;
            ASL_EXIT=139) crashed=$((crashed+1))
                printf 'SIGSEGV  %-42s %-14s asl crashed (exit 139)\n' "$dname" "$f" ;;
        esac
    done
done

echo
echo "=== SWEEP 1 TOTALS ==="
echo "probes swept   : $total"
echo "not seen to vary in $N runs : $stable"
echo "UNSTABLE       : $unstable"
echo "timed out      : $timedout"
echo "asl SIGSEGV    : $crashed"
if [[ ${#UNSTABLE_ROWS[@]} -gt 0 ]]; then
    echo "--- unstable rows ---"
    printf '%s\n' "${UNSTABLE_ROWS[@]}"
fi
if [[ ${#OVERSTRIP_ROWS[@]} -gt 0 ]]; then
    echo "--- POSSIBLE OVER-STRIP (date/time-shaped text beyond asl's own stamps) ---"
    printf '%s\n' "${OVERSTRIP_ROWS[@]}"
else
    echo "over-strip audit: no probe stream carried date/time-shaped text beyond asl's own stamps"
fi
