"""stub.py <src-corpus> <dst-corpus> <A|B>

MEASUREMENT SCAFFOLD, never for landing. Rewrites the constructs sigil refuses
into spellings asl assembles to the SAME bytes, so the run can reach the stage
behind them. Every replacement asserts the original text first and the count
of sites, and every stub keeps the file's line count, so a row or a byte can
be mapped back to the untouched source by line number.

A = every residual class except the driver placement. asl+p2bin on a stub-A
    tree must reproduce the reference ROM md5, which is the proof that stub A
    is byte-neutral.
B = A + the driver moved OUT of the 1 MiB image: sounddriver `!org 0` becomes
    `!org $300000` + `phase 0` (the blank line after it), and a `dephase` is
    appended at the end of the driver file. Z80 labels stay 0-based; the
    uncompressed driver lands at physical $300000, past the ROM, where it can
    be compared against asl's own Z80 record. asl cannot assemble stub B
    (its physical PC would leave the Z80 range), so B is sigil-only.
"""
import os, re, shutil, sys

src, dst, mode = sys.argv[1], sys.argv[2], sys.argv[3]
if os.path.exists(dst):
    shutil.rmtree(dst)
shutil.copytree(src, dst, symlinks=True)
log = []

def edit(path, fn):
    p = os.path.join(dst, path)
    lines = open(p, encoding='latin-1').read().split('\n')
    n0 = len(lines)
    fn(lines)
    assert len(lines) == n0 or path.endswith('sounddriver.asm'), (path, n0, len(lines))
    open(p, 'w', encoding='latin-1').write('\n'.join(lines))

def rep(lines, n, old, new, tag):
    assert lines[n - 1] == old, (tag, n, lines[n - 1])
    lines[n - 1] = new
    log.append('%s %d: %r -> %r' % (tag, n, old, new))

def s2(lines):
    # S1 palette: macro argument text with a digit-led word (`2p`). Same file
    # contents under a name with no digit-led word.
    for n, k in ((3935, '1'), (3936, '2'), (3937, '3')):
        old = 'Pal_SS%s_2p:palette Special Stage %s 2p.bin ; Special Stage %s 2p palette' % (k, k, k)
        new = 'Pal_SS%s_2p:palette Special Stage %s twop.bin ; Special Stage %s 2p palette' % (k, k, k)
        rep(lines, n, old, new, 'S1-palette')
    # S2 charset '\H' (AS escape for an apostrophe, $27): the numeric spelling.
    rep(lines, 14480, " charset '\\H',\"\\x39\\x37\\x38\"", ' charset $27,"\\x39\\x37\\x38"', 'S2-charset')
    rep(lines, 14606, " charset '\\H',\"\\x38\\x36\\x37\"", ' charset $27,"\\x38\\x36\\x37"', 'S2-charset')
    # S3 memory-form shifts with no size: asl defaults to .w (probe p_memshift).
    for n in (36098, 37592, 39179, 39180, 39181, 39200, 40447, 46924):
        old = lines[n - 1]
        m = re.match(r'^\t(asl|asr)\t(.*)$', old)
        assert m, (n, old)
        rep(lines, n, old, '\t%s.w\t%s' % (m.group(1), m.group(2)), 'S3-memshift')
    # S4 pushv/popv: an explicit save and restore through a set symbol.
    rep(lines, 69387, '\tpushv ,SonicDplcVer\t; Backup previous value of SonicDplcVer',
        'SonicDplcVer_stub := SonicDplcVer\t; Backup previous value of SonicDplcVer', 'S4-pushv')
    rep(lines, 69774, '\tpopv ,SonicDplcVer\t; Switch back to the previous DPLC format',
        'SonicDplcVer := SonicDplcVer_stub\t; Switch back to the previous DPLC format', 'S4-popv')
    # S8 lastbit: the value asl computed at this line on the clean run ($100000).
    rep(lines, 91263, '\t\tcnop\t-1,2<<lastbit(*-StartOfRom-1)', '\t\tcnop\t-1,$100000', 'S8-lastbit')
    # S9 shared: asl ignores it without -c; it only feeds the .h side file.
    rep(lines, 91275, '\tshared movewZ80CompSize', ';\tshared movewZ80CompSize', 'S9-shared')

def rename_1up(lines):
    c = 0
    for i, l in enumerate(lines):
        if '1upPlaying' in l:
            new = re.sub(r'(?<![A-Za-z0-9_])1upPlaying', 'OneUpPlaying', l)
            if new != l:
                c += 1
                log.append('S5-1up %d: %r -> %r' % (i + 1, l, new))
                lines[i] = new
    return c

HALF = {1863: ('\tld\ta,iyl', '0FDh,7Dh'), 1866: ('\tadc\ta,iyu', '0FDh,8Ch'),
        1976: ('\tld\ta,iyl', '0FDh,7Dh'), 1979: ('\tadc\ta,iyu', '0FDh,8Ch'),
        2258: ('\tld\te,ixl', '0DDh,5Dh'), 2259: ('\tld\td,ixu', '0DDh,54h'),
        3475: ('\tld\ta,ixl', '0DDh,7Dh'), 3478: ('\tadc\ta,ixu', '0DDh,8Ch'),
        3650: ('\tld\te,ixl', '0DDh,5Dh'), 3651: ('\tld\td,ixu', '0DDh,54h'),
        3687: ('\tadd\ta,ixl', '0DDh,85h'), 3689: ('\tadc\ta,ixu', '0DDh,8Ch')}

