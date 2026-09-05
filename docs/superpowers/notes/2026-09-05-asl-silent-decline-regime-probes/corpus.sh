#!/usr/bin/env bash
# Enumerate the silent-decline regime's population in the two public Sonic
# disassemblies. Prints COUNTS, never a sample, and prints the command that
# produced each one so a reader can disagree with it.
#
#   ./corpus.sh [dir ...]     # default: s1disasm and s2disasm
#
# Four parameters, because no one of them finds the whole regime:
#
#   P1  every `<name> function <params>,<body>` definition, and which of them
#       have a body that never mentions any parameter — those are the ones a
#       LAZY expander folds without ever looking at the argument
#   P2  `<fnname>(<register>)` anywhere, for every name P1 found
#   P3  `dc.<size>` whose whole operand is a bare 68000 register
#   P4  `#<anything>(<register>)` — the immediate shape, name-independent, so
#       it also catches a function this pass did not identify
#
# A count of zero is a RESULT and is printed as one. It is never rendered as a
# pass, and a directory that cannot be read is reported UNMEASURABLE rather than
# counted as zero.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
DIRS=("$@")
if [ ${#DIRS[@]} -eq 0 ]; then
    DIRS=(/home/volence/sonic_hacks/s1disasm /home/volence/sonic_hacks/s2disasm)
fi

# The 68000 names asl's expression parser resolves as registers. Derived from
# probe `r04`/`shapes.tsv`, not guessed: a0-a7, d0-d7 and sp are SILENT there;
# pc, sr, ccr and usp are loud (`error #1010: symbol undefined`) and so are
# outside this regime.
REGS='([aAdD][0-7]|[sS][pP])'

for dir in "${DIRS[@]}"; do
    echo "############################################################"
    if [ ! -d "$dir" ]; then
        echo "# $dir  UNMEASURABLE: not a directory — reported, not counted as zero"
        continue
    fi
    echo "# $dir  git $(git -C "$dir" rev-parse --short HEAD 2>/dev/null || echo '(not a git checkout)')"
    files=$(find "$dir" -name '*.asm' -not -path '*/.git/*' | wc -l)
    echo "#   .asm files walked: $files"

    echo
    echo "--- P1  function definitions"
    echo "    cmd: grep -rEin '^[A-Za-z_.][A-Za-z0-9_.]*[[:space:]]+function[[:space:]]' --include='*.asm'"
    defs=$(grep -rEin '^[A-Za-z_.][A-Za-z0-9_.]*[[:space:]]+function[[:space:]]' --include='*.asm' "$dir")
    ndefs=$(printf '%s' "$defs" | grep -c . )
    echo "    definitions: $ndefs"
    names=$(printf '%s\n' "$defs" | sed -E 's/^[^:]*:[0-9]+://' | sed -E 's/[[:space:]].*$//' | sort -u | grep -v '^$')
    echo "    distinct names: $(printf '%s\n' "$names" | grep -c .)"
    printf '%s\n' "$names" | sed 's/^/      /'

    echo
    echo "--- P1b function bodies that never mention a parameter"
    echo "    (a lazy expander folds these without evaluating the argument at all,"
    echo "     so an argument asl would refuse never reaches a refusal)"
    printf '%s\n' "$defs" | python3 "$HERE/param_ignoring.py" | sed 's/^/      /'

    echo
    echo "--- P2  <function name>(<register>) anywhere"
    p2total=0
    while IFS= read -r n; do
        [ -n "$n" ] || continue
        c=$(grep -rEc "(^|[^A-Za-z0-9_.])$n\\($REGS\\)" --include='*.asm' "$dir" 2>/dev/null | grep -v ':0$' | wc -l)
        hits=$(grep -rEo "(^|[^A-Za-z0-9_.])$n\\($REGS\\)" --include='*.asm' "$dir" 2>/dev/null | wc -l)
        echo "      $n: $hits use(s) in $c file(s)"
        p2total=$((p2total + hits))
    done <<< "$names"
    echo "    P2 TOTAL: $p2total"

    echo
    echo "--- P2b  of those, the ones NOT at the end of the operand"
    echo "    A trailing \`(An)\`/\`(An,Xn)\` group is PEELED by asl before anything is"
    echo "    evaluated, so \`name(a1)\` at the end of an operand is an addressing mode"
    echo "    and never a call — loud (\`error #1010\`) when the name is only a function."
    echo "    A hit with more expression after the \`)\` is not peeled and IS a call."
    p2b=0
    while IFS= read -r n; do
        [ -n "$n" ] || continue
        # A comma ENDS the operand just as whitespace and `;` do — `move.w
        # id(a0),d0` is a source operand and still a peeled addressing mode — so
        # it must be excluded here. Leaving it in counted 45 EA sites as calls.
        h=$(grep -rEo "(^|[^A-Za-z0-9_.])$n\($REGS\)[^[:space:];,]" --include='*.asm' "$dir" 2>/dev/null | wc -l)
        [ "$h" != 0 ] && echo "      $n: $h"
        p2b=$((p2b + h))
    done <<< "$names"
    echo "    P2b TOTAL: $p2b"

    echo
    echo "--- P3  dc.<size> whose whole operand is a bare register"
    echo "    cmd: grep -rEc '^[^;]*[[:space:]]dc\\.[bwl][[:space:]]+$REGS[[:space:]]*(;|$)'"
    p3=$(grep -rEo "[[:space:]]dc\.[bwlBWL][[:space:]]+$REGS[[:space:]]*(;|$)" --include='*.asm' "$dir" 2>/dev/null | wc -l)
    echo "    P3 TOTAL: $p3"

    echo
    echo "--- P4  #<name>(<register>) — the immediate shape, name-independent"
    echo "    cmd: grep -rEo '#[A-Za-z_.][A-Za-z0-9_.]*\\($REGS\\)'"
    p4=$(grep -rEo "#[A-Za-z_.][A-Za-z0-9_.]*\($REGS\)" --include='*.asm' "$dir" 2>/dev/null | wc -l)
    echo "    P4 TOTAL: $p4"
    if [ "$p4" != 0 ]; then
        grep -rEn "#[A-Za-z_.][A-Za-z0-9_.]*\($REGS\)" --include='*.asm' "$dir" 2>/dev/null | sed 's/^/      /'
    fi
    echo
done
echo "=== CORPUS ENUMERATION DONE ==="
