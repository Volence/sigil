#!/bin/zsh
# Run one probe through asl and print exit status, stderr and the listing.
ASL=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
D=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a903dfc1b67f61ad8/.probework
n=$1
rm -f $D/$n.p $D/$n.lst $D/$n.out $D/$n.err
( cd $D && $ASL -xx -n -q -A -L -U -E -i . $n.asm >$n.out 2>$n.err )
rc=$?
echo "### $n  exit=$rc"
echo "--- stdout:"
cat $D/$n.out
echo "--- stderr:"
cat $D/$n.err
echo "--- listing:"
cat $D/$n.lst
