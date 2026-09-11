"""Compressor test vectors whose expected outputs come from the pinned p2bin binary.

Each input is produced by a small deterministic generator that the Rust test
reimplements exactly (the input's CRC32 is pinned too, so a drift in either
generator is caught before any compressor is judged). Each input is placed as
a Z80 blob with `binclude`, assembled by the pinned asl, and handed to p2bin
with each compressor; the stored length comes from p2bin's own header file.
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptools import asl, check_tools, crc, md5, P2BIN, ASL
import subprocess

D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'vectors')
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
            if r % 7 == 0:
                length = 2 + nxt() % 300
            else:
                length = 2 + nxt() % 40
            for _ in range(length):
                out.append(out[-dist])
        else:
            out.append(nxt() % alphabet)
    return bytes(out[:n])


VECTORS = [
    # (name, seed, length, alphabet)
    ('one', 1, 1, 256),
    ('two', 2, 2, 256),
    ('small', 3, 17, 16),
    ('odd_a', 4, 300, 7),
    ('odd_b', 5, 301, 7),
    ('odd_c', 6, 1000, 256),
    ('window', 7, 5000, 12),
    ('driver_sized', 8, 7110, 40),
    ('zeros_run', 9, 4100, 1),
    ('limit', 10, 0x2000, 30),
    ('p1', 11, 100, 3),
    ('p2', 12, 101, 3),
    ('p3', 13, 102, 3),
    ('p4', 14, 103, 3),
]

ASM = '''	cpu 68000
	padding off
Guess = $C000
	org 0
	dc.b 1,2,3,4,5,6,7,8
Drv:
	save
	!org 0
	cpu z80
	binclude "vec.bin"
	restore
	padding off
	!org Drv+Guess
	dc.b $AA,$BB
'''
open(os.path.join(D, 'vec.asm'), 'w').write(ASM)
rows = []
for name, seed, n, alphabet in VECTORS:
    data = gen(seed, n, alphabet)
    open(os.path.join(D, 'vec.bin'), 'wb').write(data)
    open(os.path.join(D, name + '.in'), 'wb').write(data)
    asl(D, 'vec.asm')
    row = [name, seed, n, alphabet, crc(data)]
    for c in ['kosinski', 'saxman', 'saxman-bugged']:
        hdr = os.path.join(D, 'h.h')
        open(hdr, 'w').write(' ' * 64 + '\n')
        r = subprocess.run([P2BIN, '-p=FF', '-z=0,%s,Guess,after' % c, 'vec.p', 'o.bin', 'h.h'],
                           cwd=D, capture_output=True)
        assert r.returncode == 0, (name, c, r.stdout, r.stderr)
        size = int(open(hdr).read().split()[1], 16)
        rom = open(os.path.join(D, 'o.bin'), 'rb').read()
        stream = rom[8:8 + size]
        open(os.path.join(D, '%s.%s' % (name, c)), 'wb').write(stream)
        row += [size, crc(stream)]
        if c == 'saxman-bugged':
            plain = open(os.path.join(D, '%s.saxman' % name), 'rb').read()
            row.append('bugged = saxman + %s (saxman len %d, %s)' % (
                stream[len(plain):].hex(), len(plain), 'odd' if len(plain) % 2 else 'even'))
            assert stream[:len(plain)] == plain
    rows.append(row)
print('p2bin', md5(P2BIN), 'asl', md5(ASL))
for r in rows:
    print(r)
