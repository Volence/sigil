"""Red-first proof: each mutation applied on disk, the line quoted back, the
named tests run, then the file restored from the committed baseline."""
import os, subprocess, sys

W = os.environ.get('SIGIL_WORKTREE', os.getcwd())
CODEC = 'crates/sigil-cli/src/p2bin_codec.rs'
BLOB = 'crates/sigil-link/src/blob.rs'
LIB = 'crates/sigil-clownlzss-sys/src/lib.rs'
# CARGO_TARGET_DIR must be set by the caller: this repo's shared target/ is a
# frozen artifact other lanes depend on and no ad-hoc cargo run may relink it.
ENV = dict(os.environ)
assert ENV.get('CARGO_TARGET_DIR'), 'set CARGO_TARGET_DIR before running this'

MUTATIONS = [
    ('M1 kosinski-optimised stores the AUTHENTIC kosinski stream', [
        (CODEC, 'BlobFormat::KosinskiOptimised => expect_compressed(format, sigil_clownlzss_sys::compress_kosinski(data)),',
                'BlobFormat::KosinskiOptimised => sigil_clownlzss_sys::accurate::compress_kosinski_authentic(data), // MUTATION M1')],
     ['-p', 'sigil-cli', '--test', 'as_driver_placement']),
    ('M2 saxman-optimised stores the HEADER-FRAMED stream', [
        (CODEC, 'BlobFormat::SaxmanOptimised => expect_compressed(format, sigil_clownlzss_sys::compress_saxman(data, false)),',
                'BlobFormat::SaxmanOptimised => expect_compressed(format, sigil_clownlzss_sys::compress_saxman(data, true)), // MUTATION M2')],
     ['-p', 'sigil-cli', '--test', 'as_driver_placement']),
    ('M3 saxman-optimised stores the AUTHENTIC saxman stream', [
        (CODEC, 'BlobFormat::SaxmanOptimised => expect_compressed(format, sigil_clownlzss_sys::compress_saxman(data, false)),',
                'BlobFormat::SaxmanOptimised => sigil_clownlzss_sys::accurate::compress_saxman_authentic(data), // MUTATION M3')],
     ['-p', 'sigil-cli', '--test', 'as_driver_placement']),
    ('M4 the optimised pair is read back by the WRONG decompressor', [
        (CODEC, 'sigil_clownlzss_sys::decompress_kosinski(stored).map_err(|e| e.to_string())',
                'sigil_clownlzss_sys::decompress_kosplus(stored).map_err(|e| e.to_string()) // MUTATION M4')],
     ['-p', 'sigil-cli', '--test', 'as_driver_placement']),
    ('M5 saxman keeps saxman-bugged\'s junk-byte-trimming stored length', [
        (CODEC, 'sigil_clownlzss_sys::decompress_saxman_no_header(stored, stored.len()).map_err(|e| e.to_string())',
                'sigil_clownlzss_sys::decompress_saxman_no_header(stored, stored.len() - 1).map_err(|e| e.to_string()) // MUTATION M5')],
     ['-p', 'sigil-cli', '--test', 'as_driver_placement']),
    ('M6 compress_kosinski is wired to the kosinskiplus shim', [
        (LIB, 'run_compress(data, clownlzss_kosinski_compress)\n}',
              'run_compress(data, clownlzss_kosinskiplus_compress) // MUTATION M6\n}')],
     ['-p', 'sigil-clownlzss-sys', '--test', 'p2bin_optimised_vectors']),
    ('M7 two format names swapped in BlobFormat::name', [
        (BLOB, 'BlobFormat::KosinskiOptimised => "kosinski-optimised",\n            BlobFormat::Saxman => "saxman",',
               'BlobFormat::KosinskiOptimised => "saxman", // MUTATION M7\n            BlobFormat::Saxman => "kosinski-optimised",')],
     ['-p', 'sigil-link', '--lib']),
    ('M8 kosinski-optimised loses its own name (two formats share one)', [
        (BLOB, 'BlobFormat::KosinskiOptimised => "kosinski-optimised",',
               'BlobFormat::KosinskiOptimised => "kosinski", // MUTATION M8')],
     ['-p', 'sigil-link', '--lib']),
    ('M9 header-less compress_saxman is wired to the header-framed shim', [
        (LIB, 'run_compress(data, clownlzss_saxman_compress_without_header)?',
              'run_compress(data, clownlzss_saxman_compress_with_header)? // MUTATION M9')],
     ['-p', 'sigil-clownlzss-sys', '--test', 'p2bin_optimised_vectors']),
]

log = []
def say(s):
    print(s)
    log.append(s)

for title, edits, test_args in MUTATIONS:
    say('\n===== %s' % title)
    for path, old, new in edits:
        full = os.path.join(W, path)
        text = open(full, encoding='utf-8').read()
        assert old in text, ('anchor missing', path, old[:60])
        open(full, 'w', encoding='utf-8').write(text.replace(old, new, 1))
    stat = subprocess.run(['git', '-C', W, 'diff', '--stat'], capture_output=True, text=True).stdout
    say('  APPLIED, git diff --stat:\n%s' % '\n'.join('    ' + l for l in stat.strip().split('\n')))
    for path, _, new in edits:
        for i, line in enumerate(open(os.path.join(W, path), encoding='utf-8'), 1):
            if 'MUTATION' in line:
                say('  ON DISK %s:%d: %s' % (path, i, line.strip()))
    r = subprocess.run(['cargo', 'test', '--release'] + test_args, cwd=W, env=ENV,
                       capture_output=True, text=True)
    out = r.stdout + r.stderr
    verdict = [l for l in out.split('\n') if l.startswith('test result:') or l.startswith('    ') and l.strip().startswith(('the_', 'a_', 'clownlzss_', 'every_', 'no_', 'kosinski_', 'both_'))]
    fails = [l for l in out.split('\n') if ' ... FAILED' in l]
    say('  cargo exit %d' % r.returncode)
    for l in fails:
        say('    %s' % l.strip())
    for l in out.split('\n'):
        if l.startswith('test result:'):
            say('    %s' % l.strip())
    if r.returncode == 0:
        say('  *** APPLIED-AND-STILL-GREEN: the runner is not executing what was patched ***')
    for path, _, _ in edits:
        subprocess.run(['git', '-C', W, 'checkout', '--', path], check=True)
    assert not subprocess.run(['git', '-C', W, 'diff', '--quiet'], cwd=W).returncode, 'restore failed'

open(os.environ.get('MUTATION_LOG', 'mutations.log'), 'w').write('\n'.join(log))
