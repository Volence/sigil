#!/usr/bin/env python3
"""Static masking scan over sigil test sources.

For every #[test] fn, flatten its execution into an ordered event stream:
  P  a call that lowers, links, assembles or builds a unit (a non-local call whose
     name matches PRODUCER_RE; local fns are inlined instead)
  C  a non-diagnostic assertion (assert!/assert_eq!/assert_ne!/panic!/unreachable!
     whose condition or message is not about diagnostics), i.e. a comparison
     that can stop the test
  D  a diagnostic assertion (checks lower/link diags, unwrap_or_else/expect)
Loop bodies (for/while/loop) and closure bodies are unrolled twice.
A test is FLAGGED when some C lies strictly between two P events: a comparison
can stop the test after one unit was produced and before another was.

Each test is also tagged AEON when its flattened calls reach the aeon tree (AEON_RE, or
an AEON_DIR / s4.bin / demo.bin mention in its body), else inline.

Usage: mask_scan.py [--rev REV] [--verbose] [--trace] [--byte-filter] [--all-fns] files...
  --rev REV      read each file from `git show REV:path` (paths relative to the repo root)
  --verbose      also print the tests that are not flagged
  --trace        print the P/C event sequence under each flag
  --byte-filter  count a C only when its text names compared bytes (narrower; it misses
                 placement and pin comparisons, so the broad default is the census)
  --all-fns      treat every fn as a root, not only #[test] fns
Run from the repo root over every `crates/*/tests/**/*.rs` outside `vectors/`.
"""
import re, sys, subprocess, os

PRODUCER_RE = re.compile(
    r'^(lower\w*|link\w*|resolve_layout\w*|place_sections|assemble\w*|compile\w*|'
    r'build_\w*|emit_\w*|native_section\w*|resolve_canonical_sections|resolve_frozen_layout)$')
AEON_RE = re.compile(
    r'^(aeon_root|aeon_dir|aeon_checkout|reference_tree\w*|listing_\w+|sonic4_shape_defines|'
    r'shipped_shapes|sonic4_profile|demo_profile|config_[ab]_profile|lean_profile|ensure_generated|'
    r'game_contract_env_from_aeon|zero_byte_module|section_const_modules|mddbg_entry_labels|'
    r'extend_from_listing\w*|emp_const_\w+|bg_layout_size_const_src|engine_const_src|'
    r'scene_dsl_\w+|shadow_aeon_tree|read_dac_declarations|sound_layout|aeon_\w+)$')
ASSERT_MACROS = {'assert', 'assert_eq', 'assert_ne', 'panic', 'unreachable', 'debug_assert',
                 'debug_assert_eq', 'todo'}
DIAG_COND_RE = re.compile(r'diag|Level::Error|\berrs?\b|errors|warnings|is_err\(|is_ok\(')
DIAG_MSG_RE = re.compile(
    r'(?i)diag|\berror|failed|unknown name|unresolved|cannot read|STRICT_GATE|must load|'
    r'parse|not found|missing|refus')
BYTE_RE = re.compile(
    r'(?i)refrom|expected|golden|\brom\b|rom\[|reference|bytes|candidate|diff|image|\bwant|\bgot\b|'
    r'\bslice|window|identical|differ|match|crc')
KEYWORDS = {'if', 'while', 'for', 'match', 'loop', 'return', 'fn', 'let', 'in', 'else',
            'Some', 'Ok', 'Err', 'None', 'move', 'as', 'ref', 'mut', 'impl', 'where',
            'unsafe', 'async', 'await', 'dyn', 'struct', 'enum', 'type', 'use', 'mod',
            'pub', 'crate', 'self', 'Self', 'super', 'const', 'static', 'true', 'false',
            'break', 'continue', 'box'}


def mask(src):
    """Return a copy with comments blanked and string/char literal CONTENTS blanked,
    same length, so byte offsets align with the original."""
    out = list(src)
    i, n = 0, len(src)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != '\n':
                out[k] = ' '
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            blank(i, j); i = j; continue
        if src.startswith('/*', i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith('/*', j): depth += 1; j += 2
                elif src.startswith('*/', j): depth -= 1; j += 2
                else: j += 1
            blank(i, j); i = j; continue
        m = re.match(r'b?r(#*)"', src[i:i + 20])
        if m and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == '_')):
            hashes = m.group(1)
            start = i + m.end()
            end = src.find('"' + hashes, start)
            end = n if end < 0 else end
            blank(start, end); i = end + 1 + len(hashes); continue
        if c == '"' or (c == 'b' and src.startswith('b"', i) and
                        (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == '_'))):
            j = i + (2 if c == 'b' else 1)
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            blank(i + (2 if c == 'b' else 1), j); i = j + 1; continue
        if c == "'":
            m = re.match(r"'(\\.[^']*|[^\\'])'", src[i:i + 12])
            if m:
                blank(i + 1, i + m.end() - 1); i += m.end(); continue
        i += 1
    return ''.join(out)


def match_close(m, i, open_c, close_c):
    depth = 0
    for j in range(i, len(m)):
        if m[j] == open_c: depth += 1
        elif m[j] == close_c:
            depth -= 1
            if depth == 0: return j
    return len(m) - 1


