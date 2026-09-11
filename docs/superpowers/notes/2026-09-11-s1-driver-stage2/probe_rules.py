"""Settle p2bin's -z / -p rules by running the pinned binary on small probes."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptools import asl, p2bin, records, check_tools, md5, P2BIN

D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'probes')
check_tools()
print('p2bin', P2BIN, md5(P2BIN))

PROBES = ['p_after', 'p_big', 'p_gap', 'p_before', 'p_two', 'p_seg', 'p_segback',
          'p_phase_after', 'p_hex']
for p in PROBES:
    asl(D, p + '.asm')
    print('==', p, 'records:', [(hex(c), hex(s), len(b)) for c, s, b in records(os.path.join(D, p + '.p'))])


def run(label, args, probe='p_after'):
    rc, text, data = p2bin(D, args + [probe + '.p', 'o.bin'])
    shown = data.hex(' ') if data is not None else None
    print('%-44s rc=%d out=%s' % (label, rc, shown))
    if text:
        for line in text.splitlines()[:4]:
            print('      | ' + line)
    return rc, data


print('\n# placement, uncompressed')
run('after -p=FF', ['-p=FF', '-z=0,uncompressed,Guess,after'])
run('after -p=0', ['-p=0', '-z=0,uncompressed,Guess,after'])
run('after, no -p', ['-z=0,uncompressed,Guess,after'])
run('no -z, -p=FF', ['-p=FF'])
run('constant Small (=4)', ['-p=FF', '-z=0,uncompressed,Small,after'])
run('constant Nope (undefined)', ['-p=FF', '-z=0,uncompressed,Nope,after'])
run('overflow 20 > 16, after', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_big')
run('overflow 20 > 16, before', ['-p=FF', '-z=0,uncompressed,Guess,before'], 'p_big')
run('gapped predecessor, after', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_gap')
run('gapped predecessor, before', ['-p=FF', '-z=0,uncompressed,Guess,before'], 'p_gap')
run('before', ['-p=FF', '-z=0,uncompressed,Guess,before'], 'p_before')
run('before shape, after', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_before')
run('after shape, before', ['-p=FF', '-z=0,uncompressed,Guess,before'])
run('two -z before', ['-p=FF', '-z=0,uncompressed,Guess,before', '-z=1300,uncompressed,Guess2,before'], 'p_two')
run('two -z reversed order', ['-p=FF', '-z=1300,uncompressed,Guess2,before', '-z=0,uncompressed,Guess,before'], 'p_two')
run('two blobs, only the first -z', ['-p=FF', '-z=0,uncompressed,Guess,before'], 'p_two')
run('two blobs, only the second -z', ['-p=FF', '-z=1300,uncompressed,Guess2,before'], 'p_two')
run('segmented driver (gap)', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_seg')
run('segmented driver (gap) -p=0', ['-p=0', '-z=0,uncompressed,Guess,after'], 'p_seg')
run('backward org inside driver', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_segback')
run('phased z80 block after driver', ['-p=FF', '-z=0,uncompressed,Guess,after'], 'p_phase_after')
run('driver at 10h, -z=10', ['-p=FF', '-z=10,uncompressed,Guess,after'], 'p_hex')
run('driver at 10h, -z=16', ['-p=FF', '-z=16,uncompressed,Guess,after'], 'p_hex')
run('driver at 10h, -z=0x10', ['-p=FF', '-z=0x10,uncompressed,Guess,after'], 'p_hex')
run('-z address matches nothing', ['-p=FF', '-z=40,uncompressed,Guess,after'])

print('\n# -p grammar')
for v in ['FF', 'ff', '0xFF', '0XFF', '$FF', '', '-1', '100', 'FFx', '1FF', '+7', '0x', 'g', '7F']:
    run('-p=' + v, ['-p=' + v, '-z=0,uncompressed,Guess,after'])
run('-pFF (no =)', ['-pFF', '-z=0,uncompressed,Guess,after'])
run('-p=FF -p=0 (last wins?)', ['-p=FF', '-p=0', '-z=0,uncompressed,Guess,after'])

print('\n# -z grammar')
for z in ['0,Uncompressed,Guess,after', '0,uncompressed,Guess,After', '0,uncompressed,Guess',
          '0,uncompressed,Guess,after,x', '0,uncompressed,,after', ',uncompressed,Guess,after',
          '0,lzma,Guess,after', '0,uncompressed,Guess,sideways', '0,,Guess,after',
          '0, uncompressed,Guess,after', 'x,uncompressed,Guess,after', '00,uncompressed,Guess,after',
          '0,uncompressed,Guess,afterx', '0,uncompressedx,Guess,after']:
    run('-z=' + z, ['-p=FF', '-z=' + z])
run('-z0,... (no =)', ['-p=FF', '-z0,uncompressed,Guess,after'])
run('options after the file names', ['p_after.p', 'o.bin', '-p=FF', '-z=0,uncompressed,Guess,after'], 'p_after')
run('unknown option -q', ['-q', '-p=FF', '-z=0,uncompressed,Guess,after'])

print('\n# compressors, after, p_after (10-byte driver, 16-byte gap)')
for c in ['kosinski', 'kosinski-optimised', 'saxman', 'saxman-bugged', 'saxman-optimised', 'kosinskiplus']:
    run(c, ['-p=FF', '-z=0,%s,Guess,after' % c])
