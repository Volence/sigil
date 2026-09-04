#!/usr/bin/env bash
# The RAM-layout byte sweep: rebuild sigil from the CURRENT tree, run the
# 1,553-symbol dump through it, and compare against asl's image.
set -uo pipefail
W=/home/volence/sonic_hacks/.sigil-struct
C=/home/volence/sonic_hacks/.s1-struct-s1disasm
CARGO_TARGET_DIR=$W/.target-land cargo build --release --manifest-path "$W/Cargo.toml" --bin sigil >/dev/null 2>&1 \
  || { echo "SWEEP: BUILD FAILED"; exit 3; }
rm -f /tmp/ramprobe.sig
( cd "$C" && "$W/.target-land/release/sigil" ramprobe.asm -o /tmp/ramprobe.sig >/tmp/ramprobe.diag 2>&1 )
se=$?
python3 - "$se" <<'PY'
import sys, zlib, os
se = sys.argv[1]
a = open('/home/volence/sonic_hacks/.s1-struct-s1disasm/ramprobe.bin','rb').read()
p = '/tmp/ramprobe.sig'
if not os.path.exists(p):
    print("SWEEP: RED (sigil produced no image; exit=%s)" % se)
    print(open('/tmp/ramprobe.diag').read()[:400]); raise SystemExit
s = open(p,'rb').read()
f = lambda d: '%08x/%d' % (zlib.crc32(d)&0xffffffff, len(d))
print("asl %s   sigil %s (exit=%s)" % (f(a), f(s), se))
if a == s:
    print("SWEEP: GREEN — 1553/1553 RAM symbols identical")
else:
    n = min(len(a), len(s))
    bad = [i for i in range(n) if a[i] != s[i]]
    print("SWEEP: RED — %d differing bytes of %d (first at %s)" % (len(bad), n, bad[:6]))
PY
