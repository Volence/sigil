#!/bin/bash
# asl reference values for the AS-MOMPASS parcel, and the sigil answer beside
# each. Every asl invocation's exit status is printed; a run carrying any error
# is not a source of values and is marked so at the row.
set -u
ASL=/home/volence/sonic_hacks/sonic_hack/tools/as/asl
SIGIL=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f/.target-land/release/sigil
D=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f/.mompassprobe
cd "$D" || exit 9

echo "asl   md5: $(md5sum $ASL)"
echo "sigil md5: $(md5sum $SIGIL)"
echo

for f in m_eq1 m_gt1 m_eq2 m_eq3 m_val m_cmpd m_fatal m_msg pA pB pI; do
  rm -f "$f.lst" "$f.p"
  aslout=$("$ASL" -xx -n -A -L -U -E -i . "$f.asm" 2>&1); arc=$?
  passes=$(printf '%s\n' "$aslout" | grep -oE '[0-9]+ pass(es)?' | tail -1)
  errs=$(printf '%s\n' "$aslout" | grep -oE '[0-9]+ error(s)?' | tail -1)
  # The emitted bytes, read out of the listing's code column.
  bytes=$(grep -E '^ +[0-9]+/ +[0-9A-F]+ : [0-9A-F]' "$f.lst" 2>/dev/null \
          | sed -E 's/^ +[0-9]+\/ +[0-9A-F]+ : //' | cut -c1-20 | tr -d ' \n')
  msgs=$(printf '%s\n' "$aslout" | grep -cE 'first pass only')
  sout=$("$SIGIL" "$f.asm" --hex 2>"$D/$f.sigerr"); src=$?
  sbytes=$(printf '%s' "$sout" | tr -d ' \n')
  smsg=$(grep -cE 'first pass only' "$D/$f.sigerr")
  echo "$f"
  echo "   asl   : exit=$arc  $passes  $errs  bytes=$bytes  author-msg-lines=$msgs"
  echo "   sigil : exit=$src  bytes=$sbytes  author-msg-lines=$smsg"
  if [ -s "$D/$f.sigerr" ]; then
    echo "   sigil diagnostics: $(head -2 "$D/$f.sigerr" | tr '\n' ' ')"
  fi
done
echo REFVALS-END
