#!/usr/bin/env bash
# postcheck.sh: hand-run the exact asl and p2bin lines common.lua's assemble_file() runs
# for buildSK.lua (only through asl_ref.sh's asl_run), keep p2bin's raw output (no
# fix_header), then ask: does fix_header change any byte of it, and of sigil's image?
# Also the same asl line on the wrapper root, to show the wrapper is neutral for asl.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
ID="python3 $S/scripts/ident.py"
P2BIN="$ASLDIR/p2bin"
echo "asl: $ASL md5 $(md5sum "$ASL" | cut -d' ' -f1)"
echo "p2bin: $P2BIN md5 $(md5sum "$P2BIN" | cut -d' ' -f1)"
W=$S/trees/sk-post
rm -rf "$W"; cp -a "$S/trees/sk-gen" "$W"
cd "$W" || exit 1

echo "== A: the buildSK line: asl -D Sonic3_Complete=0 sonic3k.asm; p2bin -p=FF -z... =="
rm -f sonic3k.p sonic3k.log
asl_run -xx -n -q -A -L -U -E -i . -D Sonic3_Complete=0 sonic3k.asm; echo "asl rc=$?"
[ -f sonic3k.log ] && { echo "sonic3k.log present:"; cat sonic3k.log; } || echo "no sonic3k.log (no diagnostics)"
grep -E '^ +[0-9]+ passe?s?$' sonic3k.lst
"$P2BIN" -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before sonic3k.p raw-A.bin; echo "p2bin rc=$?"
$ID raw-A.bin
cp sonic3k.lst "$S/logs/asl-A-sonic3k.lst"

echo "== B: the same asl line on the wrapper root (no -D) =="
rm -f wrapper.p wrapper.log
asl_run -xx -n -q -A -L -U -E -i . wrapper.asm; echo "asl rc=$?"
[ -f wrapper.log ] && { echo "wrapper.log present:"; cat wrapper.log; } || echo "no wrapper.log (no diagnostics)"
"$P2BIN" -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before wrapper.p raw-B.bin; echo "p2bin rc=$?"
$ID raw-B.bin

echo "== fix_header, the toolchain's own lua, applied to copies =="
cp raw-A.bin fixed-A.bin
cp "$S/runs/wrapper/image.bin" fixed-sigil.bin
lua -e 'local c = require "build_tools.lua.common"; c.fix_header("fixed-A.bin"); c.fix_header("fixed-sigil.bin")'; echo "lua rc=$?"
$ID raw-A.bin fixed-A.bin raw-B.bin "$S/runs/wrapper/image.bin" fixed-sigil.bin "$S/ref.bin"
cp raw-A.bin fixed-A.bin raw-B.bin fixed-sigil.bin "$S/logs/"
echo POSTCHECK_END
