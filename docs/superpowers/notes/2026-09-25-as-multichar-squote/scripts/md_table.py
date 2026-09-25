"""md_table.py <log> <out.md>: the full probe table (probe, source, asl, sigil base,
sigil tip, verdict) as markdown, sources read from the probe files themselves."""
import re, sys, os
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
log = open(sys.argv[1]).read()
rows = []
seen = set()
for b in log.split('== ')[1:]:
    fname = b.split('\n')[0]
    name = fname.replace('.asm', '')
    if name in seen:
        continue
    seen.add(name)
    def get(tag):
        m = re.search(tag + r' \(exit 0\): (\S*)', b)
        return (m.group(1) or '(nothing)') if m else None
    asl = get('asl  ')
    if asl is None:
        m = re.search(r'error #(\d+)', b)
        asl = 'refused #' + m.group(1) if m else 'refused'
    base = get('sigil-base') or 'refused'
    tip = get('sigil-tip') or 'refused'
    ok_b = base == asl or (asl.startswith('refused') and base == 'refused')
    ok_t = tip == asl or (asl.startswith('refused') and tip == 'refused')
    verdict = 'same' if ok_t else 'differs'
    if ok_t and not ok_b:
        verdict = 'fixed'
    src = open(os.path.join(P, fname), encoding='latin-1').read()
    lines = [l.strip() for l in src.split('\n') if l.strip() and l.strip() not in ('cpu 68000', 'cpu z80', 'padding off', 'org 0', 'end')]
    cpu = 'z80' if 'cpu z80' in src else '68k'
    s = ' / '.join(lines).replace('|', '\\|')
    if len(s) > 110:
        s = s[:107] + '...'
    rows.append(f"| {name} | {cpu} | `{s}` | {asl} | {base} | {tip} | {verdict} |")
with open(sys.argv[2], 'w') as f:
    f.write("| probe | cpu | source | asl (exit 0 bytes, or refusal) | sigil base `1e146771` | sigil tip | tip vs asl |\n")
    f.write("|---|---|---|---|---|---|---|\n")
    f.write('\n'.join(rows) + '\n')
print(len(rows))
