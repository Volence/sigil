#!/bin/sh
# Run one probe against the reference assembler, three times, and print each run
# verbatim. Usage: run.sh <probe-stem>
set -u
DIR=$(dirname "$0")
TOOLS=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
AS_MSGPATH=$TOOLS
export AS_MSGPATH
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
