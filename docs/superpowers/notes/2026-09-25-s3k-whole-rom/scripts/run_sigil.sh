#!/usr/bin/env bash
# run_sigil.sh <tag> <tree> <root.asm> [sigil args...]: assemble with this parcel's sigil,
# record provenance, exit, streams, image identity.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
SIGIL=${SIGIL:-$S/target/release/sigil}
TAG="$1"; TREE="$2"; ROOT="$3"; shift 3
OUT=$S/runs/$TAG
rm -rf "$OUT"; mkdir -p "$OUT"
{
  echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)"
  echo "sigil rev: $("$SIGIL" --version | sed -n 2p)"
  echo "tree: $TREE root: $ROOT"
  echo "args: $*"
  echo "uptime: $(uptime)"
} > "$OUT/provenance"
T0=$(date +%s.%N)
( cd "$TREE" && "$SIGIL" "$ROOT" -o "$OUT/image.bin" "$@" ) > "$OUT/stdout" 2> "$OUT/stderr"
RC=$?
T1=$(date +%s.%N)
echo "SIGIL_EXIT=$RC elapsed=$(python3 -c "print(round($T1 - $T0, 2))")" > "$OUT/exit"
echo "stderr lines: $(wc -l < "$OUT/stderr")" >> "$OUT/exit"
grep -E '(error|warning):' "$OUT/stderr" | sort > "$OUT/rows"
echo "rows: $(wc -l < "$OUT/rows")" >> "$OUT/exit"
[ -f "$OUT/image.bin" ] && python3 "$S/scripts/ident.py" "$OUT/image.bin" >> "$OUT/exit"
cat "$OUT/provenance" "$OUT/exit"
echo "RUN_END_$TAG"