def drv(lines):
    # S5 `1upPlaying` (digit-led struct member): renamed at its definition and every use.
    c = rename_1up(lines)
    assert c == 11, ('1upPlaying sites', c)
    # S6 music_metadata with FLAGS omitted: `(FLAGS)` expands to `()`; pass 0.
    c = 0
    for n in range(3822, 3852):
        old = lines[n - 1]
        m = re.match(r'^(zMusIDPtr_\w+:\t+music_metadata Mus_\w+)$', old)
        assert m, (n, old)
        rep(lines, n, old, m.group(1) + ',0', 'S6-flags')
        c += 1
    assert c == 30
    assert lines[3852 - 1].endswith('music_metadata Mus_Drowning,MusFlag_SlowerOnPAL')
    # S7 IX/IY half registers: asl's own bytes from the clean run's listing.
    for n, (prefix, bytes_) in HALF.items():
        old = lines[n - 1]
        assert old.startswith(prefix), (n, old)
        rep(lines, n, old, '\tdb\t%s\t; stub for:%s' % (bytes_, prefix.replace('\t', ' ')), 'S7-ixiy')
    if mode in ('B', 'D'):
        rep(lines, 248, '    !org 0 ; Z80 code starting at address 0 has special meaning to s2p2bin.exe',
            '    !org $300000 ; STUB-B: driver outside the ROM image', 'S10-placement')
        rep(lines, 249, '', '    phase 0 ; STUB-B', 'S10-placement')
        lines.append('\tdephase ; STUB-B')
        log.append('S10-placement EOF: appended dephase')

def as_unescape(s):
    """AS string escapes as measured on the reference asl (probes p_esc*):
    \\xHH hex, \\NNN decimal, \\H or \\h apostrophe, \\A bell; anything else
    is refused here rather than guessed."""
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c != '\\':
            out.append(ord(c)); i += 1; continue
        i += 1
        c = s[i]
        if c in 'xX':
            j = i + 1
            while j < len(s) and j < i + 3 and s[j] in '0123456789abcdefABCDEF':
                j += 1
            out.append(int(s[i + 1:j], 16)); i = j
        elif c.isdigit():
            j = i
            while j < len(s) and j < i + 3 and s[j].isdigit():
                j += 1
            out.append(int(s[i:j])); i = j
        elif c in 'Hh':
            out.append(0x27); i += 1
        elif c == 'A':
            out.append(7); i += 1
        else:
            raise ValueError('unmodelled escape \\%s in %r' % (c, s))
    return out

CS = re.compile(r"^(\s*)charset\s+(?:'(\\H|.)'|\$([0-9A-Fa-f]+)),\"([^\"]*)\"\s*(;.*)?$")

def charset_expand(lines):
    # STUB C/D: every escape-bearing charset mapping string becomes one numeric
    # `charset $code,$value` per character, joined into the SAME list slot so
    # the list length (and every later list index) is unchanged; the written
    # file gains lines, so C/D rows no longer map by line number (bytes do).
    c = 0
    for i, l in enumerate(lines):
        m = CS.match(l)
        if not m or '\\' not in m.group(4):
            continue
        ind, ch, hexstart, body = m.group(1), m.group(2), m.group(3), m.group(4)
        start = 0x27 if ch == '\\H' else (ord(ch) if ch is not None else int(hexstart, 16))
        vals = as_unescape(body)
        new = '\n'.join('%scharset $%02X,$%02X' % (ind if ind else ' ', start + k, v) for k, v in enumerate(vals))
        log.append('S11-charset %d: %r -> %d single-character lines' % (i + 1, l, len(vals)))
        lines[i] = new
        c += 1
    assert c == 35, ('escape-bearing charset lines', c)

edit('s2.asm', s2)
if mode in ('C', 'D'):
    edit('s2.asm', charset_expand)
edit('s2.sounddriver.asm', drv)
for k in ('1', '2', '3'):
    a = os.path.join(dst, 'art/palettes/Special Stage %s 2p.bin' % k)
    b = os.path.join(dst, 'art/palettes/Special Stage %s twop.bin' % k)
    shutil.copyfile(a, b)
    log.append('S1-palette file copy: %s -> %s' % (os.path.basename(a), os.path.basename(b)))
# no other file may mention 1upPlaying
for root, _, files in os.walk(dst):
    if '/.git' in root:
        continue
    for f in files:
        if f.endswith('.asm'):
            t = open(os.path.join(root, f), encoding='latin-1').read()
            assert '1upPlaying' not in t, os.path.join(root, f)
open(os.path.join(os.path.dirname(dst), 'stub-%s.log' % mode), 'w').write('\n'.join(log) + '\n')
print('stub', mode, 'edits', len(log))
