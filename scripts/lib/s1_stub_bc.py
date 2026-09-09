#!/usr/bin/env python3
"""s1_stub_bc.py - stand in for the two constructs Sonic 1 still refuses.

This is a MEASUREMENT aid, not a fix and not a build. It edits a scratch copy of
the corpus so that the two remaining frontend causes stop refusing, and the run
is allowed to walk past them and report whatever was standing behind them. The
population it exposes is the thing a fix parcel has to be sized from, because a
complaint count can only see what the front end REFUSED, never what is left.

Two stubs, and they are NOT equally honest:

  E, `switch`/`case` over an integer  ->  `if`/`elseif`/`else` over the same
      expression. AS's `switch` is an if/elseif chain, not a C fallthrough, so
      this is byte-neutral by construction: the same expression selects the same
      arm. `switch` over a STRING is left alone, sigil already accepts it.

  B, `charset`  ->  the line is commented out. This is NOT byte-neutral. A
      `charset` remaps the code page for subsequent string literals, so the text
      it governs comes out in ASCII rather than in the font's encoding. Any byte
      claim about the region a `charset` governs is void under this stub, and
      the script says so on stdout rather than leaving a reader to remember.

Usage: s1_stub_bc.py <corpus-dir>
Exit 0 only if BOTH stubs applied to a non-zero number of lines. A stub that
matched nothing is a failed edit that would otherwise read as a clean pass.
"""
import re
import sys
import pathlib

SWITCH = re.compile(r'^(\s*)switch\s+(\S.*?)\s*(;.*)?$')
CASE = re.compile(r'^(\s*)case\s+(\S.*?)\s*(;.*)?$')
ELSECASE = re.compile(r'^(\s*)elsecase\b\s*(;.*)?$')
ENDCASE = re.compile(r'^(\s*)endcase\b\s*(;.*)?$')
CHARSET = re.compile(r'^(\s*)charset\b')


def stub_switch(path):
    """Rewrite integer `switch`/`case` blocks as `if`/`elseif`/`else`."""
    lines = path.read_text(encoding='utf-8', errors='surrogateescape').split('\n')
    out = []
    stack = []          # (subject expression, arms seen so far)
    changed = 0
    for line in lines:
        m = SWITCH.match(line)
        if m:
            indent, subject = m.group(1), m.group(2)
            # A string subject is already accepted; leave the whole block alone.
            passthrough = subject.startswith('"') or subject.startswith("'")
            stack.append([subject, 0, passthrough])
            if passthrough:
                out.append(line)
            else:
                out.append(indent + '; [stub E] switch ' + subject)
                changed += 1
            continue
        if stack and not stack[-1][2]:
            subject, arms, _ = stack[-1]
            m = CASE.match(line)
            if m:
                indent, value = m.group(1), m.group(2)
                kw = 'if' if arms == 0 else 'elseif'
                stack[-1][1] = arms + 1
                out.append('%s%s (%s)=(%s)' % (indent, kw, subject, value))
                changed += 1
                continue
            m = ELSECASE.match(line)
            if m:
                out.append(m.group(1) + 'else')
                changed += 1
                continue
            m = ENDCASE.match(line)
            if m:
                stack.pop()
                out.append(m.group(1) + 'endif')
                changed += 1
                continue
        elif stack and ENDCASE.match(line):
            stack.pop()
            out.append(line)
            continue
        out.append(line)
    if stack:
        print('  FATAL: %s ends with %d unclosed switch block(s)' % (path, len(stack)))
        return -1
    path.write_text('\n'.join(out), encoding='utf-8', errors='surrogateescape')
    return changed


def stub_charset(path):
    """Comment out `charset`. NOT byte-neutral; see the module docstring."""
    lines = path.read_text(encoding='utf-8', errors='surrogateescape').split('\n')
    out = []
    changed = 0
    for line in lines:
        if CHARSET.match(line):
            out.append('; [stub B, NOT byte-neutral] ' + line.strip())
            changed += 1
        else:
            out.append(line)
    path.write_text('\n'.join(out), encoding='utf-8', errors='surrogateescape')
    return changed


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    root = pathlib.Path(sys.argv[1])
    if not root.is_dir():
        print('FATAL: %s is not a directory' % root)
        return 2

    sources = sorted(
        p for p in root.rglob('*')
        if p.suffix.lower() in ('.asm', '.inc') and p.is_file()
    )
    print('== stub population ==')
    print('  scanned     %d .asm/.inc file(s) under %s' % (len(sources), root))

    n_switch = 0
    n_charset = 0
    for p in sources:
        text = p.read_text(encoding='utf-8', errors='surrogateescape')
        if 'switch' in text or 'endcase' in text:
            r = stub_switch(p)
            if r < 0:
                return 3
            if r:
                print('  stub E      %-40s %d line(s)' % (p.relative_to(root), r))
            n_switch += r
        if 'charset' in text:
            r = stub_charset(p)
            if r:
                print('  stub B      %-40s %d line(s)' % (p.relative_to(root), r))
            n_charset += r

    print('  TOTAL       stub E %d line(s), stub B %d line(s)' % (n_switch, n_charset))
    print()
    print('  WARNING: stub B (charset) is NOT byte-neutral. Under it the text')
    print('           the charset governs is emitted in ASCII, so no byte claim')
    print('           about that region survives this edit. Stub E is neutral by')
    print('           construction: AS switch/case IS an if/elseif chain.')
    if n_switch == 0 or n_charset == 0:
        print('  FATAL: a stub matched ZERO lines. That is a failed edit, which')
        print('         would otherwise be indistinguishable from a clean tree.')
        return 3
    return 0


if __name__ == '__main__':
    sys.exit(main())
