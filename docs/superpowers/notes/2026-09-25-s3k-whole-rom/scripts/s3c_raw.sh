#!/usr/bin/env bash
# s3c_raw.sh: buildS3Complete.lua's asl + p2bin lines by hand (asl_run only), keeping
# p2bin's RAW output (no fix_header), against sigil's image. sigil folds fix_header into
# its AS route (5d72e4a4), so any difference confined to 0x18E/0x1A4 is that fold.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
T=$S/trees/s3c-raw
rm -rf "$T"; cp -a "$S/trees/sk-gen" "$T"
cd "$T" || exit 1
rm -f sonic3k.p sonic3k.log
asl_run -xx -n -q -A -L -U -E -i . -D Sonic3_Complete=1 sonic3k.asm; echo "asl rc=$?"
[ -f sonic3k.log ] && { echo "sonic3k.log:"; head sonic3k.log; } || echo "no sonic3k.log"
"$ASLDIR/p2bin" -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before sonic3k.p raw.bin; echo "p2bin rc=$?"
cp raw.bin "$S/flips/asl-raw-s3c.bin"
python3 "$S/scripts/ident.py" "$S/flips/asl-raw-s3c.bin" "$S/runs/s3c/image.bin" "$S/flips/ref-s3c-run2.bin"
echo "--- raw asl+p2bin vs buildS3Complete.lua's fixed reference"
python3 "$S/scripts/diffranges.py" "$S/flips/asl-raw-s3c.bin" "$S/flips/ref-s3c-run2.bin"
echo "--- raw asl+p2bin vs sigil"
python3 "$S/scripts/diffranges.py" "$S/flips/asl-raw-s3c.bin" "$S/runs/s3c/image.bin"
python3 "$S/scripts/header.py" "$S/flips/asl-raw-s3c.bin"
echo S3CRAW_END
