#!/usr/bin/env bash
# setup_corpora.sh: cp -a copies of the three disassemblies, each with its
# build script's pre-steps run in the copy. The shared checkouts are only read.
set -u
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
D=$S/corpora
mkdir -p "$D"
for r in s1disasm s2disasm skdisasm; do
    echo "$r HEAD $(git -C /home/volence/sonic_hacks/$r rev-parse HEAD) status-lines $(git -C /home/volence/sonic_hacks/$r status --porcelain --ignored | wc -l)"
    rm -rf "$D/$r"
    cp -a /home/volence/sonic_hacks/$r "$D/$r"
done
# Sonic 2: build.lua up to (not including) "-- Build the ROM.": song compression
# and the PCM/DPCM conversions.
python3 - /home/volence/sonic_hacks/s2disasm/build.lua "$S/prestep_s2.lua" <<'EOF'
import sys
t = open(sys.argv[1]).read()
i = t.index("-- Build the ROM.")
open(sys.argv[2], "w").write(t[:i])
print("prestep_s2.lua: build.lua bytes [0, %d)" % i)
EOF
(cd "$D/s2disasm" && lua "$S/prestep_s2.lua" > "$D/s2-prestep.log" 2>&1; echo "s2 prestep exit=$?")
(cd "$D/s2disasm" && find sound/music/generated sound/PCM/generated sound/DAC/generated -type f | wc -l | sed 's/^/s2 generated files: /')
# Sonic 1: build.lua's sample conversion, as the Sonic 1 corpus test runs it.
(cd "$D/s1disasm" && lua -e 'local common = require "build_tools.lua.common"; common.convert_pcm_files_in_directory("sound/dac/pcm"); common.convert_dpcm_files_in_directory("sound/dac/dpcm")' > "$D/s1-prestep.log" 2>&1; echo "s1 prestep exit=$?")
for r in s1disasm s2disasm skdisasm; do
    echo "$r status-lines after $(git -C /home/volence/sonic_hacks/$r status --porcelain --ignored | wc -l)"
done
echo SETUP_END
