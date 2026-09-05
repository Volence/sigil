#!/bin/bash
# Run the sanctioned asl (md5 61e672562465725a8c102288a7da9098) on each named
# probe and report the exit status alongside the listing tail.
ASL=/home/volence/sonic_hacks/sonic_hack/tools/as/asl
D=/tmp/claude-1000/-home-volence-sonic-hacks-sigil/1a93ba92-b503-43b3-8939-b5973f7954ac/scratchpad/p
export AS_MSGPATH=/home/volence/sonic_hacks/sonic_hack/win32/msg
export USEANSI=n
cd "$D" || exit 9
md5sum "$ASL"
for f in "$@"; do
  echo "=== $f ==="
  cat "$f.asm"
  echo "--- asl output ---"
  rm -f "$f.p" "$f.lst"
  "$ASL" -cpu 68000 -L -olist "$f.lst" -o "$f.p" "$f.asm" 2>&1
  echo "ASL_EXIT=$?"
  if [ -f "$f.lst" ]; then
    echo "--- listing ---"
    cat "$f.lst"
  fi
done
