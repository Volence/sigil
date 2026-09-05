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
#
# NATIVE=1 keeps a control too, and a different one: `2026-09-04-as-end-probes`
# names the s2disasm build itself, so `wimm.asm` and `wrange.asm` must come back
# UNSTABLE in native mode as well. (The `s_*`/`u_*` probes here do NOT fire under
# NATIVE=1 — this directory's own scripts name s1disasm first, so native mode
# assembles them with the stable build. That is correct behaviour and not a
# regression; the live control in native mode is the `as-end` pair.)
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
    local A="${DIR_ASLDIR:-$ASLDIR}"
    rm -f "$dir/$base.p" "$dir/$base.lst"
    ( cd "$dir" && AS_MSGPATH="$A" timeout 60 "$A/asl" -xx -n -q -A -L -U -i "$dir" "$f" 2>&1; echo "ASL_EXIT=$?" ) 2>/dev/null
    echo "=== LISTING ==="
    cat "$dir/$base.lst" 2>/dev/null
}

total=0; stable=0; unstable=0; timedout=0; crashed=0
declare -a UNSTABLE_ROWS=()
declare -a OVERSTRIP_ROWS=()

# ── NATIVE MODE: sweep each corpus with the binary ITS OWN RUNNER NAMES ──────
# The probe directories were not all pinned to the same assembler: three of them
# — `2026-09-04-as-end-probes`, `2026-09-04-as-warning-exitm-probes` and
# `2026-09-05-as-interp-radix-probes` — hard-coded the s2disasm path, the build
# that carries the value instability, while the rest hard-coded s1disasm.
# Sweeping every corpus with one binary answered the wrong question for those
# three: it could report a corpus stable under an assembler its own note never
# ran.
#
#     NATIVE=1 ./sweep_probes.sh 20
#
# resolves each directory's assembler the way that directory's own runner does.
#
# **Both hard-coded groups are gone** — the s2disasm three were repinned onto
# `asl-reference/asl_ref.sh`, and so were the s1disasm ones. A directory whose
# runners source that guard is pinned BY DIGEST to the reference build, which is
# a stronger statement than the path grep ever made, so it is reported as such
# rather than being allowed to fall through to the default and come out right
# for the wrong reason. The path grep is kept for a directory that still names a
# binary outright, and for corpora added later.
NATIVE="${NATIVE:-0}"
native_asldir() {
    local dir="$1" hit
    if grep -rqlE '\.[[:space:]]+"[^"]*asl-reference/asl_ref\.sh"' "$dir" --include='*.sh' 2>/dev/null; then
        # Digest-pinned to the reference build by the guard itself.
        printf '%s' "$ASL_REF_DIR_FOR_SWEEP"
        return
    fi
    hit="$(grep -rhoE '/home/volence/sonic_hacks/[a-z_0-9.-]+/(build_tools/[A-Za-z0-9_-]+|tools/as)' "$dir" --include='*.sh' 2>/dev/null | sort | uniq -c | sort -rn | head -1 | awk '{print $2}')"
    if [[ -n $hit && -x $hit/asl ]]; then printf '%s' "$hit"; else printf '%s' "$ASLDIR"; fi
}
# The guard's own reference directory, read out of the guard rather than
# duplicated here, so this file cannot drift from it.
ASL_REF_DIR_FOR_SWEEP="$(sed -n 's/^ASL_REF_DIR=//p' "$REPO/docs/superpowers/notes/asl-reference/asl_ref.sh")"
[[ -x $ASL_REF_DIR_FOR_SWEEP/asl ]] || { echo "FATAL: asl_ref.sh names no usable ASL_REF_DIR ($ASL_REF_DIR_FOR_SWEEP)" >&2; exit 2; }

for dir in "$WORK"/*-probes; do
    dname="$(basename "$dir")"
    DIR_ASLDIR="$ASLDIR"
    if [[ $NATIVE == 1 ]]; then
        DIR_ASLDIR="$(native_asldir "$dir")"
        printf '# %-42s asl %s\n' "$dname" "$(md5sum "$DIR_ASLDIR/asl" | cut -d' ' -f1)"
    fi
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
