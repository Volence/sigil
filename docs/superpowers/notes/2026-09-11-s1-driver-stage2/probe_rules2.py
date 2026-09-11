"""Second round: compressors, -z address forms, CPU continuation, overflow output,
and the header file. Each run gets its own freshly assembled .p."""
import os, shutil, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptools import asl, p2bin, records, check_tools, md5, P2BIN

D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'probes')
check_tools()
for p in ['p_after', 'p_cont68k', 'p_kos', 'p_hex', 'p_big']:
    asl(D, p + '.asm')
    print('==', p, 'records:', [(hex(c), hex(s), len(b)) for c, s, b in records(os.path.join(D, p + '.p'))])


def run(label, args, probe='p_after', out='o.bin'):
    # A fresh copy of the .p per run, so no run can see another run's side effects.
    shutil.copy(os.path.join(D, probe + '.p'), os.path.join(D, 'in.p'))
    rc, text, data = p2bin(D, args + ['in.p', out], out)
    shown = data.hex(' ') if data is not None else None
    if shown and len(shown) > 400:
        shown = shown[:400] + ' ...(%d bytes)' % len(data)
    print('%-40s rc=%d out=%s' % (label, rc, shown))
    for line in (text.splitlines() if text else [])[:4]:
        print('      | ' + line)
    return rc, data


print('\n# -z address forms (driver at Z80 0x10)')
for a in ['10', '0x10', '010', '10j', '+10', ' 10', '10 ', '0x', '-0']:
    run('-z=%r' % a, ['-p=FF', '-z=%s,uncompressed,Guess,after' % a], 'p_hex')

print('\n# CPU continuation: a 68000 record contiguous with the Z80 run')
run('p_cont68k after', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_cont68k')

print('\n# overflow leaves which output?')
open(os.path.join(D, 'keep.bin'), 'wb').write(b'JUNKJUNK')
shutil.copy(os.path.join(D, 'p_big.p'), os.path.join(D, 'in.p'))
r = subprocess.run([P2BIN, '-p=FF', '-z=0,uncompressed,Guess,after', 'in.p', 'keep.bin'], cwd=D, capture_output=True)
kp = os.path.join(D, 'keep.bin')
print('overflow over a pre-existing output: rc=%d exists=%s content=%r' % (
    r.returncode, os.path.exists(kp), open(kp, 'rb').read()[:16] if os.path.exists(kp) else None))

print('\n# header file (third positional)')
open(os.path.join(D, 'h.h'), 'w').write('0123456789abcdefghijklmnopqrstuvwxyz\nsecond line\n')
shutil.copy(os.path.join(D, 'p_kos.p'), os.path.join(D, 'in.p'))
r = subprocess.run([P2BIN, '-p=FF', '-z=0,kosinski,Guess,after', 'in.p', 'o.bin', 'h.h'], cwd=D, capture_output=True)
print('rc=%d header now=%r' % (r.returncode, open(os.path.join(D, 'h.h')).read()))

print('\n# compressors on p_kos (driver %d bytes, gap 0x200)' % len(records(os.path.join(D, 'p_kos.p'))[1][2]))
drv = records(os.path.join(D, 'p_kos.p'))[1][2]
open(os.path.join(D, 'p_kos.z80'), 'wb').write(drv)
for c in ['uncompressed', 'kosinski', 'kosinski-optimised', 'saxman', 'saxman-bugged', 'saxman-optimised', 'kosinskiplus']:
    rc, data = run(c, ['-p=FF', '-z=0,%s,Guess,after' % c], 'p_kos')
    if data is not None:
        blob = data[8:8 + 0x200]
        # the blob is followed by pad FF up to the next record; report the non-pad prefix length
        end = len(blob.rstrip(b'\xff'))
        open(os.path.join(D, 'p_kos.%s.bin' % c), 'wb').write(blob[:end])
        print('      stored stream ends at +0x%X (trailing FF pad stripped; a stream may itself end in FF)' % end)
