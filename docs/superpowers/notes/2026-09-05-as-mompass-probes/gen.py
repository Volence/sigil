import os

d = '/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f/.mompassprobe/'
head = '\tcpu 68000\n\tpadding off\n\torg 0\n'
# One forward reference, which is what makes asl run a second pass.
fwd = '\tdc.w Later-*\nLater:\n'

cases = {
    'm_eq1':   head + '\tif MOMPASS=1\n\tdc.b $AA\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_gt1':   head + '\tif MOMPASS>1\n\tdc.b $AA\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_eq2':   head + '\tif MOMPASS=2\n\tdc.b $AA\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_eq3':   head + '\tif MOMPASS=3\n\tdc.b $AA\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_val':   head + '\tdc.b MOMPASS\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_cmpd':  head + 'Z = 3\nN = 4\n\tif (Z<>N)&&(MOMPASS=1)\n\tdc.b $AA\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_fatal': head + '\tif MOMPASS=1\n\tfatal "first pass only"\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
    'm_msg':   head + '\tif MOMPASS=1\n\tmessage "first pass only"\n\tendif\n\tdc.b $11\n' + fwd + '\tend\n',
}

for k, v in cases.items():
    open(d + k + '.asm', 'w').write(v)
    print('=== ' + k + ' ===')
    print(v)
