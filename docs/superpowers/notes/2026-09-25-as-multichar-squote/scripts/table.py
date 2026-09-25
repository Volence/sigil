"""table.py <log>: one row per probe (asl, base, tip) and the regression check:
a probe where base matched asl and tip does not."""
import re, sys
log = open(sys.argv[1]).read()
rows = []
for b in log.split('== ')[1:]:
    name = b.split('\n')[0].replace('.asm', '')
    def get(tag):
        m = re.search(tag + r' \(exit 0\): (\S*)', b)
        if m:
            return m.group(1) or '(none)'
        return None
    asl = get('asl  ')
    if asl is None:
        m = re.search(r'error #(\d+)', b)
        asl = 'refused #' + m.group(1) if m else 'refused'
    base = get('sigil-base') or 'refused'
    tip = get('sigil-tip') or 'refused'
    rows.append((name, asl, base, tip))
seen = set()
reg = 0
fixed = 0
for name, asl, base, tip in rows:
    if name in seen:
        continue
    seen.add(name)
    ok_base = base == asl or (asl.startswith('refused') and base == 'refused')
    ok_tip = tip == asl or (asl.startswith('refused') and tip == 'refused')
    flag = ''
    if ok_base and not ok_tip:
        flag = 'REGRESSION'
        reg += 1
    elif ok_tip and not ok_base:
        flag = 'fixed'
        fixed += 1
    elif not ok_tip:
        flag = 'open'
    print(f"| {name} | {asl} | {base} | {tip} | {flag} |")
print(f"probes {len(seen)} fixed {fixed} regressions {reg}")
