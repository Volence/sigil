#!/usr/bin/env bash
# One shape per file, put to BOTH assemblers, and printed side by side.
#
#   ./sigil_today.sh <path-to-sigil-binary> [N]
#
# `shapes.tsv` beside this file is the population. For each row this builds a
# minimal source — the shared preamble, an accepted `move.w #$A101,d0`, THE
# SHAPE, and an accepted `move.w #$A202,d0` — and reports, for that one line:
#
#   ASL   exit code, the emitted word, and any diagnostic
#   SIGIL exit code and any diagnostic
#
# WHY THE $A101 SETTER. asl does not report every operand it declines to value;
# it substitutes and carries on. So the word alone cannot say whether asl
# answered. Both shipped builds substitute, and they differ only in WHAT: the
# reference build (md5 61e67256…) substitutes THE LAST VALUE IT COMPUTED, the
# s2disasm build (md5 0dee1f98…) an uninitialized word. The setter directly
# above the shape makes the first case legible — a shape whose word is `A101` is
# echoing the setter and asl declined it — and running the second build N times
# makes the other case legible, because an unstable word changes between runs.
# A STABLE VALUE IS NOT AN ANSWER; only a word that is neither the setter nor
# unstable is.
#
# The reference build is digest-pinned through `../asl-reference/asl_ref.sh`.
# The varying build is reached deliberately and its md5 is printed; see that
# directory's README for why a guard on this script would delete the capability.
#
# `|| exit $?` is load-bearing — `set -uo pipefail` is not `set -e`.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SIGIL="${1:?usage: sigil_today.sh <path-to-sigil-binary> [N]}"
N="${2:-4}"
[ -x "$SIGIL" ] || { echo "FATAL: no executable sigil at $SIGIL" >&2; exit 2; }

. "$HERE/../asl-reference/asl_ref.sh" || exit $?
REF="$ASLDIR"
VAR=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64

echo "# sigil    $SIGIL"
# Not `| head -1`: sigil's --version is long, and closing the pipe on it makes it
# die of SIGPIPE and print a panic where a version line belongs.
sigil_version="$("$SIGIL" --version 2>&1)"
printf '#   %s\n' "${sigil_version%%$'\n'*}"
echo "# asl ref  $REF/asl md5 $(md5sum "$REF/asl" | cut -d' ' -f1)"
if [ -x "$VAR/asl" ]; then
    echo "# asl var  $VAR/asl md5 $(md5sum "$VAR/asl" | cut -d' ' -f1)   N=$N"
else
    echo "# asl var  UNMEASURABLE: no asl at $VAR/asl — reported, not skipped"
fi
echo

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Emit the code lines of a listing, stripped of the header and symbol table.
codelines() {
    sed -n '/Symbol Table/q;p' "$1" 2>/dev/null \
        | grep -E '^ *[0-9]+/ +[0-9A-F]+ : [0-9A-F]' | sed 's/[[:space:]]*$//'
}

while IFS=$'\t' read -r id line; do
    case "$id" in ''|\#*) continue ;; esac
    src="$WORK/$id.asm"
    {
        printf '\tcpu 68000\n\tpadding off\n'
        printf 'fu\tfunction p,(p*7)+$100\n'
        printf 'fi\tfunction p,$3C7\n'
        printf 'g\tfunction q,q+$10\n'
        printf 'f2\tfunction p,q,(p*7)+q\n'
        printf 'dsp\t=\t$2A\n'
        printf 'dsp\tfunction p,(p*7)+$100\n'
        printf 'k\t=\t3\n'
        # org 0, not the $1000 the sibling `disp-or-call` probes use: sigil's
        # `--hex` prints the whole located image, and 4KB of padding zeroes
        # would bury the three words this is measuring.
        printf '\torg 0\n'
        printf '\tmove.w\t#$A101,d0\n'
        printf "$line\n"
        printf '\tmove.w\t#$A202,d0\n'
    } > "$src"
    nlines=$(wc -l < "$src")

    echo "=== $id"
    sed -n '12,'"$((nlines - 1))"'p' "$src" | sed 's/^/    src | /'

    # --- asl, reference build ---
    ( cd "$WORK" && AS_MSGPATH="$REF" "$REF/asl" -xx -n -q -A -L -U -i "$WORK" "$id.asm" >"$id.ref.out" 2>&1 )
    rc=$?
    echo "    aslref exit=$rc"
    codelines "$WORK/$id.lst" | sed 's/^/    aslref | /'
    grep '> > >' "$WORK/$id.ref.out" | sed 's/^/    aslref ! /'
    # An error found EARLIER in a file stops asl's pass loop, after which every
    # later `symbol undefined` is emitted as a masked provisional value with no
    # diagnostic — see run.sh. One shape per file is the structural defence
    # against that, and this line is the check that the defence held.
    grep -q 'Additional necessary passes not started' "$WORK/$id.lst" 2>/dev/null \
        && echo "    aslref ! INCOMPLETE: pass loop stopped early — 'symbol undefined' reports SUPPRESSED"
    rm -f "$WORK/$id.lst" "$WORK/$id.p"

    # --- asl, varying build, N times: an unstable word is a declined operand ---
    if [ -x "$VAR/asl" ]; then
        for i in $(seq 1 "$N"); do
            ( cd "$WORK" && AS_MSGPATH="$VAR" "$VAR/asl" -xx -n -q -A -L -U -i "$WORK" "$id.asm" >/dev/null 2>&1 )
            vrc=$?
            printf '    aslvar%s exit=%s | ' "$i" "$vrc"
            codelines "$WORK/$id.lst" | tr '\n' '~' | sed 's/~/ ~ /g'
            echo
            rm -f "$WORK/$id.lst" "$WORK/$id.p"
        done
    else
        echo "    aslvar UNMEASURABLE"
    fi

    # --- sigil ---
    out=$("$SIGIL" "$src" --hex 2>&1)
    srcrc=$?
    echo "    sigil  exit=$srcrc"
    printf '%s\n' "$out" | cut -c1-160 | sed "s#$WORK/##" | sed 's/^/    sigil  | /'
    echo
done < "$HERE/shapes.tsv"
echo "=== ALL SHAPES DONE ==="
