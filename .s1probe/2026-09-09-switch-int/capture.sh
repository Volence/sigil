#!/bin/zsh
# Re-capture every probe's asl listing into asl-listings.txt, next to the probes.
#
# The oracle is asl 1.42 Beta [Bld 212], md5 61e672562465725a8c102288a7da9098,
# committed at s1disasm/build_tools/Linux-x86_64/asl. Each probe's listing is the
# source of the expected value quoted in the corresponding unit test in
# crates/sigil-frontend-as/src/eval.rs, so re-running this is how a reader checks
# that a quoted `=>TRUE` / byte column is still what the reference says.
set -e
ASL=${ASL:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl}
D=${0:a:h}
OUT=$D/asl-listings.txt
: > $OUT
{
  echo "asl banner:"
  $ASL 2>&1 | head -2
  echo "asl md5: $(md5sum $ASL | cut -d' ' -f1)"
  echo "invocation: asl -xx -n -q -A -L -U -E -i . <probe>.asm"
  echo
} >> $OUT
for f in $D/p*.asm; do
  n=${f:t:r}
  rm -f $D/$n.p $D/$n.lst
  # rc is taken from the run itself, not from a `|| true` that has already
  # replaced it: asl exits 2 on the probes that deliberately show a refused
  # shape, and reading those as 0 would hide the half of the evidence that says
  # the reference DECLINED rather than answered.
  rc=0
  ( cd $D && $ASL -xx -n -q -A -L -U -E -i . $n.asm > $D/$n.stdout 2> $D/$n.stderr ) || rc=$?
  {
    echo "=== $n  exit=$rc"
    sed -n '1,/Symbol Table/p' $D/$n.lst | sed '$d' | sed '/Source File .* Page [2-9]/d'
    echo
  } >> $OUT
  rm -f $D/$n.p $D/$n.lst $D/$n.stdout $D/$n.stderr $D/$n.log
done
echo "wrote $OUT"
