"""Every probe shape through p2bin (on asl's object file) and through sigil, compared.

Outcomes per case: SAME (both wrote the same bytes), BOTH-REFUSE, or a named
divergence. A divergence where p2bin wrote an image and sigil refused is
listed as SIGIL-REFUSES with p2bin's own behaviour beside it, so each one can be
checked against the note's list of deliberate refusals.
"""
import os, shutil, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptools import asl, p2bin, check_tools, md5, crc

S = os.path.dirname(os.path.abspath(__file__))
D = os.path.join(S, 'probes')
SIGIL = sys.argv[1] if len(sys.argv) > 1 else os.path.join(S, 'target/release/sigil')
check_tools()
print('sigil', SIGIL, md5(SIGIL))

CASES = [
    ('p_after', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_after', ['-p=0', '-z=0,uncompressed,Guess,after']),
    ('p_after', ['-z=0,uncompressed,Guess,after']),
    ('p_after', ['-p=FF', '-z=0,kosinski,Guess,after']),
    ('p_after', ['-p=FF', '-z=0,saxman-bugged,Guess,after']),
    ('p_after', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_big', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_big', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_gap', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_gap', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_before', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_before', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_two', ['-p=FF', '-z=0,uncompressed,Guess,before', '-z=1300,uncompressed,Guess2,before']),
    ('p_two', ['-p=FF', '-z=1300,uncompressed,Guess2,before', '-z=0,uncompressed,Guess,before']),
    ('p_two', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_two', ['-p=FF', '-z=1300,uncompressed,Guess2,before']),
    ('p_seg', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_segback', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_phase_after', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_hex', ['-p=FF', '-z=10,uncompressed,Guess,after']),
    ('p_hex', ['-p=FF', '-z=16,uncompressed,Guess,after']),
    ('p_cont68k', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_last', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_last', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_first', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_first', ['-p=FF', '-z=0,uncompressed,Guess,before']),
    ('p_kos', ['-p=FF', '-z=0,uncompressed,Guess,after']),
    ('p_kos', ['-p=FF', '-z=0,kosinski,Guess,after']),
    ('p_kos', ['-p=FF', '-z=0,saxman-bugged,Guess,after']),
    ('p_kos', ['-p=0', '-z=0,kosinski,Guess,before']),
]
EXTRA = sys.argv[2:]  # additional probes named probe:arg,arg
for e in EXTRA:
    p, a = e.split(':', 1)
    CASES.append((p, a.split(' ')))

tally = {}
for probe, args in CASES:
    asl(D, probe + '.asm')
    shutil.copy(os.path.join(D, probe + '.p'), os.path.join(D, 'in.p'))
    rc, text, ref = p2bin(D, args + ['in.p', 'o.bin'])
    sp = os.path.join(D, 's.bin')
    if os.path.exists(sp):
        os.remove(sp)
    r = subprocess.run([SIGIL, probe + '.asm', '-o', 's.bin'] + args, cwd=D, capture_output=True)
    got = open(sp, 'rb').read() if os.path.exists(sp) and r.returncode == 0 else None
    err = [l for l in r.stderr.decode().splitlines() if 'error' in l]
    if ref is not None and got is not None:
        verdict = 'SAME' if ref == got else 'DIFFERENT'
    elif ref is None and got is None:
        verdict = 'BOTH-REFUSE'
    elif got is None:
        verdict = 'SIGIL-REFUSES'
    else:
        verdict = 'SIGIL-ACCEPTS-P2BIN-REFUSES'
    tally[verdict] = tally.get(verdict, 0) + 1
    print('%-28s %-14s %s' % (verdict, probe, ' '.join(args)))
    if verdict == 'DIFFERENT':
        print('     p2bin %s\n     sigil %s' % (ref.hex(' '), got.hex(' ')))
    if ref is None or verdict != 'SAME':
        print('     p2bin rc=%d %s' % (rc, text.splitlines()[0] if text else ''))
    if got is None:
        print('     sigil rc=%d %s' % (r.returncode, err[0] if err else r.stderr.decode()[:200]))
print('TALLY', tally)
