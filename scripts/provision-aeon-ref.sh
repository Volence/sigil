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
#
# AND THIS FILE MISSED ITS OWN ARGUMENT ON THE ONE INPUT IT CHOOSES ITSELF. Every
# paragraph above is about not trusting an absence, and the assembler — the program
# that produces every byte this script then controls — was picked with
# `${SIGIL_BIN:-$SIGIL_ROOT/target/release/sigil}`: whatever a shared checkout last
# relinked. On 2026-09-02 that certified a byte-neutrality proof against a PRE-MERGE
# compiler and printed four `MATCHES THE GOLDEN` lines doing it. The tool is now
# resolved and PROVED to correspond to this tree before anything else happens, in
# `scripts/lib/sigil_tool.sh`, which carries the reasoning.
set -euo pipefail

SIGIL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# ── STEP 0. THE ASSEMBLER, BEFORE ANYTHING ELSE ──────────────────────────────────
# First, because it is the one input this script cannot re-derive later and because a
# run with the wrong tool is worthless however well the rest goes — and because
# refusing here touches no peer's repository: no fetch, no worktree, nothing written.
# `sigil_tool_resolve` builds the tool from THIS tree when SIGIL_BIN is unset (never
# the shared target/release/sigil), honours an explicitly pinned one, heads the log
# with its --version self-report, and proves it corresponds to this tree.
#
# REF_TARGET is hoisted here from step 5 because the tool is built into it. It is
# resolved by `sigil_tool_ref_target`, which carries the precedence and the reason a
# caller's own CARGO_TARGET_DIR outranks any derivation this script could make: a
# fixed default is one directory for every worktree of this repo, and cargo's
# checkout-independent unit hash makes that an artifact-substitution fault rather than
# an untidy one.
# shellcheck source=lib/sigil_tool.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/sigil_tool.sh"
REF_TARGET="$(sigil_tool_ref_target "$SIGIL_ROOT")"
mkdir -p "$REF_TARGET"
sigil_tool_resolve "$SIGIL_ROOT" "$REF_TARGET"

# ── THE AEON REPOSITORY, AND WHY `$SIGIL_ROOT/../aeon` WAS WRONG ─────────────────────
# SIGIL_ROOT is this script's own directory's parent, which is correct for the sigil
# checkout and NOT correct as a base for a sibling: run from a linked worktree it is
# `<sigil>/.claude/worktrees/<agent>`, so `../aeon` names
# `<sigil>/.claude/worktrees/aeon`, which does not exist. Every sigil agent runs in a
# linked worktree, so that spelling failed for the majority of its callers — measured
# here, not supposed: this parcel's own first provisioning run died on it.
#
# `git rev-parse --git-common-dir` answers with the MAIN checkout's `.git` from a linked
# worktree and from the main checkout alike, which is the fact a sibling derivation
# needs; `scripts/lib/suite_paths.sh` is the one implementation, and it accepts an
# explicit AEON_DIR ahead of any derivation.
#
# AEON_REPO IS KEPT as this script's spelling of step 1 — the drift lane passes it, and
# contract/SUITE_PATHS.md lets an alias live during the transition provided the ratified
# name is documented. It is fed into the include so a set-but-wrong AEON_REPO is a hard
# error here exactly as a set-but-wrong AEON_DIR is everywhere else, rather than a `cd`
# failure whose message names no variable.
# shellcheck source=lib/suite_paths.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/suite_paths.sh"
AEON_DIR="${AEON_REPO:-${AEON_DIR:-}}"
AEON_REPO="$(suite_resolve_checkout aeon AEON_DIR)"

GOLDEN="$SIGIL_ROOT/crates/sigil-harness/golden"
# The reference tree this script CREATES. Not a checkout yet, so it hangs off the
# resolved suite root rather than off SIGIL_ROOT, which lies from a worktree in exactly
# the same way `../aeon` did.
W="${1:-$(suite_resolve_root)/.aeon-ref}"

