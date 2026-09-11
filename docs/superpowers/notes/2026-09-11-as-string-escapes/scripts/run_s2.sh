#!/usr/bin/env bash
# run_s2.sh <binary-name>... : the census's stub B and stub D trees (recipe:
# the census's own committed stub.py over a clean s2disasm copy plus build.lua's
# pre-step outputs), each assembled by every named binary in bin/, then the
# census's own windowed compare against the reference ROM.
set -u
S=/home/volence/sonic_hacks/.scratch/as-string-escapes
N=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a3c01b40c82d95852/docs/superpowers/notes/2026-09-11-s2-residual-census/scripts
cd "$S" || exit 9
echo "reference ROM $(md5sum < ref2out/b3-final.bin | cut -c1-32)  p-file $(md5sum < ref2out/s2.run1.p | cut -c1-32)"
python3 "$N/stub.py" "$S/corpus-gen" "$S/corpus-stubB" B || exit 5
python3 "$N/stub.py" "$S/corpus-gen" "$S/corpus-stubD" D || exit 5
run() { # binary tree out
  mkdir -p "$3"
  rm -f "$3/s2.sigil.bin"
  (cd "$2" && "$1" s2.asm -o "$3/s2.sigil.bin" > "$3/stdout" 2> "$3/stderr")
  echo "SIGIL_EXIT=$? sigil=$(md5sum < "$1" | cut -c1-32) tree=$(basename "$2") rows=$(wc -l < "$3/stderr")" > "$3/exit"
  [ -f "$3/s2.sigil.bin" ] && echo "image $(md5sum < "$3/s2.sigil.bin" | cut -c1-32) size=$(stat -c%s "$3/s2.sigil.bin")" >> "$3/exit"
  cat "$3/exit"
}
for b in "$@"; do
  for t in B D; do
    run "$S/bin/sigil-$b" "$S/corpus-stub$t" "$S/runs/$b-stub$t"
    python3 "$N/compare.py" "$S/runs/$b-stub$t/s2.sigil.bin" ref2out/b3-final.bin ref2out/s2.lst ref2out/z80blob.bin ref2out/s2.run1.p > "$S/runs/compare-$b-stub$t.txt" 2>&1
    echo "COMPARE $b stub$t exit=$?"
    /usr/bin/grep -E '^(WINDOW|CONTROL|image size|sigil bytes|DIFF|DRIVER|image bytes)' "$S/runs/compare-$b-stub$t.txt"
    echo "runs: $(/usr/bin/grep -c '^  \[0x' "$S/runs/compare-$b-stub$t.txt")"
  done
done
echo S2_END
