#!/usr/bin/env python3
"""Split every `<name> FUNCTION <arg>,..,<arg>,<expr>` definition on stdin into
its parameters and its body, and report the ones whose body never mentions a
parameter.

Why this is its own file rather than a line of shell: the body may contain
commas inside parentheses — `pcmLoopCounter function sampleRate,
pcmLoopCounterBase(sampleRate,90)` is a real definition in `s1disasm` — so the
split has to count nesting. A shell `tr ',' '\\n'` gets that wrong in exactly the
direction that hides a hit, by leaving a fragment of the body in the parameter
list and then failing to find it in the remainder.

Why param-ignoring definitions are worth enumerating: AS's manual says of a
`FUNCTION` call that *"all parameters are calculated once and are then inserted
into the function's formula"* — AS is STRICT in the arguments. An expander that
is LAZY instead never evaluates an argument the body does not mention, so an
argument AS would refuse (an undefined symbol, a register) reaches no refusal at
all. Those definitions are where that difference is observable.

Input: `<path>:<lineno>:<text>` lines, as `grep -rn` prints them.
Output: one line per param-ignoring definition, then a count. Exit 0 always;
this is an enumerator, not a gate.
"""
import re
import sys


def split_top_level(s):
    """Split on commas that are not inside parentheses, brackets or quotes."""
    out, depth, cur, quote = [], 0, [], None
    for ch in s:
        if quote:
            cur.append(ch)
            if ch == quote:
                quote = None
            continue
        if ch in "'\"":
            quote = ch
            cur.append(ch)
        elif ch in "([":
            depth += 1
            cur.append(ch)
        elif ch in ")]":
            depth -= 1
            cur.append(ch)
        elif ch == ',' and depth == 0:
            out.append(''.join(cur))
            cur = []
        else:
            cur.append(ch)
    out.append(''.join(cur))
    return out


DEF = re.compile(
    r'^(?P<name>[A-Za-z_.][A-Za-z0-9_.]*)\s+[Ff][Uu][Nn][Cc][Tt][Ii][Oo][Nn]\s+(?P<rest>.*)$'
)


def main():
    hits = 0
    total = 0
    for line in sys.stdin:
        line = line.rstrip('\n')
        # grep -rn form: path:lineno:text
        parts = line.split(':', 2)
        where, text = (':'.join(parts[:2]), parts[2]) if len(parts) == 3 else ('', line)
        # AS ends a statement at an unquoted ';'
        text = text.split(';', 1)[0]
        m = DEF.match(text.strip())
        if not m:
            continue
        total += 1
        pieces = split_top_level(m.group('rest'))
        if len(pieces) < 2:
            continue
        params = [p.strip() for p in pieces[:-1]]
        body = pieces[-1]
        used = [p for p in params
                if p and re.search(r'(?<![A-Za-z0-9_.])' + re.escape(p) + r'(?![A-Za-z0-9_.])', body)]
        if not used:
            hits += 1
            print(f"PARAM-IGNORING {where}: {text.strip()}")
    print(f"definitions parsed: {total}")
    print(f"param-ignoring:     {hits}")


if __name__ == '__main__':
    main()