def find_fns(m):
    """[(name, fn_kw_pos, body_open, body_close)]"""
    fns = []
    for mo in re.finditer(r'\bfn\s+([A-Za-z_]\w*)', m):
        j, paren = mo.end(), 0
        while j < len(m):
            ch = m[j]
            if ch in '([': paren += 1
            elif ch in ')]': paren -= 1
            elif ch == ';' and paren == 0: j = -1; break
            elif ch == '{' and paren == 0: break
            j += 1
        if j < 0 or j >= len(m): continue
        fns.append((mo.group(1), mo.start(), j, match_close(m, j, '{', '}')))
    return fns


def is_test(m, fn_pos):
    back = m[max(0, fn_pos - 400):fn_pos]
    # attributes directly above the fn (allow other attrs / pub / async)
    tail = re.search(r'((?:#\[[^\]]*\]\s*)+)(?:pub\s+)?$', back)
    return bool(tail and re.search(r'#\[\s*test\s*\]', tail.group(1)))


class FileScan:
    def __init__(self, src, extra_defs):
        self.src = src
        self.m = mask(src)
        self.fns = find_fns(self.m)
        self.defs = {}
        for f in self.fns:
            self.defs.setdefault(f[0], f)
        self.extra = extra_defs  # name -> (FileScan, fn tuple) from shared modules
        # nested fn item spans, skipped when scanning their parent
        self.nested = [(f[1], f[3]) for f in self.fns]

    def lookup(self, name):
        if name in self.defs: return self, self.defs[name]
        if name in self.extra: return self.extra[name]
        return None

    def events(self, a, b, stack):
        """Events for masked range [a, b)."""
        m, ev, i = self.m, [], a
        while i < b:
            # skip nested fn items
            skip = next((e for (s, e) in self.nested if s == i and s > a), None)
            if skip is not None:
                i = skip + 1; continue
            mo = re.compile(r'\b(for|while|loop)\b').match(m, i)
            if mo and (i == 0 or not (m[i - 1].isalnum() or m[i - 1] == '_')):
                # header then body
                j, paren = mo.end(), 0
                while j < b and not (m[j] == '{' and paren == 0):
                    if m[j] in '([': paren += 1
                    elif m[j] in ')]': paren -= 1
                    j += 1
                if j < b:
                    hdr = self.events(mo.end(), j, stack)
                    close = match_close(m, j, '{', '}')
                    body = self.events(j + 1, close, stack)
                    ev += hdr + [('LOOP', body)]
                    i = close + 1; continue
            if m[i] == '|' and self._closure_start(i):
                j = m.find('|', i + 1)
                k = j + 1
                while k < b and m[k] in ' \t\n': k += 1
                if m.startswith('move', k): k += 4
                while k < b and m[k] in ' \t\n': k += 1
                if k < b and m[k] == '{':
                    close = match_close(m, k, '{', '}')
                    body = self.events(k + 1, close, stack)
                    end = close + 1
                else:
                    end = self._expr_end(k, b)
                    body = self.events(k, end, stack)
                ev.append(('LOOP', body))
                i = end; continue
            mo = re.compile(r'([A-Za-z_]\w*)\s*(!?)\s*\(').match(m, i)
            if mo and (i == 0 or not (m[i - 1].isalnum() or m[i - 1] == '_')):
                name, bang = mo.group(1), mo.group(2)
                popen = mo.end() - 1
                pclose = match_close(m, popen, '(', ')')
                prev = m[:i].rstrip()
                is_method = prev.endswith('.')
                if bang and name in ASSERT_MACROS:
                    inner = self.events(popen + 1, pclose, stack)
                    ev += inner + [(self._classify(name, popen, pclose, i), name,
                                    self.src[i:pclose + 1][:160].replace('\n', ' '))]
                    i = pclose + 1; continue
                if bang:
                    ev += self.events(popen + 1, pclose, stack)
                    i = pclose + 1; continue
                if name in ('unwrap_or_else', 'expect', 'unwrap_or', 'map_err', 'ok_or_else') and is_method:
                    inner = self.events(popen + 1, pclose, stack)
                    # a panic in these closures reports the producer's own failure
                    ev += [('D', name, x[2] if len(x) > 2 else '') if x[0] == 'C' else x
                           for x in self._flat(inner)]
                    i = pclose + 1; continue
                if name in KEYWORDS or prev.endswith('fn'):
                    i = mo.end(); continue
                args = self.events(popen + 1, pclose, stack)
                hit = self.lookup(name)
                if hit and name not in stack and len(stack) < 12:
                    fs, f = hit
                    ev += args + fs.events(f[2] + 1, f[3], stack + [name])
                elif AEON_RE.match(name):
                    ev += args + [('R', name, '')]
                elif PRODUCER_RE.match(name):
                    ev += args + [('P', name, self.src[i:min(pclose + 1, i + 100)].replace('\n', ' '))]
                else:
                    ev += args
                i = pclose + 1; continue
            i += 1
        return ev

    def _flat(self, ev):
        out = []
        for e in ev:
            if e[0] == 'LOOP': out += self._flat(e[1])
            else: out.append(e)
        return out

    def _closure_start(self, i):
        prev = self.m[:i].rstrip()
        if prev.endswith('|'): return False
        if not prev or prev[-1] in '(,=:{;' or prev.endswith('move') or prev.endswith('return'):
            j = self.m.find('|', i + 1)
            return 0 < j - i < 200
        return False

    def _expr_end(self, k, b):
        m, depth = self.m, 0
        while k < b:
            ch = m[k]
            if ch in '([{': depth += 1
            elif ch in ')]}':
                if depth == 0: return k
                depth -= 1
            elif ch in ',;' and depth == 0: return k
            k += 1
        return b

    def _classify(self, name, popen, pclose, i):
        k = self._classify0(name, popen, pclose, i)
        if BYTE_FILTER[0] and k == 'C' and not BYTE_RE.search(self.src[popen + 1:pclose]):
            return 'D'  # a precondition, not a comparison of produced bytes
        return k

    def _classify0(self, name, popen, pclose, i):
        text = self.src[popen + 1:pclose]
        mtext = self.m[popen + 1:pclose]
        if name in ('assert', 'debug_assert'):
            # condition = up to first top-level comma
            depth, cut = 0, len(mtext)
            for k, ch in enumerate(mtext):
                if ch in '([{': depth += 1
                elif ch in ')]}': depth -= 1
                elif ch == ',' and depth == 0: cut = k; break
            cond = mtext[:cut]
            if DIAG_COND_RE.search(cond): return 'D'
            return 'C'
        if name in ('assert_eq', 'assert_ne', 'debug_assert_eq'):
            return 'D' if DIAG_COND_RE.search(mtext) else 'C'
        # panic / unreachable: by message
        return 'D' if DIAG_MSG_RE.search(text) else 'C'


