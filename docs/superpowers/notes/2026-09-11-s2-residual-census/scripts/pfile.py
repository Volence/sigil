"""pfile.py <file.p> : decompose an AS P-file into its records and contiguous runs.
Record header byte: 0x00 end, 0x80 entry point (4 bytes), 0x81 extended
(cpu, segment, granularity, start LE32, length LE16, data), 0x01..0x7F old
style (header = cpu, start LE32, length LE16, data)."""
import struct, sys
d = open(sys.argv[1], 'rb').read()
assert d[:2] == b'\x89\x14', d[:2].hex()
i = 2
recs = []
while i < len(d):
    h = d[i]; i += 1
    if h == 0:
        break
    if h == 0x80:
        i += 4; continue
    if h == 0x81:
        cpu, seg, gran = d[i], d[i+1], d[i+2]; i += 3
    else:
        cpu, seg, gran = h, 1, 1
    start, length = struct.unpack_from('<IH', d, i); i += 6
    recs.append((cpu, seg, start, length, d[i:i+length])); i += length
print('records', len(recs))
runs = []
for cpu, seg, start, length, data in recs:
    if runs and runs[-1][0] == cpu and runs[-1][2] == start:
        runs[-1][2] += length; runs[-1][3] += 1
    else:
        runs.append([cpu, start, start + length, 1])
for cpu, a, b, n in runs:
    print('cpu %3d [0x%06X, 0x%06X) len 0x%X  records %d' % (cpu, a, b, b - a, n))
if len(sys.argv) > 2:
    # dump the Z80 blob (cpu != 1) that starts at address 0 and is not the first run
    z = bytearray()
    started = False
    for cpu, seg, start, length, data in recs:
        if cpu != 1 and (start == 0 or started) and not (start == 0 and started):
            if start == 0:
                started = True
            if started and start == len(z):
                z += data
        elif started:
            break
    open(sys.argv[2], 'wb').write(z)
    print('z80 blob written', len(z), 'bytes ->', sys.argv[2])