pinned_rev() {
  python3 - "$GOLDEN/provenance.toml" <<'PY'
import re, sys
last = open(sys.argv[1]).read().split('[[entry]]')[-1]
m = re.findall(r'aeon_rev = "([0-9a-f]{40})"', last)
print(m[0] if m else "", end="")
PY
}
PINNED="$(pinned_rev)"
REV="${2:-$PINNED}"
[ -n "$REV" ] || { echo "ERROR: no aeon_rev in the provenance tail and none given" >&2; exit 1; }

# WHETHER THE GOLDEN CONTROL APPLIES IS DERIVED FROM THE REVISION, not from a flag.
# The control in step 6 asserts that a ROM built here matches the frozen golden, which
# is only a control at the PINNED revision: at any other revision the goldens describe
# different source and a difference is the expected outcome, not a fault. Deriving it
# means no caller can silence a real failure by passing an opt-out, and the nightly
# drift job — which provisions at the engine lane's LIVE TIP on purpose — is not
# refused for being what it is.
if [ "$REV" = "$PINNED" ]; then
  CONTROL=required
else
  CONTROL=not-applicable
fi

echo "==> reference tree : $W"
echo "==> aeon revision  : $REV"
echo "==> golden control : $CONTROL (pinned revision is ${PINNED:-none})"

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
#    AN EXISTING TREE IS NOT A TREE AT $REV. "Leaving it alone" checked that the
#    directory existed and nothing else, so every run after the first measured whatever
#    revision the tree was first created at while the CALLER recorded $REV as the
#    revision measured. The reachability check above ran against $REV and passed —
#    on a revision the run then did not use, which is what made it look right.
#    Observed 2026-09-02: the drift ledger's first record named aeon_rev 73b07a4f
#    while this tree sat at ddaef90c, 123 commits behind. Had the build succeeded,
#    a real drift observation would have been filed under a revision it was never
#    taken at — the one falsehood this instrument exists to not produce.
if [ -d "$W" ]; then
  have=$(git -C "$W" rev-parse HEAD 2>/dev/null || true)
  want=$(git -C "$AEON_REPO" rev-parse "$REV")
  if [ "$have" = "$want" ]; then
    echo "==> worktree exists at the requested revision, leaving it alone"
  elif [ -n "$(git -C "$W" status --porcelain 2>/dev/null)" ]; then
    echo "ERROR: $W is at ${have:-an unreadable HEAD}, not $want, and has uncommitted" >&2
    echo "       changes. Refusing to move it: a measurement here would be filed under" >&2
    echo "       a revision it was not taken at. Inspect it, then remove it or commit." >&2
    exit 1
  else
    echo "==> worktree is at ${have:-unknown}, re-pointing it to $want"
    git -C "$W" checkout --force --detach "$want"
  fi
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
# THE FREEZE AND THE ATTEST WRITE DIFFERENT BLOCKS, and reading the wrong one is
# how this crashed with a bare AttributeError on a legitimate tip. `--freeze`
# writes `[entry.targets.<shape>]` (full_crc / full_size); `--attest` LATER adds
# `[entry.strict.goldens]` (shape = "crc/size"). A tip that is frozen but not yet
# attested — the normal state between the two rituals, and exactly what chain 200
# was — has the first and not the second. Prefer targets, fall back to strict, and
# refuse BY NAME rather than letting `m.group(1)` raise on None.
rows = re.findall(
    r'\[entry\.targets\.(\w+)\].*?full_crc = "([0-9a-f]{8})".*?full_size = (\d+)',
    last, re.S)
if rows:
    exp = {k: (c, int(sz)) for k, c, sz in rows}
