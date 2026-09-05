#!/bin/sh
# gen_org_both.sh <org-expr> <align-n>
# Run one isolated org+align case through BOTH asl builds present in the
# workspace and print each answer with the binary that produced it. A version
# string is not a binary identity: these two both say "1.42 Beta [Bld 212]".
#
# NO DIGEST PIN HERE, DELIBERATELY. Every other runner in this directory refuses
# any build but the reference one; this is the runner whose whole job is to put
# a question to both, so pinning it would delete the capability. What it owes
# instead is that neither answer is anonymous — the header line names each build
# by md5, which is the identity the banner cannot carry.
S2=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
S1=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
printf '# s2/flamewing %s/asl md5 %s\n' "$S2" "$(md5sum "$S2/asl" | cut -d' ' -f1)"
printf '# s1/upstream  %s/asl md5 %s\n' "$S1" "$(md5sum "$S1/asl" | cut -d' ' -f1)"
one() {
  ASLDIR="$1"; T=$(mktemp -d)
  printf '\tcpu\t68000\n\tpadding\toff\n\torg\t%s\n\talign\t%s\nT:\n' "$2" "$3" > "$T/t.asm"
  (cd "$T" && AS_MSGPATH="$ASLDIR" "$ASLDIR/asl" -xx -n -q -A -L -U -i . t.asm >/dev/null 2>&1)
  grep -E '^ +5/' "$T/t.lst" | head -1 | sed 's|^ *5/ *||;s| .*||'
  rm -rf "$T"
}
A=$(one "$S2" "$1" "$2"); B=$(one "$S1" "$1" "$2")
[ "$A" = "$B" ] && V=agree || V="*** DISAGREE ***"
printf '%-12s n=%-6s s2/flamewing=%-12s s1/upstream=%-12s %s\n' "$1" "$2" "$A" "$B" "$V"
