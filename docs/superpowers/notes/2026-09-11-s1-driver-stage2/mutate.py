"""The half-fix matrix: one modelled half-fix per row, each a set of literal
substitutions into committed files, each substitution required to match
exactly once and to carry a `MUTATION <id>` marker.

Per row: refuse on a dirty tree; apply; quote every marked line back from disk;
run the targets that pin this behaviour; run the rebuilt binary on a witness
probe; restore each file from HEAD with `git show HEAD:<path>`; require an
empty `git status --porcelain`. Usage: mutate.py <id>... (or `all`).
"""
import os, re, subprocess, sys

W = '/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a4947ad0cc81e36dc'
S = '/home/volence/sonic_hacks/.scratch/s1-driver-stage2'
ENV = dict(os.environ, SIGIL_ALLOW_PARTIAL='1', CARGO_TARGET_DIR=S + '/target')
for k in ('AEON_DIR', 'SIGIL_STRICT_GATE'):
    ENV.pop(k, None)
BLOB = 'crates/sigil-link/src/blob.rs'
RELAX = 'crates/sigil-link/src/relax.rs'
MAIN = 'crates/sigil-cli/src/main.rs'
ACC = 'crates/sigil-clownlzss-sys/src/accurate.rs'
EVAL = 'crates/sigil-frontend-as/src/eval.rs'

CLI = ['cargo', 'test', '--release', '-p', 'sigil-cli', '--test', 'as_driver_placement',
       '--test', 'as_driver_placement_corpus', '--test', 'as_second_address_space', '--no-fail-fast']
LINK = ['cargo', 'test', '--release', '-p', 'sigil-link', '--lib', '--no-fail-fast']
VEC = ['cargo', 'test', '--release', '-p', 'sigil-clownlzss-sys', '--test', 'accurate_vectors', '--no-fail-fast']

MUTATIONS = {
    'M1': ('-z parsed, the blob stored uncompressed', [
        (BLOB, 'BlobFormat::Kosinski => sigil_clownlzss_sys::accurate::compress_kosinski_authentic(data),',
         'BlobFormat::Kosinski => data.to_vec(), // MUTATION M1')], [CLI],
        ('probes/p_after.asm', ['-p=FF', '-z=0,kosinski,Guess,after'])),
    'M2': ('the wrong Kosinski variant: clownlzss optimal', [
        (BLOB, 'BlobFormat::Kosinski => sigil_clownlzss_sys::accurate::compress_kosinski_authentic(data),',
         'BlobFormat::Kosinski => sigil_clownlzss_sys::compress_kosinski(data).expect("kosinski"), // MUTATION M2')], [CLI],
        ('probes/p_after.asm', ['-p=FF', '-z=0,kosinski,Guess,after'])),
    'M2b': ('the wrong Kosinski variant: Sega mistake 1 corrected (match cap 0x100)', [
        (ACC, 'const KOS_MAX_MATCH: usize = 0xFD;', 'const KOS_MAX_MATCH: usize = 0x100; // MUTATION M2b')], [VEC, CLI],
        None),
    'M3': ('`after` read as `before`', [
        (BLOB, '"after" => BlobInsert::After,', '"after" => BlobInsert::Before, // MUTATION M3')], [CLI],
        ('probes/p_after.asm', ['-p=FF', '-z=0,uncompressed,Guess,after'])),
    'M4': ('the blob stored at its own Z80 address instead of the gap', [
        (BLOB, '(prev_end, reserved, where_)', '(z.address, reserved, where_) /* MUTATION M4 */')], [CLI],
        ('probes/p_after.asm', ['-p=FF', '-z=0,uncompressed,Guess,after'])),
    'M5': ('an overflow accepted', [
        (BLOB, 'reserved.filter(|&r| (stored.len() as i64) > r)', 'reserved.filter(|&r| false && (stored.len() as i64) > r) /* MUTATION M5 */')],
        [CLI], ('probes/p_cont68k.asm', ['-p=FF', '-z=0,uncompressed,Guess,after'])),
    'M6': ('-p ignored', [
        (MAIN, 'sigil_link::flatten_placing(&resolved, &linked, &blobs, pad.unwrap_or(0x00))',
         'sigil_link::flatten_placing(&resolved, &linked, &blobs, 0x00) /* MUTATION M6 */')], [CLI],
        ('probes/p_after.asm', ['-p=FF', '-z=0,uncompressed,Guess,after'])),
    'M7': ('only the first of two -z honoured', [
        (MAIN, 'Ok(blob) => blobs.push(blob),', 'Ok(blob) => if blobs.is_empty() { blobs.push(blob) }, // MUTATION M7')], [CLI],
        ('probes/p_two.asm', ['-p=FF', '-z=0,uncompressed,Guess,before', '-z=1300,uncompressed,Guess2,before'])),
    'M8': ('a second space no -z names placed anyway (both layers)', [
        (RELAX, '.filter(|space| !(space.cpu == sigil_ir::Cpu::Z80 && placed_origins.contains(&space.secs[0].1)))',
         '.filter(|space| !(space.cpu == sigil_ir::Cpu::Z80 && !placed_origins.is_empty())) /* MUTATION M8 */'),
        (BLOB, 'if matches!(r.space, AddressSpace::Foreign { .. }) && !claimed[idx] {',
         'if false && matches!(r.space, AddressSpace::Foreign { .. }) && !claimed[idx] { // MUTATION M8')], [CLI],
        ('probes/p_two.asm', ['-p=FF', '-z=0,uncompressed,Guess,before'])),
    'M8a': ('the layout layer alone lets the uncovered space through', [
        (RELAX, '.filter(|space| !(space.cpu == sigil_ir::Cpu::Z80 && placed_origins.contains(&space.secs[0].1)))',
         '.filter(|space| !(space.cpu == sigil_ir::Cpu::Z80 && !placed_origins.is_empty())) /* MUTATION M8a */')], [CLI],
        ('probes/p_two.asm', ['-p=FF', '-z=0,uncompressed,Guess,before'])),
    'M8b': ('the placement layer alone lets the uncovered space through', [
        (BLOB, 'if matches!(r.space, AddressSpace::Foreign { .. }) && !claimed[idx] {',
         'if false && matches!(r.space, AddressSpace::Foreign { .. }) && !claimed[idx] { // MUTATION M8b')], [CLI],
        ('probes/p_two.asm', ['-p=FF', '-z=0,uncompressed,Guess,before'])),
    'M9': ('the round-trip check removed', [
        (BLOB, 'Ok(back) if back == blob => {}', 'Ok(back) if back == blob || true => {} // MUTATION M9')], [CLI],
        ('probes/p_straddle.asm', ['-p=FF', '-z=0,saxman-bugged,Guess,after'])),
    'M10': ('a restore that changes the processor does not end the section', [
        (EVAL, 'if self.state.cpu != before {', 'if false && self.state.cpu != before { // MUTATION M10')], [CLI],
        ('probes/p_cont68k.asm', ['-p=FF', '-z=0,uncompressed,Guess,after'])),
    'M11': ('the saxman-bugged junk byte always 00', [
        (ACC, 'let junk = if out.len() % 2 == 1 { 0x4E } else { 0x00 };', 'let junk = 0x00u8; // MUTATION M11')], [VEC, CLI],
        None),
    'M12': ('`-p=..` and `-z=..` not matched by the option scan', [
        (MAIN, "o.name == arg || (o.attached && arg.strip_prefix(o.name).is_some_and(|v| v.starts_with('=')))",
         'o.name == arg /* MUTATION M12 */')], [CLI],
        ('probes/p_after.asm', ['-p=FF', '-z=0,uncompressed,Guess,after'])),
}


