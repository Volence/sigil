#!/usr/bin/env bash
# Assemble one probe with EVERY asl build that runs on this machine, three runs
# each, and print each build's md5 above its result. The banner is identical
# across all four, so the md5 is the only identification this prints.
#
# usage: all4.sh <probe.asm> [extended-regex]
#   The listing and stdout are filtered to lines matching the regex (default:
#   listing rows carrying a byte column or an `=` value, and every diagnostic
#   line), and the three runs of each build are compared: "stable" means the
#   filtered text was identical all three times, "VARIES" means it was not.
#
# Unguarded by design: the build is the subject, and every build it runs is
# named by digest above its output. It reads the listing, so a non-zero exit
# is printed first and the byte column of such a run is not an answer (see
# ../asl-reference/README.md, "The run: asl_run").
#
# Scratch goes under $WORK (default: .work beside this script), never /tmp.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="${WORK:-$HERE/.work}"
SRC="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
PAT="${2:-^ *[0-9]+/ *[0-9A-F]+ : [0-9A-F=(]|> > >|error|warning|Assertion|Abort}"
B=/home/volence/sonic_hacks
BUILDS="$B/s1disasm/build_tools/Linux-x86_64 $B/s2disasm/build_tools/Linux-x86_64 $B/s1disasm/build_tools/Linux-x86 $B/s2disasm/build_tools/Linux-x86"
name="$(basename "$SRC" .asm)"
for d in $BUILDS; do
    md5="$(md5sum "$d/asl" | cut -d' ' -f1)"
    echo "=== $name  md5 $md5  ($d/asl)"
    prev=""; verdict=stable
    for run in 1 2 3; do
        w="$WORK/$name/$md5/$run"
        mkdir -p "$w"
        rm -f "$w/$name".*
        cp "$SRC" "$w/"
        ( cd "$w" && AS_MSGPATH="$d" "$d/asl" -xx -n -q -A -L -U -i . "$name.asm" >"$name.out" 2>&1; echo "$?" >"$name.exit" )
        # The listing interleaves the diagnostics with the rows, so it is read
        # alone when it exists; stdout only when asl wrote no listing (a crash).
        if [ -f "$w/$name.lst" ]; then src="$w/$name.lst"; else src="$w/$name.out"; fi
        cur="$(grep -aE "$PAT" "$src" | grep -avE '^> > > +(~|[^ ]*$)'; grep -aE 'Assertion|Abort|Segmentation' "$w/$name.out")"
        if [ "$run" = 1 ]; then
            echo "exit $(cat "$w/$name.exit")"
            echo "$cur"
            prev="$cur"
        elif [ "$cur" != "$prev" ]; then
            verdict=VARIES
            echo "--- run $run differs (exit $(cat "$w/$name.exit")):"
            echo "$cur"
        fi
    done
    echo "runs: $verdict"
done
