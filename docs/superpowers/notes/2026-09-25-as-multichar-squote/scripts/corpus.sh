#!/usr/bin/env bash
# corpus.sh <label> <sigil>: assemble S1, S2, S3K, S3C and S3 alone with <sigil> on the
# gen trees and compare each whole image against its luaref reference (compare.py,
# planted-byte control). Every run's provenance, exit, rows and identity are logged.
set -u
S=/home/volence/sonic_hacks/.scratch/as-squote
T=$S/trees
LABEL="$1"; SIGIL="$2"
echo "== corpus run $LABEL: pwd=$(pwd) date=$(date -Is)"
echo "sigil: $SIGIL md5 $(md5sum "$SIGIL" | cut -d' ' -f1)"
"$SIGIL" --version | sed -n 1,5p
run() { # run <tag> <tree> <root> <ref> [args...]
  local tag="$1" tree="$2" root="$3" ref="$4"; shift 4
  local out="$S/runs/$LABEL/$tag"
  rm -rf "$out"; mkdir -p "$out"
  ( cd "$tree" && "$SIGIL" "$root" -o "$out/image.bin" "$@" ) > "$out/stdout" 2> "$out/stderr"
  local rc=$?
  grep -E '(error|warning):' "$out/stderr" | sort > "$out/rows"
  echo "-- $tag: SIGIL_EXIT=$rc rows=$(wc -l < "$out/rows") args=$*"
  grep -c "error:" "$out/rows" | sed 's/^/   errors: /'
  if [ -f "$out/image.bin" ]; then
    python3 "$S/scripts/ident.py" "$out/image.bin"
    python3 "$S/scripts/compare.py" "$S/refs/$ref" "$out/image.bin"; echo "   compare rc=$?"
  else
    echo "   no image"
  fi
}
run s1 "$T/s1-gen" sonic.asm s1-run1.bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
run s2 "$T/s2-gen" s2.asm s2-run1.bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
run sk "$T/sk-gen" wrapper.asm sk-run1.bin -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
run s3c "$T/sk-gen" wrapper1.asm s3c-run1.bin -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
run s3 "$T/sk-gen" s3.asm s3-run3.bin -p=FF -z=0,uncompressed,Size_of_Snd_driver_guess,before -z=1300,uncompressed,Size_of_Snd_driver2_guess,before
echo "CORPUS_END_$LABEL"
