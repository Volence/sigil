"""The expected images for the CLI tests, each written by p2bin from asl's object
file for the same probe source. Prints, per case, the full image in hex when it
is short and its size and CRC32 otherwise, plus p2bin's exit and message."""
import os, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptools import asl, p2bin, check_tools, md5, crc, P2BIN, ASL

S = os.path.dirname(os.path.abspath(__file__))
D = os.path.join(S, 'probes')
check_tools()
print('p2bin', md5(P2BIN), 'asl', md5(ASL))
CASES = [
    ('p_after', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_after', ['-p=0', '-z=0,uncompressed,Guess,after']),
    ('p_after', ['-p=FF', '-z=0,kosinski,Guess,after']),
    ('p_after', ['-p=FF', '-z=0,saxman-bugged,Guess,after']),
    ('p_gap', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_before', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_two', ['-p=FF', '-z=0,uncompressed,Guess,before', '-z=1300,uncompressed,Guess2,before']),
    ('p_last', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_hex', ['-p=FF', '-z=10,uncompressed,Guess,after']),
    ('p_kos', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_kos', ['-p=FF', '-z=0,kosinski,Guess,after']),
    ('p_kos', ['-p=FF', '-z=0,saxman-bugged,Guess,after']),
    ('p_big', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_s3k', ['-p=FF', '-z=0,kosinski,Size_of_Snd_driver_guess,before', '-z=1300,kosinski,Size_of_Snd_driver2_guess,before']),
    ('p_s3k', ['-p=FF', '-z=0,uncompressed,Size_of_Snd_driver_guess,before', '-z=1300,uncompressed,Size_of_Snd_driver2_guess,before']),
]
for probe, args in CASES:
    asl(D, probe + '.asm')
    shutil.copy(os.path.join(D, probe + '.p'), os.path.join(D, 'in.p'))
    rc, text, data = p2bin(D, args + ['in.p', 'o.bin'])
    print('==', probe, ' '.join(args))
    print('   rc=%d %s' % (rc, text))
    if data is not None:
        if len(data) <= 64:
            print('   hex  %s' % data.hex())
        print('   size %d crc32 %s' % (len(data), crc(data)))
        open(os.path.join(D, '%s.%s.ref' % (probe, abs(hash(' '.join(args))) % 10000)), 'wb').write(data)
