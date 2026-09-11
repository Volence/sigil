"""The scoped suites, one crate at a time, each log stamped with pwd/HEAD/branch
and ending on an exit line; then a failures-first summary per crate."""
import os, re, subprocess

W = '/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a4947ad0cc81e36dc'
S = '/home/volence/sonic_hacks/.scratch/s1-driver-stage2'
ENV = dict(os.environ, SIGIL_ALLOW_PARTIAL='1', CARGO_TARGET_DIR=S + '/target')
for k in ('AEON_DIR', 'SIGIL_STRICT_GATE', 'EMPYREAN_SUITE_ROOT'):
    ENV.pop(k, None)


def git(*a):
    return subprocess.run(['git'] + list(a), cwd=W, capture_output=True, text=True).stdout.strip()


summary = []
import sys
CRATES = sys.argv[1:] or ['sigil-clownlzss-sys', 'sigil-link', 'sigil-frontend-as', 'sigil-cli']
for crate in CRATES:
    log = '%s/suite-%s-%s.log' % (S, crate, git('rev-parse', '--short=8', 'HEAD'))
    stamp = 'STAMP pwd=%s head=%s branch=%s crate=%s' % (W, git('rev-parse', 'HEAD'), git('branch', '--show-current'), crate)
    r = subprocess.run(['cargo', 'test', '--release', '-p', crate, '--no-fail-fast', '--', '--nocapture'],
                       cwd=W, env=ENV, capture_output=True, text=True)
    out = r.stdout + r.stderr
    open(log, 'w').write(stamp + '\n' + out + '\nSUITE_EXIT=%d\n' % r.returncode)
    res = re.findall(r'^test result: \S+\. (\d+) passed; (\d+) failed; (\d+) ignored', out, re.M)
    p = sum(int(a) for a, _, _ in res)
    f = sum(int(b) for _, b, _ in res)
    i = sum(int(c) for _, _, c in res)
    failed = sorted(set(re.findall(r'^test (\S+) \.\.\. FAILED', out, re.M)))
    ignored = sorted(set(re.findall(r'^test (\S+) \.\.\. ignored', out, re.M)))
    skips = len(re.findall(r'^skip:', out, re.M))
    own = [t for t in ['as_driver_placement', 'accurate_vectors', 'the_blob_grammar_is_p2bins',
                       'sonic_1_builds_to_the_reference_rom_byte_for_byte'] if t in out]
    summary.append('%s: exit %d, %d passed, %d failed, %d ignored, %d skip lines, binaries %d; FAILED %s; ignored %s; own tests seen %s' % (
        crate, r.returncode, p, f, i, skips, len(res), failed, ignored, own))
    print(summary[-1], flush=True)
open(S + '/suites-%s.summary' % git('rev-parse', '--short=8', 'HEAD'), 'w').write('\n'.join(summary) + '\nSUITES_DONE\n')
