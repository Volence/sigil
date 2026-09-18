"""The expected CLI-test images for p2bin's optimised formats, and for plain
saxman, each written by p2bin from asl's object file for the same probe source.

The probe sources are the stage-2 ones (`2026-09-11-s1-driver-stage2/probes`),
copied into a scratch directory so this leaves them untouched. Prints, per case,
the full image in hex when it is short and its size and CRC32 otherwise, plus
p2bin's exit and message. Those numbers are what
`sigil-cli/tests/as_driver_placement.rs` pins.
"""
import os, shutil, sys

STAGE2 = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                      '2026-09-11-s1-driver-stage2')
sys.path.insert(0, STAGE2)
from ptools import asl, p2bin, check_tools, md5, crc, P2BIN, ASL

D = os.environ.get('OPT_PROBE_DIR')
if not D:
    raise SystemExit('set OPT_PROBE_DIR to a scratch directory to work in')
if os.path.isdir(D):
    shutil.rmtree(D)
shutil.copytree(os.path.join(STAGE2, 'probes'), D)
check_tools()
print('p2bin', md5(P2BIN), 'asl', md5(ASL))

CASES = []
for f in ['kosinski', 'kosinski-optimised', 'saxman', 'saxman-optimised', 'saxman-bugged']:
    for probe in ['p_after', 'p_kos', 'p_straddle']:
        CASES.append((probe, ['-p=FF', '-z=0,%s,Guess,after' % f]))
for f in ['kosinski-optimised', 'saxman-optimised']:
    CASES.append(('p_s3k', ['-p=FF', '-z=0,%s,Size_of_Snd_driver_guess,before' % f,
                            '-z=1300,%s,Size_of_Snd_driver2_guess,before' % f]))

for probe, args in CASES:
    asl(D, probe + '.asm')
    shutil.copy(os.path.join(D, probe + '.p'), os.path.join(D, 'in.p'))
    rc, text, data = p2bin(D, args + ['in.p', 'o.bin'])
    print('== %s %s' % (probe, ' '.join(args)))
    print('   rc=%d %s' % (rc, text))
    if data is not None:
        if len(data) <= 80:
            print('   hex  %s' % data.hex())
        print('   size %d crc32 %s' % (len(data), crc(data)))
