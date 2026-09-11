import sys
src = open('/home/volence/sonic_hacks/s2disasm/s2.asm', encoding='latin-1').read().split('\n')
for n, bad in ((62701, 'JmpTo23_Objec'), (84172, 'JmpTo2_Mar')):
    line = src[n - 1]
    code = line.split(';')[0].rstrip()
    head, args = code.strip().split(None, 1)
    i = args.find(bad)
    toks = [t.strip() for t in args.split(',')]
    full = [t for t in toks if t.startswith(bad)]
    print(n, 'macro', head, 'code length', len(code), 'args length', len(args),
          'n args', len(toks), 'truncated token at chars', i, '..', i + len(bad),
          'full token', full)