def sh(cmd, **kw):
    return subprocess.run(cmd, cwd=W, capture_output=True, text=True, **kw)


def status():
    return sh(['git', 'status', '--porcelain']).stdout.strip()


def run_row(mid):
    what, subs, targets, witness = MUTATIONS[mid]
    print('\n######## %s: %s' % (mid, what), flush=True)
    if status():
        print('REFUSED: dirty tree before the row:\n' + status())
        sys.exit(3)
    files = []
    for path, old, new in subs:
        p = os.path.join(W, path)
        text = open(p).read()
        n = text.count(old)
        if n != 1:
            print('APPLY FAILED: %s matches %d sites in %s' % (mid, n, path))
            for f in files:
                open(os.path.join(W, f), 'w').write(sh(['git', 'show', 'HEAD:' + f]).stdout)
            sys.exit(4)
        open(p, 'w').write(text.replace(old, new))
        files.append(path)
    print('APPLY rc=0, %d substitution(s)' % len(subs))
    for path in sorted(set(files)):
        for i, line in enumerate(open(os.path.join(W, path)).read().split('\n'), 1):
            if 'MUTATION ' + mid + ' ' in line + ' ' or ('MUTATION ' + mid + ' */') in line:
                print('  ON DISK %s:%d: %s' % (path, i, line.strip()))
    for t in targets:
        r = sh(t, env=ENV, timeout=1800)
        out = r.stdout + r.stderr
        failed = sorted(set(re.findall(r'^test (\S+) \.\.\. FAILED', out, re.M)))
        results = re.findall(r'^test result: (\S+)\. (\d+) passed; (\d+) failed', out, re.M)
        compile_err = 'error[E' in out or 'could not compile' in out
        at = re.findall(r"panicked at (\S+):", out)
        print('  TARGET %s: results %s, compile error %s' % (' '.join(t[4:6] + t[6:8]), results, compile_err))
        for f in failed:
            print('    RED %s' % f)
        for a in sorted(set(at))[:8]:
            print('    panicked at %s' % a)
    if witness:
        probe, args = witness
        d, name = os.path.split(os.path.join(S, probe))
        r = subprocess.run([S + '/target/release/sigil', name, '--hex'] + args, cwd=d, capture_output=True, text=True)
        err = [l for l in r.stderr.splitlines() if 'error' in l]
        print('  WITNESS sigil %s %s: exit %d; %s' % (name, ' '.join(args), r.returncode,
              (err[0] if err else r.stdout.strip().splitlines()[0] if r.stdout.strip() else '')[:220]))
    for path in sorted(set(files)):
        open(os.path.join(W, path), 'w').write(sh(['git', 'show', 'HEAD:' + path]).stdout)
    st = status()
    print('  RESTORED from HEAD; git status --porcelain: %s' % (repr(st) if st else 'empty'))
    if st:
        sys.exit(5)


ids = list(MUTATIONS) if sys.argv[1:] == ['all'] else sys.argv[1:]
print('HEAD', sh(['git', 'rev-parse', 'HEAD']).stdout.strip(), 'branch', sh(['git', 'branch', '--show-current']).stdout.strip())
for mid in ids:
    run_row(mid)
print('MATRIX_END')
