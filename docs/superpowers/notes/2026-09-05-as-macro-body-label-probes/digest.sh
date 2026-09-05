#!/usr/bin/env bash
# The reading half of all.sh: strip asl's page banners and its 100-line builtin
# symbol table down to (a) the diagnostic stream, (b) the emitted-byte lines,
# (c) any USER symbol asl's table actually carries. The full listings are large
# and almost entirely builtins; this is what the note quotes.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
for f in "$HERE"/p*.asm; do
    b="$(basename "$f" .asm)"
    echo "########## $b ##########"
    "$HERE/run.sh" "$b.asm" 2>&1 >/dev/null | sed -n '1,200p'
    "$HERE/run.sh" "$b.asm" 2>/dev/null | awk '
        /^ *[0-9]+\/ *[0-9A-F]+ :/ { print; next }
        /error #|warning #|^> > >/  { print; next }
        /^ASL_EXIT=/                { print; next }
    '
    echo "--- user symbols asl kept (non-builtin) ---"
    "$HERE/run.sh" "$b.asm" 2>/dev/null | sed -n '/Symbol Table/,/^$/p' | tr '|' '\n' \
        | grep -vE '\*?(ARCHITECTURE|BIGENDIAN|BRANCHEXT|CASESENSITIVE|COMPMODE|CONSTPI|CUSTOM|DATE|FALSE|FULLPMMU|HASFPU|HASPMMU|INEXTMODE|INLWORDMODE|INMAXMODE|INSRCMODE|INSUPMODE|LISTON|MACEXP|MOMCPU|MOMCPUNAME|MOMFILE|MOMLINE|MOMPASS|MOMSECTION|NESTMAX|PACKING|PADDING|RELAXED|TIME|TRUE|VERSION|WRAPMODE|Z80SYNTAX|HAS64|FPU|PMMU|SUPMODE|SRCMODE|LWORDMODE|MAXMODE|EXTMODE|TITLE|ARCH|OP16|OP32|OPSIZE)' \
        | grep -E '[A-Za-z]' | sed 's/^ *//;s/ *$//' | grep -v '^-*$' | grep -v '^Symbol Table' | grep -v '^[0-9 ]*$'
done