def unroll(ev):
    out = []
    for e in ev:
        if e[0] == 'LOOP':
            body = unroll(e[1])
            out += body + body
        else:
            out.append(e)
    return out


def masking(ev):
    """Return (first P, masking C, later P) or None."""
    flat = unroll(ev)
    first_p = None
    pending_c = None
    for e in flat:
        if e[0] == 'P':
            if first_p is not None and pending_c is not None:
                return first_p, pending_c, e
            if first_p is None: first_p = e
        elif e[0] == 'C' and first_p is not None and pending_c is None:
            pending_c = e
    return None


TRACE = [False]
BYTE_FILTER = [False]
ALL_FNS = [False]


def read(path, rev):
    if rev:
        return subprocess.run(['git', 'show', f'{rev}:{path}'], capture_output=True,
                              text=True, check=True).stdout
    return open(path).read()


def main(argv):
    rev, verbose, files = None, False, []
    it = iter(argv)
    for a in it:
        if a == '--rev': rev = next(it)
        elif a == '--verbose': verbose = True
        elif a == '--trace': TRACE[0] = True
        elif a == '--byte-filter': BYTE_FILTER[0] = True
        elif a == '--all-fns': ALL_FNS[0] = True
        else: files.append(a)
    tests_total = flagged_total = 0
    for path in files:
        src = read(path, rev)
        # shared test modules (mod x;) next to the file are inlined by name
        extra = {}
        d = os.path.dirname(path)
        for mm in re.finditer(r'^\s*mod\s+(\w+)\s*;', mask(src), re.M):
            for cand in (f'{d}/{mm.group(1)}.rs', f'{d}/{mm.group(1)}/mod.rs'):
                try:
                    msrc = read(cand, rev)
                except Exception:
                    continue
                fs = FileScan(msrc, {})
                for f in fs.fns:
                    extra.setdefault(f[0], (fs, f))
        fs = FileScan(src, extra)
        for (name, pos, bo, bc) in fs.fns:
            if not ALL_FNS[0] and not is_test(fs.m, pos): continue
            tests_total += 1
            ev = fs.events(bo + 1, bc, [name])
            nP = sum(1 for e in unroll(ev) if e[0] == 'P')
            r = masking(ev)
            aeon = any(e[0] == 'R' for e in unroll(ev)) or bool(
                re.search(r'AEON_DIR|s4\.bin|s4\.debug\.bin|demo\.bin', fs.src[bo:bc]))
            tag = 'AEON' if aeon else 'inline'
            if r:
                flagged_total += 1
                p1, c, p2 = r
                print(f'FLAG {tag} {path}::{name}  P={nP}')
                print(f'     first P : {p1[2][:90]}')
                print(f'     compare : {c[2][:110]}')
                print(f'     later P : {p2[2][:90]}')
                if TRACE[0]:
                    last = None
                    for e in unroll(ev):
                        if e[0] not in 'PC': continue
                        line = f'       {e[0]} {e[2][:120]}'
                        if line != last: print(line)
                        last = line
            elif verbose:
                print(f'ok   {tag} {path}::{name}  P={nP}')
    print(f'SUMMARY files={len(files)} tests={tests_total} flagged={flagged_total}')


if __name__ == '__main__':
    main(sys.argv[1:])
