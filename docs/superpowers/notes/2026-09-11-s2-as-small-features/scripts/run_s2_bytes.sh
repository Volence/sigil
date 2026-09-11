#!/usr/bin/env bash
# run_s2_bytes.sh <binary-name>... : the Sonic 2 windowed byte proof.
#   stub H: this parcel's scaffold. Only the Z80 half-register lines (asl's own
#           listed bytes) and the driver placement are rewritten; every construct
#           this parcel implements is left in its ORIGINAL spelling.
#   stub B: the census's own tree (every refused class stubbed, driver moved),
#           the regression control: its image must not move.
# Each tree is assembled by every named binary from bin/, then the census's
# committed compare.py runs its asserted-window compare against the reference.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-as-small-features
N=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97/docs/superpowers/notes/2026-09-11-s2-residual-census/scripts
cd "$S" || exit 9
echo "reference ROM $(md5sum < ref2out/b3-final.bin | cut -c1-32)  p-file $(md5sum < ref2out/s2.run1.p | cut -c1-32)  listing $(md5sum < ref2out/s2.lst | cut -c1-32)  z80 $(md5sum < ref2out/z80blob.bin | cut -c1-32)"
python3 "$S/stub_h.py" "$S/corpus-gen" "$S/corpus-stubH" || exit 5
python3 "$N/stub.py" "$S/corpus-gen" "$S/corpus-stubB" B || exit 5
run() { # binary tree out
  mkdir -p "$3"
  rm -f "$3/s2.sigil.bin"
  (cd "$2" && "$1" s2.asm -o "$3/s2.sigil.bin" > "$3/stdout" 2> "$3/stderr")
  echo "SIGIL_EXIT=$? sigil=$(md5sum < "$1" | cut -c1-32) tree=$(basename "$2") rows=$(wc -l < "$3/stderr")" > "$3/exit"
  [ -f "$3/s2.sigil.bin" ] && echo "image $(md5sum < "$3/s2.sigil.bin" | cut -c1-32) size=$(stat -c%s "$3/s2.sigil.bin")" >> "$3/exit"
  cat "$3/exit"
}
for b in "$@"; do
  for t in H B; do
    run "$S/bin/sigil-$b" "$S/corpus-stub$t" "$S/runs/$b-stub$t"
    if [ -f "$S/runs/$b-stub$t/s2.sigil.bin" ]; then
      python3 "$N/compare.py" "$S/runs/$b-stub$t/s2.sigil.bin" ref2out/b3-final.bin ref2out/s2.lst ref2out/z80blob.bin ref2out/s2.run1.p > "$S/runs/compare-$b-stub$t.txt" 2>&1
      echo "COMPARE $b stub$t exit=$?"
      /usr/bin/grep -E '^(WINDOW|CONTROL|image size|sigil bytes|DIFF|DRIVER|image bytes)' "$S/runs/compare-$b-stub$t.txt"
      echo "runs listed: $(/usr/bin/grep -c '^  \[0x' "$S/runs/compare-$b-stub$t.txt")"
    else
      echo "NO IMAGE $b stub$t; stderr rows:"; head -20 "$S/runs/$b-stub$t/stderr"
    fi
  done
done
echo S2_BYTES_END
