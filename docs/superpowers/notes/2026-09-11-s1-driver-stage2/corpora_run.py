"""The three corpora, base sigil against this branch's sigil.

Sonic 2: a clean tree plus build.lua's generated inputs (from the luaref copy
where build.lua ran), rewritten by the census's stub C (every residual class
except the driver, byte-neutral under asl + p2bin), assembled by sigil with the
build script's own instruction and compared WHOLE against build.lua's ROM.

Diagnostic multisets: each run's stderr sorted into rows; the set difference
between base and new printed line for line.
"""
import hashlib, os, shutil, subprocess, sys, zlib

S = '/home/volence/sonic_hacks/.scratch/s1-driver-stage2'
W = '/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a4947ad0cc81e36dc'
BASE = S + '/sigil-base'
NEW = S + '/target/release/sigil'
D = S + '/diag'
os.makedirs(D, exist_ok=True)


def md5(p):
    return hashlib.md5(open(p, 'rb').read()).hexdigest()


def crc(b):
    return '%08x' % (zlib.crc32(b) & 0xFFFFFFFF)


print('base sigil', md5(BASE), 'new sigil', md5(NEW))


def sigil(binary, cwd, root, out, extra, tag):
    r = subprocess.run([binary, root, '-o', out] + extra, cwd=cwd, capture_output=True)
    rows = sorted(l for l in r.stderr.decode('utf-8', 'replace').splitlines() if l.strip())
    open(os.path.join(D, tag + '.rows'), 'w').write('\n'.join(rows) + ('\n' if rows else ''))
    return r.returncode, rows


def setdiff(tag, a, b):
    ra, rb = sorted(a), sorted(b)
    only_a = [x for x in ra if x not in rb]
    only_b = [x for x in rb if x not in ra]
    print('%s: base %d rows, new %d rows; only in base %d, only in new %d' % (tag, len(ra), len(rb), len(only_a), len(only_b)))
    for x in only_a:
        print('  < ' + x)
    for x in only_b:
        print('  > ' + x)


# ---- Sonic 1
s1 = S + '/ref/s1'
S1Z = ['-p=FF', '-z=0,kosinski,Size_of_DAC_driver_guess,after']
rc_b, b = sigil(BASE, s1, 'sonic.asm', D + '/s1-base.bin', [], 's1-plain-base')
rc_n, n = sigil(NEW, s1, 'sonic.asm', D + '/s1-new-plain.bin', [], 's1-plain-new')
rc_f, f = sigil(NEW, s1, 'sonic.asm', D + '/s1-new-flags.bin', S1Z, 's1-flags-new')
rc_t, t = sigil(NEW, s1, 'sonic.asm', D + '/s1-new-typo.bin', ['-p=FF', '-z=0,kosinski,NoSuchConstant,after'], 's1-typo-new')
print('\n== Sonic 1: base rc=%d, new without flags rc=%d, new with the instruction rc=%d, new with an undefined constant name rc=%d' % (rc_b, rc_n, rc_f, rc_t))
setdiff('s1 no flags', b, n)
setdiff('s1 base vs new with the instruction', b, f)
ref = open(s1 + '/out.bin', 'rb').read()
for p in ['s1-new-flags.bin', 's1-new-typo.bin']:
    img = open(D + '/' + p, 'rb').read()
    print('%s md5 %s crc32 %s size %d, equal to reference %s' % (p, md5(D + '/' + p), crc(img), len(img), img == ref))

