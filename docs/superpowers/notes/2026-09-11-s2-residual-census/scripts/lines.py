import sys
D = '/home/volence/sonic_hacks/s2disasm/'
spec = sys.argv[1:]
# args: file:start-end ...
for s in spec:
    f, r = s.rsplit(':', 1)
    a, b = (int(x) for x in r.split('-'))
    src = open(D + f, encoding='latin-1').read().split('\n')
    print('==', f, a, b)
    for n in range(a, b + 1):
        print('%6d| %s' % (n, src[n - 1]))
