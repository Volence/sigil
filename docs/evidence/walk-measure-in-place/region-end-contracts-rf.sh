#!/bin/bash
cd /home/volence/sonic_hacks/.sigil-walk
export AEON_DIR=/home/volence/sonic_hacks/.walk-freeze-aeon SIGIL_STRICT_GATE=1 CARGO_TARGET_DIR=.target-land
run() { cargo test --release -p sigil-harness --test region_end_contracts 2>&1 | grep -E "^test |^test result|panicked at|^the |^region .|measured NOTHING|resolved with" | cut -c1-260; }
echo "region_end_contracts red-first transcript ($(date -u +%F), committed baseline $(git rev-parse --short HEAD))"
echo; echo "RF-A  the parser silently drops one region (re-grounded 'did the parse break?' guard)"
python3 - <<'PY'
p='crates/sigil-harness/src/repin.rs'; s=open(p).read()
o='    let m: Manifest = toml::from_str(src).map_err(|e| format!("repin.toml parse error: {e}"))?;\n'
assert s.count(o)==1
s=s.replace(o,'    let mut m: Manifest = toml::from_str(src).map_err(|e| format!("repin.toml parse error: {e}"))?;\n    m.regions.pop(); // RF-MUTATION-A\n')
open(p,'w').write(s)
PY
echo "== mutation on disk:"; git diff -U0 crates/sigil-harness/src/repin.rs | grep "^[+-] "
run
echo "== restore: git restore --source=HEAD -- crates/sigil-harness/src/repin.rs"; git restore --source=HEAD -- crates/sigil-harness/src/repin.rs
echo; echo "RF-B  the strict end contract stops judging (negative control must go red)"
python3 - <<'PY'
p='crates/sigil-harness/src/repin.rs'; s=open(p).read()
o='    // A `section:` end IS the content end by construction; there is nothing to judge.\n'
assert s.count(o)==1
s=s.replace(o,'    let _ = (region, shape, listing, lo, hi); return Ok(()); // RF-MUTATION-B\n    #[allow(unreachable_code)]\n'+o)
open(p,'w').write(s)
PY
echo "== mutation on disk:"; git diff -U0 crates/sigil-harness/src/repin.rs | grep "^[+-] "
run
echo "== restore: git restore --source=HEAD -- crates/sigil-harness/src/repin.rs"; git restore --source=HEAD -- crates/sigil-harness/src/repin.rs
echo; echo "RF-C  the control finds no doctoring target (loud when unmeasurable)"
python3 - <<'PY'
p='crates/sigil-harness/tests/region_end_contracts.rs'; s=open(p).read()
o='        if let Some(label) = heads.first() {\n'
assert s.count(o)==1
s=s.replace(o,'        if let Some(label) = heads.first().filter(|_| false) { // RF-MUTATION-C\n')
open(p,'w').write(s)
PY
echo "== mutation on disk:"; git diff -U0 crates/sigil-harness/tests/region_end_contracts.rs | grep "^[+-] "
run
echo "== restore: git restore --source=HEAD -- crates/sigil-harness/tests/region_end_contracts.rs"; git restore --source=HEAD -- crates/sigil-harness/tests/region_end_contracts.rs
echo; echo "== restored baseline:"; git status --porcelain crates/; run
echo END