else:
    m = re.search(r'\[entry\.strict\.goldens\](.*?)(\n\[|\Z)', last, re.S)
    if m is None:
        raise SystemExit(
            "provision: the provenance tail carries NEITHER `[entry.targets.*]` (written by "
            "--freeze) NOR `[entry.strict.goldens]` (written by --attest), so this entry "
            "names no expected ROMs and there is nothing to place or verify. That is a "
            "malformed tip, not a missing tool — inspect golden/provenance.toml's last entry.")
    exp = {k: (c, int(s)) for k, c, s in
           re.findall(r'(\w+)\s*=\s*"([0-9a-f]{8})/(\d+)"', m.group(1))}
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
# The assembler is NOT chosen here any more. It was resolved and proved against this
# tree in step 0, before this run touched anything, and $SIGIL_BIN below is that
# binary. REF_TARGET was hoisted there with it.
mkdir -p "$W/engine/sound/generated"
( cd "$SIGIL_ROOT" && CARGO_TARGET_DIR="$REF_TARGET" cargo build --release --bin emit_sound_blob )

# 6. THE LISTINGS, and they are NOT optional. Several port gates resolve a
#    cross-region symbol by looking it up in s4.lst / s4.debug.lst
#    (`listing_symbol_addr`). A MISSING listing does not error there: the lookup
#    returns None, no label is pushed, and the failure surfaces much later as
#    `unresolved symbol <X> for fixup in section <Y>` in a test that has nothing
#    to do with listings. That reads exactly like a real regression in whatever
#    parcel you happen to be holding, and it cost this lane a false attribution
#    and three needless reverts before the cause was found.
#
#    At the PINNED revision, building them is also the STRONGEST control available: a
#    ROM built here from the pinned source must match the golden CRC32 byte for byte.
#    At any OTHER revision that comparison is not a control and its failure is not a
#    fault — the goldens describe different source — so the CRCs are printed as data
#    and nothing is asserted. Which of the two applies is derived from $REV above.
echo "==> building both shapes to emit the listings (and to control the ROMs)"
# The build's output is KEPT. `>/dev/null 2>&1` under `set -e` meant a failed build
# produced an exit code and no evidence whatsoever: the caller reported "provisioning
# exited 1" and the reason was gone. Observed 2026-09-02 — the reason turned out to be
# a stale tree failing a poison whose wording had moved, and recovering it took a
# by-hand rebuild that this redirection existed to make necessary. Suppression does not
# hide an error, it destroys the artifact that would have explained it.
# WHICH SHAPES GET BUILT, and why it is a choice rather than "all of them".
# The two s4 shapes are always built: the listings they emit are what `repin` resolves
# against, so no caller can skip them. The two DEMO shapes are opt-in via
# REF_BUILD_DEMO=1 because the two callers want genuinely different things and the
# difference is not a preference:
#
#   * A LANDING run wants the demo REFERENCE ROMs — the frozen goldens copied in at
#     step 3 — because the port gates compare a branch's lowering against the frozen
#     artifact. Building them there would replace the reference with the subject.
#   * The DRIFT job wants them BUILT, because a copied golden cannot exhibit drift.
#     That job measures only files whose mtime post-dates its build start, precisely so a
#     copy can never be read as a build.
#
# THE DRIFT JOB IS DELIBERATELY NOT NAMED IN THIS FILE, and re-adding its filename will
# turn a landing red. `no_landing_path_invokes_the_drift_job` asserts the job is reachable
# only from its own timer, and it matches on the NAME because it cannot tell a comment from
# an invocation — a rule that guessed from the directory instead would let a real wiring
# through. That test caught this very comment, which is the gate working: the property it
# protects (nothing executable can reach the job, so the job can block nothing) is what
# makes the job safe to leave running unattended. Adding this file to the test's lane-file
# exemption would be the wrong repair twice over: it is a SHARED provisioner that landing
# runs also call, so exempting it would blind the gate to a real wiring here forever.
#
# WITHOUT THIS FLAG THE DRIFT JOB COULD NEVER REPORT A CLEAN NIGHT. Its config declares
# four shapes; only two were ever built; `drift_report.py::observation_state` returns
# NOTHING MEASURED if ANY shape is unmeasured — deliberately, so quiet can never absorb
# an unmeasured shape — so its best possible verdict was NOTHING MEASURED, forever. A
# green nobody has ever seen is worse than a red: nobody investigates a state they have
# never been shown. The ranking in drift_report.py is right and is untouched; the defect
# was here, in a provisioner that built fewer shapes than its caller declared.
SHAPE_ENVS=("" "DEBUG=1")
SHAPE_ARGS_LIST=("" "")
if [ "${REF_BUILD_DEMO:-0}" = "1" ]; then
  SHAPE_ENVS+=("" "DEBUG=1")
  SHAPE_ARGS_LIST+=("demo" "demo")
  echo "==> REF_BUILD_DEMO=1: the two demo shapes are built too, not copied"
