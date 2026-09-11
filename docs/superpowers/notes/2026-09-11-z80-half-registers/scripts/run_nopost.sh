#!/usr/bin/env bash
# run_nopost.sh: build.lua with its two post-p2bin steps (amend_sound_driver_size
# and fix_header) removed, in its own cp -a copy, so the p2bin image can be
# compared with build.lua's final ROM. Leaves s2.h (the share file) behind.
set -u
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
D=$S/s2
mkdir -p "$D"
rm -rf "$D/nopost"
cp -a /home/volence/sonic_hacks/s2disasm "$D/nopost"
python3 - "$D/nopost/build.lua" "$D/build_nopost.lua" <<'EOF'
import sys
t = open(sys.argv[1]).read()
for call in ["\namend_sound_driver_size()\n", '\ncommon.fix_header("s2built.bin")\n']:
    assert t.count(call) == 1, call
    t = t.replace(call, "\n")
open(sys.argv[2], "w").write(t)
print("build_nopost.lua written: both post-p2bin calls removed")
EOF
cd "$D/nopost" || exit 9
lua "$D/build_nopost.lua" > "$D/nopost-build.log" 2>&1
echo "build_nopost exit=$?"
echo "s2.h:"; cat s2.h; echo
python3 - <<'EOF'
import zlib, hashlib
b = open("s2built.bin", "rb").read()
print(f"P2BIN-ONLY s2built.bin md5 {hashlib.md5(b).hexdigest()} crc32 {zlib.crc32(b):08x} size {len(b)}")
print(f"word at 0xEC050 = {b[0xEC050]:02X}{b[0xEC051]:02X}; header 0x18E = {b[0x18E]:02X}{b[0x18F]:02X}; 0x1A4 = {b[0x1A4:0x1A8].hex()}")
EOF
echo NOPOST_END