# ---- Sonic 3 & Knuckles
sk = S + '/ref/sk'
open(sk + '/sk_wrapper.asm', 'w').write('Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n')
SKZ = ['-p=FF', '-z=0,kosinski,Size_of_Snd_driver_guess,before', '-z=1300,kosinski,Size_of_Snd_driver2_guess,before']
rc1, a1 = sigil(BASE, sk, 'sonic3k.asm', D + '/sk1.bin', [], 'sk-plain-base')
rc2, a2 = sigil(NEW, sk, 'sonic3k.asm', D + '/sk2.bin', [], 'sk-plain-new')
rc3, a3 = sigil(BASE, sk, 'sk_wrapper.asm', D + '/sk3.bin', [], 'sk-wrap-base')
rc4, a4 = sigil(NEW, sk, 'sk_wrapper.asm', D + '/sk4.bin', SKZ, 'sk-wrap-new')
print('\n== S3K: plain base rc=%d new rc=%d; wrapper base rc=%d new (with buildSK instruction) rc=%d' % (rc1, rc2, rc3, rc4))
setdiff('sk plain', a1, a2)
setdiff('sk wrapper', a3, a4)

# ---- Sonic 2
s2 = S + '/s2'
S2Z = ['-p=0', '-z=0,saxman-bugged,Size_of_Snd_driver_guess,after']
clean = s2 + '/clean'
rc1, c1 = sigil(BASE, clean, 's2.asm', D + '/s2c1.bin', [], 's2-raw-base')
rc2, c2 = sigil(NEW, clean, 's2.asm', D + '/s2c2.bin', S2Z, 's2-raw-new')
print('\n== S2 raw clean tree: base rc=%d, new (with the instruction) rc=%d' % (rc1, rc2))
setdiff('s2 raw', c1, c2)
gen = s2 + '/gen'
if os.path.exists(gen):
    shutil.rmtree(gen)
shutil.copytree(clean, gen, symlinks=True)
for d in ['sound/music/generated', 'sound/PCM/generated', 'sound/DAC/generated']:
    os.makedirs(os.path.join(gen, d), exist_ok=True)
    src = os.path.join(s2, 'luaref', d)
    for name in os.listdir(src):
        shutil.copy2(os.path.join(src, name), os.path.join(gen, d, name))
rc1, g1 = sigil(BASE, gen, 's2.asm', D + '/s2g1.bin', [], 's2-gen-base')
rc2, g2 = sigil(NEW, gen, 's2.asm', D + '/s2g2.bin', S2Z, 's2-gen-new')
print('\n== S2 with build.lua generated inputs: base rc=%d, new (with the instruction) rc=%d' % (rc1, rc2))
setdiff('s2 gen', g1, g2)
stub = s2 + '/stubC'
r = subprocess.run(['python3', W + '/docs/superpowers/notes/2026-09-11-s2-residual-census/scripts/stub.py', gen, stub, 'C'], capture_output=True)
print('\nstub C exit', r.returncode, r.stdout.decode()[-300:], r.stderr.decode()[-300:])
rc1, s1r = sigil(BASE, stub, 's2.asm', D + '/s2s1.bin', [], 's2-stubC-base')
rc2, s2r = sigil(NEW, stub, 's2.asm', D + '/s2-stubC-new.bin', S2Z, 's2-stubC-new')
print('== S2 stub C: base rc=%d, new (with the instruction) rc=%d' % (rc1, rc2))
setdiff('s2 stub C', s1r, s2r)
refp = s2 + '/luaref/s2built.bin'
ref2 = open(refp, 'rb').read()
print('reference', md5(refp), crc(ref2), len(ref2))
if rc2 == 0:
    img = open(D + '/s2-stubC-new.bin', 'rb').read()
    print('sigil stub C image md5 %s crc32 %s size %d' % (md5(D + '/s2-stubC-new.bin'), crc(img), len(img)))
    n = min(len(img), len(ref2))
    runs = []
    cur = None
    for x in range(n):
        if img[x] != ref2[x]:
            if cur and cur[1] == x:
                cur[1] = x + 1
            else:
                cur = [x, x + 1]
                runs.append(cur)
    print('WHOLE COMPARE, no window: %d bytes differ in %d runs; sizes %d vs %d' % (sum(b - a for a, b in runs), len(runs), len(img), len(ref2)))
    for a, b in runs[:40]:
        print('  [0x%06X, 0x%06X) sigil %s ref %s' % (a, b, img[a:min(b, a + 12)].hex(' '), ref2[a:min(b, a + 12)].hex(' ')))
print('CORPORA_END')