fi
for i in "${!SHAPE_ENVS[@]}"; do
  shape_env="${SHAPE_ENVS[$i]}"
  shape_arg="${SHAPE_ARGS_LIST[$i]}"
  tag="${shape_arg:-sonic4}${shape_env:+-debug}"
  build_log="$W/.provision-build-$tag.log"
  if ! ( cd "$W" && env $shape_env SIGIL_BUILD="$SIGIL_BIN" \
        SIGIL_EMIT="$REF_TARGET/release/emit_sound_blob" NO_LINT=1 ./build.sh $shape_arg ) \
        > "$build_log" 2>&1; then
    echo "ERROR: ./build.sh $shape_arg ${shape_env:-plain} failed in $W; its last 40 lines:" >&2
    tail -40 "$build_log" >&2
    echo "(full output: $build_log)" >&2
    exit 1
  fi
done
for l in s4.lst s4.debug.lst; do
  [ -s "$W/$l" ] || { echo "ERROR: $l was not produced; port gates will fail misleadingly" >&2; exit 1; }
done
python3 - "$GOLDEN" "$W" "$CONTROL" <<'PY2'
import re, sys, zlib, pathlib
golden, w, control = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
last = (golden / "provenance.toml").read_text().split('[[entry]]')[-1]
# Same freeze-vs-attest split as step 3: `[entry.targets.*]` is written by --freeze and
# is always present; `[entry.strict.goldens]` only appears once --attest has run. Reading
# only the second crashed on a frozen-but-unattested tip.
rows = re.findall(
    r'\[entry\.targets\.(\w+)\].*?full_crc = "([0-9a-f]{8})".*?full_size = (\d+)',
    last, re.S)
if rows:
    exp = {k: (c, int(sz)) for k, c, sz in rows}
else:
    m = re.search(r'\[entry\.strict\.goldens\](.*?)(\n\[|\Z)', last, re.S)
    if m is None:
        raise SystemExit("provision: the provenance tail names no expected ROMs "
                         "(neither [entry.targets.*] nor [entry.strict.goldens])")
    exp = {k: (c, int(sz)) for k, c, sz in
           re.findall(r'(\w+)\s*=\s*"([0-9a-f]{8})/(\d+)"', m.group(1))}
bad = 0
for key, fn in (("s4", "s4.bin"), ("s4_debug", "s4.debug.bin")):
    d = (w / fn).read_bytes()
    got = format(zlib.crc32(d) & 0xffffffff, '08x')
    ok = key in exp and got == exp[key][0] and len(d) == exp[key][1]
    if control != "required":
        # Not a control at this revision: the goldens describe other source. The CRCs
        # are DATA here, and the line says so rather than reading as a verdict.
        print(f"    BUILT (no control at this revision) {fn:16} {got}/{len(d)}")
        continue
    print(f"    REBUILD CONTROL {fn:16} {got}/{len(d)} {'MATCHES THE GOLDEN' if ok else 'DIFFERS'}")
    bad += 0 if ok else 1
if bad:
    raise SystemExit("a rebuilt ROM does not match its golden; this tree is NOT the pinned revision")
PY2

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

Built by: $SIGIL_BIN
Its full --version self-report heads this log. Quote it, not this line, when the
measurement is written up: a result is only about the compiler that produced it, and
a CRC cannot say which one that was.
EOF
