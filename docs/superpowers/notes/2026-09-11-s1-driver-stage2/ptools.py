"""Pinned asl / p2bin runners and an AS .p record reader, for the stage-2 probes.

asl_ref.sh cannot be sourced under this agent's isolation guard, so its three
checks are done here: the binary's md5 against the literal pin, the exit status,
and the listing's pass footer (present, and without the incomplete-pass line).
"""
import hashlib, os, re, struct, subprocess, zlib

ASL = '/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl'
ASL_MD5 = '61e672562465725a8c102288a7da9098'
P2BIN = '/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/p2bin'
P2BIN_MD5 = '4f2fff99c3347bafb93b12d5be1db754'


def md5(path):
    return hashlib.md5(open(path, 'rb').read()).hexdigest()


def crc(data):
    return '%08x' % (zlib.crc32(data) & 0xFFFFFFFF)


def check_tools():
    got = md5(ASL)
    assert got == ASL_MD5, ('asl is not the reference build', got)
    got = md5(P2BIN)
    assert got == P2BIN_MD5, ('p2bin is not the pinned build', got)


def asl(dirpath, name):
    """Assemble dirpath/name with the pinned asl. Refuses on any failure."""
    check_tools()
    env = dict(os.environ, AS_MSGPATH=os.path.dirname(ASL))
    r = subprocess.run([ASL, '-xx', '-n', '-q', '-A', '-L', '-U', '-E', '-i', '.', name],
                       cwd=dirpath, env=env, capture_output=True)
    lst_path = os.path.join(dirpath, name.split('.')[0] + '.lst')
    lst = open(lst_path, encoding='latin-1').read()
    incomplete = re.search(r'^[ \t]+Additional necessary passes not started', lst, re.M)
    footer = re.search(r'^ +[0-9]+ passe?s?$', lst, re.M)
    if r.returncode != 0 or not footer or incomplete:
        log = os.path.join(dirpath, name.split('.')[0] + '.log')
        extra = open(log).read() if os.path.exists(log) else ''
        raise SystemExit('ASL REFUSED %s: exit %d footer %s incomplete %s\n%s' % (
            name, r.returncode, bool(footer), bool(incomplete), extra))
    return lst


def p2bin(dirpath, args, out='o.bin'):
    """Run the pinned p2bin with args (a list). Returns (rc, text, bytes-or-None)."""
    check_tools()
    outp = os.path.join(dirpath, out)
    if os.path.exists(outp):
        os.remove(outp)
    r = subprocess.run([P2BIN] + args, cwd=dirpath, capture_output=True)
    text = (r.stdout + r.stderr).decode('latin-1').strip()
    data = open(outp, 'rb').read() if os.path.exists(outp) else None
    return r.returncode, text, data


def records(path):
    """(cpu, start, bytes) per data record of an AS .p file, in file order."""
    d = open(path, 'rb').read()
    assert d[:2] == b'\x89\x14', d[:2].hex()
    i = 2
    recs = []
    while i < len(d):
        h = d[i]
        i += 1
        if h == 0:
            break
        if h == 0x80:
            i += 4
            continue
        if h == 0x81:
            cpu = d[i]
            i += 3
        else:
            cpu = h
        start, length = struct.unpack_from('<IH', d, i)
        i += 6
        recs.append((cpu, start, d[i:i + length]))
        i += length
    return recs
