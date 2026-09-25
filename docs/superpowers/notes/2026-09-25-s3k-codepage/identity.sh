#!/usr/bin/env bash
# identity.sh: cmp every shape base vs after (each with the planted-byte control),
# then put the golden ROMs back in AEON_DIR (four_shapes.sh deletes them) and CRC them.
S=/home/volence/sonic_hacks/.scratch/s3k-codepage
A=/home/volence/sonic_hacks/.aeon-s3k-codepage
G=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a08cead7e136bc593/crates/sigil-harness/golden
cd "$S/aeon" || exit 9
for f in sonic4.s4.bin sonic4-debug.s4.debug.bin demo.demo.bin demo-debug.demo.debug.bin; do
  echo "== $f"
  python3 "$S/compare.py" "base/$f" "after/$f"; echo "compare rc=$?"
done
for f in s4.bin s4.debug.bin demo.bin demo.debug.bin; do
  cp "$G/$f" "$A/$f"
  python3 -c 'import zlib,sys;d=open(sys.argv[1],"rb").read();print("restored",sys.argv[1],"%08x"%(zlib.crc32(d)&0xffffffff),len(d))' "$A/$f"
done
echo IDENTITY_END
