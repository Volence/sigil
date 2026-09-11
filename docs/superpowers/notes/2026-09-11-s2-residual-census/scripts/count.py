import re
D = '/home/volence/sonic_hacks/s2disasm/'
for f in ('s2.sounddriver.asm', 's2.asm'):
    src = open(D + f, encoding='latin-1').read().split('\n')
    for name in ('music_metadata', 'dac_sample_metadata', 'music_ptr', 'dac_sample_pointer'):
        hits = [i + 1 for i, l in enumerate(src)
                if re.match(r'^\s*(\S+:?\s+)?' + name + r'\b', l.split(';')[0]) and 'macro' not in l]
        if hits:
            print(f, name, len(hits), hits[:3], '...', hits[-3:])
for f in ('s2.asm', 's2.constants.asm'):
    for i, l in enumerate(open(D + f, encoding='latin-1').read().split('\n')):
        if re.search(r'padToPowerOfTwo\s*=|zeroOffsetOptimization\s*=|removeJmpTos\s*=|gameRevision\s*=|fixBugs\s*=', l):
            print(f, i + 1, l.strip())
