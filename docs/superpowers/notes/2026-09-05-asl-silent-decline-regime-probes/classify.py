#!/usr/bin/env python3
"""Turn `sigil_today.sh`'s output into the regime table, and count it.

    ./sigil_today.sh <sigil> 4 | ./classify.py

Two classifications, and neither is a judgement — both are read straight off the
run:

  asl SILENT   the shape's own file exited 0 and printed no diagnostic at all.
               asl still emitted something; that something is in the `asl word`
               column, and where it is the `$A101` the setter above it computed,
               asl DECLINED the operand and substituted. A SILENT row with a
               distinct word is an ANSWER, which is what the `ctl_` rows are for.
  asl LOUD     anything else.

  sigil accept exit 0.
  sigil refuse anything else.

The rows that matter are the disagreements, and they are printed separately at
the end rather than left for a reader to spot: an asl-SILENT row sigil accepts is
a shape where NEITHER assembler refuses, and an asl-LOUD row sigil accepts is a
shape where sigil is the more permissive of the two — the worse direction.

The control rows must every one of them be SILENT-with-an-answer and accepted by
sigil. A control that comes back carrying the setter's word means the harness is
measuring itself, and the whole table is void.
"""
import re
import sys

SETTER = 'A101'


def main():
    text = sys.stdin.read()
    blocks = text.split('\n=== ')
    rows = []
    for b in blocks[1:]:
        lines = b.split('\n')
        ident = lines[0].strip()
        if ident.startswith('ALL SHAPES'):
            continue
        aslexit = aslword = sigexit = None
        loud = incomplete = False
        sigmsg = []
        for l in lines:
            m = re.match(r'\s*aslref exit=(\d+)', l)
            if m:
                aslexit = int(m.group(1))
            # line 12 of the generated file is the shape itself
            m = re.match(r'\s*aslref \| \s*12/\s*\S+ : (.*?)\t', l)
            if m:
                aslword = ' '.join(m.group(1).split())
            if l.strip().startswith('aslref !'):
                loud = True
                if 'INCOMPLETE' in l:
                    incomplete = True
            m = re.match(r'\s*sigil  exit=(\d+)', l)
            if m:
                sigexit = int(m.group(1))
            m = re.match(r'\s*sigil  \| (.*)', l)
            if m:
                sigmsg.append(m.group(1))
        rows.append({
            'id': ident,
            'asl': 'LOUD' if loud else 'SILENT',
            'exit': aslexit,
            'word': aslword or '(no bytes)',
            'sigil': 'accept' if sigexit == 0 else 'refuse',
            'detail': sigmsg[0] if sigmsg else '',
            'incomplete': incomplete,
        })

    print(f"{'shape':32} {'asl':7} {'exit':4} {'asl word':24} {'sigil':7} detail")
    for r in rows:
        print(f"{r['id']:32} {r['asl']:7} {str(r['exit']):4} {r['word']:24} "
              f"{r['sigil']:7} {r['detail'][:70]}")

    ctl = [r for r in rows if r['id'].startswith('ctl_')]
    sub = [r for r in rows if not r['id'].startswith('ctl_')]
    silent = [r for r in sub if r['asl'] == 'SILENT']
    loudr = [r for r in sub if r['asl'] == 'LOUD']

    print()
    print(f"shapes measured (non-control): {len(sub)}")
    print(f"  asl SILENT (exit 0, no diagnostic): {len(silent)}")
    print(f"  asl LOUD:                           {len(loudr)}")
    print(f"controls: {len(ctl)}")

    bad_ctl = [r['id'] for r in ctl
               if r['asl'] != 'SILENT' or SETTER in r['word'] or r['sigil'] != 'accept']
    if bad_ctl:
        print(f"  HARNESS VOID — controls that did not answer: {bad_ctl}")
    else:
        print("  every control answered on both assemblers — harness sound")

    inc = [r['id'] for r in rows if r['incomplete']]
    print(f"listings whose pass loop stopped early (measurement INCOMPLETE): "
          f"{len(inc)} {inc if inc else ''}")

    print()
    print("asl SILENT and sigil ACCEPTS (neither refuses):",
          [r['id'] for r in silent if r['sigil'] == 'accept'])
    print("asl LOUD and sigil ACCEPTS (sigil the more permissive):",
          [r['id'] for r in loudr if r['sigil'] == 'accept'])


if __name__ == '__main__':
    main()
