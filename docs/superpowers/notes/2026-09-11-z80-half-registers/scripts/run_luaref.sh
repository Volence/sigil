#!/usr/bin/env bash
# run_luaref.sh: the Sonic 2 reference ROM, built by s2disasm's own build.lua,
# unmodified, in a cp -a copy. The shared checkout is only read; its status is
# counted before and after.
set -u
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
D=$S/s2
mkdir -p "$D"
echo "s2disasm HEAD $(git -C /home/volence/sonic_hacks/s2disasm rev-parse HEAD)"
echo "s2disasm status lines before: $(git -C /home/volence/sonic_hacks/s2disasm status --porcelain --ignored | wc -l)"
rm -rf "$D/luaref"
cp -a /home/volence/sonic_hacks/s2disasm "$D/luaref"
cd "$D/luaref" || exit 9
md5sum build_tools/Linux-x86_64/asl build_tools/Linux-x86_64/p2bin build_tools/Linux-x86_64/saxman /usr/bin/lua
lua build.lua > "$D/luaref-build.log" 2>&1
echo "build.lua exit=$?"
ls -l s2built.bin
python3 - <<'EOF'
import zlib, hashlib
b = open("s2built.bin", "rb").read()
print(f"REF s2built.bin md5 {hashlib.md5(b).hexdigest()} crc32 {zlib.crc32(b):08x} size {len(b)}")
EOF
echo "s2disasm status lines after: $(git -C /home/volence/sonic_hacks/s2disasm status --porcelain --ignored | wc -l)"
echo LUAREF_END
