#!/bin/sh
# Run one probe against the reference assembler, three times, and print each run
# verbatim. Usage: run.sh <probe-stem>
#
# THE THREE RUNS CARRY NO CLOCK READING, AND THAT IS AN ACCIDENT OF THE FLAGS.
# asl stamps the wall clock into its output in three places — the page banner's
# date and time, the DATE/TIME builtins in the symbol table, and
# `N.NN seconds assembly time` in the trailer — and all three live in the
# LISTING, which `-L` writes and `-q` keeps off stdout. This invocation passes
# `-q` and no `-L`, so the only stream it prints is stderr, which carries none of
# them: comparing the three runs here compares asl and nothing else.
#
# Add `-L` (or drop `-q`) and that stops being true: the trailer's duration is a
# DURATION, so a batch straddling a tick reports the assembler disagreeing with
# itself when only the clock moved. Filter the stream through
# `../asl-declock/declock.sed` if this runner ever grows a listing.
#
# THE ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER — see `../asl-reference/`.
# It matters here for the same reason the three runs do: this runner exists to
# show asl agreeing with itself, and the build it used to name disagrees with
# itself on any operand it declined to give a value. Three identical runs from
# THAT binary would have been a statement about which operands happened to
# resolve, not about the assembler.
set -u
DIR=$(cd "$(dirname "$0")" && pwd)
. "$DIR/../asl-reference/asl_ref.sh" || exit $?
TOOLS="$ASLDIR"
cd "$DIR" || exit 1
for run in 1 2 3; do
	echo "=== $1 run $run ==="
	rm -f "$1.p" "$1.bin"
	"$TOOLS/asl" -U -q "$1.asm" 2>&1
	echo "asl exit=$?"
	if [ -f "$1.p" ]; then
		"$TOOLS/p2bin" "$1.p" "$1.bin" -k >/dev/null 2>&1
		if [ -f "$1.bin" ]; then
			echo -n "image:"
			od -An -tx1 "$1.bin"
		else
			echo "image: (none)"
		fi
	else
		echo "image: (no .p)"
	fi
done
rm -f "$1.p" "$1.bin"
