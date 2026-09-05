#!/bin/sh
# gen_org.sh <org-expr> <align-n> [extra-lines-file]
# One isolated org+align probe. Prints "org  n  -> PC-at-label".
# One case per file on purpose: repeated org+align in a single assembly
# perturbs the answer, so every case gets its own run.
#
# THE ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER — see
# `../../notes/asl-reference/`. `gen_org_both.sh` beside this file is the one
# that deliberately runs a second build, and it names each by digest in its own
# output; this one has a single instrument and pins it.
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/../../notes/asl-reference/asl_ref.sh" || exit $?
T=$(mktemp -d)
printf '\tcpu\t68000\n\tpadding\toff\n\torg\t%s\n\talign\t%s\nT:\n' "$1" "$2" > "$T/t.asm"
(cd "$T" && "$ASLDIR/asl" -xx -n -q -A -L -U -i . t.asm >/dev/null 2>&1)
BEFORE=$(grep -E '^ +4/' "$T/t.lst" | head -1 | sed 's|^ *4/ *||;s| .*||')
AFTER=$(grep -E '^ +5/' "$T/t.lst" | head -1 | sed 's|^ *5/ *||;s| .*||')
printf '%-14s n=%-6s  before=%-18s after=%s\n' "$1" "$2" "$BEFORE" "$AFTER"
rm -rf "$T"
