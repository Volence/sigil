#!/bin/bash
C=/home/volence/sonic_hacks/.walk-pyc-r2
L=/home/volence/sonic_hacks/.walk-runs
export SIGIL_BUILD=/home/volence/sonic_hacks/.sigil-walk/.target-land/release/sigil SIGIL_EMIT=/home/volence/sonic_hacks/.sigil-walk/.target-land/release/emit_sound_blob
cd $C
echo "tree: cp -a of .aeon-sigil-ref-99cb963d at $C ($(git rev-parse --short HEAD)); SIGIL_BUILD=$SIGIL_BUILD"
echo "== RED: copied __pycache__ present: $(find $C -name __pycache__ -type d | wc -l) dirs"
python3 - <<'PY'
import marshal,glob
f=glob.glob('/home/volence/sonic_hacks/.walk-pyc-r2/tools/__pycache__/test_citation_form.cpython-314-pytest-*.pyc')[0]
co=marshal.loads(open(f,'rb').read()[16:]); print("co_filename in", f.split('/')[-1], "=", co.co_filename)
PY
python3 -m pytest tools -q --no-header -p no:cacheprovider -m "not needs_build" > $L/pyc-red.log 2>&1; echo "RED rc=$?"
grep "LAND GATE:" $L/pyc-red.log; grep -E "passed.* in " $L/pyc-red.log
echo "== remove the copied caches: find $C -name __pycache__ -type d -prune -exec rm -rf {} +"
find $C -name __pycache__ -type d -prune -exec rm -rf {} +
echo "remaining __pycache__ dirs: $(find $C -name __pycache__ -type d | wc -l)"
python3 -m pytest tools -q --no-header -p no:cacheprovider -m "not needs_build" > $L/pyc-green.log 2>&1; echo "GREEN rc=$?"
echo "LAND GATE sections: $(grep -c 'LAND GATE:' $L/pyc-green.log)"; grep -E "passed.* in " $L/pyc-green.log
echo END
