import subprocess, re, collections

roots = {
    's2disasm': '/home/volence/sonic_hacks/s2disasm-mompass-clean',
    's1disasm': '/home/volence/sonic_hacks/s1disasm',
    'skdisasm': '/home/volence/sonic_hacks/skdisasm',
}
tot = collections.Counter()
for name, r in roots.items():
    out = subprocess.run(
        ['grep', '-rhn', 'MOMPASS', r, '--include=*.asm', '--include=*.inc'],
        capture_output=True, text=True).stdout
    c = collections.Counter(
        m[0] + m[1] for m in re.findall(r'MOMPASS\s*(==|=|<>|>=|<=|>|<)\s*(\d+)', out))
    print(name, dict(c), 'total MOMPASS occurrences:', out.count('MOMPASS'))
    tot.update(c)
print('TOTAL', dict(tot))
