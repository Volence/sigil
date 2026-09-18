"""Which Rust compressor is which p2bin format, measured on the p2bin binary.

The same fourteen generated inputs as the stage-2 vector set (`gen` below is the
generator `2026-09-11-s1-driver-stage2/gen_vectors.py` uses, and the Rust tests
reimplement), each placed as a Z80 blob with `binclude`, assembled by the pinned
asl, and handed to p2bin once per compressed format. The stored length is the
`comp_z80_size` p2bin writes into its header file; the stream is that many bytes
at the blob's ROM offset.

Prints one row per input: its CRC32, then `<format>=<size>/<crc32>` per format.
Those numbers are what `sigil-clownlzss-sys/tests/p2bin_optimised_vectors.rs`
and `accurate_vectors.rs` pin.
"""
import os, subprocess, sys

STAGE2 = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                      '2026-09-11-s1-driver-stage2')
sys.path.insert(0, STAGE2)
from ptools import asl, check_tools, crc, md5, P2BIN, ASL

D = os.environ.get('OPT_VECTOR_DIR')
if not D:
    raise SystemExit('set OPT_VECTOR_DIR to a scratch directory to work in')
os.makedirs(D, exist_ok=True)
check_tools()
M = (1 << 64) - 1


def gen(seed, n, alphabet):
    state = seed & M
    out = bytearray()

    def nxt():
        nonlocal state
        state = (state * 6364136223846793005 + 1442695040888963407) & M
        return state >> 33

    while len(out) < n:
        r = nxt()
        if len(out) > 8 and r % 3 == 0:
            dist = 1 + nxt() % min(len(out), 0x2100)
            length = (2 + nxt() % 300) if r % 7 == 0 else (2 + nxt() % 40)
            for _ in range(length):
                out.append(out[-dist])
        else:
            out.append(nxt() % alphabet)
    return bytes(out[:n])


VECTORS = [
    ('one', 1, 1, 256), ('two', 2, 2, 256), ('small', 3, 17, 16),
    ('odd_a', 4, 300, 7), ('odd_b', 5, 301, 7), ('odd_c', 6, 1000, 256),
    ('window', 7, 5000, 12), ('driver_sized', 8, 7110, 40),
    ('zeros_run', 9, 4100, 1), ('limit', 10, 0x2000, 30),
    ('p1', 11, 100, 3), ('p2', 12, 101, 3), ('p3', 13, 102, 3), ('p4', 14, 103, 3),
]

ASM = '''\tcpu 68000
\tpadding off
Guess = $C000
\torg 0
\tdc.b 1,2,3,4,5,6,7,8
Drv:
\tsave
\t!org 0
\tcpu z80
\tbinclude "vec.bin"
\trestore
\tpadding off
\t!org Drv+Guess
\tdc.b $AA,$BB
'''
FORMATS = ['kosinski', 'kosinski-optimised', 'saxman', 'saxman-optimised', 'saxman-bugged']

open(os.path.join(D, 'vec.asm'), 'w').write(ASM)
print('p2bin', md5(P2BIN), 'asl', md5(ASL))
for name, seed, n, alphabet in VECTORS:
    data = gen(seed, n, alphabet)
    open(os.path.join(D, 'vec.bin'), 'wb').write(data)
    open(os.path.join(D, name + '.in'), 'wb').write(data)
    asl(D, 'vec.asm')
    row = ['%-13s len=%-5d in=%s' % (name, n, crc(data))]
    for c in FORMATS:
        hdr = os.path.join(D, 'h.h')
        open(hdr, 'w').write(' ' * 64 + '\n')
        r = subprocess.run([P2BIN, '-p=FF', '-z=0,%s,Guess,after' % c, 'vec.p', 'o.bin', 'h.h'],
                           cwd=D, capture_output=True)
        assert r.returncode == 0, (name, c, r.stdout, r.stderr)
        size = int(open(hdr).read().split()[1], 16)
        stream = open(os.path.join(D, 'o.bin'), 'rb').read()[8:8 + size]
        open(os.path.join(D, '%s.%s' % (name, c)), 'wb').write(stream)
        row.append('%s=%d/%s' % (c, size, crc(stream)))
    print('  '.join(row))
