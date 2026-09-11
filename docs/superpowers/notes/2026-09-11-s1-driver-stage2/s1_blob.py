"""Sonic 1: asl's record structure around the driver, the driver's bytes, and the
stored stream in the reference ROM against the accurate-kosinski compressor."""
import collections, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptools import records, md5, crc

S = os.path.dirname(os.path.abspath(__file__))
R = os.path.join(S, 'ref', 's1')
recs = records(os.path.join(R, 'sonic.p'))
print('records', len(recs))
lens = collections.Counter(len(b) for _, _, b in recs)
print('record length: max 0x%X, most common %s' % (max(lens), lens.most_common(5)))
# where do records split inside one contiguous run of one CPU?
splits = []
for k in range(1, len(recs)):
    c0, s0, b0 = recs[k - 1]
    c1, s1, b1 = recs[k]
    if c0 == c1 and s0 + len(b0) == s1:
        splits.append((s1, len(b0)))
print('splits inside a contiguous same-CPU run:', len(splits), 'first few', [(hex(a), hex(l)) for a, l in splits[:8]])
zk = [k for k, (c, s, b) in enumerate(recs) if c != 1 and s == 0]
print('Z80 records at address 0:', zk)
k = zk[0]
for j in range(k - 2, min(k + 4, len(recs))):
    c, s, b = recs[j]
    print('  rec %d cpu 0x%X [0x%X, 0x%X) len 0x%X' % (j, c, s, s + len(b), len(b)))
# the blob: consecutive, contiguous Z80 records from k
blob = bytearray()
j = k
while j < len(recs) and recs[j][0] == recs[k][0] and recs[j][1] == len(blob):
    blob += recs[j][2]
    j += 1
prev_end = recs[k - 1][1] + len(recs[k - 1][2])
nxt = recs[j][1]
print('blob records %d..%d, %d bytes (0x%X); prev end 0x%X; next record 0x%X; gap 0x%X' % (
    k, j - 1, len(blob), len(blob), prev_end, nxt, nxt - prev_end))
open(os.path.join(S, 's1_driver.z80'), 'wb').write(blob)
rom = open(os.path.join(R, 'out.bin'), 'rb').read()
kc = '/home/volence/sonic_hacks/programs/accurate-kosinski/build/kosinski-compress'
print('accurate-kosinski compressor', kc, md5(kc))
r = subprocess.run([kc, os.path.join(S, 's1_driver.z80'), os.path.join(S, 's1_driver.ak')], capture_output=True)
print('kosinski-compress rc', r.returncode, (r.stdout + r.stderr)[:200])
ak = open(os.path.join(S, 's1_driver.ak'), 'rb').read()
stored = rom[prev_end:nxt]
print('accurate-kosinski output %d bytes (0x%X), crc %s' % (len(ak), len(ak), crc(ak)))
print('ROM[prev_end : prev_end+len] equal:', stored[:len(ak)] == ak)
print('ROM tail after the stream: %d bytes, set %s' % (len(stored) - len(ak), sorted(set(stored[len(ak):]))))
