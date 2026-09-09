"""Half one of the losslessness proof for a cut of the boot read.

    OMD_BASE=<rev before the cut> python3 scripts/boot_read_cut_lossless.py

Every non-blank line that left `docs/OVERSEER.md` must be present in the text this
cut appended to `docs/OVERSEER-REFERENCE.md` or `docs/OVERSEER-LOG.md`. Exit 0 when
nothing was lost, exit 1 naming each line that was.

Multiset semantics, not set semantics: a line removed twice must appear twice in the
destinations, so a duplicated line cannot be laundered by one surviving copy.

**This is half of the proof, and on its own it is not a proof.** A set difference is
blind to a sentence left behind whose antecedent moved out with its paragraph, and it
cannot tell a move from a deliberate rewrite. So the second half is a human reading
every seam, and every rewrite is declared by hand in DECLARED_REPAIRS below with the
reason it was not a move. A cut that grows that dict without a report saying why has
turned this instrument off one line at a time.

Run the positive control before believing a green: perturb one moved line in each
destination, confirm this goes red naming both, restore, and confirm the files' md5
returns to its pre-perturbation value. A restore that is not checked by digest is
indistinguishable from a mutation that never applied.
"""
import os
import subprocess
import sys
from collections import Counter

ROOT = subprocess.run(['git', 'rev-parse', '--show-toplevel'],
                      capture_output=True, text=True, check=True).stdout.strip()
os.chdir(ROOT)
BASE = os.environ.get('OMD_BASE', 'HEAD')

BOOT = 'docs/OVERSEER.md'
DESTINATIONS = ('docs/OVERSEER-REFERENCE.md', 'docs/OVERSEER-LOG.md')

# Text a cut deliberately REWROTE rather than moved. Each entry is a line that left the
# boot read and is deliberately nowhere in the destinations, with the reason beside it.
# Anything else that vanishes is a loss and fails.
DECLARED_REPAIRS = {
    # 2026-09-09 cut, second cut of the boot read:
    # a cross-reference into the moved SIGIL-DECOUPLE block, whose "below" no longer resolves
    "SIGIL-DECOUPLE section below (what the coupling buys) and in": 1,
    # the article on the preceding line went with it ("in the SIGIL-DECOUPLE section" -> "in `path`")
    "bug and `[layout.odd-field]` are the same shape one layer out, already banked in the": 1,
    # the 2026-09-04 index now names the two later cuts, so the chain of indexes is followable
    "the whole of `docs/OVERSEER-REFERENCE.md`.": 1,
}


def at_base(path):
    return subprocess.run(['git', 'show', '%s:%s' % (BASE, path)],
                          capture_output=True, text=True, check=True).stdout.split('\n')


def now(path):
    return open(path, encoding='utf-8').read().split('\n')


def body(lines):
    return Counter(line for line in lines if line.strip())


removed = body(at_base(BOOT)) - body(now(BOOT))

appended = Counter()
for path in DESTINATIONS:
    appended += body(now(path)) - body(at_base(path))

missing = removed - appended - Counter(DECLARED_REPAIRS)
stale = Counter(DECLARED_REPAIRS) - (removed - appended)

print('base revision                                  :', BASE)
print('lines removed from the boot read (multiset)    :', sum(removed.values()))
print('declared in-pass repairs (rewritten, not moved):', sum(DECLARED_REPAIRS.values()))
print('lines this cut appended to the destinations    :', sum(appended.values()))
print('removed lines NOT found in the destinations    :', sum(missing.values()))
if stale:
    print('WARNING: a declared repair did not fire, so it is describing a cut that is not this one:')
    for line, count in stale.items():
        print('  STALE x%d | %s' % (count, line[:150]))
for line, count in missing.items():
    print('  MISSING x%d | %s' % (count, line[:150]))

# An empty measurement is never a pass. With no OMD_BASE the base defaults to HEAD,
# so the boot read is compared with itself: nothing was removed, nothing can be
# missing, and this exits 0 having examined no text at all. That green is
# indistinguishable from a lossless cut and is the shape it exists to refuse, so it
# refuses itself first. Anything that reads this exit status is entitled to assume
# some text was actually weighed.
if not removed:
    print('REFUSED: no line left the boot read between %s and the working tree, so this run'
          % BASE)
    print('         weighed no text and its zero is not a finding. Name the revision BEFORE')
    print('         the cut: OMD_BASE=<rev> python3 scripts/boot_read_cut_lossless.py')
    sys.exit(2)

sys.exit(1 if missing else 0)
