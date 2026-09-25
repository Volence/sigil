lines = open('/home/volence/sonic_hacks/.scratch/as-squote/trees/sk-pristine/s3.asm', encoding='latin-1').read().split('\n')
blk = '\n'.join(lines[392:420])  # s3.asm lines 393..420
src = "\tcpu 68000\n\tpadding off\n\torg 0\nMessageData:\n" + blk + "\n\tend\n"
open('/home/volence/sonic_hacks/.scratch/as-squote/probes/s3blk.asm', 'w').write(src)
open('/home/volence/sonic_hacks/.scratch/as-squote/probes/list19.txt', 'w').write('s3blk')
print(repr(blk))
