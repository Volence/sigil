#!/usr/bin/env bash
# s3k_define.sh <tip-sigil> <base-sigil>: S&K and S3 Complete from the PLAIN root
# sonic3k.asm with -D, against references re-derived by the build scripts
# themselves in a git archive copy of skdisasm 2fcd861c.
set -u
TIP="$1"; BASE="$2"
S=/home/volence/sonic_hacks/.scratch/as-cli-define/s3k
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d
ID="python3 $S/scripts/ident.py"
CMP="python3 $S/scripts/compare.py"
SK=/home/volence/sonic_hacks/skdisasm
REV=2fcd861c208f342b6d14df694c6422c74f20a4be
echo "sigil worktree: pwd=$W HEAD=$(git -C "$W" rev-parse HEAD) branch=$(git -C "$W" branch --show-current)"
echo "tip:  $($TIP --version | sed -n '1,2p' | tr '\n' ' ') md5 $(md5sum "$TIP" | cut -d' ' -f1)"
echo "base: $($BASE --version | sed -n '1,2p' | tr '\n' ' ') md5 $(md5sum "$BASE" | cut -d' ' -f1)"
mkdir -p "$S/trees" "$S/runs" "$S/logs"
rm -rf "$S/trees/"*

# --- the source: git archive of the pinned revision ---
git -C "$SK" archive "$REV" > "$S/sk.tar"
$ID "$S/sk.tar"
mkdir -p "$S/trees/pristine"; tar -x -C "$S/trees/pristine" -f "$S/sk.tar"

# --- references: each build script, unmodified, three runs in its own copy ---
for spec in "sk:buildSK.lua:skbuilt.bin" "s3c:buildS3Complete.lua:sonic3k.bin"; do
  IFS=: read -r tag script out <<< "$spec"
  T=$S/trees/ref-$tag
  cp -a "$S/trees/pristine" "$T"
  grep -n -- '-D Sonic3_Complete' "$T/$script" | sed "s/^/  $script: /"
  for i in 1 2 3; do
    ( cd "$T" && lua "$script" ) > "$S/logs/ref-$tag-$i.log" 2>&1; echo "ref $tag run $i rc=$?"
    $ID "$T/$out"
    cp "$T/$out" "$S/runs/ref-$tag-run$i.bin"
  done
done

# --- the input tree: pristine plus the generated PCM/DAC the scripts' pre-steps write ---
cp -a "$S/trees/pristine" "$S/trees/gen"
for d in Sound/DAC/generated Sound/PCM/generated; do
  mkdir -p "$S/trees/gen/$d"; cp -a "$S/trees/ref-sk/$d/." "$S/trees/gen/$d/"
done
echo "gen tree: $(find "$S/trees/gen" -type f | wc -l) files, $(find "$S/trees/gen" -name 'wrapper*' | wc -l) wrapper roots"

P2BIN=(-p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before)
sig() {  # sig <tag> <binary> <args...>
  local tag="$1" bin="$2"; shift 2
  rm -f "$S/runs/$tag.bin"
  ( cd "$S/trees/gen" && "$bin" sonic3k.asm -o "$S/runs/$tag.bin" "$@" ) > "$S/logs/$tag.stdout" 2> "$S/logs/$tag.stderr"
  local rc=$?
  echo "$tag: exit $rc, stderr lines $(wc -l < "$S/logs/$tag.stderr"), error rows $(grep -c ': error: \|^error: ' "$S/logs/$tag.stderr")"
  [ -f "$S/runs/$tag.bin" ] && $ID "$S/runs/$tag.bin"
}

# --- acceptance: tip, plain root, three runs each ---
for i in 1 2 3; do
  sig "tip-sk-$i" "$TIP" -D Sonic3_Complete=0 "${P2BIN[@]}"
  $CMP "$S/runs/ref-sk-run$i.bin" "$S/runs/tip-sk-$i.bin"; echo "compare sk run $i rc=$?"
done
for i in 1 2 3; do
  sig "tip-s3c-$i" "$TIP" -D Sonic3_Complete=1 "${P2BIN[@]}"
  $CMP "$S/runs/ref-s3c-run$i.bin" "$S/runs/tip-s3c-$i.bin"; echo "compare s3c run $i rc=$?"
done
# asl's spelling order: -D first, as common.lua writes it, is covered above; the flag
# after the p2bin options gives the same image.
sig "tip-sk-late" "$TIP" "${P2BIN[@]}" -D Sonic3_Complete=0
$CMP "$S/runs/ref-sk-run1.bin" "$S/runs/tip-sk-late.bin"; echo "compare sk late-flag rc=$?"

# --- controls ---
# the plain root without -D: the 17 rows the wrapper existed for
sig "tip-sk-noD" "$TIP" "${P2BIN[@]}"
grep -c 'Sonic3_Complete\|strip_padding\|LockonHeader' "$S/logs/tip-sk-noD.stderr" | sed 's/^/  rows naming Sonic3_Complete, strip_padding or LockonHeader: /'
# the base binary does not take -D at all
sig "base-sk-D" "$BASE" -D Sonic3_Complete=0 "${P2BIN[@]}"
head -2 "$S/logs/base-sk-D.stderr" | sed 's/^/  /'
# the value is live: -D Sonic3_Complete=1 on the S&K reference differs
$CMP "$S/runs/ref-sk-run1.bin" "$S/runs/tip-s3c-1.bin" | sed -n '1,3p'
echo S3K_DEFINE_END
