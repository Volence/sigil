"""mutate.py: red-first mutations of the IMPLEMENTATION. For each: apply one exact
replacement on disk, show the on-disk diff, run the parcel's test binaries, record
which tests fail and the first failure lines, restore the file from the committed
HEAD (git show HEAD:path), and prove the tree is clean again."""
import subprocess, re, sys
W = '/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a216a42e1f5962449'
S = '/home/volence/sonic_hacks/.scratch/as-squote'
ENV = {'AEON_DIR': '/home/volence/sonic_hacks/.aeon-squote', 'CARGO_TARGET_DIR': S + '/target'}
EVAL = 'crates/sigil-frontend-as/src/eval.rs'
CHARSET = 'crates/sigil-frontend-as/src/charset.rs'
MUTS = [
    ('M1 no operand is single-quoted', EVAL,
     "        single(toks.first()) && single(toks.last())\n",
     "        false && single(toks.first()) && single(toks.last())\n"),
    ('M2 the rule reads only the first token', EVAL,
     "        single(toks.first()) && single(toks.last())\n",
     "        single(toks.first())\n"),
    ('M3 Z80 dw zero-extends a character', EVAL,
     "            DataUnit::WordLe => i16::from(b as i8).to_le_bytes().to_vec(),\n",
     "            DataUnit::WordLe => u16::from(b).to_le_bytes().to_vec(),\n"),
    ('M4 string+integer keeps the raw result bytes', EVAL,
     "        StrTyped::Str(out.into_iter().filter_map(|b| cs.lowest_preimage(b)).collect())\n",
     "        StrTyped::Str(out.into_iter().map(char::from).collect())\n"),
    ('M5 a string symbol forgets its quoting (equ and set both)', EVAL,
     "            let single_quoted = self.single_quoted_operand(rest);\n",
     "            let single_quoted = false && self.single_quoted_operand(rest);\n", 2),
    ('M6 a single-quoted charset target is a table', EVAL,
     "                if self.single_quoted_operand(groups[1]) {\n",
     "                if false && self.single_quoted_operand(groups[1]) {\n"),
    ('M7 include opens a single-quoted path', EVAL,
     "            Tok::Str(s, Quote::Double) => Some(s.clone()),\n",
     "            Tok::Str(s, _) => Some(s.clone()),\n"),
    ('M8 the fit is strictly shorter than the element', EVAL,
     "        if single_quoted && bytes.len() <= width {\n",
     "        if single_quoted && bytes.len() < width {\n"),
    ('M9 the highest preimage instead of the lowest', CHARSET,
     "        self.map.iter().position(|&t| t == b).map(|i| char::from(i as u8))\n",
     "        self.map.iter().rposition(|&t| t == b).map(|i| char::from(i as u8))\n"),
]
sel = sys.argv[1:] or [m[0].split()[0] for m in MUTS]
log = open(S + '/logs/mutations.log', 'a')
def sh(cmd, **kw):
    return subprocess.run(cmd, cwd=W, capture_output=True, text=True, **kw)
def out(s):
    print(s); log.write(s + '\n'); log.flush()
head = sh(['git', 'rev-parse', 'HEAD']).stdout.strip()
out(f"=== mutation run: pwd={W} HEAD={head} branch={sh(['git','branch','--show-current']).stdout.strip()}")
for m in MUTS:
    name, path, old, new = m[:4]
    want = m[4] if len(m) > 4 else 1
    if name.split()[0] not in sel:
        continue
    assert sh(['git', 'status', '--porcelain', '--', path]).stdout == '', 'dirty before mutation'
    src = open(f"{W}/{path}").read()
    n = src.count(old)
    assert n == want, f"{name}: target occurs {n} times, want {want}"
    open(f"{W}/{path}", 'w').write(src.replace(old, new))
    diff = sh(['git', 'diff', '--', path]).stdout
    out(f"--- {name}: on-disk diff\n{diff}")
    import os
    env = dict(os.environ, **ENV)
    r = subprocess.run(['cargo', 'test', '--release', '-p', 'sigil-frontend-as', '--test', 'as_single_quoted_string',
                        '--test', 'as_string_numeric_typing', '--no-fail-fast'], cwd=W, capture_output=True, text=True, env=env)
    t = r.stdout + r.stderr
    fails = sorted(set(re.findall(r'^test (\S+) \.\.\. FAILED', t, re.M)))
    counts = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed', t)
    comp = 'error[' in t or 'error:' in t and 'could not compile' in t
    out(f"{name}: cargo rc={r.returncode} results={counts} compile_error={comp} FAILED={fails}")
    for line in re.findall(r'^(?:\S+: asl .*|.*cases differ from asl.*|.*panicked at.*|  left: .*|.*refusal does not.*|.*: \w+ refused.*)$', t, re.M)[:8]:
        out("    " + line[:300])
    # restore from the committed baseline
    blob = sh(['git', 'show', f'HEAD:{path}']).stdout
    open(f"{W}/{path}", 'w').write(blob)
    clean = sh(['git', 'status', '--porcelain', '--', path]).stdout
    out(f"{name}: restored from HEAD:{path}, clean={clean == ''}")
out("MUTATIONS_END")
