#!/usr/bin/env bash
# mk_s12.sh: the Sonic 1 / Sonic 2 gen trees for the identity runs: a copy of the
# census's gen trees, checked against a fresh `git archive` of the census revisions
# (the only differences must be the build.lua pre-step outputs).
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
C=/home/volence/sonic_hacks/.scratch/as-corpus-census/trees
mkdir -p "$S/trees"
for p in "s1disasm f6ece657c1cf253404312137dfcb8ec15fa42318" "s2disasm e45ebf332f39987424ca3102e50c717628f71269"; do
  set -- $p; n=$1; rev=$2
  rm -rf "$S/trees/$n-pristine" "$S/trees/$n-gen"
  mkdir -p "$S/trees/$n-pristine"
  git -C "/home/volence/sonic_hacks/$n" archive --format=tar "$rev" | tar -x -C "$S/trees/$n-pristine"
  cp -a "$C/$n-gen" "$S/trees/$n-gen"
  echo "== $n $rev: pristine $(find "$S/trees/$n-pristine" -type f | wc -l) files, gen $(find "$S/trees/$n-gen" -type f | wc -l) files"
  echo "   differing (content) files: $(diff -rq "$S/trees/$n-pristine" "$S/trees/$n-gen" | grep -c '^Files')"
  echo "   only in gen: $(diff -rq "$S/trees/$n-pristine" "$S/trees/$n-gen" | grep -c "^Only in $S/trees/$n-gen")"
  echo "   only in pristine: $(diff -rq "$S/trees/$n-pristine" "$S/trees/$n-gen" | grep -c "^Only in $S/trees/$n-pristine")"
  diff -rq "$S/trees/$n-pristine" "$S/trees/$n-gen" | sed "s|$S/trees/||g" | head -12
done
echo MKS12_END
