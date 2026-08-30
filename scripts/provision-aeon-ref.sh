#!/usr/bin/env bash
# Provision an AEON_DIR reference tree for sigil's strict/port gates.
#
# WHY THIS EXISTS: a bare `git worktree add --detach` is NECESSARY AND NOT
# SUFFICIENT. The tree needs gitignored build artifacts that the worktree does not
# carry, and WITHOUT THEM THE SUITE REPORTS ~200 FAILURES THAT READ EXACTLY LIKE
# GOLDEN DIVERGENCE. That signature is unfalsifiable from the log alone, which is
# why this is a script and not a paragraph somebody re-derives under time pressure.
#
# Usage:  scripts/provision-aeon-ref.sh [<worktree-path>] [<aeon-rev>]
# Default rev is the aeon_rev pinned by the LAST entry of golden/provenance.toml.
#
# The positive witness that provisioning WORKED is not "no errors". It is
#   repin --check  ->  "pins.rs unchanged"
# A tree provisioned wrongly cannot reproduce the pinned revision's placement, so
# an unchanged pin file is a POSITIVE result rather than an absence. Run it at the
# end (this script tells you the exact command) and do not trust the tree until it
# passes.
set -euo pipefail

SIGIL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AEON_REPO="${AEON_REPO:-$(cd "$SIGIL_ROOT/../aeon" && pwd)}"
GOLDEN="$SIGIL_ROOT/crates/sigil-harness/golden"
W="${1:-$SIGIL_ROOT/../.aeon-ref}"

pinned_rev() {
  python3 - "$GOLDEN/provenance.toml" <<'PY'
import re, sys
last = open(sys.argv[1]).read().split('[[entry]]')[-1]
m = re.findall(r'aeon_rev = "([0-9a-f]{40})"', last)
print(m[0] if m else "", end="")
PY
}
REV="${2:-$(pinned_rev)}"
[ -n "$REV" ] || { echo "ERROR: no aeon_rev in the provenance tail and none given" >&2; exit 1; }

echo "==> reference tree : $W"
echo "==> aeon revision  : $REV"

# 1. The revision must be REACHABLE FROM THE REMOTE, read with ls-remote at
#    measurement time. A local tracking ref is a cached answer that goes stale
#    silently, and a sibling checkout's HEAD is one working tree's opinion.
git -C "$AEON_REPO" fetch -q origin
if ! git -C "$AEON_REPO" merge-base --is-ancestor "$REV" origin/master 2>/dev/null; then
  echo "ERROR: $REV is not reachable from aeon origin/master. Refusing to pin to it." >&2
  exit 1
fi

# 2. The worktree itself. NEVER use ../aeon's live tree: it is a peer's working
#    directory and may be mid-edit or deliberately behind its own remote.
if [ -d "$W" ]; then
  echo "==> worktree exists, leaving it alone"
else
  git -C "$AEON_REPO" worktree add --detach "$W" "$REV"
fi

# 3. Reference ROMs, copied from sigil's own goldens and VERIFIED by CRC32+size
#    against the provenance tail. Non-circular: they are not built by the tree
#    under test. Provenance identity is CRC32+size, never SHA1.
echo "==> placing and verifying reference ROMs"
python3 - "$GOLDEN" "$W" <<'PY'
import re, sys, zlib, shutil, pathlib
golden, w = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
last = (golden / "provenance.toml").read_text().split('[[entry]]')[-1]
m = re.search(r'\[entry\.strict\.goldens\](.*?)(\n\[|\Z)', last, re.S)
exp = {k: (c, int(s)) for k, c, s in re.findall(r'(\w+)\s*=\s*"([0-9a-f]{8})/(\d+)"', m.group(1))}
names = {"s4": "s4.bin", "s4_debug": "s4.debug.bin",
         "demo": "demo.bin", "demo_debug": "demo.debug.bin"}
bad = 0
for key, fn in names.items():
    if key not in exp:
        print(f"    {fn:16} no provenance entry, SKIPPED"); continue
    src = golden / fn
    if not src.exists():
        print(f"    {fn:16} MISSING FROM GOLDENS"); bad += 1; continue
    shutil.copy2(src, w / fn)
    d = (w / fn).read_bytes()
    got, want = format(zlib.crc32(d) & 0xffffffff, '08x'), exp[key]
    ok = got == want[0] and len(d) == want[1]
    print(f"    {fn:16} {got}/{len(d)} {'OK' if ok else 'MISMATCH expected ' + want[0] + '/' + str(want[1])}")
    bad += 0 if ok else 1
if bad:
    raise SystemExit(f"{bad} reference ROM(s) failed verification; refusing to continue")
PY

# 4. The vendored ZX0 packer, then the compression self-test vectors it feeds.
#    Missing vectors present as `no module engine.compression_vectors` plus a run
#    of [embed.not-found], NOT as divergence, so the gate names them honestly.
echo "==> salvador + compression vectors"
if [ ! -x "$W/tools/bin/salvador" ]; then
  make -C "$W/tools/salvador" -s
  mkdir -p "$W/tools/bin"
  cp "$W/tools/salvador/salvador" "$W/tools/bin/salvador"
fi
( cd "$W" && python3 tools/gen_compression_vectors.py )

# 5. The sound blob tree. repin resolves sound-ON, so it needs emit_sound_blob;
#    build it into a DEDICATED target dir. Never share one CARGO_TARGET_DIR
#    between two worktrees of this repo, and never relink the shared
#    target/release/sigil as a side effect of provisioning: that artifact is
#    consumed by other lanes and moving it is a broadcast-worthy act.
echo "==> emit_sound_blob + engine/sound/generated"
REF_TARGET="${REF_TARGET:-$SIGIL_ROOT/../.sigil-ref-target}"
mkdir -p "$REF_TARGET" "$W/engine/sound/generated"
( cd "$SIGIL_ROOT" && CARGO_TARGET_DIR="$REF_TARGET" cargo build --release --bin emit_sound_blob )

cat <<EOF

Provisioned. NOW PROVE IT, because "no errors" is not a witness:

  cd "$SIGIL_ROOT" && \\
  AEON_DIR="$W" \\
  SIGIL_EMIT="$REF_TARGET/release/emit_sound_blob" \\
  CARGO_TARGET_DIR="$REF_TARGET" \\
    cargo run --release -p sigil-harness --bin repin -- --check

Expect the burndown warnings, then "pins.rs unchanged". That line is the positive
witness that this tree reproduces $REV's placement. Anything else means the
provisioning is wrong, NOT that the pins have drifted.
EOF
