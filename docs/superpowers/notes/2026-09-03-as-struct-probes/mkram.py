import re, subprocess
C='/home/volence/sonic_hacks/.s1-struct-s1disasm/'
# 1) every RAM symbol declared in _Variables.asm, in source order
names=[]; seen=set()
def add(n):
    if n not in seen: seen.add(n); names.append(n)
for line in open(C+'_Variables.asm'):
    m=re.match(r'^([A-Za-z_][A-Za-z_0-9]*):', line) or re.match(r'^([A-Za-z_][A-Za-z_0-9]*)\s+equ\s', line)
    if m: add(m.group(1))
# 2) the struct layouts, read from the DECLARATION source (not from an asl dump)
ram=open(C+'s1.sounddriver.ram.asm').read().splitlines()
def members(struct):
    out=[]; on=False
    for l in ram:
        if re.match(r'^%s\s+struct\b'%re.escape(struct), l, re.I): on=True; continue
        if on and re.match(r'^\s*endstruct\b', l): break
        if on:
            m=re.match(r'^([A-Za-z_][A-Za-z_0-9]*):', l)
            if m: out.append(m.group(1))
    return out
track=members('SMPS_Track'); smpsram=members('SMPS_RAM')
# which SMPS_RAM members are embedded SMPS_Track instances
embeds=[m.group(1) for l in ram for m in [re.match(r'^([A-Za-z_][A-Za-z_0-9]*):\s+SMPS_Track\s*$', l.rstrip())] if m]
for t in track: add('SMPS_Track.'+t)
add('SMPS_Track.len')
for t in smpsram: add('SMPS_RAM.'+t)
add('SMPS_RAM.len')
for e in embeds:
    for t in track: add('SMPS_RAM.%s.%s'%(e,t))
# 3) the same layout seen through the real instantiation
for t in smpsram: add('v_snddriver_ram.'+t)
for e in embeds:
    for t in track: add('v_snddriver_ram.%s.%s'%(e,t))
with open('ramdump.inc','w') as f:
    for n in names: f.write('\tdc.l\t%s\n'%n)
print('symbols dumped:', len(names), '| track members', len(track), '| SMPS_RAM members', len(smpsram), '| embeds', len(embeds))
